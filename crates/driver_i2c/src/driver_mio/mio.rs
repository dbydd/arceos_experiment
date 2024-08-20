use core::mem::size_of;
use log::*;

use crate::driver_iic::io::*;

use crate::driver_mio::mio_hw::*;

fn fiopad_reg0_func_set(x: u8) -> u32 {
    ((x as u32) << 0) & (((!0u32) - (1u32 << (0)) + 1) & (!0u32 >> (32 - 1 - (2))))
}

// 定义 FMioConfig 结构体
#[derive(Debug, Clone, Copy, Default)]
pub struct FMioConfig {
    pub instance_id: u32,      // mio id
    pub func_base_addr: usize, // I2C or UART function address
    pub irq_num: u32,          // Device interrupt id
    pub mio_base_addr: usize,  // MIO control address
}

// 定义 FMioCtrl 结构体
#[feature(const_trait_impl)]
#[derive(Debug, Clone, Copy, Default)]
pub struct FMioCtrl {
    pub config: FMioConfig, // mio config
    pub is_ready: u32,      // mio initialize the complete flag
}

pub static mut MASTER_MIO_CTRL: FMioCtrl = FMioCtrl {
    config: FMioConfig {
        instance_id: 0,
        func_base_addr: 0,
        irq_num: 0,
        mio_base_addr: 0,
    },
    is_ready: 0,
};

/// 初始化 MIO 功能
pub fn fmio_func_init(instance: &mut FMioCtrl, mio_type: u32) -> bool {
    assert!(instance.is_ready != 0x11111111u32);
    let ret = fmio_select_func(instance.config.mio_base_addr, mio_type);

    if ret == true {
        instance.is_ready = 0x11111111u32;
    }

    ret
}

/// 去初始化 MIO 功能
pub fn fmio_func_deinit(instance: &mut FMioCtrl) -> bool {
    let ret = fmio_select_func(instance.config.mio_base_addr, 0b00);
    instance.is_ready = 0;
    // 清零配置
    unsafe {
        core::ptr::write_bytes(instance as *mut FMioCtrl, 0, size_of::<FMioCtrl>());
    }

    ret
}

/// 获取功能设置的基地址
pub fn fmio_func_get_address(instance: &FMioCtrl, mio_type: u32) -> usize {
    assert!(instance.is_ready == 0x11111111u32);

    if fmio_get_func(instance.config.mio_base_addr) != mio_type {
        trace!(
            "Mio instance_id: {}, mio_type error, initialize the type first.",
            instance.config.instance_id
        );
        return 0;
    }

    instance.config.func_base_addr
}

/// 获取 MIO 的中断号
pub fn fmio_func_get_irq_num(instance: &FMioCtrl, mio_type: u32) -> u32 {
    assert!(instance.is_ready == 0x11111111u32);

    if fmio_get_func(instance.config.mio_base_addr) != mio_type {
        trace!(
            "Mio instance_id: {}, mio_type error, initialize the type first.",
            instance.config.instance_id
        );
        return 0;
    }

    instance.config.irq_num
}

pub fn fiopad_set_func(instance_p: &FIOPadCtrl, pin_reg_off: u32, func: u8) -> bool {
    assert!(instance_p.is_ready == 0x11111111u32);

    let base_addr = instance_p.config.base_address;
    let mut reg_val: u32 = input_32(
        base_addr.try_into().unwrap(),
        pin_reg_off.try_into().unwrap(),
    );

    reg_val &= !(((!0u32) - (1u32 << (0)) + 1) & (!0u32 >> (32 - 1 - (2))));
    reg_val |= fiopad_reg0_func_set(func);

    output_32(
        base_addr.try_into().unwrap(),
        pin_reg_off.try_into().unwrap(),
        reg_val,
    );

    let test_val = input_32(
        base_addr.try_into().unwrap(),
        pin_reg_off.try_into().unwrap(),
    );

    if reg_val != test_val {
        trace!(
            "ERROR: FIOPad write failed, pin is {:x}, 0x{:x} != 0x{:x}",
            pin_reg_off,
            reg_val,
            test_val
        );
    }

    true
}
