//! The bus's only door to the protocol (research R-4): every exchange builds
//! a `protocol::Link` here, and unicast exchanges are retried here.

use core::marker::PhantomData;

use crate::error::{Error, Instruction, Request, TransportError};
use crate::protocol::{Clock, Link, P2, Pong, Protocol, late_bytes};
use crate::register::Register;
use crate::{Id, Transport};

use super::Config;

/// The wire as one bus call sees it: the transport, the clock and the
/// config, borrowed from the bus.
pub(crate) struct Wire<'b, P, T, C> {
    transport: &'b mut T,
    clock: &'b mut C,
    config: &'b Config,
    /// What the bus call is about (register, setup step), merged into the
    /// request of every error.
    context: Request,
    protocol: PhantomData<fn() -> P>,
}

/// Whether a failed attempt may be tried again: only a silent actuator or a
/// corrupted reply. A status error, a refused value or a broken line is
/// final.
fn retryable(err: &Error) -> bool {
    match err {
        Error::Transport(err) => err.is_timeout(),
        Error::Protocol(_) => true,
        _ => false,
    }
}

impl<P, T, C> Wire<'_, P, T, C> {
    /// Sets what the following exchanges are about, e.g. the register of a
    /// multi-actuator read or a setup step; it fills the request of their
    /// errors.
    pub(crate) fn set_context(&mut self, context: Request) {
        self.context = context;
    }

    /// The current context.
    pub(crate) fn context(&self) -> Request {
        self.context
    }
}

impl<'b, P: Protocol, T: Transport, C: Clock> Wire<'b, P, T, C> {
    /// Borrows the parts of a bus.
    pub(crate) fn new(transport: &'b mut T, clock: &'b mut C, config: &'b Config) -> Self {
        Self {
            transport,
            clock,
            config,
            context: Request::NONE,
            protocol: PhantomData,
        }
    }

    /// Runs `exchange` with a unicast `Link` whose request is the context
    /// filled from `request`, at most `retries` extra times after a timeout
    /// or a corrupted reply.
    ///
    /// When the last attempt got no byte at all, the line is watched for
    /// one more timeout: bytes arriving then turn the error into
    /// [`TransportError::LateReply`].
    fn unicast<R>(
        &mut self,
        request: Request,
        mut exchange: impl FnMut(&mut Link<'_, T, C>) -> Result<R, Error>,
    ) -> Result<R, Error> {
        let mut left = self.config.retries;
        let request = self.context.filled(request);
        loop {
            let mut link = self.link_with(self.config.timeout, request);
            match exchange(&mut link) {
                Err(err) if left > 0 && retryable(&err) => left -= 1,
                Err(Error::Transport(TransportError::NoReply {
                    request,
                    timeout_us,
                })) => {
                    let bytes = late_bytes(&mut link)?;
                    return Err(if bytes > 0 {
                        TransportError::LateReply {
                            request,
                            timeout_us,
                            bytes,
                        }
                    } else {
                        TransportError::NoReply {
                            request,
                            timeout_us,
                        }
                    }
                    .into());
                }
                result => return result,
            }
        }
    }

    /// Pings `id`.
    pub(crate) fn ping(&mut self, id: Id) -> Result<Pong, Error> {
        self.unicast(Request::NONE, |link| P::ping(link, id))
    }

    /// Reads `out.len()` bytes at `reg`'s address on `id`.
    pub(crate) fn read(&mut self, id: Id, reg: Register, out: &mut [u8]) -> Result<(), Error> {
        self.unicast(reg.request(), |link| P::read(link, id, reg.addr, out))
    }

    /// Writes `bytes` at `reg`'s address on `id` and waits for the status.
    pub(crate) fn write(&mut self, id: Id, reg: Register, bytes: &[u8]) -> Result<(), Error> {
        self.unicast(reg.request(), |link| P::write(link, id, reg.addr, bytes))
    }

    /// Reads `out.len()` bytes at `addr` on `id`, whatever register is
    /// there.
    #[cfg(feature = "raw-registers")]
    pub(crate) fn read_at(&mut self, id: Id, addr: u16, out: &mut [u8]) -> Result<(), Error> {
        self.unicast(Request::NONE, |link| P::read(link, id, addr, out))
    }

    /// Writes `bytes` at `addr` on `id`, whatever register is there, and
    /// waits for the status.
    #[cfg(feature = "raw-registers")]
    pub(crate) fn write_at(&mut self, id: Id, addr: u16, bytes: &[u8]) -> Result<(), Error> {
        self.unicast(Request::NONE, |link| P::write(link, id, addr, bytes))
    }

    /// Finds every actuator answering at this baud rate (P2 broadcast ping,
    /// P1 sweep of IDs 0..=252), waiting `scan_timeout` per window. Never
    /// retried.
    pub(crate) fn scan(&mut self, on_reply: impl FnMut(Result<Pong, Error>)) -> Result<(), Error> {
        let mut link = self.link_with(self.config.scan_timeout, self.context);
        P::scan(&mut link, on_reply)
    }

    /// Writes `size` bytes at `addr` on every actuator in `entries` with one
    /// Sync Write. No reply, so never retried; the timeout only bounds the
    /// drain of stale input.
    pub(crate) fn sync_write<'d, I>(&mut self, addr: u16, size: u8, entries: I) -> Result<(), Error>
    where
        I: Iterator<Item = (Id, &'d [u8])> + Clone,
    {
        let mut link = self.link();
        P::sync_write(&mut link, addr, u16::from(size), entries)
    }

    /// Reads `size` bytes at `addr` from every ID in `ids` (P2 one Sync
    /// Read, P1 one Read per ID). `on_reply` gets each ID's data or error,
    /// in `ids` order. Never retried; `Err` only for an invalid request or
    /// a failing transport.
    pub(crate) fn read_many<I, F>(
        &mut self,
        addr: u16,
        size: u8,
        ids: I,
        scratch: &mut [u8],
        on_reply: F,
    ) -> Result<(), Error>
    where
        I: Iterator<Item = Id> + Clone,
        F: FnMut(Id, Result<&[u8], Error>),
    {
        let mut link = self.link();
        P::read_many(&mut link, addr, u16::from(size), ids, scratch, on_reply)
    }

    /// Reads each `(id, addr, size)` entry with one Bulk Read: `on_reply`
    /// gets each ID's data or error, in entry order. Never retried; `Err`
    /// only for an invalid request or a failing transport.
    pub(crate) fn bulk_read<I, F>(
        &mut self,
        entries: I,
        scratch: &mut [u8],
        on_reply: F,
    ) -> Result<(), Error>
    where
        I: Iterator<Item = (Id, u16, u16)> + Clone,
        F: FnMut(Id, Result<&[u8], Error>),
    {
        let mut link = self.link();
        P::bulk_read(&mut link, entries, scratch, on_reply)
    }

    /// A `Link` with the unicast timeout and the context as its request.
    fn link(&mut self) -> Link<'_, T, C> {
        self.link_with(self.config.timeout, self.context)
    }

    /// A `Link` with `timeout` and `request`.
    fn link_with(&mut self, timeout: core::time::Duration, request: Request) -> Link<'_, T, C> {
        Link {
            transport: &mut *self.transport,
            clock: &mut *self.clock,
            timeout,
            request,
        }
    }

