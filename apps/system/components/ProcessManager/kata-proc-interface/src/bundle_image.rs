//! Cantrip OS Bundle image loader.

use core::cmp;
use core::mem::size_of;
use core::ops::Range;
use core::ptr;
use cantrip_memory_interface::ObjDescBundle;
use cantrip_os_common::cspace_slot::CSpaceSlot;
use cantrip_os_common::sel4_sys;
use log::{error, trace};

use sel4_sys::seL4_CapRights;
use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_Default_VMAttributes;
use sel4_sys::seL4_Error;
use sel4_sys::seL4_PageBits;
use sel4_sys::seL4_Page_Map;
use sel4_sys::seL4_Page_Unmap;

use cantrip_io as io;
use io::Read;
use io::Seek;

// TODO(sleffler): use ObjDesc::size_bytes and support multiple page sizes
const PAGE_SIZE: usize = 1 << seL4_PageBits;

extern "C" {
    static SELF_VSPACE_ROOT: seL4_CPtr;
    static mut BUNDLE_IMAGE: [u8; PAGE_SIZE];
}

#[derive(Debug)]
#[allow(dead_code)] // until BadSection* are used
enum BundleImageError {
    PageMapFailed,
    PageUnmapFailed,
    PageNotFound,
    CapMoveFailed,
    BadSectionMagic,
    BadSectionCrc,
    BadSectionIO,
}
impl From<seL4_Error> for BundleImageError {
    fn from(_err: seL4_Error) -> BundleImageError {
        BundleImageError::CapMoveFailed
    }
}

// On-disk header format.
#[repr(packed)]
#[allow(dead_code)]
struct SectionHeader {
    magic: u64, // Magic number
    vaddr: u64, // Virtual address of section (bytes)
    entry: u64, // Entry point; valid only when SECTION_ENTRYPOINT is set in flags
    flags: u32, // See below
    fsize: u32, // Length of data that follows (bytes)
    msize: u32, // Size of memory region (bytes)
    align: u32, // Section data alignment (bytes)
    pad: u32, // <ignore, reserved for future use>
    crc32: u32, // CRC32 of the data that follows
}
const SECTION_MAGIC: u64 = 0x0405_1957_1014_1955;

const SECTION_READ: u32 = 0x1; // Data are readable
const SECTION_WRITE: u32 = 0x2; // Data are writeable
const SECTION_EXEC: u32 = 0x4; // Data are executable
const SECTION_ENTRYPOINT: u32 = 0x8; // Entry point valid

// In-memory (parsed) section format.
#[derive(Debug)]
pub struct BundleImageSection {
    flags: u32,
    pub fsize: usize,
    pub msize: usize,
    pub crc32: usize,
    pub align: usize,
    pub entry: Option<usize>,
    pub vaddr: usize,
}
impl BundleImageSection {
    pub fn is_read(&self) -> bool { (self.flags & SECTION_READ) != 0 }
    pub fn is_write(&self) -> bool { (self.flags & SECTION_WRITE) != 0 }
    pub fn is_exec(&self) -> bool { (self.flags & SECTION_EXEC) != 0 }
    pub fn get_rights(&self) -> seL4_CapRights {
        seL4_CapRights::new(
            /*grantreply=*/ 0,
            /*grant=*/ self.is_exec() as usize,
            /*read=*/ self.is_read() as usize,
            /*write=*/ self.is_write() as usize,
        )
    }
    pub fn data_range(&self) -> Range<usize> { 0..self.fsize }
    pub fn zero_range(&self) -> Range<usize> { self.fsize..self.msize }
}

