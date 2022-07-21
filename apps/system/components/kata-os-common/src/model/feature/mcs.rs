// MCS Kernel Support.

use crate::CantripOsModel;
use capdl::CDL_ObjectType::*;
use capdl::*;
use log::debug;

use sel4_sys::seL4_CNode;
use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_CapInitThreadTCB;
use sel4_sys::seL4_Error;
use sel4_sys::seL4_Result;
use sel4_sys::seL4_SchedContext;
use sel4_sys::seL4_SchedContext_NoFlag;
use sel4_sys::seL4_SchedControl;
use sel4_sys::seL4_SchedControl_ConfigureFlags;
use sel4_sys::seL4_TCB_Configure;
use sel4_sys::seL4_TCB_SetSchedParams;
use sel4_sys::seL4_TCB_SetTimeoutEndpoint;
use sel4_sys::seL4_Time;
use sel4_sys::seL4_Word;

use static_assertions::assert_cfg;
assert_cfg!(feature = "CONFIG_KERNEL_MCS");

impl<'a> CantripOsModel<'a> {
    pub fn init_sched_ctrl(&mut self) -> seL4_Result {
        for index in 0..(self.bootinfo.schedcontrol.end - self.bootinfo.schedcontrol.start) {
            self.set_sched_ctrl_cap(index, self.bootinfo.schedcontrol.start + index);
        }
        Ok(())
    }

    pub fn init_scs(&self) -> seL4_Result {
        let affinity: CDL_Core = 0;
        for obj_id in 0..self.spec.num {
            let cdl_sc = self.get_object(obj_id);
            if cdl_sc.r#type() == CDL_SchedContext {
                /* all scs get configured on core 0, any scs that should be bound to a tcb will
                be reconfigured for the correct core in init_tcbs */

                SchedControl_Configure(
                    /*sched_ctrl=*/ self.get_sched_ctrl_cap(affinity),
                    /*sel4_sc=*/ self.get_orig_cap(obj_id),
                    /*affinity=*/ 0,
                    cdl_sc.sc_budget(),
                    cdl_sc.sc_period(),
                    cdl_sc.sc_data(),
                )?;
            }
        }
        Ok(())
    }

    pub fn init_fault_ep(
        &mut self,
        cdl_tcb: &CDL_Object,
    ) -> Result<(seL4_CPtr, seL4_CPtr), seL4_Error> {
        // NB: fault ep cptrs are in the caller's cspace.

        let sel4_tempfault_ep: seL4_CPtr =
            if let Some(cap_tempfault_ep) = cdl_tcb.get_cap_at(CDL_TCB_TemporalFaultEP_Slot) {
                self.get_orig_cap(cap_tempfault_ep.obj_id)
            } else {
                debug!("TCB {} has no temporal fault endpoint", cdl_tcb.name());
                0
            };

        let sel4_fault_ep: seL4_CPtr =
            if let Some(cap_fault_ep) = cdl_tcb.get_cap_at(CDL_TCB_FaultEP_Slot) {
                let fault_ep_obj = cap_fault_ep.obj_id;
                let fault_ep_badge = cap_fault_ep.cap_data();
                if fault_ep_badge != 0 {
                    self.mint_cap(fault_ep_obj, fault_ep_badge)?
                } else {
                    self.get_orig_cap(fault_ep_obj)
                }
            } else {
                debug!("TCB {} has no fault endpoint", cdl_tcb.name());
                0
            };
        Ok((sel4_fault_ep, sel4_tempfault_ep))
    }
}

// TODO(sleffler): match syscall types
pub fn SchedControl_Configure(
    sched_ctrl: seL4_SchedControl,
    sel4_sc: seL4_SchedContext,
    _affinity: seL4_Word,
    sc_budget: seL4_Time,
    sc_period: seL4_Time,
    sc_data: seL4_Word,
) -> seL4_Result {
    assert!(sel4_sc != 0);
    unsafe {
        seL4_SchedControl_ConfigureFlags(
            sched_ctrl,
            sel4_sc,
            sc_budget,
            sc_period,
            /*extra_refills=*/ 0,
            /*badge=*/ sc_data,
            /*flags=*/ seL4_SchedContext_NoFlag,
        )
    }
}

pub fn TCB_Configure(
    sel4_tcb: seL4_Word,
    _sel4_fault_ep: seL4_CPtr,
    sel4_cspace_root: seL4_CNode,
    sel4_cspace_root_data: seL4_Word,
    sel4_vspace_root: seL4_CPtr,
    sel4_vspace_root_data: seL4_Word,
    ipcbuffer_addr: seL4_Word,
    sel4_ipcbuffer: seL4_CPtr,
) -> seL4_Result {
    unsafe {
        seL4_TCB_Configure(
            sel4_tcb,
            sel4_cspace_root,
            sel4_cspace_root_data,
            sel4_vspace_root,
            sel4_vspace_root_data,
            ipcbuffer_addr,
            sel4_ipcbuffer,
        )
    }
}

pub fn TCB_SchedParams(
    sel4_tcb: seL4_Word,
    max_priority: seL4_Word,
    priority: seL4_Word,
    sel4_sc: seL4_Word,
    sel4_fault_ep: seL4_CPtr,
) -> seL4_Result {
    unsafe {
        seL4_TCB_SetSchedParams(
            sel4_tcb,
            seL4_CapInitThreadTCB,
            max_priority,
            priority,
            sel4_sc,
            sel4_fault_ep,
        )
    }
}

pub fn TCB_SetTimeoutEndpoint(sel4_tcb: seL4_Word, sel4_tempfault_ep: seL4_CPtr) -> seL4_Result {
    unsafe { seL4_TCB_SetTimeoutEndpoint(sel4_tcb, sel4_tempfault_ep) }
}
