use crate::driver_iic::io::*;

pub fn FMIO_FUNC_STATE_MASK() -> u32 {
    ((!0u32) - (1u32 << (0)) + 1) & (!0u32 >> (32 - 1 - (1)))
}

pub fn FMioSelectFunc(addr: usize, mio_type: u32) -> bool {
    assert!(mio_type < 2);
    assert!(addr != 0);

    let reg_val = input_32(addr as u32, 0x04) & FMIO_FUNC_STATE_MASK();

    if mio_type == reg_val {
        return true;
    }

    output_32(addr as u32, 0x00, mio_type);

    true
}

pub fn FMioGetFunc(addr: usize) -> u32 {
    assert!(addr != 0);

    input_32(addr as u32, 0x04) & FMIO_FUNC_STATE_MASK()
}

pub fn FMioGetVersion(addr: usize) -> u32 {
    assert!(addr != 0);

    input_32(addr as u32, 0x100) & (((!0u32) - (1u32 << (0)) + 1) & (!0u32 >> (32 - 1 - (31))))
}
