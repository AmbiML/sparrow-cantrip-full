#![allow(dead_code)]

// Support for debugging capability handling. These only do something
// in a DEBUG build where cap_identify should work. Beware of the
// hardwired cap type codes; the 3 macros defined should be portable
// but seL4 generates defs that are architecture & feature-dependent.
// Note one can implement an is_slot_empty check without
// seL4_DebugCapIdentify by checking the error code from doing a cap
// move slot -> slot. We do not use it mainly because it will spam
// the console if CONFIG_PRINTING is enabled.

use crate::seL4_CPtr;

#[inline]
#[cfg(feature = "CONFIG_DEBUG_BUILD")]
pub fn cap_identify(cap: seL4_CPtr) -> Option<u32> {
    Some(unsafe { crate::seL4_DebugCapIdentify(cap) })
}

#[inline]
#[cfg(not(feature = "CONFIG_DEBUG_BUILD"))]
pub fn cap_identify(_cap: seL4_CPtr) -> Option<u32> {
    None // cap_null_cap
}

#[macro_export]
macro_rules! debug_assert_slot_empty {
    // 0 is cap_null_cap
    ($cap:expr) => {
        debug_assert!($crate::cap_identify($cap) == Some(0))
    };
    ($cap:expr, $($arg:tt)+) => {
        debug_assert!($crate::cap_identify($cap) == Some(0), $($arg)+)
    };
}

#[macro_export]
macro_rules! debug_assert_slot_frame {
    // 1 is cap_frame_cap
    ($cap:expr) => {
        debug_assert!($crate::cap_identify($cap) == Some(1))
    };
    ($cap:expr, $($arg:tt)+) => {
        debug_assert!($crate::cap_identify($cap) == Some(1), $($arg)+)
    };
}

#[macro_export]
macro_rules! debug_assert_slot_cnode {
    // 10 is cap_frame_cap
    ($cap:expr) => {
        debug_assert!($crate::cap_identify($cap) == Some(10))
    };
    ($cap:expr, $($arg:tt)+) => {
        debug_assert!($crate::cap_identify($cap) == Some(10), $($arg)+)
    };
}
