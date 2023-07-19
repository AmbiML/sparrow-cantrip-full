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

// Sparrow platform definitions not exported in top_matcha.

// UART primary clock frequency.
pub const TOP_MATCHA_SMC_UART_CLOCK_FREQ_PERIPHERAL_HZ: u64 = 48_000_000;

// SMC timer base frequency.
pub const TOP_MATCHA_SMC_TIMER0_BASE_FREQ_HZ: u64 = 24_000_000;

// The address of the Vector Core's TCM, viewed from the SMC.
pub const TOP_MATCHA_VC_TOP_DMEM_BASE_ADDR: usize = 0x34000000;

// The size (bytes) of the Vector Core's Tightly Coupled Memory (TCM).
pub const TOP_MATCHA_VC_TOP_DMEM_SIZE_BYTES: usize = 0x1000000;
