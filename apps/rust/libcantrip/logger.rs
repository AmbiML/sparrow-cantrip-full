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

use core2::io::{Cursor, Write};
use log::{Metadata, Record};
use sdk_interface::sdk_log;

// TODO(sleffler): until we can copy directly into shared memory
//   stack allocation (can be up to 4096).
const MAX_MSG_LEN: usize = 2048;

pub struct SDKLogger;

impl log::Log for SDKLogger {
    fn enabled(&self, metadata: &Metadata) -> bool { metadata.level() <= log::max_level() }
    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let mut buf = [0 as u8; MAX_MSG_LEN];
            let mut cur = Cursor::new(&mut buf[..]);
            // NB: no need to prepend record.target(), SDKRuntime will
            //   prepend the bundle-id
            write!(&mut cur, "{}", record.args()).unwrap_or_else(|_| {
                // Too big, indicate overflow with a trailing "...".
                cur.set_position((MAX_MSG_LEN - 3) as u64);
                cur.write(b"...").expect("write!");
                ()
            });
            // NB: this releases the ref on buf held by the Cursor
            let pos = cur.position() as usize;
            // TODO(sleffler): handle error
            let _ = sdk_log(core::str::from_utf8(&buf[..pos]).unwrap());
        }
    }
    fn flush(&self) {}
}
