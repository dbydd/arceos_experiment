#![no_std]
#![allow(dead_code)]
#![allow(static_mut_refs)]
#![allow(private_interfaces)]
#![allow(unused_assignments)]
#![allow(unused_unsafe)]
#![allow(unused_attributes)]
#![allow(unused_variables)]
use log::*;
pub mod driver_iic;
pub mod driver_mio;
pub mod example;

use crate::driver_iic::io::*;

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

pub unsafe fn oled_init() -> bool {
    let mut ret: bool;
    (0..1000000).for_each(|_i| {
        // 上电延时
    });
    let cmd = OLED_INIT_CMDS.clone();
    for i in 0..24 {
        ret = fi2c_master_write(&mut [cmd[i]], 1, 0);
        if ret != true {
            return ret;
        }
    }
    return true;
}

pub unsafe fn oled_display_on() -> bool {
    let mut ret: bool;
    let display_data = [0xFF; 128];

    for _ in 0..8 {
        // SSD1306有8页
        for i in 0..128 {
            ret = fi2c_master_write(&mut [display_data[i]], 1, 0);
            if ret != true {
                trace!("failed");
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
        let speed_rate: u32 = 100000; /*kb/s*/
        fiopad_cfg_initialize(&mut IOPAD_CTRL, &fiopad_lookup_config(0).unwrap());
        ret = fi2c_mio_master_init(address, speed_rate);
        if ret != true {
            trace!("FI2cMioMasterInit mio_id {:?} is error!", 1);
        }
        ret = oled_init();
        ret = oled_display_on();
    }
}
