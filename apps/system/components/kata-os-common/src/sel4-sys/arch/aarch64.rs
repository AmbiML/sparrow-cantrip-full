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

pub use seL4_ARM_PageUpperDirectoryObject as seL4_PageUpperDirectoryObject;
pub use seL4_ARM_PageGlobalDirectoryObject as seL4_PageGlobalDirectoryObject;

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
    fn from(val: u32) -> seL4_ARM_VMAttributes {
        unsafe { ::core::mem::transmute(val & 7) }
    }
}
pub const seL4_ARM_Default_VMAttributes: seL4_ARM_VMAttributes =
    seL4_ARM_VMAttributes::Default;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
            seL4_NotificationObject =>  Some(seL4_EndpointBits),
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
    fn from(type_: seL4_ObjectType) -> seL4_Word {
        type_ as seL4_Word
    }
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
pub unsafe fn seL4_GetMR(regnum: usize) -> seL4_Word {
    (*seL4_GetIPCBuffer()).msg[regnum]
}

#[inline(always)]
pub unsafe fn seL4_SetMR(regnum: usize, value: seL4_Word) {
    (*seL4_GetIPCBuffer()).msg[regnum] = value;
}

#[inline(always)]
pub unsafe fn seL4_GetUserData() -> seL4_Word {
    (*seL4_GetIPCBuffer()).userData
}

#[inline(always)]
pub unsafe fn seL4_SetUserData(data: seL4_Word) {
    (*seL4_GetIPCBuffer()).userData = data;
}

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
    ((*ipcbuffer).receiveCNode,
     (*ipcbuffer).receiveIndex,
     (*ipcbuffer).receiveDepth)
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

#[inline(always)]
pub unsafe fn seL4_Send(dest: seL4_CPtr, msgInfo: seL4_MessageInfo) {
    asm!("svc 0",
        in("x7") swinum!(SyscallId::Send),
        in("x0") dest,
        in("x1") msgInfo.words[0],
        in("x2") seL4_GetMR(0),
        in("x3") seL4_GetMR(1),
        in("x4") seL4_GetMR(2),
        in("x5") seL4_GetMR(3),
    );
}

macro_rules! opt_assign {
    ($loc:expr, $val:expr) => {
        if !$loc.is_null() {
            *$loc = $val;
        }
    };
}

#[inline(always)]
pub unsafe fn seL4_SendWithMRs(
    dest: seL4_CPtr,
    msgInfo: seL4_MessageInfo,
    mr0: *mut seL4_Word,
    mr1: *mut seL4_Word,
    mr2: *mut seL4_Word,
    mr3: *mut seL4_Word,
) {
    let mut msg0 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg1 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg2 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg3 = ::core::mem::MaybeUninit::uninit().assume_init();

    if !mr0.is_null() && msgInfo.get_length() > 0 {
        msg0 = *mr0;
    }
    if !mr1.is_null() && msgInfo.get_length() > 1 {
        msg1 = *mr1;
    }
    if !mr2.is_null() && msgInfo.get_length() > 2 {
        msg2 = *mr2;
    }
    if !mr3.is_null() && msgInfo.get_length() > 3 {
        msg3 = *mr3;
    }

    asm!("svc 0",
        in("x7") swinum!(SyscallId::Send),
        in("x0") dest,
        in("x1") msgInfo.words[0],
        in("x2") msg0,
        in("x3") msg1,
        in("x4") msg2,
        in("x5") msg3,
    );
}

#[inline(always)]
pub unsafe fn seL4_NBSend(dest: seL4_CPtr, msgInfo: seL4_MessageInfo) {
    asm!("svc 0",
        in("x7") swinum!(SyscallId::NBSend),
        in("x0") dest,
        in("x1") msgInfo.words[0],
        in("x2") seL4_GetMR(0),
        in("x3") seL4_GetMR(1),
        in("x4") seL4_GetMR(2),
        in("x5") seL4_GetMR(3),
    );
}

