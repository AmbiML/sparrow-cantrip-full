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

//! Cantrip OS security coordinator support

#![cfg_attr(not(test), no_std)]
#![allow(stable_features)]
// NB: "error[E0658]: trait bounds other than `Sized` on const fn parameters are unstable"
#![feature(const_fn_trait_bound)]

extern crate alloc;
use cantrip_security_interface::SecurityCoordinatorInterface;

#[cfg(all(feature = "fake", feature = "sel4"))]
compile_error!("features \"fake\" and \"sel4\" are mutually exclusive");

#[cfg_attr(feature = "sel4", path = "impl.rs")]
#[cfg_attr(feature = "fake", path = "fakeimpl/mod.rs")]
mod platform;
pub use platform::CantripSecurityCoordinatorInterface;

mod upload;

// CantripSecurityCoordinator bundles an instance of the SecurityCoordinator that operates
// on CantripOS interfaces. There is a two-step dance to setup an instance because we want
// CANTRIP_SECURITY static.
// NB: no locking is done; we assume the caller/user is single-threaded
pub struct CantripSecurityCoordinator<SC> {
    manager: Option<SC>,
}
impl<SC: SecurityCoordinatorInterface> CantripSecurityCoordinator<SC> {
    // Constructs a partially-initialized instance; to complete call init().
    // This is needed because we need a const fn for static setup.
    pub const fn empty() -> CantripSecurityCoordinator<SC> {
        CantripSecurityCoordinator { manager: None }
    }

    pub fn is_empty(&self) -> bool {
        self.manager.is_none()
    }

    pub fn init(&mut self, manager: SC) {
        self.manager = Some(manager);
    }

    pub fn get(&mut self) -> &mut impl SecurityCoordinatorInterface {
        self.manager
            .as_mut()
            .expect("must call init before first get")
    }
}
