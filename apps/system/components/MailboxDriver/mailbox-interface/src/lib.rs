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
pub enum MailboxError {
    Success = 0,
    SerializeFailed,
    DeserializeFailed,
    #[default]
    UnknownError,
    // Generic errors.
    SendFailed,
    RecvFailed,
}
impl From<MailboxError> for Result<(), MailboxError> {
    fn from(err: MailboxError) -> Result<(), MailboxError> {
        if err == MailboxError::Success {
            Ok(())
        } else {
            Err(err)
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecvResponse {
    pub paddr: u32,
    pub size: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MailboxRequest {
    Send(u32, u32),
    Recv, // -> RecvResponse
}

pub const MAILBOX_REQUEST_DATA_SIZE: usize = 24;

#[inline]
fn mailbox_request<T: DeserializeOwned>(request: &MailboxRequest) -> Result<T, MailboxError> {
    let (request_buffer, reply_slice) = rpc_basic_buffer!().split_at_mut(MAILBOX_REQUEST_DATA_SIZE);
    let request_slice =
        postcard::to_slice(request, request_buffer).or(Err(MailboxError::SerializeFailed))?;
    match rpc_basic_send!(uart_read, request_slice.len()).0.into() {
        MailboxError::Success => {
            let reply =
                postcard::from_bytes(reply_slice).or(Err(MailboxError::DeserializeFailed))?;
            Ok(reply)
        }
        err => Err(err),
    }
}

#[inline]
pub fn mailbox_send(paddr: u32, size: u32) -> Result<(), MailboxError> {
    mailbox_request(&MailboxRequest::Send(paddr, size))
}

#[inline]
pub fn mailbox_recv() -> Result<(u32, u32), MailboxError> { mailbox_request(&MailboxRequest::Recv) }
