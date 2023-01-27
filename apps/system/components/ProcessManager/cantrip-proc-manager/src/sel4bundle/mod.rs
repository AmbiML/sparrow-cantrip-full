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

//! Cantrip OS seL4 bundle support

#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

extern crate alloc;
use alloc::string::String;
use cantrip_memory_interface::cantrip_cnode_alloc;
use cantrip_memory_interface::cantrip_object_alloc_in_toplevel;
use cantrip_memory_interface::cantrip_object_free;
use cantrip_memory_interface::cantrip_object_free_in_cnode;
use cantrip_memory_interface::ObjDesc;
use cantrip_memory_interface::ObjDescBundle;
use cantrip_os_common::copyregion::CopyRegion;
use cantrip_os_common::cspace_slot::CSpaceSlot;
use cantrip_os_common::scheduling::Domain;
use cantrip_os_common::sel4_sys;
use cantrip_proc_interface::Bundle;
use cantrip_proc_interface::BundleImage;
use cantrip_proc_interface::BundleImplInterface;
use cantrip_proc_interface::ProcessManagerError;
use cantrip_sdk_manager::cantrip_sdk_manager_get_endpoint;
use cantrip_sdk_manager::cantrip_sdk_manager_release_endpoint;
use core::cmp;
use core::mem::size_of;
use core::ptr;
#[cfg(feature = "CONFIG_CHECK_BUNDLE_IMAGE")]
use crc::{crc32, Hasher32};
use log::{debug, error, info, trace};
use smallvec::smallvec;
use smallvec::SmallVec;

use cantrip_io as io;
use io::Read;

use sel4_sys::seL4_ASIDPool_Assign;
use sel4_sys::seL4_CNode_CapData;
use sel4_sys::seL4_CNode_Move;
use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_CapRights;
use sel4_sys::seL4_Default_VMAttributes;
use sel4_sys::seL4_DomainSet_Set;
use sel4_sys::seL4_Error;
use sel4_sys::seL4_MinSchedContextBits;
use sel4_sys::seL4_Result;
use sel4_sys::seL4_SchedContextObject;
use sel4_sys::seL4_SmallPageObject;
use sel4_sys::seL4_TCBObject;
use sel4_sys::seL4_TCB_Resume;
use sel4_sys::seL4_TCB_Suspend;
use sel4_sys::seL4_TCB_WriteRegisters;
use sel4_sys::seL4_UserContext;
use sel4_sys::seL4_Word;
use sel4_sys::seL4_WordBits;

use static_assertions::const_assert;

extern "C" {
    // The rootserver hands-off these caps because we mark our CAmkES
    // component. Well-known C symbols identify the slots where the
    // caps land our CSpace.
    static ASID_POOL: seL4_CPtr;
    static SCHED_CTRL: seL4_CPtr;
    static DOMAIN_CTRL: seL4_CPtr;

    // Our thread's TCB; used in setting up scheduling of new TCB's.
    static SELF_TCB_PROCESS_MANAGER_PROC_CTRL_0000: seL4_CPtr;

    // Region for mapping data when loading the contents of a BundleImage.
    static mut LOAD_APPLICATION: [seL4_Word; PAGE_SIZE / size_of::<seL4_Word>()];
}
use SELF_TCB_PROCESS_MANAGER_PROC_CTRL_0000 as SELF_TCB;

// Setup arch- & feature-specific support.

// Target-architecture specific support (please keep sorted)
#[cfg_attr(target_arch = "aarch64", path = "arch/aarch64.rs")]
#[cfg_attr(target_arch = "riscv32", path = "arch/riscv32.rs")]
mod arch;

use arch::PAGE_SIZE;

// MCS feature support
#[cfg_attr(feature = "CONFIG_KERNEL_MCS", path = "feature/mcs.rs")]
#[cfg_attr(not(feature = "CONFIG_KERNEL_MCS"), path = "feature/no_mcs.rs")]
mod scheduler;

// SMP feature support
#[cfg_attr(feature = "CONFIG_SMP_SUPPORT", path = "feature/smp.rs")]
#[cfg_attr(not(feature = "CONFIG_SMP_SUPPORT"), path = "feature/no_smp.rs")]
mod smp;

