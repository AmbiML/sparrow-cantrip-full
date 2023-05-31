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

//! Cantrip OS memory management support

#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::borrow::Cow;
use alloc::vec;
use alloc::vec::Vec;
use cantrip_os_common::camkes;
use cantrip_os_common::sel4_sys;
use cantrip_os_common::slot_allocator;
use core::fmt;
use log::trace;
use num_enum::{FromPrimitive, IntoPrimitive};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use camkes::*;

use sel4_sys::seL4_CNode_Move;
use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_Error;
use sel4_sys::seL4_ObjectType;
use sel4_sys::seL4_ObjectType::*;
use sel4_sys::seL4_PageBits;
use sel4_sys::seL4_PageTableObject;
use sel4_sys::seL4_Result;
use sel4_sys::seL4_SmallPageObject;
use sel4_sys::seL4_WordBits;

use slot_allocator::CANTRIP_CSPACE_SLOTS;

extern "Rust" {
    // Each CAmkES-generated CNode has a writable self-reference to itself in
    // the slot SELF_CNODE to enable dynamic management of capabilities.
    static SELF_CNODE: seL4_CPtr;

    // Each CAmkES-component has a CNode setup at a well-known slot. In lieu
    // of any supplied CNode we can use that container to pass capabilities.
    static MEMORY_RECV_CNODE: seL4_CPtr;
    static MEMORY_RECV_CNODE_DEPTH: u8;
}

// The MemoryManager takes collections of Object Descriptors.
//
// For an alloc request an object descriptor provides everything needed
// to allocate & retype untyped memory. Capabilities for the realized
// objects are attached to the IPC buffer holding the reply in a CNode
// container. For free requests the same object descriptors should be
// provided. Otherwise clients are responsible for filling in
// allocated objects; e.g. map page frames into a VSpace, bind endpoints
// to irq's, configure TCB slots, etc.
//
// TODO(sleffler): support setting fixed physical address for drivers
// TODO(sleffler): maybe allocate associated resources like endpoint #'s?
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct ObjDesc {
    // Requested object type or type of object being released.
    pub type_: seL4_ObjectType,

    // Count of consecutive objects with the same type or, for CNode
    // objects the log2 number of slots to use in sizing the object,
    // or for untyped objects the log2 size in bytes, or for scheduler
    // context objects the size in bits. See seL4_ObjectType::size_bits().
    count: usize, // XXX oversized (except for untyped use)

    // CSpace address for realized objects requested. If |count| is >1
    // this descriptor describes objects with |cptr|'s [0..|count|).
    // Since each block of objects has it's own |cptr| one can describe
    // a collection with random layout in CSpace (useful for construction).
    //
    // Object capabilities returned by the MemoryManager have the maximal
    // rights. We depend on trusted agents (e.g. ProcessManager) to reduce
    // rights when assigning them to an application. This also applies to
    // the vm attributes of page frames (e.g. mark not executable as
    // appropriate).
    pub cptr: seL4_CPtr,
}
impl ObjDesc {
    pub fn new(type_: seL4_ObjectType, count: usize, cptr: seL4_CPtr) -> Self {
        ObjDesc { type_, count, cptr }
    }

    // Returns a new ObjDesc with count of 1 and the cptr offset by |index|.
    pub fn new_at(&self, index: usize) -> ObjDesc {
        assert!(index < self.retype_count());
        ObjDesc::new(self.type_, 1, self.cptr + index)
    }

    // Parameters for seL4_Untyped_Retype call.
    pub fn retype_size_bits(&self) -> Option<usize> {
        match self.type_ {
            seL4_UntypedObject  // Log2 memory size
            | seL4_CapTableObject // Log2 number of slots
            | seL4_SchedContextObject => Some(self.count), // Log2 context size
            _ => self.type_.size_bits(),
        }
    }
    pub fn retype_count(&self) -> usize {
        match self.type_ {
            // NB: we don't support creating multiple instances of the same
            //   size; the caller must supply multiple object descriptors.
            seL4_UntypedObject | seL4_CapTableObject | seL4_SchedContextObject => 1,
            _ => self.count,
        }
    }

