use alloc::string::String;
use core::fmt::{write, Display};
use xhci::ring::trb::event::CompletionCode;

/// USB System Related Errors
#[derive(Debug)]
pub enum Error {
    Unknown(String),
    Param(String),
    CMD(CompletionCode),
    Pip,
    TimeOut,
    DontDoThatOnControlPipe,
}

impl Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Unknown(msg) => write!(f, "unknown usb err: {}", msg),
            Error::Param(msg) => write!(f, "param err: {}", msg),
            Error::TimeOut => write!(f, "timeout"),
            Error::CMD(cmd) => write!(f, "cmd fail: {:#?}", cmd),
            Error::Pip => write!(f, "piped"),
            Error::DontDoThatOnControlPipe => {
                write!(f, "don't do that on controller pipe! illegal operation!")
            }
        }
    }
}

pub type Result<T = ()> = core::result::Result<T, Error>;
