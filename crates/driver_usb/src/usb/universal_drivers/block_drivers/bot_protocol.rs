use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec,
    vec::Vec,
};
use log::trace;
use num_traits::FromPrimitive;
use spinlock::SpinNoIrq;
use xhci::context::EndpointType;

use crate::{
    abstractions::PlatformAbstractions,
    glue::driver_independent_device_instance::DriverIndependentDeviceInstance,
    host::data_structures::MightBeInited,
    usb::{
        descriptors::{
            desc_device::StandardUSBDeviceClassCode,
            desc_endpoint::Endpoint,
            topological_desc::{
                TopologicalUSBDescriptorEndpoint, TopologicalUSBDescriptorFunction,
            },
        },
        drivers::driverapi::{USBSystemDriverModule, USBSystemDriverModuleInstance},
        universal_drivers::hid_drivers::USBHidDeviceSubClassProtocol,
    },
    USBSystemConfig,
};

use super::USBMassStorageSubclassCode;
const BulkOnlyTransportProtocol: u8 = 0x50;

pub enum BOTProtocolStateMachine {
    Init,
    Idle,
    Command,
    Error,
    Ready,
    Waiting,
    Shutdown,
}

pub enum BOTCommands {
    GetStatus,
    ClearFeature,
    MassStorageReset,
    GetMaxLUN,
    BulkOnlyTransport,
    Read10,
    Write10,
    Verify10,
    Read12,
    Write12,
    Verify12,
    Reserved,
}

pub struct USBMassBOTDeviceDriver<O>
where
    O: PlatformAbstractions,
{
    config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
    device_slot_id: usize,
    bulk_out_channels: Vec<u32>,
    bulk_in_channels: Vec<u32>,
    interface_value: u8, //temporary place them here
    interface_alternative_value: u8,
    config_value: usize, // same
    bot_device_state_machine: BOTProtocolStateMachine,
}

impl<'a, O> USBMassBOTDeviceDriver<O>
where
    O: PlatformAbstractions + 'static,
{
    pub fn new_and_init(
        device_slot_id: usize,
        endpoints: Vec<Endpoint>,
        config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
        interface_value: u8,
        alternative_val: u8,
        config_value: usize,
    ) -> Arc<SpinNoIrq<dyn USBSystemDriverModuleInstance<'a, O>>> {
        Arc::new(SpinNoIrq::new(Self {
            config,
            device_slot_id,
            bulk_out_channels: endpoints
                .iter()
                .filter_map(|e| match e.endpoint_type() {
                    _ => None,
                    EndpointType::BulkOut => Some(e.doorbell_value_aka_dci()),
                })
                .collect(),
            bulk_in_channels: endpoints
                .iter()
                .filter_map(|e| match e.endpoint_type() {
                    _ => None,
                    EndpointType::BulkIn => Some(e.doorbell_value_aka_dci()),
                })
                .collect(),
            interface_value,
            interface_alternative_value: alternative_val,
            config_value,
            bot_device_state_machine: BOTProtocolStateMachine::Init,
        }))
    }
}

impl<'a, O> USBSystemDriverModuleInstance<'a, O> for USBMassBOTDeviceDriver<O>
where
    O: PlatformAbstractions + 'static,
{
    fn prepare_for_drive(&mut self) -> Option<alloc::vec::Vec<crate::usb::urb::URB<'a, O>>> {
        trace!("prepare for block device drive!");
        todo!()
    }

    fn gather_urb(&mut self) -> Option<alloc::vec::Vec<crate::usb::urb::URB<'a, O>>> {
        todo!()
    }

    fn receive_complete_event(&mut self, ucb: crate::glue::ucb::UCB<O>) {
        todo!()
    }
}

pub struct USBMassBOTDeviceDriverModule;

impl<'a, O> USBSystemDriverModule<'a, O> for USBMassBOTDeviceDriverModule
where
    O: PlatformAbstractions + 'static,
{
    fn should_active(
        &self,
        independent_dev: &DriverIndependentDeviceInstance<O>,
        config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
    ) -> Option<Vec<Arc<SpinNoIrq<dyn USBSystemDriverModuleInstance<'a, O>>>>> {
        if let MightBeInited::Inited(inited) = &*independent_dev.descriptors {
            let device = inited.device.first().unwrap();
            return match (
                StandardUSBDeviceClassCode::from_u8(device.data.class),
                USBMassStorageSubclassCode::from_u8(device.data.protocol).unwrap(),
                device.data.subclass,
            ) {
                (
                    Some(StandardUSBDeviceClassCode::MassStorage),
                    USBMassStorageSubclassCode::SCSI_TransparentCommandSet,
                    BulkOnlyTransportProtocol,
                ) => Some(vec![USBMassBOTDeviceDriver::new_and_init(
                    independent_dev.slotid,
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
                            .filter_map(|e| {
                                if let TopologicalUSBDescriptorEndpoint::Standard(ep) = e {
                                    Some(ep)
                                } else {
                                    None
                                }
                            })
                            .collect()
                    },
                    config.clone(),
                    independent_dev.interface_val as _,
                    independent_dev.current_alternative_interface_value as _,
                    independent_dev.configuration_val,
                )]),
                (Some(StandardUSBDeviceClassCode::ReferInterfaceDescriptor), _, _) => Some({
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
                            crate::usb::descriptors::topological_desc::TopologicalUSBDescriptorFunction::InterfaceAssociation(_) => todo!("wtf, please fix meeeeeeeeee!"),
                            crate::usb::descriptors::topological_desc::TopologicalUSBDescriptorFunction::Interface(interfaces) => {
                                let (interface, additional,endpoints) = interfaces.get(0).expect("wtf, it should be here!");
                                if let (Some(StandardUSBDeviceClassCode::MassStorage),Some(USBMassStorageSubclassCode::SCSI_TransparentCommandSet),BulkOnlyTransportProtocol) = (StandardUSBDeviceClassCode::from_u8(interface.interface_class),USBMassStorageSubclassCode::from_u8(interface.interface_subclass),interface.interface_subclass){
                                    Some(USBMassBOTDeviceDriver::new_and_init(independent_dev.slotid,  endpoints.iter().filter_map(|e|if let TopologicalUSBDescriptorEndpoint::Standard(e) = e{
                                        Some(e.clone())
                                    }else {
                                        None
                                    }).collect(), config.clone(), interface.interface_number, interface.alternate_setting, independent_dev.configuration_val))
                                }else {
                                    None
                                }
                            },
                        }).collect();
                    collect
                }),
                _ => None,
            };
        } else {
            None
        }
    }

    fn preload_module(&self) {
        trace!("preloading MassStorage device driver!")
    }

    fn driver_name(&self) -> String {
        "block device (bot kind) driver".to_string()
    }
}
