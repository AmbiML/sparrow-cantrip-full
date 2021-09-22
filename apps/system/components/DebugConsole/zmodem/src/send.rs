use cantrip_io as io;

use consts::*;
use frame::*;
use proto::*;

const SUBPACKET_SIZE: usize = 1024 * 8;
const SUBPACKET_PER_ACK: usize = 10;

#[derive(Debug, PartialEq)]
enum State {
    /// Waiting ZRINIT invite (do nothing)
    WaitingInit,

    /// Sending ZRQINIT
    SendingZRQINIT,

    /// Sending ZFILE frame
    SendingZFILE,

    /// Do nothing, just waiting for ZPOS
    WaitingZPOS,

    /// Sending ZDATA & subpackets
    SendingData,

    /// Sending ZFIN
    SendingZFIN,

    /// All works done, exiting
    Done,
}

impl State {
    fn new() -> State {
        State::WaitingInit
    }

    fn next(self, frame: &Frame) -> State {
        match (self, frame.get_frame_type()) {
            (State::WaitingInit, ZRINIT) => State::SendingZFILE,
            (State::WaitingInit, _) => State::SendingZRQINIT,

            (State::SendingZRQINIT, ZRINIT) => State::SendingZFILE,

            (State::SendingZFILE, ZRPOS) => State::SendingData,
            (State::SendingZFILE, ZRINIT) => State::WaitingZPOS,

            (State::WaitingZPOS, ZRPOS) => State::SendingData,

            (State::SendingData, ZACK) => State::SendingData,
            (State::SendingData, ZRPOS) => State::SendingData,
            (State::SendingData, ZRINIT) => State::SendingZFIN,

            (State::SendingZFIN, ZFIN) => State::Done,

            (s, _) => {
                error!("Unexpected (state, frame) combination: {:#?} {}", s, frame);
                s // don't change current state
            }
        }
    }
}

pub fn send<CI, CO, DI>(
    mut channel_in: CI,
    mut channel_out: CO,
    mut r: DI,
    filename: &str,
    filesize: Option<u32>,
) -> io::Result<()>
where
    CI: io::Read,
    CO: io::Write,
    DI: io::Read + io::Seek,
{
    let mut data = [0; SUBPACKET_SIZE];
    let mut offset: u32;

    write_zrqinit(&mut channel_out)?;

    let mut state = State::new();

    while state != State::Done {
        channel_out.flush()?;

        if !find_zpad(&mut channel_in)? {
            continue;
        }

        let frame = match parse_header(&mut channel_in)? {
            Some(x) => x,
            None => {
                write_znak(&mut channel_out)?;
                continue;
            }
        };

        state = state.next(&frame);
        debug!("State: {:?}", state);

        // do things according new state
        match state {
            State::SendingZRQINIT => {
                write_zrqinit(&mut channel_out)?;
            }
            State::SendingZFILE => {
                write_zfile(&mut channel_out, filename, filesize)?;
            }
            State::SendingData => {
                offset = frame.get_count();
                r.seek(io::SeekFrom::Start(offset as u64))?;

                let num = r.read(&mut data)?;

                if num == 0 {
                    write_zeof(&mut channel_out, offset)?;
                } else {
                    // ZBIN32|ZDATA
                    // ZCRCG - best perf
                    // ZCRCQ - mid perf
                    // ZCRCW - worst perf
                    // ZCRCE - send at end
                    write_zdata(&mut channel_out, offset)?;

                    let mut i = 0;
                    loop {
                        i += 1;

                        write_zlde_data(&mut channel_out, ZCRCG, &data[..num])?;
                        offset += num as u32;

                        let num = r.read(&mut data)?;
                        if num < data.len() || i >= SUBPACKET_PER_ACK {
                            write_zlde_data(&mut channel_out, ZCRCW, &data[..num])?;
                            break;
                        }
                    }
                }
            }
            State::SendingZFIN => {
                write_zfin(&mut channel_out)?;
            }
            State::Done => {
                write_over_and_out(&mut channel_out)?;
            }
            _ => (),
        }
    }

    Ok(())
}
