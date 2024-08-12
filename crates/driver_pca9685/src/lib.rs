#![no_std]
#![no_main]
use axhal::time::busy_wait;
use core::default;
use core::ptr;
use core::slice;
use core::time::Duration;
use driver_i2c::driver_iic;
use log::debug;

use driver_i2c::driver_iic::i2c::*;
use driver_i2c::driver_iic::i2c_hw::*;
use driver_i2c::driver_iic::i2c_intr::*;
use driver_i2c::driver_iic::i2c_master::*;
use driver_i2c::driver_iic::i2c_sinit::*;
use driver_i2c::driver_iic::io::*;
use driver_i2c::driver_mio::mio::*;
use driver_i2c::driver_mio::mio_g::*;
use driver_i2c::driver_mio::mio_hw::*;
use driver_i2c::driver_mio::mio_sinit::*;
use driver_i2c::example::*;

const PCA9685_ADDRESS: u8 = 0x60;
const MODE1: u8 = 0x00;
const PRE_SCALE: u8 = 0xFE;
const LED0_ON_L: u8 = 0x06;

// ##################################################################

unsafe fn Car_run_Task(proposal: i32) {
    match proposal {
        0 => Stop(),
        1 => Advance(),
        2 => Back(),
        3 => Move_Left(),
        4 => Move_Right(),
        5 => Trun_Left(),
        6 => Trun_Right(),
        7 => Advance_Left(),
        8 => Advance_Right(),
        9 => Back_Left(),
        10 => Back_Right(),
        11 => Rotate_Left(),
        12 => Rotate_Right(),
        _ => Stop(),
    }
}

unsafe fn Stop() {
    //停止
    status_control(0, 0, 0, 0);
}
unsafe fn Advance() {
    //前进
    status_control(1, 1, 1, 1);
}
unsafe fn Back() {
    //后退
    status_control(-1, -1, -1, -1);
}
unsafe fn Move_Left() {
    //平移向左
    status_control(-1, 1, 1, -1);
}
unsafe fn Move_Right() {
    //平移向右
    status_control(1, -1, -1, 1);
}
unsafe fn Trun_Left() {
    //左转
    status_control(0, 1, 1, 1);
}
unsafe fn Trun_Right() {
    //右转
    status_control(1, 0, 1, 1);
}
unsafe fn Advance_Left() {
    //左前
    status_control(0, 1, 1, 0);
}
unsafe fn Advance_Right() {
    //右前
    status_control(1, 0, 0, 1);
}
unsafe fn Back_Left() {
    //左后
    status_control(-1, 0, 0, -1);
}
unsafe fn Back_Right() {
    //右后
    status_control(0, -1, -1, 0);
}
unsafe fn Rotate_Right() {
    //左旋转
    status_control(1, -1, 1, -1);
}
unsafe fn Rotate_Left() {
    //右旋转
    status_control(-1, 1, -1, 1);
}
unsafe fn LX_90D(t_ms: usize) {
    //左旋转90度
    Rotate_Left();
    busy_wait(Duration::from_millis(((t_ms as f64) / 1000.0) as u64));
    Stop();
}
unsafe fn RX_90D(t_ms: usize) {
    //右旋转90度
    Rotate_Right();
    busy_wait(Duration::from_millis(((t_ms as f64) / 1000.0) as u64));
    Stop();
}
unsafe fn GS_run(L_speed: u16, R_speed: u16) {
    set_pwm(L_speed, R_speed, L_speed, R_speed);
}

unsafe fn write_byte_data(address:u8,offset:u8,value:u16){
    let mut high_byte = (value >> 8) as u8; // 高8位
    let mut low_byte = (value & 0xFF) as u8; // 低8位
    FI2cMasterWrite(&mut [high_byte],1,offset as u32);
    FI2cMasterWrite(&mut [low_byte],1,offset as u32);
}

unsafe fn read_byte_data(address:u8, offset:u8) -> u16{
    let mut high_byte:u8 = 0x00; // 高8位
    let mut low_byte:u8 = 0x00; 
    FI2cMasterRead(&mut [high_byte], 1, offset as u32);
    FI2cMasterRead(&mut [low_byte], 1, offset as u32);
    ((high_byte as u16) << 8) | (low_byte as u16)
}

unsafe fn pca_init(d1: u16, d2: u16, d3: u16, d4: u16) {
    let mut ret: bool = true;
    let address: u32 = PCA9685_ADDRESS as u32;
    let mut speed_rate: u32 = 100000; /*kb/s*/
    FIOPadCfgInitialize(&mut iopad_ctrl, &FIOPadLookupConfig(0).unwrap());
    ret = FI2cMioMasterInit(address, speed_rate);
    if ret != true {
        debug!("FI2cMioMasterInit mio_id {:?} is error!", 1);
    }
    set_pwm_frequency(50);
    set_pwm(d1, d2, d3, d4);
    Stop();
    traffic_light_release();
}

