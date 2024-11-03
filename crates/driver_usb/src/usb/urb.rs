use core::sync::atomic::AtomicUsize;

use alloc::sync::Arc;
use log::trace;
use spinlock::{BaseSpinLock, SpinNoIrq};
use xhci::ring::trb::event;

use crate::PlatformAbstractions;

use super::{
    drivers::driverapi::{USBSystemDriverModule, USBSystemDriverModuleInstance},
    operation::{Configuration, Debugop, ExtraStep},
    transfer::{control::ControlTransfer, interrupt::InterruptTransfer, isoch::IsochTransfer},
};

///USB Request Block: The USB Request Block (URB) is a data structure used by the USB driver to manage USB transfers. it Similar to URB in Linux, but not that mess.
#[derive(Clone)]
pub struct URB<'a, O>
where
    O: PlatformAbstractions,
{
    pub device_slot_id: usize,
    pub operation: RequestedOperation<'a>,
    pub sender: Option<Arc<SpinNoIrq<dyn USBSystemDriverModuleInstance<'a, O>>>>,
}

impl<'a, O> core::fmt::Debug for URB<'a, O>
where
    O: PlatformAbstractions,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("URB")
            .field("device_slot_id", &self.device_slot_id)
            .field("operation", &self.operation)
            .finish()
    }
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
///Operations in USB driver Requests, we use enhanced enum to record these requests.
#[derive(Debug, Clone)]
pub enum RequestedOperation<'a> {
    ///Some ExtraSteps. eg. Fill Noop into transfer queue to flip cycle state
    ExtraStep(ExtraStep),
    /// Control Transfer: 1 of 4 types of transfer in usb
    Control(ControlTransfer),
    /// Bulk Transfer: 1 of 4 types of transfer in usb, basiclly Completely same as Inerrupt Transfer , useage of transaction that is not very strict about latency, and had huge data size
    Bulk,
    ///Interrupt Transfer: 1 of 4 types of transfer in usb, use if you want to transfer data with low latency, but not frequently
    Interrupt(InterruptTransfer),
    /// Isoch Transfer: 1 of 4 types of transfer in usb, use if you want to transfer data with low latency, and frequently with a lot of data
    Isoch(IsochTransfer),
    ///Device Instanse Related Operations.
    ConfigureDevice(Configuration<'a>),
    /// Debug Operations
    Debug(Debugop),
}
