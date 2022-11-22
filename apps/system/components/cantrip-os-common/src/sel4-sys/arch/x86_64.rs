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

pub const seL4_WordBits: usize = 64;
pub const seL4_PageBits: usize = 12;
pub const seL4_SlotBits: usize = 5;
pub const seL4_TCBBits: usize = 11;
pub const seL4_EndpointBits: usize = 4;
#[cfg(not(feature = "CONFIG_KERNEL_MCS"))]
pub const seL4_NotificationBits: usize = 5;
#[cfg(feature = "CONFIG_KERNEL_MCS")]
pub const seL4_NotificationBits: usize = 6;
pub const seL4_ReplyBits: usize = 6; // Only relevant when CONFIG_KERNEL_MCS is enabled
pub const seL4_PageTableBits: usize = 12;
pub const seL4_PageDirBits: usize = 12;
pub const seL4_PDPTBits: usize = 12;
pub const seL4_PML4Bits: usize = 12;
pub const seL4_IOPageTableBits: usize = 12;
pub const seL4_LargePageBits: usize = 21;
pub const seL4_HugePageBits: usize = 30;

pub const seL4_VCPUBits: usize = 14;
pub const seL4_EPTPTBits: usize = 12;
pub const seL4_EPTPDBits: usize = 12;
pub const seL4_EPTPDPTBits: usize = 12;
pub const seL4_EPTPML4Bits: usize = 12;

pub const seL4_ASIDPoolBits: usize = 12;

pub const seL4_MinUntypedBits: usize = 4;
pub const seL4_MaxUntypedBits: usize = 47;

pub const seL4_NumHWBreakpoints: usize = 4;
pub const seL4_FirstBreakpoint: usize = !1;
pub const seL4_NumExclusiveBreakpoints: usize = 0;
pub const seL4_FirstWatchpoint: usize = !1;
pub const seL4_NumExclusiveWatchpoints: usize = 0;
pub const seL4_FirstDualFunctionMonitor: usize = 0;
pub const seL4_NumDualFunctionMonitors: usize = 4;

pub type seL4_X64_PML4 = seL4_CPtr; // XXX X86 v X64
pub type seL4_X86_ASIDControl = seL4_CPtr;
pub type seL4_X86_ASIDPool = seL4_CPtr;
pub type seL4_X86_IOPageTable = seL4_CPtr;
pub type seL4_X86_IOPortControl = seL4_CPtr;
pub type seL4_X86_IOPort = seL4_CPtr;
pub type seL4_X86_IOSpace = seL4_CPtr;
pub type seL4_X86_PageDirectory = seL4_CPtr;
pub type seL4_X86_Page = seL4_CPtr;
pub type seL4_X86_PageTable = seL4_CPtr;
pub type seL4_X86_PDPT = seL4_CPtr;

#[cfg(feature = "arch_generic")]
include!("x86_generic.rs");

error_types!(u64);

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

    seL4_X86_PDPTObject,
    seL4_X64_PML4Object,
    #[cfg(feature = "CONFIG_HUGE_PAGE")]
    seL4_X64_HugePageObject,

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
    pub rip: seL4_Word,
    pub rsp: seL4_Word,
    pub rflags: seL4_Word,
    pub rax: seL4_Word,
    pub rbx: seL4_Word,
    pub rcx: seL4_Word,
    pub rdx: seL4_Word,
    pub rsi: seL4_Word,
    pub rdi: seL4_Word,
    pub rbp: seL4_Word,
    pub r8: seL4_Word,
    pub r9: seL4_Word,
    pub r10: seL4_Word,
    pub r11: seL4_Word,
    pub r12: seL4_Word,
    pub r13: seL4_Word,
    pub r14: seL4_Word,
    pub r15: seL4_Word,
    pub fs_base: seL4_Word,
    pub gs_base: seL4_Word,
}

#[repr(C, packed)]
pub struct seL4_VBEInfoBlock {
    pub signature: [u8; 4],
    pub version: u16,
    pub oemStringPtr: u32,
    pub capabilities: u32,
    pub modeListPtr: u32,
    pub totalMemory: u16,
    pub oemSoftwareRev: u16,
    pub oemVendorNamePtr: u32,
    pub oemProductNamePtr: u32,
    pub reserved: [u8; 222],
    pub oemData: [u8; 256],
}

#[repr(C, packed)]
pub struct seL4_VBEModeInfoBlock {
    // all revisions
    pub modeAttr: u16,
    pub winAAttr: u8,
    pub winBAttr: u8,
    pub winGranularity: u16,
    pub winSize: u16,
    pub winASeg: u16,
    pub winBSeg: u16,
    pub winFuncPtr: u32,
    pub bytesPerScanLine: u16,

    // 1.2+
    pub xRes: u16,
    pub yRes: u16,
    pub xCharSize: u8,
    pub yCharSize: u8,
    pub planes: u8,
    pub bitsPerPixel: u8,
    pub banks: u8,
    pub memoryMmodel: u8,
    pub bankSize: u8,
    pub imagePages: u8,
    pub reserved1: u8,

