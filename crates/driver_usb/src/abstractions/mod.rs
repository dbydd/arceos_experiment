pub mod dma;
pub mod event;

use core::{alloc::Allocator, fmt::Debug};

use event::USBSystemEvent;

// pub trait PlatformAbstractions: Clone + Send + Sync + Sized {
//     type VirtAddr;
//     const PAGE_SIZE: usize;
//     type DMA: Allocator + Send + Sync + Clone;
//     fn dma_alloc(&self) -> Self::DMA;
//     fn force_sync_cache();
// }

pub trait PlatformAbstractions: OSAbstractions + HALAbstractions {}

impl<A> PlatformAbstractions for A where A: OSAbstractions + HALAbstractions {}

pub trait OSAbstractions: Clone + Send + Sync + Sized {
    type VirtAddr: From<usize> + Into<usize> + Clone + Send + Sync;
    type DMA: Allocator + Send + Sync + Clone;
    const PAGE_SIZE: usize;
    fn dma_alloc(&self) -> Self::DMA;
    fn send_event(&self, event: USBSystemEvent);
}
pub trait HALAbstractions: Clone + Send + Sync + Sized {
    fn force_sync_cache();
}
