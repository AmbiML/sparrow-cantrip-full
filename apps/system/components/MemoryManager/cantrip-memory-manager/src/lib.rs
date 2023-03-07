//! Cantrip OS global memory management support

#![cfg_attr(not(test), no_std)]

use cantrip_memory_interface::MemoryError;
use cantrip_memory_interface::MemoryLifetime;
use cantrip_memory_interface::MemoryManagerInterface;
use cantrip_memory_interface::MemoryManagerStats;
use cantrip_memory_interface::ObjDescBundle;
use cantrip_os_common::sel4_sys;
use core::ops::Range;
use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_UntypedDesc;
use spin::Mutex;
use spin::MutexGuard;

mod memory_manager;
pub use memory_manager::MemoryManager;

// CantripMemoryManager bundles an instance of the MemoryManager that operates
// on CantripOS interfaces and synchronizes public use with a Mutex. There is
// a two-step dance to setup an instance because we want CANTRIP_MEMORY static
// and MemoryManager is incapable of supplying a const fn due it's use of
// hashbrown::HashMap.
pub struct CantripMemoryManager {
    manager: Mutex<Option<MemoryManager>>,
}
impl CantripMemoryManager {
    // Constructs a partially-initialized instance; to complete call init().
    pub const fn empty() -> CantripMemoryManager {
        CantripMemoryManager {
            manager: Mutex::new(None),
        }
    }

    pub fn get(&self) -> Guard {
        Guard {
            manager: self.manager.lock(),
        }
    }
}
pub struct Guard<'a> {
    manager: MutexGuard<'a, Option<MemoryManager>>,
}
impl Guard<'_> {
    pub fn is_empty(&self) -> bool { self.manager.is_none() }

    // Finishes the setup started by empty():
    pub fn init(&mut self, ut_slots: Range<seL4_CPtr>, untypeds: &[seL4_UntypedDesc]) {
        assert!(self.manager.is_none());
        *self.manager = Some(MemoryManager::new(ut_slots, untypeds));
    }
}
impl MemoryManagerInterface for Guard<'_> {
    fn alloc(&mut self, objs: &ObjDescBundle, lifetime: MemoryLifetime) -> Result<(), MemoryError> {
        self.manager.as_mut().unwrap().alloc(objs, lifetime)
    }
    fn free(&mut self, objs: &ObjDescBundle) -> Result<(), MemoryError> {
        self.manager.as_mut().unwrap().free(objs)
    }
    fn stats(&self) -> Result<MemoryManagerStats, MemoryError> {
        self.manager.as_ref().unwrap().stats()
    }
    fn debug(&self) -> Result<(), MemoryError> { self.manager.as_ref().unwrap().debug() }
}
