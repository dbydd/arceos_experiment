#![no_std]
#![no_main]
use axhal::time::busy_wait;
use core::default;
use core::ptr;
use core::slice;
use core::time::Duration;
use log::debug;
pub mod driver_iic;
pub mod driver_mio;
pub mod example;

use crate::driver_iic::i2c::*;
use crate::driver_iic::i2c_hw::*;
use crate::driver_iic::i2c_intr::*;
use crate::driver_iic::i2c_master::*;
use crate::driver_iic::i2c_sinit::*;
use crate::driver_iic::io::*;

use crate::driver_mio::mio::*;
use crate::driver_mio::mio_g::*;
use crate::driver_mio::mio_hw::*;
use crate::driver_mio::mio_sinit::*;

use crate::example::*;

const OLED_INIT_CMDS: [u8; 24] = [
    0xAE, // Display off
    0x00, // Set low column address
    0x10, // Set high column address
    0x40, // Set start line address
    0x81, // Set contrast control register
    0xFF, // Maximum contrast
    0xA1, // Set segment re-map
    0xA6, // Set normal display
    0xA8, // Set multiplex ratio
    0x3F, // 1/64 duty
    0xC8, // Set COM output scan direction
    0xD3, // Set display offset
    0x00, // No offset
    0xD5, // Set display clock divide ratio/oscillator frequency
    0x80, // Set divide ratio
    0xD8, // Set pre-charge period
    0x05, // Pre-charge period
    0xD9, // Set COM pin hardware configuration
    0xF1, // COM pin hardware configuration
    0xDA, // Set VCOMH deselect level
    0x30, // VCOMH deselect level
    0x8D, // Set charge pump
    0x14, // Enable charge pump
    0xAF, // Display ON
];

pub unsafe fn OledInit() -> bool {
    let mut ret: bool;
    let mut i: u8;
    for i in 0..1000000 {
        // 上电延时
    }
    let mut cmd = OLED_INIT_CMDS.clone();
    for i in 0..24 {
        ret = FI2cMasterWrite(&mut [cmd[i]], 1, 0);
        if ret != true {
            return ret;
        }
    }
    return true;
}

pub unsafe fn OledDisplayOn() -> bool {
    let mut ret: bool;
    let mut display_data = [0xFF; 128];

    for _ in 0..8 {
        // SSD1306有8页
        for i in 0..128 {
            ret = FI2cMasterWrite(&mut [display_data[i]], 1, 0);
            if ret != true {
                debug!("failed");
                return ret;
            }
        }
    }
    return true;
}

pub fn run_iicoled() {
    unsafe {
        let mut ret: bool = true;
        let address: u32 = 0x3c;
        let mut speed_rate: u32 = 100000; /*kb/s*/
        FIOPadCfgInitialize(&mut iopad_ctrl, &FIOPadLookupConfig(0).unwrap());
        ret = FI2cMioMasterInit(address, speed_rate);
        if ret != true {
            debug!("FI2cMioMasterInit mio_id {:?} is error!", 1);
        }
        ret = OledInit();
        ret = OledDisplayOn();
    }
}
