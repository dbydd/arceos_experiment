use axhal::mem::{phys_to_virt, PhysAddr};
use core::alloc::Layout;
use core::clone::Clone;
use core::prelude::rust_2024::derive;
use log::info;
use xhci::accessor::Mapper;

const MMIO_BASE: usize = 0xFD50_8000;
// const MMIO_BASE: usize = 0x6_0000_0000;

#[derive(Clone)]
pub struct MemoryMapper;

impl Mapper for MemoryMapper {
    unsafe fn map(&mut self, phys_start: usize, bytes: usize) -> core::num::NonZeroUsize {
        // let global_allocator = axalloc::global_allocator();
        // let vaddr = global_allocator
        //     .alloc(Layout::from_size_align_unchecked(bytes, 8))
        //     .unwrap()
        //     .as_ptr() as usize;
        info!("phys_start,bytes: {:x},{:x},mapping", phys_start, bytes);
        let usize = phys_to_virt(PhysAddr::from(phys_start))
            // .align_up(bytes)
            .as_usize();
        info!("aligned:{:x}", usize);
        let non_zero_usize = core::num::NonZeroUsize::new(usize).unwrap();
        info!("mapped:{:x}", non_zero_usize);
        non_zero_usize
    }

    fn unmap(&mut self, virt_start: usize, bytes: usize) {}
}

impl MemoryMapper {
    pub fn init() -> MemoryMapper {
        MemoryMapper {}
    }
}

pub fn init() -> xhci::Registers<MemoryMapper> {
    info!("initing");
    let mut r = unsafe { xhci::Registers::new(MMIO_BASE, MemoryMapper::init()) };
    // r.capability.caplength.read_volatile();

    // let o = &mut r.operational;
    info!("inited");
    // o.usbcmd.update_volatile(|u| {
    //     u.set_run_stop();
    //     u.set_interrupter_enable();
    // });
    info!("updated");
    r
}
