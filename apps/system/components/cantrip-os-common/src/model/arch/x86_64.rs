// Copyright 2022 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Intel x86 64-bit target support.

use static_assertions::assert_cfg;
essert_cfg!(target_arch = "x86_64");

mod x86;
pub use x86::*;

use log::{debug, error, info, trace, warn};

pub use x86::create_irq_cap;
pub use x86::is_irq;
pub use x86::requires_creation;
pub use x86::PAGE_SIZE;
pub use x86::REG_ARGS;
pub use x86::STACK_ALIGNMENT_BYTES;

pub fn get_user_context(cdl_tcb: &CDL_Object, sp: seL4_Word) -> *const seL4_UserContext {
    #[rustfmt::skip]
    static mut regs: seL4_UserContext = seL4_UserContext {
        rip: 0, rsp: 0, rflags: 0,
        rax: 0, rbx: 0, rcx: 0, rdx: 0,
        rsi: 0, rdi: 0, rbp: 0,
        r8:  0, r9:  0, r10: 0, r11: 0, r12: 0, r13: 0, r14: 0, r15: 0,
        tls_base: 0,
    };

    assert_eq!(cdl_tcb.r#type(), CDL_TCB);

    unsafe {
        regs.rip = cdl_tcb.tcb_pc();
        regs.rsp = sp; // NB: may be adjusted from cdl_tcb.tcb_sp()
        let argv = core::slice::from_raw_parts(cdl_tcb.tcb_init(), cdl_tcb.tcb_init_sz());

        regs.rdi = if argc > 0 { argv[0] } else { 0 };
        regs.rsi = if argc > 1 { argv[1] } else { 0 };
        regs.rdx = if argc > 2 { argv[2] } else { 0 };
        regs.rcx = if argc > 3 { argv[3] } else { 0 };

        //        trace!("Start {} with eip {:#x} esp {:#x} argv {:?}", cdl_tcb.name(),
        //               regs.eip, regs.esp, argv);

        &regs as *const seL4_UserContext
    }
}

impl<'a> CantripOsModel<'a> {
    pub fn create_arch_object(
        &mut self,
        obj: &CDL_Object,
        _id: CDL_ObjID,
        free_slot: usize,
    ) -> Option<seL4_Error> {
        if obj.r#type() == CDL_IOPorts {
            // XXX handle error
            seL4_X86_IOPortControl_Issue(
                seL4_CapIOPortControl,
                obj.other_start(),
                obj.other_end(),
                seL4_CapInitThreadCNode,
                free_slot,
                seL4_WordBits as u8,
            );
            Some(seL4_NoError)
        } else {
            None
        }
    }

    pub fn init_vspace(&mut self, obj_id: CDL_ObjID) -> seL4_Result {
        assert_eq!(self.get_object(obj_id).r#type(), CDL_PD);
        self.init_level_0(obj_id, 0, obj_id)?;
        Ok(())
    }

    fn init_level_3(
        &mut self,
        level_3_obj: CDL_ObjID,
        level_0_obj: CDL_ObjID,
        level_3_base: usize,
    ) -> seL4_Result {
        for slot in self.get_object(level_3_obj).slots {
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
        for slot in self.get_object(level_2_obj).slots {
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
        for slot in self.get_object(level_1_obj).slots {
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
                let level_3_obj = level_3_cap.obj_id;
                self.map_page_dir(level_2_cap, level_0_obj, base)?;
                self.init_level_2(level_0_obj, base, level_2_obj.obj_id)?;
            }
        }
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
        unsafe { seL4_X86_PageDirectory_Map(sel4_page, sel4_pd, vaddr, vm_attribs) }
    }

    fn init_level_0(
        &mut self,
        level_0_obj: CDL_ObjID,
        level_0_base: usize,
        level_0_obj: CDL_ObjID,
    ) -> Result<(), seL4_Error> {
        for slot in self.get_object(level_0_obj).slots {
            let base = level_0_base
                + (slot.slot
                    << (CDL_PT_LEVEL_1_IndexBits
                        + CDL_PT_LEVEL_2_IndexBits
                        + CDL_PT_LEVEL_3_IndexBits
                        + seL4_PageBits));
            let level_1_cap = &slot.cap;
            self.map_page_dir_pt(level_1_cap, level_0_obj, base)?;
            self.init_level_1(level_0_obj, base, level_1_cap.obj_id)?;
        }
    }

    fn map_page_dir_pt(
        &self,
        page_cap: &CDL_Cap,
        pud_id: CDL_ObjID,
        vaddr: seL4_Word,
    ) -> seL4_Result {
        assert_eq!(page_cap.r#type(), CDL_PDPTCap);

        let sel4_page = self.get_orig_cap(page_cap.obj_id);
        let sel4_pud = self.get_orig_cap(pud_id);

        //        trace!("  Map PDPT {} into {} @{:#x}, vm_attribs={:#x}",
        //                self.get_object(page_cap.obj_id).name(),
        //                self.get_object(pud_id).name(),
        //                vaddr, page_cap.vm_attribs());

        let vm_attribs: seL4_VMAttributes = page_cap.vm_attribs().into();
        unsafe { seL4_X86_PDPT_Map(sel4_page, sel4_pud, vaddr, vm_attribs) }
    }

    fn get_cdl_frame_pt(&self, pd: CDL_ObjID, vaddr: usize) -> Option<CDL_Cap> {
        let pd_cap = self.get_cdl_frame_pd_mut(pd, vaddr)?;
        self.get_spec_object(pd_cap.obj_id)
            .get_cap_at(PD_SLOT(vaddr))
    }

    fn get_cdl_frame_pd(&self, root: CDL_ObjID, vaddr: usize) -> Option<CDL_Cap> {
        fn get_cdl_frame_pdpt(&self, root: CDL_ObjID, vaddr: usize) -> Option<CDL_Cap> {
            self.get_spec_object(root).get_cap_at_mut(PML4_SLOT(vaddr))
        }

        let pdpt_cap = self.get_cdl_frame_pdpt_mut(root, vaddr)?;
        self.get_spec_object(pdpt_cap.obj_id)
            .get_cap_at(PDPT_SLOT(vaddr))
    }
}
