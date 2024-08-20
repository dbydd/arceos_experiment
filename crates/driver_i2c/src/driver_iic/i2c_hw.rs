use axhal::time::busy_wait;
use core::time::Duration;
use log::*;

use crate::driver_iic::io::*;

// 定义速度配置相关的结构体
#[derive(Debug, Clone, Copy, Default)]
pub struct FI2cSpeedCfg {
    speed_mode: u32,
    scl_lcnt: u32,
    scl_hcnt: u32,
    sda_hold: u32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FI2cSpeedModeInfo {
    speed: u32,
    min_scl_hightime_ns: u32,
    min_scl_lowtime_ns: u32,
    def_risetime_ns: u32,
    def_falltime_ns: u32,
}

pub const I2C_SPEED_CFG: [FI2cSpeedModeInfo; 3] = [
    FI2cSpeedModeInfo {
        speed: 100000,
        min_scl_hightime_ns: 4000,
        min_scl_lowtime_ns: 4700,
        def_risetime_ns: 1000,
        def_falltime_ns: 300,
    },
    FI2cSpeedModeInfo {
        speed: 400000,
        min_scl_hightime_ns: 600,
        min_scl_lowtime_ns: 1300,
        def_risetime_ns: 300,
        def_falltime_ns: 300,
    },
    FI2cSpeedModeInfo {
        speed: 1000000,
        min_scl_hightime_ns: 390,
        min_scl_lowtime_ns: 460,
        def_risetime_ns: 60,
        def_falltime_ns: 160,
    },
];

pub fn div_round_up(n: u32, d: u32) -> u32 {
    (((n) + (d) - 1) / (d)) as u32
}

pub fn fi2c_ic_sar_mask() -> u32 {
    ((!0u32) - (1u32 << (0)) + 1) & (!0u32 >> (32 - 1 - (9)))
}

pub fn fi2c_ic_tar_mask() -> u32 {
    ((!0u32) - (1u32 << (0)) + 1) & (!0u32 >> (32 - 1 - (9)))
}

pub fn fi2c_con_speed_mask() -> u32 {
    ((!0u32) - (1u32 << 1) + 1) & (!0u32 >> (32 - 1 - 2))
}

//设置I2C控制器的使能状态
pub fn fi2c_set_enable(addr: u32, enable: bool) -> bool {
    let status: u32 = match enable {
        true => 0x1 << 0,
        false => 0x0 << 0,
    };
    let mut timeout: u32 = 50000;
    while 0 != timeout {
        output_32(addr, 0x6C, status);
        if ((input_32(addr, 0x9C)) & (0x1 << 0)) == status {
            return true;
        }
        timeout -= 1;
    }
    trace!("the enable is {:?}", enable);
    return false;
}

//计算I2C的上升沿下降沿配置
pub fn fi2c_calc_timing(bus_clk_hz: u32, spk_cnt: u32, speed_cfg_p: &mut FI2cSpeedCfg) -> bool {
    // 确保 speed_cfg_p 不为空
    assert!(Some(speed_cfg_p.clone()).is_some());

    let speed_mode: usize = speed_cfg_p.speed_mode as usize;
    let info_p = &I2C_SPEED_CFG[speed_mode];

    let fall_cnt: i32;
    let rise_cnt: i32;
    let min_t_low_cnt: i32;
    let min_t_high_cnt: i32;
    let mut hcnt: i32;
    let mut lcnt: i32;
    let period_cnt: i32;
    let diff: i32;
    let mut tot: i32;
    let scl_rise_time_ns: i32;
    let scl_fall_time_ns: i32;

    period_cnt = (bus_clk_hz / info_p.speed) as i32;
    scl_rise_time_ns = (info_p.def_risetime_ns) as i32;
    scl_fall_time_ns = (info_p.def_falltime_ns) as i32;

    // 将周期转换为 IC 时钟周期的数量
    fall_cnt = div_round_up(bus_clk_hz / 1000 * (scl_rise_time_ns as u32), 1000000) as i32;
    rise_cnt = div_round_up(bus_clk_hz / 1000 * (scl_fall_time_ns as u32), 1000000) as i32;
    min_t_low_cnt = div_round_up(bus_clk_hz / 1000 * info_p.min_scl_lowtime_ns, 1000000) as i32;
    min_t_high_cnt = div_round_up(bus_clk_hz / 1000 * info_p.min_scl_hightime_ns, 1000000) as i32;

    // 打印调试信息
    trace!(
        "i2c: mode {}, bus_clk {}, speed {}, period {} rise {} fall {} tlow {} thigh {} spk {}",
        speed_mode,
        bus_clk_hz,
        info_p.speed,
        period_cnt,
        rise_cnt,
        fall_cnt,
        min_t_low_cnt,
        min_t_high_cnt,
        spk_cnt
    );

    // 根据以下公式反推 hcnt 和 lcnt
    // SCL_High_time = [(HCNT + IC_*_SPKLEN + 7) * icClk] + SCL_Fall_time
    // SCL_Low_time = [(LCNT + 1) * icClk] - SCL_Fall_time + SCL_Rise_time
    hcnt = min_t_high_cnt - fall_cnt - 7 - spk_cnt as i32;
    lcnt = min_t_low_cnt - rise_cnt + fall_cnt - 1;

    if hcnt < 0 || lcnt < 0 {
        trace!("i2c: bad counts. hcnt = {} lcnt = {}", hcnt, lcnt);
        return false;
    }

    // 确保周期符合要求。如果不符合，将差值均分并偏向 lcnt
    tot = hcnt + lcnt + 7 + spk_cnt as i32 + rise_cnt + 1;

    if tot < period_cnt {
        diff = (period_cnt - tot) / 2;
        hcnt += diff;
        lcnt += diff;
        tot = hcnt + lcnt + 7 + spk_cnt as i32 + rise_cnt + 1;
        lcnt += period_cnt - tot;
    }

    speed_cfg_p.scl_lcnt = lcnt as u32;
    speed_cfg_p.scl_hcnt = hcnt as u32;
    speed_cfg_p.sda_hold = (bus_clk_hz / 1000 * 300 + 1000000 - 1) / 1000000; // 使用默认值，除非另有指定

    // 打印最终配置
    trace!(
        "i2c: hcnt = {} lcnt = {} sda hold = {}",
        speed_cfg_p.scl_hcnt,
        speed_cfg_p.scl_lcnt,
        speed_cfg_p.sda_hold
    );

    true
}

//计算I2C的速度配置
// enum
// {
//     FI2C_STANDARD_SPEED = 0,
//     FI2C_FAST_SPEED,
//     FI2C_HIGH_SPEED,
//     FI2C_SPEED_MODE_MAX
// };
pub fn fi2c_calc_speed_cfg(
    addr: u32,
    speed: u32,
    bus_clk_hz: u32,
    speed_cfg_p: &mut FI2cSpeedCfg,
) -> bool {
    assert!(Some(speed_cfg_p.clone()).is_some()); // 确保 speed_cfg_p 不为空
    let spk_cnt: u32;

    if speed >= 1000000 {
        speed_cfg_p.speed_mode = 2;
        spk_cnt = input_32(addr, 0xa4);
    } else if speed >= 400000 {
        speed_cfg_p.speed_mode = 1;
        spk_cnt = input_32(addr, 0xa0);
    } else if speed >= 100000 {
        speed_cfg_p.speed_mode = 0;
        spk_cnt = input_32(addr, 0xa0);
    } else {
        return false;
    }

    fi2c_calc_timing(bus_clk_hz, spk_cnt, speed_cfg_p)
}

//设置I2C控制器的速率
pub fn fi2c_set_speed(addr: u32, speed_rate: u32) -> bool {
    let mut ret = true;
    let mut speed_cfg = FI2cSpeedCfg {
        ..Default::default()
    }; // 初始化 speed_cfg
    let enable_status: u32;
    let mut reg_val: u32;

    // 计算速率配置
    ret = fi2c_calc_speed_cfg(addr, speed_rate, 50000000, &mut speed_cfg);
    if ret != true {
        return false;
    }

    // 获取启用状态
    enable_status = input_32(addr, 0x9C);

    // 重置速率模式位
    reg_val = input_32(addr, 0x00) & !fi2c_con_speed_mask();
    match speed_cfg.speed_mode {
        0 => {
            reg_val |= 0x1 << 1;
            output_32(addr, 0x14, speed_cfg.scl_hcnt);
            output_32(addr, 0x18, speed_cfg.scl_lcnt);
        }
        1 => {
            reg_val |= 0x2 << 1;
            output_32(addr, 0x1C, speed_cfg.scl_hcnt);
            output_32(addr, 0x20, speed_cfg.scl_lcnt);
        }
        2 => {
            reg_val |= 0x3 << 1;
            output_32(addr, 0x24, speed_cfg.scl_hcnt);
            output_32(addr, 0x28, speed_cfg.scl_lcnt);
        }
        _ => {
            return false;
        }
    }

    output_32(addr, 0x00, reg_val);

    // 配置 SDA 保持时间（如果需要）
    if speed_cfg.sda_hold != 0 {
        output_32(addr, 0x7c, speed_cfg.sda_hold);
    }

    // 恢复 I2C 状态
    if enable_status == (0x1 << 0) {
        ret |= fi2c_set_enable(addr, true);
    }

    ret
}

//等待特定的I2C状态位直到状态不存在或者超时
pub fn fi2c_wait_status(addr: u32, stat_bit: u32) -> bool {
    let mut timeout: u32 = 0;

    // 等待状态位设置或超时
    while !((input_32(addr, 0x70) & stat_bit) != 0) && (50000 > timeout) {
        busy_wait(Duration::from_millis(1)); // 等待 1 微秒
        timeout += 1;
    }

    if timeout >= 50000 {
        trace!("Timeout when wait status: {:?}", stat_bit);
        return false;
    }

    true
}

//等待I2C总线忙
pub fn fi2c_wait_bus_busy(addr: u32) -> bool {
    let mut ret = true;

    if (input_32(addr, 0x70) & (0x1 << 5)) != 0
        && (true != fi2c_wait_status(addr, 0x1 << 2)) != true
    {
        ret = false;
        trace!("Timeout when wait i2c bus busy.");
    }

    ret
}

//设置与I2C主机通信的从机地址
pub fn fi2c_set_tar(addr: u32, tar_addr: u32) -> bool {
    let enable_status = input_32(addr, 0x9C);
    let mut ret = true;

    if enable_status == (0x1 << 0) {
        ret = fi2c_set_enable(addr, false);
    }

    if ret == true {
        output_32(addr, 0x04, tar_addr & fi2c_ic_tar_mask());
    }

    if enable_status == (0x1 << 0) {
        ret = fi2c_set_enable(addr, true);
    }

    ret
}

//从机模式下，设置I2C地址
pub fn fi2c_set_sar(addr: u32, sar_addr: u32) -> bool {
    let enable_status = input_32(addr, 0x9C);
    let mut ret = true;

    if enable_status == (0x1 << 0) {
        ret = fi2c_set_enable(addr, false);
    }

    if ret == true {
        output_32(addr, 0x08, sar_addr & fi2c_ic_sar_mask());
    }

    if enable_status == (0x1 << 0) {
        ret = fi2c_set_enable(addr, true);
    }

    ret
}

//等待接收Fifo传输完成
pub fn fi2c_flush_rx_fifo(addr: u32) -> bool {
    let mut data: u8;
    let mut timeout = 0;
    let mut ret = true;

    // 读取数据直到 FIFO 为空
    while (input_32(addr, 0x70) & (0x1 << 3)) != 0 {
        data = input_32(addr, 0x10) as u8;

        if timeout >= 50000 {
            ret = false;
            trace!("Timeout when flush fifo.");
            break;
        }

        busy_wait(Duration::from_millis(1)); // 等待 1 微秒
        timeout += 1;
    }

    ret
}

//清除中断状态位，返回清除前的中断状态
pub fn fi2c_clear_intr_bits(addr: u32, last_err_p: &mut u32) -> u32 {
    assert!(Some(last_err_p.clone()).is_some());

    let stat: u32 = input_32(addr, 0x2C);

    // 读取以清除中断状态位
    if (stat & (0x1 << 6)) != 0 {
        *last_err_p = input_32(addr, 0x80); // 读取中止源
        input_32(addr, 0x54); // 清除 TX_ABRT 中断
    }

    if (stat & (0x1 << 0)) != 0 {
        input_32(addr, 0x44); // 清除 RX_UNDER 中断
    }

    if (stat & (0x1 << 1)) != 0 {
        input_32(addr, 0x48); // 清除 RX_OVER 中断
    }

    if (stat & (0x1 << 3)) != 0 {
        input_32(addr, 0x4C); // 清除 TX_OVER 中断
    }

    if (stat & (0x1 << 7)) != 0 {
        input_32(addr, 0x58); // 清除 RX_DONE 中断
    }

    if (stat & (0x1 << 8)) != 0 {
        input_32(addr, 0x5c); // 清除 ACTIVITY 中断
    }

    if (stat & (0x1 << 9)) != 0 {
        input_32(addr, 0x60); // 清除 STOP_DET 中断
    }

    if (stat & (0x1 << 10)) != 0 {
        input_32(addr, 0x64); // 清除 START_DET 中断
    }

    if (stat & (0x1 << 11)) != 0 {
        input_32(addr, 0x68); // 清除 GEN_CALL 中断
    }

    stat
}

//
pub fn fi2c_clear_abort(addr: u32) {
    let mut reg_val: u32;
    let mut timeout: u32 = 50000;

    loop {
        // 清除中断状态
        input_32(addr, 0x40);
        reg_val = input_32(addr, 0x80);

        if reg_val == 0 {
            return;
        }

        if timeout == 0 {
            trace!("Timeout when clear abort.");
            return;
        }

        timeout -= 1;
        busy_wait(Duration::from_millis(1)); // 等待 1 微秒
    }
}
