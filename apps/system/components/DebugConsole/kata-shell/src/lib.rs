#![no_std]

extern crate alloc;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;
use core::fmt::Write;
use hex;
use log;

use cantrip_io as io;
use cantrip_line_reader::LineReader;
use cantrip_memory_interface::*;
use cantrip_os_common::sel4_sys;
use cantrip_os_common::slot_allocator;
use cantrip_proc_interface::cantrip_pkg_mgmt_install;
use cantrip_proc_interface::cantrip_pkg_mgmt_uninstall;
use cantrip_proc_interface::cantrip_proc_ctrl_get_running_bundles;
use cantrip_proc_interface::cantrip_proc_ctrl_start;
use cantrip_proc_interface::cantrip_proc_ctrl_stop;
use cantrip_storage_interface::cantrip_storage_delete;
use cantrip_storage_interface::cantrip_storage_read;
use cantrip_storage_interface::cantrip_storage_write;
use cantrip_timer_interface::TimerServiceError;
use cantrip_timer_interface::timer_service_completed_timers;
use cantrip_timer_interface::timer_service_oneshot;
use cantrip_timer_interface::timer_service_wait;

use sel4_sys::seL4_CNode_Delete;
use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_MinSchedContextBits;
use sel4_sys::seL4_ObjectType::*;
use sel4_sys::seL4_WordBits;

use slot_allocator::CANTRIP_CSPACE_SLOTS;

mod rz;

extern "C" {
    static SELF_CNODE: seL4_CPtr;
}

/// Error type indicating why a command line is not runnable.
enum CommandError {
    UnknownCommand,
    BadArgs,
    IO,
    Memory,
    Formatter(fmt::Error),
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CommandError::UnknownCommand => write!(f, "unknown command"),
            CommandError::BadArgs => write!(f, "invalid arguments"),
            CommandError::IO => write!(f, "input / output error"),
            CommandError::Memory => write!(f, "memory allocation error"),
            CommandError::Formatter(e) => write!(f, "{}", e),
        }
    }
}

impl From<core::num::ParseIntError> for CommandError {
    fn from(_err: core::num::ParseIntError) -> CommandError {
        CommandError::BadArgs
    }
}

impl From<core::num::ParseFloatError> for CommandError {
    fn from(_err: core::num::ParseFloatError) -> CommandError {
        CommandError::BadArgs
    }
}

impl From<core::str::ParseBoolError> for CommandError {
    fn from(_err: core::str::ParseBoolError) -> CommandError {
        CommandError::BadArgs
    }
}

impl From<fmt::Error> for CommandError {
    fn from(err: fmt::Error) -> CommandError {
        CommandError::Formatter(err)
    }
}

impl From<io::Error> for CommandError {
    fn from(_err: io::Error) -> CommandError {
        CommandError::IO
    }
}

/// Read-eval-print loop for the DebugConsole command line interface.
pub fn repl<T: io::BufRead>(output: &mut dyn io::Write, input: &mut T) -> ! {
    let mut line_reader = LineReader::new();
    loop {
        const PROMPT: &str = "CANTRIP> ";
        let _ = output.write_str(PROMPT);
        match line_reader.read_line(output, input) {
            Ok(cmdline) => dispatch_command(cmdline, input, output),
            Err(e) => {
                let _ = writeln!(output, "\n{}", e);
            }
        }
    }
}

