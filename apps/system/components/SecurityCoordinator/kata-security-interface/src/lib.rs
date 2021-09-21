//! Cantrip OS Security Coordinator support

#![cfg_attr(not(test), no_std)]

use core::str;
use postcard;
use serde::{Deserialize, Serialize};

// NB: serde helper for arrays w/ >32 elements
//   c.f. https://github.com/serde-rs/serde/pull/1860
use serde_big_array::big_array;
big_array! { BigArray; }

// Size of the buffers used to pass serialized data between Rust <> C.
// The data structure size is bounded by the camkes ipc buffer (2K bytes!)
// and also by it being allocated on the stack of the rpc glue code.
// So we need to balance these against being able to handle all values.

const SECURITY_REQUEST_DATA_SIZE: usize = 2048;

pub const SECURITY_REPLY_DATA_SIZE: usize = 2048;
pub type SecurityReplyData = [u8; SECURITY_REPLY_DATA_SIZE];

// NB: struct's marked repr(C) are processed by cbindgen to get a .h file
//   used in camkes C interfaces.

mod ptr_helper {
    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<T, S>(ptr: &*const T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (*ptr as usize).serialize(serializer)
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<*const T, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(usize::deserialize(deserializer)? as *const T)
    }
}

mod mut_ptr_helper {
    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<T, S>(ptr: &*mut T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (*ptr as usize).serialize(serializer)
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<*mut T, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(usize::deserialize(deserializer)? as *mut T)
    }
}

// SecurityRequestEcho
#[derive(Debug, Serialize, Deserialize)]
pub struct EchoRequest<'a> {
    pub value: &'a [u8],
}

// SecurityRequestInstall
#[derive(Debug, Serialize, Deserialize)]
pub struct InstallRequest {
    pub pkg_buffer_size: u32,

    // Gather-list of shared memory pages holding package data.
    // TODO(sleffler) gather list
    #[serde(with = "ptr_helper")]
    pub pkg_buffer: *const u8,
}

// SecurityRequestUninstall
#[derive(Debug, Serialize, Deserialize)]
pub struct UninstallRequest<'a> {
    pub bundle_id: &'a str,
}

// SecurityRequestSizeBuffer
#[derive(Debug, Serialize, Deserialize)]
pub struct SizeBufferRequest<'a> {
    pub bundle_id: &'a str,
}

// SecurityRequestGetManifest
#[derive(Debug, Serialize, Deserialize)]
pub struct GetManifestRequest<'a> {
    pub bundle_id: &'a str,
}

// SecurityRequestLoadApplication
#[derive(Debug, Serialize, Deserialize)]
pub struct LoadApplicationRequest<'a> {
    pub bundle_id: &'a str,

    // Scatter-list of shared memory pages where application should be loaded.
    // TODO(sleffler) scatter list
    #[serde(with = "mut_ptr_helper")]
    pub app_binary: *mut u8,
}

// SecurityRequestLoadModel
#[derive(Debug, Serialize, Deserialize)]
pub struct LoadModelRequest<'a> {
    pub bundle_id: &'a str,
    pub model_id: &'a str,

    // Scatter-list of shared memory pages where model should be loaded.
    // TODO(sleffler) scatter list
    #[serde(with = "mut_ptr_helper")]
    pub model_binary: *mut u8,
}

// SecurityRequestReadKey
#[derive(Debug, Serialize, Deserialize)]
pub struct ReadKeyRequest<'a> {
    pub bundle_id: &'a str,
    pub key: &'a str,
}

// SecurityRequestWriteKey
#[derive(Debug, Serialize, Deserialize)]
pub struct WriteKeyRequest<'a> {
    pub bundle_id: &'a str,
    pub key: &'a str,
    pub value: &'a [u8],
}

// SecurityRequestDeleteKey
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteKeyRequest<'a> {
    pub bundle_id: &'a str,
    pub key: &'a str,
}

// NB: this is the union of InstallInterface & StorageInterface because
//   the camkes-generated interface code uses basic C which does not
//   tolerate overlapping member names.
#[repr(C)]
#[derive(Debug, Eq, PartialEq)]
pub enum SecurityRequestError {
    SreSuccess = 0,
    SreBundleIdInvalid,
    SreBundleDataInvalid,
    SreBundleNotFound,
    SreKeyNotFound,
    SrePackageBufferLenInvalid,
    SreValueInvalid,
    SreKeyInvalid,
    SreSerializeFailed,
    // Generic errors, mostly used in unit tests
    SreEchoFailed,
    SreInstallFailed,
    SreUninstallFailed,
    SreSizeBufferFailed,
    SreGetManifestFailed,
    SreLoadApplicationFailed,
    SreLoadModelFailed,
    SreReadFailed,
    SreWriteFailed,
    SreDeleteFailed,
}

#[repr(C)]
#[derive(Debug, Eq, PartialEq)]
pub enum SecurityRequest {
    SrEcho = 0, // Security core replies with request payload

    SrInstall,   // Install package [pkg_buffer] -> bundle_id
    SrUninstall, // Uninstall package [bundle_id]

    SrSizeBuffer,      // Size application image [bundle_id] -> u32
    SrGetManifest,     // Return application manifest [bundle_id] -> String
    SrLoadApplication, // Load application [bundle_id]
    // TODO(sleffler): define <tag>?
    SrLoadModel, // Load ML model [bundle_id, <tag>]

    SrReadKey,   // Read key value [bundle_id, key] -> value
    SrWriteKey,  // Write key value [bundle_id, key, value]
    SrDeleteKey, // Delete key [bundle_id, key]
}

// Interface to underlying facilities; also used to inject fakes for unit tests.
pub trait SecurityCoordinatorInterface {
    fn request(
        &mut self,
        request_id: SecurityRequest,
        request_buffer: &[u8],
        reply_buffer: &mut [u8],
    ) -> Result<(), SecurityRequestError>;
}

// TODO(sleffler): try cantrip_security_request<T> to lower serde work
#[inline]
#[allow(dead_code)]
pub fn cantrip_security_request<T: Serialize>(
    request: SecurityRequest,
    request_args: &T,
    reply_buffer: &mut SecurityReplyData,
) -> Result<(), SecurityRequestError> {
    // NB: this assumes the SecurityCoordinator component is named "security".
    extern "C" {
        pub fn security_request(
            c_request: SecurityRequest,
            c_request_buffer_len: u32,
            c_request_buffer: *const u8,
            c_reply_buffer: *mut SecurityReplyData,
        ) -> SecurityRequestError;
    }
    let mut request_buffer = [0u8; SECURITY_REQUEST_DATA_SIZE];
    let _ = postcard::to_slice(request_args, &mut request_buffer[..])
        .map_err(|_| SecurityRequestError::SreSerializeFailed)?;
    match unsafe {
        security_request(
            request,
            request_buffer.len() as u32,
            request_buffer.as_ptr(),
            reply_buffer as *mut _,
        )
    } {
        SecurityRequestError::SreSuccess => Ok(()),
        status => Err(status),
    }
}
