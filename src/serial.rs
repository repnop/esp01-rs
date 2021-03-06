use embedded_hal as hal;
use nb::{self, block};

use crate::timing::{Millisecond, LongTimer};


#[derive(Debug)]
pub enum Error<E> {
    /// Serial interface error
    Serial(E),
    /// Timeout before receiving reply
    TimedOut,
}

pub fn read_with_timeout<S, T>(
    serial: &mut S,
    timer: &mut T,
    timeout: Millisecond,
) -> Result<u8, Error<S::Error>>
where
    T: LongTimer,
    S: hal::serial::Read<u8>,
{
    timer.start(timeout);
    loop {
        match serial.read() {
            // raise error
            Err(nb::Error::Other(e)) => return Err(Error::Serial(e)),
            Err(nb::Error::WouldBlock) => {
                // no data available yet, check the timer below
            },
            Ok(byte) => return Ok(byte),
        }

        match timer.wait() {
            Err(nb::Error::Other(_e)) => {
                // The error type specified by `timer.wait()` is `!`, which
                // means no error can actually occur. The Rust compiler
                // still forces us to provide this match arm, though.
                unreachable!("Error was !, something has gone horribly wrong")
            },
            // no timeout yet, try again
            Err(nb::Error::WouldBlock) => continue,
            Ok(()) => {
                return Err(Error::TimedOut);
            }
        }
    }
}

/**
*/
pub fn read_until_message<S, T, C, R>(
    rx: &mut S,
    timer: &mut T,
    timeout: Millisecond,
    buffer: &mut [u8],
    parser: &C
) -> Result<R, Error<S::Error>>
where
    T: LongTimer,
    S: hal::serial::Read<u8>,
    C: Fn(&[u8], usize) -> Option<R>,
{
    let mut ptr = 0;
    loop {
        match read_with_timeout(rx, timer, timeout) {
            Ok(byte) => {
                buffer[ptr] = byte;
                ptr = (ptr+1) % buffer.len();

                if let Some(val) = parser(buffer, ptr) {
                    return Ok(val);
                }
            },
            Err(Error::TimedOut) => {
                // If the remote end has already sent bytes and has now
                // stopped, we assume the transmission has ended
                return Err(Error::TimedOut);
            },
            Err(e) => {
                return Err(e)
            }
        };
    }
}

pub fn write_all<S>(serial: &mut S, buffer: &[u8]) -> Result<(), S::Error>
where
    S: hal::serial::Write<u8>
{
    for &byte in buffer {
        block!(serial.write(byte))?;
    }

    Ok(())
}