/// Runs a command line.
///
/// The line is split on whitespace. The first token is the command; the
/// remaining tokens are the arguments.
fn dispatch_command(cmdline: &str, input: &mut dyn io::BufRead, output: &mut dyn io::Write) {
    let mut args = cmdline.split_ascii_whitespace();
    match args.nth(0) {
        Some(command) => {
            // Statically binds command names to implementations fns, which are
            // defined below.
            //
            // Since even the binding is static, it is fine for each command
            // implementation to use its own preferred signature.
            let result = match command {
                "add" => add_command(&mut args, output),
                "echo" => echo_command(cmdline, output),
                "clear" => clear_command(output),
                "bundles" => bundles_command(output),
                "kvdelete" => kvdelete_command(&mut args, output),
                "kvread" => kvread_command(&mut args, output),
                "kvwrite" => kvwrite_command(&mut args, output),
                "install" => install_command(&mut args, input, output),
                "loglevel" => loglevel_command(&mut args, output),
                "malloc" => malloc_command(&mut args, output),
                "mfree" => mfree_command(&mut args, output),
                "mstats" => mstats_command(&mut args, output),
                "rz" => rz_command(input, output),
                "ps" => ps_command(output),
                "scecho" => scecho_command(cmdline, output),
                "start" => start_command(&mut args, output),
                "stop" => stop_command(&mut args, output),
                "uninstall" => uninstall_command(&mut args, output),

                "test_alloc" => test_alloc_command(output),
                "test_alloc_error" => test_alloc_error_command(output),
                "test_bootinfo" => test_bootinfo_command(output),
                "test_mlexecute" => test_mlexecute_command(),
                "test_mlcontinuous" => test_mlcontinuous_command(&mut args),
                "test_obj_alloc" => test_obj_alloc_command(output),
                "test_panic" => test_panic_command(),
                "test_timer_async" => test_timer_async_command(&mut args, output),
                "test_timer_blocking" => test_timer_blocking_command(&mut args, output),
                "test_timer_completed" => test_timer_completed_command(output),

                _ => Err(CommandError::UnknownCommand),
            };
            if let Err(e) = result {
                let _ = writeln!(output, "{}", e);
            };
        }
        None => {
            let _ = output.write_str("\n");
        }
    };
}

/// Implements an "echo" command which writes its arguments to output.
fn echo_command(cmdline: &str, output: &mut dyn io::Write) -> Result<(), CommandError> {
    const COMMAND_LENGTH: usize = 5; // "echo "
    if cmdline.len() < COMMAND_LENGTH {
        Ok(())
    } else {
        Ok(writeln!(
            output,
            "{}",
            &cmdline[COMMAND_LENGTH..cmdline.len()]
        )?)
    }
}

/// Implements an "scecho" command that sends arguments to the Security Core's echo service.
fn scecho_command(cmdline: &str, output: &mut dyn io::Write) -> Result<(), CommandError> {
    use cantrip_security_interface::cantrip_security_request;
    use cantrip_security_interface::EchoRequest;
    use cantrip_security_interface::SecurityRequest;
    use cantrip_security_interface::SECURITY_REPLY_DATA_SIZE;

    let (_, request) = cmdline.split_at(7); // 'scecho'
    let reply = &mut [0u8; SECURITY_REPLY_DATA_SIZE];
    match cantrip_security_request(
        SecurityRequest::SrEcho,
        &EchoRequest {
            value: request.as_bytes(),
        },
        reply,
    ) {
        Ok(_) => {
            writeln!(
                output,
                "{}",
                String::from_utf8_lossy(&reply[..request.len()])
            )?;
        }
        Err(status) => {
            writeln!(output, "ECHO replied {:?}", status)?;
        }
    }
    Ok(())
}

/// Implements a command to configure the max log level for the DebugConsole.
fn loglevel_command(
    args: &mut dyn Iterator<Item = &str>,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    if let Some(level) = args.nth(0) {
        use log::LevelFilter;
        match level {
            "off" => log::set_max_level(LevelFilter::Off),
            "debug" => log::set_max_level(LevelFilter::Debug),
            "info" => log::set_max_level(LevelFilter::Info),
            "error" => log::set_max_level(LevelFilter::Error),
            "trace" => log::set_max_level(LevelFilter::Trace),
            "warn" => log::set_max_level(LevelFilter::Warn),
            _ => writeln!(output, "Unknown log level {}", level)?,
        }
    }
    Ok(writeln!(output, "{}", log::max_level())?)
}

