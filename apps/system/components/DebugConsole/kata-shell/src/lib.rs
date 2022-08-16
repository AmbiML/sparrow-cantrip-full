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

extern crate alloc;
use alloc::vec::Vec;
use core::fmt;
use core::fmt::Write;
use cpio::CpioNewcReader;
use hashbrown::HashMap;

use cantrip_io as io;
use cantrip_line_reader::LineReader;
use cantrip_memory_interface::*;
use cantrip_ml_interface::*;
use cantrip_os_common::sel4_sys;
use cantrip_os_common::slot_allocator;
use cantrip_proc_interface::cantrip_pkg_mgmt_install;
use cantrip_proc_interface::cantrip_pkg_mgmt_uninstall;
use cantrip_proc_interface::cantrip_proc_ctrl_get_running_bundles;
use cantrip_proc_interface::cantrip_proc_ctrl_start;
use cantrip_proc_interface::cantrip_proc_ctrl_stop;
use cantrip_storage_interface::cantrip_storage_delete;
use cantrip_storage_interface::cantrip_storage_read;
use cantrip_storage_interface::cantrip_storage_write;

use sel4_sys::seL4_CNode_Delete;
use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_WordBits;

use slot_allocator::CANTRIP_CSPACE_SLOTS;

mod rz;

#[cfg(feature = "FRINGE_CMDS")]
mod fringe_cmds;
#[cfg(feature = "TEST_GLOBAL_ALLOCATOR")]
mod test_global_allocator;
#[cfg(feature = "TEST_MEMORY_MANAGER")]
mod test_memory_manager;
#[cfg(feature = "TEST_ML_COORDINATOR")]
mod test_ml_coordinator;
#[cfg(feature = "TEST_PANIC")]
mod test_panic;
#[cfg(feature = "TEST_SDK_RUNTIME")]
mod test_sdk_runtime;
#[cfg(feature = "TEST_SECURITY_COORDINATOR")]
mod test_security_coordinator;
#[cfg(feature = "TEST_TIMER_SERVICE")]
mod test_timer_service;
#[cfg(feature = "TEST_UART")]
mod test_uart;

extern "C" {
    static SELF_CNODE: seL4_CPtr;
}

/// Error type indicating why a command line is not runnable.
pub enum CommandError {
    UnknownCommand,
    BadArgs,
    IO,
    Memory,
    Formatter(fmt::Error),
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CommandError::UnknownCommand => write!(f, "unknown command"),
            CommandError::BadArgs => write!(f, "invalid arguments"),
            CommandError::IO => write!(f, "input / output error"),
            CommandError::Memory => write!(f, "memory allocation error"),
            CommandError::Formatter(e) => write!(f, "{}", e),
        }
    }
}

impl From<core::num::ParseIntError> for CommandError {
    fn from(_err: core::num::ParseIntError) -> CommandError { CommandError::BadArgs }
}

impl From<core::num::ParseFloatError> for CommandError {
    fn from(_err: core::num::ParseFloatError) -> CommandError { CommandError::BadArgs }
}

impl From<core::str::ParseBoolError> for CommandError {
    fn from(_err: core::str::ParseBoolError) -> CommandError { CommandError::BadArgs }
}

impl From<fmt::Error> for CommandError {
    fn from(err: fmt::Error) -> CommandError { CommandError::Formatter(err) }
}

impl From<io::Error> for CommandError {
    fn from(_err: io::Error) -> CommandError { CommandError::IO }
}

type CmdFn = fn(
    args: &mut dyn Iterator<Item = &str>,
    input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    builtin_cpio: &[u8],
) -> Result<(), CommandError>;

