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

// MCS system call templates. Expect asm_* proc macros.

assert_cfg!(feature = "CONFIG_KERNEL_MCS");

#[inline(always)]
pub unsafe fn seL4_Recv(
    src: seL4_CPtr,
    sender: *mut seL4_Word,
    reply: seL4_CPtr,
) -> seL4_MessageInfo {
    let mut info: seL4_Word;
    let mut badge: seL4_Word;
    let mut msg0;
    let mut msg1;
    let mut msg2;
    let mut msg3;

    asm_recv!(SyscallId::Recv, src => badge, info, msg0, msg1, msg2, msg3, reply);

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
    let mut msg0;
    let mut msg1;
    let mut msg2;
    let mut msg3;

    asm_recv!(SyscallId::NBRecv, src => badge, info, msg0, msg1, msg2, msg3, reply);

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

    asm_send_recv!(SyscallId::ReplyRecv, src => badge, msgInfo.words[0] => info, msg0, msg1, msg2, msg3, reply);

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
    let mut msg0 = MaybeUninit::<seL4_Word>::uninit();
    let mut msg1 = MaybeUninit::<seL4_Word>::uninit();
    let mut msg2 = MaybeUninit::<seL4_Word>::uninit();
    let mut msg3 = MaybeUninit::<seL4_Word>::uninit();

    if !mr0.is_null() && msgInfo.get_length() > 0 {
        *msg0.assume_init_mut() = *mr0;
    }
    if !mr1.is_null() && msgInfo.get_length() > 1 {
        *msg1.assume_init_mut() = *mr1;
    }
    if !mr2.is_null() && msgInfo.get_length() > 2 {
        *msg2.assume_init_mut() = *mr2;
    }
    if !mr3.is_null() && msgInfo.get_length() > 3 {
        *msg3.assume_init_mut() = *mr3;
    }

    asm_send_recv!(SyscallId::ReplyRecv, src => badge, msgInfo.words[0] => info, *msg0.assume_init_mut(), *msg1.assume_init_mut(), *msg2.assume_init_mut(), *msg3.assume_init_mut(), reply);

    opt_assign!(mr0, *msg0.assume_init_mut());
    opt_assign!(mr1, *msg1.assume_init_mut());
    opt_assign!(mr2, *msg2.assume_init_mut());
    opt_assign!(mr3, *msg3.assume_init_mut());

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

    asm_nbsend_recv!(SyscallId::NBSendRecv, src => badge, msgInfo.words[0] => info, msg0, msg1, msg2, msg3, reply, dest);

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
    let mut msg0 = MaybeUninit::<seL4_Word>::uninit();
    let mut msg1 = MaybeUninit::<seL4_Word>::uninit();
    let mut msg2 = MaybeUninit::<seL4_Word>::uninit();
    let mut msg3 = MaybeUninit::<seL4_Word>::uninit();

    if !mr0.is_null() && msgInfo.get_length() > 0 {
        *msg0.assume_init_mut() = *mr0;
    }
    if !mr1.is_null() && msgInfo.get_length() > 1 {
        *msg1.assume_init_mut() = *mr1;
    }
    if !mr2.is_null() && msgInfo.get_length() > 2 {
        *msg2.assume_init_mut() = *mr2;
    }
    if !mr3.is_null() && msgInfo.get_length() > 3 {
        *msg3.assume_init_mut() = *mr3;
    }

    asm_nbsend_recv!(SyscallId::NBSendRecv, src => badge, msgInfo.words[0] => info, *msg0.assume_init_mut(), *msg1.assume_init_mut(), *msg2.assume_init_mut(), *msg3.assume_init_mut(), reply, dest);

    opt_assign!(mr0, *msg0.assume_init_mut());
    opt_assign!(mr1, *msg1.assume_init_mut());
    opt_assign!(mr2, *msg2.assume_init_mut());
    opt_assign!(mr3, *msg3.assume_init_mut());

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

    asm_nbsend_recv!(SyscallId::NBSendWait, src => badge, msgInfo.words[0] => info, msg0, msg1, msg2, msg3, dest, 0);

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
    let mut msg0 = MaybeUninit::<seL4_Word>::uninit();
    let mut msg1 = MaybeUninit::<seL4_Word>::uninit();
    let mut msg2 = MaybeUninit::<seL4_Word>::uninit();
    let mut msg3 = MaybeUninit::<seL4_Word>::uninit();

    if !mr0.is_null() && msgInfo.get_length() > 0 {
        *msg0.assume_init_mut() = *mr0;
    }
    if !mr1.is_null() && msgInfo.get_length() > 1 {
        *msg1.assume_init_mut() = *mr1;
    }
    if !mr2.is_null() && msgInfo.get_length() > 2 {
        *msg2.assume_init_mut() = *mr2;
    }
    if !mr3.is_null() && msgInfo.get_length() > 3 {
        *msg3.assume_init_mut() = *mr3;
    }

    asm_nbsend_recv!(SyscallId::NBSendWait, src => badge, msgInfo.words[0] => info, *msg0.assume_init_mut(), *msg1.assume_init_mut(), *msg2.assume_init_mut(), *msg3.assume_init_mut(), dest, 0);

    opt_assign!(mr0, *msg0.assume_init_mut());
    opt_assign!(mr1, *msg1.assume_init_mut());
    opt_assign!(mr2, *msg2.assume_init_mut());
    opt_assign!(mr3, *msg3.assume_init_mut());

    opt_assign!(sender, badge);

    seL4_MessageInfo { words: [info] }
}

#[inline(always)]
pub unsafe fn seL4_Wait(src: seL4_CPtr, sender: *mut seL4_Word) -> seL4_MessageInfo {
    let mut info: seL4_Word;
    let mut badge: seL4_Word;
    // NB: all MR's are returned by kerrnel
    let mut msg0;
    let mut msg1;
    let mut msg2;
    let mut msg3;

    asm_recv!(SyscallId::Wait, src => badge, info, msg0, msg1, msg2, msg3);

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
    let mut msg0: seL4_Word;
    let mut msg1: seL4_Word;
    let mut msg2: seL4_Word;
    let mut msg3: seL4_Word;

    asm_recv!(SyscallId::Wait, src => badge, info, msg0, msg1, msg2, msg3);

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
    // NB: all MR's are returned by kerrnel
    let mut msg0;
    let mut msg1;
    let mut msg2;
    let mut msg3;

    asm_recv!(SyscallId::NBWait, src => badge, info, msg0, msg1, msg2, msg3);

    seL4_SetMR(0, msg0);
    seL4_SetMR(1, msg1);
    seL4_SetMR(2, msg2);
    seL4_SetMR(3, msg3);

    opt_assign!(sender, badge);

    seL4_MessageInfo { words: [info] }
}

#[inline(always)]
pub unsafe fn seL4_Poll(src: seL4_CPtr, sender: *mut seL4_Word) -> seL4_MessageInfo {
    seL4_NBWait(src, sender)
}