#[inline(always)]
pub unsafe fn seL4_NBSendWithMRs(
    dest: seL4_CPtr,
    msgInfo: seL4_MessageInfo,
    mr0: *mut seL4_Word,
    mr1: *mut seL4_Word,
    mr2: *mut seL4_Word,
    mr3: *mut seL4_Word,
) {
    let mut msg0 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg1 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg2 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg3 = ::core::mem::MaybeUninit::uninit().assume_init();

    if !mr0.is_null() && msgInfo.get_length() > 0 {
        msg0 = *mr0;
    }
    if !mr1.is_null() && msgInfo.get_length() > 1 {
        msg1 = *mr1;
    }
    if !mr2.is_null() && msgInfo.get_length() > 2 {
        msg2 = *mr2;
    }
    if !mr3.is_null() && msgInfo.get_length() > 3 {
        msg3 = *mr3;
    }

    asm!("svc 0",
        in("x7") swinum!(SyscallId::NBSend),
        in("x0") dest,
        in("x1") msgInfo.words[0],
        in("x2") msg0,
        in("x3") msg1,
        in("x4") msg2,
        in("x5") msg3,
    );
}

#[cfg(not(feature = "CONFIG_KERNEL_MCS"))]
#[inline(always)]
pub unsafe fn seL4_Reply(msgInfo: seL4_MessageInfo) {
    asm!("svc 0",
        in("x7") swinum!(SyscallId::Reply),
        in("x1") msgInfo.words[0],
        in("x2") seL4_GetMR(0),
        in("x3") seL4_GetMR(1),
        in("x4") seL4_GetMR(2),
        in("x5") seL4_GetMR(3),
    );
}

#[cfg(not(feature = "CONFIG_KERNEL_MCS"))]
#[inline(always)]
pub unsafe fn seL4_ReplyWithMRs(
    msgInfo: seL4_MessageInfo,
    mr0: *mut seL4_Word,
    mr1: *mut seL4_Word,
    mr2: *mut seL4_Word,
    mr3: *mut seL4_Word,
) {
    let mut msg0 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg1 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg2 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg3 = ::core::mem::MaybeUninit::uninit().assume_init();

    if !mr0.is_null() && msgInfo.get_length() > 0 {
        msg0 = *mr0;
    }
    if !mr1.is_null() && msgInfo.get_length() > 1 {
        msg1 = *mr1;
    }
    if !mr2.is_null() && msgInfo.get_length() > 2 {
        msg2 = *mr2;
    }
    if !mr3.is_null() && msgInfo.get_length() > 3 {
        msg3 = *mr3;
    }

    asm!("svc 0",
        in("x7") swinum!(SyscallId::Reply),
        in("x1") msgInfo.words[0],
        in("x2") msg0,
        in("x3") msg1,
        in("x4") msg2,
        in("x5") msg3,
    );
}

#[inline(always)]
pub unsafe fn seL4_Signal(dest: seL4_CPtr) {
    let info = seL4_MessageInfo::new(0, 0, 0, 0).words[0];
    asm!("svc 0",
        in("x7") swinum!(SyscallId::Send),
        in("x0") dest,
        in("x1") info,
    )
}

#[inline(always)]
pub unsafe fn seL4_Recv(mut src: seL4_CPtr, sender: *mut seL4_Word) -> seL4_MessageInfo {
    let mut info: seL4_Word;
    let mut msg0 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg1 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg2 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg3 = ::core::mem::MaybeUninit::uninit().assume_init();

    asm!("svc 0",
        in("x7") swinum!(SyscallId::Recv),
        out("x0") src,
        out("x1") info,
        out("x2") msg0,
        out("x3") msg1,
        out("x4") msg2,
        out("x5") msg3
    );

    seL4_SetMR(0, msg0);
    seL4_SetMR(1, msg1);
    seL4_SetMR(2, msg2);
    seL4_SetMR(3, msg3);

    opt_assign!(sender, src);

    seL4_MessageInfo { words: [info] }
}

#[inline(always)]
pub unsafe fn seL4_NBRecv(mut src: seL4_CPtr, sender: *mut seL4_Word) -> seL4_MessageInfo {
    let info: seL4_Word;
    let mut msg0 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg1 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg2 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg3 = ::core::mem::MaybeUninit::uninit().assume_init();

    asm!("svc 0",
        in("x7") swinum!(SyscallId::NBRecv),
        inout("x0") src,
        out("x1") info,
        out("x2") msg0,
        out("x3") msg1,
        out("x4") msg2,
        out("x5") msg3
    );

    seL4_SetMR(0, msg0);
    seL4_SetMR(1, msg1);
    seL4_SetMR(2, msg2);
    seL4_SetMR(3, msg3);

    opt_assign!(sender, src);

    seL4_MessageInfo { words: [info] }
}

