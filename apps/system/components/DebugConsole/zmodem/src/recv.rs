use alloc::vec::Vec;
use core::str::from_utf8;

use cantrip_io as io;

use consts::*;
use frame::*;
use proto::*;

#[derive(Debug, PartialEq)]
enum State {
    /// Sending ZRINIT
    SendingZRINIT,

    /// Processing ZFILE supplementary data
    ProcessingZFILE,

    /// Receiving file's content
    ReceivingData,

    /// Checking length of received data
    CheckingData,

    /// All works done, exiting
    Done,
}

impl State {
    fn new() -> State {
        State::SendingZRINIT
    }

    fn next(self, frame: &Frame) -> State {
        match (self, frame.get_frame_type()) {
            (State::SendingZRINIT, ZFILE) => State::ProcessingZFILE,
            (State::SendingZRINIT, _) => State::SendingZRINIT,

            (State::ProcessingZFILE, ZDATA) => State::ReceivingData,
            (State::ProcessingZFILE, _) => State::ProcessingZFILE,

            (State::ReceivingData, ZDATA) => State::ReceivingData,
            (State::ReceivingData, ZEOF) => State::CheckingData,

            (State::CheckingData, ZDATA) => State::ReceivingData,
            (State::CheckingData, ZFIN) => State::Done,

            (s, _) => {
                error!("Unexpected (state, frame) combination: {:#?} {}", s, frame);
                s // don't change current state
            }
        }
    }
}

/// Receives data by Z-Modem protocol
pub fn recv<CI, CO, DO>(
    mut channel_in: CI,
    mut channel_out: CO,
    mut data_out: DO,
) -> io::Result<usize>
where
    CI: io::BufRead,
    CO: io::Write,
    DO: io::Write,
{
    let mut count = 0;

    let mut state = State::new();

    write_zrinit(&mut channel_out)?;

    while state != State::Done {
        if !find_zpad(&mut channel_in)? {
            continue;
        }

        let frame = match parse_header(&mut channel_in)? {
            Some(x) => x,
            None => {
                recv_error(&mut channel_out, &state, count)?;
                continue;
            }
        };

        state = state.next(&frame);
        debug!("State: {:?}", state);

        // do things according new state
        match state {
            State::SendingZRINIT => {
                write_zrinit(&mut channel_out)?;
            }
            State::ProcessingZFILE => {
                let mut buf = Vec::new();

                if recv_zlde_frame(frame.get_header(), &mut channel_in, &mut buf)?.is_none() {
                    write_znak(&mut channel_out)?;
                } else {
                    write_zrpos(&mut channel_out, count)?;

                    // TODO: process supplied data
                    if let Ok(s) = from_utf8(&buf) {
                        debug!(target: "proto", "ZFILE supplied data: {}", s);
                    }
                }
            }
            State::ReceivingData => {
                if frame.get_count() != count
                    || !recv_data(
                        frame.get_header(),
                        &mut count,
                        &mut channel_in,
                        &mut channel_out,
                        &mut data_out,
                    )?
                {
                    write_zrpos(&mut channel_out, count)?;
                }
            }
            State::CheckingData => {
                if frame.get_count() != count {
                    error!(
                        "ZEOF offset mismatch: frame({}) != recv({})",
                        frame.get_count(),
                        count
                    );
                    // receiver ignores the ZEOF because a new zdata is coming
                } else {
                    write_zrinit(&mut channel_out)?;
                }
            }
            State::Done => {
                write_zfin(&mut channel_out)?;
                // lexxvir/zmodem had a 30ms sleep here, maybe for the
                // following behavior from the ZMODEM spec: "The receiver
                // waits briefly for the "O" characters, then exits whether
                // they were received or not."
                //
                // sz does send these characters, and 2 more bytes before them.
                // If we don't consume them here, they will become garbage on
                // input after returning.
                read_until_match(OO.as_bytes(), &mut channel_in)?;
            }
        }
    }

    Ok(count as usize)
}

fn recv_error<W>(w: &mut W, state: &State, count: u32) -> io::Result<()>
where
    W: io::Write,
{
    // TODO: flush input

    match *state {
        State::ReceivingData => write_zrpos(w, count),
        _ => write_znak(w),
    }
}

fn read_until_match<R: io::Read>(pattern: &[u8], mut r: R) -> io::Result<()> {
    let mut remainder = pattern;
    let mut b = [0u8; 1];
    while !remainder.is_empty() {
        r.read(&mut b)?;
        if b[0] == remainder[0] {
            remainder = &remainder[1..];
        }
    }
    Ok(())
}
