// Infrequently used shell commands

extern crate alloc;
use crate::rz;
use crate::CmdFn;
use crate::CommandError;
use alloc::vec::Vec;
use core::fmt::Write;

use cantrip_io as io;

pub fn add_cmds(cmds: &mut HashMap<&str, CmdFn>) {
    cmds.extend([
        ("add", add_command as CmdFn),
        ("echo", echo_command as CmdFn),
        ("clear", clear_command as CmdFn),
        ("rz", rz_command as CmdFn),
    ]);
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
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    let x_str = args.next().ok_or(CommandError::BadArgs)?;
    let x = x_str.parse::<f32>()?;
    let y_str = args.next().ok_or(CommandError::BadArgs)?;
    let y = y_str.parse::<f32>()?;
    return Ok(writeln!(output, "{}", x + y)?);
}

/// Implements a command that outputs the ANSI "clear console" sequence.
fn clear_command(
    _args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    Ok(output.write_str("\x1b\x63")?)
}

/// Implements an "echo" command which writes its arguments to output.
fn echo_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    let value = args.collect::<Vec<&str>>().join(" ");
    Ok(writeln!(output, "{}", &value)?)
}

/// Implements a command to receive a blob using ZMODEM.
fn rz_command(
    _args: &mut dyn Iterator<Item = &str>,
    input: &mut dyn io::BufRead,
    mut output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    let upload = rz::rz(input, &mut output)?;
    writeln!(
        output,
        "size: {}, crc32: {}",
        upload.len(),
        hex::encode(upload.crc32().to_be_bytes())
    )?;
    Ok(())
}
