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

//! Panic-related shell test commands

use crate::CmdFn;
use crate::CommandError;
use crate::HashMap;

use cantrip_io as io;

pub fn add_cmds(cmds: &mut HashMap<&str, CmdFn>) {
    cmds.extend([("test_panic", panic_command as CmdFn)]);
}

/// Implements a command that tests panic handling.
fn panic_command(
    _args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    _output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    panic!("testing");
}
