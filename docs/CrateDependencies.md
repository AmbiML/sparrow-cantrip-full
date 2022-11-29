
### Depending on CantripOS Rust crates

To use CantripOS crates you can reference them from a local repository or
directly from GitHub using git; e.g. in a Config.toml:
```
cantrip-os-common = { path = "../system/components/cantrip-os-common" }
cantrip-os-common = { git = "https://github.com/AmbiML/sparrow-cantrip-full" }
```
NB: the git usage depends on cargo's support for searching for a crate
named "cantrip-os-common" in the cantrip repo.

Note that many CantripOS crates need the seL4 kernel configuration
(e.g. to know whether MCS is configured). This is handled by the
cantrip-os-common/sel4-config crate that is used by a build.rs to import
kernel configuration parameters as Cargo features. In a Cargo.toml create
a features manifest with the kernel parameters you need e.g.

```
[features]
default = []
# Used by sel4-config to extract kernel config
CONFIG_PRINTING = []
```

then specify build-dependencies:

```
[build-dependencies]
# build.rs depends on SEL4_OUT_DIR = "${ROOTDIR}/out/cantrip/kernel"
sel4-config = { path = "../../cantrip/apps/system/components/cantrip-os-common/src/sel4-config" }
```

and use a build.rs that includes at least:

```
extern crate sel4_config;
use std::env;

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
}
```

Note how build.rs expects an SEL4_OUT_DIR environment variable that has the path to
the top of the kernel build area. The build-sparrow.sh script sets this for you but, for
example, if you choose to run ninja directly you will need it set in your environment.

Similar to SEL4_OUT_DIR the cantrip-os-common/src/sel4-sys crate that has the seL4 system
call wrappers for Rust programs requires an SEL4_DIR envronment variable that has the
path to the top of the kernel sources. This also is set by build-sparrow.sh.
