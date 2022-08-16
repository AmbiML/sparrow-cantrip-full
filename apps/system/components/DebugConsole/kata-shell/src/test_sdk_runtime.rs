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

//! SDK Runtime shell test commands

use crate::CmdFn;
use crate::CommandError;
use crate::HashMap;
use core::fmt::Write;

use cantrip_io as io;

use cantrip_sdk_interface::cantrip_sdk_ping;

pub fn add_cmds(cmds: &mut HashMap<&str, CmdFn>) {
    cmds.extend([("test_sdkping", sdk_ping_command as CmdFn)]);
}

fn sdk_ping_command(
    _args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    match cantrip_sdk_ping() {
        Ok(()) => {
            writeln!(output, "pong received")?;
        }
        Err(sdkerror) => {
            writeln!(output, "ping failed: {:?}", sdkerror)?;
        }
    }
    Ok(())
}
