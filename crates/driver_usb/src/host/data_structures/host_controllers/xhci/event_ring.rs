use core::sync::atomic::{fence, Ordering};

pub use super::ring::{Ring, TrbData};
use crate::abstractions::dma::DMA;
use crate::abstractions::OSAbstractions;
use crate::err::*;
use log::debug;
use num_traits::Zero;
use tock_registers::interfaces::Writeable;
use tock_registers::register_structs;
use tock_registers::registers::{ReadOnly, ReadWrite, WriteOnly};
use xhci::extended_capabilities::hci_extended_power_management::Data;
use xhci::ring::trb::event::{CompletionCode, TransferEvent};
use xhci::ring::trb::{self, event::Allowed};

register_structs! {
    EventRingSte {
        (0x000 => addr_low: ReadWrite<u32>),
        (0x004 => addr_high: ReadWrite<u32>),
        (0x008 => size: ReadWrite<u16>),
        (0x00A => _reserved),
        (0x010 => @END),
    }
}

///yet another Ring buffer
pub struct EventRing<O>
where
    O: OSAbstractions,
{
    pub ring: Ring<O>,
    pub ste: DMA<[EventRingSte], O::DMA>,
}

impl<O> EventRing<O>
where
    O: OSAbstractions,
{
    pub fn new(os: O) -> Result<Self> {
        let a = os.dma_alloc();
        let mut ring = EventRing {
            ste: DMA::zeroed(1, 64, a),
            ring: Ring::new(os, 256, false)?,
        };
        ring.ring.cycle = true;
        ring.ste[0].addr_low.set(ring.ring.register() as u32);
        ring.ste[0]
            .addr_high
            .set((ring.ring.register() as u64 >> 32) as u32);
        ring.ste[0].size.set(ring.ring.trbs.len() as u16);

        Ok(ring)
    }

    /// 完成一次循环返回 true
    pub fn next(&mut self) -> Option<(Allowed, bool)> {
        let (data, flag) = self.ring.current_data();
        let data = unsafe {
            let mut out = [0u32; 4];
            for i in 0..out.len() {
                out[i] = data.as_ptr().offset(i as _).read_volatile();
            }
            out
        };

        let mut allowed = Allowed::try_from(data).ok()?;

        if flag != allowed.cycle_bit() {
            return None;
        }
        if let Allowed::TransferEvent(c) = allowed
            && let Ok(CompletionCode::Invalid) = c.completion_code()
        {
            return None;
        }

        fence(Ordering::SeqCst);

        let cycle = self.ring.inc_deque();
        Some((allowed, cycle))
    }

    pub fn erdp(&self) -> u64 {
        self.ring.register() & 0xFFFF_FFFF_FFFF_FFF0 //temporary bug, we need to fix up platforms issue about overrun!
    }

    ///there is still a lot of device didn't support iommu/DMA Page, so we just return phys
    ///address!
    pub fn erstba(&self) -> u64 {
        let ptr = &self.ste[0];
        O::map_virt_to_phys((ptr as *const EventRingSte as usize).into()) as u64
    }
}
