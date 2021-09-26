#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
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

/// Partial mimic of std::io::BufRead.
pub trait BufRead: Read {
    fn fill_buf(&mut self) -> Result<&[u8]>;

    fn consume(&mut self, amt: usize);

    fn read_until(&mut self, delim: u8, buf: &mut Vec<u8>) -> Result<usize> {
        // Implementation adapted from std::io.
        let mut read = 0;
        loop {
            let (done, used) = {
                let available = self.fill_buf()?;
                match memchr::memchr(delim, available) {
                    Some(i) => {
                        buf.extend_from_slice(&available[..=i]);
                        (true, i + 1)
                    }
                    None => {
                        buf.extend_from_slice(available);
                        (false, available.len())
                    }
                }
            };
            self.consume(used);
            read += used;
            if done || used == 0 {
                return Ok(read);
            }
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

pub struct BufReader<R> {
    inner: R,
    buf: Box<[u8]>,
    pos: usize,
    cap: usize,
}

impl<R: Read> BufReader<R> {
    pub fn new(inner: R) -> BufReader<R> {
        const BUFFER_SIZE : usize = 1024;  // free to be changed
        BufReader {
            inner: inner,
            buf: Box::new([0u8; BUFFER_SIZE]),
            pos: 0,
            cap: 0,
        }
    }

    fn discard_buffer(&mut self) {
        // Implementation copied from std::io.
        self.pos = 0;
        self.cap = 0;
    }
}

impl<R: Read> Read for BufReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        // Implementation copied from std::io.

        // If we don't have any buffered data and we're doing a massive read
        // (larger than our internal buffer), bypass our internal buffer
        // entirely.
        if self.pos == self.cap && buf.len() >= self.buf.len() {
            self.discard_buffer();
            return self.inner.read(buf);
        }
        let nread = {
            let mut rem = self.fill_buf()?;
            rem.read(buf)?
        };
        self.consume(nread);
        Ok(nread)
    }
}

impl<R: Read> BufRead for BufReader<R> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        // Implementation copied from std::io.

        // If we've reached the end of our internal buffer then we need to fetch
        // some more data from the underlying reader.
        // Branch using `>=` instead of the more correct `==`
        // to tell the compiler that the pos..cap slice is always valid.
        if self.pos >= self.cap {
            debug_assert!(self.pos == self.cap);
            self.cap = self.inner.read(&mut self.buf)?;
            self.pos = 0;
        }
        Ok(&self.buf[self.pos..self.cap])
    }

    fn consume(&mut self, amt: usize) {
        // Implementation copied from std::io.
        self.pos = cmp::min(self.pos + amt, self.cap);
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

/// Forwarding implementation of BufRead for &mut
impl<'a, T: ?Sized> BufRead for &'a mut T
where
    T: BufRead,
{
    fn fill_buf(&mut self) -> Result<&[u8]> {
        (**self).fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        (**self).consume(amt)
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
