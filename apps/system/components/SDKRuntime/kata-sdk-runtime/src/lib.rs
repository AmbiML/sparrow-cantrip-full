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

#![cfg_attr(not(test), no_std)]
#![feature(build_hasher_simple_hash_one)]

use cantrip_os_common::camkes::seL4_CPath;
use cantrip_os_common::sel4_sys;
use cantrip_sdk_interface::error::SDKError;
use cantrip_sdk_interface::SDKAppId;
use cantrip_sdk_interface::SDKRuntimeInterface;
use cantrip_sdk_manager::SDKManagerError;
use cantrip_sdk_manager::SDKManagerInterface;
use spin::Mutex;

use sel4_sys::seL4_CPtr;

mod runtime;
use runtime::SDKRuntime;

/// Wrapper around SDKRuntime implementation. Because we have two CAmkES
/// interfaces there may be concurrent calls so we lock at this level.
pub struct CantripSDKRuntime {
    runtime: Mutex<Option<SDKRuntime>>,
}
impl CantripSDKRuntime {
    // Constructs a partially-initialized instance; to complete call init().
    // This is needed because we need a const fn for static setup.
    pub const fn empty() -> CantripSDKRuntime {
        CantripSDKRuntime {
            runtime: Mutex::new(None),
        }
    }
    // Finishes the setup started by empty():
    pub fn init(&self, endpoint: &seL4_CPath) {
        *self.runtime.lock() = Some(SDKRuntime::new(endpoint));
    }
    // Returns the bundle capacity.
    pub fn capacity(&self) -> usize { self.runtime.lock().as_ref().unwrap().capacity() }
}
// These just lock accesses and handle the necessary indirection.
impl SDKManagerInterface for CantripSDKRuntime {
    fn get_endpoint(&mut self, app_id: &str) -> Result<seL4_CPtr, SDKManagerError> {
        self.runtime.lock().as_mut().unwrap().get_endpoint(app_id)
    }
    fn release_endpoint(&mut self, app_id: &str) -> Result<(), SDKManagerError> {
        self.runtime
            .lock()
            .as_mut()
            .unwrap()
            .release_endpoint(app_id)
    }
}
impl SDKRuntimeInterface for CantripSDKRuntime {
    fn ping(&self, app_id: SDKAppId) -> Result<(), SDKError> {
        self.runtime.lock().as_ref().unwrap().ping(app_id)
    }
    fn log(&self, app_id: SDKAppId, msg: &str) -> Result<(), SDKError> {
        self.runtime.lock().as_ref().unwrap().log(app_id, msg)
    }
}
