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
#![feature(const_trait_impl)]

use core::{mem::MaybeUninit, usize};

use abstractions::{dma::DMA, PlatformAbstractions};
use alloc::{
    collections::{btree_map::BTreeMap, btree_set::BTreeSet},
    sync::Arc,
    vec::Vec,
};
use cfg_if::cfg_if;
use glue::driver_independent_device_instance::DriverIndependentDeviceInstance;
use host::{data_structures::MightBeInited, USBHostSystem};
use log::{error, trace};
use spinlock::SpinNoIrq;
use usb::{
    descriptors::{
        construct_control_transfer_type, parser::RawDescriptorParser,
        topological_desc::TopologicalUSBDescriptorRoot, USBStandardDescriptorTypes,
    },
    operation,
    transfer::control::{bmRequestType, ControlTransfer, DataTransferType, StandardbRequest},
    urb::{RequestedOperation, URB},
    USBDriverSystem,
};
use xhci::ring::trb::transfer::{Direction, TransferType};

extern crate alloc;

/// PlatformAbstractions
pub mod abstractions;

///USB System Related Errors
pub mod err;

///Some glue code to connect these layers.
pub mod glue;

///Host Controller Layer
pub mod host;
///USB Layer
pub mod usb;

cfg_if! {
 if #[cfg(feature = "pci")]{
        /// pci interface, only available when feature "pci" is enabled. XHCI host controller is designed to attach on PCI BUS, thus we provide some abstraction to make it easier to use.
        pub mod pci_interface;
        pub use pci_interface::*;
        pub use pci_interface::XHCIPCIDriver;
    }
}
/// The Configuration of the USB system.
/// * base_addr: The base address of the USB controller registers.
/// * irq_num: The IRQ number of the USB controller. *Note*: we recomment use polling instead of IRQ. Interrupt would cause performance issue some times.
/// * irq_priority: The IRQ priority of the USB controller.
/// * transfer_ring_size: The Size of the transfer ring, which means how *long* is the ring buffer, in xhci we use it in both CommandRing/EventRing/TransferRing, The buffer *Must* not Cross Memory Page
/// * os: The PlatformAbstraction, refer[`PlatformAbstractions``].
#[derive(Clone, Debug)]
pub struct USBSystemConfig<O>
where
    O: PlatformAbstractions,
{
    pub(crate) base_addr: O::VirtAddr,
    pub(crate) irq_num: u32,
    pub(crate) irq_priority: u32,
    pub(crate) transfer_ring_size: usize,
    pub(crate) os: O,
}

