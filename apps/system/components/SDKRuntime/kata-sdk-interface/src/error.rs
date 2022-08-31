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

use serde::{Deserialize, Serialize};

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
}

impl From<postcard::Error> for SDKError {
    fn from(_err: postcard::Error) -> SDKError { SDKError::SerializeFailed }
}

/// C-version of SDKError presented over the CAmkES rpc interface.
#[repr(C)]
#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SDKRuntimeError {
    SDKSuccess = 0,
    SDKDeserializeFailed,
    SDKSerializeFailed,
    SDKInvalidBadge,
    SDKInvalidString,
    SDKReadKeyFailed,
    SDKWriteKeyFailed,
    SDKDeleteKeyFailed,
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
        }
    }
}
