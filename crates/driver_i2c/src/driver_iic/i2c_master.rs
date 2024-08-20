use axhal::time::busy_wait;
use core::time::Duration;
use log::*;

use crate::driver_iic::i2c::*;
use crate::driver_iic::i2c_hw::*;
use crate::driver_iic::i2c_intr::*;
use crate::driver_iic::io::*;

fn fi2c_data_mask() -> u32 {
    ((!0u32) - (1u32 << 0) + 1) & (!0u32 >> (32 - 1 - 7))
}

pub fn fi2c_master_start_trans(
    instance_p: &mut FI2c,
    mem_addr: u32,
    mem_byte_len: u8,
    flag: u16,
) -> bool {
    assert!(Some(instance_p.clone()).is_some());
    let base_addr = instance_p.config.base_addr;
    let mut addr_len: u32 = mem_byte_len as u32;
    let mut ret = true;

    ret = fi2c_wait_bus_busy(base_addr.try_into().unwrap());
    if ret != true {
        return ret;
    }
    ret = fi2c_set_tar(base_addr.try_into().unwrap(), instance_p.config.slave_addr);

    while addr_len > 0 {
        if fi2c_wait_status(base_addr.try_into().unwrap(), 0x1 << 1) != true {
            break;
        }
        if input_32(base_addr.try_into().unwrap(), 0x80) != 0 {
            return false;
        }
        if input_32(base_addr.try_into().unwrap(), 0x70) & (0x1 << 1) != 0 {
            addr_len -= 1;
            let value = (mem_addr >> (addr_len * 8)) & fi2c_data_mask();
            if addr_len != 0 {
                output_32(base_addr.try_into().unwrap(), 0x10, value);
            } else {
                output_32(base_addr.try_into().unwrap(), 0x10, value + flag as u32);
            }
        }
    }
    ret
}

pub fn fi2c_master_stop_trans(instance_p: &mut FI2c) -> bool {
    assert!(Some(instance_p.clone()).is_some());
    let mut ret = true;
    let base_addr = instance_p.config.base_addr;
    let mut reg_val = 0;
    let mut timeout = 0;

    loop {
        if input_32(base_addr.try_into().unwrap(), 0x34) & (0x1 << 9) != 0 {
            reg_val = input_32(base_addr.try_into().unwrap(), 0x60);
            break;
        } else if 500 < timeout {
            break;
        }
        timeout += 1;
        busy_wait(Duration::from_millis(1));
    }

    ret = fi2c_wait_bus_busy(base_addr.try_into().unwrap());
    if ret == true {
        ret = fi2c_flush_rx_fifo(base_addr.try_into().unwrap());
    }
    ret
}

pub fn fi2c_master_read_poll(
    instance_p: &mut FI2c,
    mem_addr: u32,
    mem_byte_len: u8,
    buf_p: &mut [u8],
    buf_len: u32,
) -> bool {
    assert!(Some(instance_p.clone()).is_some());
    let mut ret = true;
    let reg_val: u32 = 0;
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

    ret = fi2c_master_start_trans(instance_p, mem_addr, mem_byte_len, 0x0 << 8);
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
                0x1 << 8
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
                buf_p[i] = (input_32(base_addr, 0x10) & fi2c_data_mask()) as u8;
                i += 1;
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
        ret = fi2c_master_stop_trans(instance_p);
    }
    ret
}

pub unsafe fn fi2c_master_write_poll(
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

    ret = fi2c_master_start_trans(instance_p, mem_addr, mem_byte_len, 0x0 << 8);
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
                    (fi2c_data_mask() & buf_p[i] as u32) | (0x0 << 8) | (0x1 << 9)
                } else {
                    (fi2c_data_mask() & buf_p[i] as u32) | (0x0 << 8)
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
        trace!("================================================================");
    }
    if ret == true {
        ret = fi2c_master_stop_trans(instance_p);
    }
    ret
}

pub fn fi2c_master_read_intr(
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
    ret = fi2c_master_start_trans(instance_p, mem_addr, mem_byte_len, 0x0 << 8);
    instance_p.status = 0x2;
    if ret != true {
        return ret;
    }
    let mut mask = input_32(instance_p.config.base_addr.try_into().unwrap(), 0x30);
    mask |= ((0x1 << 4) | (0x1 << 6)) | (0x1 << 2);
    fi2c_master_setup_intr(instance_p, mask)
}

pub fn fi2c_master_write_intr(
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

    ret = fi2c_master_start_trans(instance_p, mem_addr, mem_byte_len, 0x0 << 8);
    if ret != true {
        return ret;
    }
    instance_p.status = 0x1;
    let mut mask = input_32(instance_p.config.base_addr.try_into().unwrap(), 0x30);
    mask |= (0x1 << 4) | (0x1 << 6);
    fi2c_master_setup_intr(instance_p, mask)
}
