use alloc::{boxed::Box, collections::binary_heap::Iter, sync::Arc, vec::Vec};
use data_structures::host_controllers::{xhci::XHCI, Controller, ControllerArc};
use log::{debug, trace};
use spinlock::SpinNoIrq;
use xhci::ring::trb::event;

use crate::{
    abstractions::PlatformAbstractions,
    err,
    glue::{driver_independent_device_instance::DriverIndependentDeviceInstance, ucb::UCB},
    usb::{self, operation::Configuration, transfer::control::ControlTransfer, urb::URB},
    USBSystemConfig,
};

/// Abstractions of Host Controller Operation Models
pub mod data_structures;

impl<O> USBSystemConfig<O>
where
    O: PlatformAbstractions,
{
    pub fn new(
        mmio_base_addr: usize,
        irq_num: u32,
        irq_priority: u32,
        transfer_ring_buffer_size: usize,
        os_dep: O,
    ) -> Self {
        let base_addr = O::VirtAddr::from(mmio_base_addr);
        Self {
            base_addr,
            irq_num,
            irq_priority,
            os: os_dep,
            transfer_ring_size: transfer_ring_buffer_size.min(O::PAGE_SIZE),
        }
    }
}

///USB HOST Controller Layer Instance
#[derive(Clone)]
pub struct USBHostSystem<O>
where
    O: PlatformAbstractions,
{
    config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
    controller: ControllerArc<O>,
}

impl<O> USBHostSystem<O>
where
    O: PlatformAbstractions + 'static,
{
    /// Create a new USB Host Controller Layer Instance
    pub fn new(config: Arc<SpinNoIrq<USBSystemConfig<O>>>) -> crate::err::Result<Self> {
        let controller = Arc::new(SpinNoIrq::new({
            let xhciregisters: Box<(dyn Controller<O> + 'static)> = {
                if cfg!(feature = "xhci") {
                    debug!("new xhci device host!");
                    Box::new(XHCI::new(config.clone()))
                } else {
                    panic!("no host controller defined")
                }
            };
            xhciregisters
        }));
        Ok(Self { config, controller })
    }

    ///initialze host controller hardware and data structs
    pub fn init(&self) {
        self.controller.lock().init();
        trace!("controller init complete");
    }

    /// probe for devices that attached, and do basic initialize.
    pub fn probe<F>(&self, consumer: F)
    where
        F: FnMut(DriverIndependentDeviceInstance<O>),
    {
        let mut probe = self.controller.lock().probe();
        probe
            .iter()
            .map(|slot_id| {
                DriverIndependentDeviceInstance::new(slot_id.clone(), self.controller.clone())
            })
            .for_each(consumer);
    }

    pub fn control_transfer(
        &mut self,
        dev_slot_id: usize,
        urb_req: ControlTransfer,
    ) -> crate::err::Result<UCB<O>> {
        self.controller
            .lock()
            .control_transfer(dev_slot_id, urb_req)
    }

    pub fn configure_device(
        &mut self,
        dev_slot_id: usize,
        urb_req: Configuration,
        dev: Option<&mut DriverIndependentDeviceInstance<O>>,
    ) -> crate::err::Result<UCB<O>> {
        self.controller
            .lock()
            .configure_device(dev_slot_id, urb_req, dev)
    }

    ///Receive a URB request and return a UCB. Once a time
    pub fn urb_request(
        &mut self,
        request: URB<O>,
        dev: &mut Vec<DriverIndependentDeviceInstance<O>>,
    ) -> crate::err::Result<UCB<O>> {
        trace!("urb request! {:?}", request);
        match request.operation {
            usb::urb::RequestedOperation::Control(control) => {
                trace!("request transfer!");
                self.control_transfer(request.device_slot_id, control)
            }
            usb::urb::RequestedOperation::Bulk => todo!("bulk has completely same request block as interrupt, so use interrupt urb instead for now"),
            usb::urb::RequestedOperation::Interrupt(interrupt_transfer) => self
                .controller
                .lock()
                .interrupt_transfer(request.device_slot_id, interrupt_transfer),
            usb::urb::RequestedOperation::Isoch(isoch_transfer) => self
                .controller
                .lock()
                .isoch_transfer(request.device_slot_id, isoch_transfer),
            usb::urb::RequestedOperation::ConfigureDevice(configure) => {
                self.controller.lock().configure_device(
                    request.device_slot_id,
                    configure,
                    dev.get_mut(request.device_slot_id - 1),
                )
            }
            usb::urb::RequestedOperation::ExtraStep(step) => self
                .controller
                .lock()
                .extra_step(request.device_slot_id, step),
            usb::urb::RequestedOperation::Debug(deb) => {
                self.controller.lock().debug_op(request.device_slot_id, deb)
            }
        }
    }

    ///tocK: execute all URB requests
    pub fn tock(
        &mut self,
        todo_list_list: Vec<Vec<URB<O>>>,
        dev: &mut Vec<DriverIndependentDeviceInstance<O>>,
    ) {
        trace!("tock! check deadlock!");
        todo_list_list.iter().for_each(|list| {
            list.iter().for_each(|todo| {
                if let Ok(ok) = self.urb_request(todo.clone(), dev)
                    && let Some(sender) = &todo.sender
                {
                    trace!("tock! check deadlock! 2");
                    sender.lock().receive_complete_event(ok);
                };
            })
        })
    }
}
