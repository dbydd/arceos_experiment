#![no_std]
#![no_main]
use axhal::time::busy_wait;
use core::time::Duration;
use log::*;

use driver_i2c::driver_iic::io::*;
use driver_i2c::example::*;

const PCA9685_ADDRESS: u8 = 0x60;
const MODE1: u8 = 0x00;
const PRE_SCALE: u8 = 0xFE;
const LED0_ON_L: u8 = 0x06;

// ##################################################################

pub fn car_run_task(proposal: Quest) {
    unsafe {
        match proposal {
            Quest::Stop => stop(),
            Quest::Advance => advance(),
            Quest::Back => back(),
            Quest::MoveLeft => move_left(),
            Quest::MoveRight => move_right(),
            Quest::TrunLeft => trun_left(),
            Quest::TrunRight => trun_right(),
            Quest::AdvanceLeft => advance_left(),
            Quest::AdvanceRight => advance_right(),
            Quest::BackLeft => back_left(),
            Quest::BackRight => back_right(),
            Quest::RotateLeft => rotate_left(),
            Quest::RotateRight => rotate_right(),
        }
    }
}

pub enum Quest {
    Stop,
    Advance,
    Back,
    MoveLeft,
    MoveRight,
    TrunLeft,
    TrunRight,
    AdvanceLeft,
    AdvanceRight,
    BackLeft,
    BackRight,
    RotateLeft,
    RotateRight,
}

unsafe fn stop() {
    //停止
    status_control(0, 0, 0, 0);
}
unsafe fn advance() {
    //前进
    status_control(1, 1, 1, 1);
}
unsafe fn back() {
    //后退
    status_control(-1, -1, -1, -1);
}
unsafe fn move_left() {
    //平移向左
    status_control(-1, 1, 1, -1);
}
unsafe fn move_right() {
    //平移向右
    status_control(1, -1, -1, 1);
}
unsafe fn trun_left() {
    //左转
    status_control(0, 1, 1, 1);
}
unsafe fn trun_right() {
    //右转
    status_control(1, 0, 1, 1);
}
unsafe fn advance_left() {
    //左前
    status_control(0, 1, 1, 0);
}
unsafe fn advance_right() {
    //右前
    status_control(1, 0, 0, 1);
}
unsafe fn back_left() {
    //左后
    status_control(-1, 0, 0, -1);
}
unsafe fn back_right() {
    //右后
    status_control(0, -1, -1, 0);
}
unsafe fn rotate_right() {
    //左旋转
    status_control(1, -1, 1, -1);
}
unsafe fn rotate_left() {
    //右旋转
    status_control(-1, 1, -1, 1);
}


unsafe fn write_byte_data(_address: u8, offset: u8, value: u16) {
    let high_byte = (value >> 8) as u8; // 高8位
    let low_byte = (value & 0xFF) as u8; // 低8位
    FI2cMasterWrite(&mut [high_byte], 1, offset as u32);
    FI2cMasterWrite(&mut [low_byte], 1, offset as u32);
}

unsafe fn read_byte_data(_address: u8, offset: u8) -> u16 {
    let high_byte: u8 = 0x00; // 高8位
    let low_byte: u8 = 0x00;
    FI2cMasterRead(&mut [high_byte], 1, offset as u32);
    FI2cMasterRead(&mut [low_byte], 1, offset as u32);
    ((high_byte as u16) << 8) | (low_byte as u16)
}

pub fn pca_init(d1: u16, d2: u16, d3: u16, d4: u16) {
    unsafe {
        let address: u32 = PCA9685_ADDRESS as u32;
        let speed_rate: u32 = 100000; /*kb/s*/
        FIOPadCfgInitialize(&mut iopad_ctrl, &FIOPadLookupConfig(0).unwrap());
        if FI2cMioMasterInit(address, speed_rate) != true {
            trace!("FI2cMioMasterInit mio_id {:?} is error!", 1);
        }
        set_pwm_frequency(50);
        set_pwm(d1, d2, d3, d4);
        stop();
        // traffic_light_release();
    }
}

unsafe fn set_pwm_frequency(freq: u16) {
    let prescale_val = (25000000.0 / ((4096 * freq) as f64) - 1.0) as u16; //计算预分频器值 prescale_val
    let old_mode = read_byte_data(PCA9685_ADDRESS, MODE1);
    let new_mode = (old_mode & 0x7F) | 0x10;
    write_byte_data(PCA9685_ADDRESS, MODE1, new_mode); //休眠模式
    write_byte_data(PCA9685_ADDRESS, PRE_SCALE, prescale_val); //设置PWM频率
    write_byte_data(PCA9685_ADDRESS, MODE1, old_mode); //退出休眠模式
    busy_wait(Duration::from_millis(5)); //等待至少500us，以确保OSC稳定
    write_byte_data(PCA9685_ADDRESS, MODE1, old_mode | 0x80); //重启pca
    write_byte_data(PCA9685_ADDRESS, MODE1, 0x00); //
}

