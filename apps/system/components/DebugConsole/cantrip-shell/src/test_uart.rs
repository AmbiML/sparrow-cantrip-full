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

//! UART driver shell test commands

use crate::CmdFn;
use crate::CommandError;
use crate::HashMap;
use core::fmt::Write;

use cantrip_io as io;

// NB: not exported by driver so may diverge
const CIRCULAR_BUFFER_CAPACITY: usize = 512;

pub fn add_cmds(cmds: &mut HashMap<&str, CmdFn>) {
    cmds.extend([("test_uart", uart_command as CmdFn)]);
}

/// Exercise the UART driver.
fn uart_command(
    _args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    // Just like the cantrip shell prompt.
    const TESTING: &str = "testing...";
    output.write_str(TESTING)?;
    output.write_str(&"\n")?;

    // Fill the UART driver circular buffer.
    let not_too_long = "ok".repeat(CIRCULAR_BUFFER_CAPACITY / 2);
    output.write_str(&not_too_long)?;
    output.write_str(&"\n")?;

    // Overflow the UART driver circular buffer.
    let too_long = "no".repeat((CIRCULAR_BUFFER_CAPACITY / 2) + 1);
    output.write_str(&too_long)?;

    writeln!(output, "Success!")?;

    Ok(())
}
