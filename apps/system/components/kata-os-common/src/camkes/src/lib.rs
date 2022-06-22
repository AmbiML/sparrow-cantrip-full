//! Cantrip OS CAmkES component helpers

#![no_std]
#![allow(non_camel_case_types)]

use allocator;
use logger::CantripLogger;
use slot_allocator::CANTRIP_CSPACE_SLOTS;
use log::trace;
use sel4_sys;

use sel4_sys::seL4_CNode_Delete;
use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_GetCapReceivePath;
use sel4_sys::seL4_SetCap;
use sel4_sys::seL4_SetCapReceivePath;
use sel4_sys::seL4_Word;
use sel4_sys::seL4_WordBits;

pub type seL4_CPath = (seL4_CPtr, seL4_CPtr, seL4_Word);

extern "C" {
    // CAmkES components marked with
    //    attribute integer cantripos = 1;
    // automatically get a self-reference to their top-level CNode and
    // the slot #'s of the first & last free slots in the CNode.
    static SELF_CNODE: seL4_CPtr;
    static SELF_CNODE_FIRST_SLOT: seL4_CPtr;
    static SELF_CNODE_LAST_SLOT: seL4_CPtr;
}

// RAII wrapper for handling request cap cleanup.
pub struct RequestCapCleanup {}
impl Drop for RequestCapCleanup {
    fn drop(&mut self) {
        unsafe { seL4_SetCap(0, 0); }
    }
}

pub struct Camkes {
    name: &'static str, // Component name
    recv_path: seL4_CPath, // IPCBuffer receive path
}

impl Camkes {
    pub const fn new(name: &'static str) -> Self {
        Camkes {
            name,
            recv_path: (seL4_CPtr::MAX, seL4_CPtr::MAX, seL4_Word::MAX),
        }
    }

    pub fn init_logger(self: &Camkes, level: log::LevelFilter) {
        static CANTRIP_LOGGER: CantripLogger = CantripLogger;
        log::set_logger(&CANTRIP_LOGGER).unwrap();
        log::set_max_level(level);
    }

    pub fn init_allocator(self: &Camkes, heap: &'static mut [u8]) {
        unsafe {
            allocator::ALLOCATOR.init(heap.as_mut_ptr() as usize, heap.len());
        }
        trace!("setup heap: start_addr {:p} size {}", heap.as_ptr(), heap.len());
    }

    pub fn init_slot_allocator(self: &Camkes, first_slot: seL4_CPtr, last_slot: seL4_CPtr) {
        unsafe {
            CANTRIP_CSPACE_SLOTS.init(self.name, first_slot, last_slot - first_slot);
            trace!("setup cspace slots: first slot {} free {}",
                   CANTRIP_CSPACE_SLOTS.base_slot(),
                   CANTRIP_CSPACE_SLOTS.free_slots());
        }
    }

    pub fn pre_init(
        self: &Camkes,
        level: log::LevelFilter,
        heap: &'static mut [u8],
    ) {
        self.init_logger(level);
        self.init_allocator(heap);
        unsafe { self.init_slot_allocator(SELF_CNODE_FIRST_SLOT, SELF_CNODE_LAST_SLOT); }
    }

    pub fn top_level_path(slot: seL4_CPtr) -> seL4_CPath {
        unsafe { (SELF_CNODE, slot, seL4_WordBits) }
    }

    // Initializes the IPCBuffer receive path with |path|.
    pub fn init_recv_path(self: &mut Camkes, path: &seL4_CPath) {
        self.recv_path = *path;
        unsafe { seL4_SetCapReceivePath(path.0, path.1, path.2); }
        trace!("{}: Cap receive path {:?}", self.name, path);
    }

    // Returns the path specified with init_recv_path.
    pub fn get_recv_path(self: &Camkes) -> seL4_CPath { self.recv_path }

    // Returns the component name.
    pub fn get_name(self: &Camkes) -> &'static str { self.name }

    // Returns the current receive path from the IPCBuffer.
    pub fn get_current_recv_path(self: &Camkes) -> seL4_CPath {
        unsafe { seL4_GetCapReceivePath() }
    }

    // Clears any capability the receive path path points to.
    pub fn clear_recv_path(self: &Camkes) {
        let path = &self.recv_path;
        // Assert since future receives are likely to fail
        unsafe { seL4_CNode_Delete(path.0, path.1, path.2 as u8) }.expect(self.name);
    }

    // Check the current receive path in the IPCBuffer against what was
    // setup with init_recv_path.
    pub fn check_recv_path(self: &Camkes) -> bool {
        self.get_current_recv_path() == self.get_recv_path()
    }

    // Like check_recv_path but asserts if there is an inconsistency.
    pub fn assert_recv_path(self: &Camkes) {
        assert!(self.check_recv_path(),
                "Current receive path {:?} does not match init'd path {:?}",
                self.get_current_recv_path(), self.recv_path);
    }

    // Attaches a capability to a CAmkES RPC request msg. seL4 will copy
    // the capabiltiy.
    #[must_use]
    pub fn set_request_cap(cptr: seL4_CPtr) -> RequestCapCleanup {
        unsafe { seL4_SetCap(0, cptr); }
        RequestCapCleanup{}
    }

    // Wrappers for sel4_sys::debug_assert macros.
    pub fn debug_assert_slot_empty(tag: &str, path: &seL4_CPath) {
        sel4_sys::debug_assert_slot_empty!(path.1,
            "{}: expected slot {:?} empty but has cap type {:?}",
            tag, path, sel4_sys::cap_identify(path.1));
    }
    pub fn debug_assert_slot_cnode(tag: &str, path: &seL4_CPath) {
        sel4_sys::debug_assert_slot_cnode!(path.1,
            "{}: expected cnode in slot {:?} but found cap type {:?}",
            tag, path, sel4_sys::cap_identify(path.1));
    }
}
