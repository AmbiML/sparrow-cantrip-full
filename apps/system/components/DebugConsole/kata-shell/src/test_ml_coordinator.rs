// MlCoordinator service shell test commands

use core::fmt::Write;
use crate::CmdFn;
use crate::CommandError;
use crate::HashMap;

use cantrip_io as io;
use cantrip_ml_interface::*;

pub fn add_cmds(cmds: &mut HashMap::<&str, CmdFn>) {
    cmds.extend([
        ("test_mlcancel",       mlcancel_command as CmdFn),
        ("test_mlexecute",      mlexecute_command as CmdFn),
        ("test_mlperiodic",     mlperiodic_command as CmdFn),
    ]);
}

/// Implements a command that cancels an ML execution.
fn mlcancel_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    let bundle_id = args.next().ok_or(CommandError::BadArgs)?;
    let model_id = args.next().ok_or(CommandError::BadArgs)?;

    if let Err(e) = cantrip_mlcoord_cancel(bundle_id, model_id) {
        writeln!(output, "Cancel {:?} {:?} err: {:?}", bundle_id, model_id, e)?;
    } else {
        writeln!(output, "Cancelled {:?} {:?}", bundle_id, model_id)?;
    }
    Ok(())
}

/// Implements a command that runs a oneshot ML execution.
fn mlexecute_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    let bundle_id = args.next().ok_or(CommandError::BadArgs)?;
    let model_id = args.next().ok_or(CommandError::BadArgs)?;

    if let Err(e) = cantrip_mlcoord_oneshot(bundle_id, model_id) {
        writeln!(output, "Execute {:?} {:?} err: {:?}", bundle_id, model_id, e)?;
    }

    Ok(())
}

/// Implements a command that runs a periodic ML execution.
fn mlperiodic_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    let bundle_id = args.next().ok_or(CommandError::BadArgs)?;
    let model_id = args.next().ok_or(CommandError::BadArgs)?;
    let rate_str = args.next().ok_or(CommandError::BadArgs)?;
    let rate_in_ms = rate_str.parse::<u32>()?;
  
    if let Err(e) = cantrip_mlcoord_periodic(bundle_id, model_id, rate_in_ms) {
        writeln!(output, "Periodic {:?} {:?} err: {:?}", bundle_id, model_id, e)?;
    }

    Ok(())
}
