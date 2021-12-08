#![no_std]

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::fmt::Write;
use hex;
use log;

use cantrip_io as io;
use cantrip_line_reader::LineReader;
use cantrip_os_common::sel4_sys::seL4_DebugDumpScheduler;
use cantrip_proc_interface::cantrip_pkg_mgmt_install;
use cantrip_proc_interface::cantrip_pkg_mgmt_uninstall;
use cantrip_proc_interface::cantrip_proc_ctrl_get_running_bundles;
use cantrip_proc_interface::cantrip_proc_ctrl_start;
use cantrip_proc_interface::cantrip_proc_ctrl_stop;
use cantrip_storage_interface::cantrip_storage_delete;
use cantrip_storage_interface::cantrip_storage_read;
use cantrip_storage_interface::cantrip_storage_write;

mod rz;

/// Error type indicating why a command line is not runnable.
enum CommandError {
    UnknownCommand,
    BadArgs,
    IO,
    Formatter(fmt::Error),
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CommandError::UnknownCommand => write!(f, "unknown command"),
            CommandError::BadArgs => write!(f, "invalid arguments"),
            CommandError::IO => write!(f, "input / output error"),
            CommandError::Formatter(e) => write!(f, "{}", e),
        }
    }
}

impl From<core::num::ParseIntError> for CommandError {
    fn from(_err: core::num::ParseIntError) -> CommandError {
        CommandError::BadArgs
    }
}

impl From<core::num::ParseFloatError> for CommandError {
    fn from(_err: core::num::ParseFloatError) -> CommandError {
        CommandError::BadArgs
    }
}

impl From<core::str::ParseBoolError> for CommandError {
    fn from(_err: core::str::ParseBoolError) -> CommandError {
        CommandError::BadArgs
    }
}

impl From<fmt::Error> for CommandError {
    fn from(err: fmt::Error) -> CommandError {
        CommandError::Formatter(err)
    }
}

impl From<io::Error> for CommandError {
    fn from(_err: io::Error) -> CommandError {
        CommandError::IO
    }
}

/// Read-eval-print loop for the DebugConsole command line interface.
pub fn repl<T: io::BufRead>(output: &mut dyn io::Write, input: &mut T) -> ! {
    let mut line_reader = LineReader::new();
    loop {
        const PROMPT: &str = "CANTRIP> ";
        let _ = output.write_str(PROMPT);
        match line_reader.read_line(output, input) {
            Ok(cmdline) => dispatch_command(cmdline, input, output),
            Err(e) => {
                let _ = writeln!(output, "\n{}", e);
            }
        }
    }
}

/// Runs a command line.
///
/// The line is split on whitespace. The first token is the command; the
/// remaining tokens are the arguments.
fn dispatch_command(cmdline: &str, input: &mut dyn io::BufRead, output: &mut dyn io::Write) {
    let mut args = cmdline.split_ascii_whitespace();
    match args.nth(0) {
        Some(command) => {
            // Statically binds command names to implementations fns, which are
            // defined below.
            //
            // Since even the binding is static, it is fine for each command
            // implementation to use its own preferred signature.
            let result = match command {
                "add" => add_command(&mut args, output),
                "echo" => echo_command(cmdline, output),
                "clear" => clear_command(output),
                "bundles" => bundles_command(output),
                "kvdelete" => kvdelete_command(&mut args, output),
                "kvread" => kvread_command(&mut args, output),
                "kvwrite" => kvwrite_command(&mut args, output),
                "install" => install_command(&mut args, output),
                "loglevel" => loglevel_command(&mut args, output),
                "rz" => rz_command(input, output),
                "ps" => ps_command(),
                "scecho" => scecho_command(cmdline, output),
                "start" => start_command(&mut args, output),
                "stop" => stop_command(&mut args, output),
                "uninstall" => uninstall_command(&mut args, output),

                "test_alloc" => test_alloc_command(output),
                "test_alloc_error" => test_alloc_error_command(output),
                "test_panic" => test_panic_command(),
                "test_mlexecute" => test_mlexecute_command(),
                "test_mlcontinuous" => test_mlcontinuous_command(&mut args),

                _ => Err(CommandError::UnknownCommand),
            };
            if let Err(e) = result {
                let _ = writeln!(output, "{}", e);
            };
        }
        None => {
            let _ = output.write_str("\n");
        }
    };
}

