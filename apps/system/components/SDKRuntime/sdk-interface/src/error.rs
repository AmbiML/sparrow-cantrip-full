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

use num_enum::TryFromPrimitive;

/// Rust Error enum used for representing an SDK error with postcard. This is
/// what most rust components will actually use as their error handling enum.
#[derive(Debug, Eq, PartialEq)]
pub enum SDKError {
    DeserializeFailed,
    SerializeFailed,
    InvalidBadge,
    InvalidString,
    ReadKeyFailed,
    WriteKeyFailed,
    DeleteKeyFailed,
    MapPageFailed,
    UnknownRequest,
    UnknownResponse,
    NoSuchTimer,
    TimerAlreadyExists,
    NoPlatformSupport,
    NoSuchModel,
    InvalidTimer,
    LoadModelFailed,
    OutOfResources,
    NoModelOutput,
}

impl From<postcard::Error> for SDKError {
    fn from(_err: postcard::Error) -> SDKError { SDKError::SerializeFailed }
}

/// SDKError presented over the seL4 IPC interface. We need repr(seL4_Word)
/// but cannot use that so use the implied usize type instead.
#[repr(usize)]
#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum SDKRuntimeError {
    SDKSuccess = 0,
    SDKDeserializeFailed,
    SDKSerializeFailed,
    SDKInvalidBadge,
    SDKInvalidString,
    SDKReadKeyFailed,
    SDKWriteKeyFailed,
    SDKDeleteKeyFailed,
    SDKMapPageFailed,
    SDKUnknownRequest,
    SDKUnknownResponse,
    SDKNoSuchTimer,
    SDKTimerAlreadyExists,
    SDKNoPlatformSupport,
    SDKNoSuchModel,
    SDKInvalidTimer,
    SDKLoadModelFailed,
    SDKOutOfResources,
    SDKNoModelOutput,
}

/// Mapping function from Rust -> C.
impl From<SDKError> for SDKRuntimeError {
    fn from(err: SDKError) -> SDKRuntimeError {
        match err {
            SDKError::DeserializeFailed => SDKRuntimeError::SDKDeserializeFailed,
            SDKError::SerializeFailed => SDKRuntimeError::SDKSerializeFailed,
            SDKError::InvalidBadge => SDKRuntimeError::SDKInvalidBadge,
            SDKError::InvalidString => SDKRuntimeError::SDKInvalidString,
            SDKError::ReadKeyFailed => SDKRuntimeError::SDKReadKeyFailed,
            SDKError::WriteKeyFailed => SDKRuntimeError::SDKWriteKeyFailed,
            SDKError::DeleteKeyFailed => SDKRuntimeError::SDKDeleteKeyFailed,
            SDKError::MapPageFailed => SDKRuntimeError::SDKMapPageFailed,
            SDKError::NoSuchTimer => SDKRuntimeError::SDKNoSuchTimer,
            SDKError::TimerAlreadyExists => SDKRuntimeError::SDKTimerAlreadyExists,
            SDKError::UnknownRequest => SDKRuntimeError::SDKUnknownRequest,
            SDKError::UnknownResponse => SDKRuntimeError::SDKUnknownResponse,
            SDKError::NoPlatformSupport => SDKRuntimeError::SDKNoPlatformSupport,
            SDKError::NoSuchModel => SDKRuntimeError::SDKNoSuchModel,
            SDKError::InvalidTimer => SDKRuntimeError::SDKInvalidTimer,
            SDKError::LoadModelFailed => SDKRuntimeError::SDKLoadModelFailed,
            SDKError::OutOfResources => SDKRuntimeError::SDKOutOfResources,
            SDKError::NoModelOutput => SDKRuntimeError::SDKNoModelOutput,
        }
    }
}

/// Helper to map from a Result and SDKError to C enum mapping.
impl From<Result<(), SDKError>> for SDKRuntimeError {
    fn from(result: Result<(), SDKError>) -> SDKRuntimeError {
        result.map_or_else(SDKRuntimeError::from, |_| SDKRuntimeError::SDKSuccess)
    }
}

/// Inverse mapping function from C -> Rust Result.
impl From<SDKRuntimeError> for Result<(), SDKError> {
    fn from(err: SDKRuntimeError) -> Result<(), SDKError> {
        match err {
            SDKRuntimeError::SDKSuccess => Ok(()),
            SDKRuntimeError::SDKDeserializeFailed => Err(SDKError::DeserializeFailed),
            SDKRuntimeError::SDKSerializeFailed => Err(SDKError::SerializeFailed),
            SDKRuntimeError::SDKInvalidBadge => Err(SDKError::InvalidBadge),
            SDKRuntimeError::SDKInvalidString => Err(SDKError::InvalidString),
            SDKRuntimeError::SDKReadKeyFailed => Err(SDKError::ReadKeyFailed),
            SDKRuntimeError::SDKWriteKeyFailed => Err(SDKError::WriteKeyFailed),
            SDKRuntimeError::SDKDeleteKeyFailed => Err(SDKError::DeleteKeyFailed),
            SDKRuntimeError::SDKMapPageFailed => Err(SDKError::DeleteKeyFailed),
            SDKRuntimeError::SDKNoSuchTimer => Err(SDKError::NoSuchTimer),
            SDKRuntimeError::SDKTimerAlreadyExists => Err(SDKError::TimerAlreadyExists),
            SDKRuntimeError::SDKUnknownRequest => Err(SDKError::UnknownRequest),
            SDKRuntimeError::SDKUnknownResponse => Err(SDKError::UnknownResponse),
            SDKRuntimeError::SDKNoPlatformSupport => Err(SDKError::NoPlatformSupport),
            SDKRuntimeError::SDKNoSuchModel => Err(SDKError::NoSuchModel),
            SDKRuntimeError::SDKInvalidTimer => Err(SDKError::InvalidTimer),
            SDKRuntimeError::SDKLoadModelFailed => Err(SDKError::LoadModelFailed),
            SDKRuntimeError::SDKOutOfResources => Err(SDKError::OutOfResources),
            SDKRuntimeError::SDKNoModelOutput => Err(SDKError::NoModelOutput),
        }
    }
}
