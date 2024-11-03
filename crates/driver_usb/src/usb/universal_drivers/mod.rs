use crate::abstractions::PlatformAbstractions;

pub mod hid_drivers;
pub mod uvc_drivers;

#[derive(Debug)]
pub enum BasicSendReceiveStateMachine {
    Waiting,
    Sending,
}

#[derive(Debug)]
pub enum BasicDriverLifeCycleStateMachine {
    BeforeFirstSendAkaPreparingForDrive,
    Driving,
    Ending,
    Sleeping,
}