// Spill TCB arguments to stack support
#[cfg_attr(
    feature = "CONFIG_CAPDL_LOADER_CC_REGISTERS",
    path = "feature/no_spill_tcb_args.rs"
)]
#[cfg_attr(
    not(feature = "CONFIG_CAPDL_LOADER_CC_REGISTERS"),
    path = "feature/spill_tcb_args.rs"
)]
mod tcb_args;

// seL4_WordBits is passed as a depth parameter for CSpace addresses
// (sigh); verify it fits in a u8 until we can change the api's
const_assert!(seL4_WordBits == 32 || seL4_WordBits == 64);

// Constructs a CSpace guard word from bits & size (see the seL4 manual
// for an explanation of how this is used).
fn make_guard(guard_bits: seL4_Word, guard_size: seL4_Word) -> seL4_Word {
    seL4_CNode_CapData::new(guard_bits, guard_size).words[0]
}

fn roundup(a: usize, b: usize) -> usize { ((a + b - 1) / b) * b }

#[allow(dead_code)]
fn is_path_empty((root, index, depth): (seL4_CPtr, seL4_CPtr, u8)) -> bool {
    let e = unsafe { seL4_CNode_Move(root, index, depth, root, index, depth) };
    e == Err(sel4_sys::seL4_Error::seL4_FailedLookup)
}

#[allow(dead_code)]
fn check_bundle(bundle: &ObjDescBundle) {
    for od in &bundle.objs {
        let mut path = (bundle.cnode, 0, bundle.depth);
        for off in 0..od.retype_count() {
            path.1 = od.cptr + off;
            if is_path_empty(path) {
                debug!("{:?} empty", &path);
            } else {
                debug!("{:?} occupied", &path);
            }
        }
    }
}

const NOCAP: seL4_CPtr = 0;

// Initial layout of the CNode holding dynamic_objs. See the comment
// below about cantrip_object_alloc_in_toplevel linearizing the caps
// returned by MemoryManager.
// TODO(sleffler): SDK runtime state should be setup by SDK in case it
//    needs more than 1 endpoint + 1 small frame
const SLOT_TCB: usize = 0;
const SLOT_SCHED_CONTEXT: usize = SLOT_TCB + 1;
// NB: reserve slots for a 4-level page mapping; on arch's that
//   need fewer the middle slots will not be used
const SLOT_ROOT: usize = SLOT_SCHED_CONTEXT + 1;
const SLOT_PUD: usize = SLOT_ROOT + 1;
const SLOT_PD: usize = SLOT_PUD + 1;
const SLOT_PT: usize = SLOT_PD + 1;
const SLOT_IPCBUFFER: usize = SLOT_PT + 1;
const SLOT_SDK_FRAME: usize = SLOT_IPCBUFFER + 1;
const SLOT_STACK: usize = SLOT_SDK_FRAME + 1;
const STACK_COUNT: usize = 4; // 16K for stack (XXX get from manifest)
const SLOT_FRAME: usize = SLOT_STACK + STACK_COUNT;
// NB: SLOT_FRAME count is based on the BundleImage

// Indices of ObjDesc items in dynamic_objs.
// Each arch appends the ObjDesc's they need for VSpace construction;
// one of which must be named INDEX_ROOT (for the VSpace root)
const INDEX_TCB: usize = 0;
const INDEX_SCHED_CONTEXT: usize = INDEX_TCB + 1;
const INDEX_IPCBUFFER: usize = INDEX_SCHED_CONTEXT + 1;
const INDEX_SDK_FRAME: usize = INDEX_IPCBUFFER + 1;
const INDEX_STACK: usize = INDEX_SDK_FRAME + 1;
const INDEX_FRAME: usize = INDEX_STACK + 1;
const INDEX_LAST_COMMON: usize = INDEX_FRAME;
// arch-specific descriptors start at INDEX_LAST_COMMON + 1

pub struct seL4BundleImpl {
    // Application binary pages ordered by virtual address.
    bundle_frames: ObjDescBundle,

    // Dynamically allocated CSpace contents; these start out in our
    // top-level CNode but are then moved to cspace_root.
    dynamic_objs: ObjDescBundle,

    // Top-level CNode for application. This resides in our top-level
    // CNode so long as the application is active.
    cspace_root: ObjDescBundle,

