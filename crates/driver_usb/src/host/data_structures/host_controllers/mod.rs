pub mod xhci;
use core::sync::atomic::{fence, Ordering};

use ::xhci::{
    context::{EndpointType, Input},
    ring::trb::{command, event},
};
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use log::trace;
use spinlock::SpinNoIrq;

use crate::{
    abstractions::{dma::DMA, OSAbstractions, PlatformAbstractions},
    err::Result,
    glue::{driver_independent_device_instance::DriverIndependentDeviceInstance, ucb::UCB},
    usb::{
        operation::{Configuration, Debugop, ExtraStep},
        transfer::{control::ControlTransfer, interrupt::InterruptTransfer, isoch::IsochTransfer},
    },
    USBSystemConfig,
};

pub trait Controller<O>: Send
where
    O: PlatformAbstractions,
{
    fn new(config: Arc<SpinNoIrq<USBSystemConfig<O>>>) -> Self
    where
        Self: Sized;

    fn init(&mut self);
    fn probe(&mut self) -> Vec<usize>;
    fn control_transfer(
        &mut self,
        dev_slot_id: usize,
        urb_req: ControlTransfer,
    ) -> crate::err::Result<UCB<O>>;

    fn interrupt_transfer(
        &mut self,
        dev_slot_id: usize,
        urb_req: InterruptTransfer,
    ) -> crate::err::Result<UCB<O>>;

    fn isoch_transfer(
        &mut self,
        dev_slot_id: usize,
        urb_req: IsochTransfer,
    ) -> crate::err::Result<UCB<O>>;

    fn configure_device(
        &mut self,
        dev_slot_id: usize,
        urb_req: Configuration,
        dev: Option<&mut DriverIndependentDeviceInstance<O>>,
    ) -> crate::err::Result<UCB<O>>;

    fn extra_step(&mut self, dev_slot_id: usize, urb_req: ExtraStep) -> crate::err::Result<UCB<O>>;

    fn device_slot_assignment(&mut self) -> usize;
    fn address_device(&mut self, slot_id: usize, port_id: usize);
    fn control_fetch_control_point_packet_size(&mut self, slot_id: usize) -> u8;
    fn set_ep0_packet_size(&mut self, dev_slot_id: usize, max_packet_size: u16);

    fn debug_op(&mut self, dev_slot_id: usize, debug_op: Debugop) -> crate::err::Result<UCB<O>>;
}

pub(crate) type ControllerArc<O> = Arc<SpinNoIrq<Box<dyn Controller<O>>>>;
