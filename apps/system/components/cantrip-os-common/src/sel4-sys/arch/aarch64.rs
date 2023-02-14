/*
 * Copyright 2015, Killian Coddington
 * Copyright 2014, NICTA
 *
 * This software may be distributed and modified according to the terms of
 * the BSD 2-Clause license. Note that NO WARRANTY is provided.
 * See "LICENSE_BSD2.txt" for details.
 *
 * @TAG(NICTA_BSD)
 */

use static_assertions::assert_cfg;
assert_cfg!(target_arch = "aarch64");

use cfg_if::cfg_if;

pub const seL4_WordBits: usize = 64;
pub const seL4_PageBits: usize = 12;
pub const seL4_SlotBits: usize = 5;

pub const seL4_ASIDPoolBits: usize = 12;
pub const seL4_EndpointBits: usize = 4;
pub const seL4_IOPageTableBits: usize = 12;
pub const seL4_HugePageBits: usize = 30;
pub const seL4_LargePageBits: usize = 21;
pub const seL4_PageDirBits: usize = 12;
pub const seL4_PageDirIndexBits: usize = 9;
pub const seL4_PageTableBits: usize = 12;
pub const seL4_PageTableIndexBits: usize = 9;
pub const seL4_ReplyBits: usize = 5;
pub const seL4_TCBBits: usize = 11;

cfg_if! {
    if #[cfg(feature = "CONFIG_KERNEL_MCS")] {
        pub const seL4_NotificationBits: usize = 6;
    } else {
        pub const seL4_NotificationBits: usize = 5;
    }
}

cfg_if! {
    if #[cfg(all(feature = "CONFIG_ARM_HYPERVISOR_SUPPORT", feature = "CONFIG_ARM_PA_SIZE_BITS_40"))] {
        pub const seL4_PUDIndexBits: usize = 10;
        pub const seL4_PUDBits: usize = 13;
        pub const seL4_PGDIndexBits: usize = 0;
        pub const seL4_PGDBits: usize = 0;
    } else {
        pub const seL4_PUDIndexBits: usize = 9;
        pub const seL4_PUDBits: usize = 12;
        pub const seL4_PGDIndexBits: usize = 9;
        pub const seL4_PGDBits: usize = 13;
    }
}

pub const seL4_Frame_Args: usize = 4;
pub const seL4_Frame_MRs: usize = 7;
pub const seL4_Frame_HasNPC: usize = 0;

pub type seL4_ARM_ASIDControl = seL4_CPtr;
pub type seL4_ARM_ASIDPool = seL4_CPtr;
pub type seL4_ARM_PageDirectory = seL4_CPtr;
pub type seL4_ARM_Page = seL4_CPtr;
pub type seL4_ARM_PageTable = seL4_CPtr;
pub type seL4_ARM_PageUpperDirectory = seL4_CPtr;
pub type seL4_ARM_VSpace = seL4_CPtr;

#[cfg(feature = "arch_generic")]
include!("arm_generic.rs");

pub use seL4_ARM_PageGlobalDirectoryObject as seL4_PageGlobalDirectoryObject;
pub use seL4_ARM_PageUpperDirectoryObject as seL4_PageUpperDirectoryObject;

