#![no_std]
#![no_main]
#![allow(warnings)]

use core::time::Duration;

use axalloc::GlobalNoCacheAllocator;
use axhal::{mem::VirtAddr, paging::PageSize, time::busy_wait};
use driver_usb::{USBSystem, USBSystemConfig};

#[macro_use]
extern crate axstd as std;

#[derive(Clone)]
struct PlatformAbstraction;

impl driver_usb::abstractions::OSAbstractions for PlatformAbstraction {
    type VirtAddr = VirtAddr;
    type DMA = GlobalNoCacheAllocator;

    const PAGE_SIZE: usize = PageSize::Size4K as usize;

    fn dma_alloc(&self) -> Self::DMA {
        axalloc::global_no_cache_allocator()
    }

    fn send_event(&self, event: driver_usb::abstractions::event::USBSystemEvent) {
        //println!("event:{:#?}",event);
    }
}

impl driver_usb::abstractions::HALAbstractions for PlatformAbstraction {
    fn force_sync_cache() {}
}

#[no_mangle]
fn main() {
    //comm
    let mut usbsystem = driver_usb::USBSystem::new({
        USBSystemConfig::new(0xffff_0000_31a0_8000, 48, 0, PlatformAbstraction)
    })
    .init();
    busy_wait(Duration::from_secs(5));
    usbsystem.init_probe().drive_all();
}
