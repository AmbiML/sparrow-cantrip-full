// RISC-V common target support.

#![allow(non_camel_case_types)]
#![allow(dead_code)]

use static_assertions::assert_cfg;
assert_cfg!(any(target_arch = "riscv32", target_arch = "riscv64"));

use sel4_sys::seL4_CapRights;
use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_PageBits;
use sel4_sys::seL4_PageTableIndexBits;
use sel4_sys::seL4_Result;
use sel4_sys::seL4_RISCV_Page_Map;
use sel4_sys::seL4_RISCV_VMAttributes;
use sel4_sys::seL4_Word;

pub const PAGE_SIZE: usize = 4096; // Base/small page size
pub const STACK_ALIGNMENT_BYTES: usize = 16;
pub const REG_ARGS: seL4_Word = 4; // Number of regs for passing thread args

// Architecture-independent aliases to enable arch-independent rootserver code
// TODO(sleffler): maybe move to sel4_sys?
pub use sel4_sys::seL4_RISCV_4K_Page as seL4_SmallPageObject;
pub use sel4_sys::seL4_RISCV_PageTableObject as seL4_PageTableObject;
pub use sel4_sys::seL4_PageTableIndexBits as seL4_PageDirIndexBits;

pub use sel4_sys::seL4_RISCV_ASIDControl_MakePool as seL4_ASIDControl_MakePool;
pub use sel4_sys::seL4_RISCV_ASIDPool_Assign as seL4_ASIDPool_Assign;
pub use sel4_sys::seL4_RISCV_PageTable_Map as seL4_PageTable_Map;
pub use sel4_sys::seL4_RISCV_Page_GetAddress as seL4_Page_GetAddress;
pub use sel4_sys::seL4_RISCV_Page_Unmap as seL4_Page_Unmap;
pub use sel4_sys::seL4_RISCV_VMAttributes as seL4_VMAttributes;
pub use sel4_sys::seL4_RISCV_VMAttributes::Default_VMAttributes as seL4_Default_VMAttributes;

pub unsafe fn seL4_Page_Map(
    sel4_page: seL4_CPtr,
    sel4_pd: seL4_CPtr,
    vaddr: seL4_Word,
    rights: seL4_CapRights,
    vm_attribs: seL4_VMAttributes,
) -> seL4_Result {
    if rights.get_capAllowGrant() != 0 {
        // NB: executable
        seL4_RISCV_Page_Map(sel4_page, sel4_pd, vaddr, rights, vm_attribs)
    } else {
        seL4_RISCV_Page_Map(sel4_page, sel4_pd, vaddr, rights,
                            seL4_RISCV_VMAttributes::ExecuteNever)
    }
}

fn MASK(pow2_bits: usize) -> usize { (1 << pow2_bits) - 1 }

// NB: used to setup copy_addr_pt
pub fn PD_SLOT(vaddr: usize) -> usize {
    (vaddr >> (seL4_PageTableIndexBits + seL4_PageBits)) & MASK(seL4_PageDirIndexBits)
}
// NB: used for tcb_args::maybe_spill_tcb_args
pub fn PT_SLOT(vaddr: usize) -> usize { (vaddr >> seL4_PageBits) & MASK(seL4_PageTableIndexBits) }
