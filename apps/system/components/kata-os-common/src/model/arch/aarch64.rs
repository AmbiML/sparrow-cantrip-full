// ARM aarch64 target support.

#![allow(non_camel_case_types)]

use static_assertions::assert_cfg;
assert_cfg!(target_arch = "aarch64");

mod arm;
pub use arm::*;

use capdl::*;
use capdl::CDL_CapType::*;
use capdl::CDL_ObjectType::*;
use crate::CantripOsModel;

use sel4_sys::seL4_ARM_PageDirectory_Map;
use sel4_sys::seL4_ARM_PageUpperDirectory_Map;
use sel4_sys::seL4_CapInitThreadCNode;
use sel4_sys::seL4_CapIRQControl;
use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_Error;
use sel4_sys::seL4_HugePageBits;
use sel4_sys::seL4_IRQControl_GetTrigger;
use sel4_sys::seL4_LargePageBits;
use sel4_sys::seL4_ObjectType::*;
use sel4_sys::seL4_ObjectType;
use sel4_sys::seL4_PageBits;
use sel4_sys::seL4_PageDirIndexBits;
use sel4_sys::seL4_PageTableIndexBits;
use sel4_sys::seL4_PGDIndexBits;
use sel4_sys::seL4_PUDIndexBits;
use sel4_sys::seL4_Result;
use sel4_sys::seL4_UserContext;
use sel4_sys::seL4_VMAttributes;
use sel4_sys::seL4_Word;
use sel4_sys::seL4_WordBits;

pub const CDL_PT_LEVEL_1_IndexBits: usize = seL4_PUDIndexBits;
pub const CDL_PT_LEVEL_2_IndexBits: usize = seL4_PageDirIndexBits;
pub const CDL_PT_LEVEL_3_IndexBits: usize = seL4_PageTableIndexBits;

fn MASK(pow2_bits: usize) -> usize { (1 << pow2_bits) - 1 }

pub fn PGD_SLOT(vaddr: usize) -> usize {
    (vaddr >> (seL4_PUDIndexBits + seL4_PageDirIndexBits + seL4_PageTableIndexBits + seL4_PageBits)) & MASK(seL4_PGDIndexBits)
}
pub fn PUD_SLOT(vaddr: usize) -> usize {
    (vaddr >> (seL4_PageDirIndexBits + seL4_PageTableIndexBits + seL4_PageBits)) & MASK(seL4_PUDIndexBits)
}

pub fn get_frame_type(object_size: seL4_Word) -> seL4_ObjectType {
    match object_size {
        seL4_PageBits => seL4_ARM_SmallPageObject,
        seL4_LargePageBits => seL4_ARM_LargePageObject,
        seL4_HugePageBits => seL4_ARM_HugePageObject,
        _ => panic!("Unexpected frame size {}", object_size),
    }
}

