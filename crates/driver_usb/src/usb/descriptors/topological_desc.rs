use alloc::vec::Vec;

use super::{
    desc_configuration::Configuration,
    desc_device::Device,
    desc_endpoint::Endpoint,
    desc_interface::{Interface, InterfaceAssociation},
    desc_uvc::uvc_endpoints::UVCVideoControlInterruptEndpoint,
    parser::ParserMetaData,
    USBDescriptor,
};

#[derive(Clone, Debug)]
pub struct TopologicalUSBDescriptorDevice {
    pub data: Device,
    pub child: Vec<TopologicalUSBDescriptorConfiguration>,
}

#[derive(Clone, Debug)]
pub struct TopologicalUSBDescriptorConfiguration {
    pub data: Configuration,
    pub child: Vec<TopologicalUSBDescriptorFunction>,
}

#[derive(Clone, Debug)]
pub enum TopologicalUSBDescriptorFunction {
    InterfaceAssociation((InterfaceAssociation, Vec<TopologicalUSBDescriptorFunction>)), //maybe we would have multi layer compose device in future? for now just treat it as a trick!
    Interface(
        Vec<(
            Interface,
            Vec<USBDescriptor>,
            Vec<TopologicalUSBDescriptorEndpoint>,
        )>,
    ),
}
#[derive(Clone, Debug)]
pub struct TopologicalUSBDescriptorRoot {
    pub device: Vec<TopologicalUSBDescriptorDevice>,
    pub others: Vec<USBDescriptor>,
    pub metadata: ParserMetaData,
}

#[derive(Clone, Debug)]
pub enum TopologicalUSBDescriptorEndpoint {
    Standard(Endpoint),
    UNVVideoControlInterruptEndpoint(UVCVideoControlInterruptEndpoint),
}
