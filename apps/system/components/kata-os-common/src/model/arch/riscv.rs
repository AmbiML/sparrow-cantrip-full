// RISC-V common target support.

#![allow(non_camel_case_types)]
#![allow(dead_code)]

use static_assertions::assert_cfg;
assert_cfg!(any(target_arch = "riscv32", target_arch = "riscv64"));

use capdl::CDL_ObjectType::*;
use capdl::*;

use sel4_sys::seL4_CapRights;
use sel4_sys::seL4_ObjectType;
use sel4_sys::seL4_Page;
use sel4_sys::seL4_PageBits;
use sel4_sys::seL4_PageDirIndexBits;
use sel4_sys::seL4_PageTableIndexBits;
use sel4_sys::seL4_Result;
use sel4_sys::seL4_VMAttributes;
use sel4_sys::seL4_Word;

pub const PAGE_SIZE: usize = 4096; // Base page size
pub const STACK_ALIGNMENT_BYTES: usize = 16;
pub const REG_ARGS: seL4_Word = 4; // Number of regs for passing thread args

fn MASK(pow2_bits: usize) -> usize { (1 << pow2_bits) - 1 }

// NB: used to setup copy_addr_pt
pub fn PD_SLOT(vaddr: usize) -> usize {
    (vaddr >> (seL4_PageTableIndexBits + seL4_PageBits)) & MASK(seL4_PageDirIndexBits)
}
// NB: used by tcb_args::maybe_spill_tcb_args
pub fn PT_SLOT(vaddr: usize) -> usize { (vaddr >> seL4_PageBits) & MASK(seL4_PageTableIndexBits) }

// Identifies IRQ objects that potentially need special processing.
pub fn is_irq(type_: CDL_ObjectType) -> bool { type_ == CDL_Interrupt }

// Identifies objects that need to be instantiated. This is overridden
// by architectures that have device objects that are not backed by
// untyped memory (i.e. that need creation).
pub fn requires_creation(type_: CDL_ObjectType) -> bool { !is_irq(type_) }

pub unsafe fn seL4_Page_Map_Flush(
    _page_type: seL4_ObjectType,
    _sel4_page: seL4_Page,
    _rights: seL4_CapRights,
    _vm_attribs: seL4_VMAttributes,
) -> seL4_Result { Ok(()) }
