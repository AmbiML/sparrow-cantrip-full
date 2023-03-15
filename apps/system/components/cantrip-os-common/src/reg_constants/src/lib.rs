#![no_std]

pub mod mailbox {
    include!(concat!(env!("OUT_DIR"), "/mailbox.rs"));
}

pub mod timer {
    include!(concat!(env!("OUT_DIR"), "/timer.rs"));
}

pub mod uart {
    include!(concat!(env!("OUT_DIR"), "/uart.rs"));
}
