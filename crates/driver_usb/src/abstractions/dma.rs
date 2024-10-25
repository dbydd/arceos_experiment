use core::{
    alloc::{AllocError, Allocator, Layout},
    fmt::Debug,
    marker::PhantomData,
    mem::size_of,
    ops::{Deref, DerefMut},
    ptr::{slice_from_raw_parts, slice_from_raw_parts_mut, NonNull},
};

use alloc::vec::Vec;
use axalloc::global_no_cache_allocator;
use log::debug;

///DMA: it used in multiple situation. Sometimes we need A bunch of Data Transfer, Sometimes we
///need nocache memory write operations. So we designed this struct.
pub struct DMA<T, A>
where
    T: ?Sized,
    A: Allocator,
{
    layout: Layout,
    data: NonNull<[u8]>,
    allocator: A,
    __marker: PhantomData<T>,
}

unsafe impl<T, A> Send for DMA<T, A>
where
    T: ?Sized,
    A: Allocator,
{
}

impl<T, A> DMA<T, A>
where
    T: Sized,
    A: Allocator,
{
    /// 从 `value` `align` 和 `allocator` 创建 DMA，
    /// 若不符合以下条件则 Panic `LayoutError`：
    ///
    /// * `align` 不能为 0，
    ///
    /// * `align` 必须是2的幂次方。
    pub fn new(value: T, align: usize, allocator: A) -> Self {
        //计算所需内存大小
        let buff_size = size_of::<T>();
        // 根据元素数量和对其要求创建内存布局
        let layout = Layout::from_size_align(buff_size, align).unwrap();
        // 使用分配器分配内存
        let mut data = allocator.allocate(layout).unwrap();
        let mut ptr = data.cast();
        unsafe {
            ptr.write(value);
        };
        Self {
            layout,
            data,
            allocator,
            __marker: PhantomData::default(),
        }
    }

    pub fn new_singleton_page4k(value: T, allocator: A) -> Self {
        Self::new(value, 4096, allocator)
    }

    pub fn reserve_with_0(align: usize, allocator: A) -> DMA<T, A> {
        //计算所需内存大小
        let buff_size = size_of::<T>();
        // 根据元素数量和对其要求创建内存布局
        let layout = Layout::from_size_align(buff_size, align).unwrap();
        // 使用分配器分配内存
        let mut data = allocator.allocate_zeroed(layout).unwrap();
        Self {
            layout,
            data,
            allocator,
            __marker: PhantomData::default(),
        }
    }

    pub fn fill_zero(mut self) -> Self {
        unsafe { self.data.as_mut().iter_mut().for_each(|u| *u = 0u8) }
        self
    }
}

impl<T, A> DMA<T, A>
where
    T: ?Sized,
    A: Allocator,
{
    /// 返回 [DMA] 地址
    pub fn addr(&self) -> usize {
        self.data.addr().into()
    }

    pub fn length_for_bytes(&self) -> usize {
        self.layout.size()
    }

    pub fn addr_len_tuple(&self) -> (usize, usize) {
        (self.addr(), self.length_for_bytes())
    }
}

impl<T, A> DMA<[T], A>
where
    T: Sized,
    A: Allocator,
{
    pub fn zeroed(count: usize, align: usize, allocator: A) -> Self {
        let t_size = size_of::<T>();
        let size = count * t_size;

        // 根据元素数量和对其要求创建内存布局
        let layout = Layout::from_size_align(size, align).unwrap();
        // 使用分配器分配内存
        let mut data = allocator.allocate(layout).unwrap();

        unsafe {
            for mut one in data.as_mut() {
                *one = 0;
            }
        }

        unsafe {
            Self {
                layout,
                data,
                allocator,
                __marker: PhantomData::default(),
            }
        }
    }

    pub fn new_vec(init: T, count: usize, align: usize, allocator: A) -> Self {
        let t_size = size_of::<T>();
        let size = count * t_size;

        // 根据元素数量和对其要求创建内存布局
        let layout = Layout::from_size_align(size, align).unwrap();
        // 使用分配器分配内存
        let mut data = allocator.allocate(layout).unwrap();
        // debug!("allocated data:{:?}", data);

        unsafe {
            for i in 0..count {
                let data = &mut data.as_mut()[i * t_size..(i + 1) * t_size];
                let t_ptr = &init as *const T as *const u8;
                let t_data = &*slice_from_raw_parts(t_ptr, t_size);
                data.copy_from_slice(t_data);
            }
        }

        unsafe {
            Self {
                layout,
                data,
                allocator,
                __marker: PhantomData::default(),
            }
        }
    }
}

impl<T, A> Deref for DMA<[T], A>
where
    A: Allocator,
{
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe {
            let len = self.data.len() / size_of::<T>();
            let ptr = self.data.cast::<T>();
            &*slice_from_raw_parts(ptr.as_ptr(), len)
        }
    }
}

impl<T, A> DerefMut for DMA<[T], A>
where
    A: Allocator,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            let len = self.data.len() / size_of::<T>();
            let mut ptr = self.data.cast::<T>().as_ptr();
            &mut *slice_from_raw_parts_mut(ptr, len)
        }
    }
}

impl<T, A> Deref for DMA<T, A>
where
    A: Allocator,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            let ptr = self.data.cast::<T>();
            ptr.as_ref()
        }
    }
}

impl<T, A> DerefMut for DMA<T, A>
where
    A: Allocator,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            let mut ptr = self.data.cast::<T>();
            ptr.as_mut()
        }
    }
}
impl<A, T> Drop for DMA<T, A>
where
    T: ?Sized,
    A: Allocator,
{
    fn drop(&mut self) {
        unsafe {
            let ptr = self.data.cast::<u8>();
            self.allocator.deallocate(ptr, self.layout);
        }
    }
}
