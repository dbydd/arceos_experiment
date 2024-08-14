use core::sync::atomic::AtomicUsize;

use alloc::sync::Arc;
use log::trace;
use spinlock::{BaseSpinLock, SpinNoIrq};
use xhci::ring::trb::event;

use crate::PlatformAbstractions;

use super::{
    drivers::driverapi::{USBSystemDriverModule, USBSystemDriverModuleInstance},
    operation::{Configuration, Debugop, ExtraStep},
    trasnfer::{control::ControlTransfer, interrupt::InterruptTransfer, isoch::IsochTransfer},
};

#[derive(Clone)]
pub struct URB<'a, O>
where
    O: PlatformAbstractions,
{
    pub device_slot_id: usize,
    pub operation: RequestedOperation<'a>,
    pub sender: Option<Arc<SpinNoIrq<dyn USBSystemDriverModuleInstance<'a, O>>>>,
}

impl<'a, O> URB<'a, O>
where
    O: PlatformAbstractions,
{
    pub fn new(device_slot_id: usize, op: RequestedOperation<'a>) -> Self {
        Self {
            device_slot_id,
            operation: op.clone(),
            sender: None,
        }
    }

    pub fn set_sender(&mut self, sender: Arc<SpinNoIrq<dyn USBSystemDriverModuleInstance<'a, O>>>) {
        self.sender = Some(sender)
    }
}

#[derive(Debug, Clone)]
pub enum RequestedOperation<'a> {
    ExtraStep(ExtraStep),
    Control(ControlTransfer),
    Bulk,
    Interrupt(InterruptTransfer),
    Isoch(IsochTransfer),
    ConfigureDevice(Configuration<'a>),
    Debug(Debugop),
}