    // Application thread for start/suspend/resume. This starts out
    // in the cspace_root until after the CSpace is constructed when
    // we dup the capability into our top-level CNode for suspend/resume.
    cap_tcb: CSpaceSlot,

    affinity: seL4_Word, // CPU affinity
    domain: Domain,      // Scheduling domain

    tcb_name: String,
    tcb_max_priority: seL4_Word,
    tcb_priority: seL4_Word,
    tcb_ipcbuffer_addr: seL4_Word, // Address of IPCBuffer in app's VSpace
    tcb_pc: seL4_Word,             // Initial pc in app's VSpace
    tcb_sp: seL4_Word,             // Initial stack pointer in app's VSpace

    sdk_ep_slot: seL4_CPtr,
    sdk_ep: CSpaceSlot,

    sdk_frame_addr: seL4_Word, // Address of SDK frame in app's VSpace
    stack_base: seL4_Word,     // Base address of stack in app's VSpace
    first_vaddr: seL4_Word,

    cspace_root_data: seL4_Word,
    cspace_root_depth: u8,

    vspace_root_data: seL4_Word,

    sc_budget: u64,
    sc_data: seL4_Word,
    sc_period: u64,
}
impl seL4BundleImpl {
    pub fn new(
        bundle: &Bundle,
        bundle_frames: &ObjDescBundle,
    ) -> Result<Self, ProcessManagerError> {
        trace!("seL4BundleImpl::new {:?} bundle_frames {}", bundle, bundle_frames);

        sel4_sys::debug_assert_slot_cnode!(bundle_frames.cnode);

        // TODO(sleffler): parse/extract from manifest to construct BundleImpl

        // Calculate how many pages are needed and
        // (while we're here) the entry point.
        let (nframes, first_vaddr, entry_point) =
            seL4BundleImpl::preprocess_bundle_image(bundle_frames);
        if entry_point.is_none() {
            info!(
                "Bundle {} has no entry point, using {:#x}",
                &bundle.app_id, first_vaddr
            );
            // XXX should probably just return but need to verify
            //    bundle_frames is reclaimed
        }
        // TODO(sleffler): reject empty image or no entry point?
        // TODO(sleffler): could sanity check memory requirements but
        //    for now just let MemoryManager say it lacks resources

        // Allocate the objects needed for the application. Everything
        // lands in the top-level CNode because the seL4 api's pretty much
        // force this. After the application is constructed, init_cspace()
        // will bulk move all the caps back into the application's CNode
        // and we keep only a cap for the CNode and TCB; this minimizes
        // the slots in our top-level CNode required to support multiple
        // applications. Note this scheme is a simplification of what the
        // rootserver does; it's likely we can simplify that too but
        // since we reclaim rootserver resources after it runs it's not
        // clear how useful that would be.
        //
        // VSpace construction work is done per-arch. We craft the set
        // of ObjDesc's on the stack using the arch::DynamicDescs type to
        // size the SmallVec, fill in the common descriptors, then ask the
        // arch to add what it needs to setup the VSpace. Common code does
        // not know about the VSpace internals. The end result is the CNode
        // with caps packed so that bulk operations have no empty slots
        // to trigger errors.
        //
        // NB: the toplevel CNode has a fixed size which can overflow when
        //    nframes is big (we assume app sizes are small'ish many places)
        // NB: using the stack for the work below seems preferrable to the
        //    heap because sizes are fixed: SmallVec + dynamic_objs +
        //    sel4BundleImpl return

        // NB: the order here must match INDEX_*
        let mut desc: SmallVec<arch::DynamicDescs> = smallvec![
            // Control/main-thread TCB.
            ObjDesc::new(seL4_TCBObject, 1, SLOT_TCB),
            // SchedContext for main thread
            ObjDesc::new(seL4_SchedContextObject, seL4_MinSchedContextBits, SLOT_SCHED_CONTEXT),
            // IPC buffer frame.
            ObjDesc::new(seL4_SmallPageObject, 1, SLOT_IPCBUFFER),
            // Frame for SDK RPC parameters.
            ObjDesc::new(seL4_SmallPageObject, 1, SLOT_SDK_FRAME),
            // Stack frames (guard frames are unpopulated PT slots).
            ObjDesc::new(seL4_SmallPageObject, STACK_COUNT, SLOT_STACK),
            // Page frames for application binary.
            ObjDesc::new(seL4_SmallPageObject, nframes, SLOT_FRAME),
        ];
        debug_assert_eq!(INDEX_LAST_COMMON, desc.len() - 1);

        // Append arch-specific VSpace resources.
        arch::add_vspace_desc(&mut desc);
        debug_assert!(!desc.spilled());

        // Calculate SDK endpoint slot in the final CSpace. This is just
        // one slot past the last common capability.
        let sdk_ep_slot = desc
            .iter()
            .map(|od| od.cptr + od.retype_count())
            .max()
            .unwrap();

        // NB: when caps are moved to our top-level CNode they are linearized
        // so the (careful) layout in |desc| is lost. This means one should
        // not assume SLOT_* are meaningful; use INDEX_* to fetch a cptr
        // from dynamic_objs.
        let dynamic_objs = cantrip_object_alloc_in_toplevel(desc.into_vec())
            .or(Err(ProcessManagerError::StartFailed))?;

        // Allocate the top-level CNode that will hold |dynamic_objs|.
        let cspace_root_depth = dynamic_objs.count_log2();
        let cspace_root = match cantrip_cnode_alloc(cspace_root_depth) {
            Err(e) => {
                error!("seL4BundleImpl::new: cnode alloc failed: {:?}", e);
                info!("seL4BundleImpl::new: dynamic objects: {:?}", &dynamic_objs);
                if let Err(e) = cantrip_object_free(&dynamic_objs) {
                    error!("seL4BundleImpl::new: freeing dynamic_objs returned {:?}", e);
                }
                return Err(ProcessManagerError::StartFailed);
            }
            Ok(cnode) => cnode,
        };

        Ok(seL4BundleImpl {
            bundle_frames: bundle_frames.clone(),
            dynamic_objs,
            cspace_root,
            cap_tcb: CSpaceSlot::new(), // Top-level dup for suspend/resume

            affinity: 0,            // CPU 0
            domain: Domain::System, // TODO(jtgans,sleffler): Figure out how to use this correctly. b/238811077

            tcb_name: bundle.app_id.clone(),
            tcb_max_priority: 254, // TODO(sleffler): guess
            tcb_priority: 254,     // TODO(sleffler): guess
            // NB: next fields are filled in by init_vspace
            tcb_ipcbuffer_addr: 0,
            tcb_pc: entry_point.unwrap_or(first_vaddr), // NB: filled in from BundleImage
            tcb_sp: 0,
            sdk_ep_slot,
            sdk_ep: CSpaceSlot::new(),
            sdk_frame_addr: 0,
            stack_base: 0,
            first_vaddr,

            // 1-level CSpace addressing
            cspace_root_data: make_guard(0, seL4_WordBits - cspace_root_depth),
            cspace_root_depth: cspace_root_depth as u8,

            vspace_root_data: make_guard(0, 0), // XXX unclear effect, need to investigate

            sc_period: 10000, // TODO(sleffler): guess
            sc_budget: 10000, // TODO(sleffler): guess
            sc_data: 0,       // TODO(sleffler): guess
        })
    }

