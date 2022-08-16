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

extern crate env_logger;
extern crate log;
extern crate zmodem;
#[macro_use]
extern crate lazy_static;
extern crate rand;

use std::fs::{remove_file, File, OpenOptions};
use std::io::*;
use std::process::*;
use std::result;
use std::thread::{sleep, spawn};
use std::time::*;

extern crate cantrip_io;

fn forget_err(_e: std::io::Error) -> cantrip_io::Error { cantrip_io::Error {} }

struct ReadWrapper<R: std::io::Read> {
    r: R,
}

impl<R: std::io::Read> cantrip_io::Read for ReadWrapper<R> {
    fn read(&mut self, buf: &mut [u8]) -> cantrip_io::Result<usize> {
        self.r.read(buf).map_err(forget_err)
    }
}

struct WriteWrapper<W: std::io::Write> {
    w: W,
}

impl<W: std::io::Write> cantrip_io::Write for WriteWrapper<W> {
    fn write(&mut self, buf: &[u8]) -> cantrip_io::Result<usize> {
        self.w.write(buf).map_err(forget_err)
    }

    fn flush(&mut self) -> cantrip_io::Result<()> { self.w.flush().map_err(forget_err) }
}

struct SendInput<T: std::io::Read + std::io::Seek> {
    t: T,
}

impl<T: std::io::Read + std::io::Seek> cantrip_io::Read for SendInput<T> {
    fn read(&mut self, buf: &mut [u8]) -> cantrip_io::Result<usize> {
        self.t.read(buf).map_err(forget_err)
    }
}

impl<T: std::io::Read + std::io::Seek> cantrip_io::Seek for SendInput<T> {
    fn seek(&mut self, pos: cantrip_io::SeekFrom) -> cantrip_io::Result<u64> {
        let std_pos = match pos {
            cantrip_io::SeekFrom::Start(n) => std::io::SeekFrom::Start(n),
            cantrip_io::SeekFrom::End(n) => std::io::SeekFrom::End(n),
            cantrip_io::SeekFrom::Current(n) => std::io::SeekFrom::Current(n),
        };
        self.t.seek(std_pos).map_err(forget_err)
    }
}

lazy_static! {
    static ref LOG_INIT: result::Result<(), log::SetLoggerError> = Ok(env_logger::init());
    static ref RND_VALUES: Vec<u8> = {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut buf = vec![0; 1024 * 1024 * 11];
        rng.fill_bytes(&mut buf);
        buf
    };
}

#[test]
#[cfg(unix)]
fn recv_from_sz() {
    let _ = LOG_INIT.is_ok();

    let mut f = File::create("recv_from_sz").unwrap();
    f.write_all(&RND_VALUES).unwrap();

    let sz = Command::new("sz")
        .arg("recv_from_sz")
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .spawn()
        .expect("sz failed to run");

    let mut c = Cursor::new(Vec::new());

    zmodem::recv::recv(
        ReadWrapper {
            r: sz.stdout.unwrap(),
        },
        WriteWrapper {
            w: sz.stdin.unwrap(),
        },
        WriteWrapper { w: &mut c },
    )
    .unwrap();

    sleep(Duration::from_millis(300));
    remove_file("recv_from_sz").unwrap();

    assert_eq!(RND_VALUES.clone(), c.into_inner());
}

#[test]
#[cfg(unix)]
fn send_to_rz() {
    let _ = LOG_INIT.is_ok();

    let _ = remove_file("send_to_rz");

    let rz = Command::new("rz")
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .spawn()
        .expect("rz failed to run");

    let len = RND_VALUES.len() as u32;
    let copy = RND_VALUES.clone();

    sleep(Duration::from_millis(300));

    zmodem::send::send(
        ReadWrapper {
            r: rz.stdout.unwrap(),
        },
        WriteWrapper {
            w: rz.stdin.unwrap(),
        },
        SendInput {
            t: Cursor::new(&copy),
        },
        "send_to_rz",
        Some(len),
    )
    .unwrap();
    sleep(Duration::from_millis(300));

    let mut f = File::open("send_to_rz").expect("open 'send_to_rz'");
    let mut received = Vec::new();
    f.read_to_end(&mut received).unwrap();
    remove_file("send_to_rz").unwrap();

    assert!(copy == received);
}

#[test]
#[cfg(unix)]
fn lib_send_recv() {
    let _ = LOG_INIT;

    let _ = remove_file("test-fifo1");
    let _ = remove_file("test-fifo2");

    let _ = Command::new("mkfifo")
        .arg("test-fifo1")
        .spawn()
        .expect("mkfifo failed to run")
        .wait();

    let _ = Command::new("mkfifo")
        .arg("test-fifo2")
        .spawn()
        .expect("mkfifo failed to run")
        .wait();

    sleep(Duration::from_millis(300));

    spawn(move || {
        let outf = OpenOptions::new().write(true).open("test-fifo1").unwrap();
        zmodem::send::send(
            ReadWrapper {
                r: File::open("test-fifo2").unwrap(),
            },
            WriteWrapper { w: outf },
            SendInput {
                t: Cursor::new(&RND_VALUES.clone()),
            },
            "test",
            None,
        )
        .unwrap();
    });

    let mut c = Cursor::new(Vec::new());

    zmodem::recv::recv(
        ReadWrapper {
            r: File::open("test-fifo1").unwrap(),
        },
        WriteWrapper {
            w: OpenOptions::new().write(true).open("test-fifo2").unwrap(),
        },
        WriteWrapper { w: &mut c },
    )
    .unwrap();

    let _ = remove_file("test-fifo1");
    let _ = remove_file("test-fifo2");

    assert_eq!(RND_VALUES.clone(), c.into_inner());
}
