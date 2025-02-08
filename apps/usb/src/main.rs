#![no_std]
#![no_main]
#![allow(warnings)]

#[macro_use]
extern crate axstd as std;
extern crate alloc;

use core::{alloc::GlobalAlloc, time::Duration};

// use std::thread;

use alloc::sync::Arc;
use async_lock::Semaphore;
use axalloc::GlobalNoCacheAllocator;
use axhal::{
    mem::{phys_to_virt, virt_to_phys, PhysAddr, VirtAddr},
    paging::PageSize,
};

use axusb_host::abstractions::{PlatformAbstractions, SystemWordWide, USBSystemConfig, WakeMethod};
use lazy_static::lazy_static;

#[derive(Clone)]
pub struct DummyVA(VirtAddr);
#[derive(Clone)]
pub struct DummyPA(PhysAddr);

impl From<DummyPA> for DummyVA {
    fn from(value: DummyPA) -> Self {
        Self(phys_to_virt(value.0))
    }
}

impl From<DummyVA> for DummyPA {
    fn from(value: DummyVA) -> Self {
        Self(virt_to_phys(value.0))
    }
}

impl From<usize> for DummyPA {
    fn from(value: usize) -> Self {
        Self(PhysAddr::from(value))
    }
}

impl From<DummyPA> for usize {
    fn from(value: DummyPA) -> Self {
        value.0.as_usize()
    }
}

impl From<DummyVA> for usize {
    fn from(value: DummyVA) -> Self {
        value.0.as_usize()
    }
}

impl From<usize> for DummyVA {
    fn from(value: usize) -> Self {
        Self(VirtAddr::from(value))
    }
}

#[derive(Clone)]
pub struct OSA;
impl PlatformAbstractions for OSA {
    type VirtAddr = DummyVA;

    type PhysAddr = DummyPA;

    type DMA = GlobalNoCacheAllocator;

    const PAGE_SIZE: usize = PageSize::Size4K as usize;

    const RING_BUFFER_SIZE: usize = 512usize;

    fn dma_alloc(&self) -> Self::DMA {
        axalloc::global_no_cache_allocator()
    }

    const WORD: axusb_host::abstractions::SystemWordWide = SystemWordWide::X32;
}

lazy_static! {
    static ref sem: Arc<Semaphore> = Arc::new(Semaphore::new(1));
    static ref usbsystem: axusb_host::USBSystem<'static, OSA, 512> =
        axusb_host::USBSystem::new(USBSystemConfig {
            base_addr: 0xffff_0000_31a0_8000.into(),
            // wake_method: WakeMethod::Timer(sem.clone()),
            wake_method: WakeMethod::Yield,
            os: OSA,
        });
}

#[no_mangle]
// #[embassy_executor::main]
fn main() {
    //panic handler not found, but found when use thread::spawn, wtf...
    // axstd::thread::spawn(move || {
    //     loop {
    //         axstd::thread::sleep(Duration::from_millis(10));
    //         // sem.add_permits(1);
    //     }
    // });

    usbsystem
        .stage_1_start_controller()
        .stage_2_initialize_usb_layer()
        .block_run();
    // panic!("okay?")
}
