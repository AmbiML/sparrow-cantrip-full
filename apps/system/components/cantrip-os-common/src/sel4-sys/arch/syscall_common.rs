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

// Common system call templates. Expect asm_* proc macros.

use core::mem::MaybeUninit;

#[inline(always)]
pub unsafe fn seL4_Yield() {
    asm_no_args!(SyscallId::Yield);
}

#[inline(always)]
pub unsafe fn seL4_Send(dest: seL4_CPtr, msgInfo: seL4_MessageInfo) {
    asm_send!(
        SyscallId::Send,
        dest,
        msgInfo.words[0],
        seL4_GetMR(0),
        seL4_GetMR(1),
        seL4_GetMR(2),
        seL4_GetMR(3)
    );
}

#[inline(always)]
pub unsafe fn seL4_SendWithMRs(
    dest: seL4_CPtr,
    msgInfo: seL4_MessageInfo,
    mr0: *const seL4_Word,
    mr1: *const seL4_Word,
    mr2: *const seL4_Word,
    mr3: *const seL4_Word,
) {
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

    asm_send!(
        SyscallId::Send,
        dest,
        msgInfo.words[0],
        msg0.assume_init(),
        msg1.assume_init(),
        msg2.assume_init(),
        msg3.assume_init()
    );
}

#[inline(always)]
pub unsafe fn seL4_NBSend(dest: seL4_CPtr, msgInfo: seL4_MessageInfo) {
    asm_send!(
        SyscallId::NBSend,
        dest,
        msgInfo.words[0],
        seL4_GetMR(0),
        seL4_GetMR(1),
        seL4_GetMR(2),
        seL4_GetMR(3)
    );
}

#[inline(always)]
pub unsafe fn seL4_NBSendWithMRs(
    dest: seL4_CPtr,
    msgInfo: seL4_MessageInfo,
    mr0: *const seL4_Word,
    mr1: *const seL4_Word,
    mr2: *const seL4_Word,
    mr3: *const seL4_Word,
) {
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

    asm_send!(
        SyscallId::NBSend,
        dest,
        msgInfo.words[0],
        msg0.assume_init(),
        msg1.assume_init(),
        msg2.assume_init(),
        msg3.assume_init()
    );
}

#[inline(always)]
pub unsafe fn seL4_Signal(dest: seL4_CPtr) {
    let info = seL4_MessageInfo::new(0, 0, 0, 0).words[0];
    asm_send_no_mrs!(SyscallId::Send, dest, info);
}

#[inline(always)]
pub unsafe fn seL4_Call(dest: seL4_CPtr, msgInfo: seL4_MessageInfo) -> seL4_MessageInfo {
    let mut info: seL4_Word;
    let mut msg0 = seL4_GetMR(0);
    let mut msg1 = seL4_GetMR(1);
    let mut msg2 = seL4_GetMR(2);
    let mut msg3 = seL4_GetMR(3);

    asm_send_recv!(SyscallId::Call, dest => _, msgInfo.words[0] => info, msg0, msg1, msg2, msg3);

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

    asm_send_recv!(SyscallId::Call, dest => _, msgInfo.words[0] => info, *msg0.assume_init_mut(), *msg1.assume_init_mut(), *msg2.assume_init_mut(), *msg3.assume_init_mut());

    opt_assign!(mr0, *msg0.assume_init_mut());
    opt_assign!(mr1, *msg1.assume_init_mut());
    opt_assign!(mr2, *msg2.assume_init_mut());
    opt_assign!(mr3, *msg3.assume_init_mut());

    seL4_MessageInfo { words: [info] }
}
