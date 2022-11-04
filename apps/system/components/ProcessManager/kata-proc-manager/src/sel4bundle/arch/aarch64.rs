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

// ARM aarch64 target support.

#![allow(non_camel_case_types)]

use static_assertions::assert_cfg;
assert_cfg!(target_arch = "aarch64");

use super::sel4_sys;
use super::SLOT_PD;
use super::SLOT_PT;
use super::SLOT_PUD;
use super::SLOT_ROOT;
use cantrip_memory_interface::ObjDesc;
use cantrip_memory_interface::ObjDescBundle;
use log::trace;
use smallvec::SmallVec;

mod arm;
pub use arm::*;

use sel4_sys::seL4_ARM_PageDirectoryObject;
use sel4_sys::seL4_ARM_PageGlobalDirectoryObject;
use sel4_sys::seL4_ARM_PageTableObject;
use sel4_sys::seL4_ARM_PageUpperDirectoryObject;
use sel4_sys::seL4_ARM_SmallPageObject;
use sel4_sys::seL4_CapRights;
use sel4_sys::seL4_Default_VMAttributes;
use sel4_sys::seL4_PageBits;
use sel4_sys::seL4_PageTableIndexBits;
use sel4_sys::seL4_PageTable_Map;
use sel4_sys::seL4_Page_Map;
use sel4_sys::seL4_Result;
use sel4_sys::seL4_UserContext;
use sel4_sys::seL4_VMAttributes;
use sel4_sys::seL4_Word;

use sel4_sys::seL4_ARM_PageDirectory_Map;
use sel4_sys::seL4_ARM_PageUpperDirectory_Map;

pub const INDEX_ROOT: usize = super::INDEX_LAST_COMMON + 1;
const INDEX_PUD: usize = INDEX_ROOT + 1;
const INDEX_PD: usize = INDEX_PUD + 1;
const INDEX_PT: usize = INDEX_PD + 1;
const INDEX_MAX: usize = INDEX_PT + 1;
pub type DynamicDescs = [ObjDesc; INDEX_MAX];

pub fn add_vspace_desc(desc: &mut SmallVec<DynamicDescs>) {
    // VSpace root: page global directory (PGD)
    desc.push(ObjDesc::new(seL4_ARM_PageGlobalDirectoryObject, 1, SLOT_ROOT));
    debug_assert_eq!(INDEX_ROOT, desc.len() - 1);

    // VSpace page upper directory (PUD)
    desc.push(ObjDesc::new(seL4_ARM_PageUpperDirectoryObject, 1, SLOT_PUD));
    debug_assert_eq!(INDEX_PUD, desc.len() - 1);

    // VSpace page directory (PD)
    desc.push(ObjDesc::new(seL4_ARM_PageDirectoryObject, 1, SLOT_PD));
    debug_assert_eq!(INDEX_PD, desc.len() - 1);

    // VSpace page table (PT)
    desc.push(ObjDesc::new(seL4_ARM_PageTableObject, 1, SLOT_PT));
    debug_assert_eq!(INDEX_PT, desc.len() - 1);
}

pub fn init_page_tables(dynamic_objs: &ObjDescBundle, first_vaddr: seL4_Word) -> seL4_Result {
    let root = &dynamic_objs.objs[INDEX_ROOT];
    let pud = &dynamic_objs.objs[INDEX_PUD];
    let pd = &dynamic_objs.objs[INDEX_PD];
    let pt = &dynamic_objs.objs[INDEX_PT];

    // Map 3rd level page table.
    map_page_upper_dir(pud, root, 0, seL4_Default_VMAttributes)?;

    // Map 2nd level page table.
    map_page_dir(pd, root, 0, seL4_Default_VMAttributes)?;

    // Map 1st-level page table.
    // Calculate the PD entry/address using the first vaddr of the
    // application. We only (currently) support one 2nd-level page table
    // to map the application, stack, etc. so everything has to fit
    // in 2MiB of virtual memory.
    let vaddr = PD_SLOT(first_vaddr) << (seL4_PageTableIndexBits + seL4_PageBits);
    map_page_table(pt, root, vaddr, seL4_Default_VMAttributes)
}

