use core::fmt;
use core::fmt::Debug;
use core::mem;
use core::ptr::slice_from_raw_parts;

use crate::abstractions::dma::DMA;
use crate::abstractions::OSAbstractions;
use crate::err::*;
use alloc::boxed::Box;
use alloc::slice;
use alloc::vec;
use alloc::vec::Vec;
use axhal::cpu::this_cpu_id;
use log::debug;
use log::trace;
pub use xhci::ring::trb;
use xhci::ring::trb::command;
use xhci::ring::trb::command::Allowed;
use xhci::ring::trb::command::Noop;
use xhci::ring::trb::event;
use xhci::ring::trb::transfer;
use xhci::ring::trb::Link;
const TRB_LEN: usize = 4;
pub type TrbData = [u32; TRB_LEN];

///Util:Ring buffer
pub struct Ring<O: OSAbstractions> {
    link: bool,
    pub trbs: DMA<[TrbData], O::DMA>,
    pub i: usize,
    pub cycle: bool,
}

impl<O: OSAbstractions> Ring<O> {
    pub fn new(os: O, len: usize, link: bool) -> Result<Self> {
        let a = os.dma_alloc();
        let mut trbs = DMA::new_vec([0; TRB_LEN], len, 64, a);
        Ok(Self {
            trbs,
            i: 0,
            cycle: link,
            link,
        })
    }
    pub fn len(&self) -> usize {
        self.trbs.len()
    }

    fn get_trb(&self) -> &TrbData {
        unsafe { &self.trbs[self.i] }
    }

    pub fn register(&self) -> u64 {
        O::map_virt_to_phys((self.get_trb().as_ptr() as usize).into()) as u64
    }

    pub fn reset(&mut self) -> &mut Self {
        self.trbs.fill({
            let mut noop = Noop::default();
            if !self.cycle {
                noop.set_cycle_bit();
            }
            noop.into_raw()
        });
        self.i = 0;
        self.cycle = self.link;
        self
    }

    pub fn enque_command(&mut self, mut trb: command::Allowed) -> usize {
        if self.cycle {
            trb.set_cycle_bit();
        } else {
            trb.clear_cycle_bit();
        }
        let addr = self.enque_trb(trb.clone().into_raw());
        debug!("[CMD] >> {:?} @{:X}", trb, addr);
        addr
    }

    pub fn enque_transfer(&mut self, mut trb: transfer::Allowed) -> usize {
        if self.cycle {
            trb.set_cycle_bit();
        } else {
            trb.clear_cycle_bit();
        }
        let addr = self.enque_trb(trb.clone().into_raw());
        addr
    }

    pub fn enque_transfers(&mut self, mut trb: Vec<transfer::Allowed>) -> usize {
        trb.iter().map(|t| self.enque_transfer(*t)).last().unwrap()
    }

    pub fn enque_trb(&mut self, mut trb: TrbData) -> usize {
        self.trbs[self.i].copy_from_slice(&trb);
        let addr = self.trbs[self.i].as_ptr() as usize;
        debug!(
            "enqueued {} @{:#X}\n{:x}\n{:x}\n{:x}\n{:x}\n------------------------------------------------ current cycle:{}",
            self.i, addr, trb[0], trb[1], trb[2], trb[3],self.cycle
        );
        self.next_index();
        addr
    }

    pub fn enque_trbs(&mut self, trb: Vec<TrbData>) -> usize {
        let mut last = 0;
        for ele in trb {
            last = self.enque_trb(ele);
        }
        last
    }

    fn next_index(&mut self) -> usize {
        self.i += 1;
        let mut need_link = false;
        let len = self.len();

        // link模式下，最后一个是Link
        if self.link && self.i >= len - 1 {
            self.i = 0;
            need_link = true;
            trace!("flip and link!")
        } else if self.i >= len {
            self.i = 0;
        }

        if need_link {
            trace!("link!");
            let address = self.trbs[0].as_ptr() as usize;
            let mut link = Link::new();
            link.set_ring_segment_pointer(address as u64)
                .set_toggle_cycle();

            if self.cycle {
                link.set_cycle_bit();
            } else {
                link.clear_cycle_bit();
            }
            let trb = command::Allowed::Link(link);
            let link_trb = trb.into_raw();
            let mut this_trb = &mut self.trbs[len - 1];
            this_trb.copy_from_slice(&link_trb);

            self.cycle = !self.cycle;
        }

        self.i
    }

    /// 完成一次循环返回true
    pub fn inc_deque(&mut self) -> bool {
        self.i += 1;
        let mut is_cycle = false;
        let len = self.len();
        if self.link {
        } else {
            if self.i >= len {
                self.i = 0;
                self.cycle = !self.cycle;
                is_cycle = true;
            }
        }

        is_cycle
    }

    pub fn current_data(&mut self) -> (&TrbData, bool) {
        (&self.trbs[self.i], self.cycle)
    }

    pub fn get_len(&self) -> usize {
        self.trbs.len()
    }
}
