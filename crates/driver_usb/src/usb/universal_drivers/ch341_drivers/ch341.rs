use core::mem::MaybeUninit;

use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use axtask::set_priority;
use log::trace;
use num_traits::FromPrimitive;
use spinlock::SpinNoIrq;
use xhci::context::EndpointType;
use xhci::ring::trb::transfer::Direction;

use crate::abstractions::dma::DMA;
use crate::glue::ucb::{TransferEventCompleteCode, UCB};
use crate::usb::descriptors::desc_hid::HIDDescriptorTypes;
use crate::usb::descriptors::topological_desc::{
    TopologicalUSBDescriptorEndpoint, TopologicalUSBDescriptorFunction,
};
use crate::usb::descriptors::USBStandardDescriptorTypes;
use crate::usb::operation::ExtraStep;
use crate::usb::trasnfer::bulk::BulkTransfer;
use crate::usb::trasnfer::control::{
    bRequest, bmRequestType, ControlTransfer, DataTransferType, Recipient, StandardbRequest,
};
use crate::usb::trasnfer::interrupt::InterruptTransfer;
use crate::usb::universal_drivers;
use crate::usb::urb::{RequestedOperation, URB};
use crate::USBSystemConfig;
use crate::{
    abstractions::PlatformAbstractions,
    glue::driver_independent_device_instance::DriverIndependentDeviceInstance,
    host::data_structures::MightBeInited,
    usb::{
        descriptors::{desc_device::StandardUSBDeviceClassCode, desc_endpoint::Endpoint},
        drivers::driverapi::{USBSystemDriverModule, USBSystemDriverModuleInstance},
    },
};

use super::{DeviceStateMachine, SendingWaitingWithCountStateMachine};

pub struct CH341driverModule;

