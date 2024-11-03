use core::{fmt::Debug, mem::MaybeUninit, ops::DerefMut};

use alloc::{
    collections::{btree_map::BTreeMap, vec_deque::VecDeque},
    format,
    sync::Arc,
    vec,
    vec::Vec,
};
use log::{debug, trace};
use spinlock::SpinNoIrq;
use tock_registers::interfaces::Writeable;
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
        transfer::{
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
    uvc_spec_transfer::{
        Hint, StreamControlBlockVersionToSizeMap, UVCSpecBRequest, UVCStreamControlBlock,
    },
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
    control_blocks: VecDeque<SpinNoIrq<DMA<UVCStreamControlBlock, O::DMA>>>,
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

        function.last().unwrap().iter().for_each(|(i, o, es)| {
            es //uvc standard says the last group always stream interfaces
                .last()
                .and_then(|e| {
                    if let TopologicalUSBDescriptorEndpoint::Standard(ep) = e {
                        Some(ep)
                    } else {
                        None
                    }
                })
                .inspect(|last| {
                    if !alternative_interface_endpoint.contains_key(&last.doorbell_value_aka_dci())
                    {
                        alternative_interface_endpoint
                            .insert(last.doorbell_value_aka_dci(), Vec::new());
                    }

                    alternative_interface_endpoint
                        .get_mut(&last.doorbell_value_aka_dci())
                        .and_then(|v| Some(v.push((*i, **last))));
                });
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
            receiption_buffer: None,
            isoch_endpoint: None,
            control_blocks: VecDeque::new(),
        }))
    }

    pub fn determine_stream_interface(&mut self) -> Endpoint {
        self.alternative_settings
            .iter()
            .filter_map(|(id, alternatives)| {
                alternatives
                    .iter()
                    .map(|(int, endp)| (id, int, endp))
                    .min_by(|(_, _, e1), (_, _, e2)| {
                        let max_packet_size1 = e1.max_packet_size;
                        let max_packet_size2 = e2.max_packet_size;
                        max_packet_size1.cmp(&max_packet_size2)
                    })
            })
            .last()
            .inspect(|(dci, interface, endpoint)| {
                trace!("founded!{:#?},{:#?}", interface, endpoint);
                self.interface_value = interface.interface_number as _;
                self.interface_alternative_value = interface.alternate_setting as _;
                self.isoch_endpoint = Some(**dci as _)
            })
            .map(|(_, _, e)| e.clone())
            .unwrap()
    }

    fn select_stream_interface_manual(
        &mut self,
        interface_value: usize,
        alternative_val: usize,
    ) -> Endpoint {
        trace!("current alternatives:{:#?}", self.alternative_settings);
        self.alternative_settings
            .iter()
            .find_map(|(_, alternatives)| {
                alternatives.iter().find_map(|(intf, endp)| {
                    if intf.interface_number == interface_value as _
                        && intf.alternate_setting == alternative_val as _
                    {
                        trace!("found!");
                        self.interface_alternative_value = alternative_val;
                        self.interface_value = interface_value;
                        self.isoch_endpoint = Some(endp.doorbell_value_aka_dci() as _);
                        Some(endp)
                    } else {
                        None
                    }
                })
            })
            .map(|e| e.clone())
            .unwrap()
    }

    fn configure_buffer_by_index(&mut self, format_index: usize, frame_index: usize) {
        let get_frame_size = self
            .uvc_stream_model
            .get_frame_size_by_index(format_index, frame_index);
        self.receiption_buffer = Some(SpinNoIrq::new(DMA::new_vec(
            0u8,
            get_frame_size,
            O::PAGE_SIZE,
            self.config.lock().os.dma_alloc(),
        )))
    }
}

