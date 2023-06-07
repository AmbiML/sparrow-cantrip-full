// Copyright 2022 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Spill-to-stack Calling Convention.
//! The first REG_ARGS arguments are passed to threads using registers;
//! any more arguments are written to the stack.

use super::sel4_sys;
use super::CopyRegion;
use crate::sel4bundle::arch;
use crate::sel4bundle::seL4BundleImpl;
use core::mem::size_of;
use core::ptr;

use arch::REG_ARGS;
use arch::STACK_ALIGNMENT_BYTES;

use sel4_sys::seL4_Error;
use sel4_sys::seL4_Word;

use static_assertions::assert_cfg;
assert_cfg!(not(feature = "CONFIG_CAPDL_LOADER_CC_REGISTERS"));

extern "Rust" {
    fn get_load_application_mut() -> &'static mut [u8];
}

impl seL4BundleImpl {
    // Check TCB's argv and if needed write arguments to the stack and
    // fixup the stack pointer to match.
    pub fn maybe_spill_tcb_args(
        &self,
        osp: seL4_Word,
        argv: &[seL4_Word],
    ) -> Result<seL4_Word, seL4_Error> {
        let argc = argv.len();
        let reg_args = REG_ARGS;
        if argc <= reg_args {
            return Ok(osp); // Arguments fit in registers; nothing to do.
        }

        // More arguments than will fit in registers; map the TCB's stack
        // into our address space to write the spillover.

        assert_eq!(
            STACK_ALIGNMENT_BYTES % size_of::<seL4_Word>(),
            0,
            "Stack alignment wrong for argument size"
        );
        let mut sp = osp;

        // NB: the stack pointer will initially be aligned to
        // STACK_ALIGNMENT_BYTES. Any padding required to enforce this
        // alignment will come before any stack arguments.
        let num_stack_args = argc - reg_args; // positive because argc > reg_args
        let args_per_alignment = STACK_ALIGNMENT_BYTES / size_of::<seL4_Word>();
        let num_unaligned_args = num_stack_args % args_per_alignment;
        if num_unaligned_args != 0 {
            let num_padding_args = args_per_alignment - num_unaligned_args;
            let num_padding_bytes = num_padding_args * size_of::<seL4_Word>();
            sp -= num_padding_bytes;
        }

        // Find and map the frame representing the TCB's stack. Note that
        // we do `sp - sizeof(uintptr_t)` because the stack pointer may
        // be on a page boundary.
        let frame_obj = self.get_stack_frame_obj(sp - size_of::<seL4_Word>());

        let mut copy_region = unsafe { CopyRegion::new(get_load_application_mut()) };
        copy_region.map(frame_obj.cptr)?;

        // Write spillover arguments to the TCB's stack.
        for i in (reg_args..argc).rev() {
            sp -= size_of::<seL4_Word>();

            // We could support this case with more complicated logic, but
            // choose not to.
            assert!(
                (sp % copy_region.size()) != 0,
                "TCB {}'s initial arguments cause its stack to cross a page boundary",
                self.tcb_name
            );
            let byte_offset = sp % copy_region.size();
            unsafe {
                ptr::write(
                    &mut copy_region.as_word_mut()[byte_offset / size_of::<seL4_Word>()],
                    argv[i],
                )
            };
        }

        // NB: copy_region unmap'd on drop

        Ok(sp)
    }
}
