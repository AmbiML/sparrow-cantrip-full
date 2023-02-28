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

//! Infrequently used shell commands

extern crate alloc;
use crate::CmdFn;
use crate::CommandError;
use crate::HashMap;
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
) -> Result<(), CommandError> {
    Ok(output.write_str("\x1b\x63")?)
}

/// Implements an "echo" command which writes its arguments to output.
fn echo_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    let value = args.collect::<Vec<&str>>().join(" ");
    Ok(writeln!(output, "{}", &value)?)
}

/// Implements a command to receive a blob using ZMODEM.
fn rz_command(
    _args: &mut dyn Iterator<Item = &str>,
    input: &mut dyn io::BufRead,
    mut output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    let upload = crate::rz::rz(input, &mut output)?;
    writeln!(
        output,
        "size: {}, crc32: {}",
        upload.len(),
        hex::encode(upload.crc32().to_be_bytes())
    )?;
    Ok(())
}
