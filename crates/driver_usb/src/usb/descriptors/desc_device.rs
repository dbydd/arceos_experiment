use const_enum::ConstEnum;

#[derive(Copy, Clone, Default, Debug)]
#[repr(C, packed)]
pub(crate) struct Device {
    pub len: u8,
    pub descriptor_type: u8,
    pub cd_usb: u16,
    pub class: u8,
    pub subclass: u8,
    pub protocol: u8,
    pub max_packet_size0: u8,
    pub vendor: u16,
    pub product_id: u16,
    pub device: u16,
    pub manufacture: u8,
    pub product: u8,
    pub serial_number: u8,
    pub num_configurations: u8,
}
impl Device {
    pub(crate) fn max_packet_size(&self) -> u16 {
        if let (3, _) = self.version() {
            2_u16.pow(self.max_packet_size0.into())
        } else {
            self.max_packet_size0.into()
        }
    }

    fn version(&self) -> (u8, u8) {
        let cd_usb = self.cd_usb;

        (
            (cd_usb >> 8).try_into().unwrap(),
            (cd_usb & 0xff).try_into().unwrap(),
        )
    }
}

#[derive(ConstEnum, Copy, Clone, Debug)]
#[repr(u8)]
pub enum StandardUSBDeviceClassCode {
    ReferInterfaceDescriptor = 0x00,
    Audio = 0x01,
    CommunicationsAndCDCControl = 0x02,
    HID = 0x03,
    Physical = 0x05,
    Image = 0x06,
    Printer = 0x07,
    MassStorage = 0x08,
    Hub = 0x09,
    CDCData = 0x0A,
    SmartCard = 0x0B,
    ContentSecurity = 0x0D,
    Video = 0x0E,
    PersonalHealthcare = 0x0F,
    AudioVideoDevices = 0x10,
    BillboardDeviceClass = 0x11,
    USBTypeCBridge = 0x12,
    DiagnosticDevice = 0xDC,
    WirelessController = 0xE0,
    Miscellaneous = 0xEF,
    ApplicationSpecific = 0xFE,
    VendorSpecific = 0xFF,
}

// #[derive(Copy, Clone, Debug, ConstEnum)]
// #[repr(u8)]
// pub enum StandardUSBDeviceSubClassCode {
//     Common = 0x02,
//     Any = 0x03,
// }

// #[derive(ConstEnum, Copy, Clone, Debug)]
// #[repr(u8)]
// pub enum StandardUSBDeviceProtocol {
//     ReferInterfaceAssociationDescriptor = 0x01,
// }