/// Implements a command to receive a blob using ZMODEM.
fn rz_command(
    input: &mut dyn io::BufRead,
    mut output: &mut dyn io::Write,
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

/// Implements a "ps" command that dumps seL4 scheduler state to the console.
#[cfg(feature = "CONFIG_DEBUG_BUILD")]
fn ps_command(_output: &mut dyn io::Write) -> Result<(), CommandError> {
    unsafe {
        cantrip_os_common::sel4_sys::seL4_DebugDumpScheduler();
    }
    Ok(())
}

#[cfg(not(feature = "CONFIG_DEBUG_BUILD"))]
fn ps_command(output: &mut dyn io::Write) -> Result<(), CommandError> {
    Ok(writeln!(output, "Kernel support not configured!")?)
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
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    let x_str = args.next().ok_or(CommandError::BadArgs)?;
    let x = x_str.parse::<f32>()?;
    let y_str = args.next().ok_or(CommandError::BadArgs)?;
    let y = y_str.parse::<f32>()?;
    return Ok(writeln!(output, "{}", x + y)?);
}

/// Implements a command that outputs the ANSI "clear console" sequence.
fn clear_command(output: &mut dyn io::Write) -> Result<(), CommandError> {
    Ok(output.write_str("\x1b\x63")?)
}

fn bundles_command(output: &mut dyn io::Write) -> Result<(), CommandError> {
    match cantrip_proc_ctrl_get_running_bundles() {
        Ok(bundle_ids) => {
            writeln!(output, "{}", bundle_ids.join("\n"))?;
        }
        Err(status) => {
            writeln!(output, "get_running_bundles failed: {:?}", status)?;
        }
    }
    Ok(())
}

fn collect_from_zmodem(
    input: &mut dyn io::BufRead,
    mut output: &mut dyn io::Write,
) -> Option<ObjDescBundle> {
      writeln!(output, "Starting zmodem upload...").ok()?;
      let mut upload = rz::rz(input, &mut output).ok()?;
      upload.finish();
      writeln!(output, "Received {} bytes of data, crc32 {}",
               upload.len(),
               hex::encode(upload.crc32().to_be_bytes())).ok()?;
      Some(upload.frames().clone())
}

fn install_command(
    args: &mut dyn Iterator<Item = &str>,
    input: &mut dyn io::BufRead,
    mut output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    fn clear_slot(slot: seL4_CPtr) {
        unsafe {
            CANTRIP_CSPACE_SLOTS.free(slot, 1);
            seL4_CNode_Delete(SELF_CNODE, slot, seL4_WordBits as u8)
                .expect("install");
        }
    }

    // Collect/setup the package frames. If a -z arg is present a zmodem
    // upload is used; otherwise we use some raw pages (for testing).
    let mut pkg_contents = match args.next() {
        Some("-z") => {
            collect_from_zmodem(input, &mut output).ok_or(CommandError::IO)?
        }
        _ => {
            // TODO: pattern-fill pages
            cantrip_frame_alloc(8192).map_err(|_| CommandError::IO)?
        }
    };

    // The frames are in SELF_CNODE; wrap them in a dynamically allocated
    // CNode (as expected by cantrip_pgk_mgmt_install).
    // TODO(sleffler): useful idiom, add to MemoryManager
    let cnode_depth = pkg_contents.count_log2();
    let cnode = cantrip_cnode_alloc(cnode_depth)
                    .map_err(|_| CommandError::Memory)?;  // XXX leaks pkg_contents
    pkg_contents.move_objects_from_toplevel(cnode.objs[0].cptr, cnode_depth as u8)
                .map_err(|_| CommandError::Memory)?; // XXX leaks pkg_contents + cnode
    match cantrip_pkg_mgmt_install(&pkg_contents) {
        Ok(bundle_id) => {
            writeln!(output, "Bundle \"{}\" installed", bundle_id)?;
        }
        Err(status) => {
            writeln!(output, "install failed: {:?}", status)?;
        }
    }

    // SecurityCoordinator owns the cnode & frames contained within but we
    // still have a cap for the cnode in our top-level CNode; clean it up.
    debug_assert!(cnode.cnode == unsafe { SELF_CNODE });
    sel4_sys::debug_assert_slot_cnode!(cnode.objs[0].cptr);
    clear_slot(cnode.objs[0].cptr);

    Ok(())
}

fn uninstall_command(
    args: &mut dyn Iterator<Item = &str>,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    let bundle_id = args.next().ok_or(CommandError::BadArgs)?;
    match cantrip_pkg_mgmt_uninstall(bundle_id) {
        Ok(_) => {
            writeln!(output, "Bundle \"{}\" uninstalled.", bundle_id)?;
        }
        Err(status) => {
            writeln!(output, "uninstall failed: {:?}", status)?;
        }
    }
    Ok(())
}

fn start_command(
    args: &mut dyn Iterator<Item = &str>,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    let bundle_id = args.next().ok_or(CommandError::BadArgs)?;
    match cantrip_proc_ctrl_start(bundle_id) {
        Ok(_) => {
            writeln!(output, "Bundle \"{}\" started.", bundle_id)?;
        }
        Err(status) => {
            writeln!(output, "start failed: {:?}", status)?;
        }
    }
    Ok(())
}

fn stop_command(
    args: &mut dyn Iterator<Item = &str>,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    let bundle_id = args.next().ok_or(CommandError::BadArgs)?;
    match cantrip_proc_ctrl_stop(bundle_id) {
        Ok(_) => {
            writeln!(output, "Bundle \"{}\" stopped.", bundle_id)?;
        }
        Err(status) => {
            writeln!(output, "stop failed: {:?}", status)?;
        }
    }
    Ok(())
}

fn kvdelete_command(
    args: &mut dyn Iterator<Item = &str>,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    let key = args.next().ok_or(CommandError::BadArgs)?;
    match cantrip_storage_delete(key) {
        Ok(_) => {
            writeln!(output, "Delete key \"{}\".", key)?;
        }
        Err(status) => {
            writeln!(output, "Delete key \"{}\" failed: {:?}", key, status)?;
        }
    }
    Ok(())
}

fn kvread_command(
    args: &mut dyn Iterator<Item = &str>,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    let key = args.next().ok_or(CommandError::BadArgs)?;
    match cantrip_storage_read(key) {
        Ok(value) => {
            writeln!(output, "Read key \"{}\" = {:?}.", key, value)?;
        }
        Err(status) => {
            writeln!(output, "Read key \"{}\" failed: {:?}", key, status)?;
        }
    }
    Ok(())
}

fn kvwrite_command(
    args: &mut dyn Iterator<Item = &str>,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    let key = args.next().ok_or(CommandError::BadArgs)?;
    let value = args.collect::<Vec<&str>>().join(" ");
    match cantrip_storage_write(key, value.as_bytes()) {
        Ok(_) => {
            writeln!(output, "Write key \"{}\" = {:?}.", key, value)?;
        }
        Err(status) => {
            writeln!(output, "Write key \"{}\" failed: {:?}", key, status)?;
        }
    }
    Ok(())
}

fn malloc_command(
    args: &mut dyn Iterator<Item = &str>,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    let space_str = args.next().ok_or(CommandError::BadArgs)?;
    let space_bytes = space_str.parse::<usize>()?;
    match cantrip_frame_alloc(space_bytes) {
        Ok(frames) => {
            writeln!(output, "Allocated {:?}", frames)?;
        }
        Err(status) => {
            writeln!(output, "malloc failed: {:?}", status)?;
        }
    }
    Ok(())
}

fn mfree_command(
    args: &mut dyn Iterator<Item = &str>,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    extern "C" { static SELF_CNODE: seL4_CPtr; }
    let cptr_str = args.next().ok_or(CommandError::BadArgs)?;
    let count_str = args.next().ok_or(CommandError::BadArgs)?;
    let frames = ObjDescBundle::new(
        unsafe { SELF_CNODE },
        seL4_WordBits as u8,
        vec![
            ObjDesc::new(
                sel4_sys::seL4_RISCV_4K_Page,
                count_str.parse::<usize>()?,
                cptr_str.parse::<usize>()? as seL4_CPtr,
            ),
        ],
    );
    match cantrip_object_free_toplevel(&frames) {
        Ok(_) => {
            writeln!(output, "Free'd {:?}", frames)?;
        }
        Err(status) => {
            writeln!(output, "mfree failed: {:?}", status)?;
        }
    }
    Ok(())
}

fn mstats(output: &mut dyn io::Write, stats: &MemoryManagerStats)
          -> Result<(), CommandError>
{
    writeln!(output, "{} bytes in-use, {} bytes free, {} bytes requested, {} overhead",
             stats.allocated_bytes,
             stats.free_bytes,
             stats.total_requested_bytes,
             stats.overhead_bytes)?;
    writeln!(output, "{} objs in-use, {} objs requested",
             stats.allocated_objs,
             stats.total_requested_objs)?;
    Ok(())
}

fn mstats_command(
    _args: &mut dyn Iterator<Item = &str>,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    match cantrip_memory_stats() {
        Ok(stats) => { mstats(output, &stats)?; }
        Err(status) => { writeln!(output, "stats failed: {:?}", status)?; }
    }
    Ok(())
}

/// Implements a command that tests facilities that use the global allocator.
/// Shamelessly cribbed from https://os.phil-opp.com/heap-allocation/
fn test_alloc_command(output: &mut dyn io::Write) -> Result<(), CommandError> {
    extern crate alloc;
    use alloc::{boxed::Box, rc::Rc};

    // allocate a number on the heap
    let heap_value = Box::new(41);
    writeln!(output, "heap_value at {:p}", heap_value).expect("Box failed");

    // create a dynamically sized vector
    let mut vec = Vec::new();
    for i in 0..500 {
        vec.push(i);
    }
    writeln!(output, "vec at {:p}", vec.as_slice()).expect("Vec failed");

    // create a reference counted vector -> will be freed when count reaches 0
    let reference_counted = Rc::new(vec![1, 2, 3]);
    let cloned_reference = reference_counted.clone();
    writeln!(
        output,
        "current reference count is {}",
        Rc::strong_count(&cloned_reference)
    )
    .expect("Rc 1 failed");
    core::mem::drop(reference_counted);
    writeln!(
        output,
        "reference count is {} now",
        Rc::strong_count(&cloned_reference)
    )
    .expect("Rc 2 failed");

    Ok(writeln!(output, "All tests passed!")?)
}

/// Implements a command that tests the global allocator error handling.
fn test_alloc_error_command(output: &mut dyn io::Write) -> Result<(), CommandError> {
    // Default heap holds 16KB.
    let mut vec = Vec::with_capacity(16384);
    for i in 0..16348 {
        vec.push(i);
    }
    Ok(writeln!(output, "vec at {:p}", vec.as_slice())?)
}

fn test_bootinfo_command(output: &mut dyn io::Write) -> Result<(), CommandError> {
    use cantrip_os_common::sel4_sys::seL4_BootInfo;
    extern "C" {
        fn sel4runtime_bootinfo() -> *const seL4_BootInfo;
    }
    let bootinfo_ref = unsafe { &*sel4runtime_bootinfo() };
    writeln!(output, "{}:{} empty slots {}:{} untyped",
        bootinfo_ref.empty.start, bootinfo_ref.empty.end,
        bootinfo_ref.untyped.start, bootinfo_ref.untyped.end)?;

    // NB: seL4_DebugCapIdentify is only available in debug builds
    #[cfg(feature = "CONFIG_DEBUG_BUILD")]
    for ut in bootinfo_ref.untyped.start..bootinfo_ref.untyped.end {
        let cap_tag = unsafe { cantrip_os_common::sel4_sys::seL4_DebugCapIdentify(ut) };
        assert_eq!(cap_tag, 2,
            "expected untyped (2), got {} for cap at {}", cap_tag, ut);
    }
    Ok(())
}

/// Implements a command that tests panic handling.
fn test_panic_command() -> Result<(), CommandError> {
    panic!("testing");
}

/// Implements a command that runs an ML execution.
fn test_mlexecute_command() -> Result<(), CommandError> {
    extern "C" {
        fn mlcoord_execute();
    }
    unsafe {
        mlcoord_execute();
    }
    Ok(())
}

/// Implements a command that sets whether the ml execution is continuous.
fn test_mlcontinuous_command(args: &mut dyn Iterator<Item = &str>) -> Result<(), CommandError> {
    extern "C" {
        fn mlcoord_set_continuous_mode(mode: bool);
    }
    if let Some(mode_str) = args.nth(0) {
        let mode = mode_str.parse::<bool>()?;
        unsafe {
            mlcoord_set_continuous_mode(mode);
        }
        return Ok(());
    }
    Err(CommandError::BadArgs)
}

fn test_obj_alloc_command(output: &mut dyn io::Write) -> Result<(), CommandError> {
    let before_stats = cantrip_memory_stats().expect("before stats");
    mstats(output, &before_stats)?;

    fn check_alloc(output: &mut dyn io::Write,
                   name: &str,
                   res: Result<ObjDescBundle, MemoryManagerError>) {
        match res {
            Ok(obj) => {
                if let Err(e) = cantrip_object_free_toplevel(&obj) {
                    let _ = writeln!(output, "free {} {:?} failed: {:?}", name, obj, e);
                }
            }
            Err(e) => {
                let _ = writeln!(output, "alloc {} failed: {:?}", name, e);
            }
        }
    }

    // NB: alloc+free immediately so we don't run out of top-level CNode slots
    check_alloc(output, "untyped", cantrip_untyped_alloc(12));  // NB: 4KB
    check_alloc(output, "tcb", cantrip_tcb_alloc());
    check_alloc(output, "endpoint", cantrip_endpoint_alloc());
    check_alloc(output, "notification", cantrip_notification_alloc());
    check_alloc(output, "cnode", cantrip_cnode_alloc(5));  // NB: 32 slots
    check_alloc(output, "frame", cantrip_frame_alloc(4096));
//    check_alloc(output, "large frame",  cantrip_frame_alloc(1024*1024));
    check_alloc(output, "page table", cantrip_page_table_alloc());

    #[cfg(feature = "CONFIG_KERNEL_MCS")]
    check_alloc(output, "sched context",
                cantrip_sched_context_alloc(seL4_MinSchedContextBits));

    #[cfg(feature = "CONFIG_KERNEL_MCS")]
    check_alloc(output, "reply", cantrip_reply_alloc());

    let after_stats = cantrip_memory_stats().expect("after stats");
    mstats(output, &after_stats)?;
    assert_eq!(before_stats.allocated_bytes, after_stats.allocated_bytes);
    assert_eq!(before_stats.free_bytes, after_stats.free_bytes);

    // Batch allocate into a private CNode as we might to build a process.
    const CNODE_DEPTH: usize = 7; // 128 slots
    let cnode = cantrip_cnode_alloc(CNODE_DEPTH).unwrap(); // XXX handle error
    let objs = ObjDescBundle::new(
        cnode.objs[0].cptr,
        CNODE_DEPTH as u8,
        vec![
            ObjDesc::new(seL4_TCBObject, 1, 0),        // 1 tcb
            ObjDesc::new(seL4_EndpointObject, 2, 1),   // 2 endpoiints
            ObjDesc::new(seL4_ReplyObject, 2, 3),      // 2 replys
            ObjDesc::new(seL4_SchedContextObject,                   // 1 sched context
                         seL4_MinSchedContextBits, 5),
            ObjDesc::new(seL4_RISCV_4K_Page, 10, 6),   // 10 4K pages
        ],
    );
    match cantrip_object_alloc(&objs) {
        Ok(_) => {
            writeln!(output, "Batch alloc ok: {:?}", objs)?;
            if let Err(e) = cantrip_object_free(&objs) {
                writeln!(output, "Batch free err: {:?}", e)?;
            }
        }
        Err(e) => {
            writeln!(output, "Batch alloc err: {:?} {:?}", objs, e)?;
        }
    }
    if let Err(e) = cantrip_object_free_toplevel(&cnode) {
        writeln!(output, "Cnode free err: {:?} {:?}", cnode, e)?;
    }

    Ok(writeln!(output, "All tests passed!")?)
}

/// Implements a command that starts a timer, but does not wait on the
/// notification.
fn test_timer_async_command(
    args: &mut dyn Iterator<Item = &str>,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    let id_str = args.next().ok_or(CommandError::BadArgs)?;
    let id = id_str.parse::<u32>()?;
    let time_str = args.next().ok_or(CommandError::BadArgs)?;
    let time_ms = time_str.parse::<u32>()?;

    writeln!(output, "Starting timer {} for {} ms.", id, time_ms)?;

    match timer_service_oneshot(id, time_ms) {
        TimerServiceError::TimerOk => (),
        _ => return Err(CommandError::BadArgs),
    }

    timer_service_oneshot(id, time_ms);

    return Ok(());
}

/// Implements a command that starts a timer, blocking until the timer has
/// completed.
fn test_timer_blocking_command(
    args: &mut dyn Iterator<Item = &str>,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    let time_str = args.next().ok_or(CommandError::BadArgs)?;
    let time_ms = time_str.parse::<u32>()?;

    writeln!(output, "Blocking {} ms waiting for timer.", time_ms)?;

    // Set timer_id to 0, we don't need to use multiple timers here.
    match timer_service_oneshot(0, time_ms) {
        TimerServiceError::TimerOk => (),
        _ => return Err(CommandError::BadArgs),
    }

    timer_service_wait();

    return Ok(writeln!(output, "Timer completed.")?);
}

/// Implements a command that checks the completed timers.
fn test_timer_completed_command(
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    return Ok(writeln!(output, "Timers completed: {:#032b}", timer_service_completed_timers())?);
}
