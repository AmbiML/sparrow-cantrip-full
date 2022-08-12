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

// No SMP support.
// TODO(sleffler): maybe merge into arch code

use sel4_sys::seL4_Result;
use sel4_sys::seL4_Word;

use static_assertions::assert_cfg;
assert_cfg!(not(feature = "CONFIG_SMP_SUPPORT"));

pub fn TCB_SetAffinity(_sel4_tcb: seL4_Word, _affinity: seL4_Word) -> seL4_Result { Ok(()) }