    // Calculate how many pages are needed and and identify the entry point.
    // While we're here also verify segments are ordered by vaddr; this
    // is required by load_application to handle gaps between segments.
    fn preprocess_bundle_image(bundle_frames: &ObjDescBundle) -> (usize, usize, Option<usize>) {
        let mut nframes = 0;
        let mut entry_point = None;
        let mut first_vaddr = usize::MAX;
        let mut prev_vaddr = 0;
        let mut image = BundleImage::new(bundle_frames);
        while let Some(section) = image.next_section() {
            trace!("preprocess {:?}", &section);
            let vaddr = section.vaddr;
            if vaddr < first_vaddr {
                first_vaddr = vaddr;
            }
            assert!(vaddr >= prev_vaddr); // XXX return error instead
            if let Some(pc) = section.entry {
                trace!("entry point {:#x}", pc);
                // XXX reject multiple entry's
                entry_point = Some(pc);
            }
            let first_frame = vaddr / PAGE_SIZE;
            let last_frame = roundup(vaddr + section.msize, PAGE_SIZE) / PAGE_SIZE;
            nframes += last_frame - first_frame;
            prev_vaddr = vaddr;
        }
        trace!("nframes {} first_vaddr {:#x}", nframes, first_vaddr);
        (nframes, first_vaddr, entry_point)
    }

