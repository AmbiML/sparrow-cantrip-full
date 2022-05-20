// No MCS Kernel Support.

use super::sel4_sys;

use sel4_sys::seL4_CNode;
use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_Error;
use sel4_sys::seL4_Result;
use sel4_sys::seL4_SchedContext;
use sel4_sys::seL4_SchedControl;
use sel4_sys::seL4_TCB;
use sel4_sys::seL4_TCB_Configure;
use sel4_sys::seL4_TCB_SetSchedParams;
use sel4_sys::seL4_Time;
use sel4_sys::seL4_Word;

use static_assertions::assert_cfg;
assert_cfg!(not(feature = "CONFIG_KERNEL_MCS"));

// TODO(sleffler): match syscall types
pub fn SchedControl_Configure(
    _sched_ctrl: seL4_SchedControl,
    _sel4_sc: seL4_SchedContext,
    _affinity: seL4_Word,
    _sc_budget: seL4_Time,
    _sc_period: seL4_Time,
    _sc_data: seL4_Word,
) -> seL4_Result {
    Ok(())
}

pub fn TCB_Configure(
    sel4_tcb: seL4_Word,
    sel4_fault_ep: seL4_CPtr,
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
            sel4_fault_ep,
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
    mcp: seL4_Word,
    priority: seL4_Word,
    _sel4_sc: seL4_Word,
    _sel4_fault_ep: seL4_CPtr,
) -> seL4_Result {
    unsafe {
        seL4_TCB_SetSchedParams(
            sel4_tcb,
            seL4_authority as seL4_TCB,
            mcp,
            priority,
        )
    }
}

pub fn TCB_SetTimeoutEndpoint(_sel4_tcb: seL4_Word, _sel4_tempfault_ep: seL4_CPtr) -> seL4_Result {
    Ok(())
}
