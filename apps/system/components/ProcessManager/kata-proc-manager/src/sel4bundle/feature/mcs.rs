// MCS Kernel Support.

use super::sel4_sys;

use sel4_sys::seL4_CNode;
use sel4_sys::seL4_CPtr;
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
            /*flags=*/ seL4_SchedContext_NoFlag)
    }
}

#[allow(clippy::too_many_arguments)]
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
    sel4_authority: seL4_CPtr,
    max_priority: seL4_Word,
    priority: seL4_Word,
    sel4_sc: seL4_Word,
    sel4_fault_ep: seL4_CPtr,
) -> seL4_Result {
    unsafe {
        seL4_TCB_SetSchedParams(
            sel4_tcb,
            sel4_authority,
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
