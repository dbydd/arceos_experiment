#![no_std]
#![no_main]
use log::*;
use axhal::time::busy_wait;
use core::time::Duration;
use super::driver_iic::{i2c_hw,i2c,i2c_sinit,i2c_master,io,i2c_intr};
use super::{mio_hw,mio_sinit,mio,mio_g};

use crate::driver_iic::i2c_hw::*;
use crate::driver_iic::i2c::*;
use crate::driver_iic::i2c_intr::*;
use crate::driver_iic::i2c_master::*;
use crate::driver_iic::i2c_sinit::*;
use crate::driver_iic::io::*;

use crate::driver_mio::mio::*;
use crate::driver_mio::mio_g::*;
use crate::driver_mio::mio_hw::*;


pub static FMIO_CONFIG_TBL: [FMioConfig; 3] = [
    FMioConfig {
        instance_id: 0,
        func_base_addr: 0x28014000,
        irq_num: 124,
        mio_base_addr: 0x28015000,
    },
    FMioConfig {
        instance_id: 1,
        func_base_addr: 0x28016000,
        irq_num: 125,
        mio_base_addr: 0x28017000,
    },
    FMioConfig {
        instance_id: 2,
        func_base_addr: 0x28018000,
        irq_num: 126,
        mio_base_addr: 0x28019000,
    },

];

pub fn FMioLookupConfig(instance_id: u32) -> Option<FMioConfig> {
    assert!(instance_id < 16);

    for config in FMIO_CONFIG_TBL.iter() {
        if config.instance_id == instance_id {
            return Some(*config);
        }
    }
    
    None
}