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

pub const seL4_WordBits: usize = 32;
pub const seL4_WordSizeBits: usize = 2;
pub const seL4_PageBits: usize = 12;
pub const seL4_SlotBits: usize = 4;
pub const seL4_TCBBits: usize = 9;
pub const seL4_EndpointBits: usize = 4;
pub const seL4_PageTableEntryBits: usize = 2;
pub const seL4_PageTableIndexBits: usize = 10;
pub const seL4_LargePageBits: usize = 22;
pub const seL4_PageTableBits: usize = 12;
pub const seL4_VSpaceBits: usize = seL4_PageTableBits;
pub const seL4_NumASIDPoolBits: usize = 5;
pub const seL4_ASIDPoolIndexBits: usize = 4;
pub const seL4_ASIDPoolBits: usize = 12;

pub type seL4_RISCV_Page = seL4_CPtr;
pub type seL4_RISCV_PageTable = seL4_CPtr;
pub type seL4_RISCV_ASIDControl = seL4_CPtr;
pub type seL4_RISCV_ASIDPool = seL4_CPtr;

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
    ExecuteNever = 0x1,
    Default_VMAttributes = 0,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

#[inline(always)]
pub unsafe fn seL4_GetIPCBuffer() -> *mut seL4_IPCBuffer {
    // Magic external symbol setup by runtime once TLS is primed
    enum c_void {}
    extern "C" {
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
pub unsafe fn seL4_GetCapReceivePath(
    receiveCNode: *mut seL4_CPtr,
    receiveIndex: *mut seL4_CPtr,
    receiveDepth: *mut seL4_Word,
) {
    let ipcbuffer = seL4_GetIPCBuffer();
    if !receiveCNode.is_null() {
        *receiveCNode = (*ipcbuffer).receiveCNode;
    }
    if !receiveIndex.is_null() {
        *receiveIndex = (*ipcbuffer).receiveIndex;
    }
    if !receiveDepth.is_null() {
        *receiveDepth = (*ipcbuffer).receiveDepth;
    }
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
    asm!("ecall",
        in("a7") swinum!(SyscallId::Send),
        in("a0") dest,
        in("a1") msgInfo.words[0],
        in("a2") seL4_GetMR(0),
        in("a3") seL4_GetMR(1),
        in("a4") seL4_GetMR(2),
        in("a5") seL4_GetMR(3),
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

    asm!("ecall",
        in("a7") swinum!(SyscallId::Send),
        in("a0") dest,
        in("a1") msgInfo.words[0],
        in("a2") msg0,
        in("a3") msg1,
        in("a4") msg2,
        in("a5") msg3,
    );
}

#[inline(always)]
pub unsafe fn seL4_NBSend(dest: seL4_CPtr, msgInfo: seL4_MessageInfo) {
    asm!("ecall",
        in("a7") swinum!(SyscallId::NBSend),
        in("a0") dest,
        in("a1") msgInfo.words[0],
        in("a2") seL4_GetMR(0),
        in("a3") seL4_GetMR(1),
        in("a4") seL4_GetMR(2),
        in("a5") seL4_GetMR(3),
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

    asm!("ecall",
        in("a7") swinum!(SyscallId::NBSend),
        in("a0") dest,
        in("a1") msgInfo.words[0],
        in("a2") msg0,
        in("a3") msg1,
        in("a4") msg2,
        in("a5") msg3,
    );
}

#[cfg(not(feature = "CONFIG_KERNEL_MCS"))]
#[inline(always)]
pub unsafe fn seL4_Reply(msgInfo: seL4_MessageInfo) {
    asm!("ecall",
        in("a7") swinum!(SyscallId::Reply),
        in("a1") msgInfo.words[0],
        in("a2") seL4_GetMR(0),
        in("a3") seL4_GetMR(1),
        in("a4") seL4_GetMR(2),
        in("a5") seL4_GetMR(3),
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

    asm!("ecall",
        in("a7") swinum!(SyscallId::Reply),
        in("a1") msgInfo.words[0],
        in("a2") msg0,
        in("a3") msg1,
        in("a4") msg2,
        in("a5") msg3,
    );
}

#[inline(always)]
pub unsafe fn seL4_Signal(dest: seL4_CPtr) {
    let info = seL4_MessageInfo::new(0, 0, 0, 0).words[0];
    asm!("ecall",
        in("a7") swinum!(SyscallId::Send),
        in("a0") dest,
        in("a1") info,
    );
}

#[inline(always)]
pub unsafe fn seL4_Recv(mut src: seL4_CPtr, sender: *mut seL4_Word) -> seL4_MessageInfo {
    let mut info: seL4_Word;
    let mut msg0 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg1 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg2 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg3 = ::core::mem::MaybeUninit::uninit().assume_init();

    asm!("ecall",
        in("a7") swinum!(SyscallId::Recv),
        out("a0") src,
        out("a1") info,
        out("a2") msg0,
        out("a3") msg1,
        out("a4") msg2,
        out("a5") msg3,
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
    let mut info: seL4_Word;
    let mut msg0 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg1 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg2 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg3 = ::core::mem::MaybeUninit::uninit().assume_init();

    asm!("ecall",
        in("a7") swinum!(SyscallId::NBRecv),
        inout("a0") src,
        out("a1") info,
        out("a2") msg0,
        out("a3") msg1,
        out("a4") msg2,
        out("a5") msg3,
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

    asm!("ecall",
        in("a7") swinum!(SyscallId::Call),
        in("a0") dest,
        inout("a1") msgInfo.words[0] => info,
        inout("a2") msg0,
        inout("a3") msg1,
        inout("a4") msg2,
        inout("a5") msg3,
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

    asm!("ecall",
        in("a7") swinum!(SyscallId::Call),
        in("a0") dest,
        inout("a1") msgInfo.words[0] => info,
        inout("a2") msg0,
        inout("a3") msg1,
        inout("a4") msg2,
        inout("a5") msg3,
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

    asm!("ecall",
        in("a7") swinum!(SyscallId::ReplyRecv),
        inout("a0") src,
        inout("a1") msgInfo.words[0] => info,
        inout("a2") msg0,
        inout("a3") msg1,
        inout("a4") msg2,
        inout("a5") msg3,
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

    asm!("ecall",
        in("a7") swinum!(SyscallId::ReplyRecv),
        inout("a0") src => badge,
        inout("a1") msgInfo.words[0] => info,
        inout("a2") msg0,
        inout("a3") msg1,
        inout("a4") msg2,
        inout("a5") msg3,
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

    asm!("ecall",
        in("a7") swinum!(SyscallId::NBSendRecv),
        inout("a0") src => badge,
        inout("a1") msgInfo.words[0] => info,
        inout("a2") msg0,
        inout("a3") msg1,
        inout("a4") msg2,
        inout("a5") msg3,
        in("a6") reply,
        in("t0") dest,
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

    asm!("ecall",
        in("a7") swinum!(SyscallId::NBSendRecv),
        inout("a0") src => badge,
        inout("a1") msgInfo.words[0] => info,
        inout("a2") msg0,
        inout("a3") msg1,
        inout("a4") msg2,
        inout("a5") msg3,
        in("a6") reply,
        in("t0") dest,
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

    asm!("ecall",
        in("a7") swinum!(SyscallId::NBSendWait),
        inout("a0") src => badge,
        inout("a1") msgInfo.words[0] => info,
        inout("a2") msg0,
        inout("a3") msg1,
        inout("a4") msg2,
        inout("a5") msg3,
        in("a6") dest,
        in("t0") 0,  // XXX dest
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

    asm!("ecall",
        in("a7") swinum!(SyscallId::NBSendRecv),
        inout("a0") src => badge,
        inout("a1") msgInfo.words[0] => info,
        inout("a2") msg0,
        inout("a3") msg1,
        inout("a4") msg2,
        inout("a5") msg3,
        in("a6") dest,
        in("t0") 0,  // XXX does this dtrt
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
    asm!("ecall",
        in("a7") swinum!(SyscallId::Yield),
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

    asm!("ecall",
        in("a7") swinum!(SyscallId::Wait),
        inout("a0") src => badge,
        out("a1") info,
        inout("a2") msg0,
        inout("a3") msg1,
        inout("a4") msg2,
        inout("a5") msg3,
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

    asm!("ecall",
        in("a7") swinum!(SyscallId::Wait),
        inout("a0") src => badge,
        out("a1") info,
        inout("a2") msg0,
        inout("a3") msg1,
        inout("a4") msg2,
        inout("a5") msg3,
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

    asm!("ecall",
        in("a7") swinum!(SyscallId::NBWait),
        inout("a0") src => badge,
        out("a1") info,
        inout("a2") msg0,
        inout("a3") msg1,
        inout("a4") msg2,
        inout("a5") msg3,
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
    asm!("ecall",
        in("a7") swinum!(SyscallId::DebugPutChar),
        in("a0") c,
        options(nostack),
    );
}

#[cfg(feature = "CONFIG_PRINTING")]
#[inline(always)]
pub unsafe fn seL4_DebugDumpScheduler() {
    asm!("ecall",
        in("a7") swinum!(SyscallId::DebugDumpScheduler),
        options(nomem, nostack),
    );
}

#[cfg(feature = "CONFIG_DEBUG_BUILD")]
#[inline(always)]
pub unsafe fn seL4_DebugHalt() {
    asm!("ecall",
        in("a7") swinum!(SyscallId::DebugHalt),
        options(nomem, nostack),
    );
}

#[cfg(feature = "CONFIG_DEBUG_BUILD")]
#[inline(always)]
pub unsafe fn seL4_DebugSnapshot() {
    asm!("ecall",
        in("a7") swinum!(SyscallId::DebugSnapshot),
        options(nomem, nostack),
    );
}

#[cfg(feature = "CONFIG_DEBUG_BUILD")]
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
#[cfg(feature = "CONFIG_DEBUG_BUILD")]
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

#[cfg(feature = "CONFIG_ENABLE_BENCHMARKS")]
#[inline(always)]
pub unsafe fn seL4_BenchmarkResetLog() {
    asm!("ecall",
        in("a7") swinum!(SyscallId::BenchmarkResetLog),
        options(nomem, nostack),
    );
}

#[cfg(feature = "CONFIG_ENABLE_BENCHMARKS")]
#[inline(always)]
pub unsafe fn seL4_BenchmarkFinalizeLog() {
    asm!("ecall",
        in("a7") swinum!(SyscallId::BenchmarkFinalizeLog),
        options(nomem, nostack),
    );
}

// TODO(sleffler): seL4_BenchmarkSetLogBuffer
// TODO(sleffler): seL4_BenchmarkNullSyscall
// TODO(sleffler): seL4_BenchmarkFlushCaches
// TODO(sleffler): seL4_BenchmarkFlushL1Caches

#[cfg(feature = "CONFIG_SET_TLS_BASE_SELF")]
pub unsafe fn seL4_SetTLSBase(tls_base: seL4_Word) {
    let info: seL4_Word = 0; // XXX does this dtrt?
    asm!("ecall",
        in("a7") swinum!(SyscallId::SetTLSBase),
        in("a0") tls_base,
        in("a1") info,
    );
}
