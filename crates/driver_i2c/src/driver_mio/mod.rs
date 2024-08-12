#![no_std]
#![no_main]
use log::*;
use axhal::time::busy_wait;
use core::ptr;
use core::mem::size_of;
use core::time::Duration;

pub mod mio;
pub mod mio_sinit;
pub mod mio_hw;
pub mod mio_g;
use crate::driver_iic;