unsafe fn set_pwm(duty_channel1_pwm: u16, duty_channel2_pwm: u16, duty_channel3_pwm: u16, duty_channel4_pwm: u16) {
    let duty_channel1 = duty_channel1_pwm.max(0).min(4095); //限制off_time在0-4095之间
    let duty_channel2 = duty_channel2_pwm.max(0).min(4095);
    let duty_channel3 = duty_channel3_pwm.max(0).min(4095);
    let duty_channel4 = duty_channel4_pwm.max(0).min(4095);

    write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 0, 0 & 0xFF);
    write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 0 + 1, 0 >> 8);
    write_byte_data(
        PCA9685_ADDRESS,
        LED0_ON_L + 4 * 0 + 2,
        (duty_channel1 & 0xFF) as u16,
    );
    write_byte_data(
        PCA9685_ADDRESS,
        LED0_ON_L + 4 * 0 + 3,
        (duty_channel1 >> 8) as u16,
    );

    write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 5, 0 & 0xFF);
    write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 5 + 1, 0 >> 8);
    write_byte_data(
        PCA9685_ADDRESS,
        LED0_ON_L + 4 * 5 + 2,
        (duty_channel2 & 0xFF) as u16,
    );
    write_byte_data(
        PCA9685_ADDRESS,
        LED0_ON_L + 4 * 5 + 3,
        (duty_channel2 >> 8) as u16,
    );

    write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 6, 0 & 0xFF);
    write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 6 + 1, 0 >> 8);
    write_byte_data(
        PCA9685_ADDRESS,
        LED0_ON_L + 4 * 6 + 2,
        (duty_channel3 & 0xFF) as u16,
    );
    write_byte_data(
        PCA9685_ADDRESS,
        LED0_ON_L + 4 * 6 + 3,
        (duty_channel3 >> 8) as u16,
    );

    write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 11, 0 & 0xFF);
    write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 11 + 1, 0 >> 8);
    write_byte_data(
        PCA9685_ADDRESS,
        LED0_ON_L + 4 * 11 + 2,
        (duty_channel4 & 0xFF) as u16,
    );
    write_byte_data(
        PCA9685_ADDRESS,
        LED0_ON_L + 4 * 11 + 3,
        (duty_channel4 >> 8) as u16,
    );
}

unsafe fn status_control(m1: i16, m2: i16, m3: i16, m4: i16) {
    match m1 {
        -1 => {
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 1, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 1 + 1, 0 >> 8);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 1 + 2, 4095 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 1 + 3, 4095 >> 8);

            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 2, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 2 + 1, 0 >> 8);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 2 + 2, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 2 + 3, 0 >> 8);
        }
        0 => {
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 1, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 1 + 1, 0 >> 8);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 1 + 2, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 1 + 3, 0 >> 8);

            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 2, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 2 + 1, 0 >> 8);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 2 + 2, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 2 + 3, 0 >> 8);
        }
        1 => {
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 1, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 1 + 1, 0 >> 8);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 1 + 2, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 1 + 3, 0 >> 8);

            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 2, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 2 + 1, 0 >> 8);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 2 + 2, 4095 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 2 + 3, 4095 >> 8);
        }
        _ => (),
    }

    match m2 {
        -1 => {
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 3, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 3 + 1, 0 >> 8);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 3 + 2, 4095 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 3 + 3, 4095 >> 8);

            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 4, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 4 + 1, 0 >> 8);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 4 + 2, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 4 + 3, 0 >> 8);
        }
        0 => {
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 3, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 3 + 1, 0 >> 8);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 3 + 2, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 3 + 3, 0 >> 8);

            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 4, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 4 + 1, 0 >> 8);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 4 + 2, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 4 + 3, 0 >> 8);
        }
        1 => {
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 3, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 3 + 1, 0 >> 8);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 3 + 2, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 3 + 3, 0 >> 8);

            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 4, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 4 + 1, 0 >> 8);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 4 + 2, 4095 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 4 + 3, 4095 >> 8);
        }
        _ => (),
    }

    match m3 {
        -1 => {
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 7, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 7 + 1, 0 >> 8);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 7 + 2, 4095 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 7 + 3, 4095 >> 8);

            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 8, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 8 + 1, 0 >> 8);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 8 + 2, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 8 + 3, 0 >> 8);
        }
        0 => {
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 7, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 7 + 1, 0 >> 8);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 7 + 2, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 7 + 3, 0 >> 8);

            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 8, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 8 + 1, 0 >> 8);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 8 + 2, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 8 + 3, 0 >> 8);
        }
        1 => {
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 7, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 7 + 1, 0 >> 8);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 7 + 2, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 7 + 3, 0 >> 8);

            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 8, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 8 + 1, 0 >> 8);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 8 + 2, 4095 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 8 + 3, 4095 >> 8);
        }
        _ => (),
    }

    match m4 {
        -1 => {
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 9, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 9 + 1, 0 >> 8);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 9 + 2, 4095 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 9 + 3, 4095 >> 8);

            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 10, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 10 + 1, 0 >> 8);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 10 + 2, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 10 + 3, 0 >> 8);
        }
        0 => {
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 9, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 9 + 1, 0 >> 8);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 9 + 2, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 9 + 3, 0 >> 8);

            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 10, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 10 + 1, 0 >> 8);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 10 + 2, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 10 + 3, 0 >> 8);
        }
        1 => {
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 9, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 9 + 1, 0 >> 8);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 9 + 2, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 9 + 3, 0 >> 8);

            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 10, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 10 + 1, 0 >> 8);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 10 + 2, 4095 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 10 + 3, 4095 >> 8);
        }
        _ => (),
    }
}




pub fn test_pca() {
    
}
