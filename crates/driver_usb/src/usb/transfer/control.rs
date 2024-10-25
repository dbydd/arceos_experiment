use alloc::vec::Vec;
use const_enum::ConstEnum;
use num_derive::ToPrimitive;
use xhci::ring::trb::transfer::Direction;

use crate::abstractions::{dma::DMA, PlatformAbstractions};

#[derive(Debug, Clone)]
pub struct ControlTransfer {
    pub request_type: bmRequestType,
    pub request: bRequest,
    pub index: u16,
    pub value: u16,
    pub data: Option<(usize, usize)>,
    pub report: bool,
}

#[derive(Debug, Clone)]
#[allow(non_camel_case_types)]
pub enum bRequest {
    Generic(StandardbRequest),
    DriverSpec(u8),
}

impl From<StandardbRequest> for bRequest {
    fn from(value: StandardbRequest) -> Self {
        Self::Generic(value)
    }
}

#[allow(non_camel_case_types)]
#[repr(u8)]
#[derive(Debug, Clone)]
pub enum StandardbRequest {
    GetStatus = 0,
    ClearFeature = 1,
    SetFeature = 3,
    SetAddress = 5,
    GetDescriptor = 6,
    SetDescriptor = 7,
    GetConfiguration = 8,
    SetConfiguration = 9,
    GetInterface = 10,
    SetInterface = 11,
    SynchFrame = 12,
    SetEncryption = 13,
    GetEncryption = 14,
    SetHandshake = 15,
    GetHandshake = 16,
    SetConnection = 17,
    SetSecurityData = 18,
    GetSecurityData = 19,
    SetWusbData = 20,
    LoopbackDataWrite = 21,
    LoopbackDataRead = 22,
    SetInterfaceDs = 23,
    GetFwStatus = 26,
    SetFwStatus = 27,
    SetSel = 48,
    SetIsochDelay = 49,
}

#[allow(non_camel_case_types)]
#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct bmRequestType {
    pub direction: Direction,
    pub transfer_type: DataTransferType,
    pub recipient: Recipient,
}

impl bmRequestType {
    pub fn new(
        direction: Direction,
        transfer_type: DataTransferType,
        recipient: Recipient,
    ) -> bmRequestType {
        bmRequestType {
            direction,
            transfer_type,
            recipient,
        }
    }
}

impl From<bmRequestType> for u8 {
    fn from(value: bmRequestType) -> Self {
        (value.direction as u8) << 7 | (value.transfer_type as u8) << 5 | value.recipient as u8
    }
}

#[derive(ConstEnum, Copy, Clone, Debug)]
#[repr(u8)]
pub enum DataTransferType {
    Standard = 0,
    Class = 1,
    Vendor = 2,
    Reserved = 3,
}

#[derive(ConstEnum, Copy, Clone, Debug)]
#[repr(u8)]
pub enum Recipient {
    Device = 0,
    Interface = 1,
    Endpoint = 2,
    Other = 3,
}
