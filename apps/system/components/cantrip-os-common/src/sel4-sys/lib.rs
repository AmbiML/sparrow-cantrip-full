/* Copyright (c) 2015 The Robigalia Project Developers
 * Licensed under the Apache License, Version 2.0
 * <LICENSE-APACHE or
 * http://www.apache.org/licenses/LICENSE-2.0> or the MIT
 * license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
 * at your option. All files in the project carrying such
 * notice may not be copied, modified, or distributed except
 * according to those terms.
 */
#![no_std]
#![allow(stable_features)]
#![feature(asm)]
#![feature(thread_local)]
#![allow(bad_style, unused_parens, unused_assignments)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::uninit_assumed_init)]
#![allow(clippy::too_many_arguments)]

use core::arch::asm;
use core::mem::size_of;
#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};
use static_assertions::*;

// NB: this mimics the logic in build.rs
assert_cfg!(any(
    all(target_arch = "arm", target_pointer_width = "32"),
    all(target_arch = "aarch64"),
    all(target_arch = "riscv32"),
    all(target_arch = "riscv64"),
    all(target_arch = "x86"),
    all(target_arch = "x86_64"),
));

mod debug;
pub use debug::*;

pub use seL4_BreakpointAccess::*;
pub use seL4_BreakpointType::*;
pub use seL4_Error::*;
pub use seL4_LookupFailureType::*;
pub use seL4_ObjectType::*;

// XXX: These can't be repr(C), but it needs to "match an int" according to the comments on
// SEL4_FORCE_LONG_ENUM. There's no single type that matches in Rust, so it needs to be
// per-architecture. We use a macro to define them all in one whack, with the invoker providing
// only what the size of the enums ought to be. Each arch then invokes it.
macro_rules! error_types {
    ($int_width:ident) => {
        #[repr($int_width)]
        #[derive(Debug, Copy, Clone, PartialEq, Eq)]
        pub enum seL4_Error {
            seL4_NoError = 0,
            seL4_InvalidArgument,
            seL4_InvalidCapability,
            seL4_IllegalOperation,
            seL4_RangeError,
            seL4_AlignmentError,
            seL4_FailedLookup,
            seL4_TruncatedMessage,
            seL4_DeleteFirst,
            seL4_RevokeFirst,
            seL4_NotEnoughMemory,
            // NB: Code depends on this being the last variant
        }

        #[repr($int_width)]
        #[derive(Debug, Copy, Clone, PartialEq, Eq)]
        pub enum seL4_LookupFailureType {
            seL4_NoFailure = 0,
            seL4_InvalidRoot,
            seL4_MissingCapability,
            seL4_DepthMismatch,
            seL4_GuardMismatch,
            // XXX: Code depends on this being the last variant
        }

        #[repr($int_width)]
        #[derive(Debug, Copy, Clone, PartialEq, Eq)]
        pub enum seL4_BreakpointType {
            seL4_DataBreakpoint = 0,
            seL4_InstructionBreakpoint,
            seL4_SingleStep,
            seL4_SoftwareBreakRequest,
        }

        #[repr($int_width)]
        #[derive(Debug, Copy, Clone, PartialEq, Eq)]
        pub enum seL4_BreakpointAccess {
            seL4_BreakOnRead = 0,
            seL4_BreakOnWrite,
            seL4_BreakOnReadWrite,
        }
    };
}

// NB: potentially arch-dependent
pub type seL4_Word = usize;
pub type seL4_CPtr = usize;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
include!(concat!(env!("OUT_DIR"), "/shared_types_x86.rs"));

#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
include!(concat!(env!("OUT_DIR"), "/shared_types_riscv.rs"));

#[cfg(any(target_arch = "aarch32", target_arch = "aarch64"))]
include!(concat!(env!("OUT_DIR"), "/shared_types_arm.rs"));

#[cfg(target_arch = "x86")]
include!("arch/x86.rs");

#[cfg(target_arch = "x86_64")]
include!("arch/x86_64.rs");

#[cfg(all(target_arch = "arm", target_pointer_width = "32"))]
include!("arch/aarch32.rs");

#[cfg(target_arch = "aarch64")]
include!("arch/aarch64.rs");

#[cfg(target_arch = "riscv32")]
include!("arch/riscv32.rs");

#[cfg(target_arch = "riscv64")]
include!("arch/riscv64.rs");

#[cfg(all(target_arch = "x86"))]
include!(concat!(env!("OUT_DIR"), "/ia32_invocation.rs"));

#[cfg(all(target_arch = "x86_64"))]
include!(concat!(env!("OUT_DIR"), "/x86_64_invocation.rs"));

