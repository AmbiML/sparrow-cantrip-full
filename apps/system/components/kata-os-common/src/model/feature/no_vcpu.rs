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

// No Hypervisor Support.

use crate::CantripOsModel;
use capdl::*;

use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_Result;

use static_assertions::assert_cfg;
assert_cfg!(not(any(
    feature = "CONFIG_ARM_HYPERVISOR_SUPPORT",
    feature = "CONFIG_VTX"
)));

impl<'a> CantripOsModel<'a> {
    pub fn set_tcb_vcpu(&self, _cdl_tcb: &CDL_Object, _sel4_tcb: seL4_CPtr) -> seL4_Result {
        Ok(())
    }
}
