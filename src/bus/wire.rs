//! The bus's only door to the protocol (research R-4): every exchange builds
//! a `protocol::Link` here, and unicast exchanges are retried here.

use core::marker::PhantomData;

use crate::error::{Error, TransportError, ValidationError};
use crate::protocol::{Clock, Link, Pong, Protocol};
use crate::register::Register;
use crate::{Id, Transport};

use super::Config;

/// The wire as one bus call sees it: the transport, the clock and the
/// config, borrowed from the bus.
pub(crate) struct Wire<'b, P, T, C> {
    transport: &'b mut T,
    clock: &'b mut C,
    config: &'b Config,
    protocol: PhantomData<fn() -> P>,
}

/// Whether a failed attempt may be tried again: only a silent actuator or a
/// corrupted reply. A status error, a refused value or a broken line is
/// final.
fn retryable(err: &Error) -> bool {
    matches!(
        err,
        Error::Transport(TransportError::Timeout { .. }) | Error::Protocol(_)
    )
}

impl<'b, P: Protocol, T: Transport, C: Clock> Wire<'b, P, T, C> {
    /// Borrows the parts of a bus.
    pub(crate) fn new(transport: &'b mut T, clock: &'b mut C, config: &'b Config) -> Self {
        Self {
            transport,
            clock,
            config,
            protocol: PhantomData,
        }
    }

