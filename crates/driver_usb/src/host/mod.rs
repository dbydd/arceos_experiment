pub(crate) mod structures;
pub mod xhci;
use axhal::mem::PhysAddr;
use driver_common::BaseDriverOps;

/// The information of the graphics device.
#[derive(Debug, Clone, Copy)]
pub struct USBHostInfo {}

/// Operations that require a graphics device driver to implement.
pub trait USBHostDriverOps: BaseDriverOps {}