    // Loads the application contents into the new VSpace and return the
    // vaddr of the next frame to be mapped. Assumes the image fits into
    // a single PT level and that the PT has been setup.
    fn load_application(&self) -> Result<usize, seL4_Error> {
        trace!("load_application");
        let vm_attribs = seL4_Default_VMAttributes;

        let root = &self.dynamic_objs.objs[arch::INDEX_ROOT];
        let page_frames = &self.dynamic_objs.objs[INDEX_FRAME];
        let bundle_frames = &self.bundle_frames;

        // Map application pages. The |page_frames| are in the top-level
        // CNode but unmapped. We temporarily map them in a copy region
        // to fill from the |bundle_frames| and/or zero-fill.
        let mut image = BundleImage::new(bundle_frames);

        let mut copy_region =
            unsafe { CopyRegion::new(ptr::addr_of_mut!(LOAD_APPLICATION[0]), PAGE_SIZE) };

        let mut vaddr_top = 0;
        // Track last allocated page that was mapped to handle gaps between
        // segments. Note page_offset is accumulated to handle multiple gaps.
        let mut page_adjust = 0;
        let mut prev_last_page = 0;
        while let Some(section) = image.next_section() {
            trace!("load {:?}", &section);
            let rights = &section.get_rights();
            // Section-adjusted ranges; maybe belongs in BundleImage?
            assert!(section.fsize <= section.msize);
            let data_range = section.vaddr..(section.vaddr + section.fsize);

            // Data is packed in the BundleImage by section. Need to copy
            // page-by-page from this region to pages in the VSpace. Both src
            // and dest are at unknown byte offsets. We linearly read from the
            // section data and calculate how much to copy and where the data
            // lands in the mapped VSpace pages. Zero-fill data are treated
            // identically, just filled with 0's instead of reading (for now
            // we pre-zero each frame which may need to be revisited).
            let mut vaddr = section.vaddr;
            #[cfg(feature = "CONFIG_CHECK_BUNDLE_IMAGE")]
            let mut digest = crc32::Digest::new(crc32::IEEE);

            // Calculate range of pages in the section's VSpace in order to
            // do page-by-page copying or zero-filling.
            let first_page = vaddr / PAGE_SIZE;
            let last_page = roundup(vaddr + section.msize, PAGE_SIZE) / PAGE_SIZE;
            // NB: this assumes segments are ordered by vaddr.
            page_adjust += first_page - prev_last_page;
            let page_range = (first_page - page_adjust)..(last_page - page_adjust);
            for index in page_range {
                let frame = &page_frames.new_at(index);
                let frame_vaddr = (vaddr / PAGE_SIZE) * PAGE_SIZE;

                // Temporarily map the VSpace frame into the copy region to
                // load from the bundle image. For now we pre-zero each frame
                // to avoid dealing with partial zero-fill logic (both from
                // zero_range and "to the left of" data_range).
                copy_region.map(frame.cptr)?;
                copy_region.as_word_mut().fill(0); // A bit faster than as_mut()
                if data_range.contains(&vaddr) {
                    let start = if index > 0 {
                        0
                    } else {
                        vaddr - data_range.start
                    };
                    let end = cmp::min(data_range.end - vaddr, copy_region.size());
                    image
                        .read_exact(&mut copy_region.as_mut()[start..end])
                        .or(Err(seL4_Error::seL4_NoError))?; // XXX
                    #[cfg(feature = "CONFIG_CHECK_BUNDLE_IMAGE")]
                    digest.write(&copy_region.as_ref()[start..end]);
                }
                copy_region.unmap()?;

                // Frame is now setup, map it into the VSpace at the
                // page-aligned virtual address.
                trace!("map slot {} vaddr {:#x} {:?}", frame.cptr, frame_vaddr, rights);
                arch::map_page(frame, root, frame_vaddr, *rights, vm_attribs)?;
                vaddr += frame.size_bytes().unwrap();
            }
            #[cfg(feature = "CONFIG_CHECK_BUNDLE_IMAGE")]
            if section.crc32 != 0 && section.crc32 != (digest.sum32() as usize) {
                error!(
                    "CRC mismatch: section {:#x} != calculated {:#x}",
                    section.crc32,
                    digest.sum32()
                );
            }
            prev_last_page = last_page;
            if vaddr > vaddr_top {
                // NB: leaves an unused frame in the gap but should not matter
                vaddr_top = vaddr;
            }
        }
        Ok(vaddr_top)
    }

