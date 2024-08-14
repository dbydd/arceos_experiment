use core::fmt::Debug;

use alloc::{sync::Arc, vec::Vec};
use spinlock::SpinNoIrq;
use xhci::ring::trb::event;

use crate::{
    abstractions::PlatformAbstractions,
    glue::{driver_independent_device_instance::DriverIndependentDeviceInstance, ucb::UCB},
    usb::urb::URB,
    USBSystemConfig,
};

pub trait USBSystemDriverModule<'a, O>: Send + Sync
where
    O: PlatformAbstractions,
{
    fn should_active(
        &self,
        independent_dev: &mut DriverIndependentDeviceInstance<O>,
        config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
    ) -> Option<Vec<Arc<SpinNoIrq<dyn USBSystemDriverModuleInstance<'a, O>>>>>;

    fn preload_module(&self);
}

pub trait USBSystemDriverModuleInstance<'a, O>: Send + Sync
where
    O: PlatformAbstractions,
{
    fn prepare_for_drive(&mut self) -> Option<Vec<URB<'a, O>>>;

    fn gather_urb(&mut self) -> Option<Vec<URB<'a, O>>>;

    fn receive_complete_event(&mut self, ucb: UCB<O>);
}
