use core::usize;

use axalloc::global_no_cache_allocator;
use axhal::mem::{phys_to_virt, virt_to_phys};
use bit_field::BitField;
use driver_common::BaseDriverOps;
use log::debug;
use memory_addr::VirtAddr;

use crate::{
    abstractions::{HALAbstractions, OSAbstractions, PlatformAbstractions},
    USBSystem, USBSystemConfig,
};

pub type XHCIPCIDriver<'a> = USBSystem<'a, PlatAbstraction>;

#[derive(Clone, Debug)]
pub struct PlatAbstraction;

impl OSAbstractions for PlatAbstraction {
    type VirtAddr = VirtAddr;

    type DMA = axalloc::GlobalNoCacheAllocator; //todo: fix nocache allocator!

    const PAGE_SIZE: usize = 4096;

    fn dma_alloc(&self) -> Self::DMA {
        global_no_cache_allocator()
    }

    fn map_virt_to_phys(vaddr: Self::VirtAddr) -> usize {
        virt_to_phys(vaddr).as_usize()
    }

    fn map_phys_to_virt(paddr: usize) -> Self::VirtAddr {
        phys_to_virt(paddr.into())
    }
}

impl HALAbstractions for PlatAbstraction {
    fn force_sync_cache() {
        use core::arch::asm;
        unsafe {}
    }
}

/// create a USB system from PCI device
pub fn create_xhci_from_pci<'a>(
    phys_address: usize,
    irq_num: usize,
    irq_priority: usize,
) -> Option<XHCIPCIDriver<'a>> {
    let phys_to_virt = phys_to_virt(phys_address.into());
    debug!("create xhci! addr:{:x}", phys_to_virt.as_usize());
    Some(XHCIPCIDriver::new(USBSystemConfig::new(
        phys_to_virt.as_usize(),
        irq_num as _,
        irq_priority as _,
        4096,
        PlatAbstraction {},
    )))
}

/// Filter xhci device from pci descriptors
/// return true if this is a xhci device
#[inline]
pub fn filter_xhci(class_id: u8, subclass_id: u8, prog_if: u8) -> bool {
    debug!("filter:class-{class_id},sub-{subclass_id},progif-{prog_if}");
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