pub fn create_irq_cap(irq: CDL_IRQ, obj: &CDL_Object, free_slot: seL4_CPtr) -> seL4_Result {
    assert_eq!(obj.r#type(), CDL_ARMInterrupt);
    // XXX seL4_IRQControl_GetTriggerCore for NUM_NODES > 1
    unsafe {
        seL4_IRQControl_GetTrigger(
            seL4_CapIRQControl,
            irq,
            obj.armirq_trigger(),
            /*root=*/ seL4_CapInitThreadCNode as usize,
            /*index=*/ free_slot,
            /*depth=*/ seL4_WordBits as u8,
        )
    }
}

pub fn get_user_context(cdl_tcb: &CDL_Object, sp: seL4_Word) -> *const seL4_UserContext {
    #[rustfmt::skip]
    static mut regs: seL4_UserContext = seL4_UserContext {
        pc: 0, sp: 0, spsr: 0,
        x0:  0, x1:  0, x2:  0, x3:  0, x4:  0, x5:  0, x6:  0, x7:  0,
        x8:  0, x9:  0, x10: 0, x11: 0, x12: 0, x13: 0, x14: 0, x15: 0,
        x16: 0, x17: 0, x18: 0, x19: 0, x20: 0, x21: 0, x22: 0, x23: 0,
        x24: 0, x25: 0, x26: 0, x27: 0, x28: 0, x29: 0, x30: 0,
        tpidr_el0: 0, tpidrro_el0: 0,
    };

    assert_eq!(cdl_tcb.r#type(), CDL_TCB);

    unsafe {
        regs.pc = cdl_tcb.tcb_pc();
        regs.sp = sp; // NB: may be adjusted from cdl_tcb.tcb_sp()

        let argv = core::slice::from_raw_parts(cdl_tcb.tcb_init(), cdl_tcb.tcb_init_sz());
        regs.x0 = if argv.len() > 0 { argv[0] } else { 0 };
        regs.x1 = if argv.len() > 1 { argv[1] } else { 0 };
        regs.x2 = if argv.len() > 2 { argv[2] } else { 0 };
        regs.x3 = if argv.len() > 3 { argv[3] } else { 0 };

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
        // CDL_SID objects with CONFIG_ARM_SMU?
        None
    }

    pub fn init_vspace(&mut self, obj_id: CDL_ObjID) -> seL4_Result {
        if cfg!(all(
            feature = "CONFIG_ARM_HYPERVISOR_SUPPORT",
            feature = "CONFIG_ARM_PA_SIZE_BITS_40"
        )) {
            self.init_level_1(obj_id, 0, obj_id)
        } else {
            self.init_level_0(obj_id, 0, obj_id)
        }
    }

    fn init_level_3(
        &mut self,
        level_3_obj: CDL_ObjID,
        level_0_obj: CDL_ObjID,
        level_3_base: usize,
    ) -> seL4_Result {
        for slot in self.get_object(level_3_obj).slots_slice() {
            let frame_cap = &slot.cap;
            self.map_page_frame(
                frame_cap,
                level_0_obj,
                frame_cap.cap_rights().into(),
                level_3_base + (slot.slot << seL4_PageBits),
            )?;
        }
        Ok(())
    }

    fn init_level_2(
        &mut self,
        level_0_obj: CDL_ObjID,
        level_2_base: usize,
        level_2_obj: CDL_ObjID,
    ) -> seL4_Result {
        for slot in self.get_object(level_2_obj).slots_slice() {
            let base = level_2_base + (slot.slot << (CDL_PT_LEVEL_3_IndexBits + seL4_PageBits));
            let level_3_cap = &slot.cap;
            if level_3_cap.r#type() == CDL_FrameCap {
                self.map_page_frame(
                    level_3_cap,
                    level_0_obj,
                    level_3_cap.cap_rights().into(),
                    base,
                )?;
            } else {
                let level_3_obj = level_3_cap.obj_id;
                self.map_page_table(level_3_cap, level_0_obj, base)?;
                self.init_level_3(level_3_obj, level_0_obj, base)?;
            }
        }
        Ok(())
    }

    fn init_level_1(
        &mut self,
        level_0_obj: CDL_ObjID,
        level_1_base: usize,
        level_1_obj: CDL_ObjID,
    ) -> seL4_Result {
        for slot in self.get_object(level_1_obj).slots_slice() {
            let base = level_1_base
                + (slot.slot
                    << (CDL_PT_LEVEL_2_IndexBits + CDL_PT_LEVEL_3_IndexBits + seL4_PageBits));
            let level_2_cap = &slot.cap;
            if level_2_cap.r#type() == CDL_FrameCap {
                self.map_page_frame(
                    level_2_cap,
                    level_0_obj,
                    level_2_cap.cap_rights().into(),
                    base,
                )?;
            } else {
                self.map_page_dir(level_2_cap, level_0_obj, base)?;
                self.init_level_2(level_0_obj, base, level_2_cap.obj_id)?;
            }
        }
        Ok(())
    }

    fn map_page_dir(&self, page_cap: &CDL_Cap, pd_id: CDL_ObjID, vaddr: seL4_Word) -> seL4_Result {
        assert_eq!(page_cap.r#type(), CDL_PDCap);

        let sel4_page = self.get_orig_cap(page_cap.obj_id);
        let sel4_pd = self.get_orig_cap(pd_id);

        //        trace!("  Map PD {} into {} @{:#x}, vm_attribs={:#x}",
        //                self.get_object(page_cap.obj_id).name(),
        //                self.get_object(pd_id).name(),
        //                vaddr, page_cap.vm_attribs());

        let vm_attribs: seL4_VMAttributes = page_cap.vm_attribs().into();
        unsafe { seL4_ARM_PageDirectory_Map(sel4_page, sel4_pd, vaddr, vm_attribs) }
    }

    fn init_level_0(
        &mut self,
        level_0_obj: CDL_ObjID,
        level_0_base: usize,
        _level_0_obj: CDL_ObjID,
    ) -> seL4_Result {
        for slot in self.get_object(level_0_obj).slots_slice() {
            let base = (level_0_base + slot.slot)
                    << (CDL_PT_LEVEL_1_IndexBits
                        + CDL_PT_LEVEL_2_IndexBits
                        + CDL_PT_LEVEL_3_IndexBits
                        + seL4_PageBits);
            let level_1_cap = &slot.cap;
            self.map_page_upper_dir(level_1_cap, level_0_obj, base)?;
            self.init_level_1(level_0_obj, base, level_1_cap.obj_id)?;
        }
        Ok(())
    }

    fn map_page_upper_dir(
        &self,
        page_cap: &CDL_Cap,
        pud_id: CDL_ObjID,
        vaddr: seL4_Word,
    ) -> seL4_Result {
        assert_eq!(page_cap.r#type(), CDL_PUDCap);

        let sel4_page = self.get_orig_cap(page_cap.obj_id);
        let sel4_pud = self.get_orig_cap(pud_id);

        //        trace!("  Map PUD {} into {} @{:#x}, vm_attribs={:#x}",
        //                self.get_object(page_cap.obj_id).name(),
        //                self.get_object(pud_id).name(),
        //                vaddr, page_cap.vm_attribs());

        let vm_attribs: seL4_VMAttributes = page_cap.vm_attribs().into();
        unsafe { seL4_ARM_PageUpperDirectory_Map(sel4_page, sel4_pud, vaddr, vm_attribs) }
    }

    pub fn get_cdl_frame_pt(&self, pd: CDL_ObjID, vaddr: usize) -> Option<&'a CDL_Cap> {
        let pd_cap = self.get_cdl_frame_pd(pd, vaddr)?;
        self.get_object(pd_cap.obj_id)
            .get_cap_at(PD_SLOT(vaddr))
    }

    fn get_cdl_frame_pd(&self, root: CDL_ObjID, vaddr: usize) -> Option<&'a CDL_Cap> {
        if cfg!(all(
            feature = "CONFIG_ARM_HYPERVISOR_SUPPORT",
            feature = "CONFIG_ARM_PA_SIZE_BITS_40"
        )) {
            self.get_object(root).get_cap_at(PUD_SLOT(vaddr))
        } else {
            let pud_cap = self.get_cdl_frame_pud(root, vaddr)?;
            self.get_object(pud_cap.obj_id)
                .get_cap_at(PUD_SLOT(vaddr))
        }
    }

    fn get_cdl_frame_pud(&self, root: CDL_ObjID, vaddr: usize) -> Option<&'a CDL_Cap> {
        self.get_object(root).get_cap_at(PGD_SLOT(vaddr))
    }
}