#[inline(always)]
pub unsafe fn seL4_Call(dest: seL4_CPtr, msgInfo: seL4_MessageInfo) -> seL4_MessageInfo {
    let mut info: seL4_Word;
    let mut msg0 = seL4_GetMR(0);
    let mut msg1 = seL4_GetMR(1);
    let mut msg2 = seL4_GetMR(2);
    let mut msg3 = seL4_GetMR(3);

    asm!("svc 0",
        in("x7") swinum!(SyscallId::Call),
        in("x0") dest,
        inout("x1") msgInfo.words[0] => info,
        inout("x2") msg0,
        inout("x3") msg1,
        inout("x4") msg2,
        inout("x5") msg3
    );

    seL4_SetMR(0, msg0);
    seL4_SetMR(1, msg1);
    seL4_SetMR(2, msg2);
    seL4_SetMR(3, msg3);

    seL4_MessageInfo { words: [info] }
}

#[inline(always)]
pub unsafe fn seL4_CallWithMRs(
    dest: seL4_CPtr,
    msgInfo: seL4_MessageInfo,
    mr0: *mut seL4_Word,
    mr1: *mut seL4_Word,
    mr2: *mut seL4_Word,
    mr3: *mut seL4_Word,
) -> seL4_MessageInfo {
    let mut info: seL4_Word;
    let mut msg0 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg1 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg2 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg3 = ::core::mem::MaybeUninit::uninit().assume_init();

    if !mr0.is_null() && msgInfo.get_length() > 0 {
        msg0 = *mr0;
    }
    if !mr1.is_null() && msgInfo.get_length() > 1 {
        msg1 = *mr1;
    }
    if !mr2.is_null() && msgInfo.get_length() > 2 {
        msg2 = *mr2;
    }
    if !mr3.is_null() && msgInfo.get_length() > 3 {
        msg3 = *mr3;
    }

    asm!("svc 0",
        in("x7") swinum!(SyscallId::Call),
        in("x0") dest,
        inout("x1") msgInfo.words[0] => info,
        inout("x2") msg0,
        inout("x3") msg1,
        inout("x4") msg2,
        inout("x5") msg3
    );

    opt_assign!(mr0, msg0);
    opt_assign!(mr1, msg1);
    opt_assign!(mr2, msg2);
    opt_assign!(mr3, msg3);

    seL4_MessageInfo { words: [info] }
}

#[inline(always)]
pub unsafe fn seL4_ReplyRecv(
    mut src: seL4_CPtr,
    msgInfo: seL4_MessageInfo,
    sender: *mut seL4_Word,
) -> seL4_MessageInfo {
    let mut info: seL4_Word;
    let mut msg0 = seL4_GetMR(0);
    let mut msg1 = seL4_GetMR(1);
    let mut msg2 = seL4_GetMR(2);
    let mut msg3 = seL4_GetMR(3);

    asm!("svc 0",
        in("x7") swinum!(SyscallId::ReplyRecv),
        inout("x0") src,
        inout("x1") msgInfo.words[0] => info,
        inout("x2") msg0,
        inout("x3") msg1,
        inout("x4") msg2,
        inout("x5") msg3
    );

    seL4_SetMR(0, msg0);
    seL4_SetMR(1, msg1);
    seL4_SetMR(2, msg2);
    seL4_SetMR(3, msg3);

    opt_assign!(sender, src);

    seL4_MessageInfo { words: [info] }
}

#[inline(always)]
pub unsafe fn seL4_ReplyRecvWithMRs(
    src: seL4_CPtr,
    msgInfo: seL4_MessageInfo,
    sender: *mut seL4_Word,
    mr0: *mut seL4_Word,
    mr1: *mut seL4_Word,
    mr2: *mut seL4_Word,
    mr3: *mut seL4_Word,
) -> seL4_MessageInfo {
    let mut info: seL4_Word;
    let mut badge: seL4_Word;
    let mut msg0 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg1 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg2 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg3 = ::core::mem::MaybeUninit::uninit().assume_init();

    if !mr0.is_null() && msgInfo.get_length() > 0 {
        msg0 = *mr0;
    }
    if !mr1.is_null() && msgInfo.get_length() > 1 {
        msg1 = *mr1;
    }
    if !mr2.is_null() && msgInfo.get_length() > 2 {
        msg2 = *mr2;
    }
    if !mr3.is_null() && msgInfo.get_length() > 3 {
        msg3 = *mr3;
    }
    asm!("svc 0",
        in("x7") swinum!(SyscallId::ReplyRecv),
        inout("x0") src => badge,
        inout("x1") msgInfo.words[0] => info,
        inout("x2") msg0,
        inout("x3") msg1,
        inout("x4") msg2,
        inout("x5") msg3
    );

    opt_assign!(mr0, msg0);
    opt_assign!(mr1, msg1);
    opt_assign!(mr2, msg2);
    opt_assign!(mr3, msg3);

    opt_assign!(sender, badge);

    seL4_MessageInfo { words: [info] }
}

