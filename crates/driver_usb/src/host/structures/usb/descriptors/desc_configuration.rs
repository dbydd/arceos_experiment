#[derive(Copy, Clone, Debug, Default)]
#[repr(C, packed)]
pub(crate) struct Configuration {
    length: u8,
    ty: u8,
    total_length: u16,
    num_interfaces: u8,
    config_val: u8,
    config_string: u8,
    attributes: u8,
    max_power: u8,
}
impl Configuration {
    pub(crate) fn config_val(&self) -> u8 {
        self.config_val
    }
}
