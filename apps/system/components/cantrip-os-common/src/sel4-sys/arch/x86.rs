/*
 * Copyright 2015, Corey Richardson
 * Copyright 2014, NICTA
 *
 * This software may be distributed and modified according to the terms of
 * the BSD 2-Clause license. Note that NO WARRANTY is provided.
 * See "LICENSE_BSD2.txt" for details.
 *
 * @TAG(NICTA_BSD)
 */

use core::mem::uninitialized;

pub const seL4_WordBits: usize = 32;
pub const seL4_PageBits: usize = 12;
pub const seL4_SlotBits: usize = 4;
pub const seL4_TCBBits: usize = 11;
pub const seL4_EndpointBits: usize = 4;
pub const seL4_NotificationBits: usize = 4;
pub const seL4_PageTableBits: usize = 12;
pub const seL4_PageDirBits: usize = 12;
pub const seL4_IOPageTableBits: usize = 12;
pub const seL4_ASIDPoolBits: usize = 12;

pub const seL4_HugePageBits: usize = 30;

pub const seL4_VCPUBits: usize = 14;
pub const seL4_EPTPML4Bits: usize = 12;
pub const seL4_EPTPDPTBits: usize = 12;
pub const seL4_EPTPDBits: usize = 12;
pub const seL4_EPTPTBits: usize = 12;

pub const seL4_PDPTBits: usize = 5;
pub const seL4_LargePageBits: usize = 21;

pub const seL4_MinUntypedBits: usize = 4;
pub const seL4_MaxUntypedBits: usize = 29;

pub type seL4_X86_ASIDControl = seL4_CPtr;
pub type seL4_X86_ASIDPool = seL4_CPtr;
pub type seL4_X86_IOPageTable = seL4_CPtr;
pub type seL4_X86_IOPort = seL4_CPtr;
pub type seL4_X86_IOSpace = seL4_CPtr;
pub type seL4_X86_PageDirectory = seL4_CPtr;
pub type seL4_X86_Page = seL4_CPtr;
pub type seL4_X86_PageTable = seL4_CPtr;

#[cfg(feature = "arch_generic")]
include!("x86_generic.rs");

