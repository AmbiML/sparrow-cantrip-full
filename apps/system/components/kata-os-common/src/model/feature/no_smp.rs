// No SMP support.
// TODO(sleffler): maybe merge into arch code

use sel4_sys::seL4_Result;
use sel4_sys::seL4_Word;

use static_assertions::assert_cfg;
assert_cfg!(not(feature = "CONFIG_SMP_SUPPORT"));

pub fn TCB_SetAffinity(_sel4_tcb: seL4_Word, _affinity: seL4_Word) -> seL4_Result { Ok(()) }