impl<'a, O> USBSystemDriverModuleInstance<'a, O> for GenericUVCDriver<O>
where
    O: PlatformAbstractions + 'static,
{
    fn prepare_for_drive(&mut self) -> Option<Vec<crate::usb::urb::URB<'a, O>>> {
        // todo!();

        let mut todo_list = Vec::new();

        {
            let control_block = {
                let mut ctrl_block =
                    DMA::create_uvcstream_control_block(self.config.lock().os.dma_alloc());

                ctrl_block.hint.set(1);
                ctrl_block.format_index = 2;
                ctrl_block.frame_index = 1;
                ctrl_block.frame_interval = 333333;
                ctrl_block.keyframe_rate = 0;
                ctrl_block.p_frame_rate = 0;
                ctrl_block.compress_quality = 0;
                ctrl_block.compress_window_size = 0;
                ctrl_block.delay = 0;
                ctrl_block.max_video_frame_size = 614400;
                ctrl_block.max_payload_transfer_size = 3072;

                let buffer = ctrl_block.to_buffer(StreamControlBlockVersionToSizeMap::Ver1p0);
                self.control_blocks.push_back(SpinNoIrq::new(ctrl_block));
                buffer
            };

            todo_list.push(URB::new(
                self.device_slot_id,
                RequestedOperation::Control(ControlTransfer {
                    request_type: bmRequestType::new(
                        Direction::Out,
                        DataTransferType::Class,
                        Recipient::Interface,
                    ),
                    request: crate::usb::transfer::control::bRequest::DriverSpec(
                        UVCSpecBRequest::SET_CUR as _,
                    ),
                    index: 1,
                    value: 256,
                    data: Some(control_block),
                    report: true,
                }),
            ))
        }

        {
            let find_map = self
                .interrupt_endpoints
                .iter()
                .find_map(|a| {
                    if let TopologicalUSBDescriptorEndpoint::Standard(ep) = a {
                        Some(ep)
                    } else {
                        None
                    }
                })
                .unwrap();

            todo_list.push(URB::new(
                self.device_slot_id,
                RequestedOperation::ConfigureDevice(Configuration::ReEnableEndpoint(
                    find_map.clone(),
                )),
            ));
        }

        todo_list.push(URB::new(
            self.device_slot_id,
            RequestedOperation::Control(ControlTransfer {
                request_type: bmRequestType::new(
                    Direction::Out,
                    DataTransferType::Standard,
                    Recipient::Device,
                ),
                request: StandardbRequest::SetConfiguration.into(),
                index: 0,
                value: 1,
                data: None,
                report: true,
            }),
        ));

        todo_list.push(URB::new(
            self.device_slot_id,
            RequestedOperation::ConfigureDevice(Configuration::SwitchInterface(1, 0)),
        ));

        let determined_endpoint = self.select_stream_interface_manual(1, 6);
        self.configure_buffer_by_index(2, 1);

        todo_list.push(URB::new(
            self.device_slot_id,
            RequestedOperation::ConfigureDevice(Configuration::ReEnableEndpoint(
                determined_endpoint,
            )),
        ));

        todo_list.push(URB::new(
            self.device_slot_id,
            RequestedOperation::ConfigureDevice(Configuration::SwitchInterface(
                self.interface_value,
                self.interface_alternative_value,
            )),
        ));

        Some(todo_list)
    }

    fn gather_urb(&mut self) -> Option<Vec<crate::usb::urb::URB<'a, O>>> {
        match self.lifecycle_machine {
            ExtraLifeCycle::STDWorking(
                BasicDriverLifeCycleStateMachine::BeforeFirstSendAkaPreparingForDrive,
            ) => {
                self.lifecycle_machine = ExtraLifeCycle::ConfigureCS;
                self.send_receive_state = BasicSendReceiveStateMachine::Waiting;
                self.control_blocks.pop_front();
                None
            }
            ExtraLifeCycle::ConfigureCS => {
                let mut todos = Vec::new();

                {
                    let buffer = {
                        let mut ctrl_block =
                            DMA::create_uvcstream_control_block(self.config.lock().os.dma_alloc());

                        ctrl_block.hint.set(1);
                        ctrl_block.format_index = 2;
                        ctrl_block.frame_index = 1;
                        ctrl_block.frame_interval = 333333;
                        ctrl_block.keyframe_rate = 0;
                        ctrl_block.p_frame_rate = 0;
                        ctrl_block.compress_quality = 0;
                        ctrl_block.compress_window_size = 0;
                        ctrl_block.delay = 0;
                        ctrl_block.max_video_frame_size = 0;
                        ctrl_block.max_payload_transfer_size = 0;

                        let buffer =
                            ctrl_block.to_buffer(StreamControlBlockVersionToSizeMap::Ver1p0);
                        self.control_blocks.push_back(SpinNoIrq::new(ctrl_block));
                        buffer
                    };

                    todos.push(URB::new(
                        self.device_slot_id,
                        RequestedOperation::Control(ControlTransfer {
                            request_type: bmRequestType::new(
                                Direction::Out,
                                DataTransferType::Class,
                                Recipient::Interface,
                            ),
                            request: crate::usb::transfer::control::bRequest::DriverSpec(
                                UVCSpecBRequest::SET_CUR as _,
                            ),
                            index: 1,
                            value: 256,
                            data: Some(buffer),
                            report: true,
                        }),
                    ))
                }

                {
                    let control_block = {
                        let mut ctrl_block =
                            DMA::create_uvcstream_control_block(self.config.lock().os.dma_alloc());

                        ctrl_block.hint.set(1);
                        ctrl_block.format_index = 2;
                        ctrl_block.frame_index = 1;
                        ctrl_block.frame_interval = 333333;
                        ctrl_block.keyframe_rate = 0;
                        ctrl_block.p_frame_rate = 0;
                        ctrl_block.compress_quality = 0;
                        ctrl_block.compress_window_size = 0;
                        ctrl_block.delay = 0;
                        ctrl_block.max_video_frame_size = 614400;
                        ctrl_block.max_payload_transfer_size = 3072;
                        ctrl_block.clock_frequency = 15000000;

                        let buffer =
                            ctrl_block.to_buffer(StreamControlBlockVersionToSizeMap::Ver1p0);
                        self.control_blocks.push_back(SpinNoIrq::new(ctrl_block));
                        buffer
                    };

                    todos.push(URB::new(
                        self.device_slot_id,
                        RequestedOperation::Control(ControlTransfer {
                            request_type: bmRequestType::new(
                                Direction::Out,
                                DataTransferType::Class,
                                Recipient::Interface,
                            ),
                            request: crate::usb::transfer::control::bRequest::DriverSpec(
                                UVCSpecBRequest::SET_CUR as _,
                            ),
                            index: 1,
                            value: 512,
                            data: Some(control_block),
                            report: true,
                        }),
                    ))
                }

                Some(todos)
            }
            ExtraLifeCycle::STDWorking(BasicDriverLifeCycleStateMachine::Driving) => {
                let mut todos = Vec::new();

                todos.push(URB::new(
                    self.device_slot_id,
                    RequestedOperation::Isoch(IsochTransfer {
                        endpoint_id: self.isoch_endpoint.unwrap(),
                        buffer_addr_len: {
                            self.receiption_buffer
                                .as_ref()
                                .map(|b| O::map_addr_len_tuple(b.lock().addr_len_tuple()))
                                .unwrap()
                        },
                        transfer_size_bytes: 614400,
                        max_packet: 32, //hardcode, refer linux
                    }),
                ));

                self.send_receive_state = BasicSendReceiveStateMachine::Waiting;

                Some(todos)
            }
            _ => None,
        }
    }

    fn receive_complete_event(&mut self, ucb: crate::glue::ucb::UCB<O>) {
        trace!(
            "received ucb:{:#?} at state: {:?}-{:?}",
            ucb.code,
            self.send_receive_state,
            self.lifecycle_machine
        );
        match self.send_receive_state {
            BasicSendReceiveStateMachine::Waiting => match self.lifecycle_machine {
                ExtraLifeCycle::STDWorking(
                    BasicDriverLifeCycleStateMachine::BeforeFirstSendAkaPreparingForDrive,
                ) => {}
                ExtraLifeCycle::STDWorking(BasicDriverLifeCycleStateMachine::Driving) => {
                    self.send_receive_state = BasicSendReceiveStateMachine::Sending;
                    self.receiption_buffer
                        .as_ref()
                        .inspect(|b| debug!("{:?}", b.lock().iter().max()));
                }
                ExtraLifeCycle::STDWorking(_) => {}
                ExtraLifeCycle::ConfigureCS => {
                    self.control_blocks.pop_front();
                    if self.control_blocks.is_empty() {
                        self.send_receive_state = BasicSendReceiveStateMachine::Sending;
                        self.lifecycle_machine =
                            ExtraLifeCycle::STDWorking(BasicDriverLifeCycleStateMachine::Driving)
                    }
                }
            },
            BasicSendReceiveStateMachine::Sending => {}
        }
    }
}

#[derive(Debug)]
enum ExtraLifeCycle {
    STDWorking(BasicDriverLifeCycleStateMachine),
    ConfigureCS,
}
