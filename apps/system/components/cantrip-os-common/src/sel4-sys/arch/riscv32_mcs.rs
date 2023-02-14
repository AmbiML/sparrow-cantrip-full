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
// NB: these correspond to riscv_sys_* in libsel4's syscalls.h files

// Fills the receiver identity and expects the badge of the sender plus all
// message registers to be returned. Used for directed receives that return
// data like seL4_Recv.
macro_rules! asm_recv {
    ($syscall:expr, $src:expr => $badge:expr, $info:expr, $mr0:expr, $mr1:expr, $mr2:expr, $mr3:expr, $reply:expr) => {
        asm!("ecall",
            in("a7") swinum!($syscall),
            inout("a0") $src => $badge,
            out("a1") $info,
            out("a2") $mr0,
            out("a3") $mr1,
            out("a4") $mr2,
            out("a5") $mr3,
            in("a6") $reply,
        )
    };
    ($syscall:expr, $src:expr => $badge:expr, $info:expr, $mr0:expr, $mr1:expr, $mr2:expr, $mr3:expr) => {
        asm!("ecall",
            in("a7") swinum!($syscall),
            inout("a0") $src => $badge,
            out("a1") $info,
            out("a2") $mr0,
            out("a3") $mr1,
            out("a4") $mr2,
            out("a5") $mr3,
            in("a6") 0,
        )
    };
}

// Does a send operation (with message registers) followed by a receive that
// returns the sender's badge plus all message registers. Used for directed
// send+receive where data flows in both directions, like seL4_Call.
#[macro_export]
macro_rules! asm_send_recv {
    ($syscall:expr, $src:expr => $badge:expr, $info:expr => $info_recv:expr, $mr0:expr, $mr1:expr, $mr2:expr, $mr3:expr, $reply:expr) => {
        asm!("ecall",
            in("a7") swinum!($syscall),
            inout("a0") $src => $badge,
            inout("a1") $info => $info_recv,
            inout("a2") $mr0,
            inout("a3") $mr1,
            inout("a4") $mr2,
            inout("a5") $mr3,
            in("a6") $reply,
        )
    };
    // NB: for seL4_Call*
    ($syscall:expr, $src:expr => _, $info:expr => $info_recv:expr, $mr0:expr, $mr1:expr, $mr2:expr, $mr3:expr) => {
        asm!("ecall",
            in("a7") swinum!($syscall),
            inout("a0") $src => _,
            inout("a1") $info => $info_recv,
            inout("a2") $mr0,
            inout("a3") $mr1,
            inout("a4") $mr2,
            inout("a5") $mr3,
            in("a6") 0,
        )
    };
}

// Does a non-blocking send operation followed by a receive that returns
// the sender's badge plus all message registers. Used for directed send+recv
// where data flows both directions on separate caps, e.g. seL4_NBSendRecv.
macro_rules! asm_nbsend_recv {
    ($syscall:expr, $src:expr => $badge:expr, $info:expr => $info_recv:expr, $mr0:expr, $mr1:expr, $mr2:expr, $mr3:expr, $reply:expr, $dest:expr) => {
        asm!("ecall",
            in("a7") swinum!($syscall),
            inout("a0") $src => $badge,
            inout("a1") $info => $info_recv,
            inout("a2") $mr0,
            inout("a3") $mr1,
            inout("a4") $mr2,
            inout("a5") $mr3,
            in("a6") $reply,
            in("t0") $dest,
        )
    };
    ($syscall:expr, $src:expr => $badge:expr, $info:expr => $info_recv:expr, $mr0:expr, $mr1:expr, $mr2:expr, $mr3:expr, $reply:expr) => {
        asm!("ecall",
            in("a7") swinum!($syscall),
            inout("a0") $src => $badge,
            inout("a1") $info => $info_recv,
            inout("a2") $mr0,
            inout("a3") $mr1,
            inout("a4") $mr2,
            inout("a5") $mr3,
            in("a6") $reply,
            in("t0") 0,
        )
    };
}
