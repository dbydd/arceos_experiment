use crate::driver_iic::i2c::*;

pub const FI2C_CONFIG_TBL: [FI2cConfig; 1] = [FI2cConfig {
    instance_id: 1,
    base_addr: 0x28012000,
    irq_num: 122,
    irq_priority: 0,
    ref_clk_hz: 50000000,
    work_mode: 0,
    slave_addr: 0,
    use_7bit_addr: true,
    speed_rate: 100000,
}];

pub fn fi2c_lookup_config(instance_id: u32) -> Option<FI2cConfig> {
    let mut ptr: Option<FI2cConfig> = None;

    for index in 0..1 {
        unsafe {
            if FI2C_CONFIG_TBL[index].instance_id == instance_id {
                ptr = Some(FI2C_CONFIG_TBL[index].clone());
                break;
            }
        }
    }

    ptr
}
