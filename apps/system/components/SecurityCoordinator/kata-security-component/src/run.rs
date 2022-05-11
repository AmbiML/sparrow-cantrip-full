//! Cantrip OS Security Coordinator component support.

// Code here binds the camkes component to the rust code.
#![no_std]

use core::slice;
use cantrip_os_common::allocator;
use cantrip_os_common::logger::CantripLogger;
use cantrip_os_common::sel4_sys;
use cantrip_os_common::slot_allocator;
use cantrip_security_coordinator::CANTRIP_SECURITY;
use cantrip_security_interface::*;
use cantrip_storage_interface::KEY_VALUE_DATA_SIZE;
use log::{info, trace};
use postcard;

use SecurityRequestError::*;

use sel4_sys::seL4_CNode_Delete;
use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_GetCapReceivePath;
use sel4_sys::seL4_SetCap;
use sel4_sys::seL4_SetCapReceivePath;
use sel4_sys::seL4_Word;
use sel4_sys::seL4_WordBits;

use slot_allocator::CSpaceSlot;
use slot_allocator::CANTRIP_CSPACE_SLOTS;

extern "C" {
    // Each CAmkES-generated CNode has a writable self-reference to itself in
    // the slot SELF_CNODE.
    static SELF_CNODE: seL4_CPtr;

    static SELF_CNODE_FIRST_SLOT: seL4_CPtr;
    static SELF_CNODE_LAST_SLOT: seL4_CPtr;
}

static mut SECURITY_RECV_SLOT: seL4_CPtr = 0;

#[no_mangle]
pub extern "C" fn pre_init() {
    static CANTRIP_LOGGER: CantripLogger = CantripLogger;
    log::set_logger(&CANTRIP_LOGGER).unwrap();
    // NB: set to max; the LoggerInterface will filter
    log::set_max_level(log::LevelFilter::Trace);

    static mut HEAP_MEMORY: [u8; 8 * 1024] = [0; 8 * 1024];
    unsafe {
        allocator::ALLOCATOR.init(HEAP_MEMORY.as_mut_ptr() as usize, HEAP_MEMORY.len());
        trace!(
            "setup heap: start_addr {:p} size {}",
            HEAP_MEMORY.as_ptr(),
            HEAP_MEMORY.len()
        );
    }

    // Complete CANTRIP_SECURITY setup. This is as early as we can do it given that
    // it needs the GlobalAllocator.
    unsafe {
        CANTRIP_SECURITY.init();
    }

    unsafe {
        CANTRIP_CSPACE_SLOTS.init(
            /*first_slot=*/ SELF_CNODE_FIRST_SLOT,
            /*size=*/ SELF_CNODE_LAST_SLOT - SELF_CNODE_FIRST_SLOT
        );
        trace!("setup cspace slots: first slot {} free {}",
               CANTRIP_CSPACE_SLOTS.base_slot(),
               CANTRIP_CSPACE_SLOTS.free_slots());

        SECURITY_RECV_SLOT = CANTRIP_CSPACE_SLOTS.alloc(1).unwrap();
    }
}


#[no_mangle]
pub extern "C" fn security__init() {
    unsafe {
        // Point the receive path to the well-known empty slot. This will be
        // used to receive CNode's from clients for install requests.
        //
        // NB: this must be done here (rather than someplace like pre_init)
        // so it's in the context of the SecurityCoordinatorInterface thread
        // (so we write the correct ipc buffer).
        let path = (SELF_CNODE, SECURITY_RECV_SLOT, seL4_WordBits);
        seL4_SetCapReceivePath(path.0, path.1, path.2);
        info!("security cap receive path {:?}", path);
        debug_check_empty("security__init", &path);
    }
}

fn debug_check_empty(tag: &str, path: &(seL4_CPtr, seL4_CPtr, seL4_Word)) {
    sel4_sys::debug_assert_slot_empty!(path.1,
        "{}: expected slot {:?} empty but has cap type {:?}",
        tag, path, sel4_sys::cap_identify(path.1));
}

// Clears any capability the specified path points to.
fn _clear_path(path: &(seL4_CPtr, seL4_CPtr, seL4_Word)) {
    // TODO(sleffler): assert since future receives are likely to fail?
    if let Err(e) = unsafe { seL4_CNode_Delete(path.0, path.1, path.2 as u8) } {
        // NB: no error is returned if the slot is empty.
        info!("Failed to clear receive path {:?}: {:?}", path, e);
    }
    debug_check_empty("clear_path", path);
}

