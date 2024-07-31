use core::mem::MaybeUninit;

use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use log::trace;
use num_traits::FromPrimitive;
use spinlock::SpinNoIrq;
use xhci::context::EndpointType;
use xhci::ring::trb::transfer::Direction;

use crate::abstractions::dma::DMA;
use crate::glue::ucb::{TransferEventCompleteCode, UCB};
use crate::usb::descriptors::DescriptorType;
use crate::usb::operation::ExtraStep;
use crate::usb::trasnfer::control::{
    bRequest, bmRequestType, ControlTransfer, DataTransferType, Recipient,
};
use crate::usb::trasnfer::interrupt::InterruptTransfer;
use crate::usb::urb::{RequestedOperation, URB};
use crate::USBSystemConfig;
use crate::{
    abstractions::PlatformAbstractions,
    glue::driver_independent_device_instance::DriverIndependentDeviceInstance,
    host::data_structures::MightBeInited,
    usb::{
        descriptors::{
            desc_device::USBDeviceClassCode, desc_endpoint::Endpoint,
            TopologicalUSBDescriptorFunction,
        },
        drivers::driverapi::{USBSystemDriverModule, USBSystemDriverModuleInstance},
    },
};

use super::USBHidDeviceSubClassCode;

pub enum ReportDescState<O>
where
    O: PlatformAbstractions,
{
    Binary(SpinNoIrq<DMA<u8, O::DMA>>),
}

pub struct HidMouseDriver<O>
//Driver should had a copy of independent device,at least should had ref of interface/config val and descriptors
where
    O: PlatformAbstractions,
{
    config: Arc<SpinNoIrq<USBSystemConfig<O>>>,

    bootable: usize,
    device_slot_id: usize,
    interrupt_in_channels: Vec<u32>,
    interrupt_out_channels: Vec<u32>,
    interface_value: usize, //temporary place them here
    interface_alternative_value: usize,
    config_value: usize, // same
    report_descriptor: Option<ReportDescState<O>>,
    driver_state_machine: HidMouseStateMachine,
    receiption_buffer: Option<SpinNoIrq<DMA<[u8], O::DMA>>>,
}

pub enum HidMouseStateMachine {
    Waiting,
    Sending,
}

impl<'a, O> HidMouseDriver<O>
where
    O: PlatformAbstractions + 'static,
{
    fn new_and_init(
        device_slot_id: usize,
        bootable: u8,
        endpoints: Vec<Endpoint>,
        config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
        interface_value: usize,
        alternative_val: usize,
        config_value: usize,
    ) -> Arc<SpinNoIrq<dyn USBSystemDriverModuleInstance<'a, O>>> {
        Arc::new(SpinNoIrq::new(Self {
            device_slot_id,
            interrupt_in_channels: {
                endpoints
                    .iter()
                    .filter_map(|ep| match ep.endpoint_type() {
                        EndpointType::InterruptIn => Some(ep.doorbell_value_aka_dci()),
                        _ => None,
                    })
                    .collect()
            },
            interrupt_out_channels: {
                endpoints
                    .iter()
                    .filter_map(|ep| match ep.endpoint_type() {
                        EndpointType::InterruptOut => Some(ep.doorbell_value_aka_dci()),
                        _ => None,
                    })
                    .collect()
            },
            config,
            interface_value,
            config_value,
            interface_alternative_value: alternative_val,
            bootable: bootable as usize,
            report_descriptor: None,
            driver_state_machine: HidMouseStateMachine::Sending,
            receiption_buffer: None,
        }))
    }
}

