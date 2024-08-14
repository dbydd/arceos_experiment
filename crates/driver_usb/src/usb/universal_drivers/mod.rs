pub mod hid_drivers;
pub mod uvc_drivers;
pub mod ch341_drivers;
pub enum BasicSendReceiveStateMachine {
    Waiting,
    Sending,
}

pub enum BasicDriverLifeCycleStateMachine {
    BeforeFirstSendAkaPreparingForDrive,
    Driving,
    Ending,
    Sleeping,
}


