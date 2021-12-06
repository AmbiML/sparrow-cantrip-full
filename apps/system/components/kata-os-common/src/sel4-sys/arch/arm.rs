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
pub const seL4_PageBits: usize = 12;
pub const seL4_SlotBits: usize = 4;
pub const seL4_TCBBits: usize = 9;
pub const seL4_EndpointBits: usize = 4;
pub const seL4_PageTableBits: usize = 10;
pub const seL4_PageDirBits: usize = 14;
pub const seL4_ASIDPoolBits: usize = 12;

pub const seL4_Frame_Args: usize = 4;
pub const seL4_Frame_MRs: usize = 7;
pub const seL4_Frame_HasNPC: usize = 0;

pub const seL4_GlobalsFrame: *mut u8 = 0xffffc000 as *mut u8;

pub type seL4_ARM_Page = seL4_CPtr;
pub type seL4_ARM_PageTable = seL4_CPtr;
pub type seL4_ARM_PageDirectory = seL4_CPtr;
pub type seL4_ARM_ASIDControl = seL4_CPtr;
pub type seL4_ARM_ASIDPool = seL4_CPtr;

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
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum seL4_ARM_VMAttributes {
    PageCacheable = 1,
    ParityEnabled = 2,
    ExecuteNever = 4,
}
pub const Default_VMAttributes: usize = 0;

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

    seL4_ARM_SmallPageObject,
    seL4_ARM_LargePageObject,
    seL4_ARM_SectionObject,  // XXX aarch32
    seL4_ARM_SuperSectionObject,  // XXX aarch32
    seL4_ARM_PageTableObject,
    seL4_ARM_PageDirectoryObject,

    #[cfg(feature = "CONFIG_ARM_HYPERVISOR_SUPPORT")]
    seL4_ARM_VCPUObject,

    #[cfg(feature = "CONFIG_TK1_SMMU")]
    seL4_ARM_IOPageTableObject,
}
impl From<seL4_ObjectType> for seL4_Word {
    fn from(type_: seL4_ObjectType) -> seL4_Word {
        type_ as seL4_Word
    }
}

// NB: capDL is defined using this (sigh)
pub const seL4_ObjectTypeCount: isize = seL4_ObjectType::seL4_LastObjectType as isize;

#[inline(always)]
pub unsafe fn seL4_GetIPCBuffer() -> *mut seL4_IPCBuffer {
	*(seL4_GlobalsFrame as *mut *mut seL4_IPCBuffer)
}

#[inline(always)]
pub unsafe fn seL4_GetTag() -> seL4_MessageInfo {
	(*seL4_GetIPCBuffer()).tag
}

