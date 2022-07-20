// UART driver shell test commands

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