error_types!(u64);

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct seL4_UserContext {
    pub pc: seL4_Word,
    pub sp: seL4_Word,
    pub spsr: seL4_Word,
    pub x0: seL4_Word,
    pub x1: seL4_Word,
    pub x2: seL4_Word,
    pub x3: seL4_Word,
    pub x4: seL4_Word,
    pub x5: seL4_Word,
    pub x6: seL4_Word,
    pub x7: seL4_Word,
    pub x8: seL4_Word,
    pub x16: seL4_Word,
    pub x17: seL4_Word,
    pub x18: seL4_Word,
    pub x29: seL4_Word,
    pub x30: seL4_Word,
    pub x9: seL4_Word,
    pub x10: seL4_Word,
    pub x11: seL4_Word,
    pub x12: seL4_Word,
    pub x13: seL4_Word,
    pub x14: seL4_Word,
    pub x15: seL4_Word,
    pub x19: seL4_Word,
    pub x20: seL4_Word,
    pub x21: seL4_Word,
    pub x22: seL4_Word,
    pub x23: seL4_Word,
    pub x24: seL4_Word,
    pub x25: seL4_Word,
    pub x26: seL4_Word,
    pub x27: seL4_Word,
    pub x28: seL4_Word,
    pub tpidr_el0: seL4_Word,
    pub tpidrro_el0: seL4_Word,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum seL4_ARM_VMAttributes {
    Default = 0,
    PageCacheable = 1,
    ParityEnabled = 2,
    ExecuteNever = 4,
}
impl From<u32> for seL4_ARM_VMAttributes {
    fn from(val: u32) -> seL4_ARM_VMAttributes { unsafe { ::core::mem::transmute(val & 7) } }
}
pub const seL4_ARM_Default_VMAttributes: seL4_ARM_VMAttributes = seL4_ARM_VMAttributes::Default;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum seL4_ObjectType {
    seL4_UntypedObject = 0,
    seL4_TCBObject,
    seL4_EndpointObject,
    seL4_NotificationObject,
    seL4_CapTableObject,

    #[cfg(feature = "CONFIG_KERNEL_MCS")]
    seL4_SchedContextObject,
    #[cfg(feature = "CONFIG_KERNEL_MCS")]
    seL4_ReplyObject,

    seL4_ARM_HugePageObject,
    seL4_ARM_PageUpperDirectoryObject,
    seL4_ARM_PageGlobalDirectoryObject,

    seL4_ARM_SmallPageObject,
    seL4_ARM_LargePageObject,
    seL4_ARM_PageTableObject,
    seL4_ARM_PageDirectoryObject,

    #[cfg(feature = "CONFIG_ARM_HYPERVISOR_SUPPORT")]
    seL4_ARM_VCPUObject,

    #[cfg(feature = "CONFIG_TK1_SMMU")]
    seL4_ARM_IOPageTableObject,

    seL4_LastObjectType,
}
impl seL4_ObjectType {
    // Returns the log2 size of fixed-size objects; typically for use
    // with seL4_Retype_Untyped. seL4_UntypedObject has no fixed-size,
    // callers must specify a size. seL4_CapTableObject has a per-slot
    // fixed-size that callers must scale by the #slots.
    // seL4_SchedContextObject size must be in the range
    // [seL4_MinSchedContextBits..seL4_MaxSchedContextBits].
    pub fn size_bits(&self) -> Option<usize> {
        match self {
            seL4_TCBObject => Some(seL4_TCBBits),
            seL4_EndpointObject => Some(seL4_EndpointBits),
            seL4_NotificationObject => Some(seL4_EndpointBits),
            #[cfg(feature = "CONFIG_KERNEL_MCS")]
            seL4_ReplyObject => Some(seL4_ReplyBits),
            #[cfg(feature = "CONFIG_KERNEL_MCS")]
            seL4_SchedContextObject => Some(seL4_MinSchedContextBits), // XXX maybe None?
            // NB: caller must scale by #slots
            seL4_CapTableObject => Some(seL4_SlotBits),

            seL4_ARM_HugePageObject => Some(seL4_HugePageBits),
            seL4_ARM_PageUpperDirectoryObject => Some(seL4_PUDBits),
            seL4_ARM_PageGlobalDirectoryObject => Some(seL4_PGDBits),

            seL4_ARM_SmallPageObject => Some(seL4_PageBits),
            seL4_ARM_LargePageObject => Some(seL4_LargePageBits),
            seL4_ARM_PageTableObject => Some(seL4_PageTableBits),
            seL4_ARM_PageDirectoryObject => Some(seL4_PageDirBits),

            _ => None,
        }
    }
}
impl From<seL4_ObjectType> for seL4_Word {
    fn from(type_: seL4_ObjectType) -> seL4_Word { type_ as seL4_Word }
}

#[inline(always)]
pub unsafe fn seL4_GetIPCBuffer() -> *mut seL4_IPCBuffer {
    // Use magic external symbol setup by runtime once TLS is primed
    enum c_void {}
    extern "C" {
        #[thread_local]
        static __sel4_ipc_buffer: *const c_void;
    }
    __sel4_ipc_buffer as *mut seL4_IPCBuffer
}

#[inline(always)]
pub unsafe fn seL4_GetMR(regnum: usize) -> seL4_Word { (*seL4_GetIPCBuffer()).msg[regnum] }

#[inline(always)]
pub unsafe fn seL4_SetMR(regnum: usize, value: seL4_Word) {
    (*seL4_GetIPCBuffer()).msg[regnum] = value;
}

#[inline(always)]
pub unsafe fn seL4_GetUserData() -> seL4_Word { (*seL4_GetIPCBuffer()).userData }

#[inline(always)]
pub unsafe fn seL4_SetUserData(data: seL4_Word) { (*seL4_GetIPCBuffer()).userData = data; }

#[inline(always)]
pub unsafe fn seL4_GetBadge(index: usize) -> seL4_Word {
    (*seL4_GetIPCBuffer()).caps_or_badges[index]
}

#[inline(always)]
pub unsafe fn seL4_GetCap(index: usize) -> seL4_CPtr {
    (*seL4_GetIPCBuffer()).caps_or_badges[index] as seL4_CPtr
}

#[inline(always)]
pub unsafe fn seL4_SetCap(index: usize, cptr: seL4_CPtr) {
    (*seL4_GetIPCBuffer()).caps_or_badges[index] = cptr as seL4_Word;
}

#[inline(always)]
pub unsafe fn seL4_GetCapReceivePath() -> (seL4_CPtr, seL4_CPtr, seL4_CPtr) {
    let ipcbuffer = seL4_GetIPCBuffer();
    (
        (*ipcbuffer).receiveCNode,
        (*ipcbuffer).receiveIndex,
        (*ipcbuffer).receiveDepth,
    )
}

#[inline(always)]
pub unsafe fn seL4_SetCapReceivePath(
    receiveCNode: seL4_CPtr,
    receiveIndex: seL4_CPtr,
    receiveDepth: seL4_Word,
) {
    let ipcbuffer = seL4_GetIPCBuffer();
    (*ipcbuffer).receiveCNode = receiveCNode;
    (*ipcbuffer).receiveIndex = receiveIndex;
    (*ipcbuffer).receiveDepth = receiveDepth;
}

macro_rules! swinum {
    ($val:expr) => {
        $val as seL4_Word
    };
}

macro_rules! opt_assign {
    ($loc:expr, $val:expr) => {
        if !$loc.is_null() {
            *$loc = $val;
        }
    };
}

// Syscall asm idioms. MCS-dependent asm wrappers are defined in
// the _mcs.rs & _no_mcs.rs files included below.
// NB: these correspond to arm_sys_* in libsel4's syscalls.h files

// Fills all message registers. Discards everything returned by the kerrnel.
// Used for 1-way sends like seL4_Send.
macro_rules! asm_send {
    ($syscall:expr, $dest:expr, $info:expr, $mr0:expr, $mr1:expr, $mr2:expr, $mr3:expr) => {
        asm!("svc 0",
            in("x7") swinum!($syscall),
            inout("x0") $dest => _,
            inout("x1") $info => _,
            inout("x2") $mr0 => _,
            inout("x3") $mr1 => _,
            inout("x4") $mr2 => _,
            inout("x5") $mr3 => _,
        )
    };
    ($syscall:expr, $dest:expr, $info:expr => $info_recv:expr, $mr0:expr, $mr1:expr, $mr2:expr, $mr3:expr) => {
        asm!("svc 0",
            in("x7") swinum!($syscall),
            inout("x0") $dest => _,
            inout("x1") $info => $info_recv,
            inout("x2") $mr0 => _,
            inout("x3") $mr1 => _,
            inout("x4") $mr2 => _,
            inout("x5") $mr3 => _,
        )
    };
}

// Fills no message registers. Discards everything returned by the kernel.
// Used for 1-way sends that contain no data, like seL4_Notify.
macro_rules! asm_send_no_mrs {
    ($syscall:expr, $dest:expr, $info:expr) => {
        asm!("svc 0",
            in("x7") swinum!($syscall),
            inout("x0") $dest => _,
            inout("x1") $info => _,
        )
    };
}

// Fills only the syscall number. Indicates nothing in memory
// is clobbered. Used for calls like seL4_Yield.
macro_rules! asm_no_args {
    ($syscall:expr) => {
        asm!("svc 0",
            in("x7") swinum!($syscall),
            options(nomem, nostack),
        )
    };
}

include!("syscall_common.rs");

cfg_if! {
    if #[cfg(feature = "CONFIG_KERNEL_MCS")] {
        include!("aarch64_mcs.rs");
        include!("syscall_mcs.rs");
    } else {
        include!("aarch64_no_mcs.rs");
        include!("syscall_no_mcs.rs");
    }
}

