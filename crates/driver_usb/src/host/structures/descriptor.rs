use core::ptr;

use alloc::vec::Vec;
use bit_field::BitField;
use log::debug;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use page_box::PageBox;
use xhci::context::EndpointType;

#[derive(Copy, Clone, Debug)]
pub(crate) enum Descriptor {
    Device(Device),
    Configuration(Configuration),
    Str,
    Interface(Interface),
    Endpoint(Endpoint),
    Hid,
}
impl Descriptor {
    pub(crate) fn from_slice(raw: &[u8]) -> Result<Self, Error> {
        assert_eq!(raw.len(), raw[0].into());
        match FromPrimitive::from_u8(raw[1]) {
            Some(t) => {
                let raw: *const [u8] = raw;
                match t {
                    // SAFETY: This operation is safe because the length of `raw` is equivalent to the
                    // one of the descriptor.
                    Type::Device => Ok(Self::Device(unsafe { ptr::read(raw.cast()) })),
                    Type::Configuration => {
                        Ok(Self::Configuration(unsafe { ptr::read(raw.cast()) }))
                    }
                    Type::Str => Ok(Self::Str),
                    Type::Interface => Ok(Self::Interface(unsafe { ptr::read(raw.cast()) })),
                    Type::Endpoint => Ok(Self::Endpoint(unsafe { ptr::read(raw.cast()) })),
                    Type::Hid => Ok(Self::Hid),
                }
            }
            None => Err(Error::UnrecognizedType(raw[1])),
        }
    }
}

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

#[derive(Copy, Clone, Default, Debug)]
#[repr(C, packed)]
pub(crate) struct Interface {
    len: u8,
    descriptor_type: u8,
    interface_number: u8,
    alternate_setting: u8,
    num_endpoints: u8,
    interface_class: u8,
    interface_subclass: u8,
    interface_protocol: u8,
    interface: u8,
}
impl Interface {
    pub(crate) fn ty(&self) -> (u8, u8, u8) {
        (
            self.interface_class,
            self.interface_subclass,
            self.interface_protocol,
        )
    }
}

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
    pub(crate) fn endpoint_type(self) -> EndpointType {
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

    pub(crate) fn doorbell_value(self) -> u32 {
        2 * u32::from(self.endpoint_address.get_bits(0..=3))
            + self.endpoint_address.get_bit(7) as u32
    }
}

#[derive(FromPrimitive)]
pub(crate) enum Type {
    Device = 1,
    Configuration = 2,
    Str = 3,
    Interface = 4,
    Endpoint = 5,
    Hid = 33,
}

#[derive(Debug)]
pub(crate) enum Error {
    UnrecognizedType(u8),
}
pub(crate) struct RawDescriptorParser {
    raw: PageBox<[u8]>,
    current: usize,
    len: usize,
}
impl RawDescriptorParser {
    pub fn new(raw: PageBox<[u8]>) -> Self {
        let len = raw.len();

        Self {
            raw,
            current: 0,
            len,
        }
    }

    pub fn parse(&mut self) -> Vec<Descriptor> {
        let mut v = Vec::new();
        while self.current < self.len && self.raw[self.current] > 0 {
            match self.parse_first_descriptor() {
                Ok(t) => v.push(t),
                Err(e) => debug!("Unrecognized USB descriptor: {:?}", e),
            }
        }
        v
    }

    fn parse_first_descriptor(&mut self) -> Result<Descriptor, Error> {
        let raw = self.cut_raw_descriptor();
        Descriptor::from_slice(&raw)
    }

    fn cut_raw_descriptor(&mut self) -> Vec<u8> {
        let len: usize = self.raw[self.current].into();
        let v = self.raw[self.current..(self.current + len)].to_vec();
        self.current += len;
        v
    }
}
