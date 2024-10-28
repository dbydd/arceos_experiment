//! Defines types and probe methods of all supported devices.

#![allow(unused_imports)]

use core::{any::Any, time::Duration};

use crate::AxDeviceEnum;
use axalloc::{global_allocator, global_no_cache_allocator};
use axhal::time::busy_wait;
use cfg_if::cfg_if;
use driver_common::DeviceType;
use driver_pci::types::ConfigCommand;
use driver_usb::create_xhci_from_pci;
// use driver_usb::OsDep;

const VL805_VENDOR_ID: u16 = 0x1106;
const VL805_DEVICE_ID: u16 = 0x3483;

#[cfg(feature = "virtio")]
use crate::virtio::{self, VirtIoDevMeta};

#[cfg(feature = "bus-pci")]
use driver_pci::{types::ConfigSpace, DeviceFunction, DeviceFunctionInfo, PciAddress, PciRoot};

pub use super::dummy::*;

pub trait DriverProbe {
    fn probe_global() -> Option<AxDeviceEnum> {
        None
    }

    #[cfg(bus = "mmio")]
    fn probe_mmio(_mmio_base: usize, _mmio_size: usize) -> Option<AxDeviceEnum> {
        None
    }

    #[cfg(bus = "pci")]
    fn probe_pci(
        _root: &mut PciRoot,
        _bdf: DeviceFunction,
        _dev_info: &DeviceFunctionInfo,
        _config: &ConfigSpace,
    ) -> Option<AxDeviceEnum> {
        use driver_pci::types::ConfigSpace;

        None
    }
}

#[cfg(net_dev = "virtio-net")]
register_net_driver!(
    <virtio::VirtIoNet as VirtIoDevMeta>::Driver,
    <virtio::VirtIoNet as VirtIoDevMeta>::Device
);

#[cfg(block_dev = "virtio-blk")]
register_block_driver!(
    <virtio::VirtIoBlk as VirtIoDevMeta>::Driver,
    <virtio::VirtIoBlk as VirtIoDevMeta>::Device
);

#[cfg(display_dev = "virtio-gpu")]
register_display_driver!(
    <virtio::VirtIoGpu as VirtIoDevMeta>::Driver,
    <virtio::VirtIoGpu as VirtIoDevMeta>::Device
);

cfg_if::cfg_if! {
    if #[cfg(block_dev = "ramdisk")] {
        pub struct RamDiskDriver;
        register_block_driver!(RamDiskDriver, driver_block::ramdisk::RamDisk);

        impl DriverProbe for RamDiskDriver {
            fn probe_global() -> Option<AxDeviceEnum> {
                // TODO: format RAM disk
                Some(AxDeviceEnum::from_block(
                    driver_block::ramdisk::RamDisk::new(0x100_0000), // 16 MiB
                ))
            }
        }
    }
}

cfg_if::cfg_if! {
    if #[cfg(block_dev = "bcm2835-sdhci")]{
        pub struct BcmSdhciDriver;
        register_block_driver!(MmckDriver, driver_block::bcm2835sdhci::SDHCIDriver);

        impl DriverProbe for BcmSdhciDriver {
            fn probe_global() -> Option<AxDeviceEnum> {
                debug!("mmc probe");
                driver_block::bcm2835sdhci::SDHCIDriver::try_new().ok().map(AxDeviceEnum::from_block)
            }
        }
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature="pci-xhci")]{
       pub struct PciXHCIDriver;
       register_usb_host_driver!(PciXHCIDriver,driver_usb::XHCIPCIDriver<'static>);

       impl DriverProbe for PciXHCIDriver{

           fn probe_pci(
               root: &mut PciRoot,
               bdf: DeviceFunction,
               dev_info: &DeviceFunctionInfo,
               config: &ConfigSpace,
           ) -> Option<AxDeviceEnum> {
               use driver_pci::types::ConfigSpace;
               if driver_usb::filter_xhci(dev_info.class,dev_info.subclass,dev_info.prog_if) {
                   //info!("XHCI PCI device found at {:?}", bdf);

                   return match root.bar_info(bdf,0).unwrap() {
                       driver_pci::types::Bar::Memory32 { address, size, prefetchable } => {
                        config.header.set_command([
                            ConfigCommand::IoSpaceEnable,
                            ConfigCommand::MemorySpaceEnable,
                            ConfigCommand::BusMasterEnable,
                        ]);



                        busy_wait(Duration::from_millis(100));
                           let (interrupt_pin,mut interrupt_line) = root.interrupt_pin_line(bdf);
                            if interrupt_line == 0xff{
                                root.set_interrupt_pin_line(bdf, interrupt_pin, 9);
                                interrupt_line = 9;
                            }

                           create_xhci_from_pci(address as _, interrupt_line as _, 1)
                       },
                       driver_pci::types::Bar::Memory64 { address, size, prefetchable } => {
                        config.header.set_command([
                            ConfigCommand::IoSpaceEnable,
                            ConfigCommand::MemorySpaceEnable,
                            ConfigCommand::BusMasterEnable,
                        ]);
                        busy_wait(Duration::from_millis(100));
                           let (interrupt_pin,interrupt_line) = root.interrupt_pin_line(bdf);
                           create_xhci_from_pci(address as _, interrupt_line as _, 1)
                       },
                       driver_pci::types::Bar::Io { port } => {
                           error!("xhci: BAR0 is of I/O type");
                           None
                       },
                   }.map(|dev|AxDeviceEnum::from_usb(dev))

               }
               None
           }
       }
    }
}

