use super::Locked;
use alloc::alloc::GlobalAlloc;
use alloc::alloc::Layout;
use core::{mem, ptr, ptr::NonNull};

struct ListNode {
    next: Option<&'static mut ListNode>,
}

/// 要使用的块大小
///
/// 各块大小必须为2的幂，因为它们同时被
/// 用作块内存对齐（对齐方式必须始终为2的幂）
const BLOCK_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];

fn list_index(layout: &Layout) -> Option<usize> {
    let required_block_size = layout.size().max(layout.align());
    BLOCK_SIZES.iter().position(|&s| s >= required_block_size)
}

/// 固定大小块分配器：
/// 1. 维护一个链表数组，每个链表中的块大小相同，使用2的幂作为块大小，最差情况会浪费一半的内存。
/// 2. 有一个后备分配器，原理同 LinkedListAllocator，但支持合并空闲块。
///     - 固定大小块不够时由后备分配器分配，释放时插回数组相应位置头部，下次分配时直接使用
///     - 超出最大块大小的块同样由后备分配器分配，释放时交由后备分配器释放。
pub struct FixedSizeBlockAllocator {
    list_heads: [Option<&'static mut ListNode>; BLOCK_SIZES.len()],
    fallback_allocator: linked_list_allocator::Heap, // 支持合并空闲块
}

impl FixedSizeBlockAllocator {
    pub const fn new() -> Self {
        const EMPTY: Option<&'static mut ListNode> = None;
        Self {
            list_heads: [EMPTY; BLOCK_SIZES.len()],
            fallback_allocator: linked_list_allocator::Heap::empty(),
        }
    }

    /// 用给定的堆边界初始化分配器
    ///
    /// 此函数是不安全的，因为调用者必须保证给定的堆边界是有效的且堆是
    /// 未使用的。此方法只能调用一次。
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        unsafe {
            self.fallback_allocator.init(heap_start, heap_size);
        }
    }

    /// 使用后备分配器分配
    fn fallback_alloc(&mut self, layout: Layout) -> *mut u8 {
        match self.fallback_allocator.allocate_first_fit(layout) {
            Ok(ptr) => ptr.as_ptr(),
            Err(_) => ptr::null_mut(),
        }
    }
}

unsafe impl GlobalAlloc for Locked<FixedSizeBlockAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();
        match list_index(&layout) {
            Some(index) => {
                match allocator.list_heads[index].take() {
                    Some(node) => {
                        allocator.list_heads[index] = node.next.take();
                        node as *mut ListNode as *mut u8
                    }
                    None => {
                        // 没有块存在于列表中 => 分配新块
                        let block_size = BLOCK_SIZES[index];
                        // 只有当所有块大小都是 2 的幂时才有效
                        let block_align = block_size;
                        let layout = Layout::from_size_align(block_size, block_align).unwrap();
                        allocator.fallback_alloc(layout)
                    }
                }
            }
            None => allocator.fallback_alloc(layout),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut allocator = self.lock();
        match list_index(&layout) {
            Some(index) => {
                let new_node = ListNode {
                    next: allocator.list_heads[index].take(),
                };
                // 验证块是否满足存储节点所需的大小和对齐方式要求
                assert!(mem::size_of::<ListNode>() <= BLOCK_SIZES[index]);
                assert!(mem::align_of::<ListNode>() <= BLOCK_SIZES[index]);
                let new_node_ptr = ptr as *mut ListNode;
                unsafe {
                    new_node_ptr.write(new_node);
                    allocator.list_heads[index] = Some(&mut *new_node_ptr);
                }
            }
            None => {
                let ptr = NonNull::new(ptr).unwrap();
                unsafe {
                    allocator.fallback_allocator.deallocate(ptr, layout);
                }
            }
        }
    }
}
