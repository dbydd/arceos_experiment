#![no_std]
#![feature(allocator_api)]

use alloc::{collections::btree_map::BTreeMap, sync::Arc, vec, vec::Vec};
use events::{mouse::MouseEvent, EventData, EventHandler, Events};
use lazy_static::lazy_static;
use spinlock::SpinNoIrq;

extern crate alloc;

pub mod events;

lazy_static! {
    static ref EVENT_BUS: SpinNoIrq<EventBus> = SpinNoIrq::new(EventBus::new());
}

struct EventBus {
    bus: BTreeMap<Events, Vec<Arc<dyn EventHandler>>>,
}

impl EventBus {
    fn new() -> Self {
        Self {
            bus: BTreeMap::new(),
        }
    }
}

pub fn post_event(event: Events, mut data: EventData) -> bool {
    EVENT_BUS
        .lock()
        .bus
        .get(&event)
        .map(|handlers| !handlers.iter().any(|handler| !handler.handle(&mut data)))
        .unwrap_or(false)
}

pub fn register_handler(event: Events, handler: &Arc<dyn EventHandler>) {
    EVENT_BUS
        .lock()
        .bus
        .entry(event)
        .and_modify(|v| v.push(handler.clone()))
        .or_insert(vec![handler.clone()]);
}
