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
use hashbrown::HashMap;

use cantrip_line_reader::LineReader;
use cantrip_memory_interface::*;
#[cfg(feature = "ml_support")]
use cantrip_ml_interface::*;
use cantrip_os_common::cspace_slot::CSpaceSlot;
use cantrip_os_common::sel4_sys;
use cantrip_proc_interface::cantrip_proc_ctrl_get_running_bundles;
use cantrip_proc_interface::cantrip_proc_ctrl_start;
use cantrip_proc_interface::cantrip_proc_ctrl_stop;
use cantrip_security_interface::cantrip_security_delete_key;
use cantrip_security_interface::cantrip_security_get_packages;
use cantrip_security_interface::cantrip_security_load_application;
use cantrip_security_interface::cantrip_security_read_key;
use cantrip_security_interface::cantrip_security_write_key;

use sel4_sys::seL4_CPtr;

use cantrip_io as io;

#[cfg(any(
    feature = "dynamic_load_support",
    all(feature = "CONFIG_DEBUG_BUILD", feature = "FRINGE_CMDS"),
))]
mod rz;

#[cfg(feature = "dynamic_load_support")]
mod dynamic_load;
#[cfg(all(feature = "CONFIG_DEBUG_BUILD", feature = "FRINGE_CMDS"))]
mod fringe_cmds;
#[cfg(all(feature = "CONFIG_DEBUG_BUILD", feature = "TEST_GLOBAL_ALLOCATOR"))]
mod test_global_allocator;
#[cfg(all(feature = "CONFIG_DEBUG_BUILD", feature = "TEST_MEMORY_MANAGER"))]
mod test_memory_manager;
#[cfg(all(
    feature = "ml_support",
    feature = "CONFIG_DEBUG_BUILD",
    feature = "TEST_ML_COORDINATOR"
))]
mod test_ml_coordinator;
#[cfg(all(feature = "CONFIG_DEBUG_BUILD", feature = "TEST_PANIC"))]
mod test_panic;
#[cfg(all(feature = "CONFIG_DEBUG_BUILD", feature = "TEST_SECURITY_COORDINATOR"))]
mod test_security_coordinator;
#[cfg(all(feature = "CONFIG_DEBUG_BUILD", feature = "TEST_TIMER_SERVICE"))]
mod test_timer_service;
#[cfg(all(feature = "CONFIG_DEBUG_BUILD", feature = "TEST_UART"))]
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
) -> Result<(), CommandError>;

fn get_cmds() -> HashMap<&'static str, CmdFn> {
    let mut cmds = HashMap::<&str, CmdFn>::new();
    cmds.extend([
        ("builtins", packages_command as CmdFn), // NB: for backwards compat
        ("bundles", bundles_command as CmdFn),
        ("capscan", capscan_command as CmdFn),
        ("kvdelete", kvdelete_command as CmdFn),
        ("kvread", kvread_command as CmdFn),
        ("kvwrite", kvwrite_command as CmdFn),
        ("loglevel", loglevel_command as CmdFn),
        ("mdebug", mdebug_command as CmdFn),
        ("mstats", mstats_command as CmdFn),
        ("packages", packages_command as CmdFn),
        ("ps", ps_command as CmdFn),
        #[cfg(feature = "timer_support")]
        ("sleep", sleep_command as CmdFn),
        ("source", source_command as CmdFn),
        ("start", start_command as CmdFn),
        ("stop", stop_command as CmdFn),
    ]);
    #[cfg(feature = "ml_support")]
    cmds.extend([("state_mlcoord", state_mlcoord_command as CmdFn)]);
    #[cfg(feature = "dynamic_load_support")]
    dynamic_load::add_cmds(&mut cmds);
    #[cfg(all(feature = "CONFIG_DEBUG_BUILD", feature = "FRINGE_CMDS"))]
    fringe_cmds::add_cmds(&mut cmds);
    #[cfg(all(feature = "CONFIG_DEBUG_BUILD", feature = "TEST_GLOBAL_ALLOCATOR"))]
    test_global_allocator::add_cmds(&mut cmds);
    #[cfg(all(feature = "CONFIG_DEBUG_BUILD", feature = "TEST_MEMORY_MANAGER"))]
    test_memory_manager::add_cmds(&mut cmds);
    #[cfg(all(
        feature = "ml_support",
        feature = "CONFIG_DEBUG_BUILD",
        feature = "TEST_ML_COORDINATOR"
    ))]
    test_ml_coordinator::add_cmds(&mut cmds);
    #[cfg(all(feature = "CONFIG_DEBUG_BUILD", feature = "TEST_PANIC"))]
    test_panic::add_cmds(&mut cmds);
    #[cfg(all(feature = "CONFIG_DEBUG_BUILD", feature = "TEST_SECURITY_COORDINATOR"))]
    test_security_coordinator::add_cmds(&mut cmds);
    #[cfg(all(feature = "CONFIG_DEBUG_BUILD", feature = "TEST_TIMER_SERVICE"))]
    test_timer_service::add_cmds(&mut cmds);
    #[cfg(all(feature = "CONFIG_DEBUG_BUILD", feature = "TEST_UART"))]
    test_uart::add_cmds(&mut cmds);

    cmds
}

