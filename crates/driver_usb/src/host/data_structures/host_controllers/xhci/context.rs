use crate::abstractions::dma::DMA;
use crate::abstractions::{OSAbstractions, PlatformAbstractions};
use crate::host::data_structures::host_controllers::xhci::ring::Ring;
use crate::host::data_structures::host_controllers::ControllerArc;
use crate::{err::*, USBSystemConfig};
use alloc::alloc::Allocator;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::sync::Arc;
use alloc::{boxed::Box, vec::Vec};
use alloc::{format, vec};
use core::borrow::BorrowMut;
use core::{mem, num};
use log::debug;
use spinlock::SpinNoIrq;
use xhci::context::Input64Byte;
pub use xhci::context::{Device, Device64Byte, DeviceHandler};
const NUM_EPS: usize = 32;

pub struct DeviceContextList<O>
//SHOULD We Rearrange these code,and shatter these array into single device?
where
    O: PlatformAbstractions,
{
    config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
    pub dcbaa: DMA<[u64; 256], O::DMA>,
    pub device_out_context_list: Vec<DMA<Device64Byte, O::DMA>>,
    pub device_input_context_list: Vec<DMA<Input64Byte, O::DMA>>,
    pub transfer_rings: Vec<Vec<Ring<O>>>,
}

impl<O> DeviceContextList<O>
where
    O: PlatformAbstractions,
{
    pub fn new(max_slots: u8, config: Arc<SpinNoIrq<USBSystemConfig<O>>>) -> Self {
        let os = config.lock().os.clone();
        let a = os.dma_alloc();

        let mut dcbaa = DMA::new([0u64; 256], 4096, a.clone());
        let mut out_context_list = Vec::with_capacity(max_slots as _);
        let mut in_context_list = Vec::with_capacity(max_slots as _);
        for i in 0..max_slots as usize {
            let out_context = DMA::new(Device::new_64byte(), 4096, a.clone()).fill_zero();
            dcbaa[i] = out_context.addr() as u64;
            out_context_list.push(out_context);
            in_context_list.push(DMA::new(Input64Byte::new_64byte(), 4096, a.clone()).fill_zero());
        }
        let mut transfer_rings = Vec::with_capacity(max_slots as _);
        for _ in 0..transfer_rings.capacity() {
            transfer_rings.push(Vec::new());
        }

        Self {
            dcbaa,
            device_out_context_list: out_context_list,
            device_input_context_list: in_context_list,
            transfer_rings,
            config: config.clone(),
        }
    }

    pub fn dcbaap(&self) -> usize {
        self.dcbaa.as_ptr() as _
    }

    pub fn new_slot(
        &mut self,
        slot: usize,
        hub: usize,
        port: usize,
        num_ep: usize, // cannot lesser than 0, and consider about alignment, use usize
    ) {
        if slot > self.device_out_context_list.len() {
            panic!("slot {} > max {}", slot, self.device_out_context_list.len())
        }

        let os = self.config.lock().os.clone();

        let entries_per_page =
            self.config.lock().transfer_ring_size / mem::size_of::<ring::TrbData>();

        let trs = (0..num_ep)
            .into_iter()
            .map(|i| Ring::new(os.clone(), entries_per_page, true).unwrap())
            .collect();

        self.transfer_rings[slot] = trs;
    }
}

use tock_registers::interfaces::Writeable;
use tock_registers::register_structs;
use tock_registers::registers::{ReadOnly, ReadWrite, WriteOnly};

use super::ring;

register_structs! {
    ScratchpadBufferEntry{
        (0x000 => value_low: ReadWrite<u32>),
        (0x004 => value_high: ReadWrite<u32>),
        (0x008 => @END),
    }
}

impl ScratchpadBufferEntry {
    pub fn set_addr(&mut self, addr: u64) {
        self.value_low.set(addr as u32);
        self.value_high.set((addr >> 32) as u32);
    }
}

pub struct ScratchpadBufferArray<O>
where
    O: OSAbstractions,
{
    pub entries: DMA<[ScratchpadBufferEntry], O::DMA>,
    pub pages: Vec<DMA<[u8], O::DMA>>,
}

unsafe impl<O: OSAbstractions> Sync for ScratchpadBufferArray<O> {}

impl<O> ScratchpadBufferArray<O>
where
    O: OSAbstractions,
{
    pub fn new(entries: u32, os: O) -> Self {
        let page_size = O::PAGE_SIZE;
        let align = 64;

        let mut entries: DMA<[ScratchpadBufferEntry], O::DMA> =
            DMA::zeroed(entries as usize, align, os.dma_alloc());

        let pages = entries
            .iter_mut()
            .map(|entry| {
                let dma = DMA::zeroed(page_size, align, os.dma_alloc());

                assert_eq!(dma.addr() % page_size, 0);
                entry.set_addr(dma.addr() as u64);
                dma
            })
            .collect();

        Self { entries, pages }
    }
    pub fn register(&self) -> usize {
        self.entries.addr()
    }
}