    // Construct the VSpace for the application. We use a 2-level page
    // table setup with pages from the provided collection mapped according
    // to the BundleImage section headers. Following the application data
    // is a guard page, the stack, another guard page, and the ipc buffer.
    //
    // NB: guard pages are unmapped frames (not a frame mapped read-only).
    // XXX verify resources are reclaimed on failure?
    // TODO(sleffler): who zero's any of this (or maybe not needed)?
    fn init_vspace(&mut self) -> seL4_Result {
        trace!("init_vspace");
        let rights_rwn = seL4_CapRights::new(
            // NB: grant =>'s X on ARM+RISCV
            /*grant_reply=*/ 0,
            /*grant=*/ 0, /*read=*/ 1, /*write=*/ 1,
        );
        let vm_attribs = seL4_Default_VMAttributes;

        let root = &self.dynamic_objs.objs[arch::INDEX_ROOT];
        let ipcbuffer_frame = &self.dynamic_objs.objs[INDEX_IPCBUFFER];
        let sdk_frame = &self.dynamic_objs.objs[INDEX_SDK_FRAME];
        let stack_frames = &self.dynamic_objs.objs[INDEX_STACK];

        // Initializes the VSpace root (PD) in the ASID pool.
        // NB: must happen before anything is mapped.
        unsafe { seL4_ASIDPool_Assign(ASID_POOL, root.cptr) }?;

        // Setup the page tables. Applications get a 1-level page table
        // for the executable, stack, guard pages, ipc_buffer, etc.
        // Given our target platform has only 4MiB of memory this should
        // fine but for other arch's this may be too restrictive.
        arch::init_page_tables(&self.dynamic_objs, self.first_vaddr)?;

        // Setup the bundle image.
        let vaddr_top = self.load_application()?;

        // Setup the stack & IPC buffer.

        // NB: no need for actual guard pages, just leave 'em unmapped.
        let mut vaddr = roundup(vaddr_top, PAGE_SIZE);
        trace!("guard page vaddr {:#x}", vaddr);
        vaddr += PAGE_SIZE; // Guard page below stack

        // Save lowest stack address for get_stack_frame_obj().
        self.stack_base = vaddr;
        for index in 0..stack_frames.retype_count() {
            let frame = &stack_frames.new_at(index);
            trace!("map stack slot {} vaddr {:#x} {:?}", frame.cptr, vaddr, rights_rwn);
            arch::map_page(frame, root, vaddr, rights_rwn, vm_attribs)?;
            vaddr += frame.size_bytes().unwrap();
        }
        // TODO(sleffler): sp points to the guard page, do we need - size_of::<seL4_Word>()?
        self.tcb_sp = vaddr; // NB: stack grows down (maybe arch-dependent?)
        trace!("guard page vaddr {:#x}", vaddr);
        vaddr += PAGE_SIZE; // Guard page between stack & ipc buffer

        // Map IPC buffer.
        self.tcb_ipcbuffer_addr = vaddr;
        trace!(
            "map ipcbuffer slot {} vaddr {:#x} {:?}",
            ipcbuffer_frame.cptr,
            vaddr,
            rights_rwn,
        );
        arch::map_page(ipcbuffer_frame, root, vaddr, rights_rwn, vm_attribs)?;
        vaddr += ipcbuffer_frame.size_bytes().unwrap();

        // Map SDK RPC frame.
        self.sdk_frame_addr = vaddr;
        trace!(
            "map sdk_runtime slot {} vaddr {:#x} {:?}",
            sdk_frame.cptr,
            vaddr,
            rights_rwn,
        );
        arch::map_page(sdk_frame, root, vaddr, rights_rwn, vm_attribs)?;

        Ok(())
    }