#[cfg(feature = "CONFIG_KERNEL_MCS")]
#[inline(always)]
pub unsafe fn seL4_NBSendRecv(
    dest: seL4_CPtr,
    msgInfo: seL4_MessageInfo,
    src: seL4_CPtr,
    sender: *mut seL4_Word,
    reply: seL4_CPtr,
) -> seL4_MessageInfo {
    let mut info: seL4_Word;
    let mut badge: seL4_Word;
    let mut msg0 = seL4_GetMR(0);
    let mut msg1 = seL4_GetMR(1);
    let mut msg2 = seL4_GetMR(2);
    let mut msg3 = seL4_GetMR(3);

    asm!("svc 0",
        in("x7") swinum!(SyscallId::NBSendRecv),
        inout("x0") src => badge,
        inout("x1") msgInfo.words[0] => info,
        inout("x2") msg0,
        inout("x3") msg1,
        inout("x4") msg2,
        inout("x5") msg3,
        in("x6") reply,
        in("x8") dest,
    );

    /* Write the message back out to memory. */
    seL4_SetMR(0, msg0);
    seL4_SetMR(1, msg1);
    seL4_SetMR(2, msg2);
    seL4_SetMR(3, msg3);

    opt_assign!(sender, badge);

    seL4_MessageInfo { words: [info] }
}

#[cfg(feature = "CONFIG_KERNEL_MCS")]
#[inline(always)]
pub unsafe fn seL4_NBSendRecvWithMRs(
    dest: seL4_CPtr,
    msgInfo: seL4_MessageInfo,
    src: seL4_CPtr,
    sender: *mut seL4_Word,
    mr0: *mut seL4_Word,
    mr1: *mut seL4_Word,
    mr2: *mut seL4_Word,
    mr3: *mut seL4_Word,
    reply: seL4_CPtr,
) -> seL4_MessageInfo {
    let mut info: seL4_Word;
    let mut badge: seL4_Word;
    let mut msg0 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg1 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg2 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg3 = ::core::mem::MaybeUninit::uninit().assume_init();
    if !mr0.is_null() && msgInfo.get_length() > 0 {
        msg0 = *mr0;
    }
    if !mr1.is_null() && msgInfo.get_length() > 1 {
        msg1 = *mr1;
    }
    if !mr2.is_null() && msgInfo.get_length() > 2 {
        msg2 = *mr2;
    }
    if !mr3.is_null() && msgInfo.get_length() > 3 {
        msg3 = *mr3;
    }

    asm!("svc 0",
        in("x7") swinum!(SyscallId::NBSendRecv),
        inout("x0") src => badge,
        inout("x1") msgInfo.words[0] => info,
        inout("x2") msg0,
        inout("x3") msg1,
        inout("x4") msg2,
        inout("x5") msg3,
        in("x6") reply,
        in("x8") dest,
    );

    opt_assign!(mr0, msg0);
    opt_assign!(mr1, msg1);
    opt_assign!(mr2, msg2);
    opt_assign!(mr3, msg3);

    opt_assign!(sender, badge);

    seL4_MessageInfo { words: [info] }
}

#[cfg(feature = "CONFIG_KERNEL_MCS")]
#[inline(always)]
pub unsafe fn seL4_NBSendWait(
    dest: seL4_CPtr,
    msgInfo: seL4_MessageInfo,
    src: seL4_CPtr,
    sender: *mut seL4_Word,
) -> seL4_MessageInfo {
    let mut info: seL4_Word;
    let mut badge: seL4_Word;
    let mut msg0 = seL4_GetMR(0);
    let mut msg1 = seL4_GetMR(1);
    let mut msg2 = seL4_GetMR(2);
    let mut msg3 = seL4_GetMR(3);

    asm!("svc 0",
        in("x7") swinum!(SyscallId::NBSendWait),
        inout("x0") src => badge,
        inout("x1") msgInfo.words[0] => info,
        inout("x2") msg0,
        inout("x3") msg1,
        inout("x4") msg2,
        inout("x5") msg3,
        in("x6") dest,
        in("x8") 0,  // XXX dest
    );

    /* Write the message back out to memory. */
    seL4_SetMR(0, msg0);
    seL4_SetMR(1, msg1);
    seL4_SetMR(2, msg2);
    seL4_SetMR(3, msg3);

    opt_assign!(sender, badge);

    seL4_MessageInfo { words: [info] }
}

