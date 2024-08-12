use core::{marker::PhantomData, time::Duration};

use alloc::{collections::BTreeMap, fmt::format, string::String, sync::Arc};
use alloc::{format, vec};
use axhal::cpu::this_cpu_id;
use axhal::{paging::PageSize, time::busy_wait};
use axtask::sleep_until;
use driver_common::BaseDriverOps;
use log::debug;
use num_traits::{FromPrimitive, ToPrimitive};
use spinlock::SpinNoIrq;
use xhci::extended_capabilities::debug;
use xhci::ring::trb::command;
use xhci::ring::trb::transfer::{self, Direction, Normal, TransferType};

use crate::host::xhci::ring::Ring;
use crate::{
    ax::USBDeviceDriverOps,
    dma::DMA,
    host::{
        usb::descriptors::{
            self,
            desc_device::{Device, USBDeviceClassCode},
            desc_hid::{Hid, USBHIDProtocolDescriptorType, USBHIDSubclassDescriptorType},
            DescriptionTypeIndexPairForControlTransfer, Descriptor, DescriptorType,
        },
        xhci::{xhci_device::DeviceAttached, Xhci},
    },
    OsDep,
};

const TAG: &str = "[USB-HID DRIVER]";
