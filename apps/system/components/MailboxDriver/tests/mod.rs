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

const MAILBOX_MMIO_SIZE: usize = 4096;
struct MAILBOX_MMIO {
    pub data: [u8; MAILBOX_MMIO_SIZE],
}
static mut MAILBOX_MMIO: MAILBOX_MMIO = MAILBOX_MMIO {
    data: [0u8; MAILBOX_MMIO_SIZE],
};

include!("../mailbox-driver/src/mailbox.rs");
