//! Cantrip OS global memory management support

extern crate alloc;
use cantrip_memory_interface::MemoryError;
use cantrip_memory_interface::MemoryLifetime;
use cantrip_memory_interface::MemoryManagerInterface;
use cantrip_memory_interface::MemoryManagerStats;
use cantrip_memory_interface::ObjDesc;
use cantrip_memory_interface::ObjDescBundle;
use cantrip_os_common::camkes::{seL4_CPath, Camkes};
use cantrip_os_common::sel4_sys;
use core::ops::Range;
use log::{debug, error, info, trace, warn};
use smallvec::SmallVec;

use sel4_sys::seL4_CNode_Delete;
use sel4_sys::seL4_CNode_Revoke;
use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_Error;
use sel4_sys::seL4_Result;
use sel4_sys::seL4_UntypedDesc;
use sel4_sys::seL4_Untyped_Describe;
use sel4_sys::seL4_Untyped_Retype;
use sel4_sys::seL4_Word;

fn delete_path(path: &seL4_CPath) -> seL4_Result {
    unsafe { seL4_CNode_Delete(path.0, path.1, path.2 as u8) }
}
fn revoke_cap(cptr: seL4_CPtr) -> seL4_Result {
    let path = Camkes::top_level_path(cptr);
    unsafe { seL4_CNode_Revoke(path.0, path.1, path.2 as u8) }
}
fn untyped_describe(cptr: seL4_CPtr) -> seL4_Untyped_Describe {
    unsafe { seL4_Untyped_Describe(cptr) }
}

// SmallVec capacity for untyped memory slabs. There are two instances;
// one for anonymous memory and one for device-backed memory. The memory
// manager is expected to be setup as a static global so these data
// structures will land in .bss and only overflow to the heap if
// initialized with more than this count.
const UNTYPED_SLAB_CAPACITY: usize = 64; // # slabs kept inline
const STATIC_UNTYPED_SLAB_CAPACITY: usize = 4; // # slabs kept inline

// The MemoryManager supports allocating & freeing seL4 objects that are
// instantiated from UntypedMemory "slabs". Allocation causes untyped memory
// to be converted to concrete types. Freeing deletes the specified capabilities
// and updates the bookkeeping. Note that a free only releases the specified
// cap; if there are dups or derived objects the memory will not be returned
// to the untyped slab from which it was allocated and the bookkeeping done
// here will be out of sync with the kernel.
// TODO(sleffler): support device-backed memory objects
#[derive(Debug)]
struct UntypedSlab {
    pub _size_bits: usize,      // NB: only used to sort
    pub free_bytes: usize,      // Available space in slab
    pub _base_paddr: seL4_Word, // Physical address of slab start
    pub _last_paddr: seL4_Word, // Physical address of slab end
    pub cptr: seL4_CPtr,        // seL4 untyped object
}
impl UntypedSlab {
    fn new(ut: &seL4_UntypedDesc, free_bytes: usize, cptr: seL4_CPtr) -> Self {
        UntypedSlab {
            _size_bits: ut.size_bits(),
            free_bytes,
            _base_paddr: ut.paddr,
            _last_paddr: ut.paddr + l2tob(ut.size_bits()),
            cptr,
        }
    }
}
pub struct MemoryManager {
    untypeds: SmallVec<[UntypedSlab; UNTYPED_SLAB_CAPACITY]>,
    static_untypeds: SmallVec<[UntypedSlab; STATIC_UNTYPED_SLAB_CAPACITY]>,
    _device_untypeds: SmallVec<[UntypedSlab; UNTYPED_SLAB_CAPACITY]>,
    cur_untyped: usize,
    cur_static_untyped: usize,
    _cur_device_untyped: usize,

    total_bytes: usize,     // Total available space
    allocated_bytes: usize, // Amount of space currently allocated
    requested_bytes: usize, // Amount of space allocated over all time
    overhead_bytes: usize,

    allocated_objs: usize, // # seL4 objects currently allocated
    requested_objs: usize, // # seL4 objects allocated over all time

