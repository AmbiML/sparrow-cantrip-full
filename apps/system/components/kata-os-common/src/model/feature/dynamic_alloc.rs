// Dynamic Object Allocation.

use capdl::*;
use capdl::CDL_FrameFill_BootInfoEnum_t::*;
use capdl::CDL_FrameFillType_t::*;
use capdl::CDL_ObjectType::*;
use crate::CantripOsModel;
use log::{debug, info, trace};
use smallvec::SmallVec;

use sel4_sys::seL4_ASIDControl_MakePool;
use sel4_sys::seL4_BootInfo;
use sel4_sys::seL4_CapASIDControl;
use sel4_sys::seL4_CapInitThreadCNode;
use sel4_sys::seL4_CapRights;
use sel4_sys::seL4_CNode_Copy;
use sel4_sys::seL4_CNode_Delete;
use sel4_sys::seL4_CNode_Move;
use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_Error;
use sel4_sys::seL4_ObjectType::*;
use sel4_sys::seL4_ObjectType;
use sel4_sys::seL4_PageBits;
use sel4_sys::seL4_Page_GetAddress;
use sel4_sys::seL4_Result;
use sel4_sys::seL4_UntypedDesc;
use sel4_sys::seL4_Untyped_Retype;
use sel4_sys::seL4_Word;
use sel4_sys::seL4_WordBits;

use crate::arch;

use static_assertions::assert_cfg;
assert_cfg!(not(feature = "CONFIG_CAPDL_LOADER_STATIC_ALLOC"));

fn BIT(bit_num: usize) -> usize { 1 << bit_num }

impl<'a> CantripOsModel<'a> {
    // Verify the untypeds in the model correspond to the contents of bootinfo.
    pub fn check_untypeds(&self) -> seL4_Result {
        assert_eq!(
            self.spec.num_untyped, 0,
            "capdl has static obj alloc but rootserver setup for dynamic"
        );
        Ok(())
    }

