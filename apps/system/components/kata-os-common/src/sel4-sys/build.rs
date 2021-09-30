/* Copyright (c) 2015 The Robigalia Project Developers
 * Licensed under the Apache License, Version 2.0
 * <LICENSE-APACHE or
 * http://www.apache.org/licenses/LICENSE-2.0> or the MIT
 * license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
 * at your option. All files in the project carrying such
 * notice may not be copied, modified, or distributed except
 * according to those terms.
 */

use std::env;
use std::fs::File;
use std::os::unix::prelude::*;
use std::process::{Command, Stdio};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let cargo_target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let cargo_target_pointer_width = env::var("CARGO_CFG_TARGET_POINTER_WIDTH").unwrap();
    println!("target_arch = {} target_pointer_width = {}",
             cargo_target_arch, cargo_target_pointer_width);

    // Default to python3 (maybe necessary for code divergence)
    let python_bin = env::var("PYTHON").unwrap_or_else(|_| "python3".to_string());

    // Default to "seL4" for backwards compat; can either use git submodule or
    // symbolic link (neither recommended)
    let sel4_dir = env::var("SEL4_DIR").unwrap_or_else(|_| "seL4".to_string());

    // Use CARGO_TARGET_ARCH and CARGO_TARGET_POINTER_WIDTH
    // to select the target architecture;
    // NB: this mimics the logic in lib.rs
    let (arch, archdir) = match cargo_target_arch.as_str() {
        "x86" =>  ("ia32", "x86"),
        "x86_64" =>  ("x86_64", "x86"),
        "arm" => match cargo_target_pointer_width.as_str() {
            "32" => ("aarch32", "arm"),
            "64" => ("aarch64", "arm"),
            _ => { panic!("Unsupported arm word size {}", cargo_target_pointer_width); }
         }
        "riscv32" => ("riscv32", "riscv"),
        "riscv64" => ("riscv64", "riscv"),
        _ => { panic!("Unsupported architecture {}", cargo_target_arch); }
    };

    let xml_interfaces_file = format!("{}/libsel4/include/interfaces/sel4.xml", sel4_dir);
    let outfile = format!("{}/{}_syscall_stub.rs", out_dir, arch);
    let xml_arch_file = &*format!(
        "{}/libsel4/arch_include/{}/interfaces/sel4arch.xml",
        sel4_dir, archdir
    );
    let xml_sel4_arch_file = format!(
        "{}/libsel4/sel4_arch_include/{}/interfaces/sel4arch.xml",
        sel4_dir, arch
    );
    let args = vec![
        "tools/syscall_stub_gen.py",
        "-a",
        arch,
        "-w",
        cargo_target_pointer_width.as_str(),
        "--buffer",
        #[cfg(feature = "CONFIG_KERNEL_MCS")]
        "--mcs",
        "-o",
        &*outfile,
        &*xml_interfaces_file,
        &*xml_arch_file,
        &*xml_sel4_arch_file,
    ];

    let mut cmd = Command::new("/usr/bin/env");
    cmd.arg(&python_bin).args(&args);

    println!("Running: {:?}", cmd);
    assert!(cmd.status().unwrap().success());

    // TODO(sleffler): requires pip install tempita
    let xml_arch_file = &*format!(
        "{}/libsel4/arch_include/{}/interfaces/sel4arch.xml",
        sel4_dir, archdir
    );
    let xml_sel4_arch_file = format!(
        "{}/libsel4/sel4_arch_include/{}/interfaces/sel4arch.xml",
        sel4_dir, arch
    );
    let mut cmd = Command::new("/usr/bin/env");
    cmd.arg(&python_bin).args(&[
        "tools/invocation_header_gen.py",
        "--dest",
        &*format!("{}/{}_invocation.rs", out_dir, arch),
        &*xml_interfaces_file,
        &*xml_arch_file,
        &*xml_sel4_arch_file,
    ]);
    println!("Running {:?}", cmd);
    assert!(cmd.status().unwrap().success());

    // TODO(sleffler): requires pip install tempita
    let mut cmd = Command::new("/usr/bin/env");
    cmd.arg(&python_bin).args(&[
        "tools/syscall_header_gen.py",
        #[cfg(feature = "CONFIG_KERNEL_MCS")]
        "--mcs",
        "--xml",
        &*format!("{}/libsel4/include/api/syscall.xml", sel4_dir),
        "--dest",
        &*format!("{}/syscalls.rs", out_dir),
    ]);
    println!("Running {:?}", cmd);
    assert!(cmd.status().unwrap().success());

    let bfin = File::open(&*format!(
        "{}/libsel4/mode_include/{}/sel4/shared_types.bf",
        sel4_dir, cargo_target_pointer_width
    ))
    .unwrap();
    println!("{}/types{}.rs", out_dir, cargo_target_pointer_width);
    let bfout = File::create(&*format!("{}/types{}.rs", out_dir, cargo_target_pointer_width)).unwrap();
    let mut cmd = Command::new("/usr/bin/env");
    cmd.arg(&python_bin)
        .arg("tools/bitfield_gen.py")
        .arg("--language=rust")
        //       .arg("--word-size=32")
        .stdin(unsafe { Stdio::from_raw_fd(bfin.as_raw_fd()) })
        .stdout(unsafe { Stdio::from_raw_fd(bfout.as_raw_fd()) });
    println!("Running {:?}", cmd);
    assert!(cmd.status().unwrap().success());
    std::mem::forget(bfin);
    std::mem::forget(bfout);
}
