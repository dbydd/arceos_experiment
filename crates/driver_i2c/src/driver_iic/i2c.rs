use log::*;

use crate::driver_iic::i2c_hw::*;
use crate::driver_iic::io::*;

/// I2C设备配置结构体
#[derive(Debug, Clone, Copy, Default)]
pub struct FI2cConfig {
    pub instance_id: u32,    // 设备实例ID
    pub base_addr: usize,    // 设备基地址
    pub irq_num: u32,        // 设备中断ID
    pub irq_priority: u32,   // 设备中断优先级
    pub ref_clk_hz: u32,     // 输入参考时钟频率（Hz）
    pub work_mode: u32,      // 设备工作模式：从机或主机
    pub slave_addr: u32,     // 主模式从机地址（读/写）或从机模式本地地址
    pub use_7bit_addr: bool, // 从机地址是否使用7位或10位
    pub speed_rate: u32,     // I2C速度
}

/// I2C事件处理程序函数类型
pub type FI2cEvtHandler = fn(instance_p: *mut FI2c, param: *mut core::ffi::c_void);

/// I2C发送数据帧
#[derive(Debug, Clone, Copy)]
pub struct FI2cFrameTX {
    pub data_buff: *const core::ffi::c_void, // 数据缓冲区
    pub tx_total_num: u32,                   // 发送数据总量
    pub tx_cnt: u32,                         // 已发送数据量
    pub flag: u32,                           // 标志位（CMD、STOP、RESTART）
}

impl Default for FI2cFrameTX {
    fn default() -> Self {
        Self {
            data_buff: core::ptr::null_mut(), // 设置默认值为 null 指针
            tx_total_num: 0,
            tx_cnt: 0,
            flag: 0,
        }
    }
}

/// I2C接收数据帧
#[derive(Debug, Clone, Copy)]
pub struct FI2cFrameRX {
    pub data_buff: *mut core::ffi::c_void, // 数据缓冲区
    pub rx_total_num: u32,                 // 接收数据总量
    pub rx_cnt: u32,                       // 已接收数据量
}

impl Default for FI2cFrameRX {
    fn default() -> Self {
        Self {
            data_buff: core::ptr::null_mut(), // 设置默认值为 null 指针
            rx_total_num: 0,
            rx_cnt: 0,
        }
    }
}

/// I2C设备实例
#[feature(const_trait_impl)]
#[derive(Debug, Clone, Copy, Default)]
pub struct FI2c {
    pub config: FI2cConfig,                               // 当前活跃配置
    pub is_ready: u32,                                    // 设备是否已初始化并准备好
    pub status: u32,                                      // 设备状态
    pub txframe: FI2cFrameTX,                             // 发送数据帧
    pub rxframe: FI2cFrameRX,                             // 接收数据帧
    pub master_evt_handlers: [Option<FI2cEvtHandler>; 3], // 主设备中断处理程序
    pub slave_evt_handlers: [Option<FI2cEvtHandler>; 6],  // 从设备中断处理程序
}

pub static mut MASTER_I2C_INSTANCE: FI2c = FI2c {
    config: FI2cConfig {
        instance_id: 0,
        base_addr: 0,
        irq_num: 0,
        irq_priority: 0,
        ref_clk_hz: 0,
        work_mode: 0,
        slave_addr: 0,
        use_7bit_addr: false,
        speed_rate: 0,
    },
    is_ready: 0,
    status: 0,
    txframe: FI2cFrameTX {
        data_buff: core::ptr::null_mut(),
        tx_total_num: 0,
        tx_cnt: 0,
        flag: 0,
    },
    rxframe: FI2cFrameRX {
        data_buff: core::ptr::null_mut(),
        rx_total_num: 0,
        rx_cnt: 0,
    },
    master_evt_handlers: [None; 3],
    slave_evt_handlers: [None; 6],
};

pub fn fi2c_cfg_initialize(instance_p: &mut FI2c, input_config_p: &FI2cConfig) -> bool {
    assert!(Some(instance_p.clone()).is_some() && Some(input_config_p).is_some());

    let mut ret = true;

    // 如果设备已启动，禁止初始化并返回已启动状态，允许用户取消初始化设备并重新初始化，但防止用户无意中初始化
    if instance_p.is_ready == 0x11111111u32 {
        trace!("Device is already initialized!!!");
        return false;
    }

    // 设置默认值和配置数据，包括将回调处理程序设置为存根，以防应用程序未分配自己的回调而导致系统崩溃
    fi2c_de_initialize(instance_p);
    instance_p.config = *input_config_p;

    // 重置设备
    ret = {
        let instance_p: &mut FI2c = instance_p;
        assert!(Some(instance_p.clone()).is_some());
        let mut ret = true;
        let config_p = &instance_p.config;
        let base_addr = config_p.base_addr;
        let mut reg_val: u32 = 0;

        ret = fi2c_set_enable(base_addr.try_into().unwrap(), false); // 禁用 i2c 控制器

        if config_p.work_mode == 0 {
            reg_val |= if config_p.use_7bit_addr {
                0x0 << 4
            } else {
                0x1 << 4
            };
            reg_val |= 0x1 << 6;
            reg_val |= 0x1 << 0;
            reg_val |= 0x1 << 5;
        } else {
            reg_val |= if config_p.use_7bit_addr {
                0x0 << 3
            } else {
                0x1 << 3
            };
            reg_val &= !(0x1 << 0);
            reg_val |= 0x0 << 0;
        }
        reg_val |= 0x1 << 1;

        output_32(base_addr.try_into().unwrap(), 0x00, reg_val);
        output_32(base_addr.try_into().unwrap(), 0x38, 0);
        output_32(base_addr.try_into().unwrap(), 0x3C, 0);
        output_32(base_addr.try_into().unwrap(), 0x30, 0); // 禁用所有中断

        ret = fi2c_set_speed(base_addr.try_into().unwrap(), config_p.speed_rate);

        if ret == true {
            ret = fi2c_set_enable(base_addr.try_into().unwrap(), true); // 启用 i2c 控制器
        }

        // 如果初始化成功且 i2c 处于从模式，则设置从地址
        if ret == true && config_p.work_mode == 1 {
            ret = fi2c_set_sar(base_addr.try_into().unwrap(), config_p.slave_addr);
        }

        ret
    };
    if ret == true {
        instance_p.is_ready = 0x11111111u32;
    }

    ret
}

pub fn fi2c_de_initialize(instance_p: &mut FI2c) {
    assert!(Some(instance_p.clone()).is_some());
    instance_p.is_ready = 0;

    // 清零实例数据
    unsafe {
        core::ptr::write_bytes(instance_p as *mut FI2c, 0, size_of::<FI2c>());
    }
}

//we don't need this now
// pub fn fi2c_error_to_message(error: FError) -> Option<&'static str> {}
