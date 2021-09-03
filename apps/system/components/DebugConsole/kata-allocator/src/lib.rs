//! An allocator for Cantrip OS (derived from CortexM).

#![no_std]
#![feature(alloc_error_handler)]

use core::alloc::{GlobalAlloc, Layout};
use core::panic;
use core::ptr::{self, NonNull};

use linked_list_allocator::Heap;
use spin::Mutex;

pub struct CantripHeap {
    heap: Mutex<Heap>,
}

#[global_allocator]
pub static ALLOCATOR: CantripHeap = CantripHeap::empty();

#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("Global allocation failure: {:?}", layout);
}

impl CantripHeap {
    /// Create a new UNINITIALIZED heap allocator. You must initialize this
    /// heap using the init method before using the allocator.
    pub const fn empty() -> CantripHeap {
        CantripHeap {
            heap: Mutex::new(Heap::empty()),
        }
    }

    /// Initializes the heap
    ///
    /// This function must be called BEFORE you run any code that makes use of the
    /// allocator.
    ///
    /// `start_addr` is the address where the heap will be located.
    ///
    /// `size` is the size of the heap in bytes.
    ///
    /// Note that:
    ///
    /// - The heap grows "upwards", towards larger addresses. Thus `end_addr` must
    ///   be larger than `start_addr`
    ///
    /// - The size of the heap is `(end_addr as usize) - (start_addr as usize)`. The
    ///   allocator won't use the byte at `end_addr`.
    ///
    /// # Safety
    ///
    /// Obey these or Bad Stuff will happen.
    ///
    /// - This function must be called exactly ONCE (per thread).
    /// - `size > 0`
    pub unsafe fn init(&self, start_addr: usize, size: usize) {
        (*self.heap.lock()).init(start_addr, size);
    }

    /// Returns an estimate of the amount of bytes in use.
    pub fn used(&self) -> usize {
        (*self.heap.lock()).used()
    }

    /// Returns an estimate of the amount of bytes available.
    pub fn free(&self) -> usize {
        (*self.heap.lock()).free()
    }
}

unsafe impl GlobalAlloc for CantripHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        (*self.heap.lock())
            .allocate_first_fit(layout)
            .ok()
            .map_or(ptr::null_mut(), |allocation| allocation.as_ptr())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        (*self.heap.lock()).deallocate(NonNull::new_unchecked(ptr), layout)
    }
}
