// MemoryManager service shell test commands

extern crate alloc;
use crate::mstats;
use crate::CmdFn;
use crate::CommandError;
use crate::HashMap;
use alloc::vec;
use core::fmt::Write;

use cantrip_io as io;
use cantrip_memory_interface::*;
use cantrip_os_common::sel4_sys;

use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_MinSchedContextBits;
use sel4_sys::seL4_ObjectType::*;
use sel4_sys::seL4_WordBits;

pub fn add_cmds(cmds: &mut HashMap<&str, CmdFn>) {
    cmds.extend([
        ("test_bootinfo", bootinfo_command as CmdFn),
        ("test_malloc", malloc_command as CmdFn),
        ("test_mfree", mfree_command as CmdFn),
        ("test_obj_alloc", obj_alloc_command as CmdFn),
    ]);
}

fn bootinfo_command(
    _args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    use cantrip_os_common::sel4_sys::seL4_BootInfo;
    extern "C" {
        fn sel4runtime_bootinfo() -> *const seL4_BootInfo;
    }
    let bootinfo_ref = unsafe { &*sel4runtime_bootinfo() };
    writeln!(
        output,
        "{}:{} empty slots {}:{} untyped",
        bootinfo_ref.empty.start,
        bootinfo_ref.empty.end,
        bootinfo_ref.untyped.start,
        bootinfo_ref.untyped.end
    )?;

    // NB: seL4_DebugCapIdentify is only available in debug builds
    #[cfg(feature = "CONFIG_DEBUG_BUILD")]
    for ut in bootinfo_ref.untyped.start..bootinfo_ref.untyped.end {
        let cap_tag = unsafe { cantrip_os_common::sel4_sys::seL4_DebugCapIdentify(ut) };
        assert_eq!(
            cap_tag, 2,
            "expected untyped (2), got {} for cap at {}",
            cap_tag, ut
        );
    }
    Ok(())
}

fn malloc_command(
    args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
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
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    extern "C" {
        static SELF_CNODE: seL4_CPtr;
    }
    let cptr_str = args.next().ok_or(CommandError::BadArgs)?;
    let count_str = args.next().ok_or(CommandError::BadArgs)?;
    let frames = ObjDescBundle::new(
        unsafe { SELF_CNODE },
        seL4_WordBits as u8,
        vec![ObjDesc::new(
            sel4_sys::seL4_RISCV_4K_Page,
            count_str.parse::<usize>()?,
            cptr_str.parse::<usize>()? as seL4_CPtr,
        )],
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

fn obj_alloc_command(
    _args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
    _builtin_cpio: &[u8],
) -> Result<(), CommandError> {
    let before_stats = cantrip_memory_stats().expect("before stats");
    mstats(output, &before_stats)?;

    fn check_alloc(
        output: &mut dyn io::Write,
        name: &str,
        res: Result<ObjDescBundle, MemoryManagerError>,
    ) {
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
    check_alloc(output, "untyped", cantrip_untyped_alloc(12)); // NB: 4KB
    check_alloc(output, "tcb", cantrip_tcb_alloc());
    check_alloc(output, "endpoint", cantrip_endpoint_alloc());
    check_alloc(output, "notification", cantrip_notification_alloc());
    check_alloc(output, "cnode", cantrip_cnode_alloc(5)); // NB: 32 slots
    check_alloc(output, "frame", cantrip_frame_alloc(4096));
    //    check_alloc(output, "large frame",  cantrip_frame_alloc(1024*1024));
    check_alloc(output, "page table", cantrip_page_table_alloc());

    #[cfg(feature = "CONFIG_KERNEL_MCS")]
    check_alloc(
        output,
        "sched context",
        cantrip_sched_context_alloc(seL4_MinSchedContextBits),
    );

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
            ObjDesc::new(seL4_TCBObject, 1, 0),      // 1 tcb
            ObjDesc::new(seL4_EndpointObject, 2, 1), // 2 endpoiints
            ObjDesc::new(seL4_ReplyObject, 2, 3),    // 2 replys
            ObjDesc::new(
                seL4_SchedContextObject, // 1 sched context
                seL4_MinSchedContextBits,
                5,
            ),
            ObjDesc::new(seL4_RISCV_4K_Page, 10, 6), // 10 4K pages
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

    // Batch allocate using the newer api that constructs a CNode based
    // on the batch of objects specified.
    match cantrip_object_alloc_in_cnode(vec![
        ObjDesc::new(seL4_TCBObject, 1, 0),      // 1 tcb
        ObjDesc::new(seL4_EndpointObject, 1, 1), // 1 endpoiints
        ObjDesc::new(seL4_ReplyObject, 1, 2),    // 1 replys
        ObjDesc::new(
            seL4_SchedContextObject, // 1 sched context
            seL4_MinSchedContextBits,
            3,
        ),
        ObjDesc::new(seL4_RISCV_4K_Page, 2, 4), // 2 4K pages
    ]) {
        Ok(objs) => {
            writeln!(output, "cantrip_object_alloc_in_cnode ok: {:?}", objs)?;
            if let Err(e) = cantrip_object_free_in_cnode(&objs) {
                writeln!(output, "cantrip_object_free_in_cnode failed: {:?}", e)?;
            }
        }
        Err(e) => {
            writeln!(output, "cantrip_object_alloc_in_cnode failed: {:?}", e)?;
        }
    }

    Ok(writeln!(output, "All tests passed!")?)
}