    // Retype requests failed due to insufficient available memory.
    untyped_slab_too_small: usize,

    // Alloc requests failed due to lack of untyped memory (NB: may be
    // due to fragmentation of untyped slabs).
    out_of_memory: usize,
}

fn _howmany(value: usize, unit: usize) -> usize { value + (unit - 1) / unit }
fn _round_up(value: usize, align: usize) -> usize { _howmany(value, align) * align }

// Log2 bits to bytes.
fn l2tob(size_bits: usize) -> usize { 1 << size_bits }

impl MemoryManager {
    // Creates a new MemoryManager instance. The allocator is seeded
    // from the untyped memory descriptors.
    pub fn new(slots: Range<seL4_CPtr>, untypeds: &[seL4_UntypedDesc]) -> Self {
        assert!(!untypeds.is_empty());
        assert_eq!(slots.end - slots.start, untypeds.len());
        let mut m = MemoryManager {
            untypeds: SmallVec::new(),
            static_untypeds: SmallVec::new(),
            _device_untypeds: SmallVec::new(),
            cur_untyped: 0,
            cur_static_untyped: 0,
            _cur_device_untyped: 0,

            total_bytes: 0,
            allocated_bytes: 0,
            requested_bytes: 0,
            overhead_bytes: 0,

            allocated_objs: 0,
            requested_objs: 0,

            untyped_slab_too_small: 0,
            out_of_memory: 0,
        };
        for (ut_index, ut) in untypeds.iter().enumerate() {
            #[cfg(feature = "CONFIG_NOISY_UNTYPEDS")]
            log::info!("slot {} {:?}", slots.start + ut_index, ut);
            let slab_size = l2tob(ut.size_bits());
            if ut.is_device() {
                m._device_untypeds
                    .push(UntypedSlab::new(ut, slab_size, slots.start + ut_index));
            } else {
                if ut.is_tainted() {
                    // Slabs marked "tainted" were used by the rootserver
                    // which has terminated. Reclaim the resources with a
                    // revoke.
                    revoke_cap(slots.start + ut_index).expect("revoke untyped");
                }
                // NB: must get the current state of the slab as the value
                //   supplied by the rootserver (in |untypeds|) will reflect
                //   resources available before the above revoke.
                let info = untyped_describe(slots.start + ut_index);
                assert_eq!(info.sizeBits, ut.size_bits());

                // We only have the remainder available for allocations.
                // Beware that slabs with existing allocations (for the
                // services constructed by the rootserver) are not generally
                // useful because we cannot recycle memory once retype'd.
                // We use those to satisfy "static object" alloc's (e.g.
                // as done by SDKRuntime); think of these requests as an
                // extension of the work done by the rootserver.
                // TODO(sleffler): split to minimize wasted space
                if info.remainingBytes > 0 {
                    let slab = UntypedSlab::new(ut, info.remainingBytes, slots.start + ut_index);
                    if info.remainingBytes == slab_size {
                        m.untypeds.push(slab);
                    } else {
                        m.static_untypeds.push(slab);
                    }
                    m.total_bytes += info.remainingBytes;
                } else {
                    trace!(
                        "Discard slot {}, size {}, no usable space",
                        slots.start + ut_index,
                        ut.size_bits()
                    );
                }

                // Use overhead to track memory allocated out of our control.
                m.overhead_bytes += slab_size - info.remainingBytes;
            }
        }
        // Sort non-device slabs by descending amount of free space.
        m.untypeds
            .sort_unstable_by(|a, b| b.free_bytes.cmp(&a.free_bytes));
        m.static_untypeds
            .sort_unstable_by(|a, b| b.free_bytes.cmp(&a.free_bytes));
        if m.static_untypeds.is_empty() {
            // No untyped memory available for static object requests;
            // seed the pool with the smallest "normal" slab.
            // TODO(sleffler): maybe split slab if "too big" (better to
            //   dynamically grow static_untypeds on demand?)
            m.static_untypeds.push(m.untypeds.pop().unwrap());
        }
        m
    }

