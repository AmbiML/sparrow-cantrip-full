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

//! Shell commands to dynamically load packages.

extern crate alloc;
use crate::CmdFn;
use crate::CommandError;
use crate::HashMap;
use crate::SELF_CNODE;
use alloc::string::String;
use cantrip_memory_interface::*;
use cantrip_os_common::sel4_sys;
use cantrip_os_common::slot_allocator::CANTRIP_CSPACE_SLOTS;
use cantrip_proc_interface::cantrip_pkg_mgmt_install;
use cantrip_proc_interface::cantrip_pkg_mgmt_uninstall;
use cantrip_proc_interface::ProcessManagerError;
use core::fmt;
use core::fmt::Write;

use sel4_sys::seL4_CNode_Delete;
use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_WordBits;

use cantrip_io as io;

pub fn add_cmds(cmds: &mut HashMap<&str, CmdFn>) {
    cmds.extend([
        ("install", install_command as CmdFn),
        ("uninstall", uninstall_command as CmdFn),
    ]);
}

fn collect_from_zmodem(
    input: &mut dyn io::BufRead,
    mut output: &mut dyn io::Write,
) -> Option<ObjDescBundle> {
    writeln!(output, "Starting zmodem upload...").ok()?;
    let mut upload = crate::rz::rz(input, &mut output).ok()?;
    upload.finish();
    writeln!(
        output,
        "Received {} bytes of data, crc32 {}",
        upload.len(),
        hex::encode(upload.crc32().to_be_bytes())
    )
    .ok()?;
    Some(upload.frames().clone())
}

fn install_command(
    args: &mut dyn Iterator<Item = &str>,
    input: &mut dyn io::BufRead,
    mut output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    fn clear_slot(slot: seL4_CPtr) {
        unsafe {
            CANTRIP_CSPACE_SLOTS.free(slot, 1);
            seL4_CNode_Delete(SELF_CNODE, slot, seL4_WordBits as u8).expect("install");
        }
    }

    enum PkgType {
        Bundle(ObjDescBundle),
    }
    impl PkgType {
        // Returns an immutable ref to |contents|.
        pub fn get(&self) -> &ObjDescBundle {
            match self {
                PkgType::Bundle(contents) => contents,
            }
        }
        // Returns a mutable ref to |contents|.
        pub fn get_mut(&mut self) -> &mut ObjDescBundle {
            match self {
                PkgType::Bundle(contents) => contents,
            }
        }
        pub fn install(&self) -> Result<String, ProcessManagerError> {
            match self {
                PkgType::Bundle(contents) => cantrip_pkg_mgmt_install(contents),
            }
        }
    }
    impl fmt::Display for PkgType {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                PkgType::Bundle(_) => write!(f, "Bundle"),
            }
        }
    }
    // XXX add drop that reclaims contents

    // Collect/setup the package frames. If a -z arg is supplied a zmodem
    // upload is used.
    let mut pkg_contents: PkgType = match args.next().ok_or(CommandError::BadArgs)? {
        "-z" => PkgType::Bundle(collect_from_zmodem(input, &mut output).ok_or(CommandError::IO)?),
        _ => {
            writeln!(output, "install only supports zmodem uploads")?;
            return Ok(());
        }
    };

    // The frames are in SELF_CNODE; wrap them in a dynamically allocated
    // CNode (as expected by cantrip_pgk_mgmt_install).
    // TODO(sleffler): useful idiom, add to MemoryManager
    let cnode_depth = pkg_contents.get().count_log2();
    let cnode = cantrip_cnode_alloc(cnode_depth).map_err(|_| CommandError::Memory)?;
    pkg_contents
        .get_mut()
        .move_objects_from_toplevel(cnode.objs[0].cptr, cnode_depth as u8)
        .map_err(|_| CommandError::Memory)?;
    match pkg_contents.install() {
        Ok(id) => {
            writeln!(output, "{} \"{}\" installed", pkg_contents, id)?;
        }
        Err(status) => {
            writeln!(output, "{} install failed: {:?}", pkg_contents, status)?;
        }
    }

    // SecurityCoordinator owns the cnode & frames contained within but we
    // still have a cap for the cnode in our top-level CNode; clean it up.
    debug_assert!(cnode.cnode == unsafe { SELF_CNODE });
    sel4_sys::debug_assert_slot_cnode!(cnode.objs[0].cptr);
    clear_slot(cnode.objs[0].cptr);

    Ok(())
}

fn uninstall_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    let bundle_id = args.next().ok_or(CommandError::BadArgs)?;
    match cantrip_pkg_mgmt_uninstall(bundle_id) {
        Ok(_) => {
            writeln!(output, "Bundle \"{}\" uninstalled.", bundle_id)?;
        }
        Err(status) => {
            writeln!(output, "uninstall failed: {:?}", status)?;
        }
    }
    Ok(())
}
