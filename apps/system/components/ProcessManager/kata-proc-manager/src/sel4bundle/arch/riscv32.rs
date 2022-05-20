// RISC-V 32-bit target support.

#![allow(non_camel_case_types)]

use static_assertions::assert_cfg;
assert_cfg!(target_arch = "riscv32");

use cantrip_memory_interface::ObjDesc;
use super::sel4_sys;

mod riscv;
pub use riscv::*;

use sel4_sys::seL4_CapRights;
use sel4_sys::seL4_PageTable_Map;
use sel4_sys::seL4_Page_Map;
use sel4_sys::seL4_Result;
use sel4_sys::seL4_RISCV_4K_Page;
use sel4_sys::seL4_RISCV_PageTableObject;
use sel4_sys::seL4_UserContext;
use sel4_sys::seL4_VMAttributes;
use sel4_sys::seL4_Word;

pub fn get_user_context(pc: seL4_Word, sp: seL4_Word, argv: &[seL4_Word])
    -> *const seL4_UserContext
{
    #[rustfmt::skip]
    static mut regs: seL4_UserContext = seL4_UserContext {
        pc: 0, ra: 0, sp: 0, gp: 0,
        s0: 0, s1: 0, s2: 0, s3: 0, s4: 0, s5: 0,
        s6: 0, s7: 0, s8: 0, s9: 0, s10: 0, s11: 0,
        a0: 0, a1: 0, a2: 0, a3: 0, a4: 0, a5: 0, a6: 0, a7: 0,
        t0: 0, t1: 0, t2: 0, t3: 0, t4: 0, t5: 0, t6: 0, tp: 0,
    };

    #[allow(clippy::len_zero)]
    unsafe {
        regs.pc = pc;
        regs.sp = sp; // NB: may be adjusted from self.tcb_sp

        regs.a0 = if argv.len() > 0 { argv[0] } else { 0 };
        regs.a1 = if argv.len() > 1 { argv[1] } else { 0 };
        regs.a2 = if argv.len() > 2 { argv[2] } else { 0 };
        regs.a3 = if argv.len() > 3 { argv[3] } else { 0 };

        &regs as *const seL4_UserContext
    }
}

pub fn map_page_table(
    pd: &ObjDesc,
    pt: &ObjDesc,
    vaddr: seL4_Word,
    vm_attribs: seL4_VMAttributes,
) -> seL4_Result {
    assert_eq!(pd.type_, seL4_RISCV_PageTableObject);
    assert_eq!(pt.type_, seL4_RISCV_PageTableObject);
    unsafe {
        seL4_PageTable_Map(pt.cptr, pd.cptr, vaddr, vm_attribs)
    }
}

pub fn map_page(
    frame: &ObjDesc,
    pd: &ObjDesc,
    vaddr: seL4_Word,
    rights: seL4_CapRights,
    vm_attribs: seL4_VMAttributes,
) -> seL4_Result {
    assert_eq!(frame.type_, seL4_RISCV_4K_Page);
    // NB: cannot distinguish between PD & PT
    assert_eq!(pd.type_, seL4_RISCV_PageTableObject);
    unsafe {
        seL4_Page_Map(frame.cptr, pd.cptr, vaddr, rights, vm_attribs)
    }
}
