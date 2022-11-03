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

//! CantripOS SDK application runtime interfaces.

#![cfg_attr(not(test), no_std)]

pub mod error;

pub use error::SDKError;
pub use error::SDKRuntimeError;

use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};

use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_Call;
use sel4_sys::seL4_MessageInfo;
use sel4_sys::seL4_PageBits;
use sel4_sys::seL4_SetCap;

const PAGE_SIZE: usize = 1 << seL4_PageBits;

// SDKRuntime client-side state setup by ProcessManager and crt0.
// TODO(sleffler): is 1 page enough? ProcessManager should probably have
//   SDKRuntime handle this
extern "C" {
    static CANTRIP_SDK_ENDPOINT: seL4_CPtr; // IPC connection to SDKRuntime
    static CANTRIP_SDK_FRAME: seL4_CPtr; // RPC parameters frame
    static CANTRIP_SDK_PARAMS: *mut u8; // Virtual address of CANTRIP_SDK_FRAME
}

// Size of the buffers used to pass serialized data. The data structure
// sizes are bounded by the single page (4K bytes) used to marshal & unmarshal
// parameters and also by their being allocated on the stack. We balance
// these against being able to handle large amounts of data.
// XXX do sensor frames need to be passed & are they too big?

// pub for server-side logic
pub const SDKRUNTIME_REQUEST_DATA_SIZE: usize = PAGE_SIZE / 2;

/// Application identity derived from seL4 Endpoint badge setup when
/// the application is started by ProcessManager.
///
/// NB: On 32-bit platforms the kernel truncates this to 28-bits;
///     on 64-bit platforms these are 64-bits.
pub type SDKAppId = usize;

// TODO(sleffler): temp constraint on value part of key-value pairs
// TOOD(sleffler): dup's security coordinator but we don't want a dependency
pub const KEY_VALUE_DATA_SIZE: usize = 100;
pub type KeyValueData = [u8; KEY_VALUE_DATA_SIZE];

/// Core api's

/// SDKRuntimeRequest::Ping
#[derive(Serialize, Deserialize)]
pub struct PingRequest {}

/// SDKRuntimeRequest::Log
#[derive(Serialize, Deserialize)]
pub struct LogRequest<'a> {
    pub msg: &'a [u8],
}

/// SecurityCoordinator key-value api's

/// SDKRuntimeRequest::ReadKey
#[derive(Serialize, Deserialize)]
pub struct ReadKeyRequest<'a> {
    pub key: &'a str,
}
#[derive(Serialize, Deserialize)]
pub struct ReadKeyResponse<'a> {
    pub value: &'a [u8],
}

/// SDKRuntimeRequest::WriteKey
#[derive(Serialize, Deserialize)]
pub struct WriteKeyRequest<'a> {
    pub key: &'a str,
    pub value: &'a [u8],
}

/// SDKRuntimeRequest::DeleteKey
#[derive(Serialize, Deserialize)]
pub struct DeleteKeyRequest<'a> {
    pub key: &'a str,
}

/// TimerService api's

pub type TimerId = u32;
pub type TimerDuration = u32;

/// SDKRuntimeRequest::TimerOneshot and SDKRuntimeRequest::TimerPeriodic
#[derive(Serialize, Deserialize)]
pub struct TimerStartRequest {
    pub id: TimerId,
    pub duration_ms: TimerDuration,
}

/// SDKRuntimeRequest::TimerCancel
#[derive(Serialize, Deserialize)]
pub struct TimerCancelRequest {
    pub id: TimerId,
}

/// SDKRuntimeRequest::TimerWait
#[derive(Serialize, Deserialize)]
pub struct TimerWaitRequest {}
#[derive(Serialize, Deserialize)]
pub struct TimerWaitResponse {
    pub id: TimerId,
}

/// SDKRequest token sent over the seL4 IPC interface. We need repr(seL4_Word)
/// but cannot use that so use the implied usize type instead.
#[repr(usize)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
pub enum SDKRuntimeRequest {
    Ping = 0, // Check runtime is alive
    Log,      // Log message: [msg: &str]

    ReadKey,   // Read key: [key: &str, &mut [u8]] -> value: &[u8]
    WriteKey,  // Write key: [key: &str, value: &KeyValueData]
    DeleteKey, // Delete key: [key: &str]

