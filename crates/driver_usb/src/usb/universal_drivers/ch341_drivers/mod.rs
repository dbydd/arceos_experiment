pub mod ch341;

pub enum DeviceStateMachine {
    FetchingVersion,
    CH341Setup,
    first,
    second,
    third,
    fourth,
    fifth,
    Opening
}

pub enum SendingWaitingWithCountStateMachine {
    Sending,
    Waiting(usize)
}