/// Implements an "echo" command which writes its arguments to output.
fn echo_command(cmdline: &str, output: &mut dyn io::Write) -> Result<(), CommandError> {
    const COMMAND_LENGTH: usize = 5; // "echo "
    if cmdline.len() < COMMAND_LENGTH {
        Ok(())
    } else {
        Ok(writeln!(
            output,
            "{}",
            &cmdline[COMMAND_LENGTH..cmdline.len()]
        )?)
    }
}

/// Implements an "scecho" command that sends arguments to the Security Core's echo service.
fn scecho_command(cmdline: &str, output: &mut dyn io::Write) -> Result<(), CommandError> {
    use cantrip_security_interface::cantrip_security_request;
    use cantrip_security_interface::EchoRequest;
    use cantrip_security_interface::SecurityRequest;
    use cantrip_security_interface::SECURITY_REPLY_DATA_SIZE;

    let (_, request) = cmdline.split_at(7); // 'scecho'
    let reply = &mut [0u8; SECURITY_REPLY_DATA_SIZE];
    match cantrip_security_request(
        SecurityRequest::SrEcho,
        &EchoRequest {
            value: request.as_bytes(),
        },
        reply,
    ) {
        Ok(_) => {
            writeln!(
                output,
                "{}",
                String::from_utf8_lossy(&reply[..request.len()])
            )?;
        }
        Err(status) => {
            writeln!(output, "ECHO replied {:?}", status)?;
        }
    }
    Ok(())
}

