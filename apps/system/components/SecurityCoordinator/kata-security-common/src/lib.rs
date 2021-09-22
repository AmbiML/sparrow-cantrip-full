//! Cantrip OS Security Coordinator support

#![cfg_attr(not(test), no_std)]

extern crate alloc;
use alloc::string::String;
use core::str;
use serde::{Deserialize, Serialize};

// NB: serde helper for arrays w/ >32 elements
//   c.f. https://github.com/serde-rs/serde/pull/1860
use serde_big_array::big_array;
big_array! { BigArray; }

// Size of the buffers used to pass serialized data between Rust <> C.
// The data structure size is bounded by the camkes ipc buffer (2K bytes!)
// and also by it being allocated on the stack of the rpc glue code.
// So we need to balance these against being able to handle all values.

pub const SECURITY_REQUEST_DATA_SIZE: usize = 2048;
pub type SecurityRequestData = [u8; SECURITY_REQUEST_DATA_SIZE];

pub const SECURITY_REPLY_DATA_SIZE: usize = 2048;
pub type SecurityReplyData = [u8; SECURITY_REPLY_DATA_SIZE];

// NB: struct's marked repr(C) are processed by cbindgen to get a .h file
//   used in camkes C interfaces.

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

// TODO(sleffler): convert String to &str

// NB: SecurityRequestInstall is handled specially.

// SecurityRequestUninstall
#[derive(Debug, Serialize, Deserialize)]
pub struct UninstallRequest {
    pub bundle_id: String,
}

// SecurityRequestSizeBuffer
#[derive(Debug, Serialize, Deserialize)]
pub struct SizeBufferRequest {
    pub bundle_id: String,
}

// SecurityRequestGetManifest
#[derive(Debug, Serialize, Deserialize)]
pub struct GetManifestRequest {
    pub bundle_id: String,
}

// SecurityRequestLoadApplication
#[derive(Debug, Serialize, Deserialize)]
pub struct LoadApplicationRequest {
    pub bundle_id: String,

    // Scatter-list of shared memory pages where application should be loaded.
    // TODO(sleffler) scatter list
    #[serde(with = "mut_ptr_helper")]
    pub app_binary: *mut u8,
}

// SecurityRequestLoadModel
#[derive(Debug, Serialize, Deserialize)]
pub struct LoadModelRequest {
    pub bundle_id: String,
    pub model_id: String,

    // Scatter-list of shared memory pages where model should be loaded.
    // TODO(sleffler) scatter list
    #[serde(with = "mut_ptr_helper")]
    pub model_binary: *mut u8,
}

// SecurityRequestReadKey
#[derive(Debug, Serialize, Deserialize)]
pub struct ReadKeyRequest {
    pub bundle_id: String,
    pub key: String,
}

// SecurityRequestWriteKey
#[derive(Debug, Serialize, Deserialize)]
pub struct WriteKeyRequest<'a> {
    pub bundle_id: String,
    pub key: String,
    pub value: &'a [u8],
}

// SecurityRequestDeleteKey
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteKeyRequest {
    pub bundle_id: String,
    pub key: String,
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
pub fn cantrip_security_request(
    request: SecurityRequest,
    request_buffer: &[u8],
    reply_buffer: &mut SecurityReplyData,
) -> SecurityRequestError {
    // NB: this assumes the SecurityCoordinator component is named "security".
    extern "C" {
        pub fn security_request(
            c_request: SecurityRequest,
            c_request_buffer_len: u32,
            c_request_buffer: *const u8,
            c_reply_buffer: *mut SecurityReplyData,
        ) -> SecurityRequestError;
    }
    unsafe {
        security_request(
            request,
            request_buffer.len() as u32,
            request_buffer.as_ptr(),
            reply_buffer as *mut _,
        )
    }
}