/// The USB System.
///
/// The USB System Contains 2 parts:
/// * Host Driver Layer: The Host Controller Driver, which is the driver of the USB controllers.
/// * USB driver layer: USB Driver systems. which contains a bunch of implemented USB Drivers, and provide interface to be invoked from outside to register new driver. And execute these Drivers!
///
/// Usage:
/// ```rust
/// #[derive(Clone)]
/// struct PlatformAbstraction;
///
/// impl driver_usb::abstractions::OSAbstractions for PlatformAbstraction {
///     type VirtAddr = VirtAddr;
///     type DMA = GlobalNoCacheAllocator;
///
///     const PAGE_SIZE: usize = PageSize::Size4K as usize;
///
///     fn dma_alloc(&self) -> Self::DMA {
///         axalloc::global_no_cache_allocator()
///     }
/// }
///
/// impl driver_usb::abstractions::HALAbstractions for PlatformAbstraction {
///     fn force_sync_cache() {}
/// }
///
/// //...
///     let mut usbsystem = driver_usb::USBSystem::new({
///        USBSystemConfig::new(
///            0xffff_0000_31a0_8000,
///            48,
///            0,
///            PageSize::Size4K.into(),
///            PlatformAbstraction,
///        )
///    })
///    .init()
///    .init_probe()
///    .drive_all();
/// ```
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
    /// Creates a new USB system
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

    /// Initialize host and usb driver systems!
    ///
    /// This is required before any other operations.
    ///
    /// it Perform initialze operations of host controller devices and register all implemented USB drivers
    pub fn init(&mut self) -> &mut Self {
        trace!("initializing!");
        self.host_driver_layer.init();
        self.usb_driver_layer.init();
        trace!("usb system init complete");
        self
    }

    /// Initialze Probe, Probe for devices that attached
    ///
    /// If found device can be drived, then create device driver instance(basiclly a State Machine)
    pub fn init_probe(&mut self) -> &mut Self {
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
            self.host_driver_layer
                .tock(preparing_list, &mut self.driver_independent_devices);

            //and do some prepare stuff
        }
        // }
        // .await;

        self
    }

    /// execute all device driver instance in a loop. each loop had 2 phase
    /// Tick: Gather Usb Request Block(URB) from all device driver instances!
    /// Tock: Execute all URBs, and return USB Complete Block(UCB) to all device driver instances
    pub fn drive_all(&mut self) -> &mut Self {
        //TODO: Drive All

        loop {
            let tick = self.usb_driver_layer.tick();
            if tick.len() != 0 {
                trace!("tick! {:?}", tick.len());
                self.host_driver_layer
                    .tock(tick, &mut self.driver_independent_devices);
            }
            // trace!("tock!");
        }
        self
    }

    /// Drop a device driver instance from slot id
    ///
    /// * WIP
    pub fn drop_device(&mut self, mut driver_independent_device_slot_id: usize) {
        //do something
    }

    /// Create A device driver instance from [`DriverIndependentDeviceInstance`]
    ///
    /// when this function been executed, the Related Structures in HOST Layer *Must* Had been initialized!
    ///
    /// steps:
    /// 1. Gather All Device Descriptors
    /// 2. Rebuild Descriper Topology
    /// 3. Try to Initialize Device Driver Instance if had suitable driver for this device
    pub fn new_device(&mut self, mut driver: DriverIndependentDeviceInstance<O>) {
        'label: {
            if let MightBeInited::Uninit = *driver.descriptors {
                let buffer_device = DMA::new_vec(
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
                            usb::transfer::control::Recipient::Device,
                        ),
                        request: StandardbRequest::GetDescriptor.into(),
                        index: 0,
                        value: construct_control_transfer_type(
                            USBStandardDescriptorTypes::Device as u8,
                            0,
                        )
                        .bits(),
                        data: Some(O::map_addr_len_tuple(buffer_device.addr_len_tuple())),
                        report: false,
                    },
                ) {
                    Ok(_) => {
                        let mut parser = RawDescriptorParser::<O>::new(buffer_device);
                        parser.single_state_cycle();
                        let num_of_configs = parser.num_of_configs();
                        for index in 0..num_of_configs {
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
                                            usb::transfer::control::Recipient::Device,
                                        ),
                                        request: StandardbRequest::GetDescriptor.into(),
                                        index: 0,
                                        value: construct_control_transfer_type(
                                            USBStandardDescriptorTypes::Configuration as u8,
                                            index as _,
                                        )
                                        .bits(),
                                        data: Some(O::map_addr_len_tuple(buffer.addr_len_tuple())),
                                        report: false,
                                    },
                                )
                                .inspect(|_| {
                                    parser.append_config(buffer);
                                });
                        }
                        driver.descriptors = Arc::new(MightBeInited::Inited(parser.summarize()));
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
                metadata,
            }) = &*driver.descriptors
            {
                self.host_driver_layer
                    .urb_request(
                        URB::new(
                            driver.slotid,
                            RequestedOperation::ConfigureDevice(
                                operation::Configuration::SetupDevice(
                                    //TODO: fixme
                                    devices.first().unwrap().child.first().unwrap(),
                                ),
                            ),
                        ),
                        &mut self.driver_independent_devices,
                    )
                    .unwrap();
            };

            self.driver_independent_devices.push(driver);
            trace!(
                "pushed new device! {:?}",
                self.driver_independent_devices.len()
            )
        }
        //do something
    }
}