    /// Reads register `reg` of `id` and decodes it.
    pub(crate) fn read_value(&mut self, id: Id, reg: Register) -> Result<i32, Error> {
        let mut buf = [0; 4];
        let request = self
            .context
            .filled(reg.request().instruction(Instruction::Read).id(id));
        let bytes = reg.buf(&mut buf).map_err(|e| e.within(request))?;
        self.read(id, reg, bytes)?;
        reg.decode(bytes).map_err(|e| e.within(request).into())
    }

    /// Encodes `raw` for register `reg` and writes it to `id`. Nothing is
    /// sent if the value does not fit.
    pub(crate) fn write_value(&mut self, id: Id, reg: Register, raw: i32) -> Result<(), Error> {
        let mut buf = [0; 4];
        let request = self
            .context
            .filled(reg.request().instruction(Instruction::Write).id(id));
        let bytes = reg.encode(raw, &mut buf).map_err(|e| e.within(request))?;
        self.write(id, reg, bytes)
    }
}

impl<T: Transport, C: Clock> Wire<'_, P2, T, C> {
    /// Writes each `(id, addr, data)` entry with one Bulk Write. No reply,
    /// so never retried.
    pub(crate) fn bulk_write<'d, I>(&mut self, entries: I) -> Result<(), Error>
    where
        I: Iterator<Item = (Id, u16, &'d [u8])> + Clone,
    {
        let mut link = self.link();
        P2::bulk_write(&mut link, entries)
    }

    /// Reboots `id` and waits for its status. Never retried: a reply lost
    /// after the actuator took the instruction would reboot it twice.
    pub(crate) fn reboot(&mut self, id: Id) -> Result<(), Error> {
        let mut link = self.link();
        P2::reboot(&mut link, id)
    }

    /// One Sync Read of `size` bytes at `addr` from every ID in `ids`, as
    /// [`read_many`](Wire::read_many) on Protocol 2. Never retried.
    pub(crate) fn sync_read<I, F>(
        &mut self,
        addr: u16,
        size: u8,
        ids: I,
        scratch: &mut [u8],
        on_reply: F,
    ) -> Result<(), Error>
    where
        I: Iterator<Item = Id> + Clone,
        F: FnMut(Id, Result<&[u8], Error>),
    {
        let mut link = self.link();
        P2::sync_read(&mut link, addr, u16::from(size), ids, scratch, on_reply)
    }
}

