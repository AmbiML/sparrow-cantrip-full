// ARM 32-bit target support.

#![allow(non_camel_case_types)]

use static_assertions::assert_cfg;
assert_cfg!(all(target_arch = "arm", target_pointer_width = "32"));

use crate::CantripOsModel;
use capdl::kobject_t::*;
use capdl::CDL_CapType::*;
use capdl::CDL_ObjectType::*;
use capdl::*;
use log::{error, trace};

use sel4_sys::seL4_ARM_Page_CleanInvalidate_Data;
use sel4_sys::seL4_ARM_Page_Map;
use sel4_sys::seL4_ARM_Page_Unify_Instruction;

use sel4_sys::seL4_CapInitThreadCNode;
use sel4_sys::seL4_CapIRQControl;
use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_Error;
use sel4_sys::seL4_IRQControl_Get;
use sel4_sys::seL4_ObjectType::*;
use sel4_sys::seL4_PageBits;
use sel4_sys::seL4_PageDirIndexBits;
use sel4_sys::seL4_PageTableIndexBits;
use sel4_sys::seL4_PUDIndexBits;
use sel4_sys::seL4_Result;
use sel4_sys::seL4_UserContext;
use sel4_sys::seL4_Word;

pub const PAGE_SIZE: usize = 4096;
pub const STACK_ALIGNMENT_BYTES: usize = 16;
pub const REG_ARGS: seL4_Word = 4; // Number of regs for passing thread args

pub const CDL_PT_LEVEL_3_IndexBits: usize = seL4_PageTableIndexBits;

// NB: this overrides what sel4_sys provides
pub fn seL4_Page_Map(
    sel4_page: seL4_ARM_Page,
    sel4_pd: seL4_ARM_PageTable,
    vaddr: seL4_Word,
    rights: seL4_CapRights,
    vm_attribs: seL4_ARM_VMAttributes,
) -> seL4_Result {
    if !rights.get_capAllowGrant() {
        vm_attribs |= seL4_ARM_ExecuteNever;
    }
    seL4_ARM_Page_Map(sel4_page, sel4_pd, vaddr, rights, vm_attribs)?;

    // XXX lookup frame_size_bits & page
    /* When seL4 creates a new frame object it zeroes the associated memory
     * through a cached kernel mapping. If we are configuring a cached
     * mapping for the target, standard coherence protocols ensure
     * everything works as expected. However, if we are configuring an
     * uncached mapping for the target, the dirty zero data cached from the
     * kernel's mapping is likely flushed to memory at some time in the
     * future causing an unpleasant surprise for the target whose own
     * uncached writes are mysteriously overwritten. To prevent this, we
     * unify the mapping here, flushing the cached data from the kernel's
     * mapping.
     */
    const CHAR_BIT: usize = 8; // XXX
    assert!(frame_size_bits <= size_of::<usize>() * CHAR_BIT - 1, "illegal object size");

    let addr = seL4_ARM_Page_GetAddress(sel4_page);
    if addr.paddr >= memory_region[0].start && addr.paddr <= memory_region[0].end {
        if !(vm_attribs & seL4_ARM_PageCacheable) && spec.objects[page].paddr() == 0 {
            seL4_ARM_Page_CleanInvalidate_Data(sel4_page, 0, BIT(frame_size_bits))?;
        }
        if rights.get_capAllowGrant() {
            seL4_ARM_Page_Unify_Instruction(sel4_page, 0, BIT(frame_size_bits))?;
        }
    }

    Ok(())
}

fn MASK(pow2_bits: usize) -> usize { (1 << pow2_bits) - 1 }

pub fn PD_SLOT(vaddr: usize) -> usize {
    (vaddr >> (seL4_PageTableIndexBits + seL4_PageBits)) & MASK(seL4_PageDirIndexBits)
}
// NB: used for tcb_args::maybe_spill_tcb_args
pub fn PT_SLOT(vaddr: usize) -> usize { (vaddr >> seL4_PageBits) & MASK(seL4_PageTableIndexBits) }

// Identifies IRQ objects that potentially need special processing.
pub fn is_irq(type_: CDL_ObjectType) -> bool { type_ == CDL_ARMInterrupt || type_ == CDL_Interrupt }

// Identifies objects that need to be instantiated.
pub fn requires_creation(type_: CDL_ObjectType) -> bool { !is_irq(type_) }

