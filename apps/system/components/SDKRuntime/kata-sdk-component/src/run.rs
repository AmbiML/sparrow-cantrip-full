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

/*!
 * CantripOS SDK Manager CAmkES component support routines.
 *
 * Functions defined here are entrypoints defined by the CAmkES component
 * definition in SDKRuntime.camkes, and bind the C entry points to Rust by
 * calling Rust methods in the SDKRuntimeInterface impl, CANTRIP_SDK.
 *
 * This is the lowest level entry point from C to Rust in CAmkES.
 */

#![no_std]
#![allow(clippy::missing_safety_doc)]

extern crate alloc;
use cantrip_os_common::camkes::Camkes;
use cantrip_sdk_interface::SDKRuntimeError;
use cantrip_sdk_interface::SDKRuntimeInterface;
use cantrip_sdk_runtime::CANTRIP_SDK;

static mut CAMKES: Camkes = Camkes::new("SDKRuntime");

/// CAmkES component pre-init method.
///
/// We use this to initialize our Rust heap, logger, etc.
#[no_mangle]
pub unsafe extern "C" fn pre_init() {
    static mut HEAP_MEMORY: [u8; 8 * 1024] = [0; 8 * 1024];
    CAMKES.pre_init(log::LevelFilter::Trace, &mut HEAP_MEMORY);
}

/// CAmkES sdk_ping method.
///
/// See also the component interface definition called
/// `SDKRuntimeInterface.camkes` outside of this crate. Since this is a C
/// function, we must use the C enum for error codes.
#[no_mangle]
pub unsafe extern "C" fn sdk_runtime_sdk_ping() -> SDKRuntimeError { CANTRIP_SDK.ping().into() }
