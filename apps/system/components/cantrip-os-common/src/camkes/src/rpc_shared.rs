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

/// RPC mechanism that uses a per-thread shared page for passing parameters.

/// The rpc_shared_*  macros are the intended api's; they hide the naming
/// conventions for CAmkES artifacts. The macros do minimal work before
/// invoking Rust implementations that can also be used directly.
use crate::Camkes;
use core::convert::From;

use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_Call;
use sel4_sys::seL4_MessageInfo;
use sel4_sys::seL4_Recv;
use sel4_sys::seL4_ReplyRecv;
use sel4_sys::seL4_Send;
use sel4_sys::seL4_Word;

/// Server-side interface to get the shared page for the client with
/// the specified badge.
pub type GetRecvBuffer = fn(seL4_Word) -> &'static mut [u8];

/// Returns a reference to the per-thread shared page for processing RPC parameters.
/// CAmkES sets up a page that is shared between client & server for each endpoint.
/// On the client side this is exposed through a name-mangled per-thread function.
/// On the server side the sender's seL4 IPC badge is used to identify which page
/// holds the parameter data.
#[macro_export]
macro_rules! rpc_shared_buffer {
    ($inf_tag:ident) => {
        $crate::paste! {
            unsafe {
                extern "C" {
                    fn [<$inf_tag _interface_shared_buffer>]() -> &'static [u8];
                }
                [<$inf_tag _interface_shared_buffer>]()
            }
        }
    };
}
#[macro_export]
macro_rules! rpc_shared_buffer_mut {
    ($inf_tag:ident) => {
        $crate::paste! {
            unsafe {
                extern "C" {
                    fn [<$inf_tag _interface_shared_buffer_mut>]() -> &'static mut [u8];
                }
                [<$inf_tag _interface_shared_buffer_mut>]()
            }
        }
    };
}

/// Sends an RPC message and blocks waiting for a reply. The message contents
/// are assumed to be marshalled in the storage returned by rpc_shared_buffer!().
/// Data in the msg buffer are assumed partitioned into <request><reply> slices
/// with the <request> slice starting at offset 0 into the slice/buffer.
/// An optional capability can be attached to the message.
pub unsafe fn send(endpoint: seL4_CPtr, opt_cap: Option<seL4_CPtr>) -> usize {
    if let Some(cap) = opt_cap {
        let _cleanup = Camkes::set_request_cap(cap);
        seL4_Call(
            endpoint,
            seL4_MessageInfo::new(
                /*label=*/ 0, /*capsUnwrapped=*/ 0,
                /*extraCaps=*/ 1, // NB: attached capability
                /*length=*/ 0,
            ),
        )
    } else {
        // XXX still needed?
        Camkes::clear_request_cap();
        seL4_Call(
            endpoint,
            seL4_MessageInfo::new(
                /*label=*/ 0, /*capsUnwrapped=*/ 0, /*extraCaps=*/ 0,
                /*length=*/ 0,
            ),
        )
    }
    .get_label()
}

#[macro_export]
macro_rules! rpc_shared_send {
    ($inf_tag:ident, $opt_cap:expr) => {
        $crate::paste! {
            rpc_shared_send!(@end [<$inf_tag:upper _INTERFACE_ENDPOINT>], $opt_cap)
        }
    };
    (@end $inf_endpoint:ident, $opt_cap:expr) => {
        unsafe {
            extern "C" {
                static $inf_endpoint: sel4_sys::seL4_CPtr;
            }
            // XXX .into()?
            crate::camkes::rpc_shared::send($inf_endpoint, $opt_cap)
        }
    };
}

/// Callback required by the server side implementation of the shared RPC
/// mechanism. The receiving thread calls |recv_loop| which handles the
/// transport mechanics and passes each message to |Self::dispatch|
/// which returns either an error or |reply_slice| data for the reply.
/// The reply parameters are assumed to be already marshalled in |reply_slice|
/// which is shared with the caller (so not copied). Note that |request_slice|
/// and |reply_slice| point into the per-thread shared buffer without
/// synchronization.
///
/// The error |E| is required to support the From trait for constructing
/// the return status code passed in seL4_IPCBuffer::label field.
///
/// No capabilities are sent or received with this interface; see below
/// for the |CapDispatch| type and |recv_*| api's.
type Dispatch<E> = fn(
    client_badge: seL4_Word, // Sender's seL4 connection badge
    request_slice: &[u8],    // Serialized request data
    reply_slice: &mut [u8],  // Buffer for serializing reply data
) -> Result<(), E>;