cfg_if::cfg_if! {
    if #[cfg(net_dev = "ixgbe")] {
        use crate::ixgbe::IxgbeHalImpl;
        use axhal::mem::phys_to_virt;
        pub struct IxgbeDriver;
        register_net_driver!(IxgbeDriver, driver_net::ixgbe::IxgbeNic<IxgbeHalImpl, 1024, 1>);
        impl DriverProbe for IxgbeDriver {
            fn probe_pci(
                    root: &mut driver_pci::PciRoot,
                    bdf: driver_pci::DeviceFunction,
                    dev_info: &driver_pci::DeviceFunctionInfo,
                    _cfg: &ConfigSpace
                ) -> Option<crate::AxDeviceEnum> {
                    use crate::ixgbe::IxgbeHalImpl;
                    use driver_net::ixgbe::{INTEL_82599, INTEL_VEND, IxgbeNic};
                    if dev_info.vendor_id == INTEL_VEND && dev_info.device_id == INTEL_82599 {
                        // Intel 10Gb Network
                        info!("ixgbe PCI device found at {:?}", bdf);

                        // Initialize the device
                        // These can be changed according to the requirments specified in the ixgbe init function.
                        const QN: u16 = 1;
                        const QS: usize = 1024;
                        let bar_info = root.bar_info(bdf, 0).unwrap();
                        match bar_info {
                            driver_pci::BarInfo::Memory64 {
                                address,
                                size,
                                ..
                            } => {
                                let ixgbe_nic = IxgbeNic::<IxgbeHalImpl, QS, QN>::init(
                                    phys_to_virt((address as usize).into()).into(),
                                    size as usize
                                )
                                .expect("failed to initialize ixgbe device");
                                return Some(AxDeviceEnum::from_net(ixgbe_nic));
                            }
                            driver_pci::BarInfo::Memory32 {
                                address,
                                size,
                                ..
                            } => {
                                let ixgbe_nic = IxgbeNic::<IxgbeHalImpl, QS, QN>::init(
                                    phys_to_virt((address as usize).into()).into(),
                                    size as usize
                                )
                                .expect("failed to initialize ixgbe device");
                                return Some(AxDeviceEnum::from_net(ixgbe_nic));
                            }
                            driver_pci::BarInfo::Io { .. } => {
                                error!("ixgbe: BAR0 is of I/O type");
                                return None;
                            }
                        }
                    }
                    None
            }
        }
    }
}

// //todo maybe we should re arrange these code
// //------------------------------------------
// use axalloc::GlobalNoCacheAllocator;
// use driver_usb::ax::USBHostDriverOps;
// use driver_usb::host::xhci::Xhci;
// use driver_usb::host::USBHost;
// pub struct XHCIUSBDriver;

// #[derive(Clone)]
// pub struct OsDepImp;

// impl OsDep for OsDepImp {
//     const PAGE_SIZE: usize = axalloc::PAGE_SIZE;
//     type DMA = GlobalNoCacheAllocator;
//     fn dma_alloc(&self) -> Self::DMA {
//         axalloc::global_no_cache_allocator()
//     }

//     fn force_sync_cache() {
//         cfg_if::cfg_if! {
//             if #[cfg(usb_host_dev = "phytium-xhci")] {
//                 unsafe{
//                     core::arch::asm!("
//                     dc cisw
//                     ")
//                 }
//             }
//         }
//     }
// }

// cfg_match! {
//     cfg(usb_host_dev = "vl805")=>{
//         register_usb_host_driver!(XHCIUSBDriver, VL805<OsDepImp>);
//     }
//     _=>{
//         register_usb_host_driver!(XHCIUSBDriver, USBHost<OsDepImp>);
//     }
// }

// impl DriverProbe for XHCIUSBDriver {
//     #[cfg(bus = "pci")]
//     cfg_match! {
//         cfg(usb_host_dev = "vl805")=>{
//         use driver_usb::platform_spec::vl805::VL805;
//             fn probe_pci(
//                 root: &mut PciRoot,
//                 bdf: DeviceFunction,
//                 dev_info: &DeviceFunctionInfo,
//                 config: &ConfigSpace,
//             ) -> Option<AxDeviceEnum> {
//                 let osdep = OsDepImp {};
//                 VL805::probe_pci(config, osdep).map(|d| AxDeviceEnum::from_usb_host(d))
//             }
//         }
//         _=>{
//             fn probe_pci(
//                 root: &mut PciRoot,
//                 bdf: DeviceFunction,
//                 dev_info: &DeviceFunctionInfo,
//                 config: &ConfigSpace,
//             ) -> Option<AxDeviceEnum> {
//                 None
//             }
//         }
//     }
// }
// //------------------------------------------
