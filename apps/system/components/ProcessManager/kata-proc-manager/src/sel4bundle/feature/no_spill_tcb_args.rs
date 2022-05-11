// Register Calling Convention.
// Max 4 arguments are passed to threads using registers.

use crate::arch::REG_ARGS;
use crate::sel4bundle::seL4Bundle;

use sel4_sys::seL4_Error;
use sel4_sys::seL4_Word;

use static_assertions::assert_cfg;
assert_cfg!(not(feature = "CONFIG_CAPDL_LOADER_CC_REGISTERS"));

impl seL4BundleImpl {
    pub fn maybe_spill_tcb_args(
        &self,
        osp: seL4_Word,
        argv: &[seL4_Word],
    ) -> Result<seL4_Word, seL4_Error> {
        let argc = argv.len();
        assert!(
            argc <= REG_ARGS,
            "TCB {} has {} arguments, which is not supported using the register calling convention",
            self.tcb_name,
            argc,
        );
        Ok(osp)
    }
}
