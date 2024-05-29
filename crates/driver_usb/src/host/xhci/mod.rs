#[cfg(feature = "phytium-xhci")]
pub mod vl805;

use axtask::{sleep, spawn, spawn_raw, yield_now};
use core::{borrow::BorrowMut, num::NonZeroUsize, task, time::Duration};
use log::{debug, error, info};
use xhci::{
    accessor::Mapper,
    extended_capabilities::debug::{self, EventRingDequeuePointer},
    registers::{
        doorbell::Register,
        operational::{ConfigureRegister, DeviceNotificationControl},
    },
    ExtendedCapability,
};

use crate::host::structures::{
    extended_capabilities, scratchpad, xhci_command_manager, xhci_event_manager,
    xhci_roothub::{self, Roothub, ROOT_HUB},
    xhci_slot_manager,
};

use super::structures::registers;

// const ARM_IRQ_PCIE_HOST_INTA: usize = 143 + 32; //todo figure out it's interrupt num
const ARM_IRQ_PCIE_HOST_INTA: usize = 48; //todo figure out it's interrupt num
const XHCI_CONFIG_MAX_EVENTS_PER_INTR: usize = 16;

#[derive(Clone, Copy)]
pub struct MemoryMapper;

impl Mapper for MemoryMapper {
    unsafe fn map(&mut self, phys_base: usize, bytes: usize) -> NonZeroUsize {
        // let virt = phys_to_virt(phys_base.into());
        // debug!("mapping: [{:x}]->[{:x}]", phys_base, virt.as_usize());
        // return NonZeroUsize::new_unchecked(virt.as_usize());
        return NonZeroUsize::new_unchecked(phys_base);
    }

    fn unmap(&mut self, virt_base: usize, bytes: usize) {}
}

pub(crate) fn init(mmio_base: usize) {
    unsafe {
        registers::init(mmio_base);
        extended_capabilities::init(mmio_base);
    };

    debug!("resetting xhci controller");
    debug!(
        "before reset:{:?},pagesize: {}",
        registers::handle(|r| r.operational.usbsts.read_volatile()),
        registers::handle(|r| r.operational.pagesize.read_volatile().get())
    );
    reset_xhci_controller();
    debug!(
        "pagesize: {}",
        registers::handle(|r| r.operational.pagesize.read_volatile().get())
    );

    xhci_slot_manager::new();
    scratchpad::new();
    xhci_command_manager::new();
    xhci_event_manager::new();
    xhci_roothub::new();

    // axhal::irq::register_handler(ARM_IRQ_PCIE_HOST_INTA, interrupt_handler);
    // axhal::irq::register_handler(ARM_IRQ_PCIE_HOST_INTA + 1, interrupt_handler);

    debug!(
        "before start:{:?}",
        registers::handle(|r| r.operational.usbsts.read_volatile())
    );
    registers::handle(|r| {
        r.operational.usbcmd.update_volatile(|r| {
            r.set_run_stop();
        });

        while r.operational.usbsts.read_volatile().hc_halted() {}
    });

    debug!(
        "init completed!, coltroller state:{:?}",
        registers::handle(|r| r.operational.usbsts.read_volatile())
    );

    let number_of_ports =
        registers::handle(|r| r.capability.hcsparams1.read_volatile().number_of_ports() as usize);

    for i in 0..number_of_ports {
        debug!(
            "port{i} : {}",
            registers::handle(|r| r
                .port_register_set
                .read_volatile_at(i)
                .portsc
                .current_connect_status())
        )
    }

    debug!("initializing roothub");
    ROOT_HUB.get().unwrap().lock().initialize(); //设备枚举
}

pub(crate) fn enum_device() {
    ROOT_HUB.get().unwrap().lock().initialize();
}

fn interrupt_handler() {
    debug!("interrupt!");
    registers::handle(|r| {
        r.operational.usbsts.update_volatile(|sts| {
            sts.clear_event_interrupt();
        });

        r.interrupter_register_set
            .interrupter_mut(0)
            .iman
            .update_volatile(|iman| {
                iman.clear_interrupt_pending();
            });

        if r.operational.usbsts.read_volatile().hc_halted() {
            error!("HC halted");
            return;
        }

        for tries in 0..XHCI_CONFIG_MAX_EVENTS_PER_INTR {
            if xhci_event_manager::handle_event().is_ok() {}
        }
    })
}

fn reset_xhci_controller() {
    registers::handle(|r| {
        debug!("stop");
        r.operational.usbcmd.update_volatile(|c| {
            c.clear_run_stop();
        });

        debug!("wait until halt");
        while !r.operational.usbsts.read_volatile().hc_halted() {}
        debug!("halted");

        debug!("HCRST!");
        r.operational.usbcmd.update_volatile(|c| {
            c.set_host_controller_reset();
        });

        while r.operational.usbcmd.read_volatile().host_controller_reset()
            || r.operational.usbsts.read_volatile().controller_not_ready()
        {}

        // debug!("get bios ownership");
        // for c in extended_capabilities::iter()
        //     .unwrap()
        //     .filter_map(Result::ok)
        // {
        //     if let ExtendedCapability::UsbLegacySupport(mut u) = c {
        //         let l = &mut u.usblegsup;
        //         l.update_volatile(|s| {
        //             s.set_hc_os_owned_semaphore();
        //         });

        //         while l.read_volatile().hc_bios_owned_semaphore()
        //             || !l.read_volatile().hc_os_owned_semaphore()
        //         {}
        //     }
        // }

        debug!("Reset xHCI Controller Globally");
    });
}
