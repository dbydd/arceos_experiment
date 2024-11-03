pub mod dma;

use core::{alloc::Allocator, fmt::Debug, usize};

pub trait PlatformAbstractions: OSAbstractions + HALAbstractions {}

impl<A> PlatformAbstractions for A where A: OSAbstractions + HALAbstractions {}

pub trait OSAbstractions: Clone + Send + Sync + Sized {
    type VirtAddr: From<usize> + Into<usize> + Clone + Send + Sync;
    type DMA: Allocator + Send + Sync + Clone;
    const PAGE_SIZE: usize;
    fn dma_alloc(&self) -> Self::DMA;
    fn map_virt_to_phys(vaddr: Self::VirtAddr) -> usize;
    fn map_phys_to_virt(paddr: usize) -> Self::VirtAddr;
    fn map_addr_len_tuple((addr, len): (usize, usize)) -> (usize, usize) {
        (Self::map_virt_to_phys(addr.into()), len)
    }
}
// fn interrupt_handler();//ADD Interrupt feature?
pub trait HALAbstractions: Clone + Send + Sync + Sized {
    fn force_sync_cache();
}