pub fn create_irq_cap(irq: CDL_IRQ, obj: &CDL_Object, free_slot: seL4_CPtr) -> seL4_Error {
    let root = seL4_CapInitThreadCNode;
    let index = free_slot;
    let depth = seL4_WordBits as u8;
    match obj.r#type() {
        // XXX seL4_IRQControl_GetTriggerCore for NUM_NODES > 1
        #[cfg(feature = "CONFIG_SMP_SUPPORT")]
        CDL_ARMInterrupt => {
            seL4_IRQControl_GetTriggerCore(
                seL4_CapIRQControl,
                irq,
                obj.armirq_trigger(),
                root,
                index,
                depth,
                obj.armirq_target(),
            )
        }
        #[cfg(not(feature = "CONFIG_SMP_SUPPORT"))]
        CDL_ARMInterrupt => {
            seL4_IRQControl_GetTrigger(
                seL4_CapIRQControl,
                irq,
                obj.armirq_trigger(),
                root,
                index,
                depth,
            )
        }
        _ => seL4_IRQControl_Get(seL4_CapIRQControl, irq, root, index, depth),
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

pub fn kobject_get_size(t: kobject_t, object_size: seL4_Word) -> seL4_Word {
    if t == KOBJECT_FRAME && object_size == seL4_HugePageBits {
        return object_size;
    }
    if t == KOBJECT_PAGE_UPPER_DIRECTORY {
        return seL4_PUDBits;
    }
    #[cfg(feature = "CONFIG_KERNEL_MCS")]
    if t == KOBJECT_SCHED_CONTEXT {
        return core::cmp::max(object_size, sel4_sys::seL4_MinSchedContextBits);
    }
    error!("Unexpected object: type {:?} size {}", t, object_size);
    0
}
pub fn kobject_get_type(t: kobject_t, object_size: seL4_Word) -> seL4_ObjectType {
    match t {
        KOBJECT_PAGE_GLOBAL_DIRECTORY => {
            return seL4_ARM_PageGlobalDirectoryObject;
        }
        KOBJECT_PAGE_UPPER_DIRECTORY => {
            return seL4_ARM_PageUpperDirectoryObject;
        }
        KOBJECT_FRAME => {
            if object_size == seL4_HugePageBits {
                return seL4_ARM_HugePageObject;
            }
            error!("Unexpected frame size {}", object_size);
        }
        _ => {}
    }
    error!("Unexpected object: type {:?} size {}", t, object_size);
    seL4_InvalidObjectType
}

impl<'a> CantripOsModel<'a> {
    pub fn create_arch_object(
        &mut self,
        obj: &CDL_Object,
        _id: CDL_ObjID,
        free_slot: usize,
    ) -> Option<seL4_Error> {
        match obj.r#type() {
            #[cfg(feature = "CONFIG_ARM_SMMU")]
            CDL_SID => {
                /* There can be multiple sids per context bank, currently only 1
                 * sid per cb is implemented for the vms. When this gets extended
                 * we need to decide to add sid number -> cb number map into the
                 * haskell / python tool or generate the capdl spec so that the
                 * order remains correct here e.g a list a stream ids followed by
                 * the cb they are mapped to, the cb condition here (1***) will the
                 * reset the stream id number back to 0 for the next context bank.
                 */
                assert!(sid_number <= MAX_STREAM_IDS);
                // XXX handle error
                seL4_ARM_SIDControl_GetSID(
                    seL4_CapSMMUSIDControl,
                    self.sid_number,
                    seL4_CapInitThreadCNode,
                    free_slot,
                    seL4_WordBits as u8,
                );
                self.sid_number += 1;
                Some(seL4_NoError)
            }
            #[cfg(feature = "CONFIG_ARM_SMMU")]
            CDL_CB => {
                self.sid_number = 0; //(1***)
                Some(seL4_ARM_CBControl_GetCB(
                    seL4_CapSMMUCBControl,
                    obj.cb_bank(),
                    seL4_CapInitThreadCNode,
                    free_slot,
                    seL4_WordBits as u8,
                ))
            }
            _ => None,
        }
    }

    pub fn init_vspace(&mut self, obj_id: CDL_ObjID) -> seL4_Result {
        assert_eq!(self.get_object(obj_id).r#type(), CDL_PD);
        // XXX C code does all PD's before PT's, not sure if this works
        self.map_page_directory(obj_id)?;
        self.map_page_directory_page_tables(obj_id)?;
        Ok(())
    }

    fn map_page_directory(&self, pd_id: CDL_ObjID) {
        fn map_page_directory_slot(pd_id: CDL_ObjID, pd_slot: &CDL_CapSlot) {
            let page_cap = &pd_slot.cap;
            let page_vaddr = pd_slot.slot << (seL4_PageTableIndexBits + seL4_PageBits);
            self.map_page(page_cap, pd_id, page_cap.cap_rights(), page_vaddr);
        }

        for slot in self.spec.get_object(pd_id).slots_slice() {
            map_page_directory_slot(pd_id, &slot);
        }
    }

    fn map_page_directory_page_tables(&self, pd: CDL_ObjID) {
        fn map_page_table_slots(pd: CDL_ObjID, pd_slot: &CDL_CapSlot) {
            fn map_page_table_slot(
                pd: CDL_ObjID,
                pt: CDL_ObjID,
                pt_vaddr: seL4_Word,
                pt_slot: &CDL_CapSlot,
            ) {
                let page_cap = &pt_slot.cap;
                let page_vaddr = pt_vaddr + (pt_slot.slot << seL4_PageBits);
                self.map_page(page_cap, pd, page_cap.cap_rights(), page_vaddr);
            }

            let page_cap = &pd_slot.cap;
            if (page_cap.r#type() == CDL_PTCap) {
                let page = page_cap.obj_id;
                let page_vaddr = pd_slot.slot << (seL4_PageTableIndexBits + seL4_PageBits);
                for slot in self.spec.get_object(page).slots_slice() {
                    self.map_page_table_slot(pd, page, page_vaddr, &slot);
                }
            }
        }

        for slot in self.spec.get_object(pd).slots_slice() {
            self.map_page_table_slots(pd, &slot);
        }
    }

    pub fn get_cdl_frame_pt(&mut self, pd: CDL_ObjID, vaddr: usize) -> Option<&'a mut CDL_Cap> {
        self.get_spec_object(pd).get_cap_at(PD_SLOT(vaddr))
    }
}