#[cfg(feature = "CONFIG_KERNEL_MCS")]
#[inline(always)]
pub unsafe fn seL4_NBSendWaitWithMRs(
    dest: seL4_CPtr,
    msgInfo: seL4_MessageInfo,
    src: seL4_CPtr,
    sender: *mut seL4_Word,
    mr0: *mut seL4_Word,
    mr1: *mut seL4_Word,
    mr2: *mut seL4_Word,
    mr3: *mut seL4_Word,
) -> seL4_MessageInfo {
    let mut info: seL4_Word;
    let mut badge: seL4_Word;
    let mut msg0 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg1 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg2 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg3 = ::core::mem::MaybeUninit::uninit().assume_init();
    if !mr0.is_null() && msgInfo.get_length() > 0 {
        msg0 = *mr0;
    }
    if !mr1.is_null() && msgInfo.get_length() > 1 {
        msg1 = *mr1;
    }
    if !mr2.is_null() && msgInfo.get_length() > 2 {
        msg2 = *mr2;
    }
    if !mr3.is_null() && msgInfo.get_length() > 3 {
        msg3 = *mr3;
    }

    asm!("svc 0",
        in("x7") swinum!(SyscallId::NBSendRecv),
        inout("x0") src => badge,
        inout("x1") msgInfo.words[0] => info,
        inout("x2") msg0,
        inout("x3") msg1,
        inout("x4") msg2,
        inout("x5") msg3,
        in("x6") dest,
        in("x8") 0,  // XXX does this dtrt
    );

    opt_assign!(mr0, msg0);
    opt_assign!(mr1, msg1);
    opt_assign!(mr2, msg2);
    opt_assign!(mr3, msg3);

    opt_assign!(sender, badge);

    seL4_MessageInfo { words: [info] }
}

#[inline(always)]
pub unsafe fn seL4_Yield() {
    asm!("svc 0",
        in("x7") swinum!(SyscallId::Yield),
        options(nomem, nostack),
    );
}

#[cfg(feature = "CONFIG_KERNEL_MCS")]
#[inline(always)]
pub unsafe fn seL4_Wait(src: seL4_CPtr, sender: *mut seL4_Word) -> seL4_MessageInfo {
    let mut info: seL4_Word;
    let mut badge: seL4_Word;
    let mut msg0 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg1 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg2 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg3 = ::core::mem::MaybeUninit::uninit().assume_init();

    asm!("svc 0",
        in("x7") swinum!(SyscallId::Wait),
        inout("x0") src => badge,
        out("x1") info,
        inout("x2") msg0,
        inout("x3") msg1,
        inout("x4") msg2,
        inout("x5") msg3,
    );

    seL4_SetMR(0, msg0);
    seL4_SetMR(1, msg1);
    seL4_SetMR(2, msg2);
    seL4_SetMR(3, msg3);

    opt_assign!(sender, badge);

    seL4_MessageInfo { words: [info] }
}

#[cfg(feature = "CONFIG_KERNEL_MCS")]
#[inline(always)]
pub unsafe fn seL4_WaitWithMRs(
    src: seL4_CPtr,
    sender: *mut seL4_Word,
    mr0: *mut seL4_Word,
    mr1: *mut seL4_Word,
    mr2: *mut seL4_Word,
    mr3: *mut seL4_Word,
) -> seL4_MessageInfo {
    let mut info: seL4_Word;
    let mut badge: seL4_Word;
    let mut msg0 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg1 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg2 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg3 = ::core::mem::MaybeUninit::uninit().assume_init();

    asm!("svc 0",
        in("x7") swinum!(SyscallId::Wait),
        inout("x0") src => badge,
        out("x1") info,
        inout("x2") msg0,
        inout("x3") msg1,
        inout("x4") msg2,
        inout("x5") msg3,
    );

    opt_assign!(mr0, msg0);
    opt_assign!(mr1, msg1);
    opt_assign!(mr2, msg2);
    opt_assign!(mr3, msg3);

    opt_assign!(sender, badge);

    seL4_MessageInfo { words: [info] }
}

