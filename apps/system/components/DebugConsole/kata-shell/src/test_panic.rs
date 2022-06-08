// Panic-related shell test commands

use crate::CmdFn;
use crate::CommandError;
use crate::HashMap;

use cantrip_io as io;

pub fn add_cmds(cmds: &mut HashMap::<&str, CmdFn>) {
    cmds.extend([
        ("test_panic",          panic_command as CmdFn),
    ]);
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