/// How a multi-actuator read reaches the wire: [`AnyProtocol`] for the Easy
/// tier, [`SyncRead`] for the Protocol 2 explicit tier.
pub(crate) trait ReadMany<P, T, C> {
    /// Reads `size` bytes at `addr` from every ID in `ids`, as
    /// [`Wire::read_many`].
    fn run<I, F>(
        wire: &mut Wire<'_, P, T, C>,
        addr: u16,
        size: u8,
        ids: I,
        scratch: &mut [u8],
        on_reply: F,
    ) -> Result<(), Error>
    where
        I: Iterator<Item = Id> + Clone,
        F: FnMut(Id, Result<&[u8], Error>);
}

/// [`Wire::read_many`]: one Sync Read on Protocol 2, one Read per ID on
/// Protocol 1.
pub(crate) struct AnyProtocol;

impl<P: Protocol, T: Transport, C: Clock> ReadMany<P, T, C> for AnyProtocol {
    fn run<I, F>(
        wire: &mut Wire<'_, P, T, C>,
        addr: u16,
        size: u8,
        ids: I,
        scratch: &mut [u8],
        on_reply: F,
    ) -> Result<(), Error>
    where
        I: Iterator<Item = Id> + Clone,
        F: FnMut(Id, Result<&[u8], Error>),
    {
        wire.read_many(addr, size, ids, scratch, on_reply)
    }
}

/// [`Wire::sync_read`]: Protocol 2 only.
pub(crate) struct SyncRead;

impl<T: Transport, C: Clock> ReadMany<P2, T, C> for SyncRead {
    fn run<I, F>(
        wire: &mut Wire<'_, P2, T, C>,
        addr: u16,
        size: u8,
        ids: I,
        scratch: &mut [u8],
        on_reply: F,
    ) -> Result<(), Error>
    where
        I: Iterator<Item = Id> + Clone,
        F: FnMut(Id, Result<&[u8], Error>),
    {
        wire.sync_read(addr, size, ids, scratch, on_reply)
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use embedded_io::ErrorKind;

    use super::*;
    use crate::bus::testing::*;
    use crate::error::{Direction, StatusError, ValidationError};
    use crate::register::{Access, Area};
    use crate::transport::{MockTime, MockTransport};

    const GOAL: Register = Register {
        name: "Goal Position",
        addr: 116,
        size: 4,
        access: Access::RW,
        area: Area::Ram,
    };

    fn ping_request(n: u8) -> Request {
        Request::NONE.instruction(Instruction::Ping).id(id(n))
    }

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
            Err(Error::Transport(TransportError::Io {
                request: ping_request(1),
                direction: Direction::ReadReady,
                kind: ErrorKind::BrokenPipe,
            }))
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
            Err(Error::Transport(TransportError::NoReply {
                request: ping_request(1),
                timeout_us: 10_000,
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
        assert_eq!(
            result,
            Err(Error::Transport(TransportError::NoReply {
                request: Request::NONE
                    .instruction(Instruction::Read)
                    .id(id(1))
                    .register("Goal Position")
                    .addr(116)
                    .len(4),
                timeout_us: 10_000,
            }))
        );
        assert_eq!(line.flushes(), 2);
        assert!(time.now() <= 3 * 10_000 + 1_000, "took {} us", time.now());
    }

    #[test]
    fn unicast_uses_config_timeout() {
        let time = MockTime::new();
        let mut clock = time.ticker(10);
        let mut line = MockTransport::new(&time);
        line.reply_at(4_000, &p2_status(1, 0, &[]));
        let short = Config::new().timeout(Duration::from_millis(3));
        let result = Wire::<P2, _, _>::new(&mut line, &mut clock, &short).ping(id(1));
        let status_len = p2_status(1, 0, &[]).len();
        assert_eq!(
            result,
            Err(Error::Transport(TransportError::LateReply {
                request: ping_request(1),
                timeout_us: 3_000,
                bytes: status_len,
            }))
        );

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
                size: 1,
                request: Request::NONE
                    .instruction(Instruction::Write)
                    .id(id(1))
                    .register("Goal Position")
                    .addr(116)
                    .len(1),
            }))
        );
        line.assert_sent(&p2_read(1, 116, 4));
    }
}
