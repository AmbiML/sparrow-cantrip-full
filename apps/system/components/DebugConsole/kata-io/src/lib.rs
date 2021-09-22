#![no_std]

use core::cmp;

#[derive(Debug)]
pub struct Error;

pub type Result<T> = core::result::Result<T, Error>;

/// Partial mimic of std::io::Read.
pub trait Read {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

    fn read_exact(&mut self, mut buf: &mut [u8]) -> Result<()> {
        while !buf.is_empty() {
            match self.read(buf) {
                Ok(0) => break,
                Ok(n) => {
                    let tmp = buf;
                    buf = &mut tmp[n..];
                }
                Err(e) => return Err(e),
            }
        }
        if !buf.is_empty() {
            Err(Error)
        } else {
            Ok(())
        }
    }
}

/// Partial mimic of std::io::Write.
pub trait Write {
    fn write(&mut self, buf: &[u8]) -> Result<usize>;

    fn flush(&mut self) -> Result<()>;

    fn write_all(&mut self, mut buf: &[u8]) -> Result<()> {
        while !buf.is_empty() {
            match self.write(buf) {
                Ok(0) => return Err(Error),
                Ok(n) => buf = &buf[n..],
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

/// Adapter for writing core::fmt formatted strings.
impl core::fmt::Write for dyn Write + '_ {
    /// Writes the bytes of a &str to the underlying writer.
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        match self.write(s.as_bytes()) {
            Ok(_) => Ok(()),
            Err(_) => Err(core::fmt::Error),
        }
    }
}

pub enum SeekFrom {
    Start(u64),
    End(i64),
    Current(i64),
}

/// Partial mimic of std::io::Seek.
pub trait Seek {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64>;
}

impl Read for &[u8] {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let amt = cmp::min(buf.len(), self.len());
        let (a, b) = self.split_at(amt);
        buf[..amt].copy_from_slice(a);
        *self = b;
        Ok(amt)
    }
}

/// Forwarding implementation of Read for &mut
impl<'a, T: ?Sized> Read for &'a mut T
where
    T: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        (**self).read(buf)
    }
}

/// Forwarding implementation of Write for &mut
impl<'a, T: ?Sized> Write for &'a mut T
where
    T: Write,
{
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        (**self).write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        (**self).flush()
    }
}