pub fn get_user_context(
    pc: seL4_Word,
    sp: seL4_Word,
    argv: &[seL4_Word],
) -> *const seL4_UserContext {
    #[rustfmt::skip]
    static mut regs: seL4_UserContext = seL4_UserContext {
        pc: 0, sp: 0, spsr: 0,
        x0:  0, x1:  0, x2:  0, x3:  0, x4:  0, x5:  0, x6:  0, x7:  0,
        x8:  0, x9:  0, x10: 0, x11: 0, x12: 0, x13: 0, x14: 0, x15: 0,
        x16: 0, x17: 0, x18: 0, x19: 0, x20: 0, x21: 0, x22: 0, x23: 0,
        x24: 0, x25: 0, x26: 0, x27: 0, x28: 0, x29: 0, x30: 0,
        tpidr_el0: 0, tpidrro_el0: 0,
    };

    unsafe {
        regs.pc = pc;
        regs.sp = sp; // NB: may be adjusted from cdl_tcb.tcb_sp()

        regs.x0 = if argv.len() > 0 { argv[0] } else { 0 };
        regs.x1 = if argv.len() > 1 { argv[1] } else { 0 };
        regs.x2 = if argv.len() > 2 { argv[2] } else { 0 };
        regs.x3 = if argv.len() > 3 { argv[3] } else { 0 };

        &regs as *const seL4_UserContext
    }
}

pub fn map_page(
    frame: &ObjDesc,
    root: &ObjDesc,
    vaddr: seL4_Word,
    rights: seL4_CapRights,
    vm_attribs: seL4_VMAttributes,
) -> seL4_Result {
    trace!("map page {:?} root {:?} at {:#x}", frame, root, vaddr);
    assert_eq!(frame.type_, seL4_ARM_SmallPageObject);
    assert_eq!(root.type_, seL4_ARM_PageGlobalDirectoryObject);
    unsafe { seL4_Page_Map(frame.cptr, root.cptr, vaddr, rights, vm_attribs) }
}

// TODO(sleffler): need variant for *PageObject
fn map_page_table(
    pt: &ObjDesc,
    root: &ObjDesc,
    vaddr: seL4_Word,
    vm_attribs: seL4_VMAttributes,
) -> seL4_Result {
    trace!("map page table pt {:?} root {:?} at {:#x}", pt, root, vaddr);
    assert_eq!(pt.type_, seL4_ARM_PageTableObject);
    assert_eq!(root.type_, seL4_ARM_PageGlobalDirectoryObject);
    unsafe { seL4_PageTable_Map(pt.cptr, root.cptr, vaddr, vm_attribs) }
}

// TODO(sleffler): need variant for *PageObject
fn map_page_dir(
    pd: &ObjDesc,
    root: &ObjDesc,
    vaddr: seL4_Word,
    vm_attribs: seL4_VMAttributes,
) -> seL4_Result {
    trace!("map page dir pd {:?} root {:?} at {:#x}", pd, root, vaddr);
    assert_eq!(pd.type_, seL4_ARM_PageDirectoryObject);
    assert_eq!(root.type_, seL4_ARM_PageGlobalDirectoryObject);
    unsafe { seL4_ARM_PageDirectory_Map(pd.cptr, root.cptr, vaddr, vm_attribs) }
}

fn map_page_upper_dir(
    pud: &ObjDesc,
    root: &ObjDesc,
    vaddr: seL4_Word,
    vm_attribs: seL4_VMAttributes,
) -> seL4_Result {
    trace!("map page upper dir pud {:?} root {:?} at {:#x}", pud, root, vaddr);
    assert_eq!(pud.type_, seL4_ARM_PageUpperDirectoryObject);
    assert_eq!(root.type_, seL4_ARM_PageGlobalDirectoryObject);
    unsafe { seL4_ARM_PageUpperDirectory_Map(pud.cptr, root.cptr, vaddr, vm_attribs) }
}
