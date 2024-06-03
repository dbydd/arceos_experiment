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
