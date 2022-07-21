// ARM common target support.

#![allow(non_camel_case_types)]

use static_assertions::assert_cfg;
assert_cfg!(any(target_arch = "arm", target_arch = "aarch64"));

use capdl::CDL_ObjectType::*;
use capdl::*;

use sel4_sys::seL4_ARM_Page_CleanInvalidate_Data;
use sel4_sys::seL4_ARM_Page_GetAddress;
use sel4_sys::seL4_ARM_Page_Unify_Instruction;
use sel4_sys::seL4_CapRights;
use sel4_sys::seL4_ObjectType;
use sel4_sys::seL4_Page;
use sel4_sys::seL4_PageBits;
use sel4_sys::seL4_PageDirIndexBits;
use sel4_sys::seL4_PageTableIndexBits;
use sel4_sys::seL4_Result;
use sel4_sys::seL4_VMAttributes;
use sel4_sys::seL4_Word;

pub const PAGE_SIZE: usize = 4096;
pub const STACK_ALIGNMENT_BYTES: usize = 16;
pub const REG_ARGS: seL4_Word = 4; // Number of regs for passing thread args

include!(concat!(env!("OUT_DIR"), "/platform_gen.rs"));

// When seL4 creates a new frame object it zeroes the associated memory
// through a cached kernel mapping. If we are configuring a cached
// mapping for the target, standard coherence protocols ensure
// everything works as expected. However, if we are configuring an
// uncached mapping for the target, the dirty zero data cached from the
// kernel's mapping is likely flushed to memory at some time in the
// future causing an unpleasant surprise for the target whose own
// uncached writes are mysteriously overwritten. To prevent this, we
// unify the mapping here, flushing the cached data from the kernel's
// mapping.
pub unsafe fn seL4_Page_Map_Flush(
    page_type: seL4_ObjectType,
    sel4_page: seL4_Page,
    rights: seL4_CapRights,
    vm_attribs: seL4_VMAttributes,
) -> seL4_Result {
    let addr = seL4_ARM_Page_GetAddress(sel4_page);
    if MEMORY_REGIONS[0].start <= addr.paddr && addr.paddr <= MEMORY_REGIONS[0].end {
        let frame_size_bits = page_type.size_bits().unwrap();
        assert!(
            frame_size_bits <= (usize::BITS - 1) as usize,
            "{:?}: illegal object size",
            page_type
        );

        // NB: could minimize invalidations by checking page's paddr, but
        //   given the cost already incurred to lookup the page's paddr
        //   just always do it
        if ((vm_attribs as u32) & sel4_sys::seL4_ARM_VMAttributes::PageCacheable as u32) == 0 {
            seL4_ARM_Page_CleanInvalidate_Data(sel4_page, 0, BIT(frame_size_bits))?;
        }
        if rights.get_capAllowGrant() != 0 {
            seL4_ARM_Page_Unify_Instruction(sel4_page, 0, BIT(frame_size_bits))?;
        }
    }
    Ok(())
}

fn BIT(bit_num: usize) -> usize { 1 << bit_num }
fn MASK(pow2_bits: usize) -> usize { BIT(pow2_bits) - 1 }

#[allow(dead_code)]
pub fn PD_SLOT(vaddr: usize) -> usize {
    (vaddr >> (seL4_PageTableIndexBits + seL4_PageBits)) & MASK(seL4_PageDirIndexBits)
}
// NB: used for tcb_args::maybe_spill_tcb_args
pub fn PT_SLOT(vaddr: usize) -> usize { (vaddr >> seL4_PageBits) & MASK(seL4_PageTableIndexBits) }

// Identifies IRQ objects that potentially need special processing.
pub fn is_irq(type_: CDL_ObjectType) -> bool { type_ == CDL_ARMInterrupt || type_ == CDL_Interrupt }

// Identifies objects that need to be instantiated.
pub fn requires_creation(type_: CDL_ObjectType) -> bool { !is_irq(type_) }
