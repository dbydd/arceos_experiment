#![no_std]
#![no_main]
use super::driver_mio::{mio, mio_g, mio_hw, mio_sinit};
use super::{i2c, i2c_hw, i2c_intr, i2c_master, i2c_sinit, io};
use axhal::time::busy_wait;
use core::ptr;
use core::time::Duration;
use log::*;

use crate::driver_iic::i2c::*;
use crate::driver_iic::i2c_hw::*;
use crate::driver_iic::i2c_intr::*;
use crate::driver_iic::i2c_sinit::*;
use crate::driver_iic::io::*;

use crate::driver_mio::mio::*;
use crate::driver_mio::mio_g::*;
use crate::driver_mio::mio_hw::*;
use crate::driver_mio::mio_sinit::*;

fn FI2C_DATA_MASK() -> u32 {
    (((!0u32) - (1u32 << 0) + 1) & (!0u32 >> (32 - 1 - 7)))
}

pub fn FI2cMasterStartTrans(
    instance_p: &mut FI2c,
    mem_addr: u32,
    mem_byte_len: u8,
    flag: u16,
) -> bool {
    assert!(Some(instance_p.clone()).is_some());
    let base_addr = instance_p.config.base_addr;
    let mut addr_len: u32 = mem_byte_len as u32;
    let mut ret = true;

    ret = FI2cWaitBusBusy(base_addr.try_into().unwrap());
    if ret != true {
        return ret;
    }
    ret = FI2cSetTar(base_addr.try_into().unwrap(), instance_p.config.slave_addr);

    while addr_len > 0 {
        if FI2cWaitStatus(base_addr.try_into().unwrap(), (0x1 << 1)) != true {
            break;
        }
        if input_32(base_addr.try_into().unwrap(), 0x80) != 0 {
            return false;
        }
        if input_32(base_addr.try_into().unwrap(), 0x70) & (0x1 << 1) != 0 {
            addr_len -= 1;
            let value = (mem_addr >> (addr_len * 8)) & FI2C_DATA_MASK();
            if addr_len != 0 {
                output_32(base_addr.try_into().unwrap(), 0x10, value);
            } else {
                output_32(base_addr.try_into().unwrap(), 0x10, value + flag as u32);
            }
        }
    }
    ret
}

pub fn FI2cMasterStopTrans(instance_p: &mut FI2c) -> bool {
    assert!(Some(instance_p.clone()).is_some());
    let mut ret = true;
    let base_addr = instance_p.config.base_addr;
    let mut reg_val = 0;
    let mut timeout = 0;

    while true {
        if input_32(base_addr.try_into().unwrap(), 0x34) & (0x1 << 9) != 0 {
            reg_val = input_32(base_addr.try_into().unwrap(), 0x60);
            break;
        } else if 500 < timeout {
            break;
        }
        timeout += 1;
        busy_wait(Duration::from_millis(1));
    }

    ret = FI2cWaitBusBusy(base_addr.try_into().unwrap());
    if ret == true {
        ret = FI2cFlushRxFifo(base_addr.try_into().unwrap());
    }
    ret
}

pub fn FI2cMasterReadPoll(
    instance_p: &mut FI2c,
    mem_addr: u32,
    mem_byte_len: u8,
    buf_p: &mut [u8],
    buf_len: u32,
) -> bool {
    assert!(Some(instance_p.clone()).is_some());
    let mut ret = true;
    let mut reg_val: u32 = 0;
    let base_addr: u32 = instance_p.config.base_addr as u32;
    let mut tx_len = buf_len;
    let mut rx_len = buf_len;
    let mut trans_timeout = 0;

    if instance_p.is_ready != 0x11111111u32 {
        return false;
    }
    if instance_p.config.work_mode != 0 {
        return false;
    }

    ret = FI2cMasterStartTrans(instance_p, mem_addr, mem_byte_len, (0x0 << 8));
    if ret != true {
        return ret;
    }

    while tx_len > 0 || rx_len > 0 {
        if input_32(base_addr, 0x80) != 0 {
            return false;
        }

        let mut rx_limit = 8 - input_32(base_addr, 0x78);
        let mut tx_limit = 8 - input_32(base_addr, 0x74);

        while tx_len > 0 && rx_limit > 0 && tx_limit > 0 {
            let reg_val = if tx_len == 1 {
                (0x1 << 8) | (0x1 << 9)
            } else {
                (0x1 << 8)
            };
            output_32(base_addr, 0x10, reg_val);
            tx_len -= 1;
            rx_limit -= 1;
            tx_limit -= 1;
        }
        let mut rx_tem: u32 = input_32(base_addr, 0x78);
        let mut i = 0;
        while rx_len > 0 && rx_tem > 0 {
            if input_32(base_addr, 0x70) & (0x1 << 3) != 0 {
                buf_p[i] = (input_32(base_addr, 0x10) & FI2C_DATA_MASK()) as u8;
                i+=1;
                rx_len -= 1;
                rx_tem -= 1;
                trans_timeout = 0;
            } else {
                trans_timeout += 1;
                busy_wait(Duration::from_millis(1));
                if trans_timeout >= 500 {
                    return false;
                }
            }
        }
        i = 0;
    }
    if ret == true {
        ret = FI2cMasterStopTrans(instance_p);
    }
    ret
}

