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

//! Cantrip OS security coordinator seL4 support

use cantrip_memory_interface::cantrip_frame_alloc;
use cantrip_memory_interface::cantrip_object_free_toplevel;
use cantrip_os_common::sel4_sys;
use cantrip_security_interface::*;
use log::trace;

use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_Page_GetAddress;

extern "C" {
    static SECURITY_RECV_SLOT: seL4_CPtr;
}

pub struct SeL4SecurityCoordinator {
    // TODO(sleffler): mailbox api state
}
impl SeL4SecurityCoordinator {
    pub fn new() -> Self { SeL4SecurityCoordinator {} }
}
pub type CantripSecurityCoordinatorInterface = SeL4SecurityCoordinator;

impl SecurityCoordinatorInterface for SeL4SecurityCoordinator {
    fn install(&mut self, _pkg_contents: &ObjDescBundle) -> Result<String, SecurityRequestError> {
        Err(SreInstallFailed)
    }
    fn uninstall(&mut self, _bundle_id: &str) -> Result<(), SecurityRequestError> {
        Err(SreUninstallFailed)
    }
    fn size_buffer(&self, _bundle_id: &str) -> Result<usize, SecurityRequestError> {
        Err(SreSizeBufferFailed)
    }
    fn get_manifest(&self, _bundle_id: &str) -> Result<String, SecurityRequestError> {
        Err(SreGetManifestFailed)
    }
    fn load_application(
        &mut self,
        _bundle_id: &str,
    ) -> Result<ObjDescBundle, SecurityRequestError> {
        Err(SreLoadApplicationFailed)
    }
    fn load_model(
        &mut self,
        _bundle_id: &str,
        _model_id: &str,
    ) -> Result<ObjDescBundle, SecurityRequestError> {
        Err(SreLoadModelFailed)
    }
    fn read_key(
        &self,
        _bundle_id: &str,
        _key: &str,
    ) -> Result<&KeyValueData, SecurityRequestError> {
        Err(SreReadFailed)
    }
    fn write_key(
        &mut self,
        _bundle_id: &str,
        _key: &str,
        _value: &KeyValueData,
    ) -> Result<(), SecurityRequestError> {
        Err(SreWriteFailed)
    }
    fn delete_key(&mut self, _bundle_id: &str, _key: &str) -> Result<(), SecurityRequestError> {
        Err(SreDeleteFailed)
    }

    fn test_mailbox(&mut self) -> Result<(), SecurityRequestError> {
        trace!("test_mailbox_command()");

        const MESSAGE_SIZE_DWORDS: usize = 17; // Just a random message size for testing.

        extern "C" {
            fn mailbox_api_send(paddr: u32, size: u32);
            fn mailbox_api_receive(paddr: *mut u32, size: *mut u32);
        }

        // Allocate a 4k page to serve as our message buffer.
        let frame_bundle =
            cantrip_frame_alloc(PAGE_SIZE).or(Err(SecurityRequestError::SreTestFailed))?;
        trace!("test_mailbox: Frame {:?}", frame_bundle);

        unsafe {
            // Map the message buffer into our copyregion so we can access it.
            // NB: re-use one of the deep_copy copyregions.
            let mut msg_region = CopyRegion::new(ptr::addr_of_mut!(DEEP_COPY_SRC[0]), PAGE_SIZE);
            msg_region
                .map(frame_bundle.objs[0].cptr)
                .or(Err(SecurityRequestError::SreTestFailed))?;

            let message_ptr = msg_region.as_word_mut();

            // Write to the message buffer through the copyregion.
            let offset_a = 0;
            let offset_b = MESSAGE_SIZE_DWORDS - 1;
            message_ptr[offset_a] = 0xDEADBEEF;
            message_ptr[offset_b] = 0xF00DCAFE;
            trace!(
                "test_mailbox: old buf contents  0x{:X} 0x{:X}",
                message_ptr[offset_a],
                message_ptr[offset_b]
            );

            // Send the _physical_ address of the message buffer to the security
            // core.
            let paddr = seL4_Page_GetAddress(frame_bundle.objs[0].cptr);
            mailbox_api_send(paddr.paddr as u32, (MESSAGE_SIZE_DWORDS * size_of::<u32>()) as u32);

            // Wait for the response to arrive.
            let mut response_paddr: u32 = 0;
            let mut response_size: u32 = 0;
            mailbox_api_receive(&mut response_paddr as *mut u32, &mut response_size as *mut u32);

            // The security core should have replaced the first and last dwords
            // with 0x12345678 and 0x87654321.
            trace!("test_mailbox: expected contents 0x12345678 0x87654321");
            trace!(
                "test_mailbox: new buf contents  0x{:X} 0x{:X}",
                message_ptr[offset_a],
                message_ptr[offset_b]
            );

            let dword_a = message_ptr[offset_a];
            let dword_b = message_ptr[offset_b];

            msg_region
                .unmap()
                .or(Err(SecurityRequestError::SreTestFailed))?;

            // Done, free the message buffer.
            cantrip_object_free_toplevel(&frame_bundle)
                .or(Err(SecurityRequestError::SreTestFailed))?;

            if dword_a != 0x12345678 || dword_b != 0x87654321 {
                return Err(SecurityRequestError::SreTestFailed);
            }
        }

        trace!("test_mailbox_command() done");
        Ok(())
    }
}