unsafe fn set_pwm_frequency(freq: u16) {
    let prescale_val = (25000000.0 / ((4096 * freq) as f64) - 1.0) as u16;//计算预分频器值 prescale_val
    let old_mode = read_byte_data(PCA9685_ADDRESS, MODE1);
    let new_mode = (old_mode & 0x7F) | 0x10;
    write_byte_data(PCA9685_ADDRESS, MODE1, new_mode);//休眠模式
    write_byte_data(PCA9685_ADDRESS, PRE_SCALE, prescale_val);//设置PWM频率
    write_byte_data(PCA9685_ADDRESS, MODE1, old_mode);//退出休眠模式
    busy_wait(Duration::from_millis(5));//等待至少500us，以确保OSC稳定
    write_byte_data(PCA9685_ADDRESS, MODE1, old_mode | 0x80);//重启pca
    write_byte_data(PCA9685_ADDRESS, MODE1, 0x00);//
}

unsafe fn set_pwm(Duty_channel1: u16, Duty_channel2: u16, Duty_channel3: u16, Duty_channel4: u16) {
    let duty_channel1 = Duty_channel1.max(0).min(4095);  //限制off_time在0-4095之间
    let duty_channel2 = Duty_channel2.max(0).min(4095);
    let duty_channel3 = Duty_channel3.max(0).min(4095);
    let duty_channel4 = Duty_channel4.max(0).min(4095);
    
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
        }
        0 => {
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 1, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 1 + 1, 0 >> 8);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 1 + 2, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 1 + 3, 0 >> 8);
        }
        1 => {
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 1, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 1 + 1, 0 >> 8);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 1 + 2, 0 & 0xFF);
            write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * 1 + 3, 0 >> 8);
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

fn set_servo_angle(angle: u16) -> u16 {
    let MIN_PULSE: u16 = 150;
    let MAX_PULSE: u16 = 2500;

    // 限制角度在0到180度之间
    let angle = angle.clamp(0, 180);

    // 计算脉冲宽度
    let pulse_width =
        ((angle as f32 / 180.0) * (MAX_PULSE as f32 - MIN_PULSE as f32) + MIN_PULSE as f32) as u16;

    // 将脉冲宽度转换为占空比
    let duty_cycle = (pulse_width as f32 / 20000.0 * 4096 as f32) as u16;

    // 打印占空比
    debug!("Duty cycle: {}", duty_cycle);

    duty_cycle
}

unsafe fn set_servo(channel: u8, angle1: u16) {
    let Duty_channel1 = set_servo_angle(angle1);
    write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * channel, 0 & 0xFF);
    write_byte_data(PCA9685_ADDRESS, LED0_ON_L + 4 * channel + 1, 0 >> 8);
    write_byte_data(
        PCA9685_ADDRESS,
        LED0_ON_L + 4 * channel + 2,
        Duty_channel1 & 0xFF,
    );
    write_byte_data(
        PCA9685_ADDRESS,
        LED0_ON_L + 4 * channel + 3,
        Duty_channel1 >> 8,
    );
}

unsafe fn release() {
    write_byte_data(PCA9685_ADDRESS, MODE1, 0x00);
}

unsafe fn traffic_light_change() {
    set_servo(12, 120);
    set_servo(13, 90);
}

unsafe fn traffic_light_release() {
    let release_angle1 = 90;
    let release_angle2 = 85;
    set_servo(12, release_angle1);
    set_servo(13, release_angle2);
}

unsafe fn servo_follow() {
    set_servo(12, 90)
}

unsafe fn servo_poss() {
    set_servo(12, 30);
}

unsafe fn servo_map() {
    set_servo(12, 105);
}

unsafe fn FT_Turn(L: u16, R: u16) {
    status_control(1, -1, 1, -1);
    set_pwm(L, R, L, R);
}

pub fn test_pca() {
    unsafe {
        pca_init(2500, 2500, 2500, 2500);
        debug!("start");
        loop {
            Car_run_Task(1);
            busy_wait(Duration::from_millis(5000));
            Car_run_Task(2);
            busy_wait(Duration::from_millis(5000));
            Car_run_Task(3);
            busy_wait(Duration::from_millis(5000));
            Car_run_Task(4);
            busy_wait(Duration::from_millis(5000));
            Car_run_Task(5);
            busy_wait(Duration::from_millis(5000));
            Car_run_Task(6);
            busy_wait(Duration::from_millis(5000));
            Car_run_Task(7);
            busy_wait(Duration::from_millis(5000));
            Car_run_Task(8);
            busy_wait(Duration::from_millis(5000));
            Car_run_Task(9);
            busy_wait(Duration::from_millis(5000));
            Car_run_Task(10);
            busy_wait(Duration::from_millis(5000));
            Car_run_Task(11);
            busy_wait(Duration::from_millis(5000));
            Car_run_Task(12);
            busy_wait(Duration::from_millis(5000));
        }
    }
}