    // Memory occupied by objects. Used mainly for bookkeeping and statistics.
    pub fn size_bytes(&self) -> Option<usize> {
        match self.type_ {
            seL4_UntypedObject | seL4_SchedContextObject => Some(1 << self.count),
            seL4_CapTableObject => self.type_.size_bits().map(|x| (1 << (x + self.count))),
            _ => self.type_.size_bits().map(|x| self.count * (1 << x)),
        }
    }

    // Checks if two descriptors can be combined. This is used to optimize
    // dynamically constructed ObjDescBundle's (e.g. rz::Upload)
    pub fn can_combine(&self, other: &ObjDesc) -> bool {
        self.type_ == other.type_ && self.cptr + self.count == other.cptr
    }
}

// ObjDescBundle holds a collection of ObjDesc's and their associated
// container (i.e. CNode). This enables full "path addressing" of the
// objects. Helper methods do move/copy operations between a component's
// top-level CNode and dynamically allocated CNodes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObjDescBundle {
    pub cnode: seL4_CPtr,
    pub depth: u8,
    pub objs: Vec<ObjDesc>,
}
impl ObjDescBundle {
    pub fn new(cnode: seL4_CPtr, depth: u8, objs: Vec<ObjDesc>) -> Self {
        // TODO(sleffler): assert the largest cptr fits in the container
        ObjDescBundle { cnode, depth, objs }
    }

    // Returns whether there are any object descriptors.
    pub fn is_empty(&self) -> bool { self.objs.len() == 0 }

    // Returns the number of object descriptors.
    pub fn len(&self) -> usize { self.objs.len() }

    // Returns the count of objects specified by the object descriptors.
    pub fn count(&self) -> usize { self.objs.iter().map(|od| od.retype_count()).sum() }

    // Returns the total bytes specified by the object descriptors.
    pub fn size_bytes(&self) -> usize { self.objs.iter().map(|od| od.size_bytes().unwrap()).sum() }

    // Returns the log2 size that holds all the objects. This is typically
    // used to size CNode's based on their intended contents. NB: we return
    // values > 0 since the kernel rejects a CapTable object with size_bits=0.
    pub fn count_log2(&self) -> usize {
        // NB: BITS & leading_zeros return u32
        (1 + usize::BITS - usize::leading_zeros(self.count())) as usize
    }

    pub fn maybe_combine_last(&mut self) -> bool {
        let len = self.len();
        if len > 1 && self.objs[len - 2].can_combine(&self.objs[len - 1]) {
            self.objs[len - 2].count += self.objs[len - 1].count;
            self.objs.pop();
            true
        } else {
            false
        }
    }

