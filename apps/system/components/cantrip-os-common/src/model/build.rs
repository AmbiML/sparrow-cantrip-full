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

extern crate sel4_config;
use std::env;
use std::fs;
use std::io::Write;

#[derive(serde::Deserialize)]
struct PlatformInfo {
    memory: Vec<MemoryRange>,
}

#[derive(serde::Deserialize)]
struct MemoryRange {
    start: u64,
    end: u64,
}

fn main() {
    // If SEL4_OUT_DIR is not set we expect the kernel build at a fixed
    // location relative to the ROOTDIR env variable.
    println!("SEL4_OUT_DIR {:?}", env::var("SEL4_OUT_DIR"));
    let sel4_out_dir = env::var("SEL4_OUT_DIR")
        .unwrap_or_else(|_| format!("{}/out/cantrip/kernel", env::var("ROOTDIR").unwrap()));
    println!("sel4_out_dir {}", sel4_out_dir);

    // Dredge seL4 kernel config for settings we need as features to generate
    // correct code: e.g. CONFIG_KERNEL_MCS enables MCS support which changes
    // the system call numbering.
    let features = sel4_config::get_sel4_features(&sel4_out_dir);
    println!("features={:?}", features);
    for feature in features {
        println!("cargo:rustc-cfg=feature=\"{}\"", feature);
    }

    // Some architectures need informations from platform_yaml.
    let platform_yaml_path = format!("{}/gen_headers/plat/machine/platform_gen.yaml", sel4_out_dir);
    if let Ok(platform_yaml) = fs::File::open(&platform_yaml_path) {
        let platform_info: PlatformInfo =
            serde_yaml::from_reader(platform_yaml).expect("invalid yaml file");
        let out_dir = env::var("OUT_DIR").unwrap();
        let out_path = std::path::Path::new(&out_dir).join("platform_gen.rs");
        let mut out_file = fs::File::create(&out_path).unwrap();

        writeln!(
            &mut out_file,
            "struct MemoryRegion {{
                 start: usize,
                 end: usize,
            }}"
        )
        .unwrap();
        writeln!(
            &mut out_file,
            "const MEMORY_REGIONS: [MemoryRegion; {}] = [",
            platform_info.memory.len()
        )
        .unwrap();
        for range in platform_info.memory {
            writeln!(
                &mut out_file,
                "    MemoryRegion {{ start: 0x{:X}, end: 0x{:X} }},",
                range.start, range.end
            )
            .ok();
        }
        writeln!(&mut out_file, "];").ok();
    }
}
