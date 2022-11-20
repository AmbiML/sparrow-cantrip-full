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
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::fmt::Write;
use cpio::CpioNewcReader;
use hashbrown::HashMap;

use cantrip_io as io;
use cantrip_line_reader::LineReader;
use cantrip_memory_interface::*;
#[cfg(feature = "ml_support")]
use cantrip_ml_interface::*;
use cantrip_os_common::sel4_sys;
use cantrip_os_common::slot_allocator;
use cantrip_proc_interface::cantrip_pkg_mgmt_install;
use cantrip_proc_interface::cantrip_pkg_mgmt_install_app;
use cantrip_proc_interface::cantrip_pkg_mgmt_uninstall;
use cantrip_proc_interface::cantrip_proc_ctrl_get_running_bundles;
use cantrip_proc_interface::cantrip_proc_ctrl_start;
use cantrip_proc_interface::cantrip_proc_ctrl_stop;
use cantrip_proc_interface::ProcessManagerError;
use cantrip_security_interface::cantrip_security_delete_key;
use cantrip_security_interface::cantrip_security_install_model;
use cantrip_security_interface::cantrip_security_read_key;
use cantrip_security_interface::cantrip_security_write_key;

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
#[cfg(all(feature = "ml_support", feature = "TEST_ML_COORDINATOR"))]
mod test_ml_coordinator;
#[cfg(feature = "TEST_PANIC")]
mod test_panic;
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

fn get_cmds() -> HashMap<&'static str, CmdFn> {
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
        #[cfg(feature = "timer_support")]
        ("sleep", sleep_command as CmdFn),
        ("source", source_command as CmdFn),
        ("start", start_command as CmdFn),
        ("stop", stop_command as CmdFn),
        ("uninstall", uninstall_command as CmdFn),
    ]);
    #[cfg(feature = "ml_support")]
    cmds.extend([("state_mlcoord", state_mlcoord_command as CmdFn)]);
    #[cfg(feature = "FRINGE_CMDS")]
    fringe_cmds::add_cmds(&mut cmds);
    #[cfg(feature = "TEST_GLOBAL_ALLOCATOR")]
    test_global_allocator::add_cmds(&mut cmds);
    #[cfg(feature = "TEST_MEMORY_MANAGER")]
    test_memory_manager::add_cmds(&mut cmds);
    #[cfg(all(feature = "ml_support", feature = "TEST_ML_COORDINATOR"))]
    test_ml_coordinator::add_cmds(&mut cmds);
    #[cfg(feature = "TEST_PANIC")]
    test_panic::add_cmds(&mut cmds);
    #[cfg(feature = "TEST_SECURITY_COORDINATOR")]
    test_security_coordinator::add_cmds(&mut cmds);
    #[cfg(feature = "TEST_TIMER_SERVICE")]
    test_timer_service::add_cmds(&mut cmds);
    #[cfg(feature = "TEST_UART")]
    test_uart::add_cmds(&mut cmds);

    cmds
}