    pub redLen: u8,
    pub redOff: u8,
    pub greenLen: u8,
    pub greenOff: u8,
    pub blueLen: u8,
    pub blueOff: u8,
    pub rsvdLen: u8,
    pub rsvdOff: u8,
    pub directColorInfo: u8,

    // 2.0+
    pub physBasePtr: u32,
    pub reserved2: [u8; 6],

    // 3.0+
    pub linBytesPerScanLine: u16,
    pub bnkImagePages: u8,
    pub linImagePages: u8,
    pub linRedLen: u8,
    pub linRedOff: u8,
    pub linGreenLen: u8,
    pub linGreenOff: u8,
    pub linBlueLen: u8,
    pub linBlueOff: u8,
    pub linRsvdLen: u8,
    pub linRsvdOff: u8,
    pub maxPixelClock: u32,
    pub modeId: u16,
    pub depth: u8,

    pub reserved3: [u8; 187],
}

#[repr(C, packed)]
pub struct seL4_X86_BootInfo_VBE {
    pub header: seL4_BootInfoHeader,
    pub vbeInfoBlock: seL4_VBEInfoBlock,
    pub vbeModeInfoBlock: seL4_VBEModeInfoBlock,
    pub vbeMode: u32,
    pub vbeInterfaceSeg: u32,
    pub vbeInterfaceOff: u32,
    pub vbeInterfaceLen: u32,
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

#[inline(always)]
pub unsafe fn seL4_Send(dest: seL4_CPtr, msgInfo: seL4_MessageInfo) {
    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::Send),
        in("rdi") dest,
        in("rsi") msgInfo.words[0],
        in("r10") seL4_GetMR(0),
        in("r8") seL4_GetMR(1),
        in("r9") seL4_GetMR(2),
        in("r15") seL4_GetMR(3),
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _);
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

    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::Send),
        in("rdi") dest,
        in("rsi") msgInfo.words[0],
        in("r10") msg0,
        in("r8") msg1,
        in("r9") msg2,
        in("r15") msg3,
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _);
}

#[inline(always)]
pub unsafe fn seL4_NBSend(dest: seL4_CPtr, msgInfo: seL4_MessageInfo) {
    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::NBSend),
        in("rdi") dest,
        in("rsi") msgInfo.words[0],
        in("r10") seL4_GetMR(0),
        in("r8") seL4_GetMR(1),
        in("r9") seL4_GetMR(2),
        in("r15") seL4_GetMR(3),
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _);
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

    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::NBSend),
        in("rdi") dest,
        in("rsi") msgInfo.words[0],
        in("r10") msg0,
        in("r8") msg1,
        in("r9") msg2,
        in("r15") msg3,
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _);
}

#[cfg(not(feature = "CONFIG_KERNEL_MCS"))]
#[inline(always)]
pub unsafe fn seL4_Reply(msgInfo: seL4_MessageInfo) {
    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::Reply),
        in("rsi") msgInfo.words[0],
        in("r10") seL4_GetMR(0),
        in("r8") seL4_GetMR(1),
        in("r9") seL4_GetMR(2),
        in("r15") seL4_GetMR(3),
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _);
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

    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::Reply),
        in("rsi") msgInfo.words[0],
        in("r10") msg0,
        in("r8") msg1,
        in("r9") msg2,
        in("r15") msg3,
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _);
}

#[inline(always)]
pub unsafe fn seL4_Signal(dest: seL4_CPtr) {
    let info = seL4_MessageInfo::new(0, 0, 0, 0).words[0];
    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::Send),
        in("rdi") dest,
        in("rsi") info,
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _);
}

