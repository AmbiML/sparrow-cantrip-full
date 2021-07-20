#![no_std]

use core::fmt;
use core::fmt::Write;
use log::info;

use cantrip_io as io;
use cantrip_line_reader::LineReader;

/// Error type indicating why a command line is not runnable.
enum CommandError {
    UnknownCommand,
    BadArgs,
    Formatter(fmt::Error),
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CommandError::UnknownCommand => write!(f, "unknown command"),
            CommandError::BadArgs => write!(f, "invalid arguments"),
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

impl From<fmt::Error> for CommandError {
    fn from(err: fmt::Error) -> CommandError {
        CommandError::Formatter(err)
    }
}

/// Read-eval-print loop for the DebugConsole command line interface.
pub fn repl(output: &mut dyn io::Write, input: &mut dyn io::Read) -> ! {
    //  let _ = write!(output, "DebugConsole::repl()\n");
    info!("DebugConsole::repl()");
    let mut line_reader = LineReader::new();
    loop {
        const PROMPT: &str = "CANTRIP_PROMPT> ";
        let _ = output.write_str(PROMPT);
        match line_reader.read_line(output, input) {
            Ok(cmdline) => dispatch_command(cmdline, output),
            Err(e) => {
                let _ = write!(output, "\n{}\n", e);
            }
        }
    }
}

/// Runs a command line.
///
/// The line is split on whitespace. The first token is the command; the
/// remaining tokens are the arguments.
fn dispatch_command(cmdline: &str, output: &mut dyn io::Write) {
    let mut args = cmdline.split_ascii_whitespace();
    match args.nth(0) {
        Some(command) => {
            // Statically binds command names to implementations fns, which are
            // defined below.
            //
            // Since even the binding is static, it is fine for each command
            // implementation to use its own preferred signature.
            let result = match command {
                "echo" => echo_command(cmdline, output),
                "add" => add_command(&mut args, output),
                "clear" => clear_command(output),
                "ps" => ps_command(),
                "alloc_test" => alloc_test_command(output),
                _ => Err(CommandError::UnknownCommand),
            };
            if let Err(e) = result {
                let _ = write!(output, "{}\n", e);
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
        Ok(write!(
            output,
            "{}\n",
            &cmdline[COMMAND_LENGTH..cmdline.len()]
        )?)
    }
}

/// Implements a "ps" command that dumps seL4 scheduler state to the console.
fn ps_command() -> Result<(), CommandError> {
    extern "C" {
        fn sel4debug_dump_scheduler();
    }
    unsafe {
        sel4debug_dump_scheduler();
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
            return Ok(write!(output, "{}\n", x + y)?);
        }
    }
    Err(CommandError::BadArgs)
}

/// Implements a command that outputs the ANSI "clear console" sequence.
fn clear_command(output: &mut dyn io::Write) -> Result<(), CommandError> {
    Ok(output.write_str("\x1b\x63")?)
}

/// Implements a command that tests facilities that use the global allocator.
/// Shamelessly cribbed from https://os.phil-opp.com/heap-allocation/
fn alloc_test_command(output: &mut dyn io::Write) -> Result<(), CommandError> {
    extern crate alloc;
    use alloc::{boxed::Box, rc::Rc, vec, vec::Vec};

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