    // Total available space.
    pub fn total_available_space(&self) -> usize { self.total_bytes }
    // Current allocated space
    pub fn allocated_space(&self) -> usize { self.allocated_bytes }
    // Current free space.
    pub fn free_space(&self) -> usize { self.total_bytes - self.allocated_bytes }
    // Total space allocated over time
    pub fn total_requested_space(&self) -> usize { self.requested_bytes }
    // Current allocated space out of our control.
    pub fn overhead_space(&self) -> usize { self.overhead_bytes }

    // Current allocated objects
    pub fn allocated_objs(&self) -> usize { self.allocated_objs }
    // Total objects allocated over time
    pub fn total_requested_objs(&self) -> usize { self.requested_objs }

    pub fn untyped_slab_too_small(&self) -> usize { self.untyped_slab_too_small }
    pub fn out_of_memory(&self) -> usize { self.out_of_memory }

    fn retype_untyped(free_untyped: seL4_CPtr, root: seL4_CPtr, obj: &ObjDesc) -> seL4_Result {
        unsafe {
            seL4_Untyped_Retype(
                free_untyped,
                /*type=*/ obj.type_.into(),
                /*size_bits=*/ obj.retype_size_bits().unwrap(),
                /*root=*/ root,
                /*node_index=*/ 0, // Ignored 'cuz depth is zero
                /*node_depth=*/ 0, // NB: store in cnode
                /*node_offset=*/ obj.cptr,
                /*num_objects=*/ obj.retype_count(),
            )
        }
    }

    fn delete_caps(root: seL4_CPtr, depth: u8, od: &ObjDesc) -> seL4_Result {
        for offset in 0..od.retype_count() {
            let path = (root, od.cptr + offset, depth as usize);
            if let Err(e) = delete_path(&path) {
                warn!("DELETE {:?} failed: od {:?} error {:?}", &path, od, e);
            }
        }
        Ok(())
    }

    fn alloc_static(&mut self, bundle: &ObjDescBundle) -> Result<(), MemoryError> {
        let first_ut = self.cur_static_untyped;
        let mut ut_index = first_ut;

        for od in &bundle.objs {
            // NB: we don't check slots are available (the kernel will tell us).
            while let Err(e) =
                Self::retype_untyped(self.static_untypeds[ut_index].cptr, bundle.cnode, od)
            {
                if e != seL4_Error::seL4_NotEnoughMemory {
                    // Should not happen.
                    panic!("static allocation failed: {:?}", e);
                }
                // This untyped does not have enough available space, try
                // the next slab until we exhaust all slabs. This is the best
                // we can do without per-slab bookkeeping.
                ut_index = (ut_index + 1) % self.static_untypeds.len();
                if ut_index == first_ut {
                    // TODO(sleffler): maybe steal memory from normal pool?
                    panic!("static allocation failed: out of space");
                }
            }
        }
        self.cur_static_untyped = ut_index;

        Ok(())
    }
}

