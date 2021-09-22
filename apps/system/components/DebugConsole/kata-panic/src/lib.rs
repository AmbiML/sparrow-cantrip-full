#![no_std]

#[cfg(not(test))]
#[inline(never)]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::sync::atomic::{self, Ordering};
    use log::error;

    // Could use panic::set_hook but we're already here...
    error!("{}", info);

    // Halt the thread.
    loop {
        // TODO(sleffler): seL4_Yield?
        atomic::compiler_fence(Ordering::SeqCst);
    }
}