pub unsafe fn FI2cMasterWritePoll(
    instance_p: &mut FI2c,
    mem_addr: u32,
    mem_byte_len: u8,
    buf_p: &mut [u8],
    buf_len: u32,
) -> bool {
    assert!(Some(instance_p.clone()).is_some());
    let mut ret = true;
    let base_addr = instance_p.config.base_addr;
    let mut buf_idx = buf_len;
    let mut trans_timeout = 0;
    let mut tx_limit: u32;
    let mut reg_val: u32;

    if instance_p.is_ready != 0x11111111u32 {
        return false;
    }
    if instance_p.config.work_mode != 0 {
        return false;
    }

    ret = FI2cMasterStartTrans(instance_p, mem_addr, mem_byte_len, (0x0 << 8));
    if ret != true {
        return ret;
    }
    while buf_idx > 0 {
        if input_32(base_addr.try_into().unwrap(), 0x80) != 0 {
            return false;
        }
        //计算传输限制 tx_limit，表示可写入 FIFO 的字节数。
        let mut tx_limit = 8 - input_32(base_addr.try_into().unwrap(), 0x74);
        let mut i = 0;
        while tx_limit > 0 && buf_idx > 0 {
            if input_32(base_addr.try_into().unwrap(), 0x70) & (0x1 << 1) != 0 {
                let reg_val = if buf_idx == 1 {
                    (FI2C_DATA_MASK() & buf_p[i] as u32) | (0x0 << 8) | (0x1 << 9)
                } else {
                    (FI2C_DATA_MASK() & buf_p[i] as u32) | (0x0 << 8)
                };
                output_32(base_addr.try_into().unwrap(), 0x10, reg_val);
                i += 1;
                buf_idx -= 1;
                tx_limit -= 1;
                trans_timeout = 0;
            } else if trans_timeout >= 500 {
                return false;
            }
            trans_timeout += 1;
            //busy_wait(Duration::from_millis(1));
        }
        i = 0;
        debug!("================================================================");
    }
    if ret == true {
        ret = FI2cMasterStopTrans(instance_p);
    }
    ret
}

pub fn FI2cMasterReadIntr(
    instance_p: &mut FI2c,
    mem_addr: u32,
    mem_byte_len: u8,
    buf_p: &mut [u8],
    buf_len: u32,
) -> bool {
    assert!(Some(instance_p.clone()).is_some());
    let mut ret = true;
    let mut mask: u32;
    let mut trans_timeout: u32 = 0;

    if instance_p.is_ready != 0x11111111u32 {
        return false;
    }
    if instance_p.config.work_mode != 0 {
        return false;
    }
    if instance_p.status == 0x3 {
        return false;
    }

    while instance_p.status != 0x0 {
        if trans_timeout >= 500 {
            return false;
        }
        trans_timeout += 1;
        busy_wait(Duration::from_millis(1));
    }

    instance_p.rxframe.data_buff = buf_p.as_mut_ptr() as *mut core::ffi::c_void;
    instance_p.rxframe.rx_total_num = buf_len;
    instance_p.txframe.tx_total_num = buf_len;
    instance_p.rxframe.rx_cnt = 0;
    output_32(instance_p.config.base_addr.try_into().unwrap(), 0x38, 0);
    ret = FI2cMasterStartTrans(instance_p, mem_addr, mem_byte_len, (0x0 << 8));
    instance_p.status = 0x2;
    if ret != true {
        return ret;
    }
    let mut mask = input_32(instance_p.config.base_addr.try_into().unwrap(), 0x30);
    mask |= (((0x1 << 4) | (0x1 << 6)) | (0x1 << 2));
    FI2cMasterSetupIntr(instance_p, mask)
}

pub fn FI2cMasterWriteIntr(
    instance_p: &mut FI2c,
    mem_addr: u32,
    mem_byte_len: u8,
    buf_p: &[u8],
    buf_len: u32,
) -> bool {
    assert!(Some(instance_p.clone()).is_some());
    let mut ret = true;
    let mut mask: u32;
    let mut trans_timeout: u32 = 0;
    if instance_p.is_ready != 0x11111111u32 {
        return false;
    }
    if instance_p.config.work_mode != 0 {
        return false;
    }
    if instance_p.status == 0x3 {
        return false;
    }
    while instance_p.status != 0x0 {
        if trans_timeout >= 500 {
            return false;
        }
        trans_timeout += 1;
        busy_wait(Duration::from_millis(1));
    }

    instance_p.txframe.data_buff = buf_p.as_ptr() as *const core::ffi::c_void;
    instance_p.txframe.tx_total_num = buf_len;
    instance_p.txframe.tx_cnt = 0;

    ret = FI2cMasterStartTrans(instance_p, mem_addr, mem_byte_len, (0x0 << 8));
    if ret != true {
        return ret;
    }
    instance_p.status = 0x1;
    let mut mask = input_32(instance_p.config.base_addr.try_into().unwrap(), 0x30);
    mask |= ((0x1 << 4) | (0x1 << 6));
    FI2cMasterSetupIntr(instance_p, mask)
}