/// Implements a command to configure the max log level for the DebugConsole.
fn loglevel_command(
    args: &mut dyn Iterator<Item = &str>,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    if let Some(level) = args.nth(0) {
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

/// Implements a command to receive a blob using ZMODEM.
fn rz_command(
    input: &mut dyn io::BufRead,
    mut output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    let upload = rz::rz(input, &mut output)?;
    writeln!(
        output,
        "size: {}, crc32: {}",
        upload.contents().len(),
        hex::encode(upload.crc32().to_be_bytes())
    )?;
    Ok(())
}

/// Implements a "ps" command that dumps seL4 scheduler state to the console.
fn ps_command() -> Result<(), CommandError> {
    unsafe {
        seL4_DebugDumpScheduler();
    }
    Ok(())
}

/// Implements a binary float addition command.
///
/// This is a toy to demonstrate that the CLI can operate on some very basic
/// dynamic input and that the Rust runtime provides floating point arithmetic
/// on integer-only hardware. It is also a prototype example of "command taking
/// arguments." It should be removed once actually useful system control
/// commands are implemented and done cribbing from it.
fn add_command(
    args: &mut dyn Iterator<Item = &str>,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    if let Some(x_str) = args.nth(0) {
        if let Some(y_str) = args.nth(0) {
            let x = x_str.parse::<f32>()?;
            let y = y_str.parse::<f32>()?;
            return Ok(writeln!(output, "{}", x + y)?);
        }
    }
    Err(CommandError::BadArgs)
}

/// Implements a command that outputs the ANSI "clear console" sequence.
fn clear_command(output: &mut dyn io::Write) -> Result<(), CommandError> {
    Ok(output.write_str("\x1b\x63")?)
}

fn bundles_command(output: &mut dyn io::Write) -> Result<(), CommandError> {
    match cantrip_proc_ctrl_get_running_bundles() {
        Ok(bundle_ids) => {
            writeln!(output, "{}", bundle_ids.join("\n"))?;
        }
        Err(status) => {
            writeln!(output, "get_running_bundles failed: {:?}", status)?;
        }
    }
    Ok(())
}

fn install_command(
    _args: &mut dyn Iterator<Item = &str>,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    // TODO(sleffler): supply a real bundle (e.g. from serial)
    let pkg_buffer = &[0u8; 64];
    match cantrip_pkg_mgmt_install(pkg_buffer) {
        Ok(bundle_id) => {
            writeln!(output, "Bundle \"{}\" installed", bundle_id)?;
        }
        Err(status) => {
            writeln!(output, "install failed: {:?}", status)?;
        }
    }
    Ok(())
}

fn uninstall_command(
    args: &mut dyn Iterator<Item = &str>,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    if let Some(bundle_id) = args.nth(0) {
        match cantrip_pkg_mgmt_uninstall(bundle_id) {
            Ok(_) => {
                writeln!(output, "Bundle \"{}\" uninstalled.", bundle_id)?;
            }
            Err(status) => {
                writeln!(output, "uninstall failed: {:?}", status)?;
            }
        }
        Ok(())
    } else {
        Err(CommandError::BadArgs)
    }
}

fn start_command(
    args: &mut dyn Iterator<Item = &str>,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    if let Some(bundle_id) = args.nth(0) {
        match cantrip_proc_ctrl_start(bundle_id) {
            Ok(_) => {
                writeln!(output, "Bundle \"{}\" started.", bundle_id)?;
            }
            Err(status) => {
                writeln!(output, "start failed: {:?}", status)?;
            }
        }
        Ok(())
    } else {
        Err(CommandError::BadArgs)
    }
}

fn stop_command(
    args: &mut dyn Iterator<Item = &str>,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    if let Some(bundle_id) = args.nth(0) {
        match cantrip_proc_ctrl_stop(bundle_id) {
            Ok(_) => {
                writeln!(output, "Bundle \"{}\" stopped.", bundle_id)?;
            }
            Err(status) => {
                writeln!(output, "stop failed: {:?}", status)?;
            }
        }
        Ok(())
    } else {
        Err(CommandError::BadArgs)
    }
}

fn kvdelete_command(
    args: &mut dyn Iterator<Item = &str>,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    if let Some(key) = args.nth(0) {
        match cantrip_storage_delete(key) {
            Ok(_) => {
                writeln!(output, "Delete key \"{}\".", key)?;
            }
            Err(status) => {
                writeln!(output, "Delete key \"{}\" failed: {:?}", key, status)?;
            }
        }
        Ok(())
    } else {
        Err(CommandError::BadArgs)
    }
}

fn kvread_command(
    args: &mut dyn Iterator<Item = &str>,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    if let Some(key) = args.nth(0) {
        match cantrip_storage_read(key) {
            Ok(value) => {
                writeln!(output, "Read key \"{}\" = {:?}.", key, value)?;
            }
            Err(status) => {
                writeln!(output, "Read key \"{}\" failed: {:?}", key, status)?;
            }
        }
        Ok(())
    } else {
        Err(CommandError::BadArgs)
    }
}

fn kvwrite_command(
    args: &mut dyn Iterator<Item = &str>,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    if let Some(key) = args.nth(0) {
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
    } else {
        Err(CommandError::BadArgs)
    }
}

/// Implements a command that tests facilities that use the global allocator.
/// Shamelessly cribbed from https://os.phil-opp.com/heap-allocation/
fn test_alloc_command(output: &mut dyn io::Write) -> Result<(), CommandError> {
    extern crate alloc;
    use alloc::{boxed::Box, rc::Rc, vec};

    // allocate a number on the heap
    let heap_value = Box::new(41);
    writeln!(output, "heap_value at {:p}", heap_value).expect("Box failed");

    // create a dynamically sized vector
    let mut vec = Vec::new();
    for i in 0..500 {
        vec.push(i);
    }
    writeln!(output, "vec at {:p}", vec.as_slice()).expect("Vec failed");

    // create a reference counted vector -> will be freed when count reaches 0
    let reference_counted = Rc::new(vec![1, 2, 3]);
    let cloned_reference = reference_counted.clone();
    writeln!(
        output,
        "current reference count is {}",
        Rc::strong_count(&cloned_reference)
    )
    .expect("Rc 1 failed");
    core::mem::drop(reference_counted);
    writeln!(
        output,
        "reference count is {} now",
        Rc::strong_count(&cloned_reference)
    )
    .expect("Rc 2 failed");

    Ok(writeln!(output, "All tests passed!")?)
}

/// Implements a command that tests the global allocator error handling.
fn test_alloc_error_command(output: &mut dyn io::Write) -> Result<(), CommandError> {
    // Default heap holds 16KB.
    let mut vec = Vec::with_capacity(16384);
    for i in 0..16348 {
        vec.push(i);
    }
    Ok(writeln!(output, "vec at {:p}", vec.as_slice())?)
}

/// Implements a command that tests panic handling.
fn test_panic_command() -> Result<(), CommandError> {
    panic!("testing");
}

/// Implements a command that runs an ML execution.
fn test_mlexecute_command() -> Result<(), CommandError> {
    extern "C" {
        fn mlcoord_execute();
    }
    unsafe {
        mlcoord_execute();
    }
    Ok(())
}

/// Implements a command that sets whether the ml execution is continuous.
fn test_mlcontinuous_command(args: &mut dyn Iterator<Item = &str>) -> Result<(), CommandError> {
    extern "C" {
        fn mlcoord_set_continuous_mode(mode: bool);
    }
    if let Some(mode_str) = args.nth(0) {
        let mode = mode_str.parse::<bool>()?;
        unsafe {
            mlcoord_set_continuous_mode(mode);
        }
        return Ok(());
    }
    Err(CommandError::BadArgs)
}
