use core::marker::PhantomData;

use xhci::ring::trb::event::CompletionCode;

use crate::abstractions::PlatformAbstractions;

pub struct UCB<O>
where
    O: PlatformAbstractions,
{
    //UCB A.K.A Usb Complete Block
    pub code: CompleteCode,
    _phantom_data: PhantomData<O>,
}

impl<O> UCB<O>
where
    O: PlatformAbstractions,
{
    pub fn new(code: CompleteCode) -> Self {
        Self {
            code,
            _phantom_data: PhantomData,
        }
    }
}

#[derive(Debug)]
pub enum CompleteCode {
    Event(TransferEventCompleteCode),
}

#[derive(Debug)]
pub enum TransferEventCompleteCode {
    Success,
    Halt,
    Unknown(u8),
}