fn serialize_failure(e: postcard::Error) -> SecurityRequestError {
    trace!("serialize failed: {:?}", e);
    SreBundleDataInvalid
}
fn deserialize_failure(e: postcard::Error) -> SecurityRequestError {
    trace!("deserialize failed: {:?}", e);
    SreDeserializeFailed
}

fn echo_request(
    request_buffer: &[u8],
    reply_buffer: &mut [u8]
) -> Result<(), SecurityRequestError> {
    let request = postcard::from_bytes::<EchoRequest>(&request_buffer[..])
        .map_err(deserialize_failure)?;

    trace!("ECHO {:?}", request.value);
    // NB: cheat, bypass serde
    reply_buffer[0..request.value.len()].copy_from_slice(request.value);
    Ok(())
}

fn install_request(
    request_buffer: &[u8],
    reply_buffer: &mut [u8]
) -> Result<(), SecurityRequestError> {
    let recv_path = unsafe { seL4_GetCapReceivePath() };
    sel4_sys::debug_assert_slot_cnode!(recv_path.1,
        "install_request: expected cnode in slot {:?} but found cap type {:?}",
        recv_path, sel4_sys::cap_identify(recv_path.1));

    let mut request = postcard::from_bytes::<InstallRequest>(&request_buffer[..])
        .map_err(deserialize_failure)?;  // XXX clear_path

    // Move the container CNode so it's not clobbered.
    // XXX who should be responsible for this
    let mut container_slot = CSpaceSlot::new();
    container_slot.move_to(recv_path.0, recv_path.1, recv_path.2 as u8)
        .map_err(|_| SecurityRequestError::SreCapMoveFailed)?; // XXX expect?
    request.set_container_cap(container_slot.slot);
    container_slot.release();

    let bundle_id = unsafe { CANTRIP_SECURITY.install(&request.pkg_contents) }?;
    let _ = postcard::to_slice(
        &InstallResponse {
            bundle_id: &bundle_id,
        },
        reply_buffer,
    ).map_err(serialize_failure)?;
    Ok(())
}

fn uninstall_request(
    request_buffer: &[u8],
    _reply_buffer: &mut [u8]
) -> Result<(), SecurityRequestError> {
    let request = postcard::from_bytes::<UninstallRequest>(&request_buffer[..])
        .map_err(deserialize_failure)?;

    trace!("UNINSTALL {}", request.bundle_id);
    unsafe { CANTRIP_SECURITY.uninstall(request.bundle_id) }
}

fn size_buffer_request(
    request_buffer: &[u8],
    reply_buffer: &mut [u8]
) -> Result<(), SecurityRequestError> {
    let request = postcard::from_bytes::<SizeBufferRequest>(&request_buffer[..])
        .map_err(deserialize_failure)?;

    trace!("SIZE BUFFER bundle_id {}", request.bundle_id);
    let buffer_size = unsafe { CANTRIP_SECURITY.size_buffer(request.bundle_id) }?;
    let _ = postcard::to_slice(
        &SizeBufferResponse {
            buffer_size: buffer_size,
        },
        reply_buffer,
    ).map_err(serialize_failure)?;
    Ok(())
}

fn get_manifest_request(
    request_buffer: &[u8],
    reply_buffer: &mut [u8]
) -> Result<(), SecurityRequestError> {
    let request = postcard::from_bytes::<GetManifestRequest>(&request_buffer[..])
        .map_err(deserialize_failure)?;

    trace!("GET MANIFEST bundle_id {}", request.bundle_id);
    let manifest = unsafe { CANTRIP_SECURITY.get_manifest(request.bundle_id) }?;
    let _ = postcard::to_slice(
        &GetManifestResponse {
            manifest: &manifest,
        },
        reply_buffer
    ).map_err(serialize_failure)?;
    Ok(())
}

fn load_application_request(
    request_buffer: &[u8],
    reply_buffer: &mut [u8]
) -> Result<(), SecurityRequestError> {
    let request = postcard::from_bytes::<LoadApplicationRequest>(&request_buffer[..])
        .map_err(deserialize_failure)?;

    trace!("LOAD APPLICATION bundle_id {}", request.bundle_id);
    let bundle_frames = unsafe {
        CANTRIP_SECURITY.load_application(request.bundle_id)
    }?;
    postcard::to_slice(
        &LoadApplicationResponse {
            bundle_frames: bundle_frames.clone(),
        },
        reply_buffer
    ).map_err(serialize_failure)?;
    trace!("LOAD APPLICATION -> {}", bundle_frames);
    unsafe { seL4_SetCap(0, bundle_frames.cnode) };
    Ok(())
}

