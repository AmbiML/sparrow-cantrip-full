
use core::mem::uninitialized;

pub const seL4_WordBits: usize = 64;
pub const seL4_PageBits: usize = 12;
pub const seL4_SlotBits: usize = 5;
pub const seL4_TCBBits: usize = 11;
pub const seL4_EndpointBits: usize = 4;
pub const seL4_NotificationBits: usize = 5;
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


pub type seL4_X86_ASIDControl = seL4_CPtr;
pub type seL4_X86_ASIDPool = seL4_CPtr;
pub type seL4_X86_IOSpace = seL4_CPtr;
pub type seL4_X86_IOPort = seL4_CPtr;
pub type seL4_X86_Page = seL4_CPtr;
pub type seL4_X86_PageDirectory = seL4_CPtr;
pub type seL4_X86_PageTable = seL4_CPtr;
pub type seL4_X86_IOPageTable = seL4_CPtr;
pub type seL4_X86_PDPT = seL4_CPtr;
pub type seL4_X64_PML4 = seL4_CPtr;

error_types!(u64);

pub const Default_VMAttributes: usize = 0;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum seL4_ObjectType {
    seL4_UntypedObject = 0,
    seL4_TCBObject,
    seL4_EndpointObject,
    seL4_NotificationObject,
    seL4_CapTableObject,
    seL4_X86_PDPTObject,
    seL4_X64_PML4Object,
    // seL4_X64_HugePageObject,
    seL4_X86_4K,
    seL4_X86_LargePageObject,
    seL4_X86_PageTableObject,
    seL4_X86_PageDirectoryObject,
    seL4_X86_IOPageTableObject,
    seL4_X86_VCPUObject,
    seL4_X86_EPTPML4Object,
    seL4_X86_EPTPDPTObject,
    seL4_X86_EPTPDObject,
    seL4_X86_EPTPTObject,
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
    pub tls_base: seL4_Word,
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
pub unsafe fn seL4_GetMR(regnum: isize) -> seL4_Word {
    let mr;
    asm!("movq %gs:8(,$1,0x8), $0" : "=r"(mr) : "r"(regnum) : : "volatile");
    mr
}

#[inline(always)]
pub unsafe fn seL4_SetMR(regnum: isize, value: seL4_Word) {
    asm!("movq $0, %gs:8(,$1,0x8)" : : "r"(value), "r"(regnum) : "memory" : "volatile");
}

#[inline(always)]
pub unsafe fn seL4_GetUserData() -> seL4_Word {
    let data;
    asm!("movq %gs:968, $0" : "=r"(data) : : : "volatile");
    data
}

#[inline(always)]
pub unsafe fn seL4_GetIPCBuffer() -> *mut seL4_IPCBuffer {
    seL4_GetUserData() as isize as *mut seL4_IPCBuffer
}

#[inline(always)]
pub unsafe fn seL4_SetUserData(data: seL4_Word) {
    asm!("movq $0, %gs:968" : : "r"(data) : "memory" : "volatile");
}

#[inline(always)]
pub unsafe fn seL4_GetBadge(index: isize) -> seL4_CapData {
    let mut badge: seL4_CapData = uninitialized();
    asm!("movq %gs:976(,$1,0x8), $0" : "=r"(badge.words[0]) : "r"(index) : : "volatile");
    badge
}

#[inline(always)]
pub unsafe fn seL4_GetCap(index: isize) -> seL4_CPtr {
    let cptr;
    asm!("movq %gs:976(,$1,0x8), $0" : "=r"(cptr) : "r"(index) : : "volatile");
    cptr
}

#[inline(always)]
pub unsafe fn seL4_SetCap(index: isize, cptr: seL4_CPtr) {
    asm!("movq $0, %gs:976(,$1,0x8)" : : "r"(cptr), "r"(index) : "memory" : "volatile");
}

#[inline(always)]
pub unsafe fn seL4_GetCapReceivePath(receiveCNode: *mut seL4_CPtr,
                                     receiveIndex: *mut seL4_CPtr,
                                     receiveDepth: *mut seL4_Word) {
    if !receiveCNode.is_null() {
        asm!("movq %gs:1000, $0" : "=r"(*receiveCNode) : : : "volatile");
    }

    if !receiveIndex.is_null() {
        asm!("movq %gs:1008, $0" : "=r"(*receiveIndex) : : : "volatile");
    }

    if !receiveDepth.is_null() {
        asm!("movq %gs:1016, $0" : "=r"(*receiveDepth) : : : "volatile");
    }
}

#[inline(always)]
pub unsafe fn seL4_SetCapReceivePath(receiveCNode: seL4_CPtr,
                                     receiveIndex: seL4_CPtr,
                                     receiveDepth: seL4_Word) {
    asm!("movq $0, %gs:1000" : : "r"(receiveCNode) : "memory" : "volatile");
    asm!("movq $0, %gs:1008" : : "r"(receiveIndex) : "memory" : "volatile");
    asm!("movq $0, %gs:1016" : : "r"(receiveDepth) : "memory" : "volatile");
}

#[inline(always)]
unsafe fn x64_sys_send(sys: seL4_Word, dest: seL4_Word, info: seL4_Word, mr1: seL4_Word, mr2: seL4_Word, mr3: seL4_Word, mr4: seL4_Word) {
    asm!("movq %rsp, %rbx
          syscall
          movq %rbx, %rsp"
          : 
          : "{rdx}" (sys),
            "{rdi}" (dest),
            "{rsi}" (info),
            "{r10}" (mr1),
            "{r8}" (mr2),
            "{r9}" (mr3),
            "{r15}" (mr4)
          : "%rcx", "%rbx", "%r11"
          : "volatile");
}

#[inline(always)]
unsafe fn x64_sys_reply(sys: seL4_Word, info: seL4_Word, mr1: seL4_Word, mr2: seL4_Word, mr3: seL4_Word, mr4: seL4_Word) {
    asm!("movq %rsp, %rbx
          syscall
          movq %rbx, %rsp"
          :
          : "{rdx}" (sys),
            "{rsi}" (info),
            "{r10}" (mr1),
            "{r8}" (mr2),
            "{r9}" (mr3),
            "{r15}" (mr4)
          : "%rcx", "%rbx", "%r11"
          : "volatile");
}

#[inline(always)]
unsafe fn x64_sys_send_null(sys: seL4_Word, dest: seL4_Word, info: seL4_Word) {
    asm!("movq %rsp, %rbx
          syscall
          movq %rbx, %rsp"
          : 
          : "{rdx}" (sys),
            "{rdi}" (dest),
            "{rsi}" (info)
          : "%rcx", "%rbx", "%r11"
          : "volatile");
}

#[inline(always)]
unsafe fn x64_sys_recv(sys: seL4_Word, src: seL4_Word, out_badge: *mut seL4_Word, out_info: *mut seL4_Word, out_mr1: *mut seL4_Word, out_mr2: *mut seL4_Word, out_mr3: *mut seL4_Word, out_mr4: *mut seL4_Word) {
    asm!("movq %rsp, %rbx
          syscall
          movq %rbx, %rsp"
          : "={rsi}" (*out_info)
            "={r10}" (*out_mr1),
            "={r8}" (*out_mr2),
            "={r9}" (*out_mr3),
            "={r15}" (*out_mr4),
            "={rdi}" (*out_badge)
          : "{rdx}" (sys),
            "{rdi}" (src)
          : "memory", "%rcx", "%rbx", "%r11"
          : "volatile");
}

#[inline(always)]
unsafe fn x64_sys_send_recv(sys: seL4_Word, dest: seL4_Word, out_dest: *mut seL4_Word, info: seL4_Word, out_info: *mut seL4_Word, in_out_mr1: *mut seL4_Word, in_out_mr2: *mut seL4_Word, in_out_mr3: *mut seL4_Word, in_out_mr4: *mut seL4_Word) {
    asm!("movq %rsp, %rbx
          syscall
          movq %rbx, %rsp"
          : "={rsi}" (*out_info)
            "={r10}" (*in_out_mr1),
            "={r8}" (*in_out_mr2),
            "={r9}" (*in_out_mr3),
            "={r15}" (*in_out_mr4),
            "={rdi}" (*out_dest)
          : "{rdx}" (sys),
            "{rsi}" (info),
            "{r10}" (*in_out_mr1),
            "{r8}" (*in_out_mr2),
            "{r9}" (*in_out_mr3),
            "{r15}" (*in_out_mr4),
            "{rdi}" (dest)
          : "memory", "%rcx", "%rbx", "%r11"
          : "volatile");
}

#[inline(always)]
unsafe fn x64_sys_null(sys: seL4_Word) {
    asm!("movq %rsp, %rbx
          syscall
          movq %rbx, %rsp"
          :
          : "{rdx}" (sys)
          : "%rcx", "%rbx", "%r11", "%rsi", "%rdi"
          : "volatile");
}

#[inline(always)]
pub unsafe fn seL4_Send(dest: seL4_CPtr, msgInfo: seL4_MessageInfo) {
    x64_sys_send(SyscallId::Send as seL4_Word, dest, msgInfo.words[0], seL4_GetMR(0), seL4_GetMR(1), seL4_GetMR(2), seL4_GetMR(3));
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
                               mr2: *mut seL4_Word, mr3: *mut seL4_Word) {
    x64_sys_send(SyscallId::Send as seL4_Word, dest, msgInfo.words[0],
                 if mr0.is_null() { 0 } else { *mr0 },
                 if mr1.is_null() { 0 } else { *mr1 },
                 if mr2.is_null() { 0 } else { *mr2 },
                 if mr3.is_null() { 0 } else { *mr3 },
                 );
}

#[inline(always)]
pub unsafe fn seL4_NBSend(dest: seL4_CPtr, msgInfo: seL4_MessageInfo) {
    x64_sys_send(SyscallId::NBSend as seL4_Word, dest, msgInfo.words[0], seL4_GetMR(0), seL4_GetMR(1), seL4_GetMR(2), seL4_GetMR(3));
}

#[inline(always)]
pub unsafe fn seL4_NBSendWithMRs(dest: seL4_CPtr, msgInfo: seL4_MessageInfo,
                                 mr0: *mut seL4_Word, mr1: *mut seL4_Word,
                                 mr2: *mut seL4_Word, mr3: *mut seL4_Word) {
    x64_sys_send(SyscallId::NBSend as seL4_Word, dest, msgInfo.words[0],
                 if mr0.is_null() { 0 } else { *mr0 },
                 if mr1.is_null() { 0 } else { *mr1 },
                 if mr2.is_null() { 0 } else { *mr2 },
                 if mr3.is_null() { 0 } else { *mr3 });
}

#[inline(always)]
pub unsafe fn seL4_Reply(msgInfo: seL4_MessageInfo) {
    x64_sys_reply(SyscallId::Reply as seL4_Word, msgInfo.words[0], seL4_GetMR(0), seL4_GetMR(1), seL4_GetMR(2), seL4_GetMR(3));

}
#[inline(always)]
pub unsafe fn seL4_ReplyWithMRs(msgInfo: seL4_MessageInfo,
                                mr0: *mut seL4_Word, mr1: *mut seL4_Word,
                                mr2: *mut seL4_Word, mr3: *mut seL4_Word) {
    x64_sys_reply(SyscallId::Reply as seL4_Word, msgInfo.words[0],
                  if mr0.is_null() { 0 } else { *mr0 },
                  if mr1.is_null() { 0 } else { *mr1 },
                  if mr2.is_null() { 0 } else { *mr2 },
                  if mr3.is_null() { 0 } else { *mr3 });
}


#[inline(always)]
pub unsafe fn seL4_Signal(dest: seL4_CPtr) {
    x64_sys_send_null(SyscallId::Send as seL4_Word, dest, seL4_MessageInfo::new(0,0,0,0).words[0]);
}

#[inline(always)]
pub unsafe fn seL4_Recv(src: seL4_CPtr, sender: *mut seL4_Word) -> seL4_MessageInfo {
    let mut info: seL4_MessageInfo = uninitialized();
    let mut badge: seL4_Word = uninitialized();
    let mut mr0: seL4_Word = uninitialized();
    let mut mr1: seL4_Word = uninitialized();
    let mut mr2: seL4_Word = uninitialized();
    let mut mr3: seL4_Word = uninitialized();

    x64_sys_recv(SyscallId::Recv as seL4_Word, src, &mut badge, &mut info.words[0], &mut mr0, &mut mr1, &mut mr2, &mut mr3);

    seL4_SetMR(0, mr0);
    seL4_SetMR(1, mr1);
    seL4_SetMR(2, mr2);
    seL4_SetMR(3, mr3);

    opt_assign!(sender, badge);

    info
}

#[inline(always)]
pub unsafe fn seL4_RecvWithMRs(src: seL4_CPtr, sender: *mut seL4_Word,
                               mr0: *mut seL4_Word, mr1: *mut seL4_Word,
                               mr2: *mut seL4_Word, mr3: *mut seL4_Word) -> seL4_MessageInfo {
    let mut info: seL4_MessageInfo = uninitialized();
    let mut badge: seL4_Word = uninitialized();
    let mut msg0: seL4_Word = uninitialized();
    let mut msg1: seL4_Word = uninitialized();
    let mut msg2: seL4_Word = uninitialized();
    let mut msg3: seL4_Word = uninitialized();

    x64_sys_recv(SyscallId::Recv as seL4_Word, src, &mut badge, &mut info.words[0], &mut msg0, &mut msg1, &mut msg2, &mut msg3);

    opt_assign!(mr0, msg0);
    opt_assign!(mr1, msg1);
    opt_assign!(mr2, msg2);
    opt_assign!(mr3, msg3);

    opt_assign!(sender, badge);

    info
}

#[inline(always)]
pub unsafe fn seL4_NBRecv(src: seL4_CPtr, sender: *mut seL4_Word) -> seL4_MessageInfo {
    let mut info: seL4_MessageInfo = uninitialized();
    let mut badge: seL4_Word = uninitialized();
    let mut mr0: seL4_Word = uninitialized();
    let mut mr1: seL4_Word = uninitialized();
    let mut mr2: seL4_Word = uninitialized();
    let mut mr3: seL4_Word = uninitialized();

    x64_sys_recv(SyscallId::NBRecv as seL4_Word, src, &mut badge, &mut info.words[0], &mut mr0, &mut mr1, &mut mr2, &mut mr3);

    seL4_SetMR(0, mr0);
    seL4_SetMR(1, mr1);
    seL4_SetMR(2, mr2);
    seL4_SetMR(3, mr3);

    opt_assign!(sender, badge);

    info
}

#[inline(always)]
pub unsafe fn seL4_Call(mut dest: seL4_CPtr, msgInfo: seL4_MessageInfo) -> seL4_MessageInfo {
    let mut info: seL4_MessageInfo = uninitialized();
    let mut mr0 = seL4_GetMR(0);
    let mut mr1 = seL4_GetMR(1);
    let mut mr2 = seL4_GetMR(2);
    let mut mr3 = seL4_GetMR(3);

    x64_sys_send_recv(SyscallId::Call as seL4_Word, dest, &mut dest, msgInfo.words[0], &mut info.words[0], &mut mr0, &mut mr1, &mut mr2, &mut mr3);

    seL4_SetMR(0, mr0);
    seL4_SetMR(1, mr1);
    seL4_SetMR(2, mr2);
    seL4_SetMR(3, mr3);

    info
}

#[inline(always)]
pub unsafe fn seL4_CallWithMRs(mut dest: seL4_CPtr, msgInfo: seL4_MessageInfo,
                               mr0: *mut seL4_Word, mr1: *mut seL4_Word,
                               mr2: *mut seL4_Word, mr3: *mut seL4_Word) -> seL4_MessageInfo {
    let mut info: seL4_MessageInfo = uninitialized();
    let mut msg0: seL4_Word = 0;
    let mut msg1: seL4_Word = 0;
    let mut msg2: seL4_Word = 0;
    let mut msg3: seL4_Word = 0;

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
    if !mr2.is_null() {
        if msgInfo.get_length() > 2 {
            msg2 = *mr2;
        }
    }
    if !mr3.is_null() {
        if msgInfo.get_length() > 3 {
            msg3 = *mr3;
        }
    }

    x64_sys_send_recv(SyscallId::Call as seL4_Word, dest, &mut dest, msgInfo.words[0], &mut info.words[0], &mut msg0, &mut msg1, &mut msg2, &mut msg3);

    opt_assign!(mr0, msg0);
    opt_assign!(mr1, msg1);
    opt_assign!(mr2, msg2);
    opt_assign!(mr3, msg3);

    info
}

#[inline(always)]
pub unsafe fn seL4_ReplyRecv(dest: seL4_CPtr, msgInfo: seL4_MessageInfo,
                             sender: *mut seL4_Word) -> seL4_MessageInfo {
    let mut info: seL4_MessageInfo = uninitialized();
    let mut badge: seL4_Word = uninitialized();
    let mut mr0 = seL4_GetMR(0);
    let mut mr1 = seL4_GetMR(1);
    let mut mr2 = seL4_GetMR(2);
    let mut mr3 = seL4_GetMR(3);

    x64_sys_send_recv(SyscallId::ReplyRecv as seL4_Word, dest, &mut badge, msgInfo.words[0], &mut info.words[0], &mut mr0, &mut mr1, &mut mr2, &mut mr3);

    seL4_SetMR(0, mr0);
    seL4_SetMR(1, mr1);
    seL4_SetMR(2, mr2);
    seL4_SetMR(3, mr3);

    opt_assign!(sender, badge);

    info
}

#[inline(always)]
pub unsafe fn seL4_ReplyWaitWithMRs(dest: seL4_CPtr, msgInfo: seL4_MessageInfo, sender: *mut seL4_Word,
                                     mr0: *mut seL4_Word, mr1: *mut seL4_Word,
                                     mr2: *mut seL4_Word, mr3: *mut seL4_Word) -> seL4_MessageInfo {
    let mut info: seL4_MessageInfo = uninitialized();
    let mut badge: seL4_Word = uninitialized();
    let mut msg0: seL4_Word = 0;
    let mut msg1: seL4_Word = 0;
    let mut msg2: seL4_Word = 0;
    let mut msg3: seL4_Word = 0;

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
    if !mr2.is_null() {
        if msgInfo.get_length() > 2 {
            msg2 = *mr2;
        }
    }
    if !mr3.is_null() {
        if msgInfo.get_length() > 3 {
            msg3 = *mr3;
        }
    }

    x64_sys_send_recv(SyscallId::ReplyRecv as seL4_Word, dest, &mut badge, msgInfo.words[0], &mut info.words[0], &mut msg0, &mut msg1, &mut msg2, &mut msg3);

    opt_assign!(mr0, msg0);
    opt_assign!(mr1, msg1);
    opt_assign!(mr2, msg2);
    opt_assign!(mr3, msg3);

    opt_assign!(sender, badge);

    info
}

#[inline(always)]
pub unsafe fn seL4_Yield() {
    x64_sys_null(SyscallId::Yield as seL4_Word);
    asm!("" ::: "memory" : "volatile");
}

#[inline(always)]
pub unsafe fn seL4_VMEnter(vcpu: seL4_CPtr, sender: *mut seL4_Word) -> seL4_Word {
    let mut fault: seL4_Word = uninitialized();
    let mut badge: seL4_Word = uninitialized();
    let mut mr0 = seL4_GetMR(0);
    let mut mr1 = seL4_GetMR(1);
    let mut mr2 = seL4_GetMR(2);
    let mut mr3 = seL4_GetMR(3);

    x64_sys_send_recv(SyscallId::VMEnter as seL4_Word, vcpu, &mut badge, 0, &mut fault, &mut mr0, &mut mr1, &mut mr2, &mut mr3);

    seL4_SetMR(0, mr0);
    seL4_SetMR(1, mr1);
    seL4_SetMR(2, mr2);
    seL4_SetMR(3, mr3);

    if fault == 0 && !sender.is_null() {
        *sender = badge;
    }

    fault
}

//#[inline(always)]
pub unsafe fn seL4_DebugPutChar(c: u8) {
    let mut unused0 = 0;
    let mut unused1 = 0;
    let mut unused2 = 0;
    let mut unused3 = 0;
    let mut unused4 = 0;
    let mut unused5 = 0;
    x64_sys_send_recv(SyscallId::DebugPutChar as seL4_Word, c as seL4_Word, &mut unused0, 0, &mut
                      unused1, &mut unused2, &mut unused3, &mut unused4, &mut unused5);
}

#[inline(always)]
pub unsafe fn seL4_DebugHalt() {
    x64_sys_null(SyscallId::DebugHalt as seL4_Word);
    asm!("" ::: "memory" : "volatile");
}

#[inline(always)]
pub unsafe fn seL4_DebugSnapshot() {
    x64_sys_null(SyscallId::DebugSnapshot as seL4_Word);
    asm!("" ::: "%esi", "%edi", "memory" : "volatile");
}

#[inline(always)]
pub unsafe fn seL4_DebugCapIdentify(mut cap: seL4_CPtr) -> u32 {
    let mut unused0 = 0;
    let mut unused1 = 0;
    let mut unused2 = 0;
    let mut unused3 = 0;
    let mut unused4 = 0;
    x64_sys_send_recv(SyscallId::DebugCapIdentify as seL4_Word, 
                      cap, &mut cap, 0, &mut unused0, &mut unused1, &mut unused2, &mut unused3, &mut unused4);
    cap as u32
}

/// Note: name MUST be NUL-terminated.
#[inline(always)]
pub unsafe fn seL4_DebugNameThread(tcb: seL4_CPtr, name: &[u8]) {
    core::ptr::copy_nonoverlapping(name.as_ptr() as *mut u8, (&mut (*seL4_GetIPCBuffer()).msg).as_mut_ptr() as *mut u8,name.len());
    let mut unused0 = 0;
    let mut unused1 = 0;
    let mut unused2 = 0;
    let mut unused3 = 0;
    let mut unused4 = 0;
    let mut unused5 = 0;
    x64_sys_send_recv(SyscallId::DebugNameThread as seL4_Word, tcb, &mut unused0, 0, &mut unused1,
                      &mut unused2, &mut unused3, &mut unused4, &mut unused5);
}

#[inline(always)]
#[cfg(feature = "SEL4_DANGEROUS_CODE_INJECTION")]
pub unsafe fn seL4_DebugRun(userfn: extern fn(*mut u8), userarg: *mut u8) {
    let userfnptr = userfn as *mut ();
    x64_sys_send_null(SyscallId::DebugRun as seL4_Word, userfnptr as seL4_Word, userarg as seL4_Word);
    asm!("" ::: "memory" : "volatile");
}

#[inline(always)]
#[cfg(feature = "SEL4_CONFIG_BENCHMARK")]
pub unsafe fn seL4_BenchmarkResetLog() -> seL4_Word {
    let mut unused0 = 0;
    let mut unused1 = 0;
    let mut unused2 = 0;
    let mut unused3 = 0;
    let mut unused4 = 0;

    let mut ret = 0;

    x64_sys_send_recv(SyscallId::BenchmarkResetLog as seL4_Word, 0, &mut ret, 0, &mut unused0, &mut unused1, &mut unused2, &mut unused3, &mut unused4);

    ret
}

#[inline(always)]
#[cfg(feature = "SEL4_CONFIG_BENCHMARK")]
pub unsafe fn seL4_BenchmarkFinalizeLog() {
    let mut unused0 = 0;
    let mut unused1 = 0;
    let mut unused2 = 0;
    let mut unused3 = 0;
    let mut unused4 = 0;
    let mut index_ret = 0;
    x64_sys_send_recv(SyscallId::BenchmarkFinalizeLog as seL4_Word, 0, &mut index_ret, &mut unused0, &mut unused1, &mut unused2, &mut unused3, &mut unused4);
}

#[inline(always)]
#[cfg(feature = "SEL4_CONFIG_BENCHMARK")]
pub unsafe fn seL4_BenchmarkSetLogBuffer(mut frame_cptr: seL4_Word) -> seL4_Word {
    let mut unused0 = 0;
    let mut unused1 = 0;
    let mut unused2 = 0;
    let mut unused3 = 0;
    let mut unused4 = 0;

    x64_sys_send_recv(SyscallId::BenchmarkSetLogBuffer as seL4_Word, 
                      frame_cptr, &mut frame_cptr, 0, &mut unused0, &mut unused1, &mut unused2, &mut unused3, &mut unused4);
    frame_cptr
}

#[inline(always)]
#[cfg(feature = "SEL4_CONFIG_BENCHMARK")]
pub unsafe fn seL4_BenchmarkNullSyscall() {
    x64_sys_null(SyscallId::BenchmarkNullSyscall as seL4_Word);
    asm!("" ::: "memory" : "volatile");
}

#[inline(always)]
#[cfg(feature = "SEL4_CONFIG_BENCHMARK")]
pub unsafe fn seL4_BenchmarkFlushCaches() {
    x64_sys_null(SyscallId::BenchmarkFlushCaches as seL4_Word);
    asm!("" ::: "%esi", "%edi", "memory" : "volatile");
}

#[inline(always)]
#[cfg(feature = "SEL4_CONFIG_BENCHMARK")]
pub unsafe fn seL4_BenchmarkGetThreadUtilization(tcb: seL4_Word) {
    let mut unused0 = 0;
    let mut unused1 = 0;
    let mut unused2 = 0;
    let mut unused3 = 0;
    let mut unused4 = 0;
    let mut unused5 = 0;
    x64_sys_send_recv(SyscallId::BenchmarkGetThreadUtilisation as seL4_Word, tcb, &mut unused0, 0,
                      &mut unused1, &mut unused2, &mut unused3, &mut unused3, &mut unused4, &mut unused5);
}

#[inline(always)]
#[cfg(feature = "SEL4_CONFIG_BENCHMARK")]
pub unsafe fn seL4_BenchmarkGetThreadUtilization(tcb: seL4_Word) {
    let mut unused0 = 0;
    let mut unused1 = 0;
    let mut unused2 = 0;
    let mut unused3 = 0;
    let mut unused4 = 0;
    let mut unused5 = 0;

    x64_sys_send_recv(SyscallId::BenchmarkResetThreadUtilisation as seL4_Word, tcb, &mut unused0, 0,
                      &mut unused1, &mut unused2, &mut unused3, &mut unused4, &mut unused5);
}