/// Server-side implementation for shared RPC without any capability
/// passing. This is normally invoked by the |rpc_shared_recv| macro.
pub unsafe fn recv_loop<E>(
    dispatch: Dispatch<E>,
    endpoint: seL4_CPtr,
    reply: seL4_CPtr,
    get_recv_buffer: GetRecvBuffer,
    request_size: usize,
    success: E,
) -> !
where
    usize: From<E>,
{
    let success_word: seL4_Word = success.into();

    let mut client_badge: seL4_Word = 0;
    seL4_Recv(
        /*src=*/ endpoint,
        /*sender=*/ &mut client_badge as _,
        /*reply=*/ reply,
    );
    loop {
        let (request_slice, reply_slice) = get_recv_buffer(client_badge).split_at_mut(request_size);
        let request_slice = &*request_slice; // NB: immutable alias

        let response = dispatch(client_badge, request_slice, reply_slice);

        seL4_ReplyRecv(
            /*src=*/ endpoint,
            /*msgInfo=*/
            seL4_MessageInfo::new(
                /*label=*/ response.map_or_else(|e| e.into(), |_| success_word),
                /*capsUnwrapped=*/ 0,
                /*extraCaps=*/ 0,
                /*length=*/ 0,
            ),
            /*sender=*/ &mut client_badge as _,
            /*reply=*/ reply,
        );
    }
}

/// Server-side implementation for shared RPC without any capability
/// passing. This macro is normally invoked in a CamkesInterfaceThread's
/// run method to implement the server side of a cantripRPCOverMultiSharedData
/// connection. The thread must implement the Dispatch callback to handle
/// deserialization of marshalled parameters and processing of the request(s).
#[macro_export]
macro_rules! rpc_shared_recv {
    ($inf_tag:ident, $inf_request_size:expr, $inf_success:expr) => {
        $crate::paste! {
            rpc_shared_recv!(@end
                Self::dispatch,
                [<$inf_tag:upper _INTERFACE_ENDPOINT>],
                [<$inf_tag:upper _INTERFACE_REPLY>],
                [<$inf_tag _interface_recv_buffer>],
                $inf_request_size,
                $inf_success
            );
        }
    };
    (@end $inf_dispatch:expr, $inf_endpoint:ident, $inf_reply:ident, $inf_recv_buffer:ident, $inf_request_size:expr, $inf_success:expr) => {
        unsafe {
            crate::camkes::rpc_shared::recv_loop(
                $inf_dispatch,
                $inf_endpoint,
                $inf_reply,
                $inf_recv_buffer,
                $inf_request_size,
                $inf_success,
            )
        }
    };
}

/// Callback required by the server side implementation of the shared RPC
/// mechanism when optionally passing a capability in a reply. This is
/// just like |Dispatch| except the return from |CapDispatch| may include
/// a capability to be attached to the IPC buffer.
type CapDispatch<E> = fn(
    client_badge: seL4_Word, // Sender's seL4 connection badge
    request_slice: &[u8],    // Serialized request data
    reply_slice: &mut [u8],  // Buffer for serializing reply data
) -> Result<Option<seL4_CPtr>, E>;

// Shared handling of RPC reply messages that optionally have a capability.
unsafe fn reply_with_cap<E>(
    response: Result<Option<seL4_CPtr>, E>,
    client_badge: &mut seL4_Word,
    endpoint: seL4_CPtr,
    reply: seL4_CPtr,
    success_word: seL4_Word,
) where
    usize: From<E>,
{
    if let Ok(Some(cap)) = response {
        // NB: Cannot use ReplyRecv here because the cleanup of
        // the reply capability will run after the receive and this
        // work reuses the IPCBuffer (for CNode_Delete_Path). Split
        // the ReplyRecv into a Reply followed by cleanup followed by
        // Recv. But beware there is no seL4_Reply defined for MCS
        // configured systems; you need to seL4_Send to the reply cap.
        {
            let _cleanup = Camkes::set_reply_cap_release(cap);
            seL4_Send(
                /*src=*/ reply,
                /*msgInfo=*/
                seL4_MessageInfo::new(
                    /*label=*/ response.map_or_else(|e| e.into(), |_| success_word),
                    /*capsUnwrapped=*/ 0,
                    /*extraCaps=*/ 1, // NB: attached capability
                    /*length=*/ 0,
                ),
            );
        }
        seL4_Recv(
            /*src=*/ endpoint,
            /*sender=*/ client_badge,
            /*reply=*/ reply,
        );
    } else {
        seL4_ReplyRecv(
            /*src=*/ endpoint,
            /*msgInfo=*/
            seL4_MessageInfo::new(
                /*label=*/ response.map_or_else(|e| e.into(), |_| success_word),
                /*capsUnwrapped=*/ 0,
                /*extraCaps=*/ 0,
                /*length=*/ 0,
            ),
            /*sender=*/ client_badge,
            /*reply=*/ reply,
        );
    }
}

