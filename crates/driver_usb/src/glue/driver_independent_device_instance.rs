use core::hash::Hash;

use alloc::sync::Arc;
use spinlock::SpinNoIrq;

use crate::{
    abstractions::PlatformAbstractions,
    host::data_structures::{host_controllers::ControllerArc, MightBeInited},
    usb::descriptors::topological_desc::TopologicalUSBDescriptorRoot,
};
///This Structure would been hold Host Layer. Since Host Layer need to identify each Device that
///attached, and might had multiple controller support in future, here we go!
#[derive(Clone)]
pub struct DriverIndependentDeviceInstance<O>
where
    O: PlatformAbstractions,
{
    pub slotid: usize,
    pub configuration_val: usize,
    pub interface_val: usize,
    pub current_alternative_interface_value: usize,
    pub descriptors: Arc<MightBeInited<TopologicalUSBDescriptorRoot>>,
    pub controller: ControllerArc<O>,
}

impl<O> DriverIndependentDeviceInstance<O>
where
    O: PlatformAbstractions,
{
    pub fn new(slotid: usize, controller: ControllerArc<O>) -> Self {
        Self {
            slotid: slotid,
            descriptors: Arc::new(MightBeInited::default()),
            controller: controller,
            configuration_val: 1,
            interface_val: 0,
            current_alternative_interface_value: 0,
        }
    }
}
