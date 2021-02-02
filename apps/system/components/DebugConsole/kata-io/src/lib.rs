#![no_std]

pub struct Error;

/// Interface for the CLI to consume bytes.
pub trait Read {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error>;
}

/// Interface for the CLI to emit bytes.
pub trait Write {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error>;
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

impl dyn Read + '_ {
    pub fn get_u8(&mut self) -> Result<u8, Error> {
        let mut buf: [u8; 1] = [0u8];
        let n_read = self.read(&mut buf)?;
        match n_read {
            1usize => Ok(buf[0]),
            _ => Err(Error),
        }
    }
}
