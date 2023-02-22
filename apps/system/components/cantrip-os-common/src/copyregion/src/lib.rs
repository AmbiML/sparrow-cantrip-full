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

//! RAII wrapper for using a CantripOS copyregion object.

#![no_std]
#![allow(non_camel_case_types)]

use core::mem::size_of;

use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_CapRights;
use sel4_sys::seL4_Default_VMAttributes;
use sel4_sys::seL4_Page_Map;
use sel4_sys::seL4_Page_Unmap;
use sel4_sys::seL4_Result;
use sel4_sys::seL4_Word;

extern "C" {
    static SELF_VSPACE_ROOT: seL4_CPtr;
}

// Sample usage:
// let mut copy_region = CopyRegion::new(unsafe { ptr::addr_of_mut!(LOAD_APPLICATION[0])}, PAGE_SIZE);
// copy_region.map(frame.cptr)?;
// copy_region.as_mut()[..].fill(0);
// let start = if index > 0 { 0 } else { vaddr - data_range.start };
// let end = cmp::min(data_range.end - vaddr, copy_region.size());
// image.read_exact(&mut copy_region.as_mut()[start..end])
//      .or(Err(seL4_Error::seL4_NoError))?; // XXX
// copy_region.unmap()?;

// TODO(sleffler): do we need to parameterize VM_Attributes & CapRights?
// TODO(sleffler): Mutex-wrapped & maybe RefCell-wrapped versions?

pub struct CopyRegion {
    region: *mut seL4_Word,
    size: usize,
    cur_frame: Option<seL4_CPtr>,
}
impl CopyRegion {
    pub fn new(region: *mut seL4_Word, size: usize) -> Self {
        CopyRegion {
            region,
            size,
            cur_frame: None,
        }
    }

    // Returns the region size in bytes.
    pub fn size(&self) -> usize { self.size }

    // Returns the region size if mapped, otherwise 0.
    pub fn mapped_bytes(&self) -> usize {
        if self.cur_frame.is_some() {
            self.size
        } else {
            0
        }
    }

    // Returns an immutable [u8] ref to the mapped region.
    pub fn as_ref(&mut self) -> &[u8] {
        assert!(self.cur_frame.is_some());
        unsafe { core::slice::from_raw_parts(self.region as _, self.size) }
    }

    // Returns a mutable [u8] ref to the mapped region.
    pub fn as_mut(&mut self) -> &mut [u8] {
        assert!(self.cur_frame.is_some());
        unsafe { core::slice::from_raw_parts_mut(self.region as _, self.size) }
    }

    // Returns an immutable [seL4_Word] ref to the mapped region.
    pub fn as_word_ref(&mut self) -> &[seL4_Word] {
        assert!(self.cur_frame.is_some());
        unsafe { core::slice::from_raw_parts(self.region, self.size / size_of::<seL4_Word>()) }
    }

    // Returns a mutable [seL4_Word] ref to the mapped region.
    pub fn as_word_mut(&mut self) -> &mut [seL4_Word] {
        assert!(self.cur_frame.is_some());
        unsafe { core::slice::from_raw_parts_mut(self.region, self.size / size_of::<seL4_Word>()) }
    }

    // Maps the |frame| in the SELF_VSPACE_ROOT for r/w.
    // XXX need rights + attribs?
    pub fn map(&mut self, frame: seL4_CPtr) -> seL4_Result {
        unsafe {
            seL4_Page_Map(
                frame,
                SELF_VSPACE_ROOT,
                self.region as seL4_Word,
                // seL4_ReadWrite
                seL4_CapRights::new(
                    /*grant_reply=*/ 0, /*grant=*/ 0, /*read=*/ 1, /*write=*/ 1,
                ),
                seL4_Default_VMAttributes,
            )
        }?;
        self.cur_frame = Some(frame);
        Ok(())
    }

    // Unmaps the current frame, if any.
    pub fn unmap(&mut self) -> seL4_Result {
        if let Some(cptr) = self.cur_frame {
            #[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
            unsafe { sel4_sys::seL4_ARM_Page_Unify_Instruction(cptr, 0, self.size()) }?;

            unsafe { seL4_Page_Unmap(cptr) }?;
            self.cur_frame = None;
        }
        Ok(())
    }
}
impl Drop for CopyRegion {
    fn drop(&mut self) { self.unmap().expect("CopyRegion"); }
}
