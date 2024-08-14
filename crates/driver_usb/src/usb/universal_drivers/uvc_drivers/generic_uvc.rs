use core::{fmt::Debug, mem::MaybeUninit};

use alloc::{collections::btree_map::BTreeMap, sync::Arc, vec, vec::Vec};
use log::trace;
use spinlock::SpinNoIrq;
use xhci::{context::EndpointType, ring::trb::transfer::Direction};

use crate::{
    abstractions::{dma::DMA, PlatformAbstractions},
    glue::driver_independent_device_instance::DriverIndependentDeviceInstance,
    host::data_structures::MightBeInited,
    usb::{
        descriptors::{
            self,
            desc_endpoint::Endpoint,
            desc_interface::Interface,
            desc_uvc::uvc_interfaces::{UVCInterface, UVCStreamingInterface},
            parser::ParserMetaData,
            topological_desc::{
                TopologicalUSBDescriptorEndpoint, TopologicalUSBDescriptorFunction,
                TopologicalUSBDescriptorRoot,
            },
            USBDescriptor,
        },
        drivers::driverapi::{USBSystemDriverModule, USBSystemDriverModuleInstance},
        operation::{Configuration, Debugop, ExtraStep},
        trasnfer::{
            control::{
                bmRequestType, ControlTransfer, DataTransferType, Recipient, StandardbRequest,
            },
            isoch::IsochTransfer,
        },
        universal_drivers::{BasicDriverLifeCycleStateMachine, BasicSendReceiveStateMachine},
        urb::{RequestedOperation, URB},
    },
    USBSystemConfig,
};

use super::{
    uvc_device_model::{
        UVCControlInterfaceModel, UVCControlInterfaceModelParser, UVCVSInterfaceModel,
    },
    uvc_spec_transfer::UVCSpecBRequest,
};

pub struct GenericUVCDriverModule; //TODO: Create annotations to register
pub struct GenericUVCDriver<O>
where
    O: PlatformAbstractions,
{
    device_slot_id: usize,
    config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
    interrupt_endpoints: Vec<TopologicalUSBDescriptorEndpoint>,
    isoch_endpoint: Option<usize>,
    uvc_control_model: UVCControlInterfaceModel,
    uvc_stream_model: UVCVSInterfaceModel,
    alternative_settings: BTreeMap<u32, Vec<(Interface, Endpoint)>>,
    interface_value: usize, //temporary place them here
    interface_alternative_value: usize,
    config_value: usize, // same
    send_receive_state: BasicSendReceiveStateMachine,
    lifecycle_machine: ExtraLifeCycle,
    receiption_buffer: Option<SpinNoIrq<DMA<[u8], O::DMA>>>,
}

impl<'a, O> USBSystemDriverModule<'a, O> for GenericUVCDriverModule
where
    O: PlatformAbstractions + 'static,
{
    fn should_active(
        &self,
        independent_dev: &mut DriverIndependentDeviceInstance<O>,
        config: Arc<SpinNoIrq<crate::USBSystemConfig<O>>>,
    ) -> Option<Vec<Arc<SpinNoIrq<dyn USBSystemDriverModuleInstance<'a, O>>>>> {
        if let MightBeInited::Inited(desc) = &*independent_dev.descriptors
            && let ParserMetaData::UVC(_) = desc.metadata
        {
            let device = desc.device.first().unwrap();

            independent_dev.interface_val = 1;
            independent_dev.current_alternative_interface_value = 0;
            Some(vec![GenericUVCDriver::new(
                independent_dev.slotid,
                config.clone(),
                {
                    device
                    .child
                    .iter()
                    .find(|c| c.data.config_val() == independent_dev.configuration_val as u8)
                    .expect("configuration not found")
                    .child
                    .iter()
                    .filter_map(|func| match func {
                        TopologicalUSBDescriptorFunction::InterfaceAssociation(function) => {
                            Some(function.1.iter().filter_map(|f| match f {
                                TopologicalUSBDescriptorFunction::InterfaceAssociation(_) => {
                                    panic!("currently, interface association cannot have association function child")
                                }
                                TopologicalUSBDescriptorFunction::Interface(func) => {
                                    Some(func)
                                },
                            }).flat_map(|a|a.clone()).collect::<Vec<_>>())
                        }
                        TopologicalUSBDescriptorFunction::Interface(_) => {
                            panic!("a uvc device is impossible had only one interface")
                        }
                    }).collect::<Vec<_>>()
                },
                independent_dev.interface_val,
                independent_dev.current_alternative_interface_value,
                independent_dev.configuration_val,
                independent_dev.descriptors.clone(),
            )])
        } else {
            None
        }
    }

    fn preload_module(&self) {
        trace!("loaded Generic UVC Driver Module!");
    }
}

