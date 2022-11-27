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

use cfg_if::cfg_if;

extern crate alloc;
use cantrip_os_common::camkes::seL4_CPath;
use cantrip_os_common::cspace_slot::CSpaceSlot;
use cantrip_os_common::sel4_sys;
use cantrip_sdk_manager::SDKManagerError;
use cantrip_sdk_manager::SDKManagerInterface;
use cantrip_security_interface::cantrip_security_delete_key;
use cantrip_security_interface::cantrip_security_read_key;
use cantrip_security_interface::cantrip_security_write_key;
use core::hash::BuildHasher;
use hashbrown::HashMap;
cfg_if! {
    if #[cfg(feature = "timer_support")] {
        use cantrip_timer_interface::cantrip_timer_cancel;
        use cantrip_timer_interface::cantrip_timer_oneshot;
        use cantrip_timer_interface::cantrip_timer_periodic;
        use cantrip_timer_interface::cantrip_timer_poll;
        use cantrip_timer_interface::cantrip_timer_wait;
        use cantrip_timer_interface::TimerServiceError;
    }
}
use log::{error, info, trace};
use sdk_interface::error::SDKError;
use sdk_interface::KeyValueData;
use sdk_interface::SDKAppId;
use sdk_interface::SDKRuntimeInterface;
use sdk_interface::TimerDuration;
use sdk_interface::TimerId;
use sdk_interface::TimerMask;
use smallstr::SmallString;

use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_CapRights;

// App capacity before spillover to the heap; should be the max concurrent
// started apps. Set very small because we expect, at least initially, that
// only one app at a time will be started.
const DEFAULT_APP_CAPACITY: usize = 3;

// BundleId capacity before spillover to the heap.
// TODO(sleffler): hide this; it's part of the implementation
// TODO(sleffler): shared with cantrip-proc-interface
const DEFAULT_BUNDLE_ID_CAPACITY: usize = 64;

type SmallId = SmallString<[u8; DEFAULT_BUNDLE_ID_CAPACITY]>;

struct SDKRuntimeState {
    id: SmallId,
}
impl SDKRuntimeState {
    pub fn new(app_id: &str) -> Self {
        Self {
            id: SmallId::from_str(app_id),
        }
    }
}

/// Cantrip OS SDK support for third-party applications, Rust core.
///
/// This is the actual Rust implementation of the SDK runtime component. Here's
/// where we can encapsulate all of our Rust fanciness, away from the C
/// bindings. This is the server-side implementation.
// XXX hashmap may be overkill, could use SmallVec and badge by index
pub struct SDKRuntime {
    endpoint: seL4_CPath,
    apps: HashMap<SDKAppId, SDKRuntimeState>,
}
impl SDKRuntime {
    pub fn new(endpoint: &seL4_CPath) -> Self {
        Self {
            endpoint: *endpoint,
            apps: HashMap::with_capacity(DEFAULT_APP_CAPACITY),
        }
    }

    // Calculates the badge assigned to the seL4 endpoint the client will use
    // to send requests to the SDKRuntime. This must be unique among active
    // clients but may be reused. There is no need to randomize or otherwise
    // secure this value since clients cannot forge an endpoint.
    // TODO(sleffler): is it worth doing a hash? counter is probably sufficient
    #[cfg(target_pointer_width = "32")]
    fn calculate_badge(&self, id: &SmallId) -> SDKAppId {
        (self.apps.hasher().hash_one(id) & 0x0ffffff) as SDKAppId
    }

    #[cfg(target_pointer_width = "64")]
    fn calculate_badge(&self, id: &SmallId) -> SDKAppId {
        self.apps.hasher().hash_one(id) as SDKAppId
    }

    pub fn capacity(&self) -> usize { self.apps.capacity() }
}
impl SDKManagerInterface for SDKRuntime {
    /// Returns an seL4 Endpoint capability for |app_id| to make SDKRuntime
    /// requests..Without a registered endpoint all requests will fail.
    /// first calling cantrip_sdk_manager_get_endpoint().
    fn get_endpoint(&mut self, app_id: &str) -> Result<seL4_CPtr, SDKManagerError> {
        let badge = self.calculate_badge(&SmallId::from_str(app_id));

        // Mint a badged endpoint for the client to talk to us.
        let mut slot = CSpaceSlot::new();
        slot.mint_to(
            self.endpoint.0,
            self.endpoint.1,
            self.endpoint.2 as u8,
            seL4_CapRights::new(
                /*grant_reply=*/ 1,
                /*grant=*/ 1, // NB: to send frame with RPC params
                /*read=*/ 0, /*write=*/ 1,
            ),
            badge,
        )
        .map_err(|_| SDKManagerError::SmGetEndpointFailed)?;

        // Create the entry & return the endpoint capability.
        assert!(self
            .apps
            .insert(badge, SDKRuntimeState::new(app_id))
            .is_none());
        Ok(slot.release())
    }

    /// Releases |app_id| state. No future requests may be made without
    /// first calling cantrip_sdk_manager_get_endpoint().
    fn release_endpoint(&mut self, app_id: &str) -> Result<(), SDKManagerError> {
        let badge = self.calculate_badge(&SmallId::from_str(app_id));
        let _ = self.apps.remove(&badge);
        Ok(())
    }
}
impl SDKRuntimeInterface for SDKRuntime {
    /// Pings the SDK runtime, going from client to server and back via CAmkES IPC.
    fn ping(&self, app_id: SDKAppId) -> Result<(), SDKError> {
        match self.apps.get(&app_id) {
            Some(_) => Ok(()),
            None => {
                // XXX potential console spammage/DOS
                error!("No entry for app_id {:x}", app_id);
                Err(SDKError::InvalidBadge)
            }
        }
    }

