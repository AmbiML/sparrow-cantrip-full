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

//! Cantrip OS CAmkES component libc compatibility glue.

extern crate alloc;
use alloc::alloc::{alloc, dealloc, Layout};
#[cfg(debug_assertions)]
use core::cmp;
use core::ffi::VaList;
use core::mem::size_of;
use core::ptr;
use cty::{c_char, c_int, size_t};
#[cfg(debug_assertions)]
use printf_compat;
use sel4_sys::seL4_DebugPutChar;

// libc glue, called by the C portion of CAmkES. A standard libc implementation,
// such as musllibc, is heavyweight so for code size reasons the few used
// functions are implemented here instead. malloc/free/strndup are called by
// CAmkES RPC, whereas printf/vsnprintf/puts are only called by debug logging
// code (ZF_LOG) and fault handlers. printf and vsnprintf are therefore only
// functional in debug builds to reduce code size. abort() is called by
// assert().
// musllibc's malloc is hard configured to malloc memory that's SIZE_ALIGN
// aligned.
// #define SIZE_ALIGN (4*sizeof(size_t))
// This malloc gives the same alignment guarantees. However rust's allocator
// APIs require Layout as argument to both 'malloc' and 'free'. Therefore an
// allocation is made SIZE_ALIGN aligned, and prepended with enough bytes to fit
// a struct DeallocArgs before the allocation exposed to C. This way in free, we
// can safely call dealloc from just the C pointer passed in. In practice the
// overhead per allocation is exactly SIZE_ALIGN = 16 bytes, as long as
// size_of::<DeallocArgs>() is at most 16.

#[derive(Copy, Clone)]
#[repr(C, align(16))]
struct DeallocArgs {
    layout: Layout,
    ptr: *mut u8,
}

#[no_mangle]
pub unsafe extern "C" fn malloc(size: usize) -> *mut u8 {
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
pub unsafe extern "C" fn free(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }

    let arg_ptr = (ptr as *mut DeallocArgs).sub(1);
    let arg = arg_ptr.read();
    assert!(arg.ptr == arg_ptr as *mut u8);
    dealloc(arg.ptr, arg.layout);
}

#[no_mangle]
pub unsafe extern "C" fn strdup(ptr: *const u8) -> *mut u8 {
    let strlen = cstr_core::CStr::from_ptr(ptr).to_bytes().len();
    let dst_ptr = malloc(strlen + 1);
    ptr::copy_nonoverlapping(ptr, dst_ptr, strlen + 1);

    dst_ptr
}

// Requires a kernel with CONFIG_PRINTING.
fn output_str(str: &str) {
    for b in str.as_bytes() {
        unsafe {
            seL4_DebugPutChar(*b);
        }
    }
}

#[cfg(debug_assertions)]
struct DebugPutCharWriter {}

#[cfg(debug_assertions)]
impl core::fmt::Write for DebugPutCharWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        output_str(s);
        Ok(())
    }
}

// Debug builds only to reduce code size.
#[no_mangle]
#[cfg(debug_assertions)]
pub unsafe extern "C" fn printf(str: *const c_char, mut args: ...) -> c_int {
    let mut writer = DebugPutCharWriter {};
    let bytes_written = printf_compat::format(str, args.as_va_list(),
        printf_compat::output::fmt_write(&mut writer));

    bytes_written
}

// Does nothing in non debug builds for code size reasons.
#[no_mangle]
#[cfg(not(debug_assertions))]
pub unsafe extern "C" fn printf(_str: *const c_char, _args: ...) -> c_int {
    // NOP in non debug builds.
    0
}

#[cfg(debug_assertions)]
struct CharPtrWriter {
    ptr : *mut c_char,
    len : size_t,
    off : size_t,
}

#[cfg(debug_assertions)]
impl core::fmt::Write for CharPtrWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        if !self.ptr.is_null() && self.len > 0 {
            let to_copy = cmp::min(s.as_bytes().len(), self.len - self.off);
            unsafe {
                ptr::copy_nonoverlapping(s.as_ptr(), self.ptr.add(self.off), to_copy);
            }
            self.off += to_copy;
        }
        Ok(())
    }
}

#[no_mangle]
#[cfg(debug_assertions)]
pub unsafe extern "C" fn vsnprintf(str: *mut c_char, n: size_t, fmt: *const c_char, ap: VaList) -> c_int {
    let mut writer = CharPtrWriter {
        ptr: str,
        len: n,
        off: 0,
    };
    let bytes_written = printf_compat::format(fmt, ap,
        printf_compat::output::fmt_write(&mut writer));
    if !str.is_null() && n > 0 {
        ptr::write(str.add(cmp::min(writer.off, n - 1)), b'\0');  // NULL terminator.
    }

    // Can be > n if output was truncated.
    bytes_written
}

#[no_mangle]
#[cfg(not(debug_assertions))]
pub unsafe extern "C" fn vsnprintf(_str: *const c_char, _n: size_t, _fmt: *const c_char, _ap: VaList) -> c_int {
    /* NOP in non debug builds. */
    0
}

#[no_mangle]
pub unsafe extern "C" fn puts(str: *const c_char) -> c_int {
    let c_str = cstr_core::CStr::from_ptr(str);
    let r_str = c_str.to_str().unwrap();
    output_str(r_str);

    // return a nonnegative number on success.
    1
}

#[no_mangle]
pub unsafe extern "C" fn abort() {
    panic!("libc abort()");
}
