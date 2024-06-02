use core::{
    borrow::{Borrow, BorrowMut},
    char::REPLACEMENT_CHARACTER,
    marker::PhantomData,
    mem,
};

use alloc::{format, vec::Vec};
use axhal::mem::VirtAddr;
use futures_util::stream::All;
use log::debug;
use page_box::PageBox;
use xhci::ring::trb::{
    self,
    transfer::{self, Allowed},
    Link,
};

use crate::host::structures::XHCI_LINK_TRB_CONTROL_TC;

use super::{
    event_ring, registers, XHCI_CONFIG_RING_SIZE, XHCI_TRB_CONTROL_C,
    XHCI_TRB_CONTROL_TRB_TYPE_SHIFT,
};

pub struct TransferRing {
    ring: PageBox<[[u32; 4]]>,
    enqueue_index: usize, // 入队索引
    deque_index: usize,   // 出队索引
    cycle_state: u32,     // 循环状态
}

impl TransferRing {
    /// 初始化命令环
    /// <br> 1. 初始化命令环，分配内存并设置初始化状态
    /// <br> 2. 配置Link TRB，设置环大小和循环状态
    pub fn new() -> Self {
        super::registers::handle(|r| {
            let mut transfer_ring = TransferRing {
                ring: PageBox::new_slice([0 as u32; 4], XHCI_CONFIG_RING_SIZE), //TODO 此处写死256，后续可更改
                enqueue_index: 0,
                deque_index: 0,
                cycle_state: XHCI_TRB_CONTROL_C as u32,
            };

            transfer_ring
        })
    }

    pub fn init(&mut self) {
        let get_trb_count = self.get_trb_count();
        let ringsize = self.ring.virt_addr().as_usize() as u64;
        let mut link_trb = &mut self.ring[get_trb_count - 1];

        // unsafe { *(link_trb.as_ptr() as *mut u64) = ringsize };
        // link_trb[2] = 0;
        // link_trb[3] = (((xhci::ring::trb::Type::Link as usize) << XHCI_TRB_CONTROL_TRB_TYPE_SHIFT)
        //     | XHCI_LINK_TRB_CONTROL_TC) as u32;
    }

    fn append_link_trb(&mut self) -> bool {
        let t = *Link::default().set_ring_segment_pointer(self.ring.virt_addr().as_usize() as u64);
        let mut t = transfer::Allowed::Link(t);
        self.set_cyclebit(&mut t);
        self.ring[self.enqueue_index] = t.into_raw();
        return t.cycle_bit();
    }

    pub fn enqueue_trbs(&mut self, ts: &[Allowed]) {
        ts.iter().for_each(|trb| self.enqueue(*trb));
    }

    // pub fn enqueue(&mut self, mut trb: transfer::Allowed) {
    //     self.set_cyclebit(&mut trb);
    //     if let Some(enque_trb) = self.get_enque_trb() {
    //         *enque_trb = trb.into_raw();
    //         self.inc_enque();
    //     }
    // }
    pub fn enqueue(&mut self, mut trb: transfer::Allowed) {
        self.set_cyclebit(&mut trb);
        if let Some(enque_trb) = self.get_enque_trb() {
            *enque_trb = trb.into_raw();
            self.inc_enque();
        }
    }

    fn set_cyclebit(&self, trb: &mut Allowed) {
        if self.cycle_state != 0 {
            trb.set_cycle_bit();
        } else {
            trb.clear_cycle_bit();
        }
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

    pub fn get_enque_trb(&mut self) -> Option<&mut [u32; 4]> {
        assert!(
            self.enqueue_index < self.get_trb_count(),
            "overflow! index: {},addr of index: {}",
            self.enqueue_index,
            (self.enqueue_index.borrow() as *const usize).addr()
        );
        let xhci_trb = &mut self.ring[self.enqueue_index];
        if (xhci_trb[3] & XHCI_TRB_CONTROL_C as u32) == self.cycle_state {
            return None;
        }

        Some(xhci_trb)
        // Allowed::try_from(xhci_trb).ok().as_mut()
    }

    pub fn inc_enque(&mut self) {
        assert!(self.enqueue_index < self.get_trb_count());
        assert_eq!(
            self.ring[self.enqueue_index][3] & XHCI_TRB_CONTROL_C as u32,
            self.cycle_state
        );
        self.enqueue_index += 1;
        debug!("enque_index: {}", self.enqueue_index);

        if self.enqueue_index == self.get_trb_count() - 1 {
            let mut xhci_trb = self.ring[self.enqueue_index];

            xhci_trb[3] ^= (XHCI_TRB_CONTROL_C as u32);

            if (xhci_trb[3] & XHCI_LINK_TRB_CONTROL_TC as u32) != 0 {
                self.cycle_state ^= (XHCI_TRB_CONTROL_C as u32)
            }
        }
    }

    pub fn cycle_state(&self) -> u32 {
        self.cycle_state
    }
}