#[inline(always)]
pub unsafe fn seL4_Recv(mut src: seL4_CPtr, sender: *mut seL4_Word) -> seL4_MessageInfo {
    let mut info: seL4_Word;
    let mut msg0 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg1 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg2 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg3 = ::core::mem::MaybeUninit::uninit().assume_init();

    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::Recv),
        out("rdi") src,
        out("rsi") info,
        out("r10") msg0,
        out("r8") msg1,
        out("r9") msg2,
        out("r15") msg3,
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _);

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

    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::NBRecv),
        inout("rdi") src,
        out("rsi") info,
        out("r10") msg0,
        out("r8") msg1,
        out("r9") msg2,
        out("r15") msg3,
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _);

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

    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::Call),
        in("rdi") dest,
        inout("rsi") msgInfo.words[0] => info,
        inout("r10") msg0,
        inout("r8") msg1,
        inout("r9") msg2,
        inout("r15") msg3,
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _);

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

    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::Call),
        in("rdi") dest,
        inout("rsi") msgInfo.words[0] => info,
        inout("r10") msg0,
        inout("r8") msg1,
        inout("r9") msg2,
        inout("r15") msg3,
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _);

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

    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::ReplyRecv),
        inout("rdi") src,
        inout("rsi") msgInfo.words[0] => info,
        inout("r10") msg0,
        inout("r8") msg1,
        inout("r9") msg2,
        inout("r15") msg3,
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _);

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

    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::ReplyRecv),
        inout("rdi") src => badge,
        inout("rsi") msgInfo.words[0] => info,
        inout("r10") msg0,
        inout("r8") msg1,
        inout("r9") msg2,
        inout("r15") msg3,
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _);

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

    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::NBSendRecv),
        inout("rdi") src => badge,
        inout("rsi") msgInfo.words[0] => info,
        inout("r10") msg0,
        inout("r8") msg1,
        inout("r9") msg2,
        inout("r15") msg3,
        in("r12") reply,
        in("r13") dest,
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _);

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

    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::NBSendRecv),
        inout("rdi") src => badge,
        inout("rsi") msgInfo.words[0] => info,
        inout("r10") msg0,
        inout("r8") msg1,
        inout("r9") msg2,
        inout("r15") msg3,
        in("r12") reply,
        in("r13") dest,
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _);

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

    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::NBSendWait),
        inout("rdi") src => badge,
        inout("rsi") msgInfo.words[0] => info,
        inout("r10") msg0,
        inout("r8") msg1,
        inout("r9") msg2,
        inout("r15") msg3,
        in("r12") dest,
        in("r13") 0,
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _);

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

    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::NBSendRecv),
        inout("rdi") src => badge,
        inout("rsi") msgInfo.words[0] => info,
        inout("r10") msg0,
        inout("r8") msg1,
        inout("r9") msg2,
        inout("r15") msg3,
        in("r12") dest,
        in("r13") 0,
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _);

    opt_assign!(mr0, msg0);
    opt_assign!(mr1, msg1);
    opt_assign!(mr2, msg2);
    opt_assign!(mr3, msg3);

    opt_assign!(sender, badge);

    seL4_MessageInfo { words: [info] }
}

#[inline(always)]
pub unsafe fn seL4_Yield() {
    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::Yield),
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _,
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

    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::Wait),
        inout("rdi") src => badge,
        out("rsi") info,
        inout("r10") msg0,
        inout("r8") msg1,
        inout("r9") msg2,
        inout("r15") msg3,
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _);

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

    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::Wait),
        inout("rdi") src => badge,
        out("rsi") info,
        inout("r10") msg0,
        inout("r8") msg1,
        inout("r9") msg2,
        inout("r15") msg3,
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _);

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

    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::NBWait),
        inout("rdi") src => badge,
        out("rsi") info,
        inout("r10") msg0,
        inout("r8") msg1,
        inout("r9") msg2,
        inout("r15") msg3,
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _);

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
    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::DebugPutChar),
        in("rdi") c as i16,
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _,
        options(nostack),
    );
}

#[cfg(feature = "CONFIG_PRINTING")]
#[inline(always)]
pub unsafe fn seL4_DebugDumpScheduler() {
    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::DebugDumpScheduler),
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _,
        options(nomem, nostack),
    );
}

#[cfg(feature = "CONFIG_DEBUG_BUILD")]
#[inline(always)]
pub unsafe fn seL4_DebugHalt() {
    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::DebugHalt),
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _,
        options(nomem, nostack),
    );
}

#[cfg(feature = "CONFIG_DEBUG_BUILD")]
#[inline(always)]
pub unsafe fn seL4_DebugSnapshot() {
    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::DebugSnapshot),
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _,
        options(nomem, nostack),
    );
}

#[cfg(feature = "CONFIG_DEBUG_BUILD")]
#[inline(always)]
pub unsafe fn seL4_DebugCapIdentify(mut cap: seL4_CPtr) -> u32 {
    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::DebugCapIdentify),
        inout("rdi") cap,
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _,
        options(nomem, nostack),
    );
    cap as _
}

#[cfg(feature = "CONFIG_PRINTING")]
#[inline(always)]
pub unsafe fn seL4_DebugDumpCNode(mut cap: seL4_CPtr) {
    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::DebugDumpCNode),
        inout("rdi") cap,
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _,
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
    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::DebugNameThread),
        in("rdi") tcb,
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _,
    );
}

#[cfg(feature = "CONFIG_DANGEROUS_CODE_INJECTION")]
#[inline(always)]
pub unsafe fn seL4_DebugRun(userfn: extern "C" fn(*mut u8), userarg: *mut u8) {
    let userfnptr = userfn as *mut ();
    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::DebugRun),
        in("rdi") userfnptr,
        in("r10") userfnptr,
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _,
    );
}

#[cfg(feature = "CONFIG_ENABLE_BENCHMARKS")]
#[inline(always)]
pub unsafe fn seL4_BenchmarkResetLog() {
    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::BenchmarkResetLog),
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _,
        options(nomem, nostack),
    );
}

#[cfg(feature = "CONFIG_ENABLE_BENCHMARKS")]
#[inline(always)]
pub unsafe fn seL4_BenchmarkFinalizeLog() {
    asm!("mov r14, rsp
          syscall
          mov rsp, r14",
        in("rdx") swinum!(SyscallId::BenchmarkFinalizeLog),
        lateout("rcx") _,
        lateout("r11") _,
        lateout("r14") _,
        options(nomem, nostack),
    );
}