// BundleImage is a loadable image that backs a Bundle. There are images
// for a bundle's application and optionally one or more images for models
// that can be loaded into the vector core. The BundleImage format is
// optimized for loading a page at a time from unmapped frame objects
// and is typically transient (create, load contents, destroy).
//
// NB: this packages a section iterator together with i/o traits to
//   avoid multi-borrow issues.
pub struct BundleImage<'a> {
    // I/O traits state.
    frames: &'a ObjDescBundle,
    cur_frame: Option<seL4_CPtr>,
    last_frame: Option<seL4_CPtr>,
    cur_pos: u64, // Current position in i/o stream
    bounce: CSpaceSlot, // Top-level CNode slot for doing map
    mapped_page: *mut u8, // Currently mapped page frame
    mapped_bytes: usize, // Bytes in mapped frame, 0 when no frame mapped
    bytes_read: usize, // Bytes read from mapped frame

    // Section iterator state.
    next_section: usize, // Byte offset to next section
}
impl<'a> BundleImage<'a> {
    pub fn new(frames: &'a ObjDescBundle) -> Self {
        BundleImage {
            frames,
            cur_frame: None,
            last_frame: None,
            cur_pos: 0,
            bounce: CSpaceSlot::new(),
            mapped_page: unsafe { ptr::addr_of_mut!(BUNDLE_IMAGE[0]) },
            mapped_bytes: 0,
            bytes_read: 0,

            next_section: 0,
        }
    }

    pub fn finish(&mut self) {
        self.unmap_current_frame().expect("finish");
    }

    // Read the current section header and setup to advance to the next
    // section on the next call. This is used in lieu of an iterator to
    // avoid BundleImage borrow issues.
    // XXX change to Result so errors are visible
    pub fn next_section(&mut self) -> Option<BundleImageSection> {
        self.seek(io::SeekFrom::Start(self.next_section as u64)).ok()?;
        let raw_data = &mut [0u8; size_of::<SectionHeader>()];
        self.read_exact(raw_data).ok()?;
        let magic = u64::from_be_bytes(raw_data[0..8].try_into().unwrap());
        if magic != SECTION_MAGIC {
            // NB: happens when the image does not end on a page boundary,
            //   check magic as a hack to detect this
            if magic != 0 {
                error!("Invalid magic number at offset {} expected 0x{:x} got 0x{:x}",
                      self.next_section, SECTION_MAGIC, magic);
            }
            return None;
        }
        let mut hdr = BundleImageSection {
            vaddr: u64::from_be_bytes(raw_data[8..16].try_into().unwrap()) as usize,
            entry: None,
            flags: u32::from_be_bytes(raw_data[24..28].try_into().unwrap()),
            fsize: u32::from_be_bytes(raw_data[28..32].try_into().unwrap()) as usize,
            msize: u32::from_be_bytes(raw_data[32..36].try_into().unwrap()) as usize,
            align: u32::from_be_bytes(raw_data[36..40].try_into().unwrap()) as usize,
            // pad [40..44]
            crc32: u32::from_be_bytes(raw_data[44..48].try_into().unwrap()) as usize,
        };
        if (hdr.flags & SECTION_ENTRYPOINT) != 0 {
            hdr.entry = Some(u64::from_be_bytes(raw_data[16..24].try_into().unwrap()) as usize);
        }
        self.next_section = (self.cur_pos as usize) + hdr.fsize;
        Some(hdr)
    }

    // Unmap the current page and reset state.
    fn unmap_current_frame(&mut self) -> Result<(), BundleImageError> {
        if let Some(cptr) = self.cur_frame {
            // XXX if unmap fails bounce is cleaned up on drop but we probably want it moved instead
            unsafe { seL4_Page_Unmap(self.bounce.slot) }
                .map_err(|_| BundleImageError::PageUnmapFailed)?;
            self.bounce.move_from(self.frames.cnode, cptr, self.frames.depth)
                .map_err(|_| BundleImageError::CapMoveFailed)?;
        }
        self.last_frame = self.cur_frame;
        self.cur_frame = None;
        self.mapped_bytes = 0;
        self.bytes_read = 0;
        Ok(())
    }