/// Read-eval-print loop for the DebugConsole command line interface.
pub fn repl<T: io::BufRead>(output: &mut dyn io::Write, input: &mut T, builtin_cpio: &[u8]) -> ! {
    let mut cmds = HashMap::<&str, CmdFn>::new();
    cmds.extend([
        ("builtins", builtins_command as CmdFn),
        ("bundles", bundles_command as CmdFn),
        ("capscan", capscan_command as CmdFn),
        ("kvdelete", kvdelete_command as CmdFn),
        ("kvread", kvread_command as CmdFn),
        ("kvwrite", kvwrite_command as CmdFn),
        ("install", install_command as CmdFn),
        ("loglevel", loglevel_command as CmdFn),
        ("mdebug", mdebug_command as CmdFn),
        ("mstats", mstats_command as CmdFn),
        ("ps", ps_command as CmdFn),
        ("start", start_command as CmdFn),
        ("stop", stop_command as CmdFn),
        ("uninstall", uninstall_command as CmdFn),
        ("state_mlcoord", state_mlcoord_command as CmdFn),
    ]);
    #[cfg(feature = "FRINGE_CMDS")]
    fringe_cmds::add_cmds(&mut cmds);
    #[cfg(feature = "TEST_GLOBAL_ALLOCATOR")]
    test_global_allocator::add_cmds(&mut cmds);
    #[cfg(feature = "TEST_MEMORY_MANAGER")]
    test_memory_manager::add_cmds(&mut cmds);
    #[cfg(feature = "TEST_ML_COORDINATOR")]
    test_ml_coordinator::add_cmds(&mut cmds);
    #[cfg(feature = "TEST_PANIC")]
    test_panic::add_cmds(&mut cmds);
    #[cfg(feature = "TEST_SDK_RUNTIME")]
    test_sdk_runtime::add_cmds(&mut cmds);
    #[cfg(feature = "TEST_SECURITY_COORDINATOR")]
    test_security_coordinator::add_cmds(&mut cmds);
    #[cfg(feature = "TEST_TIMER_SERVICE")]
    test_timer_service::add_cmds(&mut cmds);
    #[cfg(feature = "TEST_UART")]
    test_uart::add_cmds(&mut cmds);

    let mut line_reader = LineReader::new();
    loop {
        const PROMPT: &str = "CANTRIP> ";
        let _ = output.write_str(PROMPT);
        match line_reader.read_line(output, input) {
            Ok(cmdline) => {
                let mut args = cmdline.split_ascii_whitespace();
                match args.next() {
                    Some("?") | Some("help") => {
                        let mut keys: Vec<&str> = cmds.keys().copied().collect();
                        keys.sort();
                        for k in keys {
                            let _ = writeln!(output, "{}", k);
                        }
                    }
                    Some(cmd) => {
                        let result = cmds.get(cmd).map_or_else(
                            || Err(CommandError::UnknownCommand),
                            |func| func(&mut args, input, output, builtin_cpio),
                        );
                        if let Err(e) = result {
                            let _ = writeln!(output, "{}", e);
                        };
                    }
                    None => {
                        let _ = output.write_str("\n");
                    }
                }
            }
            Err(e) => {
                let _ = writeln!(output, "\n{}", e);
            }
        }
    }
}

/// Implements a "builtins" command that lists the contents of the built-in cpio archive.
fn builtins_command(
    _args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    for e in CpioNewcReader::new(builtin_cpio) {
        if e.is_err() {
            writeln!(output, "{:?}", e.unwrap_err())?;
            break; // NB: iterator does not terminate on error
        }
        let entry = e.unwrap();
        writeln!(output, "{} {}", entry.name, entry.data.len())?;
    }
    Ok(())
}

/// Implements a command to configure the max log level for the DebugConsole.
fn loglevel_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    if let Some(level) = args.next() {
        use log::LevelFilter;
        match level {
            "off" => log::set_max_level(LevelFilter::Off),
            "debug" => log::set_max_level(LevelFilter::Debug),
            "info" => log::set_max_level(LevelFilter::Info),
            "error" => log::set_max_level(LevelFilter::Error),
            "trace" => log::set_max_level(LevelFilter::Trace),
            "warn" => log::set_max_level(LevelFilter::Warn),
            _ => writeln!(output, "Unknown log level {}", level)?,
        }
    }
    Ok(writeln!(output, "{}", log::max_level())?)
}

/// Implements a "ps" command that dumps seL4 scheduler state to the console.
#[allow(unused_variables)]
fn ps_command(
    _args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    #[cfg(feature = "CONFIG_DEBUG_BUILD")]
    unsafe {
        sel4_sys::seL4_DebugDumpScheduler();
        Ok(())
    }

    #[cfg(not(feature = "CONFIG_DEBUG_BUILD"))]
    Ok(writeln!(
        output,
        "Kernel support not configured with CONFIG_DEBUG_BUILD!"
    )?)
}