// TODO(sleffler): move to syscall_common.rs

cfg_if! {
    if #[cfg(feature = "CONFIG_PRINTING")] {
        #[inline(always)]
        pub unsafe fn seL4_DebugPutChar(c: u8) {
            asm!("svc 0",
                in("x7") swinum!(SyscallId::DebugPutChar),
                in("x0") c,
                options(nostack),
            );
        }

        #[inline(always)]
        pub unsafe fn seL4_DebugDumpScheduler() {
            asm_no_args!(SyscallId::DebugDumpScheduler);
        }

        #[inline(always)]
        pub unsafe fn seL4_DebugDumpCNode(mut cap: seL4_CPtr) {
            asm!("svc 0",
                in("x7") swinum!(SyscallId::DebugDumpCNode),
                inout("x0") cap,
                options(nomem, nostack),
            );
        }
    } // CONFIG_PRINTING
}

cfg_if! {
    if #[cfg(feature = "CONFIG_DEBUG_BUILD")] {
        #[inline(always)]
        pub unsafe fn seL4_DebugHalt() {
            asm_no_args!(SyscallId::DebugHalt);
        }

        #[inline(always)]
        pub unsafe fn seL4_DebugSnapshot() {
            asm_no_args!(SyscallId::DebugSnapshot);
        }

        #[inline(always)]
        pub unsafe fn seL4_DebugCapIdentify(mut cap: seL4_CPtr) -> u32 {
            asm!("svc 0",
                in("x7") swinum!(SyscallId::DebugCapIdentify),
                inout("x0") cap,
                options(nomem, nostack),
            );
            cap as _
        }

        // Note: name MUST be NUL-terminated.
        #[inline(always)]
        pub unsafe fn seL4_DebugNameThread(tcb: seL4_CPtr, name: &[u8]) {
            core::ptr::copy_nonoverlapping(
                name.as_ptr() as *mut u8,
                (&mut (*seL4_GetIPCBuffer()).msg).as_mut_ptr() as *mut u8,
                name.len(),
            );
            asm!("svc 0",
                in("x7") swinum!(SyscallId::DebugNameThread),
                in("x0") tcb,
            );
        }
    } // CONFIG_DEBUG_BUILD
}

#[cfg(feature = "CONFIG_DANGEROUS_CODE_INJECTION")]
#[inline(always)]
pub unsafe fn seL4_DebugRun(userfn: extern "C" fn(*mut u8), userarg: *mut u8) {
    let userfnptr = userfn as *mut ();
    asm!("svc 0"
        in("x7") swinum!(SyscallId::DebugRun),
        inout("x0") userfnptr => _,
        inout("x1") userarg => _,
    );
}

cfg_if! {
    if #[cfg(feature = "CONFIG_ENABLE_BENCHMARKS")] {
        #[inline(always)]
        pub unsafe fn seL4_BenchmarkResetLog() {
            asm_no_args!(SyscallId::BenchmarkResetLog);
        }

        #[inline(always)]
        pub unsafe fn seL4_BenchmarkFinalizeLog() {
            asm_no_args!(SyscallId::BenchmarkFinalizeLog);
        }
    } // CONFIG_ENABLE_BENCHMARKS
}