#[cfg(feature = "CONFIG_KERNEL_MCS")]
#[inline(always)]
pub unsafe fn seL4_NBWait(src: seL4_CPtr, sender: *mut seL4_Word) -> seL4_MessageInfo {
    let mut info: seL4_Word;
    let mut badge: seL4_Word;
    let mut msg0 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg1 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg2 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg3 = ::core::mem::MaybeUninit::uninit().assume_init();

    asm!("svc 0",
        in("x7") swinum!(SyscallId::NBWait),
        inout("x0") src => badge,
        out("x1") info,
        inout("x2") msg0,
        inout("x3") msg1,
        inout("x4") msg2,
        inout("x5") msg3,
    );

    seL4_SetMR(0, msg0);
    seL4_SetMR(1, msg1);
    seL4_SetMR(2, msg2);
    seL4_SetMR(3, msg3);

    opt_assign!(sender, badge);

    seL4_MessageInfo { words: [info] }
}

#[cfg(feature = "CONFIG_PRINTING")]
#[inline(always)]
pub unsafe fn seL4_DebugPutChar(c: u8) {
    asm!("svc 0",
        in("x7") swinum!(SyscallId::DebugPutChar),
        in("x0") c,
        options(nostack),
    );
}

#[cfg(feature = "CONFIG_PRINTING")]
#[inline(always)]
pub unsafe fn seL4_DebugDumpScheduler() {
    asm!("svc 0",
        in("x7") swinum!(SyscallId::DebugDumpScheduler),
        options(nomem, nostack),
    );
}

#[cfg(feature = "CONFIG_DEBUG_BUILD")]
#[inline(always)]
pub unsafe fn seL4_DebugHalt() {
    asm!("svc 0",
        in("x7") swinum!(SyscallId::DebugHalt),
        options(nomem, nostack),
    );
}

#[cfg(feature = "CONFIG_DEBUG_BUILD")]
#[inline(always)]
pub unsafe fn seL4_DebugSnapshot() {
    asm!("svc 0",
        in("x7") swinum!(SyscallId::DebugSnapshot),
        options(nomem, nostack),
    );
}

#[cfg(feature = "CONFIG_DEBUG_BUILD")]
#[inline(always)]
pub unsafe fn seL4_DebugCapIdentify(mut cap: seL4_CPtr) -> u32 {
    asm!("svc 0",
        in("x7") swinum!(SyscallId::DebugCapIdentify),
        inout("x0") cap,
        options(nomem, nostack),
    );
    cap as _
}

#[cfg(feature = "CONFIG_PRINTING")]
#[inline(always)]
pub unsafe fn seL4_DebugDumpCNode(mut cap: seL4_CPtr) {
    asm!("svc 0",
        in("x7") swinum!(SyscallId::DebugDumpCNode),
        inout("x0") cap,
        options(nomem, nostack),
    );
}

// Note: name MUST be NUL-terminated.
#[cfg(feature = "CONFIG_DEBUG_BUILD")]
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

#[cfg(feature = "CONFIG_DANGEROUS_CODE_INJECTION")]
#[inline(always)]
pub unsafe fn seL4_DebugRun(userfn: extern "C" fn(*mut u8), userarg: *mut u8) {
    let userfnptr = userfn as *mut ();
    asm!("svc 0"
      in("x7") swinum!(SyscallId::DebugRun),
      in("x0") userfnptr,
      in("x1") userarg,
    );
}

#[cfg(feature = "CONFIG_ENABLE_BENCHMARKS")]
#[inline(always)]
pub unsafe fn seL4_BenchmarkResetLog() {
    asm!("svc 0",
        in("x7") swinum!(SyscallId::BenchmarkResetLog),
        options(nomem, nostack),
    );
}

#[cfg(feature = "CONFIG_ENABLE_BENCHMARKS")]
#[inline(always)]
pub unsafe fn seL4_BenchmarkFinalizeLog() {
    asm!("svc 0",
        in("x7") swinum!(SyscallId::BenchmarkFinalizeLog),
        options(nomem, nostack),
    );
}
