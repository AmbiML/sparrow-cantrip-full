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
assert_cfg!(all(target_arch = "arm", target_pointer_width = "32"));

use cfg_if::cfg_if;

pub const seL4_WordBits: usize = 32;
pub const seL4_PageBits: usize = 12;
pub const seL4_SlotBits: usize = 4;

pub const seL4_ASIDPoolBits: usize = 12;
pub const seL4_EndpointBits: usize = 4;
pub const seL4_IOPageTableBits: usize = 12;
pub const seL4_LargePageBits: usize = 16;
pub const seL4_PageDirBits: usize = 14;
pub const seL4_ReplyBits: usize = 4;

#[cfg(all(
    feature = "CONFIG_HAVE_FPU",
    any(
        all(
            feature = "CONFIG_ARM_HYPERVISOR_SUPPORT",
            feature = "CONFIG_ARM_HYP_ENABLE_VCPU_CP14_SAVE_AND_RESTORE"
        ),
        feature = "CONFIG_HARDWARE_DEBUG_API"
    )
))]
pub const seL4_TCBBits: usize = 11;
#[cfg(any(
    feature = "CONFIG_HAVE_FPU",
    all(
        feature = "CONFIG_ARM_HYPERVISOR_SUPPORT",
        feature = "CONFIG_ARM_HYP_ENABLE_VCPU_CP14_SAVE_AND_RESTORE"
    ),
    feature = "CONFIG_HARDWARE_DEBUG_API"
))]
pub const seL4_TCBBits: usize = 10;
#[cfg(not(any(
    feature = "CONFIG_HAVE_FPU",
    all(
        feature = "CONFIG_ARM_HYPERVISOR_SUPPORT",
        feature = "CONFIG_ARM_HYP_ENABLE_VCPU_CP14_SAVE_AND_RESTORE"
    ),
    feature = "CONFIG_HARDWARE_DEBUG_API"
)))]
pub const seL4_TCBBits: usize = 9;

cfg_if! {
    if #[cfg(feature = "CONFIG_KERNEL_MCS")] {
        pub const seL4_NotificationBits: usize = 5;
        pub const seL4_ReplyBits: usize = 4;
    } else {
        pub const seL4_NotificationBits: usize = 4;
    }
}

cfg_if! {
    if #[cfg(feature = "CONFIG_ARM_HYPERVISOR_SUPPORT")] {
        pub const seL4_PageTableBits: usize = 12;
        pub const seL4_PageTableEntryBits: usize = 3;
        pub const seL4_PageTableIndexBits: usize = 9;
        pub const seL4_SectionBits: usize = 21;
        pub const seL4_SuperSectionBits: usize = 25;
        pub const seL4_PageDirEntryBits: usize = 3;
        pub const seL4_PageDirIndexBits: usize = 11;
    } else {
        pub const seL4_PageTableBits: usize = 10;
        pub const seL4_PageTableEntryBits: usize = 2;
        pub const seL4_PageTableIndexBits: usize = 8;
        pub const seL4_SectionBits: usize = 20;
        pub const seL4_SuperSectionBits: usize = 24;
        pub const seL4_PageDirEntryBits: usize = 2;
        pub const seL4_PageDirIndexBits: usize = 12;
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

#[cfg(feature = "arch_generic")]
include!("arm_generic.rs");

error_types!(u32);

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct seL4_UserContext {
    pub pc: seL4_Word,
    pub sp: seL4_Word,
    pub cpsr: seL4_Word,
    pub r0: seL4_Word,
    pub r1: seL4_Word,
    pub r8: seL4_Word,
    pub r9: seL4_Word,
    pub r10: seL4_Word,
    pub r11: seL4_Word,
    pub r12: seL4_Word,
    pub r2: seL4_Word,
    pub r3: seL4_Word,
    pub r4: seL4_Word,
    pub r5: seL4_Word,
    pub r6: seL4_Word,
    pub r7: seL4_Word,
    pub r14: seL4_Word,
    pub tpidrurw: seL4_Word,
    pub tpidruro: seL4_Word,
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

    seL4_ARM_SmallPageObject,
    seL4_ARM_LargePageObject,
    seL4_ARM_SectionObject,
    seL4_ARM_SuperSectionObject,
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
            seL4_NotificationObject => Some(seL4_NotificationBits),
            #[cfg(feature = "CONFIG_KERNEL_MCS")]
            seL4_ReplyObject => Some(seL4_ReplyBits),
            #[cfg(feature = "CONFIG_KERNEL_MCS")]
            seL4_SchedContextObject => Some(seL4_MinSchedContextBits), // XXX maybe None?
            // NB: caller must scale by #slots
            seL4_CapTableObject => Some(seL4_SlotBits),

            seL4_ARM_SmallPageObject => Some(seL4_PageBits),
            seL4_ARM_LargePageObject => Some(seL4_LargePageBits),
            seL4_ARM_SectionObject => Some(seL4_SectionBits),
            seL4_ARM_SuperSectionObject => Some(seL4_SuperSectionBits),
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
        asm!("swi 0",
            in("r7") swinum!($syscall),
            inout("r0") $dest => _,
            inout("r1") $info => _,
            inout("r2") $mr0 => _,
            inout("r3") $mr1 => _,
            inout("r4") $mr2 => _,
            inout("r5") $mr3 => _,
        )
    };
    ($syscall:expr, $dest:expr, $info:expr => $info_recv:expr, $mr0:expr, $mr1:expr, $mr2:expr, $mr3:expr) => {
        asm!("swi 0",
            in("r7") swinum!($syscall),
            inout("r0") $dest => _,
            inout("r1") $info => $info_recv,
            inout("r2") $mr0 => _,
            inout("r3") $mr1 => _,
            inout("r4") $mr2 => _,
            inout("r5") $mr3 => _,
        )
    };
}

