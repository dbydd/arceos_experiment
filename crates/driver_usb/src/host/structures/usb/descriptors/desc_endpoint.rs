#[derive(Copy, Clone, Default, Debug)]
#[repr(C, packed)]
pub(crate) struct Endpoint {
    len: u8,
    descriptor_type: u8,
    pub(crate) endpoint_address: u8,
    pub(crate) attributes: u8,
    pub(crate) max_packet_size: u16,
    pub(crate) interval: u8,
}
impl Endpoint {
    pub(crate) fn endpoint_type(&self) -> EndpointType {
        EndpointType::from_u8(if self.attributes == 0 {
            4
        } else {
            self.attributes.get_bits(0..=1)
                + if self.endpoint_address.get_bit(7) {
                    4
                } else {
                    0
                }
        })
        .expect("EndpointType must be convertible from `attributes` and `endpoint_address`.")
    }

    pub(crate) fn interval(&self) -> u8 {}

    pub(crate) fn max_streams(&self) -> Option<u8> {
        //TODO: complete me
        // if self.is_bulk_out() {}
        None
    }

    pub(crate) fn mult(&self) -> u8 {
        // if !lec && self.endpoint_type() == EndpointType::IsochOut {
        //     self.ssc
        //         .as_ref()
        //         .map(|ssc| ssc.attributes & 0x3)
        //         .unwrap_or(0)
        // } else {
        //     0
        // }
        0 //TODO: complete me
    }

    pub(crate) fn doorbell_value_aka_dci(&self) -> u32 {
        2 * u32::from(self.endpoint_address.get_bits(0..=3))
            + self.endpoint_address.get_bit(7) as u32
    }
}
