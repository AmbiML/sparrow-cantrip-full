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
assert_cfg!(target_arch = "riscv32");

use cfg_if::cfg_if;

pub const seL4_WordBits: usize = 32;
pub const seL4_WordSizeBits: usize = 2;
pub const seL4_PageBits: usize = 12;
pub const seL4_SlotBits: usize = 4;
pub const seL4_TCBBits: usize = 9;
pub const seL4_ReplyBits: usize = 4;
pub const seL4_EndpointBits: usize = 4;
pub const seL4_PageTableEntryBits: usize = 2;
pub const seL4_PageTableIndexBits: usize = 10;
pub const seL4_PageDirIndexBits: usize = seL4_PageTableIndexBits;
pub const seL4_LargePageBits: usize = 22;
pub const seL4_PageTableBits: usize = 12;
pub const seL4_VSpaceBits: usize = seL4_PageTableBits;
pub const seL4_NumASIDPoolBits: usize = 5;
pub const seL4_ASIDPoolIndexBits: usize = 4;
pub const seL4_ASIDPoolBits: usize = 12;

#[cfg(feature = "CONFIG_KERNEL_MCS")]
pub const seL4_NotificationBits: usize = 5;
#[cfg(not(feature = "CONFIG_KERNEL_MCS"))]
pub const seL4_NotificationBits: usize = 4;

pub const seL4_MinUntypedBits: usize = 4;
pub const seL4_MaxUntypedBits: usize = 29;

pub type seL4_RISCV_Page = seL4_CPtr;
pub type seL4_RISCV_PageTable = seL4_CPtr;
pub type seL4_RISCV_ASIDControl = seL4_CPtr;
pub type seL4_RISCV_ASIDPool = seL4_CPtr;

#[cfg(feature = "arch_generic")]
include!("riscv_generic.rs");

error_types!(u32);

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct seL4_UserContext {
    pub pc: seL4_Word,
    pub ra: seL4_Word,
    pub sp: seL4_Word,
    pub gp: seL4_Word,
    pub s0: seL4_Word,
    pub s1: seL4_Word,
    pub s2: seL4_Word,
    pub s3: seL4_Word,
    pub s4: seL4_Word,
    pub s5: seL4_Word,
    pub s6: seL4_Word,
    pub s7: seL4_Word,
    pub s8: seL4_Word,
    pub s9: seL4_Word,
    pub s10: seL4_Word,
    pub s11: seL4_Word,

    pub a0: seL4_Word,
    pub a1: seL4_Word,
    pub a2: seL4_Word,
    pub a3: seL4_Word,
    pub a4: seL4_Word,
    pub a5: seL4_Word,
    pub a6: seL4_Word,
    pub a7: seL4_Word,

    pub t0: seL4_Word,
    pub t1: seL4_Word,
    pub t2: seL4_Word,
    pub t3: seL4_Word,
    pub t4: seL4_Word,
    pub t5: seL4_Word,
    pub t6: seL4_Word,

    pub tp: seL4_Word,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum seL4_RISCV_VMAttributes {
    Default = 0,
    ExecuteNever = 0x1,
}
impl From<u32> for seL4_RISCV_VMAttributes {
    fn from(val: u32) -> seL4_RISCV_VMAttributes { unsafe { ::core::mem::transmute(val & 1) } }
}
pub const seL4_RISCV_Default_VMAttributes: seL4_RISCV_VMAttributes =
    seL4_RISCV_VMAttributes::Default;

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

    seL4_RISCV_4K_Page,
    seL4_RISCV_Mega_Page,
    seL4_RISCV_PageTableObject,

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

            seL4_RISCV_4K_Page => Some(seL4_PageBits),
            // NB: Arch_get_ObjectSize uses seL4_PageBits which is the
            //   same for both 32- and 64-bit systems
            seL4_RISCV_PageTableObject => Some(seL4_PageTableBits),
            seL4_RISCV_Mega_Page => Some(seL4_LargePageBits),
            // seL4_RISCV_Giga_Page => Some(seL4_HugePageBits),
            // seL4_RISCV_Tera_Page => Some(seL4_TeraPageBits),
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
// NB: these correspond to riscv_sys_* in libsel4's syscalls.h files

// Fills all message registers. Discards everything returned by the kerrnel.
// Used for 1-way sends like seL4_Send.
macro_rules! asm_send {
    ($syscall:expr, $dest:expr, $info:expr, $mr0:expr, $mr1:expr, $mr2:expr, $mr3:expr) => {
        asm!("ecall",
            in("a7") swinum!($syscall),
            inout("a0") $dest => _,
            inout("a1") $info => _,
            inout("a2") $mr0 => _,
            inout("a3") $mr1 => _,
            inout("a4") $mr2 => _,
            inout("a5") $mr3 => _,
        )
    };
    ($syscall:expr, $dest:expr, $info:expr => $info_recv:expr, $mr0:expr, $mr1:expr, $mr2:expr, $mr3:expr) => {
        asm!("ecall",
            in("a7") swinum!($syscall),
            inout("a0") $dest => _,
            inout("a1") $info => $info_recv,
            inout("a2") $mr0 => _,
            inout("a3") $mr1 => _,
            inout("a4") $mr2 => _,
            inout("a5") $mr3 => _,
        )
    };
}

// Fills no message registers. Discards everything returned by the kernel.
// Used for 1-way sends that contain no data, like seL4_Notify.
macro_rules! asm_send_no_mrs {
    ($syscall:expr, $dest:expr, $info:expr) => {
        asm!("ecall",
            in("a7") swinum!($syscall),
            inout("a0") $dest => _,
            inout("a1") $info => _,
        )
    };
}

// Fills only the syscall number. Indicates nothing in memory
// is clobbered. Used for calls like seL4_Yield.
macro_rules! asm_no_args {
    ($syscall:expr) => {
        asm!("ecall",
            in("a7") swinum!($syscall),
            options(nomem, nostack),
        )
    };
}

include!("syscall_common.rs");

cfg_if! {
    if #[cfg(feature = "CONFIG_KERNEL_MCS")] {
        include!("riscv32_mcs.rs");
        include!("syscall_mcs.rs");
    } else {
        include!("riscv32_no_mcs.rs");
        include!("syscall_no_mcs.rs");
    }
}

