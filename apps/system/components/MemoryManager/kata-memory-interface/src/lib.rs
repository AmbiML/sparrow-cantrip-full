//! Cantrip OS memory management support

#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::vec;
use alloc::vec::Vec;
use cantrip_os_common::sel4_sys;
use log::trace;
use postcard;
use sel4_sys::seL4_CNode_Move;
use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_ObjectType::*;
use sel4_sys::seL4_ObjectType;
use sel4_sys::seL4_PageBits;
use sel4_sys::seL4_SetCap;
use sel4_sys::seL4_WordBits;
use serde::{Deserialize, Serialize};

// NB: @14b per desc this supports ~150 descriptors (depending
//   on serde overhead), the rpc buffer is actually 4K so we could
//   raise this
pub const RAW_OBJ_DESC_DATA_SIZE: usize = 2048;
pub type RawObjDescData = [u8; RAW_OBJ_DESC_DATA_SIZE];

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
// TODO(sleffler): allocate associated resources like endpoint #'s?
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct ObjDesc {
    // Requested object type or type of object being released.
    pub type_: seL4_ObjectType,

    // Count of consecutive objects with the same type or, for CNode
    // objects the log2 number of slots to use in sizing the object,
    // or for untyped objects the log2 size in bytes, or for scheduler
    // context objects the size in bits. See seL4_ObjectType::size_bits().
    count: usize,  // XXX oversized (except for untyped use)

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
    pub fn new(
        type_: seL4_ObjectType,
        count: usize,
        cptr: seL4_CPtr,
    ) -> Self {
        ObjDesc { type_, count, cptr }
    }
    // Parameters for seL4_Untyped_Retype call.
    pub fn retype_size_bits(&self) -> Option<usize> {
        match self.type_ {
            seL4_UntypedObject => Some(self.count),  // Log2 memory size
            seL4_CapTableObject => Some(self.count), // Log2 number of slots
            seL4_SchedContextObject => Some(self.count), // Log2 context size
            _ => self.type_.size_bits(),
        }
    }
    pub fn retype_count(&self) -> usize {
        match self.type_ {
            // NB: we don't support creating multiple instances of the same
            //   size; the caller must supply multiple object descriptors.
              seL4_UntypedObject
            | seL4_CapTableObject
            | seL4_SchedContextObject => 1,
            _ => self.count,
        }
    }

    // Memory occupied by objects. Used for bookkeeping and statistics.
    pub fn size_bytes(&self) -> Option<usize> {
        match self.type_ {
            seL4_UntypedObject => Some(1 << self.count),
            seL4_CapTableObject =>
                self.type_.size_bits().map(|x| self.count * (1 << x)),
            seL4_SchedContextObject => Some(1 << self.count),
            _ => self.type_.size_bits().map(|x| 1 << x),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObjDescBundle {
    pub cnode: seL4_CPtr,
    pub depth: u8,
    pub objs: Vec<ObjDesc>,
}
impl ObjDescBundle {
    pub fn new(
        cnode: seL4_CPtr,
        depth: u8,
        objs: Vec<ObjDesc>,
    ) -> Self {
        ObjDescBundle { cnode, depth, objs }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum MemoryError {
    ObjCountInvalid = 0,   // Too many objects requested
    ObjTypeInvalid,        // Request with invalid object type
    ObjCapInvalid,         // Request with invalid cptr XXX
    UnknownMemoryError,
    // Generic errors.
    AllocFailed,
    FreeFailed,
}

pub const RAW_MEMORY_STATS_DATA_SIZE: usize = 100;
pub type RawMemoryStatsData = [u8; RAW_MEMORY_STATS_DATA_SIZE];

#[repr(C)]
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

// Objects are potentially batched with caps to allocated objects returned
// in the container slots specified by the |bundle] objects.
pub trait MemoryManagerInterface {
    fn alloc(&mut self, bundle: &ObjDescBundle) -> Result<(), MemoryError>;
    fn free(&mut self, bundle: &ObjDescBundle) -> Result<(), MemoryError>;
    fn stats(&self) -> Result<MemoryManagerStats, MemoryError>;
}

// Public version of MemoryError presented over rpc interface.
// This is needed because the enum is exported to C users and needs to
// be unique from other enum's.
// TODO(sleffler): switch to single generic error space ala absl::StatusCode
#[repr(C)]
#[derive(Debug, Eq, PartialEq)]
pub enum MemoryManagerError {
    MmeSuccess = 0,
    MmeObjCountInvalid,
    MmeObjTypeInvalid,
    MmeObjCapInvalid,
    MmeSerializeFailed,
    MmeDeserializeFailed,
    MmeUnknownError,
    // Generic errors.
    MmeAllocFailed,
    MmeFreeFailed,
}
impl From<MemoryError> for MemoryManagerError {
    fn from(err: MemoryError) -> MemoryManagerError {
        match err {
            MemoryError::ObjCountInvalid => MemoryManagerError::MmeObjCountInvalid,
            MemoryError::ObjTypeInvalid => MemoryManagerError::MmeObjTypeInvalid,
            MemoryError::ObjCapInvalid => MemoryManagerError::MmeObjCapInvalid,
            MemoryError::AllocFailed => MemoryManagerError::MmeAllocFailed,
            MemoryError::FreeFailed => MemoryManagerError::MmeFreeFailed,
            _ => MemoryManagerError::MmeUnknownError,
        }
    }
}
impl From<Result<(), MemoryError>> for MemoryManagerError {
    fn from(result: Result<(), MemoryError>) -> MemoryManagerError {
        result.map_or_else(
            |e| MemoryManagerError::from(e),
            |_v| MemoryManagerError::MmeSuccess,
        )
    }
}
impl From<MemoryManagerError> for Result<(), MemoryManagerError> {
    fn from(err: MemoryManagerError) -> Result<(), MemoryManagerError> {
        if err == MemoryManagerError::MmeSuccess {
            Ok(())
        } else {
            Err(err)
        }
    }
}

// Client wrappers.

extern "C" {
    // Each CAmkES-generated CNode has a writable self-reference to itself in
    // the slot SELF_CNODE. This is used to pass CNode containers of dynamically
    // allocated objects between clients & the MemoryManager.
    static SELF_CNODE: seL4_CPtr;

    // Each CAmkES-component has a CNode setup at a well-known slot. We use that
    // CNode to pass capabilities with alloc & free requests.
    static RECV_CNODE: seL4_CPtr;
    static RECV_CNODE_DEPTH: u8;
}

pub fn cantrip_object_alloc(
    request: &ObjDescBundle,
) -> Result<(), MemoryManagerError> {
    extern "C" {
        // NB: this assumes the MemoryManager component is named "memory".
        fn memory_alloc(c_request_len: u32, c_request_data: *const u8)
            -> MemoryManagerError;
    }
    trace!("cantrip_object_alloc {:?}", request); // XXX
    let raw_data = &mut [0u8; RAW_OBJ_DESC_DATA_SIZE];
    postcard::to_slice(&request, &mut raw_data[..])
        .map_err(|_| MemoryManagerError::MmeSerializeFailed)?;
    unsafe {
        // Attach our CNode for returning objects; the CAmkES template
        // forces extraCaps=1 when constructing the MessageInfo struct
        // used by the seL4_Call inside memory_alloc.
        seL4_SetCap(0, request.cnode);

        memory_alloc(raw_data.len() as u32, raw_data.as_ptr()).into()
    }
}

// TODO(sleffler): is anyone going to use these convience wrappers given
//   the objects are returned inside a CNode (so it's way more convenient
//   to allocate 'em all togerher and then move the CNode to a TCB)?

// XXX need real allocator
static mut EMPTY_SLOT: seL4_CPtr = 30;  // Next empty slot in debug_console_cnode XXX

impl ObjDescBundle {
    // Move objects from |src_cnode| to our top-levbel CSpace and mutate
    // the Object Descriptor with adjusted cptr's.
    // TODO(sleffler) make generic?
    fn move_objects_to_toplevel(
        &mut self,
        dest_cnode: seL4_CPtr,
        dest_depth: u8,
    ) -> Result<(), MemoryManagerError> {
        unsafe {
            let mut dest_slot = EMPTY_SLOT;
            for od in &mut self.objs {
                for offset in 0..od.retype_count() {
                    // XXX cleanup on error?
                    seL4_CNode_Move(
                        /*root=*/ dest_cnode,
                        /*dest_index=*/ dest_slot,
                        /*dest_depth=*/ dest_depth,
                        /*src_root=*/ self.cnode,
                        /*src_index=*/ od.cptr + offset,
                        /*src_depth=*/ self.depth,
                    ).map_err(|_| MemoryManagerError::MmeObjCapInvalid)?;
                    dest_slot += 1;
                }
                od.cptr = dest_slot - od.retype_count();
            }
            EMPTY_SLOT = dest_slot;
        }
        self.cnode = dest_cnode;
        self.depth = dest_depth;
        Ok(())
    }
}

#[inline]
#[allow(dead_code)]
pub fn cantrip_untyped_alloc(space_bytes: usize)
    -> Result<ObjDescBundle, MemoryManagerError>
{
    let mut objs = ObjDescBundle::new(
        unsafe { RECV_CNODE },
        unsafe { RECV_CNODE_DEPTH },
        vec![ ObjDesc::new(seL4_UntypedObject, space_bytes, /*cptr=*/ 0) ],
    );
    cantrip_object_alloc(&objs)?;
    objs.move_objects_to_toplevel(unsafe { SELF_CNODE }, seL4_WordBits as u8)?;
    Ok(objs)
}

#[inline]
#[allow(dead_code)]
pub fn cantrip_tcb_alloc() -> Result<ObjDescBundle, MemoryManagerError> {
    let mut objs = ObjDescBundle::new(
        unsafe { RECV_CNODE },
        unsafe { RECV_CNODE_DEPTH },
        vec![ ObjDesc::new(seL4_TCBObject, 1, /*cptr=*/ 0) ],
    );
    cantrip_object_alloc(&objs)?;
    objs.move_objects_to_toplevel(unsafe { SELF_CNODE }, seL4_WordBits as u8)?;
    Ok(objs)
}

#[inline]
#[allow(dead_code)]
pub fn cantrip_endpoint_alloc() -> Result<ObjDescBundle, MemoryManagerError> {
    let mut objs = ObjDescBundle::new(
        unsafe { RECV_CNODE },
        unsafe { RECV_CNODE_DEPTH },
        vec![ ObjDesc::new(seL4_EndpointObject, 1, /*cptr=*/ 0) ],
    );
    cantrip_object_alloc(&objs)?;
    objs.move_objects_to_toplevel(unsafe { SELF_CNODE }, seL4_WordBits as u8)?;
    Ok(objs)
}

#[inline]
#[allow(dead_code)]
pub fn cantrip_notification_alloc() -> Result<ObjDescBundle, MemoryManagerError> {
    let mut objs = ObjDescBundle::new(
        unsafe { RECV_CNODE },
        unsafe { RECV_CNODE_DEPTH },
        vec![ ObjDesc::new(seL4_NotificationObject, 1, /*cptr=*/ 0) ],
    );
    cantrip_object_alloc(&objs)?;
    objs.move_objects_to_toplevel(unsafe { SELF_CNODE }, seL4_WordBits as u8)?;
    Ok(objs)
}

#[inline]
#[allow(dead_code)]
// |size_bits| is the log2 of the #slots to allocate.
pub fn cantrip_cnode_alloc(size_bits: usize) -> Result<ObjDescBundle, MemoryManagerError>
{
    let mut objs = ObjDescBundle::new(
        unsafe { RECV_CNODE },
        unsafe { RECV_CNODE_DEPTH },
        vec![ ObjDesc::new(seL4_CapTableObject, size_bits, /*cptr=*/ 0 )],
    );
    cantrip_object_alloc(&objs)?;
    objs.move_objects_to_toplevel(unsafe { SELF_CNODE }, seL4_WordBits as u8)?;
    Ok(objs)
}

#[cfg(feature = "CONFIG_KERNEL_MCS")]
#[inline]
#[allow(dead_code)]
pub fn cantrip_sched_context_alloc(size_bits: usize) -> Result<ObjDescBundle, MemoryManagerError>
{
    let mut objs = ObjDescBundle::new(
        unsafe { RECV_CNODE },
        unsafe { RECV_CNODE_DEPTH },
        vec![ ObjDesc::new(seL4_SchedContextObject, size_bits, /*cptr=*/ 0) ],
    );
    cantrip_object_alloc(&objs)?;
    objs.move_objects_to_toplevel(unsafe { SELF_CNODE }, seL4_WordBits as u8)?;
    Ok(objs)
}

#[cfg(feature = "CONFIG_KERNEL_MCS")]
#[inline]
#[allow(dead_code)]
pub fn cantrip_reply_alloc() -> Result<ObjDescBundle, MemoryManagerError>
{
    let mut objs = ObjDescBundle::new(
        unsafe { RECV_CNODE },
        unsafe { RECV_CNODE_DEPTH },
        vec![ ObjDesc::new(seL4_ReplyObject, 1, /*cptr=*/ 0) ],
    );
    cantrip_object_alloc(&objs)?;
    objs.move_objects_to_toplevel(unsafe { SELF_CNODE }, seL4_WordBits as u8)?;
    Ok(objs)
}

#[inline]
#[allow(dead_code)]
// Wrapper for allocating 4K pages.
pub fn cantrip_frame_alloc(space_bytes: usize) -> Result<ObjDescBundle, MemoryManagerError>
{
    fn howmany(value: usize, unit: usize) -> usize {
        (value + (unit - 1)) / unit
    }
    // NB: always allocate 4K pages
    let mut objs = ObjDescBundle::new(
        unsafe { RECV_CNODE },
        unsafe { RECV_CNODE_DEPTH },
        vec![ ObjDesc::new(
            seL4_RISCV_4K_Page,
            howmany(space_bytes, 1 << seL4_PageBits),
            /*cptr=*/ 0,
        )],
    );
    cantrip_object_alloc(&objs)?;
    objs.move_objects_to_toplevel(unsafe { SELF_CNODE }, seL4_WordBits as u8)?;
    Ok(objs)
}

#[inline]
#[allow(dead_code)]
pub fn cantrip_page_table_alloc() -> Result<ObjDescBundle, MemoryManagerError> {
    let mut objs = ObjDescBundle::new(
        unsafe { RECV_CNODE },
        unsafe { RECV_CNODE_DEPTH },
        vec![ ObjDesc::new(seL4_RISCV_PageTableObject, 1, /*cptr=*/ 0) ],
    );
    cantrip_object_alloc(&objs)?;
    objs.move_objects_to_toplevel(unsafe { SELF_CNODE }, seL4_WordBits as u8)?;
    Ok(objs)
}

// TODO(sleffler): other objects, esp. vm stuff

pub fn cantrip_object_free(
    request: &ObjDescBundle,
) -> Result<(), MemoryManagerError> {
    extern "C" {
        // NB: this assumes the MemoryManager component is named "memory".
        fn memory_free(c_data_len: u32, c_data: *const u8) -> MemoryManagerError;
    }
    trace!("cantrip_object_free {:?}", request); // XXX
    let raw_data = &mut [0u8; RAW_OBJ_DESC_DATA_SIZE];
    postcard::to_slice(request, &mut raw_data[..])
        .map_err(|_| MemoryManagerError::MmeSerializeFailed)?;
    unsafe {
        // Attach our CNode for returning objects; the CAmkES template
        // forces extraCaps=1 when constructing the MessageInfo struct
        // used in the seL4_Call.
        seL4_SetCap(0, request.cnode);

        memory_free(raw_data.len() as u32, raw_data.as_ptr()).into()
    }
}

impl ObjDescBundle {
    // Move objects from our top-level CSpace to |dest_cnode| and return
    // mutate Object Descriptor with adjusted cptr's.
    fn move_objects_from_toplevel(
        &mut self,
        dest_cnode: seL4_CPtr,
        dest_depth: u8,
    ) -> Result<(), MemoryManagerError> {
        unsafe {
            let mut dest_slot = 0;
            for od in &mut self.objs {
                for offset in 0..od.retype_count() {
                    // XXX cleanup on error?
                    seL4_CNode_Move(
                        /*root=*/ dest_cnode,
                        /*dest_index=*/ dest_slot,
                        /*dest_depth=*/ dest_depth,
                        /*src_root=*/ self.cnode,
                        /*src_index=*/ od.cptr + offset,
                        /*src_depth=*/ self.depth,
                    ).map_err(|_| MemoryManagerError::MmeObjCapInvalid)?;
                    dest_slot += 1;
                }
                // XXX assumes od.cptr's are sequential
                od.cptr = dest_slot - od.retype_count();
                EMPTY_SLOT -= od.retype_count(); // XXX assumes no intervening alloc
            }
        }
        self.cnode = dest_cnode;
        self.depth = dest_depth;
        Ok(())
    }
}

#[allow(dead_code)]
pub fn cantrip_object_free_toplevel(objs: &ObjDescBundle) -> Result<(), MemoryManagerError> {
    let mut objs_mut = objs.clone();
    objs_mut.move_objects_from_toplevel(
        unsafe { RECV_CNODE },
        unsafe { RECV_CNODE_DEPTH }
    )?;
    cantrip_object_free(&objs_mut)
}

#[allow(dead_code)]
pub fn cantrip_memory_stats() -> Result<MemoryManagerStats, MemoryManagerError> {
    extern "C" {
        // NB: this assumes the MemoryManager component is named "memory".
        fn memory_stats(c_data: *mut RawMemoryStatsData) -> MemoryManagerError;
    }
    let raw_data = &mut [0u8; RAW_MEMORY_STATS_DATA_SIZE];
    match unsafe { memory_stats(raw_data as *mut _) }.into() {
        MemoryManagerError::MmeSuccess => {
            let stats = postcard::from_bytes::<MemoryManagerStats>(raw_data)
                .map_err(|_| MemoryManagerError::MmeDeserializeFailed)?;
            Ok(stats)
        }
        status => Err(status),
    }
}
