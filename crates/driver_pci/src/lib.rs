//! Structures and functions for PCI bus operations.
//!
//! Currently, it just re-exports structures from the crate [virtio-drivers][1]
//! and its module [`virtio_drivers::transport::pci::bus`][2].
//!
//! [1]: https://docs.rs/virtio-drivers/latest/virtio_drivers/
//! [2]: https://docs.rs/virtio-drivers/latest/virtio_drivers/transport/pci/bus/index.html

#![no_std]
#![allow(warnings)]
#![feature(cfg_match)]

// #[cfg(feature = "bcm2711")]
mod bcm2711;
extern crate alloc;
pub mod err;
mod root_complex;
pub mod types;
use core::ops::Range;

pub use root_complex::*;
use types::ConifgPciPciBridge;
// pub use virtio_drivers::transport::pci::bus::{BarInfo};

#[derive(Clone, Copy)]
pub struct PciAddress {
    pub bus: usize,
    pub device: usize,
    pub function: usize,
}
impl core::fmt::Display for PciAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:02x}:{:02x}.{}", self.bus, self.device, self.function)
    }
}

// cfg_match! {
//     cfg(feature = "bcm2711")=>{
//         pub type RootComplex = PciRootComplex<bcm2711::BCM2711>;
//     }
//     _=>{
//         struct DummyPciRoot;
//         pub type RootComplex = PciRootComplex<DummyPciRoot>;
//         impl Access for DummyPciRoot {
//             fn setup(mmio_base: usize) {}
//
//             fn probe_bridge(mmio_base: usize, bridge_header: &ConifgPciPciBridge) {}
//
//             fn map_conf(mmio_base: usize, addr: PciAddress) -> Option<usize> {
//                 None
//             }
//         }
//     }
// }
// #[cfg(feature = "bcm2711")]
pub type RootComplex = PciRootComplex<bcm2711::BCM2711>;

pub type PciRoot = RootComplex;
pub type DeviceFunction = PciAddress;
pub type BarInfo = types::Bar;

pub fn new_root_complex(mmio_base: usize, bar_range: Range<u64>) -> RootComplex {
    PciRootComplex::new(mmio_base, bar_range)
}

pub trait Access {
    fn setup(mmio_base: usize);
    fn probe_bridge(mmio_base: usize, bridge_header: &ConifgPciPciBridge);
    fn map_conf(mmio_base: usize, addr: PciAddress) -> Option<usize>;
}