    // Create objects using a simple allocator. This relies on capDL-tool
    // grouping device objects by address and sorting non-device objects
    // by descending size to minimize fragmentation. See CapDL/PrintC.hs.
    pub fn create_objects(&mut self) -> seL4_Result {
        // Sort untypeds from largest to smallest.
        let num_normal_untypes = self.sort_untypeds(self.bootinfo);
        let mut ut_index = 0; // index into untypeds

        // Collect the roots in a local SmallVec so we can dedup entries
        // before we stash them in self.vpsace_roots. This minimizes the
        // possibility of vpsace_roots spilling to the heap.
        let mut roots = SmallVec::new();

        // Record objects that receive special treatment later on.
        let mut bootinfo_frame: CDL_ObjID = CDL_ObjID::MAX;
        let mut untyped_cnode: CDL_ObjID = CDL_ObjID::MAX;

        // First, allocate most objects and record the cslot locations.
        // The exception is ASIDPools, where create_object only allocates
        // the backing untypeds.
        let mut free_slot_index = 0;
        for (obj_id, obj) in self.spec.obj_slice().iter()
            .enumerate()
            .filter(|(_, obj)| arch::requires_creation(obj.r#type()))
        {
            let free_slot = self.free_slot_start + free_slot_index;

            //            trace!(
            //                "Creating object {} in slot {} from untyped {:#x}...",
            //                obj.name(),
            //                free_slot,
            //                self.state.get_untyped_cptr(ut_index)
            //            );

            // NB: create_object may use free_slot + 1 and free_slot + 2
            while let Err(e) =
                self.create_object(obj, obj_id, self.state.get_untyped_cptr(ut_index), free_slot)
            {
                debug!("create error {:?}", e);
                if e != seL4_Error::seL4_NotEnoughMemory {
                    panic!("Untyped retype failed, error {:?}", e);
                }
                // This untyped is exhausted, go to the next entry.
                ut_index += 1;
                if ut_index >= num_normal_untypes {
                    panic!("Out of untyped memory.");
                }
            }

            match obj.r#type() {
                // Capture VSpace roots & TCBs for later use.
                CDL_TCB => {
                    if let Some(root_cap) = obj.get_cap_at(CDL_TCB_VTable_Slot) {
                        roots.push(root_cap.obj_id);
                    }
                }
                // Capture one bootinfo frame for processing below.
                CDL_Frame => {
                    fn is_bootinfo_frame(obj: &CDL_Object) -> bool {
                        let frame_fill = &obj.frame_fill(0).unwrap();
                        frame_fill.type_ == CDL_FrameFill_BootInfo &&
                            frame_fill.get_bootinfo().type_ == CDL_FrameFill_BootInfo_BootInfo
                    }
                    if is_bootinfo_frame(obj) {
                        // NB: can instantiate multiple frames but only one
                        // CNode can receive the untypeds since we must move
                        // 'em from the rootserver (since they are "derived").
                        // XXX maybe just complain & ignore
                        trace!("Found bootinfo Frame at {}", obj_id);
                        assert!(!is_objid_valid(bootinfo_frame));
                        bootinfo_frame = obj_id;
                    }
                }
                // Look for a CNode associated with any bootinfo frame.
                CDL_CNode => {
                    if obj.cnode_has_untyped_memory() {
                        if is_objid_valid(untyped_cnode) {
                            info!("Duplicate bootinfo cnode at {}, prev {}", obj_id, untyped_cnode);
                        }
                        untyped_cnode = obj_id;
                    }
                }
                _ => {}
            }
            // Record the cslot assigned to the object.
            self.set_orig_cap(obj_id, free_slot);
            free_slot_index += 1;
        }

        // Now setup the backing untypeds for the ASID pools. This is
        // done in the order given by the ASID slot allocation policy.
        // This fixes the layout inside the kernel's ASID table, which
        // ensures consistency with verification models.
        for asid_high in 1..self.spec.num_asid_slots {
            let obj_id = self.get_asid(asid_high).unwrap();
            let asid_ut = self.get_orig_cap(obj_id);
            let asid_slot = self.free_slot_start + free_slot_index;

            trace!(
                "Create ASID pool {} asid_slot={} asid_ut={:#x}",
                self.get_object(obj_id).name(),
                asid_slot,
                asid_ut
            );

            unsafe {
                seL4_ASIDControl_MakePool(
                    seL4_CapASIDControl,
                    asid_ut,
                    seL4_CapInitThreadCNode,
                    asid_slot,
                    seL4_WordBits as u8,
                )
            }?;

            // update to point to our new ASID pool
            self.set_orig_cap(obj_id, asid_slot);
            free_slot_index += 1;
        }

        // Update the free slot to go past all the objects we just made.
        self.free_slot_start += free_slot_index;

        // Stash the VSpace roots.
        roots.sort();
        roots.dedup();
        self.vspace_roots = roots;

        // Record any CNode designated to receive the UntypedMemory caps when
        // constructing their CSpace.
        // NB: we conditionally assign based on there being a BootInfo frame
        //   because the UntypedMemory caps are not useful w/o the descriptors.
        if is_objid_valid(bootinfo_frame) {
            assert!(is_objid_valid(untyped_cnode));
            self.untyped_cnode = untyped_cnode;
        }

        Ok(())
    }

    // Sort untyped objects from largest to smallest.  This ensures that
    // fragmentation is minimized if the objects are also sorted, largest
    // to smallest, during creation.
    fn sort_untypeds(&mut self, bootinfo: &seL4_BootInfo) -> usize {
        let untyped_start = bootinfo.untyped.start;
        let untyped_end = bootinfo.untyped.end;
        let untypedList = unsafe { self.bootinfo.untyped_descs() };

        // Count how many non-device untypeds there are of each size.
        let mut count: [usize; seL4_WordBits] = [0; seL4_WordBits];
        for ut in untypedList {
            if !ut.is_device() {
                count[ut.size_bits()] += 1;
            }
        }

        // Calculate the starting index for each untyped.
        let mut total: seL4_Word = 0;
        for size in (1..seL4_WordBits).rev() {
            let oldCount = count[size];
            count[size] = total;
            total += oldCount;
        }

        // Store untypeds in untyped_cptrs array.
        let mut num_normal_untypes = 0usize;
        for untyped_index in 0..(untyped_end - untyped_start) {
            let ut = &untypedList[untyped_index];
            if !ut.is_device() {
                let index = ut.size_bits();

                //                trace!("Untyped {:3} (cptr={:#x}) (addr={:#x}) is of size {:2}. Placing in slot {}...",
                //                       untyped_index, untyped_start + untyped_index, ut.paddr, index, count[index]);

                self.state
                    .set_untyped_cptr(count[index], untyped_start + untyped_index);
                count[index] += 1;
                num_normal_untypes += 1;
            } else {
                //                trace!("Untyped {:3} (cptr={:#x}) (addr={:#x}) is of size {:2}. Skipping as it is device",
                //                       untyped_index, untyped_start + untyped_index, ut.paddr, ut.size_bits());
            }
        }
        num_normal_untypes
    }

    pub fn find_device_object(
        &self,
        free_slot: seL4_CPtr,
        _untyped_index: usize,
        sel4_type: seL4_ObjectType,
        obj_size_bits: usize,
        paddr: seL4_Word,
        obj_id: CDL_ObjID,
    ) -> seL4_Result {
        // See if an overlapping object was already created, can only do this
        // for frames. Any overlapping object will be the immediately preceding
        // one since objects are created in order of physical address.
        if sel4_type != seL4_UntypedObject && obj_id > 0 {
            // NB: if obj_id is -1 (invalid) then prev will also be
            //   out-of-range in get_object and an assert will trip.
            let prev = obj_id - 1;
            let prev_obj = self.get_object(prev);
            if prev_obj.r#type() == CDL_Frame
                && prev_obj.paddr() == Some(paddr)
                && prev_obj.size_bits() == obj_size_bits
            {
                debug!(
                    "Copy overlapping object {}'s cap {:#x}",
                    prev_obj.name(),
                    self.get_orig_cap(prev)
                );
                let seL4_AllRights = seL4_CapRights::new(
                    /*grant_reply=*/ 1, /*grant=*/ 1, /*read=*/ 1, /*write=*/ 1,
                );
                // Copy the overlapping object's capability.
                return unsafe {
                    seL4_CNode_Copy(
                        seL4_CapInitThreadCNode,
                        free_slot,
                        seL4_WordBits as u8,
                        seL4_CapInitThreadCNode,
                        self.get_orig_cap(prev),
                        seL4_WordBits as u8,
                        seL4_AllRights,
                    )
                };
            }
        }

        // Assume we are allocating from a device untyped; search the
        // untyped list for the entry where our object is located. Within
        // each region it gets harder. We have no visibility into what's
        // available so we issue repeated Retype requests to create temporary
        // Frame objects and fetch the frame's address. But to get the kernel
        // to Retype successive Frame's it's necessary to trick it by holding
        // a capability slot so the kernel doesn't just return the same slot
        // for successive Retype's. Note for all this to work the slots at
        // free_slot + 1 and free_slot + 2 must be available (true here since
        // create_objects asigns slots sequentially).
        let untypedList = unsafe { self.bootinfo.untyped_descs() };
        for i in 0..(self.bootinfo.untyped.end - self.bootinfo.untyped.start) {
            fn is_obj_inside_untyped(
                obj_addr: seL4_Word,
                obj_size_bits: usize,
                ut: &seL4_UntypedDesc,
            ) -> bool {
                ut.paddr <= obj_addr && obj_addr + obj_size_bits <= ut.paddr + BIT(ut.size_bits())
            }
            fn get_address(ut_slot: seL4_CPtr) -> Result<seL4_Page_GetAddress, seL4_Error> {
                // Create a temporary frame to get the address. We load this at slot + 2
                // to avoid the free_slot (where we want to write out object) and + 1 where
                // our "hold" cap is located (see above).
                let temp_slot = ut_slot + 2;
                unsafe {
                    seL4_Untyped_Retype(
                        ut_slot,
                        arch::get_frame_type(seL4_PageBits).into(),
                        seL4_PageBits,
                        seL4_CapInitThreadCNode,
                        0,
                        0,
                        temp_slot,
                        1,
                    )?;
                    let temp_addr = seL4_Page_GetAddress(temp_slot);
                    seL4_CNode_Delete(seL4_CapInitThreadCNode, temp_slot, seL4_WordBits as u8)?;
                    Ok(temp_addr)
                }
            }

            if is_obj_inside_untyped(paddr, BIT(obj_size_bits), &untypedList[i]) {
                // See above, loop looking for a Frame in the untyped object
                // that matches our object's address. If we run out of space
                // in the untyped the kernel will return seL4_NotEnoughMemory
                // that we pass back to the caller who then advances to the
                // next untyped region.
                let mut hold_slot: Option<seL4_CPtr> = None;
                loop {
                    unsafe {
                        seL4_Untyped_Retype(
                            self.bootinfo.untyped.start + i,
                            sel4_type.into(),
                            obj_size_bits,
                            seL4_CapInitThreadCNode,
                            0,
                            0,
                            free_slot,
                            1,
                        )
                    }?;
                    let addr: seL4_Page_GetAddress = if sel4_type == seL4_UntypedObject {
                        get_address(free_slot)?
                    } else {
                        unsafe { seL4_Page_GetAddress(free_slot) }
                    };
                    if addr.paddr == paddr {
                        // Found our object, delete any holding cap.
                        if let Some(hold_slot_cap) = hold_slot {
                            unsafe {
                                seL4_CNode_Delete(
                                    seL4_CapInitThreadCNode,
                                    hold_slot_cap,
                                    seL4_WordBits as u8,
                                )
                            }?;
                        }
                        return Ok(());
                    }

                    // Prepare to advance to the next Frame in the untyped
                    // region. If this is the first time doing this we create
                    // a holding slot (as described above); otherwise we
                    // delete the Retype'd Frame so the next trip around the
                    // loop will advance the location of the next Retype.
                    if hold_slot.is_none() {
                        hold_slot = Some(free_slot + 1);
                        unsafe {
                            seL4_CNode_Move(
                                seL4_CapInitThreadCNode,
                                hold_slot.unwrap(),
                                seL4_WordBits as u8,
                                seL4_CapInitThreadCNode,
                                free_slot,
                                seL4_WordBits as u8,
                            )
                        }?;
                    } else {
                        unsafe {
                            seL4_CNode_Delete(
                                seL4_CapInitThreadCNode,
                                free_slot,
                                seL4_WordBits as u8,
                            )
                        }?;
                    }
                }
            }
        }
        Ok(())
    }
}