    // Returns an iterator that enumerates each object's seL4_CPtr.
    pub fn cptr_iter(&self) -> impl Iterator<Item = seL4_CPtr> + '_ {
        self.objs
            .iter()
            .flat_map(|od| od.cptr..(od.cptr + od.retype_count()))
    }

    // Move objects to dynamically-allocated slots in the top-level
    // CNode and mutate the Object Descriptor with the new cptr's.
    // NB: there is no attempt to preserve the order of cptr's (and
    // in practice they are linearized).
    // TODO(sleffler) make generic (requires supplying slot allocator)?
    pub fn move_objects_to_toplevel(&mut self) -> seL4_Result {
        let dest_cnode = unsafe { SELF_CNODE };
        let dest_depth = seL4_WordBits as u8;
        for od in &mut self.objs {
            let dest_slot = unsafe { CANTRIP_CSPACE_SLOTS.alloc(od.retype_count()) }
                .ok_or(seL4_Error::seL4_NotEnoughMemory)?; // XXX seL4_Result not a good fit
            for offset in 0..od.retype_count() {
                unsafe {
                    // TODO(sleffler): cleanup on error?
                    seL4_CNode_Move(
                        /*dest_root=*/ dest_cnode,
                        /*dest_index=*/ dest_slot + offset,
                        /*dest_depth=*/ dest_depth,
                        /*src_root=*/ self.cnode,
                        /*src_index=*/ od.cptr + offset,
                        /*src_depth=*/ self.depth,
                    )?;
                }
            }
            od.cptr = dest_slot;
        }
        self.cnode = dest_cnode;
        self.depth = dest_depth;
        Ok(())
    }

    // Move objects from the top-level CSpace to |dest_cnode| and
    // release the top-level slots. The Object Descriptor are mutated
    // with adjusted cptr's.
    // TODO(sleffler): this does not preserve the order of the cptr's;
    //   doing so is easy but not very useful when move_object_to_toplevvel
    //   does not
    pub fn move_objects_from_toplevel(
        &mut self,
        dest_cnode: seL4_CPtr,
        dest_depth: u8,
    ) -> seL4_Result {
        let mut dest_slot = 0; // NB: assume empty container
        for od in &mut self.objs {
            let count = od.retype_count();
            for offset in 0..count {
                // XXX cleanup on error?
                unsafe {
                    seL4_CNode_Move(
                        /*dest_root=*/ dest_cnode,
                        /*dest_index=*/ dest_slot + offset,
                        /*dest_depth=*/ dest_depth,
                        /*src_root=*/ self.cnode,
                        /*src_index=*/ od.cptr + offset,
                        /*src_depth=*/ self.depth,
                    )
                }?;
            }
            unsafe { CANTRIP_CSPACE_SLOTS.free(od.cptr, count) };
            od.cptr = dest_slot;
            dest_slot += count;
        }
        self.cnode = dest_cnode;
        self.depth = dest_depth;
        Ok(())
    }
}
impl fmt::Display for ObjDescBundle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.cnode == unsafe { SELF_CNODE } {
            assert_eq!(self.depth as usize, seL4_WordBits);
            write!(f, "{{ SELF,  {:?} }}", &self.objs)
        } else if self.cnode == unsafe { MEMORY_RECV_CNODE } {
            assert_eq!(self.depth, unsafe { MEMORY_RECV_CNODE_DEPTH });
            write!(f, "{{ MEMORY_RECV, {:?} }}", &self.objs)
        } else {
            write!(
                f,
                "{{ cnode: {}, depth: {}, {:?} }}",
                self.cnode, self.depth, &self.objs
            )
        }
    }
}