/// Server-side implementation for shared RPC with an optional capability
/// attached to a request and/or reply. This is normally invoked by the
/// |rpc_shared_recv_with_caps| macro.
///
/// The caller is responsible for extracting and saving any received capability;
/// otherwise |recv_with_caps_loop| will zero the capability in the IPC buffer
/// and delete anything present in the receive capability slot.
/// Any capability to be attached to a reply must be present in the return
/// from |CapDispatch|; |recv_with_caps_loop| will automatically attach the
/// capability to the IPC message and cleanup local state after the reply
/// completes.
pub unsafe fn recv_with_caps_loop<E>(
    dispatch: CapDispatch<E>,
    endpoint: seL4_CPtr,
    reply: seL4_CPtr,
    get_recv_buffer: GetRecvBuffer,
    recv_slot: seL4_CPtr,
    request_size: usize,
    success: E,
) -> !
where
    usize: From<E>,
{
    let recv_path = &Camkes::top_level_path(recv_slot);
    extern "Rust" {
        static CAMKES: Camkes;
    }
    CAMKES.init_recv_path(recv_path);
    Camkes::debug_assert_slot_empty("run", recv_path); // XXX

    let success_word: seL4_Word = success.into();

    let mut client_badge: seL4_Word = 0;
    seL4_Recv(
        /*src=*/ endpoint,
        /*sender=*/ &mut client_badge as _,
        /*reply=*/ reply,
    );
    loop {
        // XXX could check info.extraCaps
        Camkes::clear_request_cap();

        let (request_slice, reply_slice) = get_recv_buffer(client_badge).split_at_mut(request_size);
        let request_slice = &*request_slice; // NB: immutable alias

        let response = dispatch(client_badge, request_slice, reply_slice);

        Camkes::delete_path(recv_path).expect("delete");

        reply_with_cap::<E>(response, &mut client_badge as _, endpoint, reply, success_word);
    }
}

/// Server-side implementation for shared RPC with optional capability
/// passing in both directions. This macro is normally invoked in a
/// CamkesInterfaceThread's run method to implement the server side of
/// a cantripRPCOverMultiSharedData connection. The thread
/// must implement the CapDispatch callback to handle deserialization of
/// marshalled parameters and processing of the request(s). Received
/// capabilities are retrieved directly from the "receive slot", typically
/// using Camkes::get_owned_current_recv_path(). Any reply capability is
/// included in the return value of |CapDispatch|
/// TODO: send vs dup of reply cap
#[macro_export]
macro_rules! rpc_shared_recv_with_caps {
    ($inf_tag:ident, $inf_recv_slot:expr, $inf_request_size:expr, $inf_success:expr) => {
        $crate::paste! {
            rpc_shared_recv_with_caps!(@end
                Self::dispatch,
                [<$inf_tag:upper _INTERFACE_ENDPOINT>],
                [<$inf_tag:upper _INTERFACE_REPLY>],
                [<$inf_tag _interface_recv_buffer>],
                $inf_recv_slot,
                $inf_request_size,
                $inf_success
            );
        }
    };
    (@end $inf_dispatch:expr, $inf_endpoint:ident, $inf_reply:ident, $inf_recv_buffer:ident, $inf_recv_slot:expr, $inf_request_size:expr, $inf_success:expr) => {
        unsafe {
            crate::camkes::rpc_shared::recv_with_caps_loop(
                $inf_dispatch,
                $inf_endpoint,
                $inf_reply,
                $inf_recv_buffer,
                $inf_recv_slot,
                $inf_request_size,
                $inf_success,
            )
        }
    };
}