impl<'a, O> GenericUVCDriver<O>
where
    O: PlatformAbstractions + 'static,
{
    pub fn new(
        device_slot_id: usize,
        config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
        function: Vec<
            Vec<(
                Interface,
                Vec<USBDescriptor>,
                Vec<TopologicalUSBDescriptorEndpoint>,
            )>,
        >,
        interface_value: usize,
        alternative_val: usize,
        config_value: usize,
        descriptors: Arc<MightBeInited<TopologicalUSBDescriptorRoot>>,
    ) -> Arc<SpinNoIrq<dyn USBSystemDriverModuleInstance<'a, O>>> {
        let uvccontrol_interface_model = function
            .iter()
            .find_map(|a| {
                a.iter().find(|b| {
                    b.1.iter().any(|interface| {
                        if let USBDescriptor::UVCInterface(UVCInterface::Control(_)) = interface {
                            true
                        } else {
                            false
                        }
                    })
                })
            })
            .map(
                |control: &(
                    Interface,
                    Vec<USBDescriptor>,
                    Vec<TopologicalUSBDescriptorEndpoint>,
                )| UVCControlInterfaceModelParser::new(control).parse(),
            )
            .expect("no control interface exist, is this broken?");

        let uvc_stream_interface_model = function
            .iter()
            .find_map(|a| {
                a.iter().find(|b| {
                    b.1.iter().any(|interface| {
                        if let USBDescriptor::UVCInterface(UVCInterface::Streaming(_)) = interface {
                            true
                        } else {
                            false
                        }
                    })
                })
            })
            .map(
                |control: &(
                    Interface,
                    Vec<USBDescriptor>,
                    Vec<TopologicalUSBDescriptorEndpoint>,
                )| UVCVSInterfaceModel::new(control),
            )
            .expect("no streaming interface exist, is this broken?");

        let mut alternative_interface_endpoint: BTreeMap<u32, Vec<(Interface, Endpoint)>> =
            BTreeMap::new();

        function
            .iter()
            .filter_map(|a| {
                a.iter().find(|(i, o, e)| {
                    o.is_empty() //yeah, this is a special point of uvc
                })
            })
            .for_each(|(interface, _, endpoints)| {
                endpoints
                    .iter()
                    .filter_map(|e| {
                        if let TopologicalUSBDescriptorEndpoint::Standard(ep) = e {
                            Some(ep)
                        } else {
                            None
                        }
                    })
                    .for_each(|ep| {
                        alternative_interface_endpoint
                            .entry(ep.doorbell_value_aka_dci())
                            .or_insert(Vec::new())
                            .push((interface.clone(), ep.clone()))
                    })
            });

        // trace!("goted function:{:#?}", function);
        Arc::new(SpinNoIrq::new(Self {
            config: config.clone(),
            interrupt_endpoints: function
                .iter()
                .flat_map(|a| {
                    a.iter().flat_map(|b| {
                        b.2.iter()
                            .filter(|tep| {
                                match tep {
                            TopologicalUSBDescriptorEndpoint::Standard(ep)
                                if let EndpointType::InterruptIn = ep.endpoint_type() =>
                            {
                                true
                            }
                            TopologicalUSBDescriptorEndpoint::UNVVideoControlInterruptEndpoint(
                                any,
                            ) => true,
                            _ => false,
                        }
                            })
                            .map(|a| a.clone())
                    })
                })
                .collect(),
            interface_value,
            interface_alternative_value: alternative_val,
            config_value,
            send_receive_state: BasicSendReceiveStateMachine::Sending,
            lifecycle_machine: ExtraLifeCycle::STDWorking(
                BasicDriverLifeCycleStateMachine::BeforeFirstSendAkaPreparingForDrive,
            ),
            device_slot_id,
            uvc_control_model: uvccontrol_interface_model,
            uvc_stream_model: uvc_stream_interface_model,
            alternative_settings: alternative_interface_endpoint,
            receiption_buffer: Some(SpinNoIrq::new(DMA::new_vec(
                0u8,
                O::PAGE_SIZE,
                O::PAGE_SIZE,
                config.clone().lock().os.dma_alloc(),
            ))),
            isoch_endpoint: None,
        }))
    }

    pub fn determine_stream_interface(&mut self) {
        // self.alternative_settings
        //     .iter()
        //     .filter_map(|(id, alternatives)| {
        //         alternatives
        //             .iter()
        //             .map(|(int, endp)| (id, int, endp))
        //             .min_by(|(_, _, e1), (_, _, e2)| {
        //                 let max_packet_size1 = e1.max_packet_size;
        //                 let max_packet_size2 = e2.max_packet_size;
        //                 max_packet_size1.cmp(&max_packet_size2)
        //             })
        //     })
        //     .last()
        //     .inspect(|(dci, interface, endpoint)| {
        //         trace!("founded!{:#?},{:#?}", interface, endpoint);
        //         self.interface_value = interface.interface_number as _;
        //         self.interface_alternative_value = interface.alternate_setting as _;
        //     });

        self.alternative_settings
            .iter()
            .find_map(|(id, alternatives)| {
                alternatives
                    .iter()
                    .map(|(int, endp)| (id, int, endp))
                    .find(|(_, _, e1)| (e1.max_packet_size & 0x7ff) == 1024)
            })
            .inspect(|(dci, interface, endpoint)| {
                trace!("founded!{:#?},{:#?}", interface, endpoint);
                self.interface_value = interface.interface_number as _;
                self.interface_alternative_value = interface.alternate_setting as _;
                self.isoch_endpoint = Some(**dci as _);
            });
    }
}