    // Sets up the TCB and related state (e.g. scheduler context).
    fn init_tcb(&self) -> seL4_Result {
        trace!("init_tcb");
        let cap_cspace_root = self.cspace_root.objs[0].cptr;
        let cap_vspace_root = self.dynamic_objs.objs[arch::INDEX_ROOT].cptr;
        let cap_tcb = self.dynamic_objs.objs[INDEX_TCB].cptr;
        let cap_sc = self.dynamic_objs.objs[INDEX_SCHED_CONTEXT].cptr;
        let cap_ipcbuffer = self.dynamic_objs.objs[INDEX_IPCBUFFER].cptr;

        // Calculate the SDK frame's slot in the app's CSpace by mapping
        // the current cptr (in ProcessManager's CSpace).
        // XXX don't need to enumerate all cptrs; just use od.cptr
        let min_cptr = self.dynamic_objs.cptr_iter().min().unwrap();
        let sdk_frame_slot = self.dynamic_objs.objs[INDEX_SDK_FRAME].cptr - min_cptr;

        // Install a badged SDK endpoint in the toplevel cspace for now. We'll
        // move it in init_cspace later. We have to do this in the toplevel
        // cspace because the fault handler is copied implicitly from this
        // thread's root CSpace into the new thread's TCB when MCS is enabled.
        cantrip_sdk_manager_get_endpoint(&self.tcb_name, &self.sdk_ep)
            .or(Err(seL4_Error::seL4_NoError))?; // XXX error

        // XXX MCS v non-MCS
        if cap_sc != NOCAP {
            // TODO(sleffler): we only support non-SMP systems: the rootserver
            //   only passes one SchedControl capability.
            assert_eq!(self.affinity, 0);

            scheduler::SchedControl_Configure(
                unsafe { SCHED_CTRL },
                cap_sc,
                self.affinity,
                self.sc_budget,
                self.sc_period,
                self.sc_data,
            )?;
        }
        assert!(self.tcb_ipcbuffer_addr != 0);
        let fault_ep_slot = self.sdk_ep.slot;
        scheduler::TCB_Configure(
            cap_tcb,
            // NB: sel4_fault_ep is ignored here with MCS
            fault_ep_slot,
            cap_cspace_root,
            self.cspace_root_data,
            cap_vspace_root,
            self.vspace_root_data,
            self.tcb_ipcbuffer_addr,
            cap_ipcbuffer,
        )?;
        scheduler::TCB_SchedParams(
            cap_tcb,
            unsafe { SELF_TCB }, // XXX
            self.tcb_max_priority,
            self.tcb_priority,
            cap_sc,
            fault_ep_slot,
        )?;
        scheduler::TCB_SetTimeoutEndpoint(cap_tcb, fault_ep_slot)?;

        smp::TCB_SetAffinity(cap_tcb, self.affinity)?;

        #[cfg(feature = "CONFIG_DEBUG_BUILD")]
        // Name the thread after its TCB name if possible.
        if let Ok(cstr) = cstr_core::CString::new(self.tcb_name.clone()) {
            use sel4_sys::seL4_DebugNameThread;
            unsafe { seL4_DebugNameThread(cap_tcb, cstr.to_bytes_with_nul()) };
        }

        let mut sp = self.tcb_sp;
        assert_eq!(sp % arch::STACK_ALIGNMENT_BYTES, 0, "TCB stack pointer mis-aligned");

        let argv: &[seL4_Word] = &[
            self.tcb_ipcbuffer_addr, // Used to setup __sel4_ipc_buffer
            self.sdk_ep_slot,        // For SDKRuntime IPCs
            sdk_frame_slot,          // NB: must wrt application CSpace
            self.sdk_frame_addr,     // For SDKRuntime parameters
        ];

        // NB: tcb_args::maybe_spill_tcb_args may write arg data to the
        // stack causing the stack pointer to be adjusted.
        sp = self.maybe_spill_tcb_args(sp, argv)?;
        assert_eq!(
            sp % arch::STACK_ALIGNMENT_BYTES,
            0,
            "Spilled TCB stack pointer mis-aligned"
        );

        unsafe {
            seL4_TCB_WriteRegisters(
                cap_tcb,
                0,
                0,
                size_of::<seL4_UserContext>() / size_of::<seL4_Word>(),
                arch::get_user_context(self.tcb_pc, sp, argv),
            )?;
            seL4_DomainSet_Set(DOMAIN_CTRL, self.domain as u8, cap_tcb)?;
        }
        Ok(())
    }

