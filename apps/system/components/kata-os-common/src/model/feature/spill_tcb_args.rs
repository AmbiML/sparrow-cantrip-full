// Spill-to-stack Calling Convention.
// The first REG_ARGS arguments are passed to threads using registers;
// any more arguments are written to the stack.

use crate::arch::PAGE_SIZE;
use crate::arch::PT_SLOT;
use crate::arch::REG_ARGS;
use crate::arch::STACK_ALIGNMENT_BYTES;
use crate::copy_region;
use crate::CantripOsModel;
use capdl::CDL_CapType::*;
use capdl::*;
use core::mem::size_of;
use core::ptr;

#[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
use sel4_sys::seL4_ARM_Page_Unify_Instruction;

use sel4_sys::seL4_CapInitThreadVSpace;
use sel4_sys::seL4_CapRights;
use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_Default_VMAttributes;
use sel4_sys::seL4_Error;
use sel4_sys::seL4_Page_Map;
use sel4_sys::seL4_Page_Unmap;
use sel4_sys::seL4_Word;

use static_assertions::assert_cfg;
assert_cfg!(not(feature = "CONFIG_CAPDL_LOADER_CC_REGISTERS"));

impl<'a> CantripOsModel<'a> {
    // Check TCB's argv and if needed write arguments to the stack and
    // fixup the stack pointer to match.
    pub fn maybe_spill_tcb_args(
        &self,
        cdl_tcb: &CDL_Object,
        osp: seL4_Word,
    ) -> Result<seL4_Word, seL4_Error> {
        let argc = cdl_tcb.tcb_init_sz();
        let reg_args = REG_ARGS;
        if argc <= reg_args {
            return Ok(osp); // Arguments fit in registers; nothing to do.
        }

        // More arguments than will fit in registers; map the TCB's stack
        // into our address space to write the spillover.

        assert_eq!(
            STACK_ALIGNMENT_BYTES % size_of::<seL4_Word>(),
            0,
            "Stack alignment wrong for argument size"
        );
        let mut sp = osp;

        // Find the TCB's PD.
        let pd = cdl_tcb.get_cap_at(CDL_TCB_VTable_Slot).unwrap().obj_id;

        // NB: the stack pointer will initially be aligned to
        // STACK_ALIGNMENT_BYTES. Any padding required to enforce this
        // alignment will come before any stack arguments.
        let num_stack_args = argc - reg_args; // positive because argc > reg_args
        let args_per_alignment = STACK_ALIGNMENT_BYTES / size_of::<seL4_Word>();
        let num_unaligned_args = num_stack_args % args_per_alignment;
        if num_unaligned_args != 0 {
            let num_padding_args = args_per_alignment - num_unaligned_args;
            let num_padding_bytes = num_padding_args * size_of::<seL4_Word>();
            sp -= num_padding_bytes;
        }

        // Find and map the frame representing the TCB's stack. Note that
        // we do `sp - sizeof(uintptr_t)` because the stack pointer may
        // be on a page boundary.
        let frame = self.get_frame_cap(pd, sp - size_of::<seL4_Word>());

        /* FIXME: The above could actually fail messily if the user has given a
         * spec with stack pointers that point outside the ELF image.
         */
        // NB: platforms that have an NX attribute will add it in
        //     seL4_Page_Map when capAllowGrant is false (e.g. arm).
        let attribs = seL4_Default_VMAttributes;
        unsafe {
            seL4_Page_Map(
                frame,
                seL4_CapInitThreadVSpace,
                ptr::addr_of!(copy_region.data[0]) as usize,
                // seL4_ReadWrite
                seL4_CapRights::new(
                    /*grant_reply=*/ 0, /*grant=*/ 0, /*read=*/ 1, /*write=*/ 1,
                ),
                attribs,
            )
        }?;

        // Write spillover arguments to the TCB's stack.
        let argv = unsafe { core::slice::from_raw_parts(cdl_tcb.tcb_init(), argc) };
        for i in (reg_args..argc).rev() {
            sp -= size_of::<seL4_Word>();

            // We could support this case with more complicated logic, but
            // choose not to.
            assert!(
                (sp % PAGE_SIZE) != 0,
                "TCB {}'s initial arguments cause its stack to cross a page boundary",
                cdl_tcb.name()
            );
            // NB: copy_region.data is [seL4_Word] but sp is in bytes
            unsafe {
                ptr::write(
                    &mut copy_region.data[(sp % PAGE_SIZE) / size_of::<seL4_Word>()],
                    argv[i],
                )
            };
        }

        #[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
        unsafe { seL4_ARM_Page_Unify_Instruction(frame, 0, PAGE_SIZE) }?;

        unsafe { seL4_Page_Unmap(frame) }?;

        Ok(sp)
    }

    // Locate page Frame associated with |vaddr| in the page directory
    // object |pd|. This is used for findings the stack of a TCB when
    // doing argv spillover to the stack.
    fn get_frame_cap(&self, pd: CDL_ObjID, vaddr: usize) -> seL4_CPtr {
        self.get_orig_cap(self.get_cdl_frame_cap(pd, vaddr).unwrap().obj_id)
    }

    fn get_cdl_frame_cap(&self, pd: CDL_ObjID, vaddr: usize) -> Option<&'a CDL_Cap> {
        // arch::get_cdl_frame_pt
        let pt_cap = self.get_cdl_frame_pt(pd, vaddr)?;
        // Check if the PT cap is actually a large frame cap.
        if pt_cap.r#type() == CDL_FrameCap {
            Some(pt_cap)
        } else {
            self.get_object(pt_cap.obj_id).get_cap_at(PT_SLOT(vaddr))
        }
    }
}
