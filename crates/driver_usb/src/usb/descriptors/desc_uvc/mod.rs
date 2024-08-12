use const_enum::ConstEnum;
use num_derive::FromPrimitive;

pub mod uvc_endpoints;
pub mod uvc_interfaces;

#[derive(ConstEnum, Copy, Clone, Debug, PartialEq, FromPrimitive)]
#[allow(non_camel_case_types)]
#[repr(u8)]
pub(crate) enum UVCDescriptorTypes {
    UVCClassSpecUnderfined = 0x20,
    UVCClassSpecDevice = 0x21,
    UVCClassSpecConfiguration = 0x22,
    UVCClassSpecString = 0x23,
    UVCClassSpecInterface = 0x24,
    UVCClassSpecVideoControlInterruptEndpoint = 0x25,
}
