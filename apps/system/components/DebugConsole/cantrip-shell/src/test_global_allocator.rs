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

//! Global allocator shell test commands

extern crate alloc;
use crate::CmdFn;
use crate::CommandError;
use crate::HashMap;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt::Write;

use cantrip_io as io;

pub fn add_cmds(cmds: &mut HashMap<&str, CmdFn>) {
    cmds.extend([
        ("test_alloc", alloc_command as CmdFn),
        ("test_alloc_error", alloc_error_command as CmdFn),
    ]);
}

/// Implements a command that tests facilities that use the global allocator.
/// Shamelessly cribbed from https://os.phil-opp.com/heap-allocation/
fn alloc_command(
    _args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    extern crate alloc;
    use alloc::{boxed::Box, rc::Rc};

    // allocate a number on the heap
    let heap_value = Box::new(41);
    writeln!(output, "heap_value at {:p}", heap_value).expect("Box failed");

    // create a dynamically sized vector
    let mut vec = Vec::new();
    for i in 0..500 {
        vec.push(i);
    }
    writeln!(output, "vec at {:p}", vec.as_slice()).expect("Vec failed");

    // create a reference counted vector -> will be freed when count reaches 0
    let reference_counted = Rc::new(vec![1, 2, 3]);
    let cloned_reference = reference_counted.clone();
    writeln!(
        output,
        "current reference count is {}",
        Rc::strong_count(&cloned_reference)
    )
    .expect("Rc 1 failed");
    core::mem::drop(reference_counted);
    writeln!(
        output,
        "reference count is {} now",
        Rc::strong_count(&cloned_reference)
    )
    .expect("Rc 2 failed");

    Ok(writeln!(output, "All tests passed!")?)
}

/// Implements a command that tests the global allocator error handling.
fn alloc_error_command(
    _args: &mut dyn Iterator<Item = &str>,
    _input: &mut dyn io::BufRead,
    output: &mut dyn io::Write,
) -> Result<(), CommandError> {
    // Default heap holds 16KB.
    let mut vec = Vec::with_capacity(16384);
    for i in 0..16384 {
        vec.push(i);
    }
    Ok(writeln!(output, "vec at {:p}", vec.as_slice())?)
}
