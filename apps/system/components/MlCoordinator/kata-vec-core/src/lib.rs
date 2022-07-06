#![no_std]

// cantrip-vec-core is the vector core driver. It is responsible for providing
// convenient methods for interacting with the hardware.

mod vc_top;

use core::mem::size_of;
use core::slice;
use cantrip_memory_interface::ObjDescBundle;
use cantrip_ml_shared::{ModelSections, Window, WMMU_PAGE_SIZE};
use cantrip_ml_shared::{TCM_SIZE, TCM_PADDR};
use cantrip_proc_interface::BundleImage;

use io::Read;
use cantrip_io as io;

extern "C" {
    static TCM: *mut u32;
}

fn round_up(a: usize, b: usize) -> usize {
    if (a % b) == 0 {
        a
    } else {
        usize::checked_add(a, b).unwrap() - (a % b)
    }
}

pub fn enable_interrupts(enable: bool) {
    let intr_enable = vc_top::IntrEnable::new()
        .with_host_req(enable)
        .with_finish(enable)
        .with_instruction_fault(enable)
        .with_data_fault(enable);
    vc_top::set_intr_enable(intr_enable);
}

pub fn set_wmmu(sections: &ModelSections) {
    // XXX: Support multiple sections.
    // The length of the window is not the size of the window, but rather
    // the last address of the window. This saves us a bit in hardware:
    // 0x400000 is 23 bits vs. 0x3FFFFF 22 bits.
    vc_top::set_mmu_window_offset(0, sections.tcm.addr);
    vc_top::set_mmu_window_length(0, sections.tcm.size - 1);
    vc_top::set_mmu_window_permission(0, vc_top::Permission::ReadWriteExecute);
}

pub fn run() {
    let ctrl = vc_top::Ctrl::new()
        .with_freeze(false)
        .with_vc_reset(false)
        .with_pc_start(0);
    vc_top::set_ctrl(ctrl);
}

// Loads the model into the TCM.
pub fn load_image(frames: &ObjDescBundle) -> Result<ModelSections, &'static str> {
    let mut image = BundleImage::new(frames);
    let mut tcm_found = false;
    // Size of window is filled in below.
    let mut window = Window {
        addr: TCM_PADDR,
        size: 0,
    };

    clear_tcm();
    // NB: we require a TCM section and that only one is present
    while let Some(section) = image.next_section() {
        let slice = if section.vaddr == TCM_PADDR {
            if tcm_found {
                return Err("dup TCM section");
            }
            tcm_found = true;

            if section.fsize > TCM_SIZE {
                return Err("TCM section too big");
            }
            window.size = round_up(section.msize, WMMU_PAGE_SIZE);
            unsafe { slice::from_raw_parts_mut(TCM as *mut u8, TCM_SIZE) }
        } else {
            return Err("Unexpected section");
        };
        image
            .read_exact(&mut slice[section.data_range()])
            .map_err(|_| "section read error")?;
        // TODO(jesionowski): Remove when clear_tcm is fully implemented.
        slice[section.zero_range()].fill(0x00);
    }
    if !tcm_found {
        return Err("Incomplete");
    }
    Ok(ModelSections {
        tcm: window,
    })
}

// Interrupts are write 1 to clear.
pub fn clear_host_req() {
    let mut intr_state = vc_top::get_intr_state();
    intr_state.set_host_req(true);
    vc_top::set_intr_state(intr_state);
}

pub fn clear_finish() {
    let mut intr_state = vc_top::get_intr_state();
    intr_state.set_finish(true);
    vc_top::set_intr_state(intr_state);
}

pub fn clear_instruction_fault() {
    let mut intr_state = vc_top::get_intr_state();
    intr_state.set_instruction_fault(true);
    vc_top::set_intr_state(intr_state);
}

pub fn clear_data_fault() {
    let mut intr_state = vc_top::get_intr_state();
    intr_state.set_data_fault(true);
    vc_top::set_intr_state(intr_state);
}

// TODO(jesionowski): Remove dead_code when TCM_SIZE fits into INIT_END.
#[allow(dead_code)]
fn clear_section(start: u32, end: u32) {
    let init_start = vc_top::InitStart::new()
        .with_address(start);
    vc_top::set_init_start(init_start);

    let init_end = vc_top::InitEnd::new().with_address(end).with_valid(true);
    vc_top::set_init_end(init_end);

    while !vc_top::get_init_status().init_done() {}
}

pub fn clear_tcm() {
    // TODO(jesionowski): Enable when TCM_SIZE fits into INIT_END.
    // clear_section(0, TCM_SIZE as u32, false);
}

// TODO(jesionowski): Remove these when error handling is refactored.
// The status will be faulty iff the interrupt line is raised, and
// we won't have the fault registers on Springbok.
fn get_tcm_slice() -> &'static mut [u32] {
    unsafe { slice::from_raw_parts_mut(TCM, TCM_SIZE / size_of::<u32>()) }
}

pub fn get_return_code() -> u32 {
    const RC_OFFSET: usize = 0x3FFFEE;
    get_tcm_slice()[RC_OFFSET]
}

pub fn get_fault_register() -> u32 {
    const FAULT_OFFSET: usize = 0x3FFFEF;
    get_tcm_slice()[FAULT_OFFSET]
}