// TODO(sleffler): move to syscall_common.rs

cfg_if! {
    if #[cfg(feature = "CONFIG_PRINTING")] {
        #[inline(always)]
        pub unsafe fn seL4_DebugPutChar(c: u8) {
            asm!("ecall",
                in("a7") swinum!(SyscallId::DebugPutChar),
                in("a0") c,
                options(nostack),
            );
        }

        #[inline(always)]
        pub unsafe fn seL4_DebugDumpScheduler() {
            asm!("ecall",
                in("a7") swinum!(SyscallId::DebugDumpScheduler),
                options(nomem, nostack),
            );
        }

        #[inline(always)]
        pub unsafe fn seL4_DebugDumpCNode(mut cap: seL4_CPtr) {
            asm!("ecall",
                in("a7") swinum!(SyscallId::DebugDumpCNode),
                inout("a0") cap,
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
            asm!("ecall",
                in("a7") swinum!(SyscallId::DebugCapIdentify),
                inout("a0") cap,
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
            asm!("ecall",
                in("a7") swinum!(SyscallId::DebugNameThread),
                in("a0") tcb,
            );
        }
    } // CONFIG_DEBUG_BUILD
}

#[cfg(feature = "CONFIG_DANGEROUS_CODE_INJECTION")]
#[inline(always)]
pub unsafe fn seL4_DebugRun(userfn: extern "C" fn(*mut u8), userarg: *mut u8) {
    let userfnptr = userfn as *mut ();
    asm!("ecall",
        in("a7") swinum!(SyscallId::DebugRun),
        in("a0") userfnptr,
        in("a1") userarg,
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

        // TODO(sleffler): seL4_BenchmarkSetLogBuffer
        // TODO(sleffler): seL4_BenchmarkNullSyscall
        // TODO(sleffler): seL4_BenchmarkFlushCaches
        // TODO(sleffler): seL4_BenchmarkFlushL1Caches
    } // CONFIG_ENABLE_BENCHMARKS
}

#[cfg(feature = "CONFIG_SET_TLS_BASE_SELF")]
pub unsafe fn seL4_SetTLSBase(tls_base: seL4_Word) {
    let info: seL4_Word = 0; // XXX does this dtrt?
    asm!("ecall",
        in("a7") swinum!(SyscallId::SetTLSBase),
        in("a0") tls_base,
        in("a1") info,
    );
}
