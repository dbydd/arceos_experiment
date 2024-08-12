#![no_std]
#![no_main]
use super::driver_mio::{mio, mio_g, mio_hw, mio_sinit};
use super::{i2c, i2c_hw, i2c_intr, i2c_master, i2c_sinit, io};
use core::ptr;
use core::ptr::write_volatile;
use __private_api::Value;
use log::*;

use crate::driver_iic::i2c::*;
use crate::driver_iic::i2c_hw::*;
use crate::driver_iic::i2c_intr::*;
use crate::driver_iic::i2c_master::*;
use crate::driver_iic::i2c_sinit::*;


use crate::driver_mio::mio::*;
use crate::driver_mio::mio_g::*;
use crate::driver_mio::mio_hw::*;
use crate::driver_mio::mio_sinit::*;

pub fn write_reg(addr: u32, value: u32) {
    debug!("Writing value {:#X} to address {:#X}", value, addr);
    unsafe {
        *(addr as *mut u32) = value;
    }
}

pub fn read_reg(addr: u32) -> u32 {
    let value:u32;
    unsafe {
        value = *(addr as *const u32);
    }
    debug!("Read value {:#X} from address {:#X}", value, addr);
    value
}

pub fn input_32(addr: u32, offset: usize) -> u32 {
    let address: u32 = addr + offset as u32;
    read_reg(address)
}

pub fn output_32(addr: u32, offset: usize, value: u32) {
    let address: u32 = addr + offset as u32;
    write_reg(address, value);
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FIOPadConfig {
    pub instance_id: u32,    // 设备实例 ID
    pub base_address: usize, // 基地址
}

#[feature(const_trait_impl)]
#[derive(Debug, Clone, Copy, Default)]
pub struct FIOPadCtrl {
    pub config: FIOPadConfig, // 配置
    pub is_ready: u32,        // 设备是否准备好
}

pub static mut iopad_ctrl:FIOPadCtrl = FIOPadCtrl{
    config:FIOPadConfig{
        instance_id: 0,    
        base_address: 0, 
    },
    is_ready:0,
};


static FIO_PAD_CONFIG_TBL: [FIOPadConfig; 1] = [FIOPadConfig {
    instance_id: 0,
    base_address: 0x32B30000usize,
}];

pub fn FIOPadCfgInitialize(instance_p: &mut FIOPadCtrl, input_config_p: &FIOPadConfig) -> bool {
    assert!(Some(instance_p.clone()).is_some(), "instance_p should not be null");
    assert!(
        Some(input_config_p.clone()).is_some(),
        "input_config_p should not be null"
    );

    let mut ret: bool = true;

    if instance_p.is_ready == 0x11111111u32 {
        debug!("Device is already initialized.");
    }

    // Set default values and configuration data
    FIOPadDeInitialize(instance_p);

    instance_p.config = *input_config_p;

    instance_p.is_ready = 0x11111111u32;

    ret
}

pub fn FIOPadDeInitialize(instance_p: &mut FIOPadCtrl) -> bool {
    // 确保 `instance_p` 不为 null，类似于 C 中的 `FASSERT(instance_p)`
    if instance_p.is_ready == 0 {
        return true;
    }

    // 标记设备为未准备好
    instance_p.is_ready = 0;

    // 清空设备数据
    unsafe {
        core::ptr::write_bytes(instance_p as *mut FIOPadCtrl, 0, size_of::<FIOPadCtrl>());
    }

    true
}

pub fn FIOPadLookupConfig(instance_id: u32) -> Option<FIOPadConfig> {
    if instance_id as usize >= 1 {
        // 对应 C 代码中的 FASSERT 语句
        return None;
    }

    for config in FIO_PAD_CONFIG_TBL.iter() {
        if config.instance_id == instance_id {
            return Some(*config);
        }
    }

    None
}
