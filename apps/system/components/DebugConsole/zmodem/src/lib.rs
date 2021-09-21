#![no_std]

extern crate alloc;
extern crate crc as crc32;
extern crate hex;
extern crate cantrip_io;
#[macro_use]
extern crate log;

mod consts;
mod crc;
mod frame;
mod proto;

pub mod recv;
pub mod send;
