// SecurityCoordinator shell test commands

extern crate alloc;
use alloc::vec::Vec;
use core::fmt::Write;
use crate::CmdFn;
use crate::CommandError;
use crate::HashMap;

use cantrip_io as io;
use cantrip_memory_interface::cantrip_object_free_in_cnode;
use cantrip_os_common::cspace_slot::CSpaceSlot;
use cantrip_security_interface::*;
use cantrip_storage_interface::KEY_VALUE_DATA_SIZE;

pub fn add_cmds(cmds: &mut HashMap::<&str, CmdFn>) {
    cmds.extend([
        ("scecho",              scecho_command as CmdFn),
        ("size_buffer",         size_buffer_command as CmdFn),
        ("get_manifest",        get_manifest_command as CmdFn),
        ("load_application",    load_application_command as CmdFn),
        ("load_model",          load_model_command as CmdFn),
        ("delete_key",          delete_key_command as CmdFn),
        ("read_key",            read_key_command as CmdFn),
        ("write_key",           write_key_command as CmdFn),
        ("test_mailbox",        test_mailbox_command as CmdFn),
    ]);
}

/// Implements an "scecho" command that sends arguments to the Security Core's echo service.
fn scecho_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    let request = args.collect::<Vec<&str>>().join(" ");
    match cantrip_security_echo(&request) {
        Ok(result) => writeln!(output, "{}", result)?,
        Err(status) => writeln!(output, "ECHO replied {:?}", status)?,
    }
    Ok(())
}

fn size_buffer_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    let bundle_id = args.next().ok_or(CommandError::BadArgs)?;
    match cantrip_security_size_buffer(bundle_id) {
        Ok(size) => writeln!(output, "{}", size)?,
        Err(status) => writeln!(output, "SizeBuffer failed: {:?}", status)?,
    }
    Ok(())
}

fn get_manifest_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    let bundle_id = args.next().ok_or(CommandError::BadArgs)?;
    match cantrip_security_get_manifest(bundle_id) {
        Ok(manifest) => writeln!(output, "{}", manifest)?,
        Err(status) => writeln!(output, "GetManifest failed: {:?}", status)?,
    }
    Ok(())
}

fn load_application_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    let bundle_id = args.next().ok_or(CommandError::BadArgs)?;
    let mut container_slot = CSpaceSlot::new();
    match cantrip_security_load_application(bundle_id, &container_slot) {
        Ok(frames) => {
            container_slot.release(); // NB: take ownership
            writeln!(output, "{:?}", &frames)?;
            let _ = cantrip_object_free_in_cnode(&frames);
        },
        Err(status) => writeln!(output, "LoadApplication failed: {:?}", status)?,
    }
    Ok(())
}

fn load_model_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    let bundle_id = args.next().ok_or(CommandError::BadArgs)?;
    let model_id = args.next().ok_or(CommandError::BadArgs)?;
    let mut container_slot = CSpaceSlot::new();
    match cantrip_security_load_model(bundle_id, model_id, &container_slot) {
        Ok(frames) => {
            container_slot.release(); // NB: take ownership
            writeln!(output, "{:?}", &frames)?;
            let _ = cantrip_object_free_in_cnode(&frames);
        }
        Err(status) => writeln!(output, "LoadApplication failed: {:?}", status)?,
    }
    Ok(())
}

fn delete_key_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    let bundle_id = args.next().ok_or(CommandError::BadArgs)?;
    let key = args.next().ok_or(CommandError::BadArgs)?;
    match cantrip_security_delete_key(bundle_id, key) {
        Ok(_) => {
            writeln!(output, "Delete key \"{}\".", key)?;
        }
        Err(status) => {
            writeln!(output, "Delete key \"{}\" failed: {:?}", key, status)?;
        }
    }
    Ok(())
}

fn read_key_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    let bundle_id = args.next().ok_or(CommandError::BadArgs)?;
    let key = args.next().ok_or(CommandError::BadArgs)?;
    let mut keyval = [0u8; KEY_VALUE_DATA_SIZE];
    match cantrip_security_read_key(bundle_id, key, &mut keyval) {
        Ok(_) => {
            writeln!(output, "Read key \"{}\" = {:?}.", key, keyval)?;
        }
        Err(status) => {
            writeln!(output, "Read key \"{}\" failed: {:?}", key, status)?;
        }
    }
    Ok(())
}

fn write_key_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    let bundle_id = args.next().ok_or(CommandError::BadArgs)?;
    let key = args.next().ok_or(CommandError::BadArgs)?;
    let value = args.collect::<Vec<&str>>().join(" ");
    match cantrip_security_write_key(bundle_id, key, value.as_bytes()) {
        Ok(_) => {
            writeln!(output, "Write key \"{}\" = {:?}.", key, value)?;
        }
        Err(status) => {
            writeln!(output, "Write key \"{}\" failed: {:?}", key, status)?;
        }
    }
    Ok(())
}

fn test_mailbox_command(
    _args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    match cantrip_security_test_mailbox() {
        Ok(_) => {
            writeln!(output, "Test mailbox OK.")?;
        }
        Err(_status) => {
            writeln!(output, "Test mailbox failed.")?;
        }
    }
    Ok(())
}