fn load_model_request(
    request_buffer: &[u8],
    reply_buffer: &mut [u8]
) -> Result<(), SecurityRequestError> {
    let request = postcard::from_bytes::<LoadModelRequest>(&request_buffer[..])
        .map_err(deserialize_failure)?;

    let model_frames = unsafe {
        CANTRIP_SECURITY.load_model(request.bundle_id, request.model_id)
    }?;
    let _ = postcard::to_slice(
        &LoadApplicationResponse {
            bundle_frames: model_frames.clone(),
        },
        reply_buffer
    ).map_err(serialize_failure)?;
    trace!("LOAD MODEL -> {}", model_frames);
    unsafe { seL4_SetCap(0, model_frames.cnode) };
    Ok(())
}

fn read_key_request(
    request_buffer: &[u8],
    reply_buffer: &mut [u8]
) -> Result<(), SecurityRequestError> {
    let request = postcard::from_bytes::<ReadKeyRequest>(&request_buffer[..])
        .map_err(deserialize_failure)?;

    trace!("READ KEY bundle_id {} key {}", request.bundle_id, request.key);
    let value = unsafe {
        CANTRIP_SECURITY.read_key(request.bundle_id, request.key)
    }?;
    let _ = postcard::to_slice(
        &ReadKeyResponse {
            value: value,
        },
        reply_buffer
    ).map_err(serialize_failure);
    Ok(())
}

fn write_key_request(
    request_buffer: &[u8],
    _reply_buffer: &mut [u8]
) -> Result<(), SecurityRequestError> {
    let request = postcard::from_bytes::<WriteKeyRequest>(&request_buffer[..])
        .map_err(deserialize_failure)?;

    trace!("WRITE KEY bundle_id {} key {} value {:?}",
        request.bundle_id, request.key, request.value);
    // NB: the serialized data are variable length so copy to convert
    let mut keyval = [0u8; KEY_VALUE_DATA_SIZE];
    keyval[..request.value.len()].copy_from_slice(request.value);
    unsafe { CANTRIP_SECURITY.write_key( request.bundle_id, request.key, &keyval) }
}

fn delete_key_request(
    request_buffer: &[u8],
    _reply_buffer: &mut [u8]
) -> Result<(), SecurityRequestError> {
    let request = postcard::from_bytes::<DeleteKeyRequest>(&request_buffer[..])
        .map_err(deserialize_failure)?;

    trace!("DELETE KEY bundle_id {} key {}", request.bundle_id, request.key);
    unsafe { CANTRIP_SECURITY.delete_key(request.bundle_id, request.key) }
}

#[no_mangle]
pub extern "C" fn security_request(
    c_request: SecurityRequest,
    c_request_buffer_len: u32,
    c_request_buffer: *const u8,
    c_reply_buffer: *mut SecurityReplyData,
) -> SecurityRequestError {
    let request_buffer = unsafe {
        slice::from_raw_parts(c_request_buffer, c_request_buffer_len as usize)
    };
    let reply_buffer = unsafe { &mut (*c_reply_buffer)[..] };
    match c_request {
        SecurityRequest::SrEcho =>
            echo_request(request_buffer, reply_buffer),
        SecurityRequest::SrInstall =>
            install_request(request_buffer, reply_buffer),
        SecurityRequest::SrUninstall =>
            uninstall_request(request_buffer, reply_buffer),
        SecurityRequest::SrSizeBuffer =>
            size_buffer_request(request_buffer, reply_buffer),
        SecurityRequest::SrGetManifest =>
            get_manifest_request(request_buffer, reply_buffer),
        SecurityRequest::SrLoadApplication =>
            load_application_request(request_buffer, reply_buffer),
        SecurityRequest::SrLoadModel =>
            load_model_request(request_buffer, reply_buffer),
        SecurityRequest::SrReadKey =>
            read_key_request(request_buffer, reply_buffer),
        SecurityRequest::SrWriteKey =>
            write_key_request(request_buffer, reply_buffer),
        SecurityRequest::SrDeleteKey =>
            delete_key_request(request_buffer, reply_buffer),
    }.map_or_else(|e| e, |_v| SecurityRequestError::SreSuccess)
}