#[cfg(all(target_arch = "arm", target_pointer_width = "32"))]
include!(concat!(env!("OUT_DIR"), "/aarch32_invocation.rs"));

#[cfg(target_arch = "aarch64")]
include!(concat!(env!("OUT_DIR"), "/aarch64_invocation.rs"));

#[cfg(target_arch = "riscv32")]
include!(concat!(env!("OUT_DIR"), "/riscv32_invocation.rs"));

#[cfg(target_arch = "riscv64")]
include!(concat!(env!("OUT_DIR"), "/riscv64_invocation.rs"));

#[cfg(all(target_arch = "x86"))]
include!(concat!(env!("OUT_DIR"), "/ia32_syscall_stub.rs"));

#[cfg(all(target_arch = "x86_64"))]
include!(concat!(env!("OUT_DIR"), "/x86_64_syscall_stub.rs"));

#[cfg(all(target_arch = "arm", target_pointer_width = "32"))]
include!(concat!(env!("OUT_DIR"), "/aarch32_syscall_stub.rs"));

#[cfg(target_arch = "aarch64")]
include!(concat!(env!("OUT_DIR"), "/aarch64_syscall_stub.rs"));

#[cfg(target_arch = "riscv32")]
include!(concat!(env!("OUT_DIR"), "/riscv32_syscall_stub.rs"));

#[cfg(target_arch = "riscv64")]
include!(concat!(env!("OUT_DIR"), "/riscv64_syscall_stub.rs"));

#[cfg(target_pointer_width = "32")]
include!(concat!(env!("OUT_DIR"), "/types32.rs"));

#[cfg(target_pointer_width = "64")]
include!(concat!(env!("OUT_DIR"), "/types64.rs"));

include!(concat!(env!("OUT_DIR"), "/syscalls.rs"));

// Well-known types from libsel4/include/sel4/types.h

pub type seL4_CNode = seL4_CPtr;
pub type seL4_Domain = seL4_Word;
pub type seL4_DomainSet = seL4_CPtr;
pub type seL4_IRQControl = seL4_CPtr;
pub type seL4_IRQHandler = seL4_CPtr;
pub type seL4_NodeId = seL4_Word;
pub type seL4_PAddr = seL4_Word;
pub type seL4_SchedContext = seL4_CPtr;
pub type seL4_SchedControl = seL4_CPtr;
pub type seL4_TCB = seL4_CPtr;
pub type seL4_Untyped = seL4_CPtr;

// TODO(sleffler): seL4 uses seL4_Uint64 but it's not defined for us
pub type seL4_Time = u64;

// NB: capDL CDL_ObjectType is defined using this (sigh)
pub const seL4_ObjectTypeCount: isize = seL4_ObjectType::seL4_LastObjectType as isize;

pub const seL4_MsgLengthBits: usize = 7;
pub const seL4_MsgMaxLength: usize = 120;
pub const seL4_MsgExtraCapBits: usize = 2;
pub const seL4_MsgMaxExtraCaps: usize = (1usize << seL4_MsgExtraCapBits) - 1;

pub const seL4_InvalidPrio: usize = usize::MAX;
pub const seL4_MinPrio: usize = 0;
pub const seL4_MaxPrio: usize = 256 - 1; // TODO(sleffler): CONFIG_NUM_PRIORITIES

// MCS definitions
pub const seL4_MinSchedContextBits: usize = 8;
// Size of a scheduling context, excluding extra refills.
pub const seL4_CoreSchedContextBytes: usize = (10 * size_of::<seL4_Word>() + (6 * 8));
// Size of a single extra refill.
pub const seL4_RefillSizeBytes: usize = (2 * 8);

// Calculate the max extra refills a scheduling context can contain for a specific size.
//
// @param  size of the schedulding context. Must be >= seL4_MinSchedContextBits
// @return the max number of extra refills that can be passed to seL4_SchedControl_Configure for
//         this scheduling context
#[inline]
pub fn seL4_MaxExtraRefills(size: seL4_Word) -> seL4_Word {
    ((1 << size) - seL4_CoreSchedContextBytes) / seL4_RefillSizeBytes
}

// Flags to be used with seL4_SchedControl_ConfigureFlags
pub const seL4_SchedContext_NoFlag: usize = 0x0;
pub const seL4_SchedContext_Sporadic: usize = 0x1;

// Syscall stubs are generated to return seL4_Result unless they return
// an API struct with an embedded error code. The latter should be replaced
// too but for now we leave it as-is.
pub type seL4_Result = Result<(), seL4_Error>;

