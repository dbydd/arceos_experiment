use const_enum::ConstEnum;

#[derive(Copy, Clone, Debug, Default)]
#[repr(C, packed)]
pub struct Hid {
    pub len: u8,
    pub descriptor_type: u8,
    pub hid_bcd: u16,
    pub country_code: u8,
    pub num_descriptions: u8,
    pub report_descriptor_type: u8, // actually, these two entry is a variant length vector, but we only pick first entry!
    pub report_descriptor_len: u16, //
}

#[derive(ConstEnum, Copy, Clone, Debug)]
#[repr(u8)]
pub enum USBHIDSubclassDescriptorType {
    None = 0,
    BootInterface = 1,
}

#[derive(ConstEnum, Copy, Clone, Debug)]
#[allow(non_camel_case_types)]
#[repr(u8)]
pub enum USBHIDProtocolDescriptorType {
    None = 0,
    KeyBoard = 1,
    Mouse = 2,
}

#[derive(ConstEnum, Copy, Clone, Debug, PartialEq)]
#[allow(non_camel_case_types)]
#[repr(u8)]
pub(crate) enum HIDDescriptorTypes {
    //HID
    Hid = 0x21,
    HIDReport = 0x22,
    HIDPhysical = 0x23,
}
