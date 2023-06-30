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

#![allow(non_camel_case_types)]
#![allow(dead_code)]

const CSR_SIZE: usize = 4096;
struct CSR {
    pub data: [u8; CSR_SIZE],
}
static mut CSR: CSR = CSR {
    data: [0u8; CSR_SIZE],
};
pub fn get_csr_mut() -> &'static mut [u8] { unsafe { &mut CSR.data[..] } }
pub fn get_csr() -> &'static [u8] { unsafe { &CSR.data[..] } }

include!("../cantrip-vec-core/src/vc_top.rs");
