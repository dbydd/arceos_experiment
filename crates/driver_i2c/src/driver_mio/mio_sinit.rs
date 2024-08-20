use crate::driver_mio::mio::*;

pub static FMIO_CONFIG_TBL: [FMioConfig; 3] = [
    FMioConfig {
        instance_id: 0,
        func_base_addr: 0x28014000,
        irq_num: 124,
        mio_base_addr: 0x28015000,
    },
    FMioConfig {
        instance_id: 1,
        func_base_addr: 0x28016000,
        irq_num: 125,
        mio_base_addr: 0x28017000,
    },
    FMioConfig {
        instance_id: 2,
        func_base_addr: 0x28018000,
        irq_num: 126,
        mio_base_addr: 0x28019000,
    },
];

pub fn fmio_lookup_config(instance_id: u32) -> Option<FMioConfig> {
    assert!(instance_id < 16);

    for config in FMIO_CONFIG_TBL.iter() {
        if config.instance_id == instance_id {
            return Some(*config);
        }
    }

    None
}
