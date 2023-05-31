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

#![no_std]

use cantrip_os_common::camkes;
use cantrip_os_common::sel4_sys;
use num_enum::{FromPrimitive, IntoPrimitive};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use camkes::rpc_basic_buffer;
use camkes::rpc_basic_send;

#[repr(usize)]
#[derive(Debug, Default, Eq, PartialEq, FromPrimitive, IntoPrimitive)]
pub enum UartDriverError {
    Success = 0,
    SerializeFailed,
    DeserializeFailed,
    BadLimit,
    #[default]
    UnknownError,
    // Generic errors.
    ReadFailed,
    WriteFailed,
    FlushFailed,
}
impl From<UartDriverError> for Result<(), UartDriverError> {
    fn from(err: UartDriverError) -> Result<(), UartDriverError> {
        if err == UartDriverError::Success {
            Ok(())
        } else {
            Err(err)
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReadResponse {
    pub num_read: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ReadRequest {
    Read(usize),
}
pub const READ_REQUEST_DATA_SIZE: usize = 64;

#[inline]
fn uart_read_request<T: core::fmt::Debug + DeserializeOwned>(
    request: &ReadRequest,
) -> Result<T, UartDriverError> {
    let (request_buffer, reply_slice) = rpc_basic_buffer!().split_at_mut(READ_REQUEST_DATA_SIZE);
    let request_slice =
        postcard::to_slice(request, request_buffer).or(Err(UartDriverError::SerializeFailed))?;
    match rpc_basic_send!(uart_read, request_slice.len()).0.into() {
        UartDriverError::Success => {
            let reply =
                postcard::from_bytes(reply_slice).or(Err(UartDriverError::DeserializeFailed))?;
            Ok(reply)
        }
        err => Err(err),
    }
}

#[inline]
pub fn uart_read(limit: usize) -> Result<usize, UartDriverError> {
    uart_read_request(&ReadRequest::Read(limit)).map(|reply: ReadResponse| reply.num_read)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WriteResponse {
    pub num_written: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WriteRequest {
    Write(usize),
    Flush,
}
pub const WRITE_REQUEST_DATA_SIZE: usize = 24;

#[inline]
fn uart_write_request<T: DeserializeOwned>(request: &WriteRequest) -> Result<T, UartDriverError> {
    let (request_buffer, reply_slice) = rpc_basic_buffer!().split_at_mut(WRITE_REQUEST_DATA_SIZE);
    let request_slice =
        postcard::to_slice(request, request_buffer).or(Err(UartDriverError::SerializeFailed))?;
    match rpc_basic_send!(uart_write, request_slice.len()).0.into() {
        UartDriverError::Success => {
            let reply =
                postcard::from_bytes(reply_slice).or(Err(UartDriverError::DeserializeFailed))?;
            Ok(reply)
        }
        err => Err(err),
    }
}

#[inline]
pub fn uart_write(num_written: usize) -> Result<usize, UartDriverError> {
    uart_write_request(&WriteRequest::Write(num_written))
        .map(|reply: WriteResponse| reply.num_written)
}

#[inline]
pub fn uart_flush() -> Result<(), UartDriverError> { uart_write_request(&WriteRequest::Flush) }
