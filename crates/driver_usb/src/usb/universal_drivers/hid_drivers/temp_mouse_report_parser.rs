use alloc::vec::Vec;
use bit_field::BitField;
use log::{debug, trace};

use crate::abstractions::event::{MouseEvent, USBSystemEvent};

pub fn parse(buf: &Vec<u8>) -> USBSystemEvent {
    let left = buf[1].get_bit(0);
    let right = buf[1].get_bit(1);
    let middle = buf[1].get_bit(2);
    let dx = i16::from_ne_bytes(unsafe { buf[3..=4].try_into().unwrap() });
    let dy = i16::from_ne_bytes(unsafe { buf[5..=6].try_into().unwrap() });
    let wheel = buf[7] as i8;

    let mouse_event = MouseEvent {
        dx: dx as _,
        dy: dy as _,
        left,
        right,
        middle,
        wheel: wheel as _,
    };
    debug!("decoded:{:#?}", mouse_event);
    USBSystemEvent::MouseEvent(mouse_event)
}