pub fn eval<T: io::BufRead>(
    cmdline: &str,
    cmds: &HashMap<&str, CmdFn>,
    output: &mut dyn io::Write,
    input: &mut T,
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
                |func| func(&mut args, input, output),
            );
            if let Err(e) = result {
                let _ = writeln!(output, "{}: {}", e, cmd);
            };
        }
        None => {
            let _ = output.write_str("\n");
        }
    }
}

/// Read-eval-print loop for the DebugConsole command line interface.
pub fn repl<T: io::BufRead>(output: &mut dyn io::Write, input: &mut T) -> ! {
    let cmds = get_cmds();
    let mut line_reader = LineReader::new();
    loop {
        const PROMPT: &str = "CANTRIP> ";
        let _ = output.write_str(PROMPT);
        match line_reader.read_line(output, input) {
            Ok(cmdline) => eval(cmdline, &cmds, output, input),
            Err(e) => {
                let _ = writeln!(output, "\n{}", e);
            }
        }
    }
}

/// Stripped down repl for running automation scripts. Like repl but prints
/// each cmd line and stops at EOF/error.
pub fn repl_eof<T: io::BufRead>(output: &mut dyn io::Write, input: &mut T) {
    let cmds = get_cmds();
    let mut line_reader = LineReader::new();
    loop {
        // NB: LineReader echo's input
        let _ = write!(output, "CANTRIP> ");
        if let Ok(cmdline) = line_reader.read_line(output, input) {
            eval(cmdline, &cmds, output, input);
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
) -> Result<(), CommandError> {
    let time_str = args.next().ok_or(CommandError::BadArgs)?;
    let time_ms = time_str.parse::<u32>()?;

    use cantrip_timer_interface::*;
    match cantrip_timer_oneshot(0, time_ms) {
        Ok(_) => {
            cantrip_timer_wait().or(Err(CommandError::IO))?;
            Ok(())
        }
        _ => Err(CommandError::BadArgs),
    }
}

/// Implements a command that interprets commands from an installed package.
fn source_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    let (quiet, filename) = match args.next().ok_or(CommandError::BadArgs)? {
        "-q" => (true, args.next().ok_or(CommandError::BadArgs)?),
        filename => (false, filename),
    };
    let mut container_slot = CSpaceSlot::new();
    match cantrip_security_load_application(filename, &container_slot) {
        Ok(frames) => {
            container_slot.release(); // NB: take ownership
            let mut script_input = io::BufReader::new(cantrip_io_objdesc::Rx::new(&frames));
            repl_eof(output, &mut script_input);
            // NB: must drop refs before freeing the bundle
            drop(script_input);
            let _ = cantrip_object_free_in_cnode(&frames);
        }
        Err(status) => {
            if !quiet {
                writeln!(output, "source {} failed: {:?}", filename, status)?
            };
        }
    }
    Ok(())
}

/// Implements a command that lists the available packages.
fn packages_command(
    _args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    match cantrip_security_get_packages() {
        Ok(bundle_ids) => {
            for b in bundle_ids {
                writeln!(output, "{}", b)?;
            }
        }
        Err(status) => {
            writeln!(output, "get_packages failed: {:?}", status)?;
        }
    }
    Ok(())
}

/// Implements a command to configure the max log level for the DebugConsole.
fn loglevel_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
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

fn start_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
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
) -> Result<(), CommandError> {
    let bundle_id = args.next().ok_or(CommandError::BadArgs)?;
    let key = args.next().ok_or(CommandError::BadArgs)?;
    match cantrip_security_read_key(bundle_id, key) {
        Ok(keyval) => {
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
) -> Result<(), CommandError> {
    cantrip_mlcoord_debug_state();
    Ok(())
}
