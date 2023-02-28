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

//! IO support for reading input from an ObjDescBundle of page frames
//! contained in a top-level CNode.

#![no_std]

extern crate alloc;
use alloc::boxed::Box;
use cantrip_memory_interface::ObjDescBundle;
use cantrip_os_common::camkes::Camkes;
use cantrip_os_common::copyregion::CopyRegion;
use cantrip_os_common::cspace_slot::CSpaceSlot;
use cantrip_os_common::sel4_sys;
use core::ptr;
use core2::io::{Cursor, Read};

use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_PageBits;
use sel4_sys::seL4_Result;

use cantrip_io as io;

const PAGE_SIZE: usize = 1 << seL4_PageBits;

extern "C" {
    static mut UPLOAD: [u8; PAGE_SIZE]; // TODO(sleffler): may need dedicated region
}

/// Rx io trait that returns data from an ObjDescBundle of page frames.
pub struct Rx<'a> {
    src: &'a ObjDescBundle,
    cptr_iter: Box<dyn Iterator<Item = seL4_CPtr> + 'a>,
    cur_frame: Option<seL4_CPtr>,
    copyregion: CopyRegion<'a>,
    cursor: Cursor<&'a [u8]>,
    top_slot: CSpaceSlot,
}
impl<'a> Rx<'a> {
    pub fn new(src: &'a ObjDescBundle) -> Self {
        Camkes::debug_assert_slot_cnode("objdesc-client::new", &Camkes::top_level_path(src.cnode));
        Self {
            src,
            cptr_iter: Box::new(src.cptr_iter()),
            cur_frame: None,
            copyregion: unsafe {
                // NB: UPLOAD is page-aligned so safe to cast
                CopyRegion::new(ptr::addr_of_mut!(UPLOAD[0]) as _, PAGE_SIZE)
            },
            cursor: Cursor::new(&[]),
            top_slot: CSpaceSlot::new(),
        }
    }

    fn is_empty(&self) -> bool {
        // XXX can't enable Cursor::is_empty for some reason
        self.cursor.position() >= self.cursor.get_ref().len() as u64
    }

    // Unmaps any page from |copyregion| & moves the cap back to |src|.
    fn unmap_current(&mut self) -> seL4_Result {
        if let Some(cptr) = self.cur_frame {
            self.copyregion.unmap()?;
            self.top_slot
                .move_from(self.src.cnode, cptr, self.src.depth as _)?;
            self.cur_frame = None;
        }
        Ok(())
    }

    // Moves |cptr| to the top-level & maps it into |copyregion|.
    fn map_current(&mut self, cptr: seL4_CPtr) -> seL4_Result {
        self.top_slot
            .move_to(self.src.cnode, cptr, self.src.depth as _)?;
        Camkes::debug_assert_slot_frame(
            "objdesc-client::read",
            &Camkes::top_level_path(self.top_slot.slot),
        );

        // NB: drop recovers move if map fails
        self.copyregion.map(self.top_slot.slot)?;
        self.cur_frame = Some(cptr);

        // XXX need slice len to end of data? (cheat for now and let LineReader discard zero's)
        self.cursor = Cursor::new(self.copyregion.as_ref());

        Ok(())
    }
}
impl<'a> Drop for Rx<'a> {
    fn drop(&mut self) { let _ = self.unmap_current(); }
}
impl<'a> io::Read for Rx<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.is_empty() {
            // Unmap any current page, advance to the next
            // ObjDesc, and setup the cursor.
            self.unmap_current().or(Err(io::Error))?;
            if let Some(cptr) = self.cptr_iter.next() {
                self.map_current(cptr).or(Err(io::Error))?;
            } else {
                return Ok(0); // EOF
            }
        }
        self.cursor.read(buf).or(Err(io::Error))
    }
}