impl<'a, O> USBSystemDriverModule<'a, O> for CH341driverModule
where
    O: PlatformAbstractions + 'static,
{
    fn should_active(
        &self,
        independent_dev: &mut DriverIndependentDeviceInstance<O>,
        config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
    ) -> Option<Vec<Arc<SpinNoIrq<dyn USBSystemDriverModuleInstance<'a, O>>>>> {
        if let MightBeInited::Inited(desc) = &*independent_dev.descriptors {
            if desc.device.iter().any(|desc| desc.data.class == 255) {
                if let Some(device1) = desc.device.get(0) {
                    if let Some(child1) = device1.child.get(0) {
                        if let Some(child2) = child1.child.get(0) {
                            match child2 {
                                TopologicalUSBDescriptorFunction::Interface(interface_data) => {
                                    for (interface, usb_descriptors, endpoints) in interface_data {
                                        if interface.interface_class == 255 {
                                            return Some(vec![CH341driver::new_and_init(
                                                independent_dev.slotid,
                                                {
                                                    device1
                                                        .child
                                                        .iter()
                                                        .find(|c| {
                                                            c.data.config_val()
                                                                == independent_dev.configuration_val
                                                                    as u8
                                                        })
                                                        .expect("configuration not found")
                                                        .child
                                                        .iter()
                                                        .filter_map(|func| match func {
                                                            TopologicalUSBDescriptorFunction::InterfaceAssociation(_) => {
                                                                panic!("a super complex device, help meeeeeeeee!");
                                                            }
                                                            TopologicalUSBDescriptorFunction::Interface(interface) => {
                                                                Some(
                                                                    interface
                                                                        .iter()
                                                                        .find(|(
                                                                            interface,
                                                                            _alternatives,
                                                                            _endpoints,
                                                                        )| {
                                                                            interface.interface_number
                                                                                == independent_dev
                                                                                    .interface_val
                                                                                    as u8
                                                                                && interface.alternate_setting
                                                                                    == independent_dev
                                                                                        .current_alternative_interface_value
                                                                                        as u8
                                                                        })
                                                                        .expect(
                                                                            "invalid interface value or alternative value",
                                                                        )
                                                                        .2
                                                                        .clone(),
                                                                )
                                                            }
                                                        })
                                                        .take(1)
                                                        .flat_map(|a| a)
                                                        .filter_map(|e| {
                                                            if let TopologicalUSBDescriptorEndpoint::Standard(
                                                                ep,
                                                            ) = e
                                                            {
                                                                Some(ep)
                                                            } else {
                                                                None
                                                            }
                                                        })
                                                        .collect()
                                                },
                                                config.clone(),
                                                independent_dev.interface_val,
                                                independent_dev.current_alternative_interface_value,
                                                independent_dev.configuration_val,
                                            )]);
                                        }
                                    }
                                }
                                _ => (),
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn preload_module(&self) {
        trace!("nothing");
    }
}

pub struct CH341driver<O>
where
    O: PlatformAbstractions,
{
    config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
    device_slot_id: usize,
    interrupt_in_channels: Vec<u32>,
    interrupt_out_channels: Vec<u32>,
    bulk_in_channels: Vec<u32>,
    bulk_out_channels: Vec<u32>,
    interface_value: usize, //temporary place them here
    interface_alternative_value: usize,
    config_value: usize, // same
    driver_state_machine: DeviceStateMachine,
    sending_waiting_with_count_state_machine: SendingWaitingWithCountStateMachine,
    receiption_buffer: SpinNoIrq<DMA<[u8], O::DMA>>,
    baud_rate: usize, /* set baud rate */
    mcr: u8,
    msr: u8,
    lcr: u8,
    version: u8,
    break_end: usize,
}

impl<'a, O> CH341driver<O>
where
    O: PlatformAbstractions + 'static,
{
    fn new_and_init(
        device_slot_id: usize,
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
            bulk_in_channels: {
                endpoints
                    .iter()
                    .filter_map(|ep| match ep.endpoint_type() {
                        EndpointType::BulkIn => Some(ep.doorbell_value_aka_dci()),
                        _ => None,
                    })
                    .collect()
            },
            bulk_out_channels: {
                endpoints
                    .iter()
                    .filter_map(|ep| match ep.endpoint_type() {
                        EndpointType::InterruptOut => Some(ep.doorbell_value_aka_dci()),
                        _ => None,
                    })
                    .collect()
            },
            config: config.clone(),
            interface_value,
            config_value,
            interface_alternative_value: alternative_val,
            driver_state_machine: DeviceStateMachine::FetchingVersion,
            receiption_buffer: SpinNoIrq::new(DMA::new_vec(
                0u8,
                8,
                O::PAGE_SIZE,
                config.lock().os.dma_alloc(),
            )),
            baud_rate: 0,
            mcr: 0,
            msr: 0,
            lcr: 0,
            version: 0,
            break_end: 0,
            sending_waiting_with_count_state_machine: SendingWaitingWithCountStateMachine::Sending,
        }))
    }
}

impl<'a, O> USBSystemDriverModuleInstance<'a, O> for CH341driver<O>
where
    O: PlatformAbstractions,
{
    fn prepare_for_drive(&mut self) -> Option<Vec<URB<'a, O>>> {
        let mut todo_list = Vec::new();
        trace!("---------------------here start----------------------------");
        todo_list.push(URB::new(
            self.device_slot_id,
            RequestedOperation::Control(ControlTransfer {
                request_type: bmRequestType::new(
                    Direction::Out,
                    DataTransferType::Standard,
                    Recipient::Device,
                ),
                request: StandardbRequest::SetConfiguration.into(),
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
                request: StandardbRequest::SetInterface.into(),
                index: self.interface_alternative_value as u16,
                value: self.interface_value as u16,
                data: None,
            }),
        ));

        self.interrupt_in_channels
            .iter()
            .chain(self.interrupt_out_channels.iter())
            .chain(self.bulk_in_channels.iter())
            .chain(self.bulk_out_channels.iter())
            .for_each(|dci| {
                todo_list.push(URB::new(
                    self.device_slot_id,
                    RequestedOperation::ExtraStep(ExtraStep::PrepareForTransfer(*dci as _)),
                ));
            });

        trace!("------------------------------here end--------------------------------");
        Some(todo_list)
    }

    fn gather_urb(&mut self) -> Option<Vec<URB<'a, O>>> {
        match self.driver_state_machine {
            DeviceStateMachine::FetchingVersion => {
                trace!("FetchingVersion!");
                return Some(vec![URB::<O>::new(
                    self.device_slot_id,
                    RequestedOperation::Control(ControlTransfer {
                        request_type: bmRequestType::new(
                            Direction::In,
                            DataTransferType::Vendor,
                            Recipient::Device,
                        ),
                        request: bRequest::DriverSpec(0x5F),
                        index: 0 as u16,
                        value: 0 as u16,
                        data: Some(self.receiption_buffer.lock().addr_len_tuple()),
                    }),
                )]);
                None
            }
            DeviceStateMachine::CH341Setup => {
                trace!("CH341Setup!");
                let mut vec = Vec::new();
                vec.push(URB::new(
                    self.device_slot_id,
                    RequestedOperation::Control(ControlTransfer {
                        request_type: bmRequestType::new(
                            Direction::Out,
                            DataTransferType::Vendor,
                            Recipient::Device,
                        ),
                        request: bRequest::DriverSpec(0xA1),
                        index: 0 as u16,
                        value: 0 as u16,
                        data: None,
                    }),
                ));
                vec.push(URB::new(
                    self.device_slot_id,
                    RequestedOperation::Control(ControlTransfer {
                        request_type: bmRequestType::new(
                            Direction::Out,
                            DataTransferType::Vendor,
                            Recipient::Device,
                        ),
                        request: bRequest::DriverSpec(0x9A),
                        index: 0xd982 as u16,
                        value: 0x1312 as u16,
                        data: None,
                    }),
                ));
                self.sending_waiting_with_count_state_machine =
                    SendingWaitingWithCountStateMachine::Waiting(vec.len());
                Some(vec)
            }
            DeviceStateMachine::first => {
                trace!("FetchingVersion!");
                return Some(vec![URB::<O>::new(
                    self.device_slot_id,
                    RequestedOperation::Control(ControlTransfer {
                        request_type: bmRequestType::new(
                            Direction::In,
                            DataTransferType::Vendor,
                            Recipient::Device,
                        ),
                        request: bRequest::DriverSpec(0x95),
                        index: 0 as u16,
                        value: 0x2c2c as u16,
                        data: Some(self.receiption_buffer.lock().addr_len_tuple()),
                    }),
                )]);
                None
            }
            DeviceStateMachine::second => {
                let mut vec = Vec::new();
                vec.push(URB::new(
                    self.device_slot_id,
                    RequestedOperation::Control(ControlTransfer {
                        request_type: bmRequestType::new(
                            Direction::Out,
                            DataTransferType::Vendor,
                            Recipient::Device,
                        ),
                        request: bRequest::DriverSpec(0x9A),
                        index: 0x0007 as u16,
                        value: 0x0f2c as u16,
                        data: None,
                    }),
                ));
                self.sending_waiting_with_count_state_machine =
                    SendingWaitingWithCountStateMachine::Waiting(vec.len());
                Some(vec)
            }
            DeviceStateMachine::third => {
                trace!("FetchingVersion!");
                return Some(vec![URB::<O>::new(
                    self.device_slot_id,
                    RequestedOperation::Control(ControlTransfer {
                        request_type: bmRequestType::new(
                            Direction::In,
                            DataTransferType::Vendor,
                            Recipient::Device,
                        ),
                        request: bRequest::DriverSpec(0x95),
                        index: 0 as u16,
                        value: 0x2518 as u16,
                        data: Some(self.receiption_buffer.lock().addr_len_tuple()),
                    }),
                )]);
                None
            }
            DeviceStateMachine::fourth => {
                trace!("FetchingVersion!");
                return Some(vec![URB::<O>::new(
                    self.device_slot_id,
                    RequestedOperation::Control(ControlTransfer {
                        request_type: bmRequestType::new(
                            Direction::In,
                            DataTransferType::Vendor,
                            Recipient::Device,
                        ),
                        request: bRequest::DriverSpec(0x95),
                        index: 0 as u16,
                        value: 0x0706 as u16,
                        data: Some(self.receiption_buffer.lock().addr_len_tuple()),
                    }),
                )]);
                None
            }
            DeviceStateMachine::fifth =>{
                let mut vec = Vec::new();
                vec.push(URB::new(
                    self.device_slot_id,
                    RequestedOperation::Control(ControlTransfer {
                        request_type: bmRequestType::new(
                            Direction::Out,
                            DataTransferType::Vendor,
                            Recipient::Device,
                        ),
                        request: bRequest::DriverSpec(0x9A),
                        index: 0x0000 as u16,
                        value: 0x2727 as u16,
                        data: None,
                    }),
                ));
                self.sending_waiting_with_count_state_machine =
                    SendingWaitingWithCountStateMachine::Waiting(vec.len());
                Some(vec)
            }
            DeviceStateMachine::Opening => {
                trace!("start transfer");
                return Some(vec![URB::<O>::new(
                    self.device_slot_id,
                    RequestedOperation::Interrupt(InterruptTransfer {
                        endpoint_id: self.interrupt_in_channels.last().unwrap().clone()
                            as usize,
                        buffer_addr_len: self.receiption_buffer.lock().addr_len_tuple(),
                    }),
                )]);
            },
        }
    }

    fn receive_complete_event(&mut self, ucb: UCB<O>) {
        match self.driver_state_machine {
            DeviceStateMachine::FetchingVersion => match ucb.code {
                crate::glue::ucb::CompleteCode::Event(TransferEventCompleteCode::Success) => {
                    trace!("completed!");
                    let vec = self.receiption_buffer.lock().to_vec();
                    trace!("current buffer:{:?}", vec);
                    self.version = vec[0];
                    self.driver_state_machine = DeviceStateMachine::CH341Setup;
                }
                other => panic!("received {:?}", other),
            },
            DeviceStateMachine::CH341Setup => match ucb.code {
                crate::glue::ucb::CompleteCode::Event(TransferEventCompleteCode::Success) => {
                    if let SendingWaitingWithCountStateMachine::Waiting(waiting_count) =
                        &mut self.sending_waiting_with_count_state_machine
                    {
                        trace!("{:#?}", waiting_count);
                        let new_count = *waiting_count - 1;
                        self.sending_waiting_with_count_state_machine =
                            SendingWaitingWithCountStateMachine::Waiting(new_count);
                        if new_count <= 0 {
                            self.driver_state_machine = DeviceStateMachine::first;
                        }
                    }
                }
                other => panic!("received {:?}", other),
            },
            DeviceStateMachine::first => match ucb.code {
                crate::glue::ucb::CompleteCode::Event(TransferEventCompleteCode::Success) => {
                    trace!("completed!");
                    let vec = self.receiption_buffer.lock().to_vec();
                    trace!("+++");
                    trace!("+++");
                    trace!("+++");
                    trace!("current buffer:{:?}", vec);
                    trace!("+++");
                    trace!("+++");
                    trace!("+++");
                    self.driver_state_machine = DeviceStateMachine::second;
                }
                other => panic!("received {:?}", other),
            },
            DeviceStateMachine::second => match ucb.code {
                crate::glue::ucb::CompleteCode::Event(TransferEventCompleteCode::Success) => {
                    if let SendingWaitingWithCountStateMachine::Waiting(waiting_count) =
                        &mut self.sending_waiting_with_count_state_machine
                    {
                        trace!("{:#?}", waiting_count);
                        let new_count = *waiting_count - 1;
                        self.sending_waiting_with_count_state_machine =
                            SendingWaitingWithCountStateMachine::Waiting(new_count);
                        if new_count <= 0 {
                            self.driver_state_machine = DeviceStateMachine::third;
                        }
                    }
                }
                other => panic!("received {:?}", other),
            },
            DeviceStateMachine::third => match ucb.code {
                crate::glue::ucb::CompleteCode::Event(TransferEventCompleteCode::Success) => {
                    trace!("completed!");
                    let vec = self.receiption_buffer.lock().to_vec();
                    trace!("+++");
                    trace!("+++");
                    trace!("+++");
                    trace!("current buffer:{:?}", vec);
                    trace!("+++");
                    trace!("+++");
                    trace!("+++");
                    self.driver_state_machine = DeviceStateMachine::fourth;
                }
                other => panic!("received {:?}", other),
            },
            DeviceStateMachine::fourth => match ucb.code {
                crate::glue::ucb::CompleteCode::Event(TransferEventCompleteCode::Success) => {
                    trace!("completed!");
                    let vec = self.receiption_buffer.lock().to_vec();
                    trace!("+++");
                    trace!("+++");
                    trace!("+++");
                    trace!("current buffer:{:?}", vec);
                    trace!("+++");
                    trace!("+++");
                    trace!("+++");
                    self.driver_state_machine = DeviceStateMachine::fifth;
                }
                other => panic!("received {:?}", other),
            },
            DeviceStateMachine::fifth => match ucb.code {
                crate::glue::ucb::CompleteCode::Event(TransferEventCompleteCode::Success) => {
                    if let SendingWaitingWithCountStateMachine::Waiting(waiting_count) =
                        &mut self.sending_waiting_with_count_state_machine
                    {
                        trace!("{:#?}", waiting_count);
                        let new_count = *waiting_count - 1;
                        self.sending_waiting_with_count_state_machine =
                            SendingWaitingWithCountStateMachine::Waiting(new_count);
                        if new_count <= 0 {
                            self.driver_state_machine = DeviceStateMachine::Opening;
                        }
                    }
                }
                other => panic!("received {:?}", other),
            },
            DeviceStateMachine::Opening => match ucb.code{
                crate::glue::ucb::CompleteCode::Event(TransferEventCompleteCode::Success) => {
                    trace!("transfer completed!");
                    let vec = self.receiption_buffer.lock().to_vec();
                    trace!("+++");
                    trace!("+++");
                    trace!("+++");
                    trace!("current buffer:{:?}", vec);
                    trace!("+++");
                    trace!("+++");
                    trace!("+++");
                }
                other => panic!("wrror no success"),
            },
        }
    }
}

// let mut lcr: u8 = 0x80 | 0x40 | 0x03;

// let mut rate: usize = 9600;
// let mut factor: u32 = (1532620800 / rate).try_into().unwrap();
// let mut divisor: u16 = 3;
// while (factor > 0xfff0) && (divisor > 0) {
//     factor >>= 3;
//     divisor -= 1;
// }
// if factor > 0xfff0 {
//     trace!("factor wrror");
// }
// factor = 0x10000 - factor;
// let mut a: u16 = (factor & 0xff00) as u16 | divisor;
// a |= 1 << 7;

// let mut mcr = self.mcr;

// vec.push(URB::<O>::new(
//     self.device_slot_id,
//     RequestedOperation::Control(ControlTransfer {
//         request_type: bmRequestType::new(
//             Direction::Out,
//             DataTransferType::Vendor,
//             Recipient::Device,
//         ),
//         request: bRequest::DriverSpec(0xA1),
//         index: 0 as u16,
//         value: 0 as u16,
//         data: None,
//     }),
// ));
// vec.push(URB::new(
//     self.device_slot_id,
//     RequestedOperation::Control(ControlTransfer {
//         request_type: bmRequestType::new(
//             Direction::Out,
//             DataTransferType::Vendor,
//             Recipient::Device,
//         ),
//         request: bRequest::DriverSpec(0x9A),
//         index: a as u16,
//         value: 0x1312 as u16,
//         data: None,
//     }),
// ));
// vec.push(URB::new(
//     self.device_slot_id,
//     RequestedOperation::Control(ControlTransfer {
//         request_type: bmRequestType::new(
//             Direction::Out,
//             DataTransferType::Vendor,
//             Recipient::Device,
//         ),
//         request: bRequest::DriverSpec(0x9A),
//         index: lcr as u16,
//         value: 0x2518 as u16,
//         data: None,
//     }),
// ));
// vec.push(URB::new(
//     self.device_slot_id,
//     RequestedOperation::Control(ControlTransfer {
//         request_type: bmRequestType::new(
//             Direction::Out,
//             DataTransferType::Vendor,
//             Recipient::Device,
//         ),
//         request: bRequest::DriverSpec(0xA4),
//         index: !mcr as u16,
//         value: 0 as u16,
//         data: None,
//     }),
// ));

// let mut rate: usize = 9600;
// let mut factor: u32 = (1532620800 / rate).try_into().unwrap();
// let mut divisor: u16 = 3;
// while (factor > 0xfff0) && (divisor > 0) {
//     factor >>= 3;
//     divisor -= 1;
// }
// if factor > 0xfff0 {
//     trace!("factor wrror");
// }
// factor = 0x10000 - factor;
// let mut a: u16 = (factor & 0xff00) as u16 | divisor;
// a |= 1 << 7;

// let mut lcr: u8 = 0x80 | 0x40;
// let nDataBits: u8 = 8;
// let nParity: u8 = 0;
// let nStopBits: u8 = 1;

// match nDataBits {
//     5 => lcr |= 0x00,
//     6 => lcr |= 0x01,
//     7 => lcr |= 0x02,
//     8 => lcr |= 0x03,
//     _ => (),
// }

// match nParity {
//     1 => lcr |= 0x08,
//     2 => lcr |= 0x08 | 0x10,
//     _ => (),
// }

// if nStopBits == 2 {
//     lcr |= 0x04;
// }

// vec.push(URB::new(
//     self.device_slot_id,
//     RequestedOperation::Control(ControlTransfer {
//         request_type: bmRequestType::new(
//             Direction::Out,
//             DataTransferType::Vendor,
//             Recipient::Device,
//         ),
//         request: bRequest::DriverSpec(0x9A),
//         index: a as u16,
//         value: 0x1312 as u16,
//         data: None,
//     }),
// ));
// vec.push(URB::new(
//     self.device_slot_id,
//     RequestedOperation::Control(ControlTransfer {
//         request_type: bmRequestType::new(
//             Direction::Out,
//             DataTransferType::Vendor,
//             Recipient::Device,
//         ),
//         request: bRequest::DriverSpec(0x9A),
//         index: lcr as u16,
//         value: 0x2518 as u16,
//         data: None,
//     }),
// ));
// self.baud_rate = rate;
// self.lcr = lcr;
// mcr |= (1 << 5) | (1 << 6);
// vec.push(URB::new(
//     self.device_slot_id,
//     RequestedOperation::Control(ControlTransfer {
//         request_type: bmRequestType::new(
//             Direction::Out,
//             DataTransferType::Vendor,
//             Recipient::Device,
//         ),
//         request: bRequest::DriverSpec(0xA4),
//         index: !mcr as u16,
//         value: 0 as u16,
//         data: None,
//     }),
// ));
// self.mcr = mcr;