error_types!(u32);

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

    seL4_X86_4K,
    seL4_X86_LargePageObject,
    seL4_X86_PageTableObject,
    seL4_X86_PageDirectoryObject,

    #[cfg(feature = "CONFIG_IOMMU")]
    seL4_X86_IOPageTableObject,

    #[cfg(feature = "CONFIG_VTX")]
    seL4_X86_VCPUObject,
    #[cfg(feature = "CONFIG_VTX")]
    seL4_X86_EPTPML4Object,
    #[cfg(feature = "CONFIG_VTX")]
    seL4_X86_EPTPDPTObject,
    #[cfg(feature = "CONFIG_VTX")]
    seL4_X86_EPTPDObject,
    #[cfg(feature = "CONFIG_VTX")]
    seL4_X86_EPTPTObject,

    seL4_LastObjectType,
}
impl From<seL4_ObjectType> for seL4_Word {
    fn from(type_: seL4_ObjectType) -> seL4_Word { type_ as seL4_Word }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum seL4_X86_VMAttributes {
    WriteBack = 0,
    WriteThrough = 1,
    CacheDisabled = 2,
    Uncacheable = 3,
    WriteCombining = 4,
}
impl From<u32> for seL4_X86_VMAttributes {
    fn from(val: u32) -> seL4_x86_VMAttributes { unsafe { ::core::mem::transmute(val & 7) } }
}
pub const seL4_X86_Default_VMAttributes: seL4_X86_VMAttributes = seL4_X86_VMAttributes::WriteBack;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct seL4_UserContext {
    pub eip: seL4_Word,
    pub esp: seL4_Word,
    pub eflags: seL4_Word,
    pub eax: seL4_Word,
    pub ebx: seL4_Word,
    pub ecx: seL4_Word,
    pub edx: seL4_Word,
    pub esi: seL4_Word,
    pub edi: seL4_Word,
    pub ebp: seL4_Word,
    pub tls_base: seL4_Word,
    pub fs: seL4_Word,
    pub gs: seL4_Word,
}

#[inline(always)]
pub unsafe fn seL4_GetMR(regnum: isize) -> seL4_Word {
    let mr;
    asm!("movl %fs:4(,$1,0x4), $0" : "=r"(mr) : "r"(regnum) : : "volatile");
    mr
}

#[inline(always)]
pub unsafe fn seL4_SetMR(regnum: isize, value: seL4_Word) {
    asm!("movl $0, %fs:4(,$1,0x4)" : : "r"(value), "r"(regnum) : "memory" : "volatile");
}

#[inline(always)]
pub unsafe fn seL4_GetUserData() -> seL4_Word {
    let data;
    asm!("movl %fs:484, $0" : "=r"(data) : : : "volatile");
    data
}

#[inline(always)]
pub unsafe fn seL4_GetIPCBuffer() -> *mut seL4_IPCBuffer {
    seL4_GetUserData() as isize as *mut seL4_IPCBuffer
}

#[inline(always)]
pub unsafe fn seL4_SetUserData(data: seL4_Word) {
    asm!("movl $0, %fs:484" : : "r"(data) : "memory" : "volatile");
}

#[inline(always)]
pub unsafe fn seL4_GetBadge(index: isize) -> seL4_CapData {
    let mut badge: seL4_CapData = uninitialized();
    asm!("movl %fs:488(,$1,0x4), $0" : "=r"(badge.words[0]) : "r"(index) : : "volatile");
    badge
}

#[inline(always)]
pub unsafe fn seL4_GetCap(index: isize) -> seL4_CPtr {
    let cptr;
    asm!("movl %fs:488(,$1,0x4), $0" : "=r"(cptr) : "r"(index) : : "volatile");
    cptr
}

#[inline(always)]
pub unsafe fn seL4_SetCap(index: isize, cptr: seL4_CPtr) {
    asm!("movl $0, %fs:488(,$1,0x4)" : : "r"(cptr), "r"(index) : "memory" : "volatile");
}

#[inline(always)]
pub unsafe fn seL4_GetCapReceivePath(
    receiveCNode: *mut seL4_CPtr,
    receiveIndex: *mut seL4_CPtr,
    receiveDepth: *mut seL4_Word,
) {
    if !receiveCNode.is_null() {
        asm!("movl %fs:500, $0" : "=r"(*receiveCNode) : : : "volatile");
    }

    if !receiveIndex.is_null() {
        asm!("movl %fs:504, $0" : "=r"(*receiveIndex) : : : "volatile");
    }

    if !receiveDepth.is_null() {
        asm!("movl %fs:508, $0" : "=r"(*receiveDepth) : : : "volatile");
    }
}

#[inline(always)]
pub unsafe fn seL4_SetCapReceivePath(
    receiveCNode: seL4_CPtr,
    receiveIndex: seL4_CPtr,
    receiveDepth: seL4_Word,
) {
    asm!("movl $0, %fs:500" : : "r"(receiveCNode) : "memory" : "volatile");
    asm!("movl $0, %fs:504" : : "r"(receiveIndex) : "memory" : "volatile");
    asm!("movl $0, %fs:508" : : "r"(receiveDepth) : "memory" : "volatile");
}

#[inline(always)]
unsafe fn x86_sys_send(
    sys: seL4_Word,
    mut dest: seL4_Word,
    info: seL4_Word,
    mr1: seL4_Word,
    mr2: seL4_Word,
) {
    asm!("pushl %ebp
          pushl %ebx
          movl %ecx, %ebp
          movl %esp, %ecx
          movl %edx, %ebx
          leal 1f, %edx
          1:
          sysenter
          popl %ebx
          popl %ebp"
          : "+{dx}" (dest)
          : "{ax}" (sys),
            "{dx}" (dest),
            "{si}" (info),
            "{di}" (mr1),
            "{cx}" (mr2)
          : "%edx"
          : "volatile");
}

#[inline(always)]
unsafe fn x86_sys_reply(sys: seL4_Word, info: seL4_Word, mr1: seL4_Word, mr2: seL4_Word) {
    asm!("pushl %ebp
          pushl %ebx
          movl %ecx, %ebp
          movl %esp, %ecx
          leal 1f, %edx
          1:
          sysenter
          popl %ebx
          popl %ebp"
          :
          : "{ax}" (sys),
            "{si}" (info),
            "{di}" (mr1),
            "{cx}" (mr2)
          : "%edx"
          : "volatile");
}

#[inline(always)]
unsafe fn x86_sys_send_null(sys: seL4_Word, mut dest: seL4_Word, info: seL4_Word) {
    asm!("pushl %ebp
          pushl %ebx
          movl %esp, %ecx
          movl %edx, %ebx
          leal 1f, %edx
          1:
          sysenter
          popl %ebx
          popl %ebp"
          : "={dx}" (dest)
          : "{ax}" (sys),
            "{si}" (info),
            "{dx}" (dest)
          : "%ecx"
          : "volatile");
}

#[inline(always)]
unsafe fn x86_sys_recv(
    sys: seL4_Word,
    src: seL4_Word,
    out_badge: *mut seL4_Word,
    out_info: *mut seL4_Word,
    out_mr1: *mut seL4_Word,
    out_mr2: *mut seL4_Word,
) {
    asm!("pushl %ebp
          pushl %ebx
          movl %esx, %ecp
          movl %edp, %ebx
          leal 1f, %edx
          1:
          sysenter
          movl %ebx, %edx
          popl %ebx
          movl %ebp, %ecx
          popl %ebp"
          : "={si}" (*out_info)
            "={di}" (*out_mr1),
            "={cx}" (*out_mr2),
            "={dx}" (*out_badge)
          : "{ax}" (sys),
            "{dx}" (src)
          : "memory"
          : "volatile");
}

#[inline(always)]
unsafe fn x86_sys_send_recv(
    sys: seL4_Word,
    dest: seL4_Word,
    out_badge: *mut seL4_Word,
    info: seL4_Word,
    out_info: *mut seL4_Word,
    in_out_mr1: *mut seL4_Word,
    in_out_mr2: *mut seL4_Word,
) {
    asm!("pushl %ebp
          pushl %ebx
          movl %ecx, %ebp
          movl %esp, %ecx
          movl %edx, %ebx
          leal 1f, %edx
          1:
          sysenter
          movl %ebx, %edx
          popl %ebx
          movl %ebp, %ecx
          popl %ebp"
          : "={si}" (*out_info)
            "={di}" (*in_out_mr1),
            "={cx}" (*in_out_mr2),
            "={dx}" (*out_badge)
          : "{ax}" (sys),
            "{si}" (info),
            "{di}" (*in_out_mr1),
            "{cx}" (*in_out_mr2),
            "{dx}" (dest)
          : "memory"
          : "volatile");
}

#[inline(always)]
unsafe fn x86_sys_null(sys: seL4_Word) {
    asm!("pushl %ebp
          pushl %ebx
          movl %esp, %ecx
          leal 1f, %edx
          1:
          sysenter
          popl %ebx
          popl %ebp"
          :
          : "{ax}" (sys)
          : "%ecx", "%edx"
          : "volatile");
}

#[inline(always)]
pub unsafe fn seL4_Send(dest: seL4_CPtr, msgInfo: seL4_MessageInfo) {
    x86_sys_send(
        SyscallId::Send as seL4_Word,
        dest,
        msgInfo.words[0],
        seL4_GetMR(0),
        seL4_GetMR(1),
    );
}

macro_rules! opt_deref {
    ($name:expr) => {
        if !$name.is_null() {
            *$name
        } else {
            0
        }
    };
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
) {
    x86_sys_send(
        SyscallId::Send as seL4_Word,
        dest,
        msgInfo.words[0],
        if mr0.is_null() { 0 } else { *mr0 },
        if mr1.is_null() { 0 } else { *mr1 },
    );
}

#[inline(always)]
pub unsafe fn seL4_NBSend(dest: seL4_CPtr, msgInfo: seL4_MessageInfo) {
    x86_sys_send(
        SyscallId::NBSend as seL4_Word,
        dest,
        msgInfo.words[0],
        seL4_GetMR(0),
        seL4_GetMR(1),
    );
}

#[inline(always)]
pub unsafe fn seL4_NBSendWithMRs(
    dest: seL4_CPtr,
    msgInfo: seL4_MessageInfo,
    mr0: *mut seL4_Word,
    mr1: *mut seL4_Word,
) {
    x86_sys_send(
        SyscallId::NBSend as seL4_Word,
        dest,
        msgInfo.words[0],
        if mr0.is_null() { 0 } else { *mr0 },
        if mr1.is_null() { 0 } else { *mr1 },
    );
}

#[inline(always)]
pub unsafe fn seL4_Reply(msgInfo: seL4_MessageInfo) {
    x86_sys_reply(
        SyscallId::Reply as seL4_Word,
        msgInfo.words[0],
        seL4_GetMR(0),
        seL4_GetMR(1),
    );
}
#[inline(always)]
pub unsafe fn seL4_ReplyWithMRs(
    msgInfo: seL4_MessageInfo,
    mr0: *mut seL4_Word,
    mr1: *mut seL4_Word,
) {
    x86_sys_reply(
        SyscallId::Reply as seL4_Word,
        msgInfo.words[0],
        if mr0.is_null() { 0 } else { *mr0 },
        if mr1.is_null() { 0 } else { *mr1 },
    );
}

#[inline(always)]
pub unsafe fn seL4_Signal(dest: seL4_CPtr) {
    x86_sys_send_null(
        SyscallId::Send as seL4_Word,
        dest,
        seL4_MessageInfo::new(0, 0, 0, 0).words[0],
    );
}

#[inline(always)]
pub unsafe fn seL4_Recv(src: seL4_CPtr, sender: *mut seL4_Word) -> seL4_MessageInfo {
    let mut info: seL4_MessageInfo = uninitialized();
    let mut badge: seL4_Word = uninitialized();
    let mut mr0: seL4_Word = uninitialized();
    let mut mr1: seL4_Word = uninitialized();

    x86_sys_recv(
        SyscallId::Recv as seL4_Word,
        src,
        &mut badge,
        &mut info.words[0],
        &mut mr0 as *mut _,
        &mut mr1,
    );

    seL4_SetMR(0, mr0);
    seL4_SetMR(1, mr1);

    opt_assign!(sender, badge);

    info
}

#[inline(always)]
pub unsafe fn seL4_RecvWithMRs(
    src: seL4_CPtr,
    sender: *mut seL4_Word,
    mr0: *mut seL4_Word,
    mr1: *mut seL4_Word,
) -> seL4_MessageInfo {
    let mut info: seL4_MessageInfo = uninitialized();
    let mut badge: seL4_Word = uninitialized();
    let mut msg0: seL4_Word = uninitialized();
    let mut msg1: seL4_Word = uninitialized();

    x86_sys_recv(
        SyscallId::Recv as seL4_Word,
        src,
        &mut badge,
        &mut info.words[0],
        &mut msg0,
        &mut msg1,
    );

    opt_assign!(mr0, msg0);
    opt_assign!(mr1, msg1);
    opt_assign!(sender, badge);

    info
}

#[inline(always)]
pub unsafe fn seL4_NBRecv(src: seL4_CPtr, sender: *mut seL4_Word) -> seL4_MessageInfo {
    let mut info: seL4_MessageInfo = uninitialized();
    let mut badge: seL4_Word = uninitialized();
    let mut mr0: seL4_Word = uninitialized();
    let mut mr1: seL4_Word = uninitialized();

    x86_sys_recv(
        SyscallId::NBRecv as seL4_Word,
        src,
        &mut badge,
        &mut info.words[0],
        &mut mr0,
        &mut mr1,
    );

    seL4_SetMR(0, mr0);
    seL4_SetMR(1, mr1);

    opt_assign!(sender, badge);

    info
}

#[inline(always)]
pub unsafe fn seL4_Call(mut dest: seL4_CPtr, msgInfo: seL4_MessageInfo) -> seL4_MessageInfo {
    let mut info: seL4_MessageInfo = uninitialized();
    let mut mr0 = seL4_GetMR(0);
    let mut mr1 = seL4_GetMR(1);

    x86_sys_send_recv(
        SyscallId::Call as seL4_Word,
        dest,
        &mut dest,
        msgInfo.words[0],
        &mut info.words[0],
        &mut mr0,
        &mut mr1,
    );

    seL4_SetMR(0, mr0);
    seL4_SetMR(1, mr1);

    info
}

#[inline(always)]
pub unsafe fn seL4_CallWithMRs(
    mut dest: seL4_CPtr,
    msgInfo: seL4_MessageInfo,
    mr0: *mut seL4_Word,
    mr1: *mut seL4_Word,
) -> seL4_MessageInfo {
    let mut info: seL4_MessageInfo = uninitialized();
    let mut msg0: seL4_Word = 0;
    let mut msg1: seL4_Word = 0;

    if !mr0.is_null() {
        if msgInfo.get_length() > 0 {
            msg0 = *mr0;
        }
    }
    if !mr1.is_null() {
        if msgInfo.get_length() > 1 {
            msg1 = *mr1;
        }
    }

    x86_sys_send_recv(
        SyscallId::Call as seL4_Word,
        dest,
        &mut dest,
        msgInfo.words[0],
        &mut info.words[0],
        &mut msg0,
        &mut msg1,
    );

    opt_assign!(mr0, msg0);
    opt_assign!(mr1, msg1);

    info
}

#[inline(always)]
pub unsafe fn seL4_ReplyRecv(
    dest: seL4_CPtr,
    msgInfo: seL4_MessageInfo,
    sender: *mut seL4_Word,
) -> seL4_MessageInfo {
    let mut info: seL4_MessageInfo = uninitialized();
    let mut badge: seL4_Word = uninitialized();
    let mut mr0 = seL4_GetMR(0);
    let mut mr1 = seL4_GetMR(1);

    x86_sys_send_recv(
        SyscallId::ReplyRecv as seL4_Word,
        dest,
        &mut badge,
        msgInfo.words[0],
        &mut info.words[0],
        &mut mr0,
        &mut mr1,
    );

    seL4_SetMR(0, mr0);
    seL4_SetMR(1, mr1);

    opt_assign!(sender, badge);

    info
}

#[inline(always)]
pub unsafe fn seL4_ReplyWaitWithMRs(
    dest: seL4_CPtr,
    msgInfo: seL4_MessageInfo,
    sender: *mut seL4_Word,
    mr0: *mut seL4_Word,
    mr1: *mut seL4_Word,
) -> seL4_MessageInfo {
    let mut info: seL4_MessageInfo = uninitialized();
    let mut badge: seL4_Word = uninitialized();
    let mut msg0: seL4_Word = 0;
    let mut msg1: seL4_Word = 0;

    if !mr0.is_null() {
        if msgInfo.get_length() > 0 {
            msg0 = *mr0;
        }
    }
    if !mr1.is_null() {
        if msgInfo.get_length() > 1 {
            msg1 = *mr1;
        }
    }

    x86_sys_send_recv(
        SyscallId::ReplyRecv as seL4_Word,
        dest,
        &mut badge,
        msgInfo.words[0],
        &mut info.words[0],
        &mut msg0,
        &mut msg1,
    );

    opt_assign!(mr0, msg0);
    opt_assign!(mr1, msg1);
    opt_assign!(sender, badge);

    info
}

#[inline(always)]
pub unsafe fn seL4_Yield() {
    x86_sys_null(SyscallId::Yield as seL4_Word);
    asm!("" ::: "%esi", "%edi", "memory" : "volatile");
}

#[inline(always)]
pub unsafe fn seL4_VMEnter(vcpu: seL4_CPtr, sender: *mut seL4_Word) -> seL4_Word {
    let mut fault: seL4_Word = uninitialized();
    let mut badge: seL4_Word = uninitialized();
    let mut mr0 = seL4_GetMR(0);
    let mut mr1 = seL4_GetMR(1);

    x86_sys_send_recv(
        SyscallId::VMEnter as seL4_Word,
        vcpu,
        &mut badge,
        0,
        &mut fault,
        &mut mr0,
        &mut mr1,
    );

    seL4_SetMR(0, mr0);
    seL4_SetMR(1, mr1);

    if fault == 0 && !sender.is_null() {
        *sender = badge;
    }

    fault
}

#[inline(always)]
pub unsafe fn seL4_DebugPutChar(c: u8) {
    let mut unused0 = 0;
    let mut unused1 = 0;
    let mut unused2 = 0;
    let mut unused3 = 0;
    x86_sys_send_recv(
        SyscallId::DebugPutChar as seL4_Word,
        c as seL4_Word,
        &mut unused0,
        0,
        &mut unused1,
        &mut unused2,
        &mut unused3,
    );
}

#[inline(always)]
pub unsafe fn seL4_DebugHalt() {
    x86_sys_null(SyscallId::DebugHalt as seL4_Word);
    asm!("" ::: "%esi", "%edi", "memory" : "volatile");
}

#[inline(always)]
pub unsafe fn seL4_DebugSnapshot() {
    x86_sys_null(SyscallId::DebugSnapshot as seL4_Word);
    asm!("" ::: "%esi", "%edi", "memory" : "volatile");
}

#[inline(always)]
pub unsafe fn seL4_DebugCapIdentify(mut cap: seL4_CPtr) -> u32 {
    let mut unused0 = 0;
    let mut unused1 = 0;
    let mut unused2 = 0;
    x86_sys_send_recv(
        SyscallId::DebugCapIdentify as seL4_Word,
        cap,
        &mut cap,
        0,
        &mut unused0,
        &mut unused1,
        &mut unused2,
    );
    cap as _
}

#[cfg(feature = "CONFIG_PRINTING")]
#[inline(always)]
pub unsafe fn seL4_DebugDumpCNode(mut cap: seL4_CPtr) {
    let mut unused0 = 0;
    let mut unused1 = 0;
    let mut unused2 = 0;
    x86_sys_send_recv(
        SyscallId::DebugDumpCNode as seL4_Word,
        cap,
        &mut cap,
        0,
        &mut unused0,
        &mut unused1,
        &mut unused2,
    );
}

/// Note: name MUST be NUL-terminated.
#[inline(always)]
pub unsafe fn seL4_DebugNameThread(tcb: seL4_CPtr, name: &[u8]) {
    core::ptr::copy_nonoverlapping(
        name.as_ptr() as *mut u8,
        (&mut (*seL4_GetIPCBuffer()).msg).as_mut_ptr() as *mut u8,
        name.len(),
    );
    let mut unused0 = 0;
    let mut unused1 = 0;
    let mut unused2 = 0;
    let mut unused3 = 0;
    x86_sys_send_recv(
        SyscallId::DebugNameThread as seL4_Word,
        tcb,
        &mut unused0,
        0,
        &mut unused1,
        &mut unused2,
        &mut unused3,
    );
}

#[inline(always)]
#[cfg(feature = "SEL4_DANGEROUS_CODE_INJECTION")]
pub unsafe fn seL4_DebugRun(userfn: extern "C" fn(*mut u8), userarg: *mut u8) {
    let userfnptr = userfn as *mut ();
    x86_sys_send_null(
        SyscallId::DebugRun as seL4_Word,
        userfnptr as seL4_Word,
        userarg as seL4_Word,
    );
    asm!("" ::: "%edi", "memory" : "volatile");
}

#[inline(always)]
#[cfg(feature = "SEL4_CONFIG_BENCHMARK")]
pub unsafe fn seL4_BenchmarkResetLog() -> seL4_Word {
    let mut unused0 = 0;
    let mut unused1 = 0;
    let mut unused2 = 0;

    let mut ret = 0;

    x86_sys_send_recv(
        SyscallId::BenchmarkResetLog as seL4_Word,
        0,
        &mut ret,
        0,
        &mut unused0 as *mut _ as usize as seL4_Word,
        &mut unused1 as *mut _ as usize as seL4_Word,
        &mut unused2 as *mut _ as usize as seL4_Word,
    );

    ret
}

#[inline(always)]
#[cfg(feature = "SEL4_CONFIG_BENCHMARK")]
pub unsafe fn seL4_BenchmarkFinalizeLog() {
    x86_sys_null(SyscallId::BenchmarkFinalizeLog as seL4_Word);
    asm!("" ::: "%esi", "%edi", "memory" : "volatile");
}

#[inline(always)]
#[cfg(feature = "SEL4_CONFIG_BENCHMARK")]
pub unsafe fn seL4_BenchmarkSetLogBuffer(mut frame_cptr: seL4_Word) -> seL4_Word {
    let mut unused0 = 0;
    let mut unused1 = 0;
    let mut unused2 = 0;
    x86_sys_send_recv(
        SyscallId::BenchmarkSetLogBuffer as seL4_Word,
        frame_cptr,
        &mut cap,
        0,
        &mut unused0 as *mut _ as usize as seL4_Word,
        &mut unused1 as *mut _ as usize as seL4_Word,
        &mut unused2 as *mut _ as usize as seL4_Word,
    );
    frame_cptr
}

#[inline(always)]
#[cfg(feature = "SEL4_CONFIG_BENCHMARK")]
pub unsafe fn seL4_BenchmarkNullSyscall() {
    x86_sys_null(SyscallId::BenchmarkNullSyscall as seL4_Word);
    asm!("" ::: "%esi", "%edi", "memory" : "volatile");
}

#[inline(always)]
#[cfg(feature = "SEL4_CONFIG_BENCHMARK")]
pub unsafe fn seL4_BenchmarkFlushCaches() {
    x86_sys_null(SyscallId::BenchmarkFlushCaches as seL4_Word);
    asm!("" ::: "%esi", "%edi", "memory" : "volatile");
}

#[inline(always)]
#[cfg(feature = "SEL4_CONFIG_BENCHMARK")]
pub unsafe fn seL4_BenchmarkGetThreadUtilization(tcb: seL4_Word) {
    let mut unused0 = 0;
    let mut unused1 = 0;
    let mut unused2 = 0;
    let mut unused3 = 0;
    x86_sys_send_recv(
        SyscallId::BenchmarkGetThreadUtilisation as seL4_Word,
        tcb,
        &mut unused0 as *mut _ as usize as seL4_Word,
        0,
        &mut unused1 as *mut _ as usize as seL4_Word,
        &mut unused2 as *mut _ as usize as seL4_Word,
        &mut unused3 as *mut _ as usize as seL4_Word,
    );
}

#[inline(always)]
#[cfg(feature = "SEL4_CONFIG_BENCHMARK")]
pub unsafe fn seL4_BenchmarkGetThreadUtilization(tcb: seL4_Word) {
    let mut unused0 = 0;
    let mut unused1 = 0;
    let mut unused2 = 0;
    let mut unused3 = 0;
    x86_sys_send_recv(
        SyscallId::BenchmarkResetThreadUtilisation as seL4_Word,
        tcb,
        &mut unused0 as *mut _ as usize as seL4_Word,
        0,
        &mut unused1 as *mut _ as usize as seL4_Word,
        &mut unused2 as *mut _ as usize as seL4_Word,
        &mut unused3 as *mut _ as usize as seL4_Word,
    );
}
