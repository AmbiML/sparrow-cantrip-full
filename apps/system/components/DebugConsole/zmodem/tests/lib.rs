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

fn forget_err(_e: std::io::Error) -> cantrip_io::Error {
    cantrip_io::Error {}
}

struct InOut<R: Read, W: Write> {
    r: R,
    w: W,
}

impl<R: Read, W: Write> InOut<R, W> {
    pub fn new(r: R, w: W) -> InOut<R, W> {
        InOut { r, w }
    }
}

impl<R: Read, W: Write> cantrip_io::Read for InOut<R, W> {
    fn read(&mut self, buf: &mut [u8]) -> cantrip_io::Result<usize> {
        self.r.read(buf).map_err(forget_err)
    }
}

impl<R: Read, W: Write> cantrip_io::Write for InOut<R, W> {
    fn write(&mut self, buf: &[u8]) -> cantrip_io::Result<usize> {
        self.w.write(buf).map_err(forget_err)
    }

    fn flush(&mut self) -> cantrip_io::Result<()> {
        self.w.flush().map_err(forget_err)
    }
}

/// Minimal vector-backed cantrip_io::Read and cantrip_io::Write
///
/// This is a workaround to not being able to implement the cantrip_io traits for
/// std::io::Cursor in this file, since none of those come from the current
/// crate.
struct WrappedCursor<T> {
    c: std::io::Cursor<T>,
}

impl<T> WrappedCursor<T> {
    pub fn into_inner(self) -> T {
        self.c.into_inner()
    }
}

impl cantrip_io::Read for WrappedCursor<&Vec<u8>> {
    fn read(&mut self, buf: &mut [u8]) -> cantrip_io::Result<usize> {
        self.c.read(buf).map_err(forget_err)
    }
}

impl cantrip_io::Write for WrappedCursor<Vec<u8>> {
    fn write(&mut self, buf: &[u8]) -> cantrip_io::Result<usize> {
        self.c.write(buf).map_err(forget_err)
    }

    fn flush(&mut self) -> cantrip_io::Result<()> {
        self.c.flush().map_err(forget_err)
    }
}

impl cantrip_io::Seek for WrappedCursor<&Vec<u8>> {
    fn seek(&mut self, pos: cantrip_io::SeekFrom) -> cantrip_io::Result<u64> {
        let std_pos = match pos {
            cantrip_io::SeekFrom::Start(n) => std::io::SeekFrom::Start(n),
            cantrip_io::SeekFrom::End(n) => std::io::SeekFrom::End(n),
            cantrip_io::SeekFrom::Current(n) => std::io::SeekFrom::Current(n),
        };
        self.c.seek(std_pos).map_err(forget_err)
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

    let child_stdin = sz.stdin.unwrap();
    let child_stdout = sz.stdout.unwrap();
    let mut inout = InOut::new(child_stdout, child_stdin);

    let mut c = WrappedCursor {
        c: Cursor::new(Vec::new()),
    };
    zmodem::recv::recv(&mut inout, &mut c).unwrap();

    sleep(Duration::from_millis(300));
    remove_file("recv_from_sz").unwrap();

    assert_eq!(RND_VALUES.clone(), c.into_inner());
}

#[test]
#[cfg(unix)]
fn send_to_rz() {
    let _ = LOG_INIT.is_ok();

    let _ = remove_file("send_to_rz");

    let sz = Command::new("rz")
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .spawn()
        .expect("rz failed to run");

    let child_stdin = sz.stdin.unwrap();
    let child_stdout = sz.stdout.unwrap();
    let mut inout = InOut::new(child_stdout, child_stdin);

    let len = RND_VALUES.len() as u32;
    let copy = RND_VALUES.clone();
    let mut cur = WrappedCursor {
        c: Cursor::new(&copy),
    };

    sleep(Duration::from_millis(300));

    zmodem::send::send(&mut inout, &mut cur, "send_to_rz", Some(len)).unwrap();

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
        let inf = File::open("test-fifo2").unwrap();
        let mut inout = InOut::new(inf, outf);

        let origin = RND_VALUES.clone();
        let mut c = WrappedCursor {
            c: Cursor::new(&origin),
        };

        zmodem::send::send(&mut inout, &mut c, "test", None).unwrap();
    });

    let mut c = WrappedCursor {
        c: Cursor::new(Vec::new()),
    };

    let inf = File::open("test-fifo1").unwrap();
    let outf = OpenOptions::new().write(true).open("test-fifo2").unwrap();
    let mut inout = InOut::new(inf, outf);

    zmodem::recv::recv(&mut inout, &mut c).unwrap();

    let _ = remove_file("test-fifo1");
    let _ = remove_file("test-fifo2");

    assert_eq!(RND_VALUES.clone(), c.into_inner());
}