// NB: these traits are used by syscall stubs.
impl From<seL4_Error> for seL4_Result {
    fn from(err: seL4_Error) -> seL4_Result {
        if err == seL4_NoError {
            Ok(())
        } else {
            Err(err)
        }
    }
}
// NB: usize works for both 32- and 64-bit architectures
impl From<usize> for seL4_Error {
    fn from(val: usize) -> seL4_Error {
        // TODO(sleffler): 10 is seL4_NotEnoughMemory
        debug_assert!(val <= 10, "Invalid seL4_Error");
        unsafe { ::core::mem::transmute(val) }
    }
}

impl From<usize> for seL4_FaultTag {
    fn from(val: usize) -> seL4_FaultTag {
        debug_assert!(val <= 6, "Invalid or unknown seL4_FaultTag");
        // TODO(b/1222568): Find a more elegant way of doing this -- hardcoding
        // u32 here might not work on other platforms.
        unsafe { ::core::mem::transmute(val as u32) }
    }
}

#[repr(C)]
#[derive(Copy, Debug)]
/// Buffer used to store received IPC messages
pub struct seL4_IPCBuffer {
    /// Message tag
    ///
    /// The kernel does not initialize this.
    pub tag: seL4_MessageInfo,
    /// Message contents
    ///
    /// The kernel only initializes the bytes which were not able to fit into physical registers.
    pub msg: [seL4_Word; seL4_MsgMaxLength],
    /// Arbitrary user data.
    ///
    /// The seL4 C libraries expect this to be a pointer to the IPC buffer in the thread's VSpace.,
    /// but this doesn't really matter.
    pub userData: seL4_Word,
    /// Capabilities to transfer (if sending) or unwrapped badges
    pub caps_or_badges: [seL4_Word; seL4_MsgMaxExtraCaps],
    /// CPtr to a CNode in the thread's CSpace from which to find the receive slot
    pub receiveCNode: seL4_CPtr,
    /// CPtr to the receive slot, relative to receiveCNode
    pub receiveIndex: seL4_CPtr,
    /// Number of bits of receiveIndex to use
    pub receiveDepth: seL4_Word,
}

impl ::core::clone::Clone for seL4_IPCBuffer {
    fn clone(&self) -> Self { *self }
}

// From libsel4/include/sel4/shared_types.h; this is defined in C as an enum
// but we use pub const because the C code intentionally declares overlapping
// values which Rust rejects. Nothing (atm) uses the actual enum type so this
// should be compatible.
pub const seL4_CapFault_IP: seL4_Word = 0;
pub const seL4_CapFault_Addr: seL4_Word = 1;
pub const seL4_CapFault_InRecvPhase: seL4_Word = 2;
pub const seL4_CapFault_LookupFailureType: seL4_Word = 3;
pub const seL4_CapFault_BitsLeft: seL4_Word = 4;
pub const seL4_CapFault_DepthMismatch_BitsFound: seL4_Word = 5;
pub const seL4_CapFault_GuardMismatch_GuardFound: seL4_Word = seL4_CapFault_DepthMismatch_BitsFound;
pub const seL4_CapFault_GuardMismatch_BitsFound: seL4_Word = 6;

// Bootinfo

// Fixed cap slots for root thread.
pub const seL4_CapNull: seL4_Word = 0; // null cap
pub const seL4_CapInitThreadTCB: seL4_Word = 1; // initial thread's TCB
pub const seL4_CapInitThreadCNode: seL4_Word = 2; // initial thread's root CNode
pub const seL4_CapInitThreadVSpace: seL4_Word = 3; // initial thread's VSpace
pub const seL4_CapIRQControl: seL4_Word = 4; // global IRQ controller
pub const seL4_CapASIDControl: seL4_Word = 5; // global ASID controller
pub const seL4_CapInitThreadASIDPool: seL4_Word = 6; // initial thread's ASID pool
pub const seL4_CapIOPort: seL4_Word = 7; // global IO port (null if not supported)
pub const seL4_CapIOSpace: seL4_Word = 8; // global IO space (null if no IOMMU support)
pub const seL4_CapBootInfoFrame: seL4_Word = 9; // bootinfo frame
pub const seL4_CapInitThreadIPCBuffer: seL4_Word = 10; // initial thread's IPC buffer frame
pub const seL4_CapDomain: seL4_Word = 11; // global domain controller

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A half-open [start..end) range of slots
pub struct seL4_SlotRegion {
    /// First CNode slot position of the region
    pub start: seL4_Word,
    /// First CNode slot position after the region
    pub end: seL4_Word, /* first CNode slot position AFTER region */
}

