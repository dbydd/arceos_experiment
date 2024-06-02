use core::alloc::Layout;

use alloc::vec::Vec;
use axalloc::{global_no_cache_allocator, GlobalNoCacheAllocator};
use axhal::mem::{self, phys_to_virt, VirtAddr};
use conquer_once::spin::OnceCell;
use futures_intrusive::buffer;
use log::debug;
use page_box::PageBox;
use spinning_top::Spinlock;

use crate::host::structures::scratchpad;

use super::{
    registers,
    xhci_command_manager::COMMAND_MANAGER,
    xhci_slot_manager::{self, SLOT_MANAGER},
};

pub(crate) static SCRATCH_PAD: OnceCell<Spinlock<ScratchPad>> = OnceCell::uninit();

struct ScratchPad {
    // buffer: PageBox<[[usize; mem::PAGE_SIZE_4K]]>,
    array: PageBox<[u64]>,
    bufs: Vec<PageBox<[u8]>, GlobalNoCacheAllocator>,
}

impl ScratchPad {
    fn allocate_buffers(&mut self) {
        let layout = Layout::from_size_align(Self::page_size(), Self::page_size());
        let layout = layout.unwrap_or_else(|_| {
            panic!(
                "Failed to create a layout for {} bytes with {} bytes alignment",
                Self::page_size(),
                Self::page_size()
            )
        });

        for _ in 0..num_of_buffers() {
            let b = PageBox::from_layout_zeroed(layout);

            self.bufs.push(b);
        }
    }

    fn write_buffer_addresses(&mut self) {
        let page_size = Self::page_size();
        for (x, buf) in self.array.iter_mut().zip(self.bufs.iter()) {
            *x = buf.virt_addr().align_up(page_size).as_usize() as u64;
        }
    }

    fn page_size() -> usize {
        registers::handle(|r| r.operational.pagesize.read_volatile().get()) as usize
    }

    fn register_with_dcbaa(&self) {
        SLOT_MANAGER
            .get()
            .unwrap()
            .lock()
            .assign_device(0, self.array.virt_addr())
    }
}

pub fn new() {
    let max_scratch_pad_buffers = num_of_buffers();
    let mut scratchpad = ScratchPad {
        array: PageBox::new_slice(0, max_scratch_pad_buffers),
        bufs: Vec::new_in(global_no_cache_allocator()),
    };

    scratchpad.allocate_buffers();
    scratchpad.write_buffer_addresses();
    scratchpad.register_with_dcbaa();

    SCRATCH_PAD.init_once(move || Spinlock::new(scratchpad));

    debug!("initialized!");
}

fn num_of_buffers() -> usize {
    registers::handle(|r| {
        r.capability
            .hcsparams2
            .read_volatile()
            .max_scratchpad_buffers()
    }) as usize
}
