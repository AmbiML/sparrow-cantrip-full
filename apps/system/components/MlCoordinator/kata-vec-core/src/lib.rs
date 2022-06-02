#![no_std]

// cantrip-vec-core is the vector core driver. It is responsible for providing
// convenient methods for interacting with the hardware.

mod vc_top;

use core::mem::size_of;
use core::slice;
use cantrip_memory_interface::ObjDescBundle;
use cantrip_ml_interface::{MlCoreInterface, ModelSections, Window};
use cantrip_proc_interface::BundleImage;

use io::Read;
use cantrip_io as io;

// The page size of the WMMU.
const WMMU_PAGE_SIZE: usize = 0x1000;

// TODO(jesionowski): Move these constants to an auto-generated file.
// TODO(b/214092253): ITCM size blow-up needs to be addressed.
const ITCM_SIZE: usize = 0x100000;
const ITCM_PADDR: usize = 0x32000000;
const DTCM_SIZE: usize = 0x1000000;
const DTCM_PADDR: usize = 0x34000000;

// TODO(jesionowski): ITCM / DTCM will eventually be merged into a single memory.
extern "C" {
    static itcm: *mut u32;
    static dtcm: *mut u32;
}

fn round_up(a: usize, b: usize) -> usize {
    if (a % b) == 0 {
        a
    } else {
        usize::checked_add(a, b).unwrap() - (a % b)
    }
}

fn get_dtcm_slice() -> &'static mut [u32] {
    unsafe { slice::from_raw_parts_mut(dtcm, DTCM_SIZE / size_of::<u32>()) }
}

pub struct MlCore {}

fn clear_section(start: u32, end: u32, is_itcm: bool) {
    let init_start = vc_top::InitStart::new()
        .with_address(start)
        .with_imem_dmem_sel(is_itcm);
    vc_top::set_init_start(init_start);

    let init_end = vc_top::InitEnd::new().with_address(end).with_valid(true);
    vc_top::set_init_end(init_end);

    while !vc_top::get_init_status().init_done() {}
}

fn clear_tcm() {
    clear_section(0, ITCM_SIZE as u32, true);
    // TODO(jesionowski): Enable when DTCM_SIZE fits into INIT_END.
    // clear_section(0, DTCM_SIZE as u32, false);
}

impl MlCoreInterface for MlCore {
    fn set_wmmu(&mut self, sections: &ModelSections) {
        // The length of the window is not the size of the window, but rather
        // the last address of the window. This saves us a bit in hardware:
        // 0x400000 is 23 bits vs. 0x3FFFFF 22 bits.
        vc_top::set_immu_window_offset(0, sections.instructions.addr);
        vc_top::set_immu_window_length(0, sections.instructions.size - 1);
        vc_top::set_immu_window_permission(0, vc_top::Permission::Read);

        vc_top::set_dmmu_window_offset(0, sections.data.addr);
        vc_top::set_dmmu_window_length(0, sections.data.size - 1);
        vc_top::set_dmmu_window_permission(0, vc_top::Permission::ReadAndWrite);
    }

    fn enable_interrupts(&mut self, enable: bool) {
        let intr_enable = vc_top::IntrEnable::new()
            .with_host_req(enable)
            .with_finish(enable)
            .with_instruction_fault(enable)
            .with_data_fault(enable);
        vc_top::set_intr_enable(intr_enable);
    }

    fn run(&mut self) {
        let ctrl = vc_top::Ctrl::new()
            .with_freeze(false)
            .with_vc_reset(false)
            .with_pc_start(0);
        vc_top::set_ctrl(ctrl);
    }

    // Loads the model into the TCM.
    fn load_image(&mut self, frames: &ObjDescBundle) -> Result<ModelSections, &'static str> {
        let mut image = BundleImage::new(frames);
        let mut itcm_found = false;
        let mut dtcm_found = false;
        // Size of windows is filled in below.
        let mut iwindow = Window {
            addr: ITCM_PADDR,
            size: 0,
        };
        let mut dwindow = Window {
            addr: DTCM_PADDR,
            size: 0,
        };

        clear_tcm();
        // NB: we require both ITCM & DTCM sections and that only one
        //   instance of each is present
        while let Some(section) = image.next_section() {
            let slice = if section.vaddr == ITCM_PADDR {
                if itcm_found {
                    return Err("dup ITCM");
                }
                itcm_found = true;

                if section.fsize > ITCM_SIZE {
                    return Err("ITCM too big");
                }
                iwindow.size = round_up(section.msize, WMMU_PAGE_SIZE);
                unsafe { slice::from_raw_parts_mut(itcm as *mut u8, ITCM_SIZE) }
            } else if section.vaddr == DTCM_PADDR {
                if dtcm_found {
                    return Err("dup DTCM");
                }
                dtcm_found = true;

                if section.fsize > DTCM_SIZE {
                    return Err("DTCM section too big");
                }
                dwindow.size = round_up(section.msize, WMMU_PAGE_SIZE);
                unsafe { slice::from_raw_parts_mut(dtcm as *mut u8, DTCM_SIZE) }
            } else {
                return Err("Unexpected section");
            };
            image
                .read_exact(&mut slice[section.data_range()])
                .map_err(|_| "section read error")?;
            // TODO(jesionowski): Remove when clear_tcm is fully implemented.
            slice[section.zero_range()].fill(0x00);
        }
        if !itcm_found || !dtcm_found {
            return Err("Incomplete");
        }
        Ok(ModelSections {
            instructions: iwindow,
            data: dwindow,
        })
    }

    // TODO(jesionowski): Read these from CSRs when available.
    fn get_return_code() -> u32 {
        const RC_OFFSET: usize = 0x3FFFEE;
        get_dtcm_slice()[RC_OFFSET]
    }

    fn get_fault_register() -> u32 {
        const FAULT_OFFSET: usize = 0x3FFFEF;
        get_dtcm_slice()[FAULT_OFFSET]
    }

    // Interrupts are write 1 to clear.
    fn clear_host_req() {
        let mut intr_state = vc_top::get_intr_state();
        intr_state.set_host_req(true);
        vc_top::set_intr_state(intr_state);
    }

    fn clear_finish() {
        let mut intr_state = vc_top::get_intr_state();
        intr_state.set_finish(true);
        vc_top::set_intr_state(intr_state);
    }

    fn clear_instruction_fault() {
        let mut intr_state = vc_top::get_intr_state();
        intr_state.set_instruction_fault(true);
        vc_top::set_intr_state(intr_state);
    }

    fn clear_data_fault() {
        let mut intr_state = vc_top::get_intr_state();
        intr_state.set_data_fault(true);
        vc_top::set_intr_state(intr_state);
    }
}
