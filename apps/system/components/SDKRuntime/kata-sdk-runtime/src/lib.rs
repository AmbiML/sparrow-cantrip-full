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

use cantrip_sdk_interface::error::SDKError;
use cantrip_sdk_interface::SDKRuntimeInterface;
use log::trace;

#[cfg(not(test))]
pub static mut CANTRIP_SDK: CantripSDKRuntime = CantripSDKRuntime {};

/// Cantrip OS SDK support for third-party applications, Rust core.
///
/// This is the actual Rust implementation of the SDK runtime component. Here's
/// where we can encapsulate all of our Rust fanciness, away from the C
/// bindings. This is the server-side implementation.
pub struct CantripSDKRuntime;
impl SDKRuntimeInterface for CantripSDKRuntime {
    fn ping(&self) -> Result<(), SDKError> {
        trace!("ping!");
        Ok(())
    }
}