fn bundles_command(
    _args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    match cantrip_proc_ctrl_get_running_bundles() {
        Ok(bundle_ids) => {
            for b in bundle_ids {
                writeln!(output, "{}", b)?;
            }
        }
        Err(status) => {
            writeln!(output, "get_running_bundles failed: {:?}", status)?;
        }
    }
    Ok(())
}

/// Implements a "capscan" command that dumps seL4 capabilities to the console.
#[allow(unused_variables)]
fn capscan_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    #[cfg(feature = "CONFIG_PRINTING")]
    match args.next() {
        Some("console") => unsafe {
            sel4_sys::seL4_DebugDumpCNode(SELF_CNODE);
        },
        Some("memory") => {
            let _ = cantrip_memory_interface::cantrip_memory_capscan();
        }
        Some("process") => {
            let _ = cantrip_proc_interface::cantrip_proc_ctrl_capscan();
        }
        Some("mlcoord") => {
            let _ = cantrip_mlcoord_capscan();
        }
        Some("security") => {
            let _ = cantrip_security_interface::cantrip_security_capscan();
        }
        Some("storage") => {
            let _ = cantrip_storage_interface::cantrip_storage_capscan();
        }
        Some("timer") => {
            let _ = cantrip_timer_interface::timer_service_capscan();
        }
        Some(bundle_id) => {
            if let Err(e) = cantrip_proc_interface::cantrip_proc_ctrl_capscan_bundle(bundle_id) {
                writeln!(output, "{}: {:?}", bundle_id, e)?;
            }
        }
        None => {
            writeln!(output, "capscan <target>, where <target> is one of:")?;
            writeln!(output, "  console (DebugConsole)")?;
            writeln!(output, "  memory (MemoryManager)")?;
            writeln!(output, "  process (ProcessManager)")?;
            writeln!(output, "  mlcoord (MlCoordinator)")?;
            writeln!(output, "  securiy (SecurityCoordinator)")?;
            writeln!(output, "  storage (StorageManager)")?;
            writeln!(output, "  timer (TimerService)")?;
            writeln!(output, "anything else is treated as a bundle_id")?;
        }
    }

    #[cfg(not(feature = "CONFIG_PRINTING"))]
    writeln!(output, "Kernel not configured with CONFIG_PRINTING!")?;

    Ok(())
}

fn collect_from_cpio(
    filename: &str,
    cpio: &[u8],
    output: &mut dyn io::Write,
) -> Option<ObjDescBundle> {
    for e in CpioNewcReader::new(cpio) {
        if e.is_err() {
            writeln!(output, "cpio error {:?}", e.unwrap_err()).ok()?;
            // NB: iterator does not terminate on error but also won't advance
            break;
        }
        let entry = e.unwrap();
        if entry.name == filename {
            // Cheat, re-use zmodem data collector.
            use cantrip_io::Write;
            let mut upload = rz::Upload::new();
            let len = upload.write(entry.data).ok()?;
            upload.finish();
            writeln!(
                output,
                "Collected {} bytes of data, crc32 {}",
                len,
                hex::encode(upload.crc32().to_be_bytes())
            )
            .ok()?;
            return Some(upload.frames().clone());
        }
    }
    writeln!(output, "Built-in file \"{}\" not found", filename).ok()?;
    None
}

