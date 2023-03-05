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

//! SecurityCoordinator shell test commands

extern crate alloc;
use crate::CmdFn;
use crate::CommandError;
use crate::HashMap;
use alloc::vec::Vec;
use core::fmt::Write;

use cantrip_io as io;
use cantrip_memory_interface::cantrip_object_free_in_cnode;
use cantrip_os_common::cspace_slot::CSpaceSlot;
use cantrip_security_interface::*;

pub fn add_cmds(cmds: &mut HashMap<&str, CmdFn>) {
    cmds.extend([
        ("scecho", scecho_command as CmdFn),
        ("size_buffer", size_buffer_command as CmdFn),
        ("get_manifest", get_manifest_command as CmdFn),
        ("load_application", load_application_command as CmdFn),
        ("load_model", load_model_command as CmdFn),
        ("test_mailbox", test_mailbox_command as CmdFn),
    ]);
}

/// Implements an "scecho" command that sends arguments to the Security Core's echo service.
fn scecho_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    let request = args.collect::<Vec<&str>>().join(" ");
    match cantrip_security_echo(&request) {
        Ok(result) => writeln!(output, "{}", result)?,
        Err(status) => writeln!(output, "ECHO replied {:?}", status)?,
    }
    Ok(())
}

fn size_buffer_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    let bundle_id = args.next().ok_or(CommandError::BadArgs)?;
    match cantrip_security_size_buffer(bundle_id) {
        Ok(size) => writeln!(output, "{}", size)?,
        Err(status) => writeln!(output, "SizeBuffer failed: {:?}", status)?,
    }
    Ok(())
}

fn get_manifest_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    let bundle_id = args.next().ok_or(CommandError::BadArgs)?;
    match cantrip_security_get_manifest(bundle_id) {
        Ok(manifest) => writeln!(output, "{}", manifest)?,
        Err(status) => writeln!(output, "GetManifest failed: {:?}", status)?,
    }
    Ok(())
}

fn load_application_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    let bundle_id = args.next().ok_or(CommandError::BadArgs)?;
    let mut container_slot = CSpaceSlot::new();
    match cantrip_security_load_application(bundle_id, &container_slot) {
        Ok(frames) => {
            container_slot.release(); // NB: take ownership
            writeln!(output, "{:?}", frames)?;
            let _ = cantrip_object_free_in_cnode(&frames);
        }
        Err(status) => writeln!(output, "LoadApplication failed: {:?}", status)?,
    }
    Ok(())
}

fn load_model_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    let bundle_id = args.next().ok_or(CommandError::BadArgs)?;
    let model_id = args.next().ok_or(CommandError::BadArgs)?;
    let mut container_slot = CSpaceSlot::new();
    match cantrip_security_load_model(bundle_id, model_id, &container_slot) {
        Ok(frames) => {
            container_slot.release(); // NB: take ownership
            writeln!(output, "{:?}", frames)?;
            let _ = cantrip_object_free_in_cnode(&frames);
        }
        Err(status) => writeln!(output, "LoadModel failed: {:?}", status)?,
    }
    Ok(())
}

fn test_mailbox_command(
    _args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    match cantrip_security_test_mailbox() {
        Ok(_) => {
            writeln!(output, "Test mailbox OK.")?;
        }
        Err(_status) => {
            writeln!(output, "Test mailbox failed.")?;
        }
    }
    Ok(())
}
