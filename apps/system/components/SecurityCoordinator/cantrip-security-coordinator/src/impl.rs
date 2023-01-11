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

#![allow(dead_code)]
#![allow(unused_variables)]

use alloc::string::String;
use cantrip_memory_interface::cantrip_frame_alloc;
use cantrip_memory_interface::cantrip_object_free_toplevel;
use cantrip_memory_interface::ObjDescBundle;
use cantrip_os_common::copyregion::CopyRegion;
use cantrip_os_common::sel4_sys;
use cantrip_security_interface::*;
use core::mem::size_of;
use log::trace;
use mailbox_interface::*;

use sel4_sys::seL4_PageBits;
use sel4_sys::seL4_Page_GetAddress;
use sel4_sys::seL4_Word;

const PAGE_SIZE: usize = 1 << seL4_PageBits;

const PAGE_SIZE: usize = 1 << seL4_PageBits;

extern "Rust" {
    fn get_deep_copy_src_mut() -> &'static mut [u8];
}

pub struct SeL4SecurityCoordinator {
    // TODO(sleffler): mailbox api state
}
impl Default for SeL4SecurityCoordinator {
    fn default() -> Self { Self::new() }
}
impl SeL4SecurityCoordinator {
    pub fn new() -> Self { SeL4SecurityCoordinator {} }
}
pub type CantripSecurityCoordinatorInterface = SeL4SecurityCoordinator;

impl SecurityCoordinatorInterface for CantripSecurityCoordinatorInterface {
    fn install(&mut self, _pkg_contents: &ObjDescBundle) -> Result<String, SecurityRequestError> {
        Err(SecurityRequestError::InstallFailed)
    }

    fn install_app(
        &mut self,
        app_id: &str,
        pkg_contents: &ObjDescBundle,
    ) -> Result<(), SecurityRequestError> {
        Err(SecurityRequestError::InstallFailed)
    }

    fn install_model(
        &mut self,
        _app_id: &str,
        model_id: &str,
        pkg_contents: &ObjDescBundle,
    ) -> Result<(), SecurityRequestError> {
        Err(SecurityRequestError::InstallModelFailed)
    }

    fn uninstall(&mut self, _bundle_id: &str) -> Result<(), SecurityRequestError> {
        Err(SecurityRequestError::UninstallFailed)
    }

    fn get_packages(&self) -> Result<BundleIdArray, SecurityRequestError> {
        Err(SecurityRequestError::GetPackagesFailed)
    }

    fn size_buffer(&self, _bundle_id: &str) -> Result<usize, SecurityRequestError> {
        Err(SecurityRequestError::SizeBufferFailed)
    }

    fn get_manifest(&self, _bundle_id: &str) -> Result<String, SecurityRequestError> {
        Err(SecurityRequestError::GetManifestFailed)
    }

    fn load_application(
        &mut self,
        _bundle_id: &str,
    ) -> Result<ObjDescBundle, SecurityRequestError> {
        Err(SecurityRequestError::LoadApplicationFailed)
    }

    fn load_model(
        &mut self,
        _bundle_id: &str,
        _model_id: &str,
    ) -> Result<ObjDescBundle, SecurityRequestError> {
        Err(SecurityRequestError::LoadModelFailed)
    }

    fn read_key(
        &self,
        _bundle_id: &str,
        _key: &str,
    ) -> Result<&KeyValueData, SecurityRequestError> {
        Err(SecurityRequestError::ReadFailed)
    }

    fn write_key(
        &mut self,
        _bundle_id: &str,
        _key: &str,
        value: &[u8],
    ) -> Result<(), SecurityRequestError> {
        Err(SecurityRequestError::WriteFailed)
    }

    fn delete_key(&mut self, _bundle_id: &str, _key: &str) -> Result<(), SecurityRequestError> {
        Err(SecurityRequestError::DeleteFailed)
    }

    fn test_mailbox(&mut self) -> Result<(), SecurityRequestError> {
        trace!("test_mailbox_command()");

        const MESSAGE_SIZE_DWORDS: usize = 17; // Just a random message size for testing.

        // Allocate a 4k page to serve as our message buffer.
        let frame_bundle =
            cantrip_frame_alloc(PAGE_SIZE).or(Err(SecurityRequestError::TestFailed))?;
        trace!("test_mailbox: Frame {:?}", frame_bundle);

        unsafe {
            // Map the message buffer into our copyregion so we can access it.
            // NB: re-use one of the deep_copy copyregions.
            let mut msg_region = CopyRegion::new(get_deep_copy_src_mut());
            msg_region
                .map(frame_bundle.objs[0].cptr)
                .or(Err(SecurityRequestError::TestFailed))?;

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
            mailbox_send(paddr.paddr as u32, (MESSAGE_SIZE_DWORDS * size_of::<u32>()) as u32)
                .or(Err(SecurityRequestError::TestFailed))?;

            // Wait for the response to arrive.
            let (response_paddr, response_size) =
                mailbox_recv().or(Err(SecurityRequestError::TestFailed))?;

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
                .or(Err(SecurityRequestError::TestFailed))?;

            // Done, free the message buffer.
            cantrip_object_free_toplevel(&frame_bundle)
                .or(Err(SecurityRequestError::TestFailed))?;

            if dword_a != 0x12345678 || dword_b != 0x87654321 {
                return Err(SecurityRequestError::TestFailed);
            }
        }

        trace!("test_mailbox_command() done");
        Ok(())
    }
}