    // Do the final work to construct the application's CSpace.
    fn init_cspace(&mut self) -> seL4_Result {
        trace!("init_cspace");

        // Move the badged SDK endpoint into the new cspace
        self.sdk_ep.move_from(
            self.cspace_root.objs[0].cptr,
            self.sdk_ep_slot,
            self.cspace_root_depth,
        )?;

        // Move everything back from the top-level CNode to the application's
        // cspace_root and release the top-level CNode slots used during
        // construction. Note this does not clobber the sdk_endpoint because
        // that slot is carefully avoided in dynamic_objs.
        self.dynamic_objs
            .move_objects_from_toplevel(self.cspace_root.objs[0].cptr, self.cspace_root_depth)?;

        // Keep a dup of the TCB in the top-level CNode for suspend/resume.
        // We do this after the bulk move to insure there's a free slot.
        self.cap_tcb.dup_to(
            self.dynamic_objs.cnode,
            self.dynamic_objs.objs[INDEX_TCB].cptr,
            self.dynamic_objs.depth,
        )?;

        // TODO(sleffler): remove the TCB from the CNode
        Ok(())
    }

    // Locate the stack page Frame associated with |vaddr|.
    // This is used when doing argv spillover to the stack.
    // NB: cannot be called before init_vspace sets up the stack
    fn get_stack_frame_obj(&self, vaddr: usize) -> &ObjDesc {
        assert!(
            self.stack_base <= vaddr && vaddr <= self.tcb_sp,
            "Invalid stack address {:x} not in range [{:x}:{:x}]",
            vaddr,
            self.stack_base,
            self.tcb_sp
        );
        &self.dynamic_objs.objs[INDEX_STACK + arch::PT_SLOT(vaddr - self.stack_base)]
    }
}
impl BundleImplInterface for seL4BundleImpl {
    fn start(&mut self) -> Result<(), ProcessManagerError> {
        fn handle_error(e: seL4_Error) -> ProcessManagerError {
            error!("start failed: {:?}", e);
            ProcessManagerError::StartFailed
        }
        self.init_vspace()
            .and_then(|_| self.init_tcb())
            .and_then(|_| self.init_cspace())
            .map_err(handle_error)?;

        self.resume() // XXX maybe map_err StartFailed
    }
    fn stop(&mut self) -> Result<(), ProcessManagerError> {
        self.suspend()?;
        cantrip_sdk_manager_release_endpoint(&self.tcb_name)
            .or(Err(ProcessManagerError::StopFailed))?;
        cantrip_object_free_in_cnode(&self.bundle_frames)
            .or(Err(ProcessManagerError::StopFailed))?;
        // NB: must delete the dup reference to the TCB before cleaning
        //    up the application's CNode so the container is treated as
        //    revokable. Otherwise seL4 will defer reclaiming the space
        //    used for the CNode which will in-turn make the parent
        //    untyped slab "occupied" and block it from being reset on
        //    the next retype operation. This will not be necessary
        //    when we remove the TCB reference in the CNode.
        self.cap_tcb = CSpaceSlot::new(); // NB: force drop
        cantrip_object_free_in_cnode(&self.dynamic_objs)
            .or(Err(ProcessManagerError::StopFailed))?;
        // XXX delete any other local caps
        Ok(())
    }
    fn resume(&self) -> Result<(), ProcessManagerError> {
        unsafe { seL4_TCB_Resume(self.cap_tcb.slot) }.or(Err(ProcessManagerError::ResumeFailed))
    }
    fn suspend(&self) -> Result<(), ProcessManagerError> {
        unsafe { seL4_TCB_Suspend(self.cap_tcb.slot) }.or(Err(ProcessManagerError::SuspendFailed))
    }
    fn capscan(&self) -> Result<(), ProcessManagerError> {
        #[cfg(feature = "CONFIG_PRINTING")]
        unsafe {
            sel4_sys::seL4_DebugDumpCNode(self.cspace_root.objs[0].cptr);
        }
        Ok(())
    }
}
