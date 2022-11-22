// Copyright 2022 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
