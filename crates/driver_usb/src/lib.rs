#![no_std]
#![feature(allocator_api)]
#![feature(strict_provenance)]
#![allow(warnings)]
#![feature(auto_traits)]
#![feature(btreemap_alloc)]
#![feature(if_let_guard)]
#![feature(get_many_mut)]
#![feature(let_chains)]
#![feature(cfg_match)]
#![feature(iter_collect_into)]

use core::usize;

use abstractions::{dma::DMA, PlatformAbstractions};
use alloc::{
    collections::{btree_map::BTreeMap, btree_set::BTreeSet},
    sync::Arc,
    vec::Vec,
};
use glue::driver_independent_device_instance::DriverIndependentDeviceInstance;
use host::{data_structures::MightBeInited, USBHostSystem};
use log::{error, trace};
use spinlock::SpinNoIrq;
use usb::{
    descriptors::{DescriptorType, TopologicalUSBDescriptorRoot},
    operation,
    trasnfer::control::{bRequest, bmRequestType, ControlTransfer, DataTransferType},
    urb::{RequestedOperation, URB},
    USBDriverSystem,
};
use xhci::ring::trb::transfer::{Direction, TransferType};

extern crate alloc;

pub mod abstractions;
pub mod err;
pub mod glue;
pub mod host;
pub mod usb;

#[derive(Clone, Debug)]
pub struct USBSystemConfig<O>
where
    O: PlatformAbstractions,
{
    pub(crate) base_addr: O::VirtAddr,
    pub(crate) irq_num: u32,
    pub(crate) irq_priority: u32,
    pub(crate) os: O,
}

pub struct USBSystem<'a, O>
where
    O: PlatformAbstractions,
{
    platform_abstractions: O,
    config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
    host_driver_layer: USBHostSystem<O>,
    usb_driver_layer: USBDriverSystem<'a, O>,
    driver_independent_devices: Vec<DriverIndependentDeviceInstance<O>>,
}

impl<'a, O> USBSystem<'a, O>
where
    O: PlatformAbstractions + 'static,
{
    pub fn new(config: USBSystemConfig<O>) -> Self {
        let config = Arc::new(SpinNoIrq::new(config));
        Self {
            config: config.clone(),
            platform_abstractions: config.clone().lock().os.clone(),
            host_driver_layer: USBHostSystem::new(config.clone()).unwrap(),
            usb_driver_layer: USBDriverSystem::new(config.clone()),
            driver_independent_devices: Vec::new(),
        }
    }

    pub fn init(mut self) -> Self {
        trace!("initializing!");
        self.host_driver_layer.init();
        self.usb_driver_layer.init();
        trace!("usb system init complete");
        self
    }

    pub fn init_probe(mut self) -> Self {
        // async { //todo:async it!
        {
            self.driver_independent_devices.clear(); //need to have a merge algorithm for hot plug
            let mut after = Vec::new();

            self.host_driver_layer.probe(|device| after.push(device));

            for driver in after {
                self.new_device(driver)
            }
            trace!("device probe complete");
        }
        {
            let mut preparing_list = Vec::new();
            self.usb_driver_layer
                .init_probe(&mut self.driver_independent_devices, &mut preparing_list);

            //probe driver modules and load them
            self.host_driver_layer.tock(preparing_list);

            //and do some prepare stuff
        }
        // }
        // .await;

        self
    }

    pub fn driver_active(mut self) -> Self {
        self
    }

    pub fn drive_all(mut self) -> Self {
        //TODO: Drive All

        loop {
            let tick = self.usb_driver_layer.tick();
            if tick.len() != 0 {
                trace!("tick! {:?}", tick.len());
                self.host_driver_layer.tock(tick);
            }
            // trace!("tock!");
        }
        self
    }

    pub fn drop_device(&mut self, mut driver_independent_device_slot_id: usize) {
        //do something
    }

    pub fn new_device(&mut self, mut driver: DriverIndependentDeviceInstance<O>) {
        'label: {
            if let MightBeInited::Uninit = driver.descriptors {
                let buffer = DMA::new_vec(
                    0u8,
                    O::PAGE_SIZE,
                    O::PAGE_SIZE,
                    self.config.lock().os.dma_alloc(),
                );

                let desc = match (&driver.controller).lock().control_transfer(
                    driver.slotid,
                    ControlTransfer {
                        request_type: bmRequestType::new(
                            Direction::In,
                            DataTransferType::Standard,
                            usb::trasnfer::control::Recipient::Device,
                        ),
                        request: bRequest::GetDescriptor,
                        index: 0,
                        value: DescriptorType::Device.forLowBit(0).bits(),
                        data: Some(buffer.addr_len_tuple()),
                    },
                ) {
                    Ok(_) => {
                        let mut parse_root_descriptors =
                            usb::descriptors::RawDescriptorParser::<O>::new(buffer)
                                .parse_root_descriptors(true);
                        {
                            let first = parse_root_descriptors.device.first_mut().unwrap();
                            for index in 0..first.data.num_configurations {
                                let buffer = DMA::new_vec(
                                    0u8,
                                    O::PAGE_SIZE,
                                    O::PAGE_SIZE,
                                    self.config.lock().os.dma_alloc(),
                                );
                                (&driver.controller)
                                    .lock()
                                    .control_transfer(
                                        driver.slotid,
                                        ControlTransfer {
                                            request_type: bmRequestType::new(
                                                Direction::In,
                                                DataTransferType::Standard,
                                                usb::trasnfer::control::Recipient::Device,
                                            ),
                                            request: bRequest::GetDescriptor,
                                            index: 0,
                                            value: DescriptorType::Configuration
                                                .forLowBit(index)
                                                .bits(),
                                            data: Some(buffer.addr_len_tuple()),
                                        },
                                    )
                                    .inspect(|_| {
                                        first.child.push(
                                            usb::descriptors::RawDescriptorParser::<O>::new(buffer)
                                                .parse_config_descriptor()
                                                .unwrap(),
                                        )
                                    });
                            }
                        }

                        driver.descriptors = MightBeInited::Inited(parse_root_descriptors);
                        //fetch driver descriptors
                    }
                    Err(err) => {
                        error!("err! {:?}", err);
                        break 'label;
                    }
                };
            }

            trace!("parsed descriptor:{:#?}", driver.descriptors);

            if let MightBeInited::Inited(TopologicalUSBDescriptorRoot {
                device: devices,
                others,
            }) = &driver.descriptors
            {
                self.host_driver_layer
                    .urb_request(URB::new(
                        driver.slotid,
                        RequestedOperation::ConfigureDevice(operation::Configuration::SetupDevice(
                            //TODO: fixme
                            devices.first().unwrap().child.first().unwrap(),
                        )),
                    ))
                    .unwrap();
            };

            self.driver_independent_devices.push(driver);
        }
        //do something
    }
}

// #[cfg(feature = "arceos")]
// pub mod ax;
