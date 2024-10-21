use bit_field::BitField;
use driver_common::BaseDriverOps;
use memory_addr::VirtAddr;

use crate::{
    abstractions::{HALAbstractions, OSAbstractions, PlatformAbstractions},
    USBSystem,
};

pub type XHCIPCIDriver<'a> = USBSystem<'a, PlatAbstraction>;

#[derive(Clone, Debug)]
pub struct PlatAbstraction;

impl OSAbstractions for PlatAbstraction {
    type VirtAddr = VirtAddr;

    type DMA = alloc::alloc::Global; //todo: fix nocache allocator!

    const PAGE_SIZE: usize = 4096;

    fn dma_alloc(&self) -> Self::DMA {
        alloc::alloc::Global
    }
}

impl HALAbstractions for PlatAbstraction {
    fn force_sync_cache() {
        todo!()
    }
}

#[inline]
pub fn filter_xhci(class_id: u8, subclass_id: u8, prog_if: u8) -> bool {
    pci_types::device_type::DeviceType::from((class_id, subclass_id))
        == pci_types::device_type::DeviceType::UsbController
        && pci_types::device_type::UsbType::try_from(prog_if)
            .is_ok_and(|id| id == pci_types::device_type::UsbType::Xhci)
}

impl BaseDriverOps for XHCIPCIDriver<'_> {
    fn device_name(&self) -> &str {
        "xhci usb controller"
    }

    fn device_type(&self) -> driver_common::DeviceType {
        driver_common::DeviceType::USBHost
    }
}