    /// Runs `exchange` with a unicast `Link`, at most `retries` extra times
    /// after a timeout or a corrupted reply.
    fn unicast<R>(
        &mut self,
        mut exchange: impl FnMut(&mut Link<'_, T, C>) -> Result<R, Error>,
    ) -> Result<R, Error> {
        let mut left = self.config.retries;
        loop {
            let mut link = Link {
                transport: &mut *self.transport,
                clock: &mut *self.clock,
                timeout: self.config.timeout,
            };
            match exchange(&mut link) {
                Err(err) if left > 0 && retryable(&err) => left -= 1,
                result => return result,
            }
        }
    }

    /// Pings `id`.
    pub(crate) fn ping(&mut self, id: Id) -> Result<Pong, Error> {
        self.unicast(|link| P::ping(link, id))
    }

    /// Reads `out.len()` bytes at `reg`'s address on `id`.
    pub(crate) fn read(&mut self, id: Id, reg: Register, out: &mut [u8]) -> Result<(), Error> {
        self.unicast(|link| P::read(link, id, reg.addr, out))
    }

    /// Writes `bytes` at `reg`'s address on `id` and waits for the status.
    pub(crate) fn write(&mut self, id: Id, reg: Register, bytes: &[u8]) -> Result<(), Error> {
        self.unicast(|link| P::write(link, id, reg.addr, bytes))
    }

    /// Finds every actuator answering at this baud rate (P2 broadcast ping,
    /// P1 sweep of IDs 0..=252), waiting `scan_timeout` per window. Never
    /// retried.
    pub(crate) fn scan(&mut self, on_reply: impl FnMut(Result<Pong, Error>)) -> Result<(), Error> {
        let mut link = Link {
            transport: &mut *self.transport,
            clock: &mut *self.clock,
            timeout: self.config.scan_timeout,
        };
        P::scan(&mut link, on_reply)
    }

    /// Writes `size` bytes at `addr` on every actuator in `entries` with one
    /// Sync Write. No reply, so never retried; the timeout only bounds the
    /// drain of stale input.
    pub(crate) fn sync_write<'d, I>(&mut self, addr: u16, size: u8, entries: I) -> Result<(), Error>
    where
        I: Iterator<Item = (Id, &'d [u8])> + Clone,
    {
        let mut link = Link {
            transport: &mut *self.transport,
            clock: &mut *self.clock,
            timeout: self.config.timeout,
        };
        P::sync_write(&mut link, addr, u16::from(size), entries)
    }

    /// Reads register `reg` of `id` and decodes it.
    pub(crate) fn read_value(&mut self, id: Id, reg: Register) -> Result<i32, Error> {
        let mut buf = [0; 4];
        let size = usize::from(reg.size);
        let bytes = buf.get_mut(..size).ok_or(ValidationError::DoesNotFit {
            raw: 0,
            size: reg.size,
        })?;
        self.read(id, reg, bytes)?;
        Ok(reg.decode(bytes)?)
    }

    /// Encodes `raw` for register `reg` and writes it to `id`. Nothing is
    /// sent if the value does not fit.
    pub(crate) fn write_value(&mut self, id: Id, reg: Register, raw: i32) -> Result<(), Error> {
        let mut buf = [0; 4];
        let bytes = reg.encode(raw, &mut buf)?;
        self.write(id, reg, bytes)
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use embedded_io::ErrorKind;

    use super::*;
    use crate::bus::testing::*;
    use crate::error::StatusError;
    use crate::protocol::P2;
    use crate::register::{Access, Area};
    use crate::transport::{MockTime, MockTransport};

    const GOAL: Register = Register {
        addr: 116,
        size: 4,
        access: Access::RW,
        area: Area::Ram,
    };

    fn config(retries: u8) -> Config {
        Config::new().retries(retries)
    }

    #[test]
    fn retries_1_timeout_then_reply_is_ok() {
        let time = MockTime::new();
        let mut clock = time.ticker(10);
        let mut line = MockTransport::new(&time);
        line.reply_to(2, &p2_pong(1, XM430, XM430_FW));
        let config = config(1);
        let pong = Wire::<P2, _, _>::new(&mut line, &mut clock, &config)
            .ping(id(1))
            .unwrap();
        assert_eq!(pong.model_number, Some(XM430));
        assert_eq!(line.flushes(), 2);
        let ping = p2_instruction(1, PING, &[]);
        line.assert_sent(&concat(&[ping.clone(), ping]));
        line.assert_done();
    }

    #[test]
    fn retries_1_corrupt_then_reply_is_ok() {
        let time = MockTime::new();
        let mut clock = time.ticker(10);
        let mut line = MockTransport::new(&time);
        let mut corrupt = p2_status(1, 0, &[]);
        let last = corrupt.len() - 1;
        corrupt[last] ^= 0xFF;
        line.reply_to(1, &corrupt)
            .reply_to(2, &p2_status(1, 0, &[]));
        let config = config(1);
        let result =
            Wire::<P2, _, _>::new(&mut line, &mut clock, &config).write_value(id(1), GOAL, 7);
        assert_eq!(result, Ok(()));
        assert_eq!(line.flushes(), 2);
        let write = p2_write(1, 116, &7i32.to_le_bytes());
        line.assert_sent(&concat(&[write.clone(), write]));
    }

    #[test]
    fn status_error_is_not_retried() {
        let time = MockTime::new();
        let mut clock = time.ticker(10);
        let mut line = MockTransport::new(&time);
        line.reply_to(1, &p2_status(1, 0x80, &[]))
            .reply_to(2, &p2_status(1, 0, &[]));
        let config = config(3);
        let result =
            Wire::<P2, _, _>::new(&mut line, &mut clock, &config).write_value(id(1), GOAL, 7);
        assert!(matches!(result, Err(Error::Status(StatusError { .. }))));
        assert_eq!(line.flushes(), 1);
    }

    #[test]
    fn io_error_is_not_retried() {
        let time = MockTime::new();
        let mut clock = time.ticker(10);
        let mut line = MockTransport::new(&time);
        line.fail_with(ErrorKind::BrokenPipe);
        let config = config(3);
        let result = Wire::<P2, _, _>::new(&mut line, &mut clock, &config).ping(id(1));
        assert_eq!(
            result,
            Err(Error::Transport(TransportError::Io(ErrorKind::BrokenPipe)))
        );
        assert_eq!(line.writes() + line.reads() + line.flushes(), 0);
    }

    #[test]
    fn retries_0_sends_once() {
        let time = MockTime::new();
        let mut clock = time.ticker(10);
        let mut line = MockTransport::new(&time);
        let config = config(0);
        let result = Wire::<P2, _, _>::new(&mut line, &mut clock, &config).ping(id(1));
        assert_eq!(
            result,
            Err(Error::Transport(TransportError::Timeout {
                id: Some(id(1))
            }))
        );
        assert_eq!(line.flushes(), 1);
        line.assert_sent(&p2_instruction(1, PING, &[]));
    }

    #[test]
    fn silent_line_is_bounded() {
        let time = MockTime::new();
        let mut clock = time.ticker(10);
        let mut line = MockTransport::new(&time);
        let config = Config::new().retries(1).timeout(Duration::from_millis(10));
        let result = Wire::<P2, _, _>::new(&mut line, &mut clock, &config).read_value(id(1), GOAL);
        assert!(matches!(
            result,
            Err(Error::Transport(TransportError::Timeout { id: Some(_) }))
        ));
        assert_eq!(line.flushes(), 2);
        assert!(time.now() <= 2 * 10_000 + 1_000, "took {} us", time.now());
    }

    #[test]
    fn unicast_uses_config_timeout() {
        let time = MockTime::new();
        let mut clock = time.ticker(10);
        let mut line = MockTransport::new(&time);
        line.reply_at(4_000, &p2_status(1, 0, &[]));
        let short = Config::new().timeout(Duration::from_millis(3));
        let result = Wire::<P2, _, _>::new(&mut line, &mut clock, &short).ping(id(1));
        assert!(matches!(
            result,
            Err(Error::Transport(TransportError::Timeout { .. }))
        ));

        let time = MockTime::new();
        let mut clock = time.ticker(10);
        let mut line = MockTransport::new(&time);
        line.reply_at(4_000, &p2_pong(1, XM430, XM430_FW));
        let long = Config::new().timeout(Duration::from_millis(5));
        let pong = Wire::<P2, _, _>::new(&mut line, &mut clock, &long)
            .ping(id(1))
            .unwrap();
        assert_eq!(pong.model_number, Some(XM430));
    }

    #[test]
    fn read_value_decodes_and_write_value_refuses_what_does_not_fit() {
        let time = MockTime::new();
        let mut clock = time.ticker(10);
        let mut line = MockTransport::new(&time);
        line.reply(&p2_status(1, 0, &(-5i32).to_le_bytes()));
        let config = Config::new();
        let mut wire = Wire::<P2, _, _>::new(&mut line, &mut clock, &config);
        assert_eq!(wire.read_value(id(1), GOAL), Ok(-5));
        let byte = Register { size: 1, ..GOAL };
        assert_eq!(
            wire.write_value(id(1), byte, 256),
            Err(Error::Validation(ValidationError::DoesNotFit {
                raw: 256,
                size: 1
            }))
        );
        line.assert_sent(&p2_read(1, 116, 4));
    }
}
