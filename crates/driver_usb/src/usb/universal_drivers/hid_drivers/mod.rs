use const_enum::ConstEnum;
use num_derive::{FromPrimitive, ToPrimitive};

pub mod hid_mouse;

#[derive(Copy, Clone, Debug, ToPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum USBHidDeviceSubClassCode {
    Mouse = 1,
}
