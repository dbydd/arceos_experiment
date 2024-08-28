use super::descriptors::{
    desc_endpoint::Endpoint, topological_desc::TopologicalUSBDescriptorConfiguration,
};

#[derive(Debug, Clone)]
pub enum Configuration<'a> {
    SetupDevice(&'a TopologicalUSBDescriptorConfiguration),
    SwitchInterface(InterfaceNumber, AltnativeNumber),
    SwitchConfig(ConfigurationID, InterfaceNumber),
    ReEnableEndpoint(Endpoint),
    DisableEndpoint(EndpointIndex),
}

pub type ConfigurationID = usize;
pub type InterfaceNumber = usize;
pub type AltnativeNumber = usize;

#[derive(Debug, Clone)]
pub enum ExtraStep {
    PrepareForTransfer(EndpointIndex),
}
pub type EndpointIndex = usize;

#[derive(Debug, Clone)]
pub enum Debugop {
    DumpDevice,
    DumpConfigAndInterface,
}