    // Map the frame containing self.|cur_pos| into our VSpace.
    fn map_next_frame(&mut self) -> Result<(), BundleImageError> {
        assert_eq!(self.cur_frame, None);
        let mut od_off: u64 = 0; // Running byte offset to start of current ObjDesc
        // n^2 in ObjDesc, track last frame
        for od in &self.frames.objs {
            // TODO(sleffler): maybe move page index logic to ObjDesc
            let size_bytes = od.size_bytes().unwrap() as u64;
            if od_off <= self.cur_pos && self.cur_pos < od_off + size_bytes {
                // The frame is in this ObjDesc, calculate the page index.
                let index = ((self.cur_pos - od_off) / (PAGE_SIZE as u64)) as usize;
                assert!(index < od.retype_count());

                // Bounce through the top-level CNode.
                sel4_sys::debug_assert_slot_empty!(self.bounce.slot,
                    "{}: expected slot {:?} empty but has cap type {:?}",
                    "map_next_frame", self.bounce.slot,
                    sel4_sys::cap_identify(self.bounce.slot));
                self.bounce.move_to(
                    self.frames.cnode,
                    od.cptr + index,
                    self.frames.depth
                ).map_err(|_| BundleImageError::CapMoveFailed)?;

                // Map the page into our VSpace
                // TODO(sleffler): if this fails maybe undo move_to
                sel4_sys::debug_assert_slot_frame!(self.bounce.slot,
                    "{}: expected frame in slot {:?} but has cap type {:?}",
                    "map_next_frame", self.bounce.slot,
                    sel4_sys::cap_identify(self.bounce.slot));
                unsafe {
                    seL4_Page_Map(
                        /*sel4_page=*/ self.bounce.slot,
                        /*seL4_pd=*/ SELF_VSPACE_ROOT,
                        /*vaddr=*/ self.mapped_page as usize,
                        seL4_CapRights::new(
                            /*grant_reply=*/0, /*grant=*/0, /*read=1*/1, /*write=*/0,
                        ),
                        seL4_Default_VMAttributes,
                    )
                }.map_err(|_| BundleImageError::PageMapFailed)?;
                self.cur_frame = Some(od.cptr + index);
                self.mapped_bytes = PAGE_SIZE;
                self.bytes_read = ((self.cur_pos - od_off) % (PAGE_SIZE as u64)) as usize;
                return Ok(())
            }
            od_off += size_bytes;
        }
        error!("No page at offset {}", self.cur_pos);
        Err(BundleImageError::PageNotFound)
    }
}
impl<'a> Drop for BundleImage<'a> {
    fn drop(&mut self) {
        self.finish();
    }
}
impl<'a> io::Seek for BundleImage<'a> {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        let new_pos = match pos {
            io::SeekFrom::Current(p) => {
                let ipos = (self.cur_pos as i64) + p;
                if ipos < 0 { return Err(io::Error) }
                ipos as u64
            }
            io::SeekFrom::End(p) => {
                // NB: potentially expensive to calculate
                let ipos = (self.frames.size_bytes() as i64) + p;
                if ipos < 0 { return Err(io::Error) }
                ipos as u64
            }
            io::SeekFrom::Start(p) => p,
        };
        if new_pos != self.cur_pos {
        trace!("SEEK: cur {} new {}", self.cur_pos, new_pos);
            // TODO(sleffler): handle seek within same page
            self.unmap_current_frame().map_err(|_| io::Error)?;
            self.cur_pos = new_pos;
        }
        Ok(self.cur_pos)
    }
}
impl<'a> io::Read for BundleImage<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut cursor = &mut *buf;
        while !cursor.is_empty() {
            let available_bytes = self.mapped_bytes - self.bytes_read;
            if available_bytes > 0 {
                // Fill from the current frame (as space permits).
                let region = unsafe {
                    core::slice::from_raw_parts_mut(self.mapped_page, self.mapped_bytes)
                };
                let bytes_to_read = cmp::min(available_bytes, cursor.len());
                unsafe {
                    ptr::copy_nonoverlapping(
                        region[self.bytes_read..].as_ptr(),
                        cursor.as_mut_ptr(),
                        bytes_to_read
                    )
                };
                self.bytes_read += bytes_to_read;
                self.cur_pos += bytes_to_read as u64;
                cursor = &mut cursor[bytes_to_read..];

                assert!(self.bytes_read <= self.mapped_bytes);
                if self.bytes_read == self.mapped_bytes {
                    // Current frame is empty; unmap and prepare for next.
                    self.unmap_current_frame().map_err(|_| io::Error)?;
                }
            }
            if cursor.is_empty() { break }

            // Map the next frame for read.
            self.map_next_frame().map_err(|_| io::Error)?;
        }
        // TODO(sleffler): self.digest.write(buf); // Update crc32 calculation
        Ok(buf.len())
    }
}
