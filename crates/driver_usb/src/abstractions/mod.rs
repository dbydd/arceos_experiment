pub mod dma;

use core::{alloc::Allocator, fmt::Debug};

pub trait PlatformAbstractions: OSAbstractions + HALAbstractions {}

impl<A> PlatformAbstractions for A where A: OSAbstractions + HALAbstractions {}

pub trait OSAbstractions: Clone + Send + Sync + Sized {
    type VirtAddr: From<usize> + Into<usize> + Clone + Send + Sync;
    type DMA: Allocator + Send + Sync + Clone;
    const PAGE_SIZE: usize;
    fn dma_alloc(&self) -> Self::DMA;
    // fn interrupt_handler();//ADD Interrupt feature?
}
pub trait HALAbstractions: Clone + Send + Sync + Sized {
    fn force_sync_cache();
}
