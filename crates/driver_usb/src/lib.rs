//! Common traits and types for graphics display device drivers.

#![no_std]
#![feature(allocator_api)]
#![feature(strict_provenance)]
#![feature(get_mut_unchecked)]
#![feature(new_uninit)]
#![allow(warnings)]
#![feature(if_let_guard)]
#![feature(let_chains)]
#![feature(generic_arg_infer)]
#![feature(pointer_is_aligned_to)]

extern crate alloc;
pub(crate) mod dma;
pub mod host;
use core::alloc::Allocator;
mod device_types;

use axhal::mem::PhysAddr;
#[doc(no_inline)]
pub use driver_common::{BaseDriverOps, DevError, DevResult, DeviceType};
use futures_intrusive::sync::{GenericMutex, GenericMutexGuard};
use log::info;
use spinning_top::RawSpinlock;

pub(crate) type Futurelock<T> = GenericMutex<RawSpinlock, T>;
pub(crate) type FuturelockGuard<'a, T> = GenericMutexGuard<'a, RawSpinlock, T>;

use host::xhci::init;
pub fn try_init(mmio_base_paddr: usize) {
    // let vaddr = axhal::mem::phys_to_virt(PhysAddr::from(mmio_base_paddr));
    // init(vaddr.as_usize())
    init(0xffff_0000_31a0_8000 as usize)
}

pub fn enum_device() {
    host::xhci::enum_device();
}
