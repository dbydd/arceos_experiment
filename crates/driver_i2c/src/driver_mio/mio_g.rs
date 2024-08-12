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
use crate::driver_mio::mio_hw::*;
use crate::driver_mio::mio_sinit::*;