#[repr(usize)]
#[derive(Debug, Default, Eq, PartialEq, FromPrimitive, IntoPrimitive)]
pub enum MemoryManagerError {
    Success = 0,
    SerializeFailed,
    DeserializeFailed,
    ObjCountInvalid, // Too many objects requested
    ObjTypeInvalid,  // Request with invalid object type
    ObjCapInvalid,   // Request with invalid cptr XXX
    CapAllocFailed,
    #[default]
    UnknownError,
    // Generic errors.
    AllocFailed,
    FreeFailed,
}
impl From<MemoryManagerError> for Result<(), MemoryManagerError> {
    fn from(err: MemoryManagerError) -> Result<(), MemoryManagerError> {
        if err == MemoryManagerError::Success {
            Ok(())
        } else {
            Err(err)
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MemoryManagerStats {
    // Current space committed to allocations.
    pub allocated_bytes: usize,

    // Current space available.
    pub free_bytes: usize,

    // Total space for user requests (independent of any alignment).
    pub total_requested_bytes: usize,

    // Space required for operation of the MemoryManager service.
    pub overhead_bytes: usize,

    // Current number of seL4 objects allocated.
    pub allocated_objs: usize,

    // Total number of seL4 objects allocated.
    pub total_requested_objs: usize,

    // Retype requests failed due to insufficient available memory.
    pub untyped_slab_too_small: usize,

    // Alloc requests failed due to lack of untyped memory.
    pub out_of_memory: usize,
}

// Hint that indicates the expected lifetime of the allocated memory
// objects. This may be used by the allocator to co-locate objects
// with a similar lifetime (especially Static).
// NB: there is limited space for Static objects, incorrect use may
//    cause the system to run out of memory
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MemoryLifetime {
    Short,  // e.g. loading a model
    Medium, // e.g. for application/process construction
    Static, // Never free'd
}

// Objects are potentially batched with caps to allocated objects returned
// in the container slots specified by the |bundle] objects.
pub trait MemoryManagerInterface {
    fn alloc(
        &mut self,
        bundle: &ObjDescBundle,
        lifetime: MemoryLifetime,
    ) -> Result<(), MemoryManagerError>;
    fn free(&mut self, bundle: &ObjDescBundle) -> Result<(), MemoryManagerError>;
    fn stats(&self) -> Result<MemoryManagerStats, MemoryManagerError>;
    fn debug(&self) -> Result<(), MemoryManagerError>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatsResponse {
    pub value: MemoryManagerStats,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MemoryManagerRequest<'a> {
    Alloc {
        bundle: Cow<'a, ObjDescBundle>,
        lifetime: MemoryLifetime,
    },
    Free(Cow<'a, ObjDescBundle>),
    Stats, // -> MemoryResponseData
    Debug,
    Capscan,
}

impl<'a> MemoryManagerRequest<'a> {
    fn get_container_cap(&self) -> Option<seL4_CPtr> {
        match self {
            Self::Alloc {
                bundle,
                lifetime: _,
            }
            | Self::Free(bundle) => Some(bundle.cnode),
            Self::Stats | Self::Debug | Self::Capscan => None,
        }
    }
}

// NB: @14b per desc this supports ~150 descriptors (depending
//   on serde overhead), the rpc buffer is actually 4K so we could
//   raise this
pub const MEMORY_REQUEST_DATA_SIZE: usize = 2048;
//pub const MEMORY_RESPONSE_DATA_SIZE: usize = 100;
//pub type MemoryResponseData = [u8; MEMORY_RESPONSE_DATA_SIZE];

fn cantrip_memory_request<T: DeserializeOwned>(
    request: &MemoryManagerRequest,
) -> Result<T, MemoryManagerError> {
    trace!(
        "cantrip_memory_request {:?} cap {:?}",
        request,
        request.get_container_cap()
    );
    let (request_slice, reply_slice) =
        rpc_shared_buffer_mut!(memory).split_at_mut(MEMORY_REQUEST_DATA_SIZE);
    let _ =
        postcard::to_slice(request, request_slice).or(Err(MemoryManagerError::SerializeFailed))?;
    match rpc_shared_send!(memory, request.get_container_cap()).into() {
        MemoryManagerError::Success => {
            let reply =
                postcard::from_bytes(reply_slice).or(Err(MemoryManagerError::DeserializeFailed))?;
            Ok(reply)
        }
        err => Err(err),
    }
}

// Allocates the objects specified in |request|. The capabilities are stored
// in |request|.cnode which is assumed to be a CNode with sufficient capacity
#[inline]
pub fn cantrip_object_alloc(request: &ObjDescBundle) -> Result<(), MemoryManagerError> {
    trace!("cantrip_object_alloc {}", request);
    cantrip_memory_request(&MemoryManagerRequest::Alloc {
        bundle: Cow::Borrowed(request),
        lifetime: MemoryLifetime::Medium,
    })
}

// Allocates the objects specified in |request| with a specified |lifetime|.
// The capabilities are stored in |request|.cnode which is assumed to be a
// CNode with sufficient capacity
#[inline]
pub fn cantrip_object_alloc_with_lifetime(
    request: &ObjDescBundle,
    lifetime: MemoryLifetime,
) -> Result<(), MemoryManagerError> {
    trace!("cantrip_object_alloc_with_lifetime {} {:?}", request, lifetime);
    cantrip_memory_request(&MemoryManagerRequest::Alloc {
        bundle: Cow::Borrowed(request),
        lifetime,
    })
}

// Allocates the objects specified in |objs|. The capabilities are moved
// to SELF_CNODE which must have sufficient space.
#[inline]
pub fn cantrip_object_alloc_in_toplevel(
    objs: Vec<ObjDesc>,
) -> Result<ObjDescBundle, MemoryManagerError> {
    // Request the objects using the dedicated MemoryManager container.
    let mut request =
        ObjDescBundle::new(unsafe { MEMORY_RECV_CNODE }, unsafe { MEMORY_RECV_CNODE_DEPTH }, objs);
    cantrip_object_alloc(&request)?;
    match request.move_objects_to_toplevel() {
        Err(_) => {
            cantrip_object_free(&request).expect("cantrip_object_alloc_in_toplevel");
            Err(MemoryManagerError::ObjCapInvalid) // TODO(sleffler): e.into
        }
        Ok(_) => Ok(request),
    }
}

// Like cantrip_object_alloc_in_toplevel but for objects that will not be
// free'd. Use this sparingly, there is a limited amount of memory available
// for Static objects.
#[inline]
pub fn cantrip_object_alloc_in_toplevel_static(
    objs: Vec<ObjDesc>,
) -> Result<ObjDescBundle, MemoryManagerError> {
    // Request the objects using the dedicated MemoryManager container.
    let mut request =
        ObjDescBundle::new(unsafe { MEMORY_RECV_CNODE }, unsafe { MEMORY_RECV_CNODE_DEPTH }, objs);
    cantrip_object_alloc_with_lifetime(&request, MemoryLifetime::Static)?;
    match request.move_objects_to_toplevel() {
        Err(_) => {
            cantrip_object_free(&request).expect("cantrip_object_alloc_in_toplevel_static");
            Err(MemoryManagerError::ObjCapInvalid) // TODO(sleffler): e.into
        }
        Ok(_) => Ok(request),
    }
}

// Allocates the objects specified in |objs|. The capabilities are stored
// in a new CNode allocated with sufficient capacity.
// Note the objects' cptr's are assumed to be consecutive and start at zero.
// Note the returned |ObjDescBundle| has the new CNode marked as the container.
#[inline]
pub fn cantrip_object_alloc_in_cnode(
    objs: Vec<ObjDesc>,
) -> Result<ObjDescBundle, MemoryManagerError> {
    fn next_log2(value: usize) -> usize {
        // NB: BITS & leading_zeros return u32
        (1 + usize::BITS - usize::leading_zeros(value)) as usize
    }
    // NB: CNode size depends on how many objects are requested.
    let cnode_depth = next_log2(objs.iter().map(|od| od.count).sum());

    // Request a top-level CNode.
    let cnode = cantrip_cnode_alloc(cnode_depth)?;

    // Now construct the request for |objs| with |cnode| as the container.
    let request = ObjDescBundle::new(cnode.objs[0].cptr, cnode_depth as u8, objs);
    match cantrip_object_alloc(&request) {
        Err(e) => {
            cantrip_object_free_toplevel(&cnode).expect("cnode free");
            Err(e)
        }
        Ok(_) => Ok(request),
    }
}

// TODO(sleffler): remove unused convience wrappers?

#[inline]
pub fn cantrip_untyped_alloc(space_bytes: usize) -> Result<ObjDescBundle, MemoryManagerError> {
    let mut objs = ObjDescBundle::new(
        unsafe { MEMORY_RECV_CNODE },
        unsafe { MEMORY_RECV_CNODE_DEPTH },
        vec![ObjDesc::new(
            seL4_UntypedObject,
            space_bytes,
            /*cptr=*/ 0,
        )],
    );
    cantrip_object_alloc(&objs)?;
    objs.move_objects_to_toplevel()
        .or(Err(MemoryManagerError::ObjCapInvalid))?;
    Ok(objs)
}

#[inline]
pub fn cantrip_tcb_alloc() -> Result<ObjDescBundle, MemoryManagerError> {
    let mut objs = ObjDescBundle::new(
        unsafe { MEMORY_RECV_CNODE },
        unsafe { MEMORY_RECV_CNODE_DEPTH },
        vec![ObjDesc::new(seL4_TCBObject, 1, /*cptr=*/ 0)],
    );
    cantrip_object_alloc(&objs)?;
    objs.move_objects_to_toplevel()
        .or(Err(MemoryManagerError::ObjCapInvalid))?;
    Ok(objs)
}

#[inline]
pub fn cantrip_endpoint_alloc() -> Result<ObjDescBundle, MemoryManagerError> {
    let mut objs = ObjDescBundle::new(
        unsafe { MEMORY_RECV_CNODE },
        unsafe { MEMORY_RECV_CNODE_DEPTH },
        vec![ObjDesc::new(seL4_EndpointObject, 1, /*cptr=*/ 0)],
    );
    cantrip_object_alloc(&objs)?;
    objs.move_objects_to_toplevel()
        .or(Err(MemoryManagerError::ObjCapInvalid))?;
    Ok(objs)
}

#[inline]
pub fn cantrip_notification_alloc() -> Result<ObjDescBundle, MemoryManagerError> {
    let mut objs = ObjDescBundle::new(
        unsafe { MEMORY_RECV_CNODE },
        unsafe { MEMORY_RECV_CNODE_DEPTH },
        vec![ObjDesc::new(seL4_NotificationObject, 1, /*cptr=*/ 0)],
    );
    cantrip_object_alloc(&objs)?;
    objs.move_objects_to_toplevel()
        .or(Err(MemoryManagerError::ObjCapInvalid))?;
    Ok(objs)
}

#[inline]
// |size_bits| is the log2 of the #slots to allocate.
pub fn cantrip_cnode_alloc(size_bits: usize) -> Result<ObjDescBundle, MemoryManagerError> {
    let mut objs = ObjDescBundle::new(
        unsafe { MEMORY_RECV_CNODE },
        unsafe { MEMORY_RECV_CNODE_DEPTH },
        vec![ObjDesc::new(
            seL4_CapTableObject,
            size_bits,
            /*cptr=*/ 0,
        )],
    );
    cantrip_object_alloc(&objs)?;
    objs.move_objects_to_toplevel()
        .or(Err(MemoryManagerError::ObjCapInvalid))?;
    Ok(objs)
}

#[cfg(feature = "CONFIG_KERNEL_MCS")]
#[inline]
pub fn cantrip_sched_context_alloc(size_bits: usize) -> Result<ObjDescBundle, MemoryManagerError> {
    let mut objs = ObjDescBundle::new(
        unsafe { MEMORY_RECV_CNODE },
        unsafe { MEMORY_RECV_CNODE_DEPTH },
        vec![ObjDesc::new(
            seL4_SchedContextObject,
            size_bits,
            /*cptr=*/ 0,
        )],
    );
    cantrip_object_alloc(&objs)?;
    objs.move_objects_to_toplevel()
        .or(Err(MemoryManagerError::ObjCapInvalid))?;
    Ok(objs)
}

#[cfg(feature = "CONFIG_KERNEL_MCS")]
#[inline]
pub fn cantrip_reply_alloc() -> Result<ObjDescBundle, MemoryManagerError> {
    let mut objs = ObjDescBundle::new(
        unsafe { MEMORY_RECV_CNODE },
        unsafe { MEMORY_RECV_CNODE_DEPTH },
        vec![ObjDesc::new(seL4_ReplyObject, 1, /*cptr=*/ 0)],
    );
    cantrip_object_alloc(&objs)?;
    objs.move_objects_to_toplevel()
        .or(Err(MemoryManagerError::ObjCapInvalid))?;
    Ok(objs)
}

// Wrapper for allocating small pages.
#[inline]
pub fn cantrip_frame_alloc(space_bytes: usize) -> Result<ObjDescBundle, MemoryManagerError> {
    fn howmany(value: usize, unit: usize) -> usize { (value + (unit - 1)) / unit }
    // NB: always allocate small pages
    let mut objs = ObjDescBundle::new(
        unsafe { MEMORY_RECV_CNODE },
        unsafe { MEMORY_RECV_CNODE_DEPTH },
        // NB: always allocate 4K pages
        vec![ObjDesc::new(
            seL4_SmallPageObject,
            howmany(space_bytes, 1 << seL4_PageBits),
            /*cptr=*/ 0,
        )],
    );
    cantrip_object_alloc(&objs)?;
    objs.move_objects_to_toplevel()
        .or(Err(MemoryManagerError::ObjCapInvalid))?;
    Ok(objs)
}

// Like cantrip_frame_alloc but also create a CNode to hold the frames.
#[inline]
pub fn cantrip_frame_alloc_in_cnode(
    space_bytes: usize,
) -> Result<ObjDescBundle, MemoryManagerError> {
    fn howmany(value: usize, unit: usize) -> usize { (value + (unit - 1)) / unit }
    // NB: always allocate small pages
    let npages = howmany(space_bytes, 1 << seL4_PageBits);
    // XXX horrible band-aid to workaround Retype "fanout" limit of 256
    // objects: split our request accordingly. This should be handled in
    // MemoryManager using the kernel config or bump the kernel limit.
    assert!(npages <= 512); // XXX 2MB
    if npages > 256 {
        cantrip_object_alloc_in_cnode(vec![
            ObjDesc::new(seL4_SmallPageObject, 256, /*cptr=*/ 0),
            ObjDesc::new(seL4_SmallPageObject, npages - 256, /*cptr=*/ 256),
        ])
    } else {
        cantrip_object_alloc_in_cnode(vec![ObjDesc::new(
            seL4_SmallPageObject,
            npages,
            /*cptr=*/ 0,
        )])
    }
}

#[inline]
pub fn cantrip_page_table_alloc() -> Result<ObjDescBundle, MemoryManagerError> {
    let mut objs = ObjDescBundle::new(
        unsafe { MEMORY_RECV_CNODE },
        unsafe { MEMORY_RECV_CNODE_DEPTH },
        vec![ObjDesc::new(seL4_PageTableObject, 1, /*cptr=*/ 0)],
    );
    cantrip_object_alloc(&objs)?;
    objs.move_objects_to_toplevel()
        .or(Err(MemoryManagerError::ObjCapInvalid))?;
    Ok(objs)
}

#[inline]
pub fn cantrip_object_free(request: &ObjDescBundle) -> Result<(), MemoryManagerError> {
    trace!("cantrip_object_free {}", request);
    cantrip_memory_request(&MemoryManagerRequest::Free(Cow::Borrowed(request)))
}

// Free |request| and then the container that holds them. The container
// is expected to be in the top-level CNode (as returned by
// cantrip_object_alloc_in_cnode).
#[inline]
pub fn cantrip_object_free_in_cnode(request: &ObjDescBundle) -> Result<(), MemoryManagerError> {
    let cnode_obj = ObjDescBundle::new(
        unsafe { SELF_CNODE },
        seL4_WordBits as u8,
        vec![ObjDesc::new(
            /*type=*/ seL4_CapTableObject,
            /*count=*/ request.depth as usize,
            /*cptr=*/ request.cnode,
        )],
    );
    cantrip_object_free(request)?;
    // No way to recover if this fails..
    cantrip_object_free_toplevel(&cnode_obj)
}

#[inline]
pub fn cantrip_object_free_toplevel(objs: &ObjDescBundle) -> Result<(), MemoryManagerError> {
    let mut objs_mut = objs.clone();
    // Move ojbects to the pre-allocated container. Note this returns
    // the toplevel slots to the slot allocator.
    objs_mut
        .move_objects_from_toplevel(unsafe { MEMORY_RECV_CNODE }, unsafe {
            MEMORY_RECV_CNODE_DEPTH
        })
        .or(Err(MemoryManagerError::ObjCapInvalid))?;
    cantrip_object_free(&objs_mut)
}

#[inline]
pub fn cantrip_memory_stats() -> Result<MemoryManagerStats, MemoryManagerError> {
    cantrip_memory_request(&MemoryManagerRequest::Stats).map(|stats: StatsResponse| stats.value)
}

#[inline]
pub fn cantrip_memory_debug() -> Result<(), MemoryManagerError> {
    cantrip_memory_request(&MemoryManagerRequest::Debug)
}

#[inline]
pub fn cantrip_memory_capscan() -> Result<(), MemoryManagerError> {
    cantrip_memory_request(&MemoryManagerRequest::Capscan)
}
