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

extern crate alloc;
use alloc::vec;
use cantrip_memory_interface::cantrip_frame_alloc;
use cantrip_memory_interface::ObjDescBundle;
use cantrip_os_common::copyregion::CopyRegion;
use cantrip_os_common::sel4_sys;
use core::cmp;
use core::ptr;

use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_WordBits;

#[derive(Debug)]
pub enum UploadError {
    PageMap,
    PageUnmap,
    Malloc,
}

extern "C" {
    static SELF_CNODE: seL4_CPtr;
}

pub struct Upload<'a> {
    frames: ObjDescBundle, // Page frames
    copyregion: CopyRegion<'a>,
    next_free: usize, // Next available byte in mapped frame
}

impl<'a> Upload<'a> {
    pub fn new(region: &'a mut [u8]) -> Self {
        Self {
            frames: ObjDescBundle::new(
                // Collect frames in the top-level CNode for now
                unsafe { SELF_CNODE },
                seL4_WordBits as u8,
                vec![],
            ),
            copyregion: unsafe { CopyRegion::new(region) },
            next_free: 0,
        }
    }
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        (self.frames.count() * self.copyregion.size())
            - (self.copyregion.mapped_bytes() - self.next_free)
    }
    pub fn finish(&mut self) { self.unmap_current_frame().expect("finish"); }
    pub fn frames(&self) -> &ObjDescBundle { &self.frames }
    pub fn frames_mut(&mut self) -> &mut ObjDescBundle { &mut self.frames }

    // Unmap the current page and reset state.
    fn unmap_current_frame(&mut self) -> Result<(), UploadError> {
        if self.frames.objs.last().is_some() {
            self.copyregion.unmap().or(Err(UploadError::PageUnmap))?;
        }
        // Try to combine this frame w/ the previous so large input
        // data streams don't generate many singleton ObjDesc's.
        self.frames.maybe_combine_last();

        self.next_free = 0;
        Ok(())
    }

    // Expand storage and map the new frame into our VSpace.
    fn expand_and_map(&mut self) -> Result<(), UploadError> {
        let new_page = cantrip_frame_alloc(self.copyregion.size()).or(Err(UploadError::Malloc))?;
        // Verify the new frame is in the same CNode as previous.
        assert_eq!(new_page.cnode, self.frames.cnode);
        assert_eq!(new_page.depth, self.frames.depth);
        self.frames.objs.push(new_page.objs[0]);

        let frame = &self.frames.objs.last().unwrap();
        self.copyregion
            .map(frame.cptr)
            .or(Err(UploadError::PageMap))?;
        self.next_free = 0;
        Ok(())
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<usize, UploadError> {
        let mut cursor = buf;
        while !cursor.is_empty() {
            let available_bytes = self.copyregion.mapped_bytes() - self.next_free;
            if available_bytes > 0 {
                // Fill the current frame (as space permits).
                let region = self.copyregion.as_mut();
                let bytes_to_write = cmp::min(available_bytes, cursor.len());
                unsafe {
                    ptr::copy_nonoverlapping(
                        cursor.as_ptr(),
                        region[self.next_free..].as_mut_ptr(),
                        bytes_to_write,
                    )
                };
                self.next_free += bytes_to_write;
                cursor = &cursor[bytes_to_write..];

                assert!(self.next_free <= self.copyregion.mapped_bytes());
                if self.next_free == self.copyregion.mapped_bytes() {
                    // Current frame is full; unmap and prepare for next.
                    self.unmap_current_frame()?;
                }
            }
            if cursor.is_empty() {
                break;
            }

            // Allocate another frame and map it for write.
            self.expand_and_map()?;
        }
        Ok(buf.len())
    }
}
