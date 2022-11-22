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

//! RISC-V 32-bit target support.

#![allow(non_camel_case_types)]

use static_assertions::assert_cfg;
assert_cfg!(target_arch = "riscv32");

use super::sel4_sys;
use super::SLOT_PT;
use super::SLOT_ROOT;
use cantrip_memory_interface::ObjDesc;
use cantrip_memory_interface::ObjDescBundle;
use log::trace;
use smallvec::SmallVec;

mod riscv;
pub use riscv::*;

use sel4_sys::seL4_CapRights;
use sel4_sys::seL4_Default_VMAttributes;
use sel4_sys::seL4_PageBits;
use sel4_sys::seL4_PageTableIndexBits;
use sel4_sys::seL4_PageTable_Map;
use sel4_sys::seL4_Page_Map;
use sel4_sys::seL4_RISCV_4K_Page;
use sel4_sys::seL4_RISCV_PageTableObject;
use sel4_sys::seL4_Result;
use sel4_sys::seL4_UserContext;
use sel4_sys::seL4_VMAttributes;
use sel4_sys::seL4_Word;

pub const INDEX_ROOT: usize = super::INDEX_LAST_COMMON + 1;
const INDEX_PT: usize = INDEX_ROOT + 1;
const INDEX_MAX: usize = INDEX_PT + 1;
pub type DynamicDescs = [ObjDesc; INDEX_MAX];

pub fn add_vspace_desc(desc: &mut SmallVec<DynamicDescs>) {
    // VSpace root: page table (PT)
    desc.push(ObjDesc::new(seL4_RISCV_PageTableObject, 1, SLOT_ROOT));
    debug_assert_eq!(INDEX_ROOT, desc.len() - 1);

    // VSpace page table (PT)
    desc.push(ObjDesc::new(seL4_RISCV_PageTableObject, 1, SLOT_PT));
    debug_assert_eq!(INDEX_PT, desc.len() - 1);
}

pub fn init_page_tables(dynamic_objs: &ObjDescBundle, first_vaddr: seL4_Word) -> seL4_Result {
    let root = &dynamic_objs.objs[INDEX_ROOT];
    let pt = &dynamic_objs.objs[INDEX_PT];

    // Calculate the PD entry/address using the first vaddr of the
    // application. We only (currently) support one 2nd-level page table
    // to map the application, stack, etc. so everything has to fit
    // in 4MiB of virtual memory.
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

pub fn map_page(
    frame: &ObjDesc,
    root: &ObjDesc,
    vaddr: seL4_Word,
    rights: seL4_CapRights,
    vm_attribs: seL4_VMAttributes,
) -> seL4_Result {
    trace!("map page {:?} root {:?} at {:#x}", frame, root, vaddr);
    assert_eq!(frame.type_, seL4_RISCV_4K_Page);
    assert_eq!(root.type_, seL4_RISCV_PageTableObject);
    unsafe { seL4_Page_Map(frame.cptr, root.cptr, vaddr, rights, vm_attribs) }
}

fn map_page_table(
    pt: &ObjDesc,
    root: &ObjDesc,
    vaddr: seL4_Word,
    vm_attribs: seL4_VMAttributes,
) -> seL4_Result {
    trace!("map page table pt {:?} root {:?} at {:#x}", pt, root, vaddr);
    assert_eq!(pt.type_, seL4_RISCV_PageTableObject);
    assert_eq!(root.type_, seL4_RISCV_PageTableObject);
    unsafe { seL4_PageTable_Map(pt.cptr, root.cptr, vaddr, vm_attribs) }
}
