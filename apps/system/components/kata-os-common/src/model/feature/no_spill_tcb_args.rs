// Register Calling Convention.
// Max 4 arguments are passed to threads using registers.

use crate::arch::REG_ARGS;
use crate::CantripOsModel;
use capdl::*;

use sel4_sys::seL4_Error;
use sel4_sys::seL4_Word;

use static_assertions::assert_cfg;
assert_cfg!(not(feature = "CONFIG_CAPDL_LOADER_CC_REGISTERS"));

impl<'a> CantripOsModel<'a> {
    pub fn maybe_spill_tcb_args(
        &self,
        cdl_tcb: &CDL_Object,
        sp: seL4_Word,
    ) -> Result<seL4_Word, seL4_Error> {
        let argc = cdl_tcb.tcb_init_sz();
        assert!(
            argc <= REG_ARGS,
            "TCB {} has {} arguments, which is not supported using {} the register calling convention",
            cdl_tcb.name(),
            argc,
        );
        Ok(sp)
    }
}
