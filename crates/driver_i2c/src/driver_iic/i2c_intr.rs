use log::*;

use crate::driver_iic::i2c::*;
use crate::driver_iic::i2c_hw::*;
use crate::driver_iic::io::*;

//暂时不用中断，先不翻译完，巨多（悲）

// // I2C主机模式下的中断处理函数
// pub fn FI2cMasterIntrHandler(vector: i32, param: *mut FI2c) {
//     let instance_p = unsafe { &mut *param };
//     let base_addr = instance_p.config.base_addr;
//     let last_err: u32;
//     let stat:u32 = FI2cClearIntrBits(base_addr.try_into().unwrap(), &mut last_err);
//     let raw_stat:u32 = input_32(base_addr.try_into().unwrap(),0x34);
//     let enabled:u32 = input_32(base_addr.try_into().unwrap(),0x6C);
//     let mut val: u32 = 0;

//     assert!(instance_p.config.work_mode == 0);

//     if !(enabled & (0x1 << 0) != 0) || !(raw_stat & !(0x1 << 8) != 0) {
//         return;
//     }

//     if stat & (0x1 << 6) != 0 {
//         trace!("last error: 0x{:x}", last_err);
//         trace!("abort source: 0x{:x}", input_32(base_addr.try_into().unwrap(),0x80));
//         instance_p.status = 0x3;
//         output_32(base_addr.try_into().unwrap(),0x30,0); // Disable all interrupts
//         input_32(base_addr.try_into().unwrap(),0x54); // Clear abort
//         output_32(base_addr.try_into().unwrap(),0x6C,1); // Re-enable I2C
//         FI2cMasterCallEvtHandler(instance_p, 0, &mut val);
//         return;
//     }

//     if stat & (0x1 << 2) != 0 {
//         FI2cMasterIntrRxFullHandler(instance_p);
//         FI2cMasterCallEvtHandler(instance_p, 1, &mut val);
//         return;
//     }

//     if stat & (0x1 << 4) != 0 {
//         FI2cMasterIntrTxEmptyHandler(instance_p);
//         FI2cMasterCallEvtHandler(instance_p, 2, &mut val);
//         return;
//     }
// }

// // I2C从机模式下的中断处理函数
// pub fn FI2cSlaveIntrHandler(vector: i32, param: *mut FI2c){
//     let instance_p = unsafe { &mut *param };
//     let base_addr = instance_p.config.base_addr;
//     let mut last_err: u32;

//     let stat = input_32(base_addr.try_into().unwrap(),0x2C);
//     let raw_stat = input_32(base_addr.try_into().unwrap(),0x34);
//     let enabled = input_32(base_addr.try_into().unwrap(),0x6C);
//     let slave_active = (input_32(base_addr.try_into().unwrap(),0x70) & (0x1 << 6)) != 0;
//     let mut val: u8 = 0;
//     let mut reg_val: u32;

//     assert!(instance_p.config.work_mode == 1);

//     if !(enabled & (0x1 << 0) != 0) || !(raw_stat & !(0x1 << 8) != 0) {
//         return;
//     }

//     let stat = FI2cClearIntrBits(base_addr.try_into().unwrap(), &mut last_err);

//     if stat & (0x1 << 2) != 0 {
//         if instance_p.status != 0x1 {
//             instance_p.status = 0x1;
//             FI2cSlaveCallEvtHandler(instance_p, 1, &mut val);
//         }
//         val = input_32(base_addr.try_into().unwrap(),0x10) as u8;
//         FI2cSlaveCallEvtHandler(instance_p, 3, &mut val);
//     }

//     if stat & (0x1 << 5) != 0 {
//         if slave_active {
//             input_32(base_addr.try_into().unwrap(),0x50); // Clear read request
//             instance_p.status = 0x1;
//             FI2cSlaveCallEvtHandler(instance_p, 0, &mut val);
//             reg_val = val as u32;
//             output_32(base_addr.try_into().unwrap(),0x10,reg_val);
//         }
//     }

//     if stat & (0x1 << 7)  != 0 {
//         FI2cSlaveCallEvtHandler(instance_p, 2, &mut val);
//         input_32(base_addr.try_into().unwrap(),0x58); // Clear RX done
//         return;
//     }

//     if stat & (0x1 << 9) != 0 {
//         instance_p.status = 0x0;
//         FI2cSlaveCallEvtHandler(instance_p, 4, &mut val);
//     }

//     if stat & (0x1 << 6) != 0 {
//         instance_p.status = 0x3;
//         fi2c_slave_call_evt_handler(instance_p, 5, &mut val);
//         trace!("last error: 0x{:x}", last_err);
//         trace!("abort source: 0x{:x}", input_32(base_addr,0x80));
//     }
// }

pub fn fi2c_master_register_intr_handler(instance_p: &mut FI2c, evt: u32, handler: FI2cEvtHandler) {
    assert!(evt < 3 as u32, "Invalid event index");
    instance_p.master_evt_handlers[evt as usize] = Some(handler);
}

pub fn fi2c_stub_handler_wrapper(instance_p: *mut FI2c, param: *mut core::ffi::c_void) {
    fi2c_stub_handler(instance_p as *mut core::ffi::c_void, param);
}

pub fn fi2c_stub_handler(instance_p: *mut core::ffi::c_void, _param: *mut core::ffi::c_void) {
    assert!(!instance_p.is_null(), "instance_p is null");

    // 将 `instance_p` 转换为 `&mut FI2c`
    let instance = unsafe { &*(instance_p as *mut FI2c) };
    let base_addr = instance.config.base_addr;

    // 使用宏或者日志框架输出信息
    let intr_stat = input_32(base_addr.try_into().unwrap(), 0x2C);
    // 假设你有一个宏定义 `fi2c_info` 用于输出日志
    trace!(
        "id: {:?}, intr cause: {:?}",
        instance.config.instance_id,
        intr_stat
    );
}

pub fn fi2c_master_setup_intr(instance_p: &mut FI2c, mask: u32) -> bool {
    assert!(
        instance_p.is_ready == 0x11111111u32,
        "i2c driver is not ready."
    );

    let config_p = &instance_p.config;
    let base_addr = config_p.base_addr;
    let mut evt: u32;

    assert!(
        instance_p.config.work_mode == 0,
        "i2c work mode shall be master."
    );

    // 禁用所有 i2c 中断并清除中断
    fi2c_clear_abort(base_addr.try_into().unwrap());

    for evt in (0..3).into_iter() {
        if !instance_p.master_evt_handlers[evt as usize].is_some() {
            fi2c_master_register_intr_handler(instance_p, evt, fi2c_stub_handler_wrapper);
            // 你可以使用宏定义代替以下输出
            trace!("evt :{:?} is default.", evt);
        }
    }

    output_32(base_addr.try_into().unwrap(), 0x30, mask);

    true
}

// 函数定义
pub fn fi2c_slave_register_intr_handler(instance_p: &mut FI2c, evt: u32, handler: FI2cEvtHandler) {
    if evt >= 6 as u32 {
        trace!("Invalid event index");
    }
    instance_p.slave_evt_handlers[evt as usize] = Some(handler);
}
