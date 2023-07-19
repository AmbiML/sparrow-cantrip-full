#![no_std]

pub mod mailbox {
    include!(concat!(env!("OUT_DIR"), "/mailbox.rs"));
}

pub mod ml_top {
    include!(concat!(env!("OUT_DIR"), "/ml_top.rs"));
}

pub mod timer {
    include!(concat!(env!("OUT_DIR"), "/timer.rs"));
}

pub mod uart {
    include!(concat!(env!("OUT_DIR"), "/uart.rs"));
}

pub mod vc_top {
    include!(concat!(env!("OUT_DIR"), "/vc_top.rs"));
}

#[cfg(feature = "CONFIG_PLAT_SPARROW")]
pub mod platform {
    include!("plat_sparrow.rs");
}

#[cfg(feature = "CONFIG_PLAT_NEXUS")]
pub mod platform {
    include!("plat_nexus.rs");
}
