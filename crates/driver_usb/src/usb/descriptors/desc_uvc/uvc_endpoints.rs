use const_enum::ConstEnum;

#[derive(ConstEnum, Copy, Clone, Debug, PartialEq)]
#[allow(non_camel_case_types)]
#[repr(u8)]
pub(crate) enum UVCVideoClassEndpointSubtypes {
    UNDEFINED = 0x00,
    GENERAL = 0x01,
    ENDPOINT = 0x02,
    INTERRUPT = 0x03,
}

#[derive(Clone, Debug)]
#[allow(non_camel_case_types)]
pub struct UVCVideoControlInterruptEndpoint {
    len: u8,
    descriptor_type: u8,
    descriptor_sub_type: u8,
    max_transfer_size: u16,
}
