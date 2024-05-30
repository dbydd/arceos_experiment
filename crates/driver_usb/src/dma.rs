use core::{
    alloc::{Allocator, Layout},
    mem::size_of,
    ops::{Deref, DerefMut},
    ptr::{slice_from_raw_parts, NonNull},
};

use log::debug;

#[derive(Debug)]
pub struct DMAVec<A: Allocator, T> {
    layout: Layout,
    ptr: NonNull<[T]>,
    allocator: A,
}

unsafe impl<A: Allocator, T> Send for DMAVec<A, T> {}
unsafe impl<A: Allocator, T> Sync for DMAVec<A, T> {}

impl<A: Allocator, T> DMAVec<A, T> {
    /// DMAVec的新建方法。
    /// <br> size: 数组期望的元素数量。
    /// <br> align: 内存对齐的字节大小。
    /// <br> allocator: 用于数组内存分配和释放的分配器实例。
    /// <br> 返回一个初始化好的DMAVec实例。
    pub fn new(size: usize, align: usize, allocator: A) -> Self {
        //计算所需内存大小
        let buff_size = size * size_of::<T>();
        // 根据元素数量和对其要求创建内存布局
        let layout = Layout::from_size_align(buff_size, align).unwrap();
        // 使用分配器分配内存
        let buff = allocator.allocate(layout).unwrap();
        let ptr;
        unsafe {
            // 将分配的原始指针转换为T类型的切片指针，并确保其非空。
            let s = &*slice_from_raw_parts(buff.as_ptr() as *const T, size);
            ptr = NonNull::from(s);
        }
        Self {
            layout,
            ptr,
            allocator,
        }
    }
}

// 实现Deref trait，使得DMAVec可以像切片一样被使用。
impl<A: Allocator, T> Deref for DMAVec<A, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

// 实现DerefMut trait，使得DMAVec可以像切片一样被变相修改。
impl<A: Allocator, T> DerefMut for DMAVec<A, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.ptr.as_mut() }
    }
}

// 实现Drop trait，用于在DMAVec实例被销毁时释放其占用的内存。
impl<A: Allocator, T> Drop for DMAVec<A, T> {
    fn drop(&mut self) {
        unsafe {
            let ptr = self.ptr.cast::<u8>();
            self.allocator.deallocate(ptr, self.layout);
        }
    }
}
