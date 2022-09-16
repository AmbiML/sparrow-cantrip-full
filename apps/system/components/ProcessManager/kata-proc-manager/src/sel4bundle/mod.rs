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
use alloc::vec;
use core::cmp;
use core::mem::size_of;
use core::ptr;
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
use log::{debug, error, info, trace};

use io::Read;
use cantrip_io as io;

use sel4_sys::seL4_ASIDPool_Assign;
use sel4_sys::seL4_CNode_CapData;
use sel4_sys::seL4_CNode_Move;
use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_CapRights;
use sel4_sys::seL4_Default_VMAttributes;
use sel4_sys::seL4_DomainSet_Set;
use sel4_sys::seL4_EndpointObject;
use sel4_sys::seL4_Error;
use sel4_sys::seL4_MinSchedContextBits;
use sel4_sys::seL4_PageTableObject;
use sel4_sys::seL4_ReplyObject;
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

// Layout of the CNode holding dynamic_objs.  All entries are singletons
// except for STACK_COUNT so symbols up to STACK_SLOT can also be used to
// index into dynamic_objs. Perhaps too fragile...
const TCB_SLOT: usize = 0;
const FAULT_EP_SLOT: usize = TCB_SLOT + 1;
const SDK_EP_SLOT: usize = FAULT_EP_SLOT + 1;
const SDK_REPLY_SLOT: usize = SDK_EP_SLOT + 1;
const SCHED_CONTEXT_SLOT: usize = SDK_REPLY_SLOT + 1;
// TODO(sleffler): VSpace layout is arch-specific
const PD_SLOT: usize = SCHED_CONTEXT_SLOT + 1;
const PT_SLOT: usize = PD_SLOT + 1;
const IPCBUFFER_SLOT: usize = PT_SLOT + 1;
const SDK_RPC_FRAME_SLOT: usize = IPCBUFFER_SLOT + 1;
const STACK_SLOT: usize = SDK_RPC_FRAME_SLOT + 1;
const STACK_COUNT: usize = 4; // 16K for stack (XXX get from manifest)
const FRAME_SLOT: usize = STACK_SLOT + STACK_COUNT;
// NB: FRAME_SLOT count is based on the BundleImage

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
    stack_base: seL4_Word,         // Base address of stack in app's VSpace

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
                "Bundle {} has no entry point, using 0x{:x}",
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
        // force this unless we're willing to use 2-level CSpace addressing
        // everywhere (which is not supported by CAmkES). After the
        // application is constructed, init_cspace() will bulk move all
        // the caps back into the application's CNode and we keep only a
        // cap for the CNode and TCB; this minimizes the slots in our
        // top-level CNode required to support multiple applications. Note
        // this scheme is a simplification of what the rootserver does; it's
        // likely we can greatly simplify that too but since we reclaim
        // rootserver resources after it runs it's not clear how useful that
        // would be.
        //
        // NB: beware the order of this must match *_SLOT above
        // TODO(sleffler): maybe construct the vec to avoid mismatches
        // TODO(sleffler): the toplevel CNode has a fixed size which
        //   can overflow when nframes is non-trivial
        let dynamic_objs = cantrip_object_alloc_in_toplevel(vec![
            // control/main-thread TCB
            ObjDesc::new(seL4_TCBObject, 1, TCB_SLOT),
            // fault redirect to SDK/ProcessManager
            ObjDesc::new(seL4_EndpointObject, 1, FAULT_EP_SLOT),
            // interface to SDK
            ObjDesc::new(seL4_EndpointObject, 1, SDK_EP_SLOT),
            ObjDesc::new(seL4_ReplyObject, 1, SDK_REPLY_SLOT),
            // SchedContext for main thread
            ObjDesc::new(seL4_SchedContextObject, seL4_MinSchedContextBits, SCHED_CONTEXT_SLOT),
            // VSpace root (PD)
            ObjDesc::new(seL4_PageTableObject, 1, PD_SLOT),
            // VSpace page table (PT)
            ObjDesc::new(seL4_PageTableObject, 1, PT_SLOT),
            // IPC buffer frame
            ObjDesc::new(seL4_SmallPageObject, 1, IPCBUFFER_SLOT),
            // RPC to SDK frame?
            ObjDesc::new(seL4_SmallPageObject, 1, SDK_RPC_FRAME_SLOT),
            // Stack frames (guard frames are unpopulated PT slots)
            ObjDesc::new(seL4_SmallPageObject, STACK_COUNT, STACK_SLOT),
            // Page frames for application binary.
            ObjDesc::new(seL4_SmallPageObject, nframes, FRAME_SLOT),
        ])
        .map_err(|_| ProcessManagerError::StartFailed)?;

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

        // XXX setup fault endpoint (allocate id)
        // XXX setup temporal fault endpoint (allocate id)
        // XXX setup SDK runtime (e.g. badge)

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
            stack_base: 0,

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
            let vaddr = section.vaddr;
            if vaddr < first_vaddr {
                first_vaddr = vaddr;
            }
            assert!(vaddr >= prev_vaddr); // XXX return error instead
            if let Some(pc) = section.entry {
                trace!("entry point 0x{:x}", pc);
                // XXX reject multiple entry's
                entry_point = Some(pc);
            }
            let first_frame = vaddr / PAGE_SIZE;
            let last_frame = roundup(vaddr + section.msize, PAGE_SIZE) / PAGE_SIZE;
            nframes += last_frame - first_frame;
            prev_vaddr = vaddr;
        }
        trace!("nframes {} first_vaddr 0x{:x}", nframes, first_vaddr);
        (nframes, first_vaddr, entry_point)
    }

    // Loads the application contents into the new VSpace and return the
    // vaddr of the next frame to be mapped. Assumes the image fits into
    // a single PT level and that the PT has been setup.
    fn load_application(&self) -> Result<usize, seL4_Error> {
        let vm_attribs = seL4_Default_VMAttributes;

        // NB: assumes pd and pt are setup (not sure we can check)
        let pd = &self.dynamic_objs.objs[PD_SLOT];
        // NB: There are 4 stack frames allocated using a single ObjDesc in
        //   dynamic_objs, so page_frames 1 past STACK_SLOT. To be fixed when
        //   dynamic_objs is constructed directly and we have const indices.
        let page_frames = &self.dynamic_objs.objs[STACK_SLOT + 1];
        let bundle_frames = &self.bundle_frames;

        // Map application pages. The |page_frames| are in the top-level
        // CNode but unmapped. We temporarily map them in a copy region
        // to fill from the |bundle_frames| and/or zero-fill.
        let mut image = BundleImage::new(bundle_frames);

        let mut copy_region =
            CopyRegion::new(unsafe { ptr::addr_of_mut!(LOAD_APPLICATION[0]) }, PAGE_SIZE);

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
                copy_region.as_mut()[..].fill(0);
                if data_range.contains(&vaddr) {
                    let start = if index > 0 {
                        0
                    } else {
                        vaddr - data_range.start
                    };
                    let end = cmp::min(data_range.end - vaddr, copy_region.size());
                    image
                        .read_exact(&mut copy_region.as_mut()[start..end])
                        .map_err(|_| seL4_Error::seL4_NoError)?; // XXX
                }
                copy_region.unmap()?;

                // Frame is now setup, map it into the VSpace at the
                // page-aligned virtual address.
                trace!("map slot {} vaddr 0x{:x} {:?}", frame.cptr, frame_vaddr, rights);
                arch::map_page(frame, pd, frame_vaddr, *rights, vm_attribs)?;
                vaddr += frame.size_bytes().unwrap();
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
    fn init_vspace(&mut self) -> seL4_Result {
        let rights_rwn = seL4_CapRights::new(
            // NB: grant =>'s X on ARM+RISCV
            /*grant_reply=*/ 0,
            /*grant=*/ 0, /*read=*/ 1, /*write=*/ 1,
        );
        let vm_attribs = seL4_Default_VMAttributes;

        let pd = &self.dynamic_objs.objs[PD_SLOT];
        let pt = &self.dynamic_objs.objs[PT_SLOT];
        let ipcbuffer_frame = &self.dynamic_objs.objs[IPCBUFFER_SLOT];
        let stack_frames = &self.dynamic_objs.objs[STACK_SLOT];

        // Initializes the VSpace root (PD) in the ASID pool.
        // NB: must happen before anything is mapped.
        unsafe { seL4_ASIDPool_Assign(ASID_POOL, pd.cptr) }?;

        // Map 2nd-level page table.
        arch::map_page_table(pd, pt, 0, vm_attribs)?;

        // Setup the bundle image.
        let vaddr_top = self.load_application()?;

        // Setup the stack & IPC buffer.

        // NB: no need for actual guard pages, just leave 'em unmapped.
        // XXX but this would give a different fault than a write to a read-only
        //   page, need to make sure this works
        let mut vaddr = roundup(vaddr_top, PAGE_SIZE);
        trace!("guard page vaddr 0x{:x}", vaddr);
        vaddr += PAGE_SIZE; // Guard page below stack

        // Save lowest stack address for get_stack_frame_obj().
        self.stack_base = vaddr;
        for index in 0..stack_frames.retype_count() {
            let frame = &stack_frames.new_at(index);
            trace!("map stack slot {} vaddr 0x{:x} {:?}", frame.cptr, vaddr, rights_rwn);
            arch::map_page(frame, pd, vaddr, rights_rwn, vm_attribs)?;
            vaddr += frame.size_bytes().unwrap();
        }
        // TODO(sleffler): sp points to the guard page, do we need - size_of::<seL4_Word>()?
        self.tcb_sp = vaddr; // NB: stack grows down (maybe arch-dependent?)
        trace!("guard page vaddr 0x{:x}", vaddr);
        vaddr += PAGE_SIZE; // Guard page between stack & ipc buffer

        // Map IPC buffer.
        self.tcb_ipcbuffer_addr = vaddr;
        trace!(
            "map ipcbuffer slot {} vaddr 0x{:x} {:?}",
            ipcbuffer_frame.cptr,
            vaddr,
            rights_rwn
        );
        arch::map_page(ipcbuffer_frame, pd, vaddr, rights_rwn, vm_attribs)
    }

    // Sets up the TCB and related state (e.g. scheduler context).
    fn init_tcb(&self) -> seL4_Result {
        let cap_cspace_root = self.cspace_root.objs[0].cptr;
        let cap_vspace_root = self.dynamic_objs.objs[PD_SLOT].cptr;
        let cap_tcb = self.dynamic_objs.objs[TCB_SLOT].cptr;
        let cap_fault_ep = self.dynamic_objs.objs[FAULT_EP_SLOT].cptr;
        let cap_tempfault_ep = cap_fault_ep; // XXX
        let cap_sc = self.dynamic_objs.objs[SCHED_CONTEXT_SLOT].cptr;
        let cap_ipcbuffer = self.dynamic_objs.objs[IPCBUFFER_SLOT].cptr;

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
        scheduler::TCB_Configure(
            cap_tcb,
            // NB: sel4_fault_ep is ignored here with MCS
            cap_fault_ep,
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
            cap_fault_ep,
        )?;
        scheduler::TCB_SetTimeoutEndpoint(cap_tcb, cap_tempfault_ep)?;

        smp::TCB_SetAffinity(cap_tcb, self.affinity)?;

        #[cfg(feature = "CONFIG_DEBUG_BUILD")]
        // Name the thread after its TCB name if possible.
        if let Ok(cstr) = cstr_core::CString::new(self.tcb_name.clone()) {
            use sel4_sys::seL4_DebugNameThread;
            unsafe { seL4_DebugNameThread(cap_tcb, cstr.to_bytes_with_nul()) };
        }

        let mut sp = self.tcb_sp;
        assert_eq!(sp % arch::STACK_ALIGNMENT_BYTES, 0, "TCB stack pointer mis-aligned");

        // XXX nonsense values for testing
        let argv: &[seL4_Word] = &[self.tcb_ipcbuffer_addr, 0x11112222, 0x22223333, 0x44445555];

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
        // Move everything back from the top-level CNode to the
        // application's cspace_root and release the top-level
        // CNode slots used during construction.
        // XXX should we remove the TCB from the CNode?
        // XXX verify no self-ref to the top-level CNode (so
        //   frames etc cannot be modified)
        self.dynamic_objs
            .move_objects_from_toplevel(self.cspace_root.objs[0].cptr, self.cspace_root_depth)?;
        // Keep a dup of the TCB in the top-level CNode for suspend/resume.
        // We do this after the bulk move to insure there's a free slot.
        self.cap_tcb.dup_to(
            self.dynamic_objs.cnode,
            self.dynamic_objs.objs[TCB_SLOT].cptr,
            self.dynamic_objs.depth,
        )?;
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
        &self.dynamic_objs.objs[STACK_SLOT + arch::PT_SLOT(vaddr - self.stack_base)]
    }
}
impl BundleImplInterface for seL4BundleImpl {
    fn start(&mut self) -> Result<(), ProcessManagerError> {
        self.init_vspace()
            .and_then(|_| self.init_tcb())
            .and_then(|_| self.init_cspace())
            .map_err(|_| ProcessManagerError::StartFailed)?;

        self.resume() // XXX maybe map_err StartFailed
    }
    fn stop(&mut self) -> Result<(), ProcessManagerError> {
        self.suspend()?;
        cantrip_object_free_in_cnode(&self.bundle_frames)
            .map_err(|_| ProcessManagerError::StopFailed)?;
        cantrip_object_free_in_cnode(&self.dynamic_objs)
            .map_err(|_| ProcessManagerError::StopFailed)?;
        self.cap_tcb = CSpaceSlot::new(); // NB: force drop
                                          // XXX delete any other local caps
        Ok(())
    }
    fn resume(&self) -> Result<(), ProcessManagerError> {
        unsafe { seL4_TCB_Resume(self.cap_tcb.slot) }.map_err(|_| ProcessManagerError::ResumeFailed)
    }
    fn suspend(&self) -> Result<(), ProcessManagerError> {
        unsafe { seL4_TCB_Suspend(self.cap_tcb.slot) }
            .map_err(|_| ProcessManagerError::SuspendFailed)
    }
    fn capscan(&self) -> Result<(), ProcessManagerError> {
        #[cfg(feature = "CONFIG_PRINTING")]
        unsafe {
            sel4_sys::seL4_DebugDumpCNode(self.cspace_root.objs[0].cptr);
        }
        Ok(())
    }
}