impl MemoryManagerInterface for MemoryManager {
    fn alloc(
        &mut self,
        bundle: &ObjDescBundle,
        lifetime: MemoryLifetime,
    ) -> Result<(), MemoryError> {
        trace!("alloc {:?} {:?}", bundle, lifetime);

        if lifetime == MemoryLifetime::Static {
            // Static allocations are handle separately.
            return self.alloc_static(bundle);
        }

        // TODO(sleffler): split by device vs no-device (or allow mixing)
        let first_ut = self.cur_untyped;
        let mut ut_index = first_ut;

        let mut allocated_bytes: usize = 0;
        let mut allocated_objs: usize = 0;

        for od in &bundle.objs {
            // NB: we don't check slots are available (the kernel will tell us).
            // TODO(sleffler): maybe check size_bytes() against untyped slab?
            //    (we depend on the kernel for now)
            while let Err(e) =
                // NB: we don't allocate ASIDPool objects but if we did it
                //   would fail because it needs to map to an UntypedObject
                Self::retype_untyped(self.untypeds[ut_index].cptr, bundle.cnode, od)
            {
                if e != seL4_Error::seL4_NotEnoughMemory {
                    // Should not happen.
                    // TODO(sleffler): reclaim allocations
                    error!("Allocation request failed (retype returned {:?})", e);
                    return Err(MemoryError::UnknownMemoryError);
                }
                // This untyped does not have enough available space, try
                // the next slab until we exhaust all slabs. This is the best
                // we can do without per-slab bookkeeping.
                self.untyped_slab_too_small += 1;
                ut_index = (ut_index + 1) % self.untypeds.len();
                trace!("Advance to untyped slab {}", ut_index);
                // XXX { self.cur_untyped = ut_index; let _ = self.debug(); }
                if ut_index == first_ut {
                    // TODO(sleffler): reclaim allocations
                    self.out_of_memory += 1;
                    debug!("Allocation request failed (out of space)");
                    return Err(MemoryError::AllocFailed);
                }
            }
            allocated_objs += od.retype_count();
            allocated_bytes += od.size_bytes().unwrap();
        }
        self.cur_untyped = ut_index;

        self.allocated_bytes += allocated_bytes;
        self.allocated_objs += allocated_objs;

        // NB: does not include requests that fail
        self.requested_objs += allocated_objs;
        self.requested_bytes += allocated_bytes;

        Ok(())
    }
    fn free(&mut self, bundle: &ObjDescBundle) -> Result<(), MemoryError> {
        trace!("free {:?}", bundle);

        for od in &bundle.objs {
            // TODO(sleffler): support leaving objects so client can do bulk
            //   reclaim on exit (maybe require cptr != 0)
            if Self::delete_caps(bundle.cnode, bundle.depth, od).is_ok() {
                // NB: atm we do not do per-untyped bookkeeping so just track
                //   global stats.
                // TODO(sleffler): temp workaround for bad bookkeeping / client mis-handling
                let size_bytes = od.size_bytes().ok_or(MemoryError::ObjTypeInvalid)?;
                if size_bytes <= self.allocated_bytes {
                    self.allocated_bytes -= size_bytes;
                    self.allocated_objs -= od.retype_count();
                } else {
                    debug!("Underflow on free of {:?}", od);
                }
            }
        }
        Ok(())
    }
    fn stats(&self) -> Result<MemoryManagerStats, MemoryError> {
        Ok(MemoryManagerStats {
            allocated_bytes: self.allocated_space(),
            free_bytes: self.free_space(),
            total_requested_bytes: self.total_requested_space(),
            overhead_bytes: self.overhead_space(),

            allocated_objs: self.allocated_objs(),
            total_requested_objs: self.total_requested_objs(),

            untyped_slab_too_small: self.untyped_slab_too_small(),
            out_of_memory: self.out_of_memory(),
        })
    }
    fn debug(&self) -> Result<(), MemoryError> {
        // TODO(sleffler): only shows !device slabs
        let cur_cptr = self.untypeds[self.cur_untyped].cptr;
        for ut in &self.untypeds {
            let info = untyped_describe(ut.cptr);
            let size = l2tob(info.sizeBits);
            info!(target: if ut.cptr == cur_cptr { "*" } else { " " },
                "[{:2}, bits {:2}] watermark {:8} available {}",
                ut.cptr,
                info.sizeBits,
                size - info.remainingBytes,
                info.remainingBytes,
            );
        }
        if !self.static_untypeds.is_empty() {
            let cur_static_cptr = self.static_untypeds[self.cur_static_untyped].cptr;
            for ut in &self.static_untypeds {
                let info = untyped_describe(ut.cptr);
                let size = l2tob(info.sizeBits);
                info!(target: if ut.cptr == cur_static_cptr { "!" } else { " " },
                    "[{:2}, bits {:2}] watermark {:8} available {}",
                    ut.cptr,
                    info.sizeBits,
                    size - info.remainingBytes,
                    info.remainingBytes,
                );
            }
        }
        Ok(())
    }
}