#[inline(always)]
pub unsafe fn seL4_SetTag(tag: seL4_MessageInfo) {
	(*seL4_GetIPCBuffer()).tag = tag;
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
pub unsafe fn seL4_GetBadge(index: usize) -> seL4_CapData {
    let mut badge: seL4_CapData = ::core::mem::uninitialized();
	badge.set_Badge((*seL4_GetIPCBuffer()).caps_or_badges[index]);
    badge
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
pub unsafe fn seL4_GetCapReceivePath(receiveCNode: *mut seL4_CPtr,
                                     receiveIndex: *mut seL4_CPtr,
                                     receiveDepth: *mut seL4_Word) {
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
pub unsafe fn seL4_SetCapReceivePath(receiveCNode: seL4_CPtr,
                                     receiveIndex: seL4_CPtr,
                                     receiveDepth: seL4_Word) {
	let ipcbuffer = seL4_GetIPCBuffer();
	(*ipcbuffer).receiveCNode = receiveCNode;
	(*ipcbuffer).receiveIndex = receiveIndex;
	(*ipcbuffer).receiveDepth = receiveDepth;
}

macro_rules! swinum {
	($val:expr) => {
		$val as seL4_Word & 0x00ffffff
	}
}

#[inline(always)]
pub unsafe fn seL4_Send(dest: seL4_CPtr, msgInfo: seL4_MessageInfo) {
	let msg0 = seL4_GetMR(0);
	let msg1 = seL4_GetMR(1);
	let msg2 = seL4_GetMR(2);
	let msg3 = seL4_GetMR(3);
	let scno = SyscallId::Send as seL4_Word;
    asm!("swi $0"
	:
	: "i" (swinum!(SyscallId::Send)),
	  "{r0}" (dest),
	  "{r1}" (msgInfo.words[0]),
	  "{r2}" (msg0), "{r3}" (msg1),
	  "{r4}" (msg2), "{r5}" (msg3),
	  "{r7}" (scno)
	: "memory", "r0", "r1", "r2", "r3", "r4", "r5", "r7"
    : "volatile");
}

macro_rules! opt_deref {
    ($name:expr) => {
        if !$name.is_null() {
            *$name
        } else {
            0
        }
    }
}

macro_rules! opt_assign {
    ($loc:expr, $val:expr) => {
        if !$loc.is_null() {
            *$loc = $val;
        }
    }
}

#[inline(always)]
pub unsafe fn seL4_SendWithMRs(dest: seL4_CPtr, msgInfo: seL4_MessageInfo,
                               mr0: *mut seL4_Word, mr1: *mut seL4_Word,
                               mr2: *mut seL4_Word, mr3: *mut seL4_Word,
                               ) {
	let mut msg0 = ::core::mem::uninitialized();
	let mut msg1 = ::core::mem::uninitialized();
	let mut msg2 = ::core::mem::uninitialized();
	let mut msg3 = ::core::mem::uninitialized();

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
	let scno = SyscallId::Send as seL4_Word;
    asm!("swi $0"
	:
	: "i" (swinum!(SyscallId::Send)),
	  "{r0}" (dest),
	  "{r1}" (msgInfo.words[0]),
	  "{r2}" (msg0), "{r3}" (msg1),
	  "{r4}" (msg2), "{r5}" (msg3),
	  "{r7}" (scno)
	: "memory", "r0", "r1", "r2", "r3", "r4", "r5", "r7"
    : "volatile");
}

#[inline(always)]
pub unsafe fn seL4_NBSend(dest: seL4_CPtr, msgInfo: seL4_MessageInfo) {
	let msg0 = seL4_GetMR(0);
	let msg1 = seL4_GetMR(1);
	let msg2 = seL4_GetMR(2);
	let msg3 = seL4_GetMR(3);
	let scno = SyscallId::NBSend as seL4_Word;
    asm!("swi $0"
	:
	: "i" (swinum!(SyscallId::NBSend)),
	  "{r0}" (dest),
	  "{r1}" (msgInfo.words[0]),
	  "{r2}" (msg0), "{r3}" (msg1),
	  "{r4}" (msg2), "{r5}" (msg3),
	  "{r7}" (scno)
	: "memory", "r0", "r1", "r2", "r3", "r4", "r5", "r7"
    : "volatile");
}
#[inline(always)]
pub unsafe fn seL4_NBSendWithMRs(dest: seL4_CPtr, msgInfo: seL4_MessageInfo,
                                 mr0: *mut seL4_Word, mr1: *mut seL4_Word,
                                 mr2: *mut seL4_Word, mr3: *mut seL4_Word,
                                 ) {
	let mut msg0 = ::core::mem::uninitialized();
	let mut msg1 = ::core::mem::uninitialized();
	let mut msg2 = ::core::mem::uninitialized();
	let mut msg3 = ::core::mem::uninitialized();

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
	let scno = SyscallId::NBSend as seL4_Word;
    asm!("swi $0"
	:
	: "i" (swinum!(SyscallId::NBSend)),
	  "{r0}" (dest),
	  "{r1}" (msgInfo.words[0]),
	  "{r2}" (msg0), "{r3}" (msg1),
	  "{r4}" (msg2), "{r5}" (msg3),
	  "{r7}" (scno)
	: "memory", "r0", "r1", "r2", "r3", "r4", "r5", "r7"
    : "volatile");
}

#[inline(always)]
pub unsafe fn seL4_Reply(msgInfo: seL4_MessageInfo) {
	let msg0 = seL4_GetMR(0);
	let msg1 = seL4_GetMR(1);
	let msg2 = seL4_GetMR(2);
	let msg3 = seL4_GetMR(3);
	let scno = SyscallId::Reply as seL4_Word;
    asm!("swi $0"
	:
	: "i" (swinum!(SyscallId::Reply)),
	  "{r1}" (msgInfo.words[0]),
	  "{r2}" (msg0), "{r3}" (msg1),
	  "{r4}" (msg2), "{r5}" (msg3),
	  "{r7}" (scno)
	: "memory", "r0", "r1", "r2", "r3", "r4", "r5", "r7"
    : "volatile");
}
#[inline(always)]
pub unsafe fn seL4_ReplyWithMRs(msgInfo: seL4_MessageInfo,
                                mr0: *mut seL4_Word, mr1: *mut seL4_Word,
                                mr2: *mut seL4_Word, mr3: *mut seL4_Word,
                                ) {
	let mut msg0 = ::core::mem::uninitialized();
	let mut msg1 = ::core::mem::uninitialized();
	let mut msg2 = ::core::mem::uninitialized();
	let mut msg3 = ::core::mem::uninitialized();

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
	let scno = SyscallId::Reply as seL4_Word;
    asm!("swi $0"
	:
	: "i" (swinum!(SyscallId::Reply)),
	  "{r1}" (msgInfo.words[0]),
	  "{r2}" (msg0), "{r3}" (msg1),
	  "{r4}" (msg2), "{r5}" (msg3),
	  "{r7}" (scno)
	: "memory", "r0", "r1", "r2", "r3", "r4", "r5", "r7"
    : "volatile");
}


#[inline(always)]
pub unsafe fn seL4_Signal(dest: seL4_CPtr) {
	let info  = seL4_MessageInfo::new(0, 0, 0, 0).words[0];
	let scno = SyscallId::Send as seL4_Word;
    asm!("swi $0"
	:
	: "i" (swinum!(SyscallId::Send)),
	  "{r0}" (dest),
	  "{r1}" (info),
	  "{r7}" (scno)
	: "memory", "r0", "r1", "r7"
    : "volatile");
}

#[inline(always)]
pub unsafe fn seL4_Recv(mut src: seL4_CPtr, sender: *mut seL4_Word) -> seL4_MessageInfo {
	let mut info = seL4_MessageInfo { words: [0] };
	let mut msg0 = ::core::mem::uninitialized();
	let mut msg1 = ::core::mem::uninitialized();
	let mut msg2 = ::core::mem::uninitialized();
	let mut msg3 = ::core::mem::uninitialized();
	let scno = SyscallId::Recv as seL4_Word;
    asm!("swi $6"
	: "={r0}" (src),
	  "={r1}" (info.words[0]),
	  "={r2}" (msg0), "={r3}" (msg1),
	  "={r4}" (msg2), "={r5}" (msg3)
	: "i" (swinum!(SyscallId::Recv)),
	  "{r7}" (scno)
	: "memory"
    : "volatile");

    seL4_SetMR(0, msg0);
    seL4_SetMR(1, msg1);
    seL4_SetMR(2, msg2);
    seL4_SetMR(3, msg3);

    opt_assign!(sender, src);

    return info
}

#[inline(always)]
pub unsafe fn seL4_NBRecv(mut src: seL4_CPtr, sender: *mut seL4_Word) -> seL4_MessageInfo {
	let info: seL4_Word;
	let mut msg0 = ::core::mem::uninitialized();
	let mut msg1 = ::core::mem::uninitialized();
	let mut msg2 = ::core::mem::uninitialized();
	let mut msg3 = ::core::mem::uninitialized();
	let scno = SyscallId::NBRecv as seL4_Word;
    asm!("swi $6"
	: "={r0}" (src)
	  "={r1}" (info),
	  "={r2}" (msg0), "={r3}" (msg1),
	  "={r4}" (msg2), "={r5}" (msg3)
	: "i" (swinum!(SyscallId::NBRecv)),
	  "{r0}" (src),
	  "{r7}" (scno)
	: "memory", "r0", "r1", "r2", "r3", "r4", "r5", "r7"
    : "volatile");

    seL4_SetMR(0, msg0);
    seL4_SetMR(1, msg1);
    seL4_SetMR(2, msg2);
    seL4_SetMR(3, msg3);

    opt_assign!(sender, src);

    seL4_MessageInfo { words: [info] }
}

#[inline(always)]
pub unsafe fn seL4_Call(dest: seL4_CPtr, mut msgInfo: seL4_MessageInfo) -> seL4_MessageInfo {
	let mut msg0 = seL4_GetMR(0);
	let mut msg1 = seL4_GetMR(1);
	let mut msg2 = seL4_GetMR(2);
	let mut msg3 = seL4_GetMR(3);

	let scno = SyscallId::Call as seL4_Word;
    asm!("swi $5"
	: "={r1}" (msgInfo.words[0]),
	  "={r2}" (msg0), "={r3}" (msg1),
	  "={r4}" (msg2), "={r5}" (msg3)
	: "i" (swinum!(SyscallId::Call)),
	  "{r7}" (scno),
      "{r0}" (dest)
	: "memory", "r0", "r1", "r2", "r3", "r4", "r5", "r7"
        : "volatile");

    seL4_SetMR(0, msg0);
    seL4_SetMR(1, msg1);
    seL4_SetMR(2, msg2);
    seL4_SetMR(3, msg3);

    msgInfo
}

#[inline(always)]
pub unsafe fn seL4_CallWithMRs(dest: seL4_CPtr, mut msgInfo: seL4_MessageInfo,
                               mr0: *mut seL4_Word, mr1: *mut seL4_Word,
                               mr2: *mut seL4_Word, mr3: *mut seL4_Word,
                               ) -> seL4_MessageInfo {
	let mut msg0 = ::core::mem::uninitialized();
	let mut msg1 = ::core::mem::uninitialized();
	let mut msg2 = ::core::mem::uninitialized();
	let mut msg3 = ::core::mem::uninitialized();

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

	let scno = SyscallId::Call as seL4_Word;
    asm!("swi $5"
	: "={r1}" (msgInfo.words[0]),
	  "={r2}" (msg0), "={r3}" (msg1),
	  "={r4}" (msg2), "={r5}" (msg3)
	: "i" (swinum!(SyscallId::Call)),
	  "{r7}" (scno),
      "{r0}" (dest),
      "{r1}" (msgInfo.words[0])
	: "memory", "r0", "r1", "r2", "r3", "r4", "r5", "r7"
    : "volatile");

    opt_assign!(mr0, msg0);
    opt_assign!(mr1, msg1);
    opt_assign!(mr2, msg2);
    opt_assign!(mr3, msg3);

    msgInfo
}

#[inline(always)]
pub unsafe fn seL4_ReplyRecv(mut src: seL4_CPtr, mut msgInfo: seL4_MessageInfo,
                             sender: *mut seL4_Word) -> seL4_MessageInfo {
	let mut msg0 = seL4_GetMR(0);
	let mut msg1 = seL4_GetMR(1);
	let mut msg2 = seL4_GetMR(2);
	let mut msg3 = seL4_GetMR(3);

	let scno = SyscallId::ReplyRecv as seL4_Word;
    asm!("swi $6"
	: "={r0}" (src), "={r1}" (msgInfo.words[0]),
	  "={r2}" (msg0), "={r3}" (msg1),
	  "={r4}" (msg2), "={r5}" (msg3)
	: "i" (swinum!(SyscallId::ReplyRecv)),
	  "{r7}" (scno),
      "{r0}" (src),
      "{r1}" (msgInfo.words[0])
	: "memory", "r0", "r1", "r2", "r3", "r4", "r5", "r7"
    : "volatile");

    seL4_SetMR(0, msg0);
    seL4_SetMR(1, msg1);
    seL4_SetMR(2, msg2);
    seL4_SetMR(3, msg3);

    opt_assign!(sender, src);
    msgInfo
}

#[inline(always)]
pub unsafe fn seL4_ReplyRecvWithMRs(mut src: seL4_CPtr, mut msgInfo: seL4_MessageInfo,
                                    sender: *mut seL4_Word,
                                    mr0: *mut seL4_Word, mr1: *mut seL4_Word,
                                    mr2: *mut seL4_Word, mr3: *mut seL4_Word,
                                     ) -> seL4_MessageInfo {
	let mut msg0 = ::core::mem::uninitialized();
	let mut msg1 = ::core::mem::uninitialized();
	let mut msg2 = ::core::mem::uninitialized();
	let mut msg3 = ::core::mem::uninitialized();
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
	let scno = SyscallId::ReplyRecv as seL4_Word;
    asm!("swi $6"
	: "={r0}" (src), "={r1}" (msgInfo.words[0]),
	  "={r2}" (msg0), "={r3}" (msg1),
	  "={r4}" (msg2), "={r5}" (msg3)
	: "i" (swinum!(SyscallId::ReplyRecv)),
	  "{r7}" (scno),
      "{r0}" (src),
      "{r1}" (msgInfo.words[0])
	: "memory", "r0", "r1", "r2", "r3", "r4", "r5", "r7"
    : "volatile");

    opt_assign!(mr0, msg0);
    opt_assign!(mr1, msg1);
    opt_assign!(mr2, msg2);
    opt_assign!(mr3, msg3);

    opt_assign!(sender, src);
    msgInfo
}

#[inline(always)]
pub unsafe fn seL4_Yield() {
    let scno = SyscallId::Yield as seL4_Word;
    asm!("swi $0"
	:
	: "i" (swinum!(SyscallId::Yield)),
	  "{r7}" (scno)
	: "memory"
    : "volatile");
}

#[inline(always)]
pub unsafe fn seL4_DebugPutChar(c: u8) {
    let scno = SyscallId::DebugPutChar as seL4_Word;
    asm!("swi $0"
	:
	: "i" (swinum!(SyscallId::DebugPutChar)),
      "{r0}" (c)
	  "{r7}" (scno)
	: "memory"
    : "volatile");
}

#[inline(always)]
pub unsafe fn seL4_DebugHalt() {
    let scno = SyscallId::DebugHalt as seL4_Word;
    asm!("swi $0"
	:
	: "i" (swinum!(SyscallId::DebugHalt)),
	  "{r7}" (scno)
	: "memory"
    : "volatile");
}

#[inline(always)]
pub unsafe fn seL4_DebugSnapshot() {
    let scno = SyscallId::DebugSnapshot as seL4_Word;
    asm!("swi $0"
	:
	: "i" (swinum!(SyscallId::DebugSnapshot)),
	  "{r7}" (scno)
	: "memory"
    : "volatile");
}

#[inline(always)]
pub unsafe fn seL4_DebugCapIdentify(mut cap: seL4_CPtr) -> u32 {
    let scno = SyscallId::DebugCapIdentify as seL4_Word;
    asm!("swi $1"
	: "={r0}" (cap)
	: "i" (swinum!(SyscallId::DebugCapIdentify)),
      "{r0}" (cap),
	  "{r7}" (scno)
	: "memory"
    : "volatile");
    cap as _
}

// Note: name MUST be NUL-terminated.
#[inline(always)]
pub unsafe fn seL4_DebugNameThread(tcb: seL4_CPtr, name: &[u8]) {
    core::ptr::copy_nonoverlapping(name.as_ptr() as *mut u8, (&mut (*seL4_GetIPCBuffer()).msg).as_mut_ptr() as *mut u8,name.len());
    let scno = SyscallId::DebugNameThread as seL4_Word;
    asm!("swi $0"
	:
	: "i" (swinum!(SyscallId::DebugNameThread)),
      "{r0}" (tcb),
	  "{r7}" (scno)
	: "memory"
    : "volatile");
}

#[inline(always)]
#[cfg(feature = "SEL4_DANGEROUS_CODE_INJECTION")]
pub unsafe fn seL4_DebugRun(userfn: extern fn(*mut u8), userarg: *mut u8) {
    let userfnptr = userfn as *mut ();
    let scno = SyscallId::DebugRun as seL4_Word;
    asm!("swi $0"
	:
	: "i" (swinum!(SyscallId::DebugRun)),
      "{r0}" (userfnptr),
      "{r1}" (userarg),
	  "{r7}" (scno)
	: "memory"
    : "volatile");
}

#[inline(always)]
#[cfg(feature = "SEL4_CONFIG_BENCHMARK")]
pub unsafe fn seL4_BenchmarkResetLog() {
    let scno = SyscallId::BenchmarkResetLog as seL4_Word;
    asm!("swi $0"
	:
	: "i" (swinum!(SyscallId::BenchmarkResetLog)),
	  "{r7}" (scno)
	: "memory"
    : "volatile");
}

#[inline(always)]
#[cfg(feature = "SEL4_CONFIG_BENCHMARK")]
pub unsafe fn seL4_BenchmarkDumpLog(mut start: seL4_Word, size: seL4_Word) -> u32 {
    let scno = SyscallId::BenchmarkDumpLog as seL4_Word;
    asm!("swi $1"
	: "={r0}" (start)
	: "i" (swinum!(SyscallId::BenchmarkDumpLog)),
	  "{r0}" (start),
      "{r1}" (size),
	  "{r7}" (scno)
	: "memory"
    : "volatile");
    start
}

#[inline(always)]
#[cfg(feature = "SEL4_CONFIG_BENCHMARK")]
pub unsafe fn seL4_BenchmarkLogSize() -> u32 {
    let mut size = 0;
    let scno = SyscallId::BenchmarkLogSize as seL4_Word;
    asm!("swi $1"
	: "={r0}" (size)
	: "i" (swinum!(SyscallId::BenchmarkLogSize)),
	  "{r7}" (scno)
	: "memory"
    : "volatile");
    size
}

#[inline(always)]
#[cfg(feature = "SEL4_CONFIG_BENCHMARK")]
pub unsafe fn seL4_BenchmarkFinalizeLog() {
    let scno = SyscallId::BenchmarkFinalizeLog as seL4_Word;
    asm!("swi $0"
	:
	: "i" (swinum!(SyscallId::BenchmarkFinalizeLog)),
	  "{r7}" (scno)
	: "memory"
    : "volatile");
}
