pub mod ch341;

pub enum DeviceStateMachine {
    FetchingVersion,
    CH341Setup,
    CH341State,
    Opening
}

pub enum SendingWaitingWithCountStateMachine {
    Sending,
    Waiting(usize)
}