impl<'a, O> USBSystemDriverModuleInstance<'a, O> for GenericUVCDriver<O>
where
    O: PlatformAbstractions + 'static,
{
    fn prepare_for_drive(&mut self) -> Option<Vec<crate::usb::urb::URB<'a, O>>> {
        // todo!();

        let mut todo_list = Vec::new();

        // todo_list.push(URB::new(
        //     self.device_slot_id,
        //     RequestedOperation::Control(ControlTransfer {
        //         request_type: bmRequestType::new(
        //             Direction::Out,
        //             DataTransferType::Class,
        //             Recipient::Interface,
        //         ),
        //         request: UVCSpecBRequest::SET_CUR.into(),
        //         index: (self.interface_value as u8) as u16,
        //         value: 1u16 << 8 | 0b00000000u16,
        //         data: todo!(),
        //     }),
        // ));

        // todo_list.push(URB::new(
        //     self.device_slot_id,
        //     RequestedOperation::Debug(Debugop::DumpDevice),
        // ));

        self.determine_stream_interface();

        // self.alternative_settings.keys().for_each(|dci| {
        //     todo_list.push(URB::new(
        //         self.device_slot_id,
        //         RequestedOperation::ExtraStep(ExtraStep::PrepareForTransfer(*dci as _)),
        //     ))
        // });

        // // todo_list.push(URB::new(
        // //     self.device_slot_id,
        // //     RequestedOperation::Debug(Debugop::DumpDevice),
        // // ));

        // todo_list.push(URB::new(
        //     self.device_slot_id,
        //     RequestedOperation::Control(ControlTransfer {
        //         request_type: bmRequestType::new(
        //             Direction::Out,
        //             DataTransferType::Standard,
        //             Recipient::Device,
        //         ),
        //         request: StandardbRequest::SetConfiguration.into(),
        //         index: 0,
        //         value: 1,
        //         data: None,
        //     }),
        // ));

        todo_list.push(URB::new(
            self.device_slot_id,
            RequestedOperation::ConfigureDevice(Configuration::SwitchInterface(
                self.interface_value,
                self.interface_alternative_value,
            )),
        ));

        // todo_list.push(URB::new(
        //     self.device_slot_id,
        //     RequestedOperation::Control(ControlTransfer {
        //         request_type: bmRequestType::new(
        //             Direction::Out,
        //             DataTransferType::Class,
        //             Recipient::Interface,
        //         ),
        //         request: UVCSpecBRequest::SET_CUR.into(),
        //         index: (self.interface_value as u8) as u16,
        //         value: 1u16 << 8 | 0b00000000u16,
        //         data: todo!(),
        //     }),
        // ));

        // if let Some(ReportDescState::Binary(buf)) = &self.report_descriptor {
        //     todo_list.push(URB::new(
        //         self.device_slot_id,
        //         RequestedOperation::Control(ControlTransfer {
        //             request_type: bmRequestType::new(
        //                 Direction::In,
        //                 DataTransferType::Standard,
        //                 Recipient::Interface,
        //             ),
        //             request: bRequest::GetDescriptor,
        //             index: self.interface_alternative_value as u16,
        //             value: crate::usb::descriptors::construct_control_transfer_type(
        //                 HIDDescriptorTypes::HIDReport as u8,
        //                 0,
        //             )
        //             .bits(),
        //             data: Some({ buf.lock().addr_len_tuple() }),
        //         }),
        //     ));
        // }

        // self.interrupt_in_channels
        //     .iter()
        //     .chain(self.interrupt_out_channels.iter())
        //     .for_each(|dci| {
        //         todo_list.push(URB::new(
        //             self.device_slot_id,
        //             RequestedOperation::ExtraStep(ExtraStep::PrepareForTransfer(*dci as _)),
        //         ));
        //     });

        Some(todo_list)
    }

    fn gather_urb(&mut self) -> Option<Vec<crate::usb::urb::URB<'a, O>>> {
        if let Some(buf) = &self.receiption_buffer {
            let mut test = Vec::new();
            // todo!() //试试直接从端口获取？
            test.push(URB::new(
                self.device_slot_id,
                RequestedOperation::Isoch(IsochTransfer {
                    endpoint_id: 3,
                    buffer_addr_len: buf.lock().addr_len_tuple(),
                    request_times: 3,
                    packet_size: 800,
                }),
            ));
            Some(test)
        } else {
            None
        }
    }

    fn receive_complete_event(&mut self, ucb: crate::glue::ucb::UCB<O>) {
        trace!("received ucb:{:#?}", ucb.code)
    }
}

enum ExtraLifeCycle {
    STDWorking(BasicDriverLifeCycleStateMachine),
    ConfigureCS(u16),
}
