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

#[cfg(feature = "springbok_support")]
pub mod vc_top {
    include!(concat!(env!("OUT_DIR"), "/vc_top.rs"));
}
