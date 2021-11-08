use core::assert;
use core::slice;
use xmas_elf::program::{SegmentData, Type};
use xmas_elf::ElfFile;

// TODO(jesionowski): Move these constants to an auto-generated file.
const ELF_SIZE: usize = 0x300000;
const ITCM_SIZE: usize = 0x40000;
const ITCM_PADDR: usize = 0x30000000;
const DTCM_SIZE: usize = 0x1000000;
const DTCM_PADDR: usize = 0x34000000;

extern "C" {
    static elf_file: *const u8;
}
extern "C" {
    static itcm: *mut u8;
}
extern "C" {
    static dtcm: *mut u8;
}

pub fn loadelf() -> Result<(), &'static str> {
    let elf_slice = unsafe { slice::from_raw_parts(elf_file, ELF_SIZE) };
    let itcm_slice = unsafe { slice::from_raw_parts_mut(itcm, ITCM_SIZE) };
    let dtcm_slice = unsafe { slice::from_raw_parts_mut(dtcm, DTCM_SIZE) };

    let elf = ElfFile::new(&elf_slice)?;

    for seg in elf.program_iter() {
        if seg.get_type()? == Type::Load {
            let fsize = seg.file_size() as usize;
            let msize = seg.mem_size() as usize;

            // TODO(jesionowski): I'm assuming that there will be two segments, each beginning at
            // the respective PADDRs. Is that assumption safe or does there need to be more
            // complex handling?
            if seg.virtual_addr() as usize == ITCM_PADDR {
                assert!(
                    fsize <= ITCM_SIZE,
                    "Elf's ITCM section is larger than than ITCM_SIZE"
                );

                // Due to being Load types we are guarunteed SegmentData::Undefined as the
                // data type.
                if let SegmentData::Undefined(bytes) = seg.get_data(&elf)? {
                    itcm_slice[..fsize].copy_from_slice(&bytes);
                }
            } else if seg.virtual_addr() as usize == DTCM_PADDR {
                // TODO(jesionowski): Change to msize. Will currently fail as a portion
                // of the memory is mapped to the vctop driver for getting return information.
                assert!(
                    fsize <= DTCM_SIZE,
                    "Elf's DTCM section is larger than than DTCM_SIZE"
                );

                if let SegmentData::Undefined(bytes) = seg.get_data(&elf)? {
                    dtcm_slice[..fsize].copy_from_slice(&bytes);
                }
                // Clear NOBITS sections.
                dtcm_slice[fsize..msize].fill(0x00);
            } else {
                assert!(false, "Elf contains LOAD section outside TCM");
            }
        }
    }

    Ok(())
}
