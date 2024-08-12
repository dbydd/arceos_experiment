#![no_std]
#![no_main]
use core::ptr;
use core::time::Duration;
use core::ptr::write_volatile;
use log::*;
use axhal::time::busy_wait;

pub mod i2c;
pub mod i2c_hw;
pub mod i2c_intr;
pub mod i2c_master;
pub mod i2c_sinit;
pub mod io;
use crate::driver_mio;

use crate::driver_iic::i2c_hw::*;
use crate::driver_iic::i2c_intr::*;
use crate::driver_iic::i2c_master::*;
use crate::driver_iic::i2c_sinit::*;
use crate::driver_iic::io::*;

use crate::driver_mio::mio::*;
use crate::driver_mio::mio_g::*;
use crate::driver_mio::mio_hw::*;
use crate::driver_mio::mio_sinit::*;


