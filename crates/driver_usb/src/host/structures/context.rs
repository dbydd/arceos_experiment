use core::ops::Add;

use super::registers;
use alloc::{boxed::Box, sync::Arc};
use axalloc::{global_no_cache_allocator, GlobalNoCacheAllocator};
use axhal::mem::{PhysAddr, VirtAddr};
use log::{debug, error};
use page_box::PageBox;
use spinning_top::Spinlock;
use xhci::context::{
    Device32Byte, Device64Byte, DeviceHandler, EndpointHandler, Input32Byte, Input64Byte,
    InputControlHandler, InputHandler, SlotHandler,
};

pub(crate) struct Context {
    pub(crate) input: Input,
    pub(crate) output: Device,
}
impl Default for Context {
    fn default() -> Self {
        let mut context = Self {
            input: Input::default(),
            output: Device::default(),
        };
        context
    }
}

#[derive(Debug)]
pub(crate) enum Input {
    Byte64(PageBox<Input64Byte>),
    Byte32(PageBox<Input32Byte>),
}
impl Input {
    pub(crate) fn control_mut(&mut self) -> &mut dyn InputControlHandler {
        match self {
            Self::Byte32(b32) => b32.control_mut(),
            Self::Byte64(b64) => b64.control_mut(),
        }
    }

    pub(crate) fn device_mut(&mut self) -> &mut dyn DeviceHandler {
        match self {
            Self::Byte32(b32) => b32.device_mut(),
            Self::Byte64(b64) => b64.device_mut(),
        }
    }

    pub(crate) fn virt_addr(&self) -> VirtAddr {
        let virt_addr = match self {
            Self::Byte32(b32) => b32.virt_addr(),
            Self::Byte64(b64) => b64.virt_addr(),
        };

        return virt_addr;
    }
}
impl Default for Input {
    fn default() -> Self {
        if csz() {
            Self::Byte64({
                let mut into: PageBox<Input64Byte> = Input64Byte::new_64byte().into();
                into
            })
        } else {
            Self::Byte32(Input32Byte::default().into())
        }
    }
}

#[derive(Debug)]
pub(crate) enum Device {
    Byte64(PageBox<Device64Byte>),
    Byte32(PageBox<Device32Byte>),
}
impl Default for Device {
    fn default() -> Self {
        if csz() {
            Self::Byte64(Device64Byte::default().into())
        } else {
            Self::Byte32(Device32Byte::default().into())
        }
    }
}

impl Device {
    pub fn virt_addr(&self) -> VirtAddr {
        match self {
            Self::Byte32(b32) => b32.virt_addr(),
            Self::Byte64(b64) => b64.virt_addr(),
        }
    }

    pub fn ep(&mut self, dci: usize) -> &mut dyn EndpointHandler {
        match self {
            Self::Byte32(b32) => b32.endpoint_mut(dci),
            Self::Byte64(b64) => b64.endpoint_mut(dci),
        }
    }
    pub fn slot(&mut self) -> &mut dyn SlotHandler {
        match self {
            Self::Byte32(b32) => b32.slot_mut(),
            Self::Byte64(b64) => b64.slot_mut(),
        }
    }
}

fn csz() -> bool {
    registers::handle(|r| r.capability.hccparams1.read_volatile().context_size())
    // false
}