// NB: C definition is not packed; this explicitly aligns to an
// seL4_Word using the Rust idiom.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct seL4_UntypedDesc {
    /// Physical address corresponding of the untyped object's backing memory
    pub paddr: seL4_Word,
    /// log2 size of the region of memory backing the untyped object
    pub sizeBits: u8,
    /// Whether the backing memory corresponds to some device memory
    pub isDevice: u8,
    /// Whether the untyped object was tainted by the kernel
    pub isTainted: u8,
    // Align to seL4_Word
    align: [seL4_Word; 0],
}
impl seL4_UntypedDesc {
    pub fn is_device(&self) -> bool { self.isDevice != 0 }
    pub fn is_tainted(&self) -> bool { self.isTainted != 0 }
    pub fn size_bits(&self) -> usize { self.sizeBits as usize }
}

// explicitly *not* Copy. the array at the end is tricky to handle.
// #[derive]` can't be used on a `#[repr(packed)]` struct that does not derive Copy (error E0133)

#[repr(C)]
pub struct seL4_BootInfo {
    /// Length of any additional bootinfo information
    pub extraLen: seL4_Word,
    /// ID [0..numNodes-1] of the current node (0 if uniprocessor)
    pub nodeID: seL4_NodeId,
    /// Number of seL4 nodes (1 if uniprocessor)
    pub numNodes: seL4_Word,
    /// Number of IOMMU PT levels (0 if no IOMMU support)
    pub numIOPTLevels: seL4_Word,
    /// pointer to root task's IPC buffer */
    pub ipcBuffer: *mut seL4_IPCBuffer,
    /// Empty slots (null caps)
    pub empty: seL4_SlotRegion,
    /// Frames shared between nodes
    pub sharedFrames: seL4_SlotRegion,
    /// Frame caps used for the loaded ELF image of the root task
    pub userImageFrames: seL4_SlotRegion,
    /// PD caps used for the loaded ELF image of the root task
    pub userImagePaging: seL4_SlotRegion,
    /// IOSpace caps for ARM SMMU
    pub ioSpaceCaps: seL4_SlotRegion,
    /// Caps fr anypages used to back the additional bootinfo information
    pub extraBIPages: seL4_SlotRegion,
    /// log2 size of root task's CNode
    pub initThreadCNodeSizeBits: seL4_Word,
    /// Root task's domain ID
    pub initThreadDomain: seL4_Domain,
    /// Memory reserved to the kernel.
    pub kernelReservedBytes: seL4_Word,

    #[cfg(feature = "CONFIG_KERNEL_MCS")]
    // Caps to sched_control for each node
    pub schedcontrol: seL4_SlotRegion,

    /// Untyped object caps
    pub untyped: seL4_SlotRegion,
    /// Information about each untyped cap
    ///
    /// *Note*! The actual length depends on kernel configuration that is
    /// unavailable at compile time. Use the `untyped_descs` method to get
    /// access to the descriptors.
    pub untypedList: [seL4_UntypedDesc; 1],
}
impl seL4_BootInfo {
    /// This is safe if you don't mutate the `untyped` field and corrupt its length.
    pub unsafe fn untyped_descs(&self) -> &[seL4_UntypedDesc] {
        let len = self.untyped.end - self.untyped.start;
        // sanity check that the number of untypeds doesn't extend past the end of the page
        debug_assert!(
            len <= (4096 - size_of::<seL4_BootInfo>() + size_of::<seL4_UntypedDesc>())
                / size_of::<seL4_UntypedDesc>()
        );
        core::slice::from_raw_parts(&self.untypedList[0], len)
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct seL4_BootInfoHeader {
    /// Identifier of the following chunk
    pub id: seL4_Word,
    /// Length of the chunk
    pub len: seL4_Word,
}

pub const SEL4_BOOTINFO_HEADER_PADDING: usize = 0;
pub const SEL4_BOOTINFO_HEADER_X86_VBE: usize = 1;
pub const SEL4_BOOTINFO_HEADER_X86_MBMMAP: usize = 2;
pub const SEL4_BOOTINFO_HEADER_X86_ACPI_RSDP: usize = 3;
pub const SEL4_BOOTINFO_HEADER_X86_FRAMEBUFFER: usize = 4;
pub const SEL4_BOOTINFO_HEADER_X86_TSC_FREQ: usize = 5;
pub const SEL4_BOOTINFO_HEADER_FDT: usize = 6;
pub const SEL4_BOOTINFO_HEADER_BOOTINFO: usize = 7; // Copy of rootserver's BootInfo
pub const SEL4_BOOTINFO_HEADER_NUM: usize = SEL4_BOOTINFO_HEADER_BOOTINFO + 1;
