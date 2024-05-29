use core::{char::REPLACEMENT_CHARACTER, marker::PhantomData, mem};

use alloc::vec::Vec;
use axhal::mem::VirtAddr;
use futures_util::stream::All;
use log::{debug, info};
use page_box::PageBox;
use xhci::ring::trb::{self, event::Allowed, Link};

use crate::host::structures::XHCI_LINK_TRB_CONTROL_TC;

use super::{registers, XHCI_CONFIG_RING_SIZE, XHCI_TRB_CONTROL_C};

pub type TypeXhciTrb = [u32; 4];

pub struct EvtRing {
    ring: PageBox<[[u32; 4]]>,
    enque_index: usize,
    deque_index: usize,
    cycle_state: u32,
}

impl EvtRing {
    pub fn new() -> Self {
        super::registers::handle(|r| {
            let event_ring = EvtRing {
                ring: PageBox::new_slice([0 as u32; 4], XHCI_CONFIG_RING_SIZE), //TODO 此处写死256，后续可更改
                enque_index: 0,
                deque_index: 0,
                cycle_state: XHCI_TRB_CONTROL_C as u32,
            };

            debug!(
                "created! event ring, check alignment of 4k: {:x}",
                event_ring.ring.virt_addr()
            );
            assert!(event_ring.ring.virt_addr().is_aligned_4k());

            event_ring
        })
    }

    pub fn get_trb_count(&self) -> usize {
        self.ring.len()
    }

    pub fn get_ring_addr(&self) -> VirtAddr {
        self.ring.virt_addr()
    }

    pub fn get_deque_trb(&self) -> Option<Allowed> {
        assert!(self.deque_index < self.get_trb_count());
        let xhci_trb = self.ring[self.deque_index];
        if (xhci_trb[3] & XHCI_TRB_CONTROL_C as u32) != self.cycle_state {
            return None;
        }

        Allowed::try_from(xhci_trb).ok()
    }

    pub fn get_enque_trb(&self) -> Option<Allowed> {
        assert!(self.enque_index < self.get_trb_count());
        let xhci_trb = self.ring[self.enque_index];
        if (xhci_trb[3] & XHCI_TRB_CONTROL_C as u32) == self.cycle_state {
            return None;
        }

        Allowed::try_from(xhci_trb).ok()
    }

    pub fn inc_deque(&mut self) -> Option<Allowed> {
        assert!(self.deque_index < self.get_trb_count());
        assert_eq!(
            self.ring[self.enque_index][3] & XHCI_TRB_CONTROL_C as u32,
            self.cycle_state
        );

        self.deque_index += 1;

        if self.deque_index == self.get_trb_count() {
            self.deque_index = 0;
            self.cycle_state ^= XHCI_TRB_CONTROL_C as u32;
        }

        Allowed::try_from(self.ring[self.deque_index]).ok()
    }
}
