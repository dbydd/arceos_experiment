use crate::host::structures::{
    dump_port_status, registers, reset_port, xhci_slot_manager::SLOT_MANAGER, XHCI_CONFIG_MAX_PORTS,
};

use super::{xhci_usb_device::XHCIUSBDevice, USBSpeed};

use core::mem::MaybeUninit;

use alloc::sync::Arc;
use log::{debug, error};
use spinning_top::Spinlock;

pub struct RootPort {
    pub(crate) root_port_id: usize,
    pub(crate) device: MaybeUninit<XHCIUSBDevice>,
    pub(crate) device_inited: bool,
}

impl RootPort {
    pub fn configure(&mut self) {
        unsafe { self.device.assume_init_mut().configure() }
    }

    pub fn initialize(&mut self) {
        if !self.connected() {
            error!("port {} not connected", self.root_port_id);
            return;
        }
        debug!("port {} connected, continue", self.root_port_id);

        //由于uboot已经探测过设备，因此设备的device context可能已被更改，因此我们比起普通的xhci驱动，还多了端口复位+设备复位的操作
        reset_port(self.root_port_id);
        dump_port_status(self.root_port_id);

        let get_speed = self.get_speed();
        if get_speed == USBSpeed::USBSpeedUnknown {
            error!("unknown speed, index:{}", self.root_port_id);
        }
        debug!("port speed: {:?}", get_speed);

        debug!("initializing device: {:?}", get_speed);

        if let Ok(mut device) = XHCIUSBDevice::new(self.root_port_id as u8) {
            debug!("writing ...");
            self.device_inited = true;
            unsafe { self.device.write(device) };
            debug!("writing complete");
        }

        unsafe { self.device.assume_init_mut().initialize() };
        debug!("initialize complete");
    }

    pub fn status_changed(&self) {
        // 检查MMIO（内存映射I/O），确保索引在有效范围内
        assert!(self.root_port_id < XHCI_CONFIG_MAX_PORTS);
        debug!("port {} status changed", self.root_port_id);
    }

    pub(crate) fn get_speed(&self) -> USBSpeed {
        registers::handle(|r| {
            r.port_register_set
                .read_volatile_at(self.root_port_id)
                .portsc
                .port_speed()
        })
        .into()
    }

    pub fn connected(&self) -> bool {
        registers::handle(|r| {
            r.port_register_set
                .read_volatile_at(self.root_port_id)
                .portsc
                .current_connect_status()
        })
    }
}
