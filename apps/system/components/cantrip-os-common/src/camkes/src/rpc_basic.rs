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

/// RPC mechanism that uses the per-thread IPCBuffer for passing parameters.

/// The rpc_basic_*  macros are the intended api's; they hide the naming
/// conventions for CAmkES artifacts. The macros do minimal work before
/// invoking Rust implementations that can also be used directly.
use crate::Camkes;
use core::convert::From;
use core::mem::size_of;

use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_Call;
use sel4_sys::seL4_GetIPCBuffer;
use sel4_sys::seL4_MessageInfo;
use sel4_sys::seL4_MsgMaxLength;
use sel4_sys::seL4_Recv;
use sel4_sys::seL4_ReplyRecv;
use sel4_sys::seL4_Send;
use sel4_sys::seL4_Word;

/// Returns a reference to the per-thread IPCBuffer for processing RPC parameters.
/// This is safe to use when the buffer is not re-used before the RPC completes
/// (usually easy to guarantee on the client side but can be problematic on the
/// server side).
pub unsafe fn get_buffer_mut() -> &'static mut [u8] {
    core::slice::from_raw_parts_mut(
        (*seL4_GetIPCBuffer()).msg.as_mut_ptr() as *mut u8,
        seL4_MsgMaxLength * size_of::<seL4_Word>(),
    )
}

#[macro_export]
macro_rules! rpc_basic_buffer {
    () => {
        unsafe { crate::camkes::rpc_basic::get_buffer_mut() }
    };
}

/// Sends an RPC message and blocks waiting for a reply. The message contents
/// are assumed to be marshalled in the storage returned by get_buffer_mut().
/// Data in the msg buffer are assumed partitioned into <request><reply> slices
/// with the <request> slice starting at offset 0 into the slice/buffer.
/// Note the msg buffer is per-thread (TLS) but global and no synchronization
/// is done to guard against re-use.
pub unsafe fn send(endpoint: seL4_CPtr, request_len: usize) -> (usize, usize) {
    const WORD_SIZE: usize = size_of::<seL4_Word>();
    let info = seL4_Call(
        endpoint,
        seL4_MessageInfo::new(
            /*label=*/ 0,
            /*capsUnwrapped=*/ 0,
            /*extraCaps=*/ 0,
            /*length=*/ (request_len + WORD_SIZE - 1) / WORD_SIZE,
        ),
    );
    (info.get_label(), info.get_length())
}

#[macro_export]
macro_rules! rpc_basic_send {
    ($inf_tag:ident, $request_len:expr) => {
        $crate::paste! {
            rpc_basic_send!(@end
                [<$inf_tag:upper _INTERFACE_ENDPOINT>],
                $request_len
            )
        }
    };
    (@end $inf_endpoint:ident, $request_len:expr) => {
        unsafe {
            extern "C" {
                static $inf_endpoint: sel4_sys::seL4_CPtr;
            }
            crate::camkes::rpc_basic::send($inf_endpoint, $request_len)
        }
    };
}

/// Callback required by the server side implementation of the simple RPC
/// mechanism. The receiving thread calls |recv_loop| which handles the
/// transport mechanics and passes each mesage to |Self::dispatch|
/// which returns either an error or |reply_slice| data for the reply.
/// The reply parameters are assumed to already be marshalled in |reply_slice|
/// which is copied to the caller. Note that |request_slice| and |reply_slice|
/// point into the global per-thread buffer without synchronization.
///
/// The error |E| is required to support the Into trait for constructing
/// the return status code passed in seL4_IPCBuffer::label.
///
/// No capabilities are sent or received with this interface; see below
/// for the |CapDispatch| type and |recv_loop_*| api's.
type Dispatch<E> = fn(
    client_badge: seL4_Word, // Sender's seL4 connection badge
    request_slice: &[u8],    // Serialized request data
    reply_slice: &mut [u8],  // Buffer for serializing reply data
) -> Result<usize, E>;

