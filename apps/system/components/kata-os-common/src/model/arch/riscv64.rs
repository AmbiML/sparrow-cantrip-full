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

// RISC-V 64-bit target support.

#![allow(non_camel_case_types)]

use static_assertions::assert_cfg;
assert_cfg!(target_arch = "riscv64");

mod riscv;
pub use riscv::*;

use crate::CantripOsModel;
use capdl::CDL_CapType::*;
use capdl::*;

use sel4_sys::seL4_HugePageBits;
use sel4_sys::seL4_LargePageBits;
use sel4_sys::seL4_ObjectType;
use sel4_sys::seL4_ObjectType::*;
use sel4_sys::seL4_PageBits;
use sel4_sys::seL4_PageTableIndexBits;
use sel4_sys::seL4_Result;
use sel4_sys::seL4_TeraPageBits;
use sel4_sys::seL4_Word;

// TODO(sleffler): support 4 (need CONFIG_PT_LEVELS from gen_headers)
// NB: all levels use the same format so no need for per-level IndexBits
const CDL_PT_NUM_LEVELS: usize = 3;

fn MASK(pow2_bits: usize) -> usize { (1 << pow2_bits) - 1 }

// Returns the virtual address of |base| mapped by |slot| at |level|
fn PT_SLOT_VADDR(base: usize, slot: usize, level: usize) -> usize {
    base + (slot << (((level - 1) * seL4_PageTableIndexBits) + seL4_PageBits))
}
// Returns the slot for |vaddr| at |level| in the PT hierarchy
fn PT_LEVEL_SLOT(vaddr: usize, level: usize) -> usize {
    (vaddr >> ((seL4_PageTableIndexBits * (level - 1)) + seL4_PageBits))
        & MASK(seL4_PageTableIndexBits)
}

pub fn get_frame_type(object_size: seL4_Word) -> seL4_ObjectType {
    match object_size {
        seL4_PageBits => seL4_RISCV_4K_Page,
        seL4_LargePageBits => seL4_RISCV_Mega_Page,
        seL4_HugePageBits => seL4_RISCV_Giga_Page,
        seL4_TeraPageBits => seL4_RISCV_Tera_Page,
        _ => panic!("Unexpected frame size {}", object_size),
    }
}

impl<'a> CantripOsModel<'a> {
    pub fn init_vspace(&mut self, obj_id: CDL_ObjID) -> seL4_Result {
        self.init_level_1(obj_id, 0, obj_id)
    }

    pub fn get_cdl_frame_pt(&self, pd: CDL_ObjID, vaddr: usize) -> Option<&'a CDL_Cap> {
        self.get_cdl_frame_pt_recurse(pd, vaddr, 2)
    }

    /**
     * Do a recursive traversal from the top to bottom of a page table structure to
     * get the cap for a particular page table object for a certain vaddr at a certain
     * level. The level variable treats level==CDL_PT_NUM_LEVELS as the root page table
     * object, and level 0 as the bottom level 4k frames.
     */
    fn get_cdl_frame_pt_recurse(
        &self,
        root: CDL_ObjID,
        vaddr: usize,
        level: usize,
    ) -> Option<&'a CDL_Cap> {
        let obj_id = if level < CDL_PT_NUM_LEVELS {
            self.get_cdl_frame_pt_recurse(root, vaddr, level + 1)?
                .obj_id
        } else {
            root
        };
        self.get_object(obj_id)
            .get_cap_at(PT_LEVEL_SLOT(vaddr, level))
    }

    fn init_level_3(
        &mut self,
        level_0_obj: CDL_ObjID,
        level_3_base: usize,
        level_3_obj: CDL_ObjID,
    ) -> seL4_Result {
        for slot in self.get_object(level_3_obj).slots_slice() {
            let base = PT_SLOT_VADDR(level_3_base, slot.slot, /*level=*/ 1);
            let frame_cap = &slot.cap;
            self.map_page_frame(frame_cap, level_0_obj, frame_cap.cap_rights().into(), base)?;
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
            let base = PT_SLOT_VADDR(level_2_base, slot.slot, /*level=*/ 2);
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
                self.init_level_3(level_0_obj, base, level_3_obj)?;
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
            let base = PT_SLOT_VADDR(level_1_base, slot.slot, /*level=*/ 3);
            let level_2_cap = &slot.cap;
            if level_2_cap.r#type() == CDL_FrameCap {
                self.map_page_frame(
                    level_2_cap,
                    level_0_obj,
                    level_2_cap.cap_rights().into(),
                    base,
                )?;
            } else {
                let level_2_obj = level_2_cap.obj_id;
                self.map_page_table(level_2_cap, level_0_obj, base)?;
                self.init_level_2(level_0_obj, base, level_2_obj)?;
            }
        }
        Ok(())
    }
}
