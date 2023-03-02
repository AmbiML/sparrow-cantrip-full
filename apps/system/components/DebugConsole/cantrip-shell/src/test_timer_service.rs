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

//! TimerService shell test commands

use crate::CmdFn;
use crate::CommandError;
use crate::HashMap;
use core::fmt::Write;

use cantrip_io as io;

use cantrip_timer_interface::cantrip_timer_completed_timers;
use cantrip_timer_interface::cantrip_timer_oneshot;
use cantrip_timer_interface::cantrip_timer_wait;

pub fn add_cmds(cmds: &mut HashMap<&str, CmdFn>) {
    cmds.extend([
        ("test_timer_async", timer_async_command as CmdFn),
        ("test_timer_blocking", timer_blocking_command as CmdFn),
        ("test_timer_completed", timer_completed_command as CmdFn),
    ]);
}

/// Implements a command that starts a timer, but does not wait on the
/// notification.
fn timer_async_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    let id_str = args.next().ok_or(CommandError::BadArgs)?;
    let id = id_str.parse::<u32>()?;
    let time_str = args.next().ok_or(CommandError::BadArgs)?;
    let time_ms = time_str.parse::<u32>()?;

    writeln!(output, "Starting timer {} for {} ms.", id, time_ms)?;

    if let Err(e) = cantrip_timer_oneshot(id, time_ms) {
        writeln!(output, "cantrip_timer_oneshot failed: {:?}", e)?;
        return Err(CommandError::BadArgs);
    }

    Ok(())
}

/// Implements a command that starts a timer, blocking until the timer has
/// completed.
fn timer_blocking_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    let time_str = args.next().ok_or(CommandError::BadArgs)?;
    let time_ms = time_str.parse::<u32>()?;

    writeln!(output, "Blocking {} ms waiting for timer.", time_ms)?;

    // Set timer_id to 0, we don't need to use multiple timers here.
    if let Err(e) = cantrip_timer_oneshot(0, time_ms) {
        writeln!(output, "cantrip_timer_periodic failed: {:?}", e)?;
        return Err(CommandError::BadArgs);
    }

    let _ = cantrip_timer_wait();

    Ok(writeln!(output, "Timer completed.")?)
}

/// Implements a command that checks the completed timers.
fn timer_completed_command(
    _args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    Ok(writeln!(
        output,
        "Timers completed: {:#032b}",
        cantrip_timer_completed_timers().unwrap(),
    )?)
}
