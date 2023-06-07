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

fn main() {
    let build_for_ci = env::var("CANTRIP_BUILD_FOR_CI");
    if build_for_ci.is_ok() {
        println!("cargo:rustc-cfg=build_for_ci");
    }
}
