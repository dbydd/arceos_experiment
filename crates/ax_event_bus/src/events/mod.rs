use alloc::{sync::Arc, vec::Vec};
use mouse::MouseEvent;

pub mod mouse;
pub enum EventData {
    MouseEvent(MouseEvent),
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum Events {
    MouseEvent,
}

pub trait EventHandler: Send + Sync {
    fn handle(&self, event: &mut EventData) -> bool;
}
