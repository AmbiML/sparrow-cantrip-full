/// Wrapper types for fully-buffered ZMODEM receives.
use alloc::vec::Vec;

use crc::crc32;
use crc::Hasher32;
use log;

use zmodem;

use cantrip_io as io;

pub struct Upload {
    digest: crc32::Digest,
    contents: Vec<u8>,
}

impl Upload {
    pub fn new() -> Upload {
        Upload {
            digest: crc32::Digest::new(crc32::IEEE),
            contents: Vec::new(),
        }
    }

    pub fn crc32(&self) -> u32 {
        self.digest.sum32()
    }

    pub fn contents(&self) -> &[u8] {
        self.contents.as_slice()
    }
}

impl io::Write for Upload {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.digest.write(buf);
        self.contents.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Receives using ZMODEM and wraps the result as an Upload.
pub fn rz<R: io::BufRead, W: io::Write>(r: R, w: W) -> Result<Upload, io::Error> {
    let mut upload = Upload::new();

    // Turn off logging, since it goes to the UART and will cause the sender to
    // abort.
    let prior_log_level = log::max_level();
    log::set_max_level(log::LevelFilter::Off);

    zmodem::recv::recv(r, w, &mut upload)?;

    log::set_max_level(prior_log_level);
    Ok(upload)
}