    /// Logs |msg| through the system logger.
    fn log(&self, app_id: SDKAppId, msg: &str) -> Result<(), SDKError> {
        match self.apps.get(&app_id) {
            Some(app) => {
                info!(target: &alloc::format!("[{}]", app.id), "{}", msg);
                Ok(())
            }
            None => Err(SDKError::InvalidBadge),
        }
    }

    /// Returns any value for the specified |key| in the app's  private key-value store.
    fn read_key<'a>(
        &self,
        app_id: SDKAppId,
        key: &str,
        keyval: &'a mut [u8],
    ) -> Result<&'a [u8], SDKError> {
        match self.apps.get(&app_id) {
            Some(app) => {
                cantrip_security_read_key(&app.id, key, keyval)
                    .map_err(|_| SDKError::ReadKeyFailed)?; // XXX
                Ok(keyval)
            }
            None => Err(SDKError::InvalidBadge),
        }
    }

    /// Writes |value| for the specified |key| in the app's private key-value store.
    fn write_key(&self, app_id: SDKAppId, key: &str, value: &KeyValueData) -> Result<(), SDKError> {
        match self.apps.get(&app_id) {
            Some(app) => {
                cantrip_security_write_key(&app.id, key, value)
                    .map_err(|_| SDKError::WriteKeyFailed)?; // XXX
                Ok(())
            }
            None => Err(SDKError::InvalidBadge),
        }
    }

    /// Deletes the specified |key| in the app's private key-value store.
    fn delete_key(&self, app_id: SDKAppId, key: &str) -> Result<(), SDKError> {
        match self.apps.get(&app_id) {
            Some(app) => {
                cantrip_security_delete_key(&app.id, key).map_err(|_| SDKError::DeleteKeyFailed)?; // XXX
                Ok(())
            }
            None => Err(SDKError::InvalidBadge),
        }
    }

    // TODO(sleffler): compose id+app.id to form timer id

    #[allow(unused_variables)]
    fn timer_oneshot(
        &self,
        app_id: SDKAppId,
        id: TimerId,
        duration_ms: TimerDuration,
    ) -> Result<(), SDKError> {
        trace!("timer_oneshot id {} duration {}", id, duration_ms);
        match self.apps.get(&app_id) {
            Some(_) => {
                #[cfg(feature = "timer_support")]
                return cantrip_timer_oneshot(id, duration_ms).map_err(|e| map_timer_err(e));

                #[cfg(not(feature = "timer_support"))]
                Err(SDKError::NoPlatformSupport)
            }
            None => Err(SDKError::InvalidBadge),
        }
    }

    #[allow(unused_variables)]
    fn timer_periodic(
        &self,
        app_id: SDKAppId,
        id: TimerId,
        duration_ms: TimerDuration,
    ) -> Result<(), SDKError> {
        trace!("timer_periodic id {} duration {}", id, duration_ms);
        match self.apps.get(&app_id) {
            Some(_) => {
                #[cfg(feature = "timer_support")]
                return cantrip_timer_periodic(id, duration_ms).map_err(|e| map_timer_err(e));

                #[cfg(not(feature = "timer_support"))]
                Err(SDKError::NoPlatformSupport)
            }
            None => Err(SDKError::InvalidBadge),
        }
    }

    #[allow(unused_variables)]
    fn timer_cancel(&self, app_id: SDKAppId, id: TimerId) -> Result<(), SDKError> {
        trace!("timer_cancel id {}", id);
        match self.apps.get(&app_id) {
            Some(_) => {
                #[cfg(feature = "timer_support")]
                return cantrip_timer_cancel(id).map_err(|e| map_timer_err(e));

                #[cfg(not(feature = "timer_support"))]
                Err(SDKError::NoPlatformSupport)
            }
            None => Err(SDKError::InvalidBadge),
        }
    }

    fn timer_wait(&self, app_id: SDKAppId) -> Result<TimerMask, SDKError> {
        trace!("timer_wait");
        match self.apps.get(&app_id) {
            Some(_) => {
                #[cfg(feature = "timer_support")]
                return cantrip_timer_wait().map_err(|e| map_timer_err(e));

                #[cfg(not(feature = "timer_support"))]
                Err(SDKError::NoPlatformSupport)
            }
            None => Err(SDKError::InvalidBadge),
        }
    }

    fn timer_poll(&self, app_id: SDKAppId) -> Result<TimerMask, SDKError> {
        trace!("timer_poll");
        match self.apps.get(&app_id) {
            Some(_) => {
                #[cfg(feature = "timer_support")]
                return cantrip_timer_poll().map_err(|e| map_timer_err(e));

                #[cfg(not(feature = "timer_support"))]
                Err(SDKError::NoPlatformSupport)
            }
            None => Err(SDKError::InvalidBadge),
        }
    }
}

#[cfg(feature = "timer_support")]
fn map_timer_err(err: TimerServiceError) -> SDKError {
    match err {
        TimerServiceError::NoSuchTimer => SDKError::NoSuchTimer,
        TimerServiceError::TimerAlreadyExists => SDKError::TimerAlreadyExists,
        _ => SDKError::NoSuchTimer, // XXX should never happen
    }
}