impl<'a, O> USBSystemDriverModuleInstance<'a, O> for HidMouseDriver<O>
where
    O: PlatformAbstractions,
{
    fn gather_urb(&mut self) -> Option<Vec<crate::usb::urb::URB<'a, O>>> {
        match self.driver_state_machine {
            HidMouseStateMachine::Waiting => None,
            HidMouseStateMachine::Sending => {
                self.driver_state_machine = HidMouseStateMachine::Waiting;
                match &self.receiption_buffer {
                    Some(buffer) => buffer.lock().fill_with(|| 0u8),
                    None => {
                        self.receiption_buffer = Some(SpinNoIrq::new(DMA::new_vec(
                            0u8,
                            8,
                            O::PAGE_SIZE,
                            self.config.lock().os.dma_alloc(),
                        )))
                    }
                }

                if let Some(buffer) = &mut self.receiption_buffer {
                    trace!("some!");
                    return Some(vec![URB::<O>::new(
                        self.device_slot_id,
                        RequestedOperation::Interrupt(InterruptTransfer {
                            endpoint_id: self.interrupt_in_channels.last().unwrap().clone()
                                as usize,
                            buffer_addr_len: buffer.lock().addr_len_tuple(),
                        }),
                    )]);
                }
                None
            }
        }
    }

    fn receive_complete_event(&mut self, ucb: UCB<O>) {
        match ucb.code {
            crate::glue::ucb::CompleteCode::Event(TransferEventCompleteCode::Success) => {
                trace!("completed!");
                self.receiption_buffer
                    .as_ref()
                    .map(|a| a.lock().to_vec().clone())
                    .inspect(|a| {
                        trace!("current buffer:{:?}", a);
                    });
                self.driver_state_machine = HidMouseStateMachine::Sending
            }
            other => panic!("received {:?}", other),
        }
    }

    fn prepare_for_drive(&mut self) -> Option<Vec<URB<'a, O>>> {
        trace!("hid mouse preparing for drive!");
        let endpoint_in = self.interrupt_in_channels.last().unwrap();
        let mut todo_list = Vec::new();
        todo_list.push(URB::new(
            self.device_slot_id,
            RequestedOperation::Control(ControlTransfer {
                request_type: bmRequestType::new(
                    Direction::Out,
                    DataTransferType::Standard,
                    Recipient::Device,
                ),
                request: bRequest::SetConfiguration,
                index: self.interface_value as u16,
                value: self.config_value as u16,
                data: None,
            }),
        ));
        todo_list.push(URB::new(
            self.device_slot_id,
            RequestedOperation::Control(ControlTransfer {
                request_type: bmRequestType::new(
                    Direction::Out,
                    DataTransferType::Standard,
                    Recipient::Interface,
                ),
                request: bRequest::SetInterfaceSpec,
                index: self.interface_alternative_value as u16,
                value: self.interface_value as u16,
                data: None,
            }),
        ));

        if self.bootable > 0 {
            todo_list.push(URB::new(
                self.device_slot_id,
                RequestedOperation::Control(ControlTransfer {
                    request_type: bmRequestType::new(
                        Direction::Out,
                        DataTransferType::Class,
                        Recipient::Interface,
                    ),
                    request: bRequest::SetInterfaceSpec, //actually set protocol
                    index: if self.bootable == 2 { 1 } else { 0 },
                    value: self.interface_value as u16,
                    data: None,
                }),
            ));
        }

        self.report_descriptor = Some(ReportDescState::<O>::Binary(SpinNoIrq::new(DMA::new(
            0u8,
            O::PAGE_SIZE,
            self.config.lock().os.dma_alloc(),
        ))));

        if let Some(ReportDescState::Binary(buf)) = &self.report_descriptor {
            todo_list.push(URB::new(
                self.device_slot_id,
                RequestedOperation::Control(ControlTransfer {
                    request_type: bmRequestType::new(
                        Direction::In,
                        DataTransferType::Standard,
                        Recipient::Interface,
                    ),
                    request: bRequest::GetDescriptor,
                    index: self.interface_alternative_value as u16,
                    value: DescriptorType::HIDReport.forLowBit(0).bits(),
                    data: Some({ buf.lock().addr_len_tuple() }),
                }),
            ));
        }

        self.interrupt_in_channels
            .iter()
            .chain(self.interrupt_out_channels.iter())
            .for_each(|dci| {
                todo_list.push(URB::new(
                    self.device_slot_id,
                    RequestedOperation::ExtraStep(ExtraStep::PrepareForTransfer(*dci as _)),
                ));
            });

        Some(todo_list)
    }
}

pub struct HidMouseDriverModule; //TODO: Create annotations to register