    OneshotTimer,  // One-shot timer: [id: TimerId, duration_ms: TimerDuration]
    PeriodicTimer, // Periodic timer: [id: TimerId, duration_ms: TimerDuration]
    CancelTimer,   // Cancel timer: [id: TimerId]
    WaitForTimer,  // Wait for timer to expire: [id: TimerId]
}

/// Rust interface for the SDKRuntime.
///
/// This trait defines all of the same verbs we expect to support in the component
/// interface, for both client and server, since CAmkES does not (yet) know how
/// to generate Rust bindings.
///
/// On the server side, the impl of this trait is instantiated in the component
/// as a global mutable object where the incoming calls from the CAmkES C side
/// are wrapped.
///
/// On the client side, this trait is implemented using top-level functions.
pub trait SDKRuntimeInterface {
    /// Pings the SDK runtime, going from client to server and back via CAmkES IPC.
    fn ping(&self, app_id: SDKAppId) -> Result<(), SDKError>;

    /// Logs |msg| through the system logger.
    fn log(&self, app_id: SDKAppId, msg: &str) -> Result<(), SDKError>;

    /// Returns any value for the specified |key| in the app's  private key-value store.
    /// Data are written to |keyval| and returned as a slice.
    fn read_key<'a>(
        &self,
        app_id: SDKAppId,
        key: &str,
        keyval: &'a mut [u8],
    ) -> Result<&'a [u8], SDKError>;

    /// Writes |value| for the specified |key| in the app's private key-value store.
    fn write_key(&self, app_id: SDKAppId, key: &str, value: &KeyValueData) -> Result<(), SDKError>;

    /// Deletes the specified |key| in the app's private key-value store.
    fn delete_key(&self, app_id: SDKAppId, key: &str) -> Result<(), SDKError>;

    /// Create a one-shot timer named |id| of |duration_ms|.
    fn timer_oneshot(
        &self,
        app_id: SDKAppId,
        id: TimerId,
        duration_ms: TimerDuration,
    ) -> Result<(), SDKError>;
    /// Create a periodic (repeating) timer named |id| of |duration_ms|.
    fn timer_periodic(
        &self,
        app_id: SDKAppId,
        id: TimerId,
        duration_ms: TimerDuration,
    ) -> Result<(), SDKError>;
    /// Cancel a previously created timer.
    fn timer_cancel(&self, app_id: SDKAppId, id: TimerId) -> Result<(), SDKError>;
    /// Wait for any running timer to complete.
    fn timer_wait(&self, app_id: SDKAppId) -> Result<TimerId, SDKError>;
}

/// Rust client-side request processing. Note there is no CAmkES stub to
/// call; everything is done here. A single page frame is attached to the
/// IPC buffer with request parameters in the first half and return values
/// in the second half. Requests must have an SDKRequestHeader written to
/// the label field of the MessageInfo. Responses must have an SDKRuntimeError
/// written to the label field of the reply. For the moment this uses
/// postcard for serde work; this may change in the future (e.g. to flatbuffers).
///
/// The caller is responsible for synchronizing access to CANTRIP_SDK_* state
/// and the IPC buffer.
//
// TODO(sleffler): this attaches the call params to the IPC; might be
//   better to keep the page(s) mapped in SDKRuntime to avoid map/unmap
//   per-RPC but that requires a vspace allocator (or somerthing special
//   purpose) and a redesign of the server side to use the endpoint badge
//   to lookup the mapped page early. Downside to a fixed mapping is it
//   limits how to handle requests w/ different-sized params (e.g. sensor
//   frame vs key-value params).
fn sdk_request<'a, S: Serialize, D: Deserialize<'a>>(
    request: SDKRuntimeRequest,
    request_args: &S,
) -> Result<D, SDKRuntimeError> {
    let params_slice = unsafe { core::slice::from_raw_parts_mut(CANTRIP_SDK_PARAMS, PAGE_SIZE) };

    // NB: server-side must do the same split
    let (request_slice, reply_slice) = params_slice.split_at_mut(SDKRUNTIME_REQUEST_DATA_SIZE);
    reply_slice.fill(0); // XXX paranoid, could zero-pad request too

    // Encode request arguments.
    let _ = postcard::to_slice(request_args, request_slice)
        .map_err(|_| SDKRuntimeError::SDKSerializeFailed)?;

    // Attach params & call the SDKRuntime; then wait (block) for a reply.
    unsafe {
        seL4_SetCap(0, CANTRIP_SDK_FRAME);
        let info = seL4_Call(
            CANTRIP_SDK_ENDPOINT,
            seL4_MessageInfo::new(
                /*label=*/ request.into(),
                /*capsUnrapped=*/ 0,
                /*extraCaps=*/ 1,
                /*length=*/ 0,
            ),
        );
        seL4_SetCap(0, 0);

        let status = SDKRuntimeError::try_from(info.get_label())
            .map_err(|_| SDKRuntimeError::SDKUnknownResponse)?;
        if status != SDKRuntimeError::SDKSuccess {
            return Err(status);
        }
    }

    // Decode response data.
    postcard::from_bytes::<D>(reply_slice).map_err(|_| SDKRuntimeError::SDKDeserializeFailed)
}

