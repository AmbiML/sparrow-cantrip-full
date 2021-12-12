// Cargo/rust build glue to import seL4 kernel configuration. We
// parse the gen_config.h file from a build area to find features
// needed by a dependent crate (sel4-sys, cantrip-os-rootserver, etc).
//
// The caller is responsible for supplying a pathname to the top
// of the kernel build area. Typically this comes from the
// SEL4_OUT_DIR environment variable.

use std::collections::BTreeSet;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::string::String;
use std::vec::Vec;

pub fn get_sel4_features(sel4_out_dir: &str) -> Vec<String> {
    // Parse the kernel's gen_config.h file to get features.
    let gen_config_file =
        File::open(format!("{}/gen_config/kernel/gen_config.h", sel4_out_dir)).unwrap();
    let kernel_config_features = BufReader::new(gen_config_file)
        .lines()
        .filter_map(|line| {
            let line = line.unwrap();
            let mut splitted = line.split_whitespace();
            match (splitted.next()?, splitted.next()?) {
                ("#define", param) => Some(param.to_owned()),
                _ => None,
            }
        })
        .collect::<BTreeSet<_>>();
    println!("kernel_config_features {:?}", kernel_config_features);

    // Return only features specified in the Cargo.toml.
    let manifest_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("Cargo.toml");
    let manifest = cargo_toml::Manifest::from_path(manifest_path).unwrap();
    manifest
        .features
        .into_keys()
        .collect::<BTreeSet<String>>()
        .intersection(&kernel_config_features)
        .cloned()
        .collect::<Vec<String>>()
}

// TODO(sleffler): unit tests