/// Server-side implementation for basic RPC without any capability
/// passing. This is normally invoked by the |rpc_basic_recv| macro.
pub unsafe fn recv_loop<E>(
    dispatch: Dispatch<E>,
    endpoint: seL4_CPtr,
    reply: seL4_CPtr,
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
        let (request_slice, reply_slice) = get_buffer_mut().split_at_mut(request_size);
        let request_slice = &*request_slice; // NB: immutable alias

        let response = dispatch(client_badge, request_slice, reply_slice);
        let (label, length) = match response {
            Ok(len) => {
                const WORD_SIZE: usize = size_of::<seL4_Word>();
                // XXX sender expects reply at offset request_size
                (success_word, (request_size + len + WORD_SIZE - 1) / WORD_SIZE)
            }
            Err(e) => (e.into(), 0),
        };

        seL4_ReplyRecv(
            /*src=*/ endpoint,
            /*msgInfo=*/
            seL4_MessageInfo::new(
                /*label=*/ label, /*capsUnwrapped=*/ 0, /*extraCaps=*/ 0,
                /*length=*/ length,
            ),
            /*sender=*/ &mut client_badge as _,
            /*reply=*/ reply,
        );
    }
}

/// Server-side implementation for basic RPC without any capability
/// passing. This macro is normally invoked in a CamkesInterfaceThread's
/// run method to implement the server side of a cantripRPCCall or
/// cantripRPCCallSignal connection. The thread must implement the
/// Dispatch callback to handle deserialization of marshalled parameters
/// and processing of the request(s).
#[macro_export]
macro_rules! rpc_basic_recv {
    ($inf_tag:ident, $inf_request_size:expr, $inf_success:expr) => {
        $crate::paste! {
            rpc_basic_recv!(@end
                Self::dispatch,
                [<$inf_tag:upper _INTERFACE_ENDPOINT>],
                [<$inf_tag:upper _INTERFACE_REPLY>],
                $inf_request_size,
                $inf_success
            );
        }
    };
    (@end $inf_dispatch:expr, $inf_endpoint:ident, $inf_reply:ident, $inf_request_size:expr, $inf_success:expr) => {
        unsafe {
            crate::camkes::rpc_basic::recv_loop(
                $inf_dispatch,
                $inf_endpoint,
                $inf_reply,
                $inf_request_size,
                $inf_success,
            )
        }
    };
}

/// Callback required by the server side implementation of the simple RPC
/// mechanism when optionally passing a capability in a reply. This is
/// just like |Dispatch| except the return from |CapDispatch| indicates
/// whether a capability has been attached to the IPC buffer.
type CapDispatch<E> = fn(
    client_badge: seL4_Word, // Sender's seL4 connection badge
    request_slice: &[u8],    // Serialized request data
    reply_slice: &mut [u8],  // Buffer for serializing reply data
) -> Result<(usize, Option<seL4_CPtr>), E>;

/// Server-side implementation for basic RPC with an optional capability
/// attached to a request and/or reply. This is normally invoked by the
/// |rpc_basic_recv_with_caps| macro.
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

        let (request_slice, reply_slice) = get_buffer_mut().split_at_mut(request_size);
        let request_slice = &*request_slice; // NB: immutable alias

        let response = dispatch(client_badge, request_slice, reply_slice);

        Camkes::delete_path(recv_path).expect("delete");

        reply_with_cap::<E>(
            response,
            &mut client_badge as _,
            endpoint,
            reply,
            request_size,
            success_word,
        );
    }
}

/// Server-side implementation for basic RPC with optional capability
/// passing in both directions. This macro is normally invoked in a
/// CamkesInterfaceThread's run method to implement the server side of
/// a cantripRPCCall or cantripRPCCallSignal connection. The thread
/// must implement the CapDispatch callback to handle deserialization of
/// marshalled parameters and processing of the request(s). Received
/// capabilities are retrieved directly from the "receive slot", typically
/// using Camkes::get_owned_current_recv_path(). Any reply capability is
/// included in the return value of |CapDispatch|
/// TODO: send vs dup of reply cap
#[macro_export]
macro_rules! rpc_basic_recv_with_caps {
    ($inf_tag:ident, $inf_recv_slot:expr, $inf_request_size:expr, $inf_success:expr) => {
        $crate::paste! {
            rpc_basic_recv_with_caps!(@end
                Self::dispatch,
                [<$inf_tag:upper _INTERFACE_ENDPOINT>],
                [<$inf_tag:upper _INTERFACE_REPLY>],
                $inf_recv_slot,
                $inf_request_size,
                $inf_success,
            );
        }
    };
    (@end $inf_dispatch:expr, $inf_endpoint:ident, $inf_reply:ident, $inf_recv_slot:expr, $inf_request_size:expr, $inf_success:expr) => {
        unsafe {
            crate::camkes::rpc_basic::recv_with_caps_loop(
                $inf_dispatch,
                $inf_endpoint,
                $inf_reply,
                $inf_recv_slot,
                $inf_request_size,
                $inf_success,
            )
        }
    };
}

