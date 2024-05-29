mod registers;
use core::usize;

use log::debug;
use virtio_drivers::PhysAddr;

use crate::{types::ConfigCommand, Access, PciAddress};

#[derive(Clone)]
pub struct PhytiumPCIeDummy {}

fn cfg_index(addr: PciAddress) -> usize {
    ((addr.device as u32) << 15 | (addr.function as u32) << 12 | (addr.bus as u32) << 20) as usize
}

//
// const EXT_CFG_INDEX: usize = 0x9000;
// const EXT_CFG_DATA: usize = 0x8000;

impl Access for PhytiumPCIeDummy {
    fn setup(mmio_base: usize) {
        debug!("PCIe link start @0x{:X}...", mmio_base);
        debug!(
            "theroticly, since uboot had already initialized it, we need't to operate it any more! \n or maybe we need to allocate bar addr!"
        )
    }

    fn probe_bridge(mmio_base: usize, bridge_header: &crate::types::ConifgPciPciBridge) {
        debug!(
            "probe phytium weird pcie bridge {}",
            bridge_header.get_secondary_bus_number()
        );

        // bridge_header.set_cache_line_size(64 / 4);
        // let limit =
        //     0x31000000u32 + (0x00020000u32 * bridge_header.get_secondary_bus_number() as u32);
        // let base = limit - 0x00020000u32;
        // //weird
        // bridge_header.set_memory_base((base >> 16) as u16); //理论上这玩意是要有定义的，但我没找到，先拿树莓派的顶着
        // bridge_header.set_memory_limit((limit >> 16) as u16); //这部分就是分配给下游设备的内存区域 //但是为啥这里设置为0？
        // bridge_header.set_control(0x01);

        // unsafe {
        //     (bridge_header.cfg_addr as *mut u8)
        //         .offset(0xac + 0x1c)
        //         .write_volatile(0x10);
        // }

        // bridge_header.to_header().set_command([
        //     ConfigCommand::MemorySpaceEnable,
        //     ConfigCommand::BusMasterEnable,
        //     ConfigCommand::ParityErrorResponse,
        //     ConfigCommand::SERREnable,
        // ])
    }

    fn map_conf(mmio_base: usize, addr: crate::PciAddress) -> Option<usize> {
        // if (addr.bus < 7 && addr.device > 0) {
        //     return None;
        // }

        // if addr.bus == 0 {
        //     return Some(mmio_base);
        // }

        let idx = cfg_index(addr);
        // unsafe {
        //     ((mmio_base + EXT_CFG_INDEX) as *mut u32).write_volatile(idx as u32);
        // }
        debug!(
            "mapconf 0x{:x}-{:?} to idx 0x{:X}",
            mmio_base,
            addr,
            mmio_base + idx
        );
        return Some(mmio_base + idx);

        return None;
    }
}
