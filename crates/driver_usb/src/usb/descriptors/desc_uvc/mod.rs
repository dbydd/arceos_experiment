use num_derive::FromPrimitive;

pub mod uvc_endpoints;
pub mod uvc_interfaces;

#[derive(FromPrimitive, Copy, Clone, Debug, PartialEq)]
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