fn collect_from_zmodem(
    input: &mut dyn io::BufRead,
    mut output: &mut dyn io::Write,
) -> Option<ObjDescBundle> {
    writeln!(output, "Starting zmodem upload...").ok()?;
    let mut upload = rz::rz(input, &mut output).ok()?;
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
    builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    fn clear_slot(slot: seL4_CPtr) {
        unsafe {
            CANTRIP_CSPACE_SLOTS.free(slot, 1);
            seL4_CNode_Delete(SELF_CNODE, slot, seL4_WordBits as u8).expect("install");
        }
    }

    // Collect/setup the package frames. If a -z arg is present a zmodem
    // upload is used; otherwise we use some raw pages (for testing).
    let mut pkg_contents = match args.next() {
        Some("-z") => collect_from_zmodem(input, &mut output).ok_or(CommandError::IO)?,
        Some(filename) => {
            collect_from_cpio(filename, builtin_cpio, output).ok_or(CommandError::IO)?
        }
        None => {
            // TODO: pattern-fill pages
            cantrip_frame_alloc(8192).map_err(|_| CommandError::IO)?
        }
    };

    // The frames are in SELF_CNODE; wrap them in a dynamically allocated
    // CNode (as expected by cantrip_pgk_mgmt_install).
    // TODO(sleffler): useful idiom, add to MemoryManager
    let cnode_depth = pkg_contents.count_log2();
    let cnode = cantrip_cnode_alloc(cnode_depth).map_err(|_| CommandError::Memory)?; // XXX leaks pkg_contents
    pkg_contents
        .move_objects_from_toplevel(cnode.objs[0].cptr, cnode_depth as u8)
        .map_err(|_| CommandError::Memory)?; // XXX leaks pkg_contents + cnode
    match cantrip_pkg_mgmt_install(&pkg_contents) {
        Ok(bundle_id) => {
            writeln!(output, "Bundle \"{}\" installed", bundle_id)?;
        }
        Err(status) => {
            writeln!(output, "install failed: {:?}", status)?;
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
    _builtin_cpio: &[u8],
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

fn start_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    let bundle_id = args.next().ok_or(CommandError::BadArgs)?;
    match cantrip_proc_ctrl_start(bundle_id) {
        Ok(_) => {
            writeln!(output, "Bundle \"{}\" started.", bundle_id)?;
        }
        Err(status) => {
            writeln!(output, "start failed: {:?}", status)?;
        }
    }
    Ok(())
}

fn stop_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    let bundle_id = args.next().ok_or(CommandError::BadArgs)?;
    match cantrip_proc_ctrl_stop(bundle_id) {
        Ok(_) => {
            writeln!(output, "Bundle \"{}\" stopped.", bundle_id)?;
        }
        Err(status) => {
            writeln!(output, "stop failed: {:?}", status)?;
        }
    }
    Ok(())
}

fn kvdelete_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    let key = args.next().ok_or(CommandError::BadArgs)?;
    match cantrip_storage_delete(key) {
        Ok(_) => {
            writeln!(output, "Delete key \"{}\".", key)?;
        }
        Err(status) => {
            writeln!(output, "Delete key \"{}\" failed: {:?}", key, status)?;
        }
    }
    Ok(())
}

fn kvread_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    let key = args.next().ok_or(CommandError::BadArgs)?;
    match cantrip_storage_read(key) {
        Ok(value) => {
            writeln!(output, "Read key \"{}\" = {:?}.", key, value)?;
        }
        Err(status) => {
            writeln!(output, "Read key \"{}\" failed: {:?}", key, status)?;
        }
    }
    Ok(())
}

fn kvwrite_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    let key = args.next().ok_or(CommandError::BadArgs)?;
    let value = args.collect::<Vec<&str>>().join(" ");
    match cantrip_storage_write(key, value.as_bytes()) {
        Ok(_) => {
            writeln!(output, "Write key \"{}\" = {:?}.", key, value)?;
        }
        Err(status) => {
            writeln!(output, "Write key \"{}\" failed: {:?}", key, status)?;
        }
    }
    Ok(())
}

fn mdebug_command(
    _args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    if let Err(status) = cantrip_memory_debug() {
        writeln!(output, "stats failed: {:?}", status)?;
    }
    Ok(())
}

fn mstats(output: &mut dyn io::Write, stats: &MemoryManagerStats) -> Result<(), CommandError> {
    writeln!(
        output,
        "{} bytes in-use, {} bytes free, {} bytes requested, {} overhead",
        stats.allocated_bytes, stats.free_bytes, stats.total_requested_bytes, stats.overhead_bytes
    )?;
    writeln!(
        output,
        "{} objs in-use, {} objs requested",
        stats.allocated_objs, stats.total_requested_objs
    )?;
    Ok(())
}

fn mstats_command(
    _args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    match cantrip_memory_stats() {
        Ok(stats) => {
            mstats(output, &stats)?;
        }
        Err(status) => {
            writeln!(output, "stats failed: {:?}", status)?;
        }
    }
    Ok(())
}

fn state_mlcoord_command(
    _args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    _output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    return Ok(cantrip_mlcoord_debug_state());
}
