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

extern crate bindgen;

extern crate sel4_config;
use std::env;
use std::path::PathBuf;

fn main() {
    // Add bindings for OpenTitan UART register header file.
    let cantrip_target_arch = env::var("CANTRIP_TARGET_ARCH").unwrap();
    let out_dir = env::var("OUT").unwrap();
    let opentitan_gen_path = format!(
        "{}/cantrip/{}/opentitan-gen/include", out_dir, cantrip_target_arch);
    let mut builder = bindgen::Builder::default().header(
        format!("{}/opentitan/uart.h", opentitan_gen_path));
    builder = builder
        .clang_arg(format!("-I/{}", opentitan_gen_path))
        .clang_arg(format!("--target={}", cantrip_target_arch));

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let bindings = builder
        .generate()
        .expect("Unable to get bindings.");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}