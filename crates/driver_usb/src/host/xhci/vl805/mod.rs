use core::alloc::Allocator;
mod mailbox;
use crate::{
    device_types::PCI_DEVICE_ID_PHYTIUM_XHCI,
    dma::DMAVec,
    host::xhci::vl805::mailbox::{Mailbox, MsgNotifyXhciReset},
};
use driver_common::*;
use driver_pci::{
    device_types::{self, PCI_VENDOR_ID_PHYTIUM},
    types::{Bar, ConfigCommand, ConfigKind, ConfigSpace},
};
use log::debug;

const VL805_VENDOR_ID: u16 = 0x1106;
const VL805_DEVICE_ID: u16 = 0x3483;

pub struct VL805<A: Allocator + Clone> {
    alloc: A,
    // regs: Registers<MemoryMapper>,
    // extended_capabilities: Option<extended_capabilities::List<MemoryMapper>>,
    base_addr: usize,
}

impl<A: Allocator + Clone + Sync + Send> BaseDriverOps for VL805<A> {
    fn device_name(&self) -> &str {
        "VL805 4-Port USB 3.0 Host Controller"
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::USBHost
    }
}

impl<A: Allocator + Clone> VL805<A> {
    fn new(mmio_base: usize, alloc: A) -> Self {
        // let mapper = MemoryMapper;
        // let regs: xhci::Registers<MemoryMapper> =
        //     unsafe { xhci::Registers::new(mmio_base, mapper) };
        // let version = regs.capability.hciversion.read_volatile();
        // debug!("xhci version: {:x}", version.get());
        // let mut o = regs.operational;
        // debug!("xhci stat: {:?}", o.usbsts.read_volatile());

        // debug!("xhci wait for ready...");
        // while o.usbsts.read_volatile().controller_not_ready() {}
        // debug!("xhci ok");

        // o.usbcmd.update_volatile(|f| {
        //     f.set_host_controller_reset();
        // });

        // while o.usbcmd.read_volatile().host_controller_reset() {}

        // debug!("XHCI reset HC");
        super::init(mmio_base);
        VL805 {
            base_addr: mmio_base,
            alloc,
        }
    }
    pub fn probe_pci(config: &ConfigSpace, dma_alloc: A) -> Option<Self> {
        let (vendor_id, device_id) = config.header.vendor_id_and_device_id();
        if !(vendor_id as usize == PCI_VENDOR_ID_PHYTIUM && device_id == PCI_DEVICE_ID_PHYTIUM_XHCI)
        {
            return None;
        }
        debug!("found phytium xhci!");

        if let ConfigKind::Endpoint { inner } = &config.kind {
            let bar = inner.bar(0).unwrap();
            if let Bar::Memory64 {
                address,
                size,
                prefetchable,
            } = bar
            {
                // let mut dma: DMAVec<A, u8> = DMAVec::new(0x100, 0x1000, dma_alloc.clone());
                // let mbox = Mailbox::new();
                // let msg = MsgNotifyXhciReset {};
                // mbox.send(&msg, &mut dma);

                debug!("Phytium xhci @0x{:X}", address);
                config.header.set_command([
                    ConfigCommand::MemorySpaceEnable,
                    ConfigCommand::BusMasterEnable,
                    ConfigCommand::ParityErrorResponse,
                    ConfigCommand::SERREnable,
                ]);
                let vl805 = VL805::new(address as _, dma_alloc);
                return Some(vl805);
            }
        }

        None
    }
}
