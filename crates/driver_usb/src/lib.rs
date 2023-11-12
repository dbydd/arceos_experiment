#![feature(prelude_2024)]
#![no_std]
mod usb_driver;

pub use usb_driver::*;

pub fn init() -> xhci::Registers<MemoryMapper> {
    usb_driver::init()
}
