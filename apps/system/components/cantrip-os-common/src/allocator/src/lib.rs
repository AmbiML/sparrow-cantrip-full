// Copyright 2022 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! An allocator for Cantrip OS (derived from CortexM).

#![no_std]
#![cfg_attr(not(test), feature(alloc_error_handler))]

extern crate alloc;
use alloc::alloc::{alloc, dealloc};
use core::alloc::{GlobalAlloc, Layout};
use core::mem::size_of;
use core::ptr::{self, NonNull};
use linked_list_allocator::Heap;
use spin::Mutex;

pub struct CantripHeap {
    heap: Mutex<Heap>,
}

#[cfg(not(test))]
#[global_allocator]
pub static ALLOCATOR: CantripHeap = CantripHeap::empty();

#[cfg(not(test))]
#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    core::panic!("Global allocation failure: {:?}", layout);
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
    pub unsafe fn init(&self, start_addr: *mut u8, size: usize) {
        (*self.heap.lock()).init(start_addr, size);
    }

    /// Returns an estimate of the amount of bytes in use.
    pub fn used(&self) -> usize { (*self.heap.lock()).used() }

    /// Returns an estimate of the amount of bytes available.
    pub fn free(&self) -> usize { (*self.heap.lock()).free() }
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

// rust_{malloc, free, strdup} are drop in replacements for their C equivalents,
// callable from C. rust_ prefix to avoid symbols clashing with musllibc that's
// currently still linked in. musllibc is hard configured to malloc memory that's
// SIZE_ALIGN aligned. musllibc defines SIZE_ALIGN as:
// #define SIZE_ALIGN (4*sizeof(size_t))
// rust_malloc gives the same alignment guarantees. However rust's allocator APIs
// require Layout as argument to both 'malloc' and 'free'. Therefore an allocation
// is made SIZE_ALIGN aligned, and prepended with enouth bytes to fit a struct
// DeallocArgs before the allocation exposed to C. This way in rust_free, we can
// safely call dealloc from just the C pointer passed in. In practice the overhead
// per allocation is exactly SIZE_ALIGN = 16 bytes, as long as
// size_of::<DeallocArgs>() is at most 16.

#[derive(Copy, Clone)]
#[repr(C, align(16))]
struct DeallocArgs {
    layout: Layout,
    ptr: *mut u8,
}

#[no_mangle]
pub unsafe extern "C" fn rust_malloc(size: usize) -> *mut u8 {
    // musllibc malloc uses alignment defined as: #define SIZE_ALIGN (4*sizeof(size_t))
    let malloc_layout = Layout::from_size_align(size, 4 * size_of::<usize>()).unwrap();
    let (alloc_layout, offset) = Layout::new::<DeallocArgs>().extend(malloc_layout).unwrap();
    let alloc_ptr = alloc(alloc_layout);
    let malloc_ptr = alloc_ptr.add(offset);
    ptr::write(
        alloc_ptr as *mut DeallocArgs,
        DeallocArgs { layout: alloc_layout, ptr: alloc_ptr }
    );

    malloc_ptr
}

#[no_mangle]
pub unsafe extern "C" fn rust_free(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }

    let arg_ptr = (ptr as *mut DeallocArgs).sub(1);
    let arg = arg_ptr.read();
    assert!(arg.ptr == arg_ptr as *mut u8);
    dealloc(arg.ptr, arg.layout);
}

#[no_mangle]
pub unsafe extern "C" fn rust_strdup(ptr: *const u8) -> *mut u8 {
    let strlen = cstr_core::CStr::from_ptr(ptr).to_bytes().len();
    let dst_ptr = rust_malloc(strlen + 1);
    ptr::copy_nonoverlapping(ptr, dst_ptr, strlen + 1);

    dst_ptr
}
