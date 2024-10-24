#[cfg_attr(feature = "dyn", path = "dyn.rs")]
#[cfg_attr(not(feature = "dyn"), path = "static.rs")]
mod imp;

use driver_common::{BaseDriverOps, DeviceType};

pub use imp::*;

#[cfg(feature = "pci-xhci")]
use crate::drivers::AxUSBHostDevice;

/// A unified enum that represents different categories of devices.
#[allow(clippy::large_enum_variant)]
pub enum AxDeviceEnum {
    /// Network card device.
    #[cfg(feature = "net")]
    Net(AxNetDevice),
    /// Block storage device.
    #[cfg(feature = "block")]
    Block(AxBlockDevice),
    /// Graphic display device.
    #[cfg(feature = "display")]
    Display(AxDisplayDevice),
    #[cfg(feature = "pci-xhci")]
    XHCI(AxUSBHostDevice),
}

impl BaseDriverOps for AxDeviceEnum {
    #[inline]
    #[allow(unreachable_patterns)]
    fn device_type(&self) -> DeviceType {
        match self {
            #[cfg(feature = "net")]
            Self::Net(_) => DeviceType::Net,
            #[cfg(feature = "block")]
            Self::Block(_) => DeviceType::Block,
            #[cfg(feature = "display")]
            Self::Display(_) => DeviceType::Display,
            #[cfg(feature = "pci-xhci")]
            Self::XHCI(_) => DeviceType::USBHost,
            _ => unreachable!(),
        }
    }

    #[inline]
    #[allow(unreachable_patterns)]
    fn device_name(&self) -> &str {
        match self {
            #[cfg(feature = "net")]
            Self::Net(dev) => dev.device_name(),
            #[cfg(feature = "block")]
            Self::Block(dev) => dev.device_name(),
            #[cfg(feature = "display")]
            Self::Display(dev) => dev.device_name(),
            #[cfg(feature = "pci-xhci")]
            Self::XHCI(dev) => dev.device_name(),
            _ => unreachable!(),
        }
    }
}