pub fn eval<T: io::BufRead>(
    cmdline: &str,
    cmds: &HashMap<&str, CmdFn>,
    output: &mut dyn io::Write,
    input: &mut T,
    builtin_cpio: &[u8],
) {
    let mut args = cmdline.split_ascii_whitespace();
    match args.next() {
        Some("?") | Some("help") => {
            let mut keys: Vec<&str> = cmds.keys().copied().collect();
            keys.sort_unstable();
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

/// Read-eval-print loop for the DebugConsole command line interface.
pub fn repl<T: io::BufRead>(output: &mut dyn io::Write, input: &mut T, builtin_cpio: &[u8]) -> ! {
    let cmds = get_cmds();
    let mut line_reader = LineReader::new();
    loop {
        const PROMPT: &str = "CANTRIP> ";
        let _ = output.write_str(PROMPT);
        match line_reader.read_line(output, input) {
            Ok(cmdline) => eval(cmdline, &cmds, output, input, builtin_cpio),
            Err(e) => {
                let _ = writeln!(output, "\n{}", e);
            }
        }
    }
}

/// Stripped down repl for running automation scripts. Like repl but prints
/// each cmd line and stops at EOF/error.
pub fn repl_eof<T: io::BufRead>(output: &mut dyn io::Write, input: &mut T, builtin_cpio: &[u8]) {
    let cmds = get_cmds();
    let mut line_reader = LineReader::new();
    loop {
        // NB: LineReader echo's input
        let _ = write!(output, "CANTRIP> ");
        if let Ok(cmdline) = line_reader.read_line(output, input) {
            eval(cmdline, &cmds, output, input, builtin_cpio);
        } else {
            let _ = writeln!(output, "EOF");
            break;
        }
    }
}

/// Implements a command that pauses for a specified period of time.
#[cfg(feature = "timer_support")]
fn sleep_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    _output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    let time_str = args.next().ok_or(CommandError::BadArgs)?;
    let time_ms = time_str.parse::<u32>()?;

    use cantrip_timer_interface::*;
    match cantrip_timer_oneshot(0, time_ms) {
        Ok(_) => {
            cantrip_timer_wait().map_err(|_| CommandError::IO)?;
            Ok(())
        }
        _ => Err(CommandError::BadArgs),
    }
}

/// Implements a "source" command that interprets commands from a file
/// in the built-in cpio archive.
fn source_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    for script_name in args {
        let mut script_data: Option<&[u8]> = None;
        for e in CpioNewcReader::new(builtin_cpio) {
            if e.is_err() {
                writeln!(output, "cpio error")?;
                break; // NB: iterator does not terminate on error
            }
            let entry = e.unwrap();
            if entry.name == script_name {
                script_data = Some(entry.data);
                break;
            }
        }
        if let Some(data) = script_data {
            let mut script_input = cantrip_io::BufReader::new(default_uart_client::Rx::new(data));
            repl_eof(output, &mut script_input, builtin_cpio);
        } else {
            writeln!(output, "{}: not found", script_name)?;
        }
    }
    Ok(())
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
            writeln!(output, "cpio error")?;
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
        #[cfg(feature = "ml_support")]
        Some("mlcoord") => {
            let _ = cantrip_mlcoord_capscan();
        }
        Some("sdk") => {
            let _ = cantrip_sdk_manager::cantrip_sdk_manager_capscan();
        }
        Some("security") => {
            let _ = cantrip_security_interface::cantrip_security_capscan();
        }
        #[cfg(feature = "timer_support")]
        Some("timer") => {
            let _ = cantrip_timer_interface::cantrip_timer_capscan();
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
            #[cfg(feature = "ml_support")]
            writeln!(output, "  mlcoord (MlCoordinator)")?;
            writeln!(output, "  sdk (SDKRuntime)")?;
            writeln!(output, "  securiy (SecurityCoordinator)")?;
            #[cfg(feature = "timer_support")]
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
            writeln!(output, "cpio error").ok()?;
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

    enum PkgType {
        Bundle(ObjDescBundle),
        App {
            app_id: String,
            contents: ObjDescBundle,
        },
        Model {
            app_id: String,
            model_id: String,
            contents: ObjDescBundle,
        },
    }
    impl PkgType {
        // Returns an immutable ref to |contents|.
        pub fn get(&self) -> &ObjDescBundle {
            match self {
                PkgType::Bundle(contents) => contents,
                PkgType::App {
                    app_id: _,
                    contents,
                } => contents,
                PkgType::Model {
                    app_id: _,
                    model_id: _,
                    contents,
                } => contents,
            }
        }
        // Returns a mutable ref to |contents|.
        pub fn get_mut(&mut self) -> &mut ObjDescBundle {
            match self {
                PkgType::Bundle(contents) => contents,
                PkgType::App {
                    app_id: _,
                    contents,
                } => contents,
                PkgType::Model {
                    app_id: _,
                    model_id: _,
                    contents,
                } => contents,
            }
        }
        pub fn install(&self) -> Result<String, ProcessManagerError> {
            match self {
                PkgType::Bundle(contents) => cantrip_pkg_mgmt_install(contents),
                PkgType::App { app_id, contents } => {
                    cantrip_pkg_mgmt_install_app(app_id, contents).map(|_| app_id.clone())
                }
                PkgType::Model {
                    app_id,
                    model_id,
                    contents,
                } =>
                // NB: models go directly to the SecurityCoordinator
                // NB: true error is masked, ok for now as this is stopgap
                {
                    cantrip_security_install_model(app_id, model_id, contents).map_or_else(
                        |_| Err(ProcessManagerError::InstallFailed),
                        |_| Ok(model_id.clone()),
                    )
                }
            }
        }
    }
    impl fmt::Display for PkgType {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                PkgType::Bundle(_) => write!(f, "Bundle"),
                PkgType::App {
                    app_id: _,
                    contents: _,
                } => write!(f, "Application"),
                PkgType::Model {
                    app_id: _,
                    model_id: _,
                    contents: _,
                } => write!(f, "Model"),
            }
        }
    }
    // XXX add drop that reclaims contents

    // Collect/setup the package frames. If a -z arg is supplied a zmodem
    // upload is used; Otherwise the arg specifies the name of a file in
    // the builtins cpio archive.
    let mut pkg_contents: PkgType = match args.next().ok_or(CommandError::BadArgs)? {
        "-z" => PkgType::Bundle(collect_from_zmodem(input, &mut output).ok_or(CommandError::IO)?),
        filename => {
            let contents =
                collect_from_cpio(filename, builtin_cpio, output).ok_or(CommandError::IO)?;
            if let Some(app_id) = filename.strip_suffix(".app") {
                PkgType::App {
                    app_id: app_id.into(),
                    contents,
                }
            } else if let Some(model_id) = filename.strip_suffix(".model") {
                PkgType::Model {
                    app_id: filename.into(),
                    model_id: model_id.into(),
                    contents,
                }
            } else {
                PkgType::Bundle(contents)
            }
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
    let bundle_id = args.next().ok_or(CommandError::BadArgs)?;
    let key = args.next().ok_or(CommandError::BadArgs)?;
    match cantrip_security_delete_key(bundle_id, key) {
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
    let bundle_id = args.next().ok_or(CommandError::BadArgs)?;
    let key = args.next().ok_or(CommandError::BadArgs)?;
    let mut keyval = [0u8; cantrip_security_interface::KEY_VALUE_DATA_SIZE];
    match cantrip_security_read_key(bundle_id, key, &mut keyval) {
        Ok(_) => {
            writeln!(output, "Read key \"{}\" = {:?}.", key, keyval)?;
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
    let bundle_id = args.next().ok_or(CommandError::BadArgs)?;
    let key = args.next().ok_or(CommandError::BadArgs)?;
    let value = args.collect::<Vec<&str>>().join(" ");
    match cantrip_security_write_key(bundle_id, key, value.as_bytes()) {
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

#[cfg(feature = "ml_support")]
fn state_mlcoord_command(
    _args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    _output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    cantrip_mlcoord_debug_state();
    Ok(())
}
