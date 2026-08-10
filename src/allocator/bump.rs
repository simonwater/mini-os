use super::{Locked, align_up};
use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr;

pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
    allocations: usize,
}

impl BumpAllocator {
    /// 创建一个新的空的bump分配器
    pub const fn new() -> Self {
        BumpAllocator {
            heap_start: 0,
            heap_end: 0,
            next: 0,
            allocations: 0,
        }
    }

    /// 用给定的堆边界初始化bump分配器
    /// 这个方法是不安全的，因为调用者必须确保给定
    /// 的内存范围没有被使用。同样，这个方法只能被调用一次。
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        self.next = heap_start;
    }
}

/// bump 分配器：
/// 1. 维护一个 next 指针，开始指向堆起始地址，每次分配内存时，next值增加相应的分配大小
/// 2. next 到达末尾，不再有可以分配的内存时，再次请求分配将导致内存不足错误。
/// 3. 维护一个已分配数量 allocations，每次调用 alloc 加1，调用 dealloc 减1。
/// 4. 当allocations计数为0时，next指针重置回指向堆起始地址
/// 局限： 只能一次性释放全部内存，意味着单个长期存在的分配就可以阻止内存重用。
unsafe impl GlobalAlloc for Locked<BumpAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut bump = self.lock(); // 获取可变引用
        let alloc_start = align_up(bump.next, layout.align());
        let alloc_end = match alloc_start.checked_add(layout.size()) {
            Some(end) => end,
            None => return ptr::null_mut(),
        };

        if alloc_end > bump.heap_end {
            ptr::null_mut() // 内存不足
        } else {
            bump.next = alloc_end;
            bump.allocations += 1;
            alloc_start as *mut u8
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        let mut bump = self.lock();
        bump.allocations -= 1;
        if bump.allocations == 0 {
            bump.next = bump.heap_start;
        }
    }
}
