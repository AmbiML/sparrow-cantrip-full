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

assert_cfg!(not(feature = "CONFIG_KERNEL_MCS"));

// Syscall asm idioms.
// NB: these correspond to arm_sys_* in libsel4's syscalls.h files

// Fills all message registers. Discards everything returned by the kerrnel.
// Used for un-directed replies like seL4_Reply.
macro_rules! asm_reply {
    ($syscall:expr, $info:expr, $mr0:expr, $mr1:expr, $mr2:expr, $mr3:expr) => {
        asm!("swi 0",
            in("r7") swinum!($syscall),
            inout("r1") $info.words[0] => _,
            inout("r2") $mr0 => _,
            inout("r3") $mr1 => _,
            inout("r4") $mr2 => _,
            inout("r5") $mr3 => _,
        )
    }
}

// Fills the receiver identity and expects the badge of the sender plus all
// message registers to be returned. Used for directed receives that return
// data like seL4_Recv.
macro_rules! asm_recv {
    ($syscall:expr, $src:expr => $badge:expr, $info:expr, $mr0:expr, $mr1:expr, $mr2:expr, $mr3:expr) => {
        asm!("swi 0",
            in("r7") swinum!($syscall),
            inout("r0") $src => $badge,
            out("r1") $info,
            out("r2") $mr0,
            out("r3") $mr1,
            out("r4") $mr2,
            out("r5") $mr3,
        )
    }
}

// Does a send operation (with message registers) followed by a receive that
// returns the sender's badge plus all message registers. Used for directed
// send+receive where data flows in both directions, like seL4_Call.
#[macro_export]
macro_rules! asm_send_recv {
    ($syscall:expr, $dest:expr => $badge:expr, $info:expr => $info_recv:expr, $mr0:expr, $mr1:expr, $mr2:expr, $mr3:expr) => {
        asm!("swi 0",
            in("r7") swinum!($syscall),
            inout("r0") $dest => $badge,
            inout("r1") $info => $info_recv,
            inout("r2") $mr0,
            inout("r3") $mr1,
            inout("r4") $mr2,
            inout("r5") $mr3,
        )
    };
    // NB: for seL4_Call*
    ($syscall:expr, $src:expr => _, $info:expr => $info_recv:expr, $mr0:expr, $mr1:expr, $mr2:expr, $mr3:expr) => {
        asm!("swi 0",
            in("r7") swinum!($syscall),
            inout("r0") $src => _,
            inout("r1") $info => $info_recv,
            inout("r2") $mr0,
            inout("r3") $mr1,
            inout("r4") $mr2,
            inout("r5") $mr3,
        )
    };
}
