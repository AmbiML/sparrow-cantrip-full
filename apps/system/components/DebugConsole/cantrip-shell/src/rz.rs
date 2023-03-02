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

//! Wrapper types for fully-buffered ZMODEM receives.

// TODO(sleffler): maybe extract the page-at-a-time support to it's own crate

use alloc::vec;
use cantrip_memory_interface::cantrip_frame_alloc;
use cantrip_memory_interface::ObjDescBundle;
use cantrip_os_common::sel4_sys;
use core::cmp;
use core::ptr;
use crc::crc32;
use crc::Hasher32;

use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_CapRights;
use sel4_sys::seL4_Default_VMAttributes;
use sel4_sys::seL4_PageBits;
use sel4_sys::seL4_Page_Map;
use sel4_sys::seL4_Page_Unmap;
use sel4_sys::seL4_WordBits;

use cantrip_io as io;

#[derive(Debug)]
enum UploadError {
    PageMap,
    PageUnmap,
    Malloc,
}
impl From<UploadError> for io::Error {
    fn from(_err: UploadError) -> io::Error { io::Error }
}

// TODO(sleffler): use ObjDesc::size_bytes
const PAGE_SIZE: usize = 1 << seL4_PageBits;

extern "C" {
    static SELF_CNODE: seL4_CPtr;
    static SELF_VSPACE_ROOT: seL4_CPtr;
    static mut UPLOAD: [u8; PAGE_SIZE];
}

pub struct Upload {
    digest: crc32::Digest,
    frames: ObjDescBundle, // Page frames
    mapped_page: *mut u8,  // Currently mapped page frame
    mapped_bytes: usize,   // Bytes in mapped_frame, 0 =>'s no frame mapped
    next_free: usize,      // Next available byte in mapped frame
}

impl Upload {
    pub fn new() -> Self {
        Upload {
            digest: crc32::Digest::new(crc32::IEEE),
            frames: ObjDescBundle::new(
                // Collect frames in the top-level CNode for now
                unsafe { SELF_CNODE },
                seL4_WordBits as u8,
                vec![],
            ),
            mapped_page: unsafe { ptr::addr_of_mut!(UPLOAD[0]) },
            mapped_bytes: 0, // NB: nothing mapped
            next_free: 0,
        }
    }
    pub fn crc32(&self) -> u32 { self.digest.sum32() }
    pub fn len(&self) -> usize {
        (self.frames.count() * PAGE_SIZE) - (self.mapped_bytes - self.next_free)
    }
    pub fn finish(&mut self) { self.unmap_current_frame().expect("finish"); }
    #[allow(dead_code)]
    pub fn frames(&self) -> &ObjDescBundle { &self.frames }

    // Unmap the current page and reset state.
    fn unmap_current_frame(&mut self) -> Result<(), UploadError> {
        if let Some(frame) = &self.frames.objs.last() {
            unsafe { seL4_Page_Unmap(frame.cptr) }.or(Err(UploadError::PageUnmap))?;
        }
        // Try to combine this frame w/ the previous so large input
        // data streams don't generate many singleton ObjDesc's.
        self.frames.maybe_combine_last();

        self.mapped_bytes = 0;
        self.next_free = 0;
        Ok(())
    }

    // Expand storage and map the new frame into our VSpace.
    fn expand_and_map(&mut self) -> Result<(), UploadError> {
        let new_page = cantrip_frame_alloc(PAGE_SIZE).or(Err(UploadError::Malloc))?;
        // Verify the new frame is in the same CNode as previous.
        assert_eq!(new_page.cnode, self.frames.cnode);
        assert_eq!(new_page.depth, self.frames.depth);
        self.frames.objs.push(new_page.objs[0]);

        let frame = &self.frames.objs.last().unwrap();
        unsafe {
            seL4_Page_Map(
                /*sel4_page=*/ frame.cptr,
                /*seL4_pd=*/ SELF_VSPACE_ROOT,
                /*vaddr=*/ self.mapped_page as usize,
                seL4_CapRights::new(
                    // NB: RW 'cuz W-only silently gets upgraded by kernel
                    /*grant_reply=*/
                    0, /*grant=*/ 0, /*read=1*/ 1, /*write=*/ 1,
                ),
                seL4_Default_VMAttributes,
            )
        }
        .or(Err(UploadError::PageMap))?;
        self.mapped_bytes = PAGE_SIZE;
        self.next_free = 0;
        Ok(())
    }
}
impl Drop for Upload {
    fn drop(&mut self) { self.finish(); }
}

impl io::Write for Upload {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut cursor = buf;
        while !cursor.is_empty() {
            let available_bytes = self.mapped_bytes - self.next_free;
            if available_bytes > 0 {
                // Fill the current frame (as space permits).
                let region =
                    unsafe { core::slice::from_raw_parts_mut(self.mapped_page, self.mapped_bytes) };
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

                assert!(self.next_free <= self.mapped_bytes);
                if self.next_free == self.mapped_bytes {
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
        self.digest.write(buf); // Update crc32 calculation
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

/// Receives using ZMODEM and wraps the result as an Upload.
pub fn rz<R: io::BufRead, W: io::Write>(r: R, w: W) -> Result<Upload, io::Error> {
    let mut upload = Upload::new();

    // Turn off logging, since it goes to the UART and will cause the sender to
    // abort.
    let prior_log_level = log::max_level();
    log::set_max_level(log::LevelFilter::Off);

    zmodem::recv::recv(r, w, &mut upload)?;

    log::set_max_level(prior_log_level);
    Ok(upload)
}
