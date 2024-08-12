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
use crate::driver_mio::mio_sinit::*;

pub fn FMIO_FUNC_STATE_MASK() -> u32{
    (((!0u32) - (1u32 << (0)) + 1) & (!0u32 >> (32 - 1 - (1))))
}

pub fn FMioSelectFunc(addr: usize, mio_type: u32) -> bool {
    assert!(mio_type < 2);
    assert!(addr != 0);

    let reg_val = input_32(addr as u32,0x04) & FMIO_FUNC_STATE_MASK();

    if mio_type == reg_val {
        return true;
    }

    output_32(addr as u32, 0x00,mio_type);

    true
}

pub fn FMioGetFunc(addr: usize) -> u32 {
    assert!(addr != 0);

    input_32(addr as u32,0x04) & FMIO_FUNC_STATE_MASK()
}

pub fn FMioGetVersion(addr: usize) -> u32 {
    assert!(addr != 0);

    input_32(addr as u32,0x100) & (((!0u32) - (1u32 << (0)) + 1) & (!0u32 >> (32 - 1 - (31))))
}