impl<'a, O> USBSystemDriverModule<'a, O> for HidMouseDriverModule
where
    O: PlatformAbstractions + 'static,
{
    fn should_active(
        &self,
        independent_dev: &DriverIndependentDeviceInstance<O>,
        config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
    ) -> Option<Vec<Arc<SpinNoIrq<dyn USBSystemDriverModuleInstance<'a, O>>>>> {
        if let MightBeInited::Inited(inited) = &independent_dev.descriptors {
            let device = inited.device.first().unwrap();
            return match (
                USBDeviceClassCode::from_u8(device.data.class),
                USBHidDeviceSubClassCode::from_u8(device.data.subclass),
                device.data.protocol,
            ) {
                (
                    Some(USBDeviceClassCode::HID),
                    Some(USBHidDeviceSubClassCode::Mouse),
                    bootable,
                ) => {
                    return Some(vec![HidMouseDriver::new_and_init(
                        independent_dev.slotid,
                        bootable,
                        {
                            device
                                .child
                                .iter()
                                .find(|c| {
                                    c.data.config_val() == independent_dev.configuration_val as u8
                                })
                                .expect("configuration not found")
                                .child
                                .iter()
                                .filter_map(|func| match func {
                                    TopologicalUSBDescriptorFunction::InterfaceAssociation(_) => {
                                        panic!("a super complex device, help meeeeeeeee!");
                                    }
                                    TopologicalUSBDescriptorFunction::Interface(interface) => Some(
                                        interface
                                            .iter()
                                            .find(|(interface, alternatives, endpoints)| {
                                                interface.interface_number
                                                    == independent_dev.interface_val as u8
                                                    && interface.alternate_setting
                                                        == independent_dev
                                                            .current_alternative_interface_value
                                                            as u8
                                            })
                                            .expect("invalid interface value or alternative value")
                                            .2
                                            .clone(),
                                    ),
                                })
                                .take(1)
                                .flat_map(|a| a)
                                .collect()
                        },
                        config.clone(),
                        independent_dev.interface_val,
                        independent_dev.current_alternative_interface_value,
                        independent_dev.configuration_val,
                    )]);
                }
                (Some(USBDeviceClassCode::ReferInterfaceDescriptor), _, _) => Some({
                    let collect = device
                        .child
                        .iter()
                        .find(|configuration| {
                            configuration.data.config_val()
                                == independent_dev.configuration_val as u8
                        })
                        .expect("configuration not found")
                        .child
                        .iter()
                        .filter_map(|interface| match interface {
                            TopologicalUSBDescriptorFunction::InterfaceAssociation((
                                asso,
                                interfaces,
                            )) if let (
                                USBDeviceClassCode::HID,
                                USBHidDeviceSubClassCode::Mouse,
                                bootable,
                            ) = (
                                USBDeviceClassCode::from_u8(asso.function_class).unwrap(),
                                USBHidDeviceSubClassCode::from_u8(asso.function_subclass).unwrap(),
                                asso.function_protocol,
                            ) =>
                            {
                                // return Some(Self::new_and_init(independent_dev.slotid, bootable));
                                panic!("a super complex device, help meeeeeeeee!");
                            }
                            TopologicalUSBDescriptorFunction::Interface(interfaces) => {
                                let (interface, additional, endpoints) = interfaces
                                    .get(independent_dev.current_alternative_interface_value)
                                    .expect("invalid anternative interface value");
                                if let (
                                    Some(USBDeviceClassCode::HID),
                                    Some(USBHidDeviceSubClassCode::Mouse),
                                    bootable,
                                ) = (
                                    USBDeviceClassCode::from_u8(interface.interface_class),
                                    USBHidDeviceSubClassCode::from_u8(interface.interface_subclass),
                                    interface.interface_protocol,
                                ) {
                                    return Some(HidMouseDriver::new_and_init(
                                        independent_dev.slotid,
                                        bootable,
                                        endpoints.clone(),
                                        config.clone(),
                                        independent_dev.interface_val,
                                        independent_dev.current_alternative_interface_value,
                                        independent_dev.configuration_val,
                                    ));
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        })
                        .collect();
                    collect
                }),
                _ => None,
            };
        }
        None
    }

    fn preload_module(&self) {
        trace!("preloading Hid mouse driver!")
    }
}
