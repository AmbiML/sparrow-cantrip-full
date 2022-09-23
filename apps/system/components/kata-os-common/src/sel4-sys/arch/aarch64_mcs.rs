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

assert_cfg!(feature = "CONFIG_KERNEL_MCS");

#[inline(always)]
pub unsafe fn seL4_Recv(
    src: seL4_CPtr,
    sender: *mut seL4_Word,
    reply: seL4_CPtr,
) -> seL4_MessageInfo {
    let mut info: seL4_Word;
    let mut badge: seL4_Word;
    let mut msg0 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg1 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg2 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg3 = ::core::mem::MaybeUninit::uninit().assume_init();

    asm!("svc 0",
        in("x7") swinum!(SyscallId::Recv),
        inout("x0") src => badge,
        out("x1") info,
        out("x2") msg0,
        out("x3") msg1,
        out("x4") msg2,
        out("x5") msg3,
        in("x6") reply,
    );

    seL4_SetMR(0, msg0);
    seL4_SetMR(1, msg1);
    seL4_SetMR(2, msg2);
    seL4_SetMR(3, msg3);

    opt_assign!(sender, badge);

    seL4_MessageInfo { words: [info] }
}

#[inline(always)]
pub unsafe fn seL4_NBRecv(
    src: seL4_CPtr,
    sender: *mut seL4_Word,
    reply: seL4_CPtr,
) -> seL4_MessageInfo {
    let info: seL4_Word;
    let mut badge: seL4_Word;
    let mut msg0 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg1 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg2 = ::core::mem::MaybeUninit::uninit().assume_init();
    let mut msg3 = ::core::mem::MaybeUninit::uninit().assume_init();

    asm!("svc 0",
        in("x7") swinum!(SyscallId::NBRecv),
        inout("x0") src => badge,
        out("x1") info,
        out("x2") msg0,
        out("x3") msg1,
        out("x4") msg2,
        out("x5") msg3,
        in("x6") reply,
    );

    seL4_SetMR(0, msg0);
    seL4_SetMR(1, msg1);
    seL4_SetMR(2, msg2);
    seL4_SetMR(3, msg3);

    opt_assign!(sender, badge);

    seL4_MessageInfo { words: [info] }
}

#[inline(always)]
pub unsafe fn seL4_ReplyRecv(
    src: seL4_CPtr,
    msgInfo: seL4_MessageInfo,
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
        in("x7") swinum!(SyscallId::ReplyRecv),
        inout("x0") src => badge,
        inout("x1") msgInfo.words[0] => info,
        inout("x2") msg0,
        inout("x3") msg1,
        inout("x4") msg2,
        inout("x5") msg3,
        in("x6") reply,
    );

    seL4_SetMR(0, msg0);
    seL4_SetMR(1, msg1);
    seL4_SetMR(2, msg2);
    seL4_SetMR(3, msg3);

    opt_assign!(sender, badge);

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
        in("x7") swinum!(SyscallId::ReplyRecv),
        inout("x0") src => badge,
        inout("x1") msgInfo.words[0] => info,
        inout("x2") msg0,
        inout("x3") msg1,
        inout("x4") msg2,
        inout("x5") msg3,
        in("x6") reply,
    );

    opt_assign!(mr0, msg0);
    opt_assign!(mr1, msg1);
    opt_assign!(mr2, msg2);
    opt_assign!(mr3, msg3);

    opt_assign!(sender, badge);

    seL4_MessageInfo { words: [info] }
}

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
        in("x6") 0,  // XXX needed?
    );

    seL4_SetMR(0, msg0);
    seL4_SetMR(1, msg1);
    seL4_SetMR(2, msg2);
    seL4_SetMR(3, msg3);

    opt_assign!(sender, badge);

    seL4_MessageInfo { words: [info] }
}

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
        in("x6") 0,  // XXX needed?
    );

    opt_assign!(mr0, msg0);
    opt_assign!(mr1, msg1);
    opt_assign!(mr2, msg2);
    opt_assign!(mr3, msg3);

    opt_assign!(sender, badge);

    seL4_MessageInfo { words: [info] }
}

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
        in("x6") 0,  // XXX needed?
    );

    seL4_SetMR(0, msg0);
    seL4_SetMR(1, msg1);
    seL4_SetMR(2, msg2);
    seL4_SetMR(3, msg3);

    opt_assign!(sender, badge);

    seL4_MessageInfo { words: [info] }
}
