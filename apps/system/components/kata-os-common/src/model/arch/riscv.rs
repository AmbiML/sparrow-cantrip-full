// RISC-V common target support.

#![allow(non_camel_case_types)]
#![allow(dead_code)]

use static_assertions::assert_cfg;
assert_cfg!(any(target_arch = "riscv32", target_arch = "riscv64"));

use crate::CantripOsModel;
use capdl::CDL_ObjectType::*;
use capdl::*;

use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_CapIRQControl;
use sel4_sys::seL4_CapInitThreadCNode;
use sel4_sys::seL4_CapRights;
use sel4_sys::seL4_Error;
use sel4_sys::seL4_IRQControl_Get;
use sel4_sys::seL4_ObjectType;
use sel4_sys::seL4_Page;
use sel4_sys::seL4_PageBits;
use sel4_sys::seL4_PageDirIndexBits;
use sel4_sys::seL4_PageTableIndexBits;
use sel4_sys::seL4_Result;
use sel4_sys::seL4_UserContext;
use sel4_sys::seL4_VMAttributes;
use sel4_sys::seL4_Word;
use sel4_sys::seL4_WordBits;

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
) -> seL4_Result {
    Ok(())
}

pub fn create_irq_cap(irq: CDL_IRQ, _obj: &CDL_Object, free_slot: seL4_CPtr) -> seL4_Result {
    unsafe {
        seL4_IRQControl_Get(
            seL4_CapIRQControl,
            irq,
            /*root=*/ seL4_CapInitThreadCNode as usize,
            /*index=*/ free_slot,
            /*depth=*/ seL4_WordBits as u8,
        )
    }
}

pub fn get_user_context(cdl_tcb: &CDL_Object, sp: seL4_Word) -> *const seL4_UserContext {
    #[rustfmt::skip]
    static mut regs: seL4_UserContext = seL4_UserContext {
        pc: 0, ra: 0, sp: 0, gp: 0,
        s0: 0, s1: 0, s2: 0, s3: 0, s4: 0, s5: 0,
        s6: 0, s7: 0, s8: 0, s9: 0, s10: 0, s11: 0,
        a0: 0, a1: 0, a2: 0, a3: 0, a4: 0, a5: 0, a6: 0, a7: 0,
        t0: 0, t1: 0, t2: 0, t3: 0, t4: 0, t5: 0, t6: 0, tp: 0,
    };

    assert_eq!(cdl_tcb.r#type(), CDL_TCB);

    unsafe {
        regs.pc = cdl_tcb.tcb_pc();
        regs.sp = sp; // NB: may be adjusted from cdl_tcb.tcb_sp()

        let argv = core::slice::from_raw_parts(cdl_tcb.tcb_init(), cdl_tcb.tcb_init_sz());
        regs.a0 = if argv.len() > 0 { argv[0] } else { 0 };
        regs.a1 = if argv.len() > 1 { argv[1] } else { 0 };
        regs.a2 = if argv.len() > 2 { argv[2] } else { 0 };
        regs.a3 = if argv.len() > 3 { argv[3] } else { 0 };

        //        trace!("Start {} with pc {:#x} sp {:#x} argv {:?}", cdl_tcb.name(),
        //               regs.pc, regs.sp, argv);

        &regs as *const seL4_UserContext
    }
}

impl<'a> CantripOsModel<'a> {
    pub fn create_arch_object(
        &mut self,
        _obj: &CDL_Object,
        _id: CDL_ObjID,
        _free_slot: usize,
    ) -> Option<seL4_Error> {
        // No architecture-specific overrides.
        None
    }
}