// Fills no message registers. Discards everything returned by the kernel.
// Used for 1-way sends that contain no data, like seL4_Notify.
macro_rules! asm_send_no_mrs {
    ($syscall:expr, $dest:expr, $info:expr) => {
        asm!("swi 0",
            in("r7") swinum!($syscall),
            inout("r0") $dest => _,
            inout("r1") $info => _,
        )
    };
}

// Fills only the syscall number. Indicates nothing in memory
// is clobbered. Used for calls like seL4_Yield.
macro_rules! asm_no_args {
    ($syscall:expr) => {
        asm!("swi 0",
            in("r7") swinum!($syscall),
            options(nomem, nostack),
        )
    };
}

include!("syscall_common.rs");

cfg_if! {
    if #[cfg(feature = "CONFIG_KERNEL_MCS")] {
        include!("aarch32_mcs.rs");
        include!("syscall_mcs.rs");
    } else {
        include!("aarch32_no_mcs.rs");
        include!("syscall_no_mcs.rs");
    }
}

// TODO(sleffler): move to syscall_common.rs

cfg_if! {
    if #[cfg(feature = "CONFIG_PRINTING")] {
        #[inline(always)]
        pub unsafe fn seL4_DebugPutChar(c: u8) {
            asm!("swi 0",
                in("r7") swinum!(SyscallId::DebugPutChar),
                in("r0") c,
                options(nostack),
            );
        }

        #[inline(always)]
        pub unsafe fn seL4_DebugDumpScheduler() {
            asm_no_args!(SyscallId::DebugDumpScheduler);
        }

        #[inline(always)]
        pub unsafe fn seL4_DebugDumpCNode(mut cap: seL4_CPtr) {
            asm!("swi 0",
                in("r7") swinum!(SyscallId::DebugDumpCNode),
                inout("r0") cap,
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
            asm!("swi 0",
                in("r7") swinum!(SyscallId::DebugCapIdentify),
                inout("r0") cap,
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
            asm!("swi 0",
                in("r7") swinum!(SyscallId::DebugNameThread),
                in("r0") tcb,
            );
        }
    } // CONFIG_DEBUG_BUILD
}

#[cfg(feature = "CONFIG_DANGEROUS_CODE_INJECTION")]
#[inline(always)]
pub unsafe fn seL4_DebugRun(userfn: extern "C" fn(*mut u8), userarg: *mut u8) {
    let userfnptr = userfn as *mut ();
    asm!("swi 0"
      in("r7") swinum!(SyscallId::DebugRun),
      in("r0") userfnptr,
      in("r1") userarg,
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
