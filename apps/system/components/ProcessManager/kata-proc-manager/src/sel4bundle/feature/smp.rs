// SMP support.
// TODO(sleffler): maybe merge into arch code

use super::sel4_sys;

use sel4_sys::seL4_Result;
use sel4_sys::seL4_TCB_SetAffinity;
use sel4_sys::seL4_Word;

use static_assertions::assert_cfg;
assert_cfg!(feature = "CONFIG_SMP_SUPPORT");

pub fn TCB_SetAffinity(sel4_tcb: seL4_Word, affinity: seL4_Word) -> seL4_Result {
    seL4_TCB_SetAffinity(sel4_tcb, affinity)
}
