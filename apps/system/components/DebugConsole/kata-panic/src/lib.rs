#![no_std]

use core::panic::PanicInfo;
use core::sync::atomic::{self, Ordering};
use log::error;

#[inline(never)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Could use panic::set_hook but we're already here...
    error!("{}", info);

    // Halt the thread.
    loop {
        // TODO(sleffler): seL4_Yield?
        atomic::compiler_fence(Ordering::SeqCst);
    }
}