// Shared handling of RPC reply messages that optionally have a capability.
unsafe fn reply_with_cap<E>(
    response: Result<(usize, Option<seL4_CPtr>), E>,
    client_badge: &mut seL4_Word,
    endpoint: seL4_CPtr,
    reply: seL4_CPtr,
    request_size: usize,
    success_word: seL4_Word,
) where
    usize: From<E>,
{
    let (label, opt_cap, length) = match response {
        Ok((len, opt_cap)) => {
            const WORD_SIZE: usize = size_of::<seL4_Word>();
            // XXX sender expects reply at offset request_size
            (
                success_word,
                opt_cap,
                (request_size + len + WORD_SIZE - 1) / WORD_SIZE,
            )
        }
        Err(e) => (e.into(), None, 0),
    };
    if let Some(cap) = opt_cap {
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
                    /*label=*/ label, /*capsUnwrapped=*/ 0,
                    /*extraCaps=*/ 1, // NB: attached capability
                    /*length=*/ length,
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
                /*label=*/ label, /*capsUnwrapped=*/ 0, /*extraCaps=*/ 0,
                /*length=*/ length,
            ),
            /*sender=*/ client_badge,
            /*reply=*/ reply,
        );
    }
}

/// Server-side implementation for basic RPC with an optional capability
/// attached to a reply. This is normally invoked by the
/// |rpc_basic_recv_with_reply| macro.
///
/// This is a version of |recv_with_caps_loop| optimmized to ignore/drop
/// received capabilities (by way of not setting up a receive slot).
///
/// Any capability to be attached to a reply must be present in the return
/// from |CapDispatch|; |recv_with_caps| will automatically attach
/// the capability to the IPC message and cleanup local state after the reply
/// completes.
pub unsafe fn recv_with_reply_cap_loop<E>(
    dispatch: CapDispatch<E>,
    endpoint: seL4_CPtr,
    reply: seL4_CPtr,
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
        let (request_slice, reply_slice) = get_buffer_mut().split_at_mut(request_size);
        let request_slice = &*request_slice; // NB: immutable alias

        let response = dispatch(client_badge, request_slice, reply_slice);

        reply_with_cap::<E>(
            response,
            &mut client_badge as _,
            endpoint,
            reply,
            request_size,
            success_word,
        );
    }
}

/// Server-side implementation for basic RPC with optional capability
/// passing in reply messages. This macro is normally invoked in a
/// CamkesInterfaceThread's run method to implement the server side of
/// a cantripRPCCall or cantripRPCCallSignal connection. The thread
/// must implement the CapDispatch callback to handle deserialization of
/// marshalled parameters and processing of the request(s). Received
/// capabilities are discarded. Any reply capability is/ included in
/// the return value of |CapDispatch|
/// TODO: send vs dup of reply cap
#[macro_export]
macro_rules! rpc_basic_recv_with_reply_cap {
    ($inf_tag:ident, $inf_request_size:expr, $inf_success:expr) => {
        $crate::paste! {
            rpc_basic_recv_with_reply_cap!(@end
                Self::dispatch,
                [<$inf_tag:upper _INTERFACE_ENDPOINT>],
                [<$inf_tag:upper _INTERFACE_REPLY>],
                $inf_request_size,
                $inf_success
            );
        }
    };
    (@end $inf_dispatch:expr, $inf_endpoint:ident, $inf_reply:ident, $inf_request_size:expr, $inf_success:expr) => {
        unsafe {
            crate::camkes::rpc_basic::recv_with_reply_cap_loop(
                $inf_dispatch,
                $inf_endpoint,
                $inf_reply,
                $inf_request_size,
                $inf_success,
            )
        }
    };
}
