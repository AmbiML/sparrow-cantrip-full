// Static Object Allocation.
// Expect a statically-allocated capDL spec.

use crate::arch;
use crate::CantripOsModel;
use capdl::CDL_ObjectType::*;
use capdl::*;
use log::debug;
use smallvec::SmallVec;

use sel4_sys::seL4_ASIDControl_MakePool;
use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_CapASIDControl;
use sel4_sys::seL4_CapInitThreadCNode;
use sel4_sys::seL4_ObjectType;
use sel4_sys::seL4_Result;
use sel4_sys::seL4_Untyped_Retype;
use sel4_sys::seL4_Word;
use sel4_sys::seL4_WordBits;

use static_assertions::assert_cfg;
assert_cfg!(feature = "CONFIG_CAPDL_LOADER_STATIC_ALLOC");

impl<'a> CantripOsModel<'a> {
    fn get_untyped(&self, untyped_num: usize) -> Option<&'a CDL_UntypedDerivation> {
        if untyped_num >= self.spec.num_untyped {
            return None;
        }
        Some(unsafe {
            &core::slice::from_raw_parts(self.spec.untyped, self.spec.num_untyped)[untyped_num]
        })
    }

    // Verify the untypeds in the model correspond to the contents of bootinfo.
    pub fn check_untypeds(&mut self) -> seL4_Result {
        let untypedList = unsafe { self.bootinfo.untyped_descs() };

        let mut bi_start = 0;
        for u in 0..self.spec.num_untyped {
            let mut found = false;
            let num_untyped = self.bootinfo.untyped.end - self.bootinfo.untyped.start;
            let ut = self.get_object(self.get_untyped(u).unwrap().untyped);
            assert_eq!(ut.r#type(), CDL_Untyped);

            // TODO(sleffler): doesn't seem correct, why not stop at first?
            //    + handling of bi_start seems awkward
            for i in bi_start..num_untyped {
                let bt_ut = &untypedList[i];
                if bt_ut.paddr == ut.paddr().unwrap() {
                    assert_eq!(
                        bt_ut.size_bits() as usize,
                        ut.size_bits(),
                        "Untyped size mismatch, bootinfo {}, capdl {}",
                        bt_ut.size_bits(),
                        ut.size_bits()
                    );
                    self.state
                        .set_untyped_cptr(u, self.bootinfo.untyped.start + i);
                    found = true;
                    if i == bi_start {
                        bi_start += 1;
                    }
                    // XXX no break?
                }
            }
            assert!(found, "Failed to find bootinfo to match untyped {:?}", ut.paddr().unwrap());
        }
        Ok(())
    }

    /*
     * Spec was statically allocated; just run its untyped derivation steps.
     */
    pub fn create_objects(&mut self) -> seL4_Result {
        debug!("Creating objects...");

        let mut free_slot_index = 0;

        // Collect the roots in a local SmallVec so we can dedup entries
        // before we stash them in self.vpsace_roots. This minimizes the
        // possibility of vpsace_roots spilling to the heap.
        let mut roots = SmallVec::new();

        /* First, allocate most objects and update the spec database with
        the cslot locations. The exception is ASIDPools, where
        create_object only allocates the backing untypeds. */
        for ut_index in 0..self.spec.num_untyped {
            let ud = self.get_untyped(ut_index).unwrap();
            for child in 0..ud.num {
                let obj_id = ud.get_child(child).unwrap();
                let free_slot = self.free_slot_start + free_slot_index;
                let obj = self.get_object(obj_id);
                let capdl_obj_type = obj.r#type();
                let untyped_cptr = self.state.get_untyped_cptr(ut_index);

                assert!(
                    !arch::requires_creation(capdl_obj_type),
                    "Object {} requires dynamic allocation",
                    obj.name()
                );

                debug!(
                    "Creating object {} in slot {}, from untyped {:x}...",
                    obj.name(),
                    free_slot,
                    untyped_cptr
                );

                self.create_object(obj, obj_id, untyped_cptr, free_slot)?;

                // Capture VSpace roots for later use.
                if obj.r#type() == CDL_TCB {
                    if let Some(root_cap) = obj.get_cap_at(CDL_TCB_VTable_Slot) {
                        roots.push(root_cap.obj_id);
                    }
                }
                self.set_orig_cap(obj_id, free_slot);
                free_slot_index += 1;
            }
        }

        // XXX common with dynamic
        /* Now, we turn the backing untypeds into ASID pools, in the order
        given by the ASID slot allocation policy. This fixes the layout
        inside the kernel's ASID table, which ensures consistency with
        verification models. */
        debug!("Creating ASID pools...");
        for asid_high in 1..self.spec.num_asid_slots {
            let obj_id = self.get_asid(asid_high).unwrap();
            let asid_ut = self.get_orig_cap(obj_id);
            let asid_slot = self.free_slot_start + free_slot_index;

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

        Ok(())
    }

    pub fn find_device_object(
        &self,
        free_slot: seL4_CPtr,
        untyped_slot: seL4_CPtr,
        sel4_type: seL4_ObjectType,
        obj_size: usize,
        _paddr: seL4_Word,
        _obj_id: CDL_ObjID,
    ) -> seL4_Result {
        unsafe {
            seL4_Untyped_Retype(
                untyped_slot,
                sel4_type.into(),
                obj_size,
                seL4_CapInitThreadCNode,
                /*node_index=*/ 0,
                /*node_depth=*/ 0,
                /*node_offset=*/ free_slot,
                /*no_objects=*/ 1,
            )
        }
    }
}
