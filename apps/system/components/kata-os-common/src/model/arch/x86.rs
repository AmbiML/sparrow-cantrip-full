// Intel x86 32-bit target support.

use static_assertions::assert_cfg;
assert_cfg!(target_arch = "x86");

use log::{debug, error, info, trace, warn};
use sel4_sys::seL4_Result;

pub const PAGE_SIZE: usize = 4096;
pub const STACK_ALIGNMENT_BYTES: usize = 16;
// XXX really?
cfg_if! {
    if #[cfg(feature = "CONFIG_CAPDL_LOADER_CC_REGISTERS")] {
        pub const REG_ARGS: seL4_Word = 4;
    } else {
        pub const REG_ARGS: seL4_Word = 0;
    }
}

// Identifies IRQ objects that potentially need special processing.
pub fn is_irq(type_: CDL_ObjectType) -> bool {
    type_ == CDL_IOAPICInterrupt || type_ == CDL_MSIInterrupt || type_ == CDL_Interrupt
}

// Identifies objects that need to be instantiated.
pub fn requires_creation(type_: CDL_ObjectType) -> bool { !is_irq(type_) }

pub unsafe fn seL4_Page_Map_Flush(
    _sel4_page: seL4_Page,
    _page_type: seL4_ObjectType,
    _rights: seL4_CapRights,
    _vm_attribs: seL4_VMAttributes,
) -> seL4_Result { Ok(()) }

pub fn get_frame_type(object_size: seL4_Word) -> seL4_ObjectType {
    match object_size {
        seL4_PageBits => seL4_X86_4K,
        seL4_LargePageBits => seL4_X86_LargePageObject,
        _ => panic!("Unexpected frame size {}", object_size),
    }
}

pub fn create_irq_cap(irq: CDL_IRQ, obj: &CDL_Object, free_slot: seL4_CPtr) -> seL4_Error {
    let root = seL4_CapInitThreadCNode;
    let index = free_slot;
    let depth = seL4_WordBits as u8;
    match obj.r#type() {
        CDL_IOAPICInterrupt => {
            seL4_IRQControl_GetIOAPIC(
                seL4_CapIRQControl,
                root,
                index,
                depth,
                obj.ioapicirq_ioapic(),
                obj.ioapicirq_pin(),
                obj.ioapicirq_level(),
                obj.ioapicirq_polarity(),
                irq,
            )
        }
        CDL_MSIInterrupt => {
            seL4_IRQControl_GetMSI(
                seL4_CapIRQControl,
                root,
                index,
                depth,
                obj.msiirq_pci_bus(),
                obj.msiirq_pci_dev(),
                obj.msiirq_pci_fun(),
                obj.msiirq_handle(),
                irq,
            )
        }
        _ => seL4_IRQControl_Get(seL4_CapIRQControl, irq, root, index, depth),
    }
}

pub fn get_user_context(cdl_tcb: &CDL_Object, sp: seL4_Word) -> *const seL4_UserContext {
    #[rustfmt::skip]
    static mut regs: seL4_UserContext = seL4_UserContext {
        eip: 0, esp: 0, eflags: 0,
        eax: 0, ebx: 0, ecx: 0, edx: 0,
        esi: 0, edi: 0,
        ebp: 0, tls_base: 0, fs: 0, gs: 0,
    };

    assert_eq!(cdl_tcb.r#type(), CDL_TCB);

    unsafe {
        regs.eip = cdl_tcb.tcb_pc();
        regs.esp = sp; // NB: may be adjusted from cdl_tcb.tcb_sp()

        // XXX REG_ARGS == 0
        let argv = core::slice::from_raw_parts(cdl_tcb.tcb_init(), cdl_tcb.tcb_init_sz());
        regs.eax = if argv.len() > 0 { argv[0] } else { 0 };
        regs.ebx = if argv.len() > 1 { argv[1] } else { 0 };
        regs.ecx = if argv.len() > 2 { argv[2] } else { 0 };
        regs.edx = if argv.len() > 3 { argv[3] } else { 0 };

        //        trace!("Start {} with eip {:#x} esp {:#x} argv {:?}", cdl_tcb.name(),
        //               regs.eip, regs.esp, argv);

        &regs as *const seL4_UserContext
    }
}

impl<'a> CantripOsModel<'a> {
    pub fn create_arch_object(
        &mut self,
        obj: &CDL_Object,
        _id: CDL_ObjID,
        free_slot: usize,
    ) -> Option<seL4_Error> {
        if obj.r#type() == CDL_IOPorts {
            // XXX handle error
            seL4_X86_IOPortControl_Issue(
                seL4_CapIOPortControl,
                obj.other_start(),
                obj.other_end(),
                seL4_CapInitThreadCNode,
                free_slot,
                seL4_WordBits as u8,
            );
            Some(seL4_NoError)
        } else {
            None
        }
    }

    pub fn init_vspace(&mut self, obj_id: CDL_ObjID) -> seL4_Result {
        assert_eq!(self.get_object(obj_id).r#type(), CDL_PD);
        // XXX C code does all PD's before PT's, not sure if this works
        self.map_page_directory(obj_id)?;
        self.map_page_directory_page_tables(obj_id)?;
        Ok(())
    }

    fn map_page_directory(&self, pd_id: CDL_ObjID) {
        fn map_page_directory_slot(pd_id: CDL_ObjID, pd_slot: &CDL_CapSlot) {
            let page_cap = &pd_slot.cap;
            let page_vaddr = pd_slot.slot << (seL4_PageTableIndexBits + seL4_PageBits);
            self.map_page(page_cap, pd_id, page_cap.cap_rights(), page_vaddr);
        }

        for slot in self.spec.get_object(pd_id).slots_slice() {
            map_page_directory_slot(pd_id, &slot);
        }
    }

    fn map_page_directory_page_tables(&self, pd: CDL_ObjID) {
        fn map_page_table_slots(pd: CDL_ObjID, pd_slot: &CDL_CapSlot) {
            fn map_page_table_slot(
                pd: CDL_ObjID,
                pt: CDL_ObjID,
                pt_vaddr: seL4_Word,
                pt_slot: &CDL_CapSlot,
            ) {
                let page_cap = &pt_slot.cap;
                let page_vaddr = pt_vaddr + (pt_slot.slot << seL4_PageBits);
                self.map_page(page_cap, pd, page_cap.cap_rights(), page_vaddr);
            }

            let page_cap = &pd_slot.cap;
            if page_cap.r#type() == CDL_PTCap {
                let page = page_cap.obj_id;
                let page_vaddr = pd_slot.slot << (seL4_PageTableIndexBits + seL4_PageBits);
                for slot in self.spec.get_object(page).slots_slice() {
                    self.map_page_table_slot(pd, page, page_vaddr, &slot);
                }
            }
        }

        for slot in self.spec.get_object(pd).slots_slice() {
            self.map_page_table_slots(pd, &slot);
        }
    }
}
