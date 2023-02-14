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

// Syscall asm idioms.
// NB: these correspond to arm_sys_* in libsel4's syscalls.h files

// Fills the receiver identity and expects the badge of the sender plus all
// message registers to be returned. Used for directed receives that return
// data like seL4_Recv.
macro_rules! asm_recv {
    ($syscall:expr, $src:expr => $badge:expr, $info:expr, $mr0:expr, $mr1:expr, $mr2:expr, $mr3:expr, $reply:expr) => {
        asm!("svc 0",
            in("x7") swinum!($syscall),
            inout("x0") $src => $badge,
            out("x1") $info,
            out("x2") $mr0,
            out("x3") $mr1,
            out("x4") $mr2,
            out("x5") $mr3,
            in("x6") $reply,
        )
    };
    ($syscall:expr, $src:expr => $badge:expr, $info:expr, $mr0:expr, $mr1:expr, $mr2:expr, $mr3:expr) => {
        asm!("svc 0",
            in("x7") swinum!($syscall),
            inout("x0") $src => $badge,
            out("x1") $info,
            out("x2") $mr0,
            out("x3") $mr1,
            out("x4") $mr2,
            out("x5") $mr3,
            in("x6") 0,
        )
    };
}

// Does a send operation (with message registers) followed by a receive that
// returns the sender's badge plus all message registers. Used for directed
// send+receive where data flows in both directions, like seL4_Call.
#[macro_export]
macro_rules! asm_send_recv {
    ($syscall:expr, $src:expr => $badge:expr, $info:expr => $info_recv:expr, $mr0:expr, $mr1:expr, $mr2:expr, $mr3:expr, $reply:expr) => {
        asm!("svc 0",
            in("x7") swinum!($syscall),
            inout("x0") $src => $badge,
            inout("x1") $info => $info_recv,
            inout("x2") $mr0,
            inout("x3") $mr1,
            inout("x4") $mr2,
            inout("x5") $mr3,
            in("x6") $reply,
        )
    };
    // NB: for seL4_Call*
    ($syscall:expr, $src:expr => _, $info:expr => $info_recv:expr, $mr0:expr, $mr1:expr, $mr2:expr, $mr3:expr) => {
        asm!("svc 0",
            in("x7") swinum!($syscall),
            inout("x0") $src => _,
            inout("x1") $info => $info_recv,
            inout("x2") $mr0,
            inout("x3") $mr1,
            inout("x4") $mr2,
            inout("x5") $mr3,
            in("x6") 0,
        )
    };
}

// Does a non-blocking send operation followed by a receive that returns
// the sender's badge plus all message registers. Used for directed send+recv
// where data flows both directions on separate caps, e.g. seL4_NBSendRecv.
macro_rules! asm_nbsend_recv {
    ($syscall:expr, $src:expr => $badge:expr, $info:expr => $info_recv:expr, $mr0:expr, $mr1:expr, $mr2:expr, $mr3:expr, $reply:expr, $dest:expr) => {
        asm!("svc 0",
            in("x7") swinum!($syscall),
            inout("x0") $src => $badge,
            inout("x1") $info => $info_recv,
            inout("x2") $mr0,
            inout("x3") $mr1,
            inout("x4") $mr2,
            inout("x5") $mr3,
            in("x6") $reply,
            in("x8") $dest,
        )
    };
    ($syscall:expr, $src:expr => $badge:expr, $info:expr => $info_recv:expr, $mr0:expr, $mr1:expr, $mr2:expr, $mr3:expr, $reply:expr) => {
        asm!("svc 0",
            in("x7") swinum!($syscall),
            inout("x0") $src => $badge,
            inout("x1") $info => $info_recv,
            inout("x2") $mr0,
            inout("x3") $mr1,
            inout("x4") $mr2,
            inout("x5") $mr3,
            in("x6") $reply,
            in("x8") 0,
        )
    };
}
