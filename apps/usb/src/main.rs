#![no_std]
#![no_main]
#![allow(warnings)]

// #[macro_use]
// extern crate axstd as std;
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

use axusb_host::abstractions::{PlatformAbstractions, USBSystemConfig, WakeMethod};

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

    const RING_BUFFER_SIZE: usize = 512;

    fn dma_alloc(&self) -> Self::DMA {
        axalloc::global_no_cache_allocator()
    }
}

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    let sem = Arc::new(Semaphore::new(1));

    let usbsystem = axusb_host::USBSystem::new(USBSystemConfig {
        base_addr: todo!(),
        wake_method: WakeMethod::Timer(sem.clone()),
        os: OSA,
    });

    axstd::thread::spawn(move || loop {
        axstd::thread::sleep(Duration::from_millis(50));
        sem.add_permits(1);
    });

    usbsystem
        .stage_1_start_controller()
        .stage_2_initialize_usb_layer()
        .block_stage_3()
        .block_run();
    panic!("okay?")
}
