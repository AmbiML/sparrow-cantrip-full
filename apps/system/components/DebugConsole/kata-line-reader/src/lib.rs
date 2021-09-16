#![no_std]

use core::fmt;

use cantrip_io as io;

const LINE_MAX: usize = 128;

pub enum LineReadError {
    IO(io::Error),
    Overflow,
    Encoding(core::str::Utf8Error),
}

impl From<io::Error> for LineReadError {
    fn from(err: io::Error) -> LineReadError {
        LineReadError::IO(err)
    }
}

impl From<core::str::Utf8Error> for LineReadError {
    fn from(err: core::str::Utf8Error) -> LineReadError {
        LineReadError::Encoding(err)
    }
}

impl fmt::Display for LineReadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LineReadError::IO(_) => write!(f, "IO error"),
            LineReadError::Overflow => write!(f, "line too long"),
            LineReadError::Encoding(_) => write!(f, "bad character encoding"),
        }
    }
}

pub struct LineReader {
    // Owned by LineReader to facilitate static allocation.
    buf: [u8; LINE_MAX],
}

fn get_u8(reader: &mut dyn io::Read) -> io::Result<u8> {
    let mut buf: [u8; 1] = [0u8];
    reader.read_exact(&mut buf)?;
    Ok(buf[0])
}

impl LineReader {
    pub fn new() -> LineReader {
        LineReader {
            buf: [0u8; LINE_MAX],
        }
    }

    pub fn read_line(
        &mut self,
        output: &mut dyn io::Write,
        input: &mut dyn io::Read,
    ) -> Result<&str, LineReadError> {
        const DEL: u8 = 127u8;
        const BACKSPACE: u8 = 8u8;
        let mut len = 0;
        while len < self.buf.len() {
            let mut c = get_u8(input)?;
            while c == DEL || c == BACKSPACE {
                if len > 0 {
                    output.write(&[BACKSPACE, b' ', BACKSPACE])?;
                    len -= 1;
                }
                c = get_u8(input)?;
            }
            if c == b'\r' || c == b'\n' {
                if len > 0 {
                    output.write(&[b'\n'])?;
                }
                return Ok(core::str::from_utf8(&self.buf[0..len])?);
            }
            self.buf[len] = c;
            len += 1;
            output.write(&[c])?;
        }
        Err(LineReadError::Overflow)
    }
}