/// Rust client-side wrapper for the ping method.
#[inline]
#[allow(dead_code)]
pub fn sdk_ping() -> Result<(), SDKRuntimeError> {
    sdk_request::<PingRequest, ()>(SDKRuntimeRequest::Ping, &PingRequest {})
}

/// Rust client-side wrapper for the log method.
#[inline]
#[allow(dead_code)]
pub fn sdk_log(msg: &str) -> Result<(), SDKRuntimeError> {
    sdk_request::<LogRequest, ()>(
        SDKRuntimeRequest::Log,
        &LogRequest {
            msg: msg.as_bytes(),
        },
    )
}

/// Rust client-side wrapper for the read key method.
// TODO(sleffler): _mut variant?
#[inline]
#[allow(dead_code)]
pub fn sdk_read_key<'a>(key: &str, keyval: &'a mut [u8]) -> Result<&'a [u8], SDKRuntimeError> {
    let response = sdk_request::<ReadKeyRequest, ReadKeyResponse>(
        SDKRuntimeRequest::ReadKey,
        &ReadKeyRequest { key },
    )?;
    keyval.copy_from_slice(response.value);
    Ok(keyval)
}

/// Rust client-side wrapper for the write key method.
#[inline]
#[allow(dead_code)]
pub fn sdk_write_key(key: &str, value: &[u8]) -> Result<(), SDKRuntimeError> {
    sdk_request::<WriteKeyRequest, ()>(SDKRuntimeRequest::WriteKey, &WriteKeyRequest { key, value })
}

/// Rust client-side wrapper for the delete key method.
#[inline]
#[allow(dead_code)]
pub fn sdk_delete_key(key: &str) -> Result<(), SDKRuntimeError> {
    sdk_request::<DeleteKeyRequest, ()>(SDKRuntimeRequest::DeleteKey, &DeleteKeyRequest { key })
}

/// Rust client-side wrapper for the timer_oneshot method.
#[inline]
#[allow(dead_code)]
pub fn sdk_timer_oneshot(id: TimerId, duration_ms: TimerDuration) -> Result<(), SDKRuntimeError> {
    sdk_request::<TimerStartRequest, ()>(
        SDKRuntimeRequest::OneshotTimer,
        &TimerStartRequest { id, duration_ms },
    )
}

/// Rust client-side wrapper for the timer_periodic method.
#[inline]
#[allow(dead_code)]
pub fn sdk_timer_periodic(id: TimerId, duration_ms: TimerDuration) -> Result<(), SDKRuntimeError> {
    sdk_request::<TimerStartRequest, ()>(
        SDKRuntimeRequest::PeriodicTimer,
        &TimerStartRequest { id, duration_ms },
    )
}

/// Rust client-side wrapper for the timer_cancel method.
#[inline]
#[allow(dead_code)]
pub fn sdk_timer_cancel(id: TimerId) -> Result<(), SDKRuntimeError> {
    sdk_request::<TimerCancelRequest, ()>(
        SDKRuntimeRequest::CancelTimer,
        &TimerCancelRequest { id },
    )
}

/// Rust client-side wrapper for the timer_wait method.
#[inline]
#[allow(dead_code)]
pub fn sdk_timer_wait() -> Result<TimerId, SDKRuntimeError> {
    let response = sdk_request::<TimerWaitRequest, TimerWaitResponse>(
        SDKRuntimeRequest::WaitForTimer,
        &TimerWaitRequest {},
    )?;
    Ok(response.id)
}
