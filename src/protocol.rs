//! Dynamixel Protocol 1 and Protocol 2: packet encoding, status parsing, and
//! the send/receive rules every exchange follows.
//!
//! Every packet layout, instruction code and error-byte meaning comes from
//! the ROBOTIS e-Manual:
//! - Protocol 1: <https://emanual.robotis.com/docs/en/dxl/protocol1/>
//! - Protocol 2: <https://emanual.robotis.com/docs/en/dxl/protocol2/>
//!
//! Packets are streamed: nothing here holds a buffer sized by the number of
//! actuators, and nothing allocates.

mod clock;
mod crc;
mod p1;
mod p2;

#[cfg(test)]
mod no_alloc;
#[cfg(test)]
pub(crate) mod vectors;

pub use clock::Clock;
#[cfg(feature = "std")]
pub use clock::StdClock;
pub use p1::P1;
pub use p2::P2;

use core::fmt;
use core::time::Duration;

use embedded_io::{ErrorKind, Write};

use crate::error::{
    Direction, Error, ProtocolError, Request, StatusError, StatusReport, TransportError,
};
use crate::{Id, Transport};

mod sealed {
    pub trait Sealed {}
}

/// A Dynamixel protocol: [`P1`] or [`P2`].
///
/// Fixed at compile time and shared by every actuator on one wire. Sealed:
/// only this crate implements it.
pub trait Protocol: sealed::Sealed + Exchange {
    /// The broadcast ID, which addresses every actuator at once.
    const BROADCAST: u8 = 0xFE;

    /// Which protocol this is, as a value.
    const VERSION: ProtocolVersion;
}

/// A Dynamixel protocol as a value, for messages and runtime checks.
///
/// [`Protocol::VERSION`] maps the compile-time [`P1`] / [`P2`] to it.
///
/// ```
/// use dxl_rs_v2::{P2, Protocol, ProtocolVersion};
///
/// assert_eq!(P2::VERSION, ProtocolVersion::P2);
/// assert_eq!(P2::VERSION.to_string(), "Protocol 2");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProtocolVersion {
    /// Dynamixel Protocol 1.0.
    P1,
    /// Dynamixel Protocol 2.0.
    P2,
}

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::P1 => "Protocol 1",
            Self::P2 => "Protocol 2",
        })
    }
}

/// One wire, as seen by a single exchange.
///
/// `Link`, [`Pong`] and [`Exchange`] are `pub` only so that [`Protocol`] can
/// carry `Exchange` as a supertrait: this module is private, so none of them
/// can be named outside the crate.
pub struct Link<'a, T, C> {
    pub(crate) transport: &'a mut T,
    pub(crate) clock: &'a mut C,
    /// How long to wait for each expected reply.
    pub(crate) timeout: Duration,
    /// What the current exchange is doing, for its errors. The bus fills
    /// the register and setup step; each exchange adds the instruction,
    /// target, address and length.
    pub(crate) request: Request,
}

impl<T: Transport, C: Clock> Link<'_, T, C> {
    /// The instant, in clock microseconds, at which a wait starting now ends.
    pub fn deadline(&mut self) -> u64 {
        let timeout = u64::try_from(self.timeout.as_micros()).unwrap_or(u64::MAX);
        self.clock.now_us().saturating_add(timeout)
    }

    /// The timeout in microseconds, for errors.
    pub(crate) fn timeout_us(&self) -> u32 {
        u32::try_from(self.timeout.as_micros()).unwrap_or(u32::MAX)
    }

    /// Runs one exchange with `request` filling what the bus did not set,
    /// then restores the bus's request for the next exchange.
    pub(crate) fn with<R>(&mut self, request: Request, exchange: impl FnOnce(&mut Self) -> R) -> R {
        let base = self.request;
        self.request.fill(request);
        let result = exchange(self);
        self.request = base;
        result
    }
}

/// An actuator's answer to Ping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pong {
    pub(crate) id: Id,
    /// Protocol 2 only; Protocol 1 Ping replies carry no data.
    pub(crate) model_number: Option<u16>,
    /// Protocol 2 only.
    pub(crate) firmware: Option<u8>,
    /// The error byte of a scan reply that had one set.
    /// Unicast ping reports a set error byte as [`Error::Status`] instead,
    /// so this is always `None` there.
    pub(crate) fault: Option<StatusReport>,
}

/// The instructions both protocols support.
///
/// Instructions only one protocol has are inherent functions on that
/// protocol's type, so they do not exist for the other.
pub trait Exchange: sealed::Sealed {
    /// Pings `id` and returns its answer.
    fn ping<T: Transport, C: Clock>(link: &mut Link<'_, T, C>, id: Id) -> Result<Pong, Error>;

    /// Reads `out.len()` bytes from `addr` on `id` into `out`.
    ///
    /// On [`Error::Status`], `out` still holds the reply's data when the
    /// reply carried `out.len()` bytes, as one with only the P2 alert bit
    /// set does; that is how Hardware Error Status is read during an alert.
    /// A reply of any other length leaves `out` zeroed.
    fn read<T: Transport, C: Clock>(
        link: &mut Link<'_, T, C>,
        id: Id,
        addr: u16,
        out: &mut [u8],
    ) -> Result<(), Error>;

    /// Writes `data` at `addr` on `id` and waits for its status packet.
    fn write<T: Transport, C: Clock>(
        link: &mut Link<'_, T, C>,
        id: Id,
        addr: u16,
        data: &[u8],
    ) -> Result<(), Error>;

    /// Writes `size` bytes at `addr` on every actuator in `entries` with one
    /// broadcast instruction. Never waits for a reply.
    fn sync_write<'d, T: Transport, C: Clock, I>(
        link: &mut Link<'_, T, C>,
        addr: u16,
        size: u16,
        entries: I,
    ) -> Result<(), Error>
    where
        I: Iterator<Item = (Id, &'d [u8])> + Clone;

    /// Finds every actuator answering at this baud rate and calls
    /// `on_reply` for each answer.
    ///
    /// P2 sends one broadcast Ping and collects replies for one
    /// `link.timeout`. P1 pings IDs `0..=252` in turn, each bounded by
    /// `link.timeout`; a silent ID is skipped, not reported. A reply whose
    /// error byte is set is still a [`Pong`], with the error in
    /// [`Pong::fault`]. A corrupted reply (bad checksum or CRC, malformed,
    /// unexpected ID or length) is passed on as `Err`, not skipped.
    /// Returns `Err` only when the transport fails.
    fn scan<T: Transport, C: Clock, F>(link: &mut Link<'_, T, C>, on_reply: F) -> Result<(), Error>
    where
        F: FnMut(Result<Pong, Error>);

    /// Reads `size` bytes at `addr` from every ID in `ids`.
    ///
    /// P2 sends one Sync Read (see `P2::sync_read`). P1 sends one Read per
    /// ID, in order, each bounded by `link.timeout`. `on_reply` is called
    /// exactly once per ID, in `ids` order, with that actuator's data or
    /// its own error (timeout, status error, corrupted reply). `scratch`
    /// holds one reply and must be at least `size` bytes.
    ///
    /// Returns `Err` when the request is invalid, before anything is sent
    /// (`scratch` too small, `size` or, on P1, `addr` out of range), or
    /// when the transport fails.
    fn read_many<T: Transport, C: Clock, I, F>(
        link: &mut Link<'_, T, C>,
        addr: u16,
        size: u16,
        ids: I,
        scratch: &mut [u8],
        on_reply: F,
    ) -> Result<(), Error>
    where
        I: Iterator<Item = Id> + Clone,
        F: FnMut(Id, Result<&[u8], Error>);

    /// Reads `size` bytes at `addr` from each `(id, addr, size)` entry with
    /// one Bulk Read instruction: addresses and sizes may differ per
    /// actuator. `on_reply` is called exactly once per entry, in entry
    /// order, with that actuator's data or its own error. `scratch` holds
    /// one reply and must be at least as long as the largest `size`.
    ///
    /// Protocol 1 has it on MX, MX(2.0) and X series firmware except
    /// XL-320; the caller checks each target's table.
    ///
    /// Returns `Err` when the request is invalid, before anything is sent
    /// (an ID listed twice, a size or address out of range), or when the
    /// transport fails.
    fn bulk_read<T: Transport, C: Clock, I, F>(
        link: &mut Link<'_, T, C>,
        entries: I,
        scratch: &mut [u8],
        on_reply: F,
    ) -> Result<(), Error>
    where
        I: Iterator<Item = (Id, u16, u16)> + Clone,
        F: FnMut(Id, Result<&[u8], Error>);

    /// Decodes a non-zero error byte of this protocol.
    fn report(error: u8) -> StatusReport;
}

/// The fields of a parsed, integrity-checked status packet. Its data went
/// into the caller's buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Status {
    /// Raw ID byte, not yet validated as an [`Id`].
    pub(crate) id: u8,
    pub(crate) error: u8,
    /// Number of data bytes in the packet. Only the first `out.len()` were
    /// stored.
    pub(crate) len: usize,
}

/// A byte-at-a-time status packet parser.
pub(crate) trait Parser {
    /// How many bytes may be read next without reading past the end of the
    /// packet being parsed. Always at least 1.
    fn wanted(&self) -> usize;

    /// Feeds one byte. Returns `Some` when a packet is complete or proven
    /// bad.
    fn push(&mut self, byte: u8) -> Option<Result<Status, Defect>>;

    /// The defect to report instead of a timeout when the deadline passes,
    /// e.g. because only malformed headers were seen.
    fn on_deadline(&self) -> Option<Defect> {
        None
    }
}

/// What made received bytes fail to be a status packet; it becomes a
/// [`ProtocolError`] once the request is attached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Defect {
    /// Protocol 1 checksum mismatch.
    Checksum {
        packet_id: u8,
        computed: u8,
        received: u8,
    },
    /// Protocol 2 CRC mismatch.
    Crc {
        packet_id: u8,
        computed: u16,
        received: u16,
    },
    /// A length field below the protocol's minimum.
    TooShort {
        protocol: ProtocolVersion,
        length: u16,
        min: u16,
    },
    /// A Protocol 2 length field above what the expected reply can be.
    TooLong { length: u16, max: u16 },
}

impl Defect {
    /// The error for this defect in a reply to `request`.
    pub(crate) fn error(self, request: Request) -> ProtocolError {
        match self {
            Self::Checksum {
                packet_id,
                computed,
                received,
            } => ProtocolError::Checksum {
                request,
                packet_id,
                computed,
                received,
            },
            Self::Crc {
                packet_id,
                computed,
                received,
            } => ProtocolError::Crc {
                request,
                packet_id,
                computed,
                received,
            },
            Self::TooShort {
                protocol,
                length,
                min,
            } => ProtocolError::LengthTooShort {
                request,
                protocol,
                length,
                min,
            },
            Self::TooLong { length, max } => ProtocolError::LengthTooLong {
                request,
                length,
                max,
            },
        }
    }
}

/// Size of the stack buffer used for reading and writing.
pub(crate) const CHUNK: usize = 32;

/// Maps a transport error from `direction` during `request`; the
/// transport's own timeout is a [`TransportError::PortTimeout`].
pub(crate) fn io_error<E: embedded_io::Error>(
    err: E,
    request: Request,
    direction: Direction,
) -> Error {
    match err.kind() {
        ErrorKind::TimedOut => TransportError::PortTimeout { request, direction }.into(),
        kind => TransportError::Io {
            request,
            direction,
            kind,
        }
        .into(),
    }
}

/// Reads whatever is waiting into `chunk`; `Ok(0)` when nothing is.
/// A read of zero bytes after `read_ready` is the end of the stream.
fn read_waiting<T: Transport, C: Clock>(
    link: &mut Link<'_, T, C>,
    chunk: &mut [u8],
) -> Result<usize, Error> {
    let request = link.request;
    if !link
        .transport
        .read_ready()
        .map_err(|e| io_error(e, request, Direction::ReadReady))?
    {
        return Ok(0);
    }
    match link
        .transport
        .read(chunk)
        .map_err(|e| io_error(e, request, Direction::Read))?
    {
        0 => Err(TransportError::EndOfStream { request }.into()),
        n => Ok(n),
    }
}

/// Discards whatever is waiting on the receive side, so stale bytes are
/// never taken for the reply to the next instruction. Gives up after one
/// timeout if the line keeps talking.
pub(crate) fn drain<T: Transport, C: Clock>(link: &mut Link<'_, T, C>) -> Result<(), Error> {
    let deadline = link.deadline();
    let mut chunk = [0u8; CHUNK];
    while link.clock.now_us() < deadline {
        if read_waiting(link, &mut chunk)? == 0 {
            break;
        }
    }
    Ok(())
}

/// Counts the bytes that arrive within one more timeout: after a wait that
/// got nothing, they are the late reply. The bytes are discarded.
pub(crate) fn late_bytes<T: Transport, C: Clock>(
    link: &mut Link<'_, T, C>,
) -> Result<usize, Error> {
    let deadline = link.deadline();
    let mut chunk = [0u8; CHUNK];
    let mut bytes = 0;
    while link.clock.now_us() < deadline {
        bytes += read_waiting(link, &mut chunk)?;
    }
    Ok(bytes)
}

/// Waits until `parser` completes a packet or `deadline` passes.
///
/// Reads only as many bytes as the parser wants, so bytes of the next packet
/// stay on the line for the next call.
///
/// At the deadline: the defect of a skipped malformed header if there was
/// one, else [`TransportError::PartialReply`] when bytes arrived and
/// [`TransportError::NoReply`] when none did.
pub(crate) fn receive<T: Transport, C: Clock, P: Parser>(
    link: &mut Link<'_, T, C>,
    parser: &mut P,
    deadline: u64,
) -> Result<Status, Error> {
    let mut chunk = [0u8; CHUNK];
    let mut bytes = 0;
    loop {
        if link.clock.now_us() >= deadline {
            let (request, timeout_us) = (link.request, link.timeout_us());
            return Err(match parser.on_deadline() {
                Some(defect) => defect.error(request).into(),
                None if bytes > 0 => TransportError::PartialReply {
                    request,
                    timeout_us,
                    bytes,
                }
                .into(),
                None => TransportError::NoReply {
                    request,
                    timeout_us,
                }
                .into(),
            });
        }
        let want = parser.wanted().clamp(1, CHUNK);
        let n = read_waiting(link, &mut chunk[..want])?.min(want);
        bytes += n;
        for &byte in &chunk[..n] {
            if let Some(result) = parser.push(byte) {
                return result.map_err(|defect| defect.error(link.request).into());
            }
        }
    }
}

/// Checks that a status packet parsed into `out` answers `id`, reports no
/// error and carries `out.len()` data bytes.
///
/// A set error byte wins over a wrong length, so a rejected instruction is
/// never reported as a malformed reply. The data of an error reply stays in
/// `out` only when it has the expected length; otherwise `out` is zeroed so
/// no partial reply is left behind.
pub(crate) fn check<P: Exchange>(
    status: Status,
    id: Id,
    out: &mut [u8],
    request: Request,
) -> Result<(), Error> {
    let len = out.len();
    if status.id != id.get() {
        return Err(ProtocolError::WrongId {
            request,
            expected: id,
            got: status.id,
        }
        .into());
    }
    if status.error != 0 {
        if status.len != len {
            out.fill(0);
        }
        return Err(StatusError {
            id,
            report: P::report(status.error),
            request,
        }
        .into());
    }
    if status.len != len {
        return Err(ProtocolError::WrongLength {
            request,
            expected: len,
            got: status.len,
        }
        .into());
    }
    Ok(())
}

/// Receives one status packet of this protocol.
pub(crate) trait Receive: Exchange {
    /// Waits until a status packet completes or `deadline` passes, storing
    /// at most `out.len()` of its data bytes into `out`.
    fn receive_status<T: Transport, C: Clock>(
        link: &mut Link<'_, T, C>,
        out: &mut [u8],
        deadline: u64,
    ) -> Result<Status, Error>;
}

/// Collects the replies to one broadcast read instruction (Sync Read, Bulk
/// Read): one status packet per `(id, addr, size)` entry, in entry order, as
/// the e-Manual says they come back.
///
/// `on_reply` is called exactly once per entry with that actuator's `size`
/// data bytes, or its own error; each reply gets a full timeout. A reply
/// from an ID later in the list means the IDs before it are not answering:
/// they get [`TransportError::Skipped`] and the reply goes to its own ID. A
/// late reply from an ID that already timed out is discarded. `scratch`
/// holds one reply and must be at least as long as the largest `size`.
/// Each error's request is `link.request` narrowed to its entry. Returns
/// `Err` only when the transport itself fails.
pub(crate) fn collect<P, T, C, I, F>(
    link: &mut Link<'_, T, C>,
    entries: I,
    scratch: &mut [u8],
    mut on_reply: F,
) -> Result<(), Error>
where
    P: Receive,
    T: Transport,
    C: Clock,
    I: Iterator<Item = (Id, u16, usize)> + Clone,
    F: FnMut(Id, Result<&[u8], Error>),
{
    let base = link.request;
    let all = entries.clone();
    let mut held = None;
    let mut pending = entries;
    let mut done = 0;
    while let Some((id, addr, size)) = pending.next() {
        let request = Request {
            id: Some(id),
            addr: Some(addr),
            len: Some(Request::len_of(size)),
            ..base
        };
        link.request = request;
        let already_handled = |raw: u8| {
            all.clone()
                .take(done)
                .any(|(earlier, _, _)| earlier != id && earlier.get() == raw)
        };
        let received = match held.take() {
            Some(status) => Ok(status),
            None => {
                let deadline = link.deadline();
                loop {
                    match P::receive_status(link, scratch, deadline) {
                        Ok(status) if already_handled(status.id) => {}
                        other => break other,
                    }
                }
            }
        };
        done += 1;
        match received {
            Ok(status)
                if status.id != id.get()
                    && pending
                        .clone()
                        .any(|(later, _, _)| later.get() == status.id) =>
            {
                held = Some(status);
                let answered = Id::new(status.id).unwrap_or(id);
                on_reply(
                    id,
                    Err(TransportError::Skipped { request, answered }.into()),
                );
            }
            Ok(status) => match scratch.get_mut(..size) {
                Some(out) => on_reply(id, check::<P>(status, id, out, request).map(|()| &*out)),
                None => on_reply(
                    id,
                    Err(crate::error::ValidationError::DataLength {
                        expected: size,
                        got: scratch.len(),
                        request,
                    }
                    .into()),
                ),
            },
            Err(Error::Transport(err)) if !err.is_timeout() => {
                link.request = base;
                return Err(err.into());
            }
            Err(err) => on_reply(id, Err(err)),
        }
    }
    link.request = base;
    Ok(())
}

/// The first ID listed twice in `ids`, if any. Bulk instructions take each
/// ID once (e-Manual Protocol 2.0, Bulk Read and Bulk Write: "The same ID
/// cannot be used multiple times in the Parameter").
pub(crate) fn repeated_id<I: Iterator<Item = Id> + Clone>(ids: I) -> Option<Id> {
    ids.clone()
        .enumerate()
        .find(|&(i, id)| ids.clone().take(i).any(|earlier| earlier == id))
        .map(|(_, id)| id)
}

/// Buffers outgoing bytes in a small stack chunk.
pub(crate) struct Writer<'t, T: Write> {
    transport: &'t mut T,
    buf: [u8; CHUNK],
    len: usize,
    /// What is being sent, for errors.
    request: Request,
}

impl<'t, T: Write> Writer<'t, T> {
    pub(crate) fn new(transport: &'t mut T, request: Request) -> Self {
        Self {
            transport,
            buf: [0; CHUNK],
            len: 0,
            request,
        }
    }

    /// Queues one byte, writing the chunk out when it is full.
    pub(crate) fn put(&mut self, byte: u8) -> Result<(), Error> {
        if self.len == CHUNK {
            self.write_buf()?;
        }
        self.buf[self.len] = byte;
        self.len += 1;
        Ok(())
    }

    /// Writes out what is queued and flushes the transport.
    pub(crate) fn finish(mut self) -> Result<(), Error> {
        self.write_buf()?;
        let request = self.request;
        self.transport
            .flush()
            .map_err(|e| io_error(e, request, Direction::Flush))
    }

    fn write_buf(&mut self) -> Result<(), Error> {
        let request = self.request;
        self.transport
            .write_all(&self.buf[..self.len])
            .map_err(|e| io_error(e, request, Direction::Write))?;
        self.len = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use crate::error::Instruction;
    use crate::transport::{MockTime, MockTransport};

    /// Accepts any `n` bytes as a packet with ID 1.
    struct Take(usize);

    impl Parser for Take {
        fn wanted(&self) -> usize {
            self.0
        }

        fn push(&mut self, _: u8) -> Option<Result<Status, Defect>> {
            self.0 -= 1;
            (self.0 == 0).then_some(Ok(Status {
                id: 1,
                error: 0,
                len: 0,
            }))
        }
    }

    fn id(raw: u8) -> Id {
        Id::new(raw).unwrap()
    }

    /// Bus code bounded only by `P: Protocol` reaches every exchange.
    #[test]
    fn protocol_bound_reaches_the_exchanges() {
        fn ping<P: Protocol>(line: &mut MockTransport<'_>, time: &MockTime) -> Result<Pong, Error> {
            let mut clock = time.ticker(1);
            P::ping(
                &mut Link {
                    transport: line,
                    clock: &mut clock,
                    timeout: Duration::from_millis(1),
                    request: Request::NONE,
                },
                id(1),
            )
        }
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        line.reply(vectors::p1::OK_REPLY);
        assert_eq!(ping::<P1>(&mut line, &time).map(|pong| pong.id), Ok(id(1)));
        let mut line = MockTransport::new(&time);
        line.reply(vectors::p2::PING_REPLY);
        assert_eq!(ping::<P2>(&mut line, &time).map(|pong| pong.id), Ok(id(1)));
    }

    #[test]
    fn version_names_each_protocol() {
        assert_eq!(P1::VERSION, ProtocolVersion::P1);
        assert_eq!(P2::VERSION, ProtocolVersion::P2);
        assert_eq!(std::format!("{}", P1::VERSION), "Protocol 1");
        assert_eq!(std::format!("{}", P2::VERSION), "Protocol 2");
    }

    #[test]
    fn receive_times_out_on_a_silent_line() {
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        let mut clock = time.ticker(100);
        let mut link = Link {
            transport: &mut line,
            clock: &mut clock,
            timeout: Duration::from_millis(1),
            request: Request::NONE,
        };
        let deadline = link.deadline();
        assert_eq!(
            receive(&mut link, &mut Take(3), deadline),
            Err(TransportError::NoReply {
                request: Request::NONE,
                timeout_us: 1_000,
            }
            .into())
        );
    }

    #[test]
    fn receive_counts_the_bytes_of_a_cut_reply() {
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        line.stale(&[1, 2]);
        let mut clock = time.ticker(100);
        let request = Request::NONE.instruction(Instruction::Read).id(id(4));
        let mut link = Link {
            transport: &mut line,
            clock: &mut clock,
            timeout: Duration::from_millis(1),
            request,
        };
        let deadline = link.deadline();
        assert_eq!(
            receive(&mut link, &mut Take(3), deadline),
            Err(TransportError::PartialReply {
                request,
                timeout_us: 1_000,
                bytes: 2,
            }
            .into())
        );
    }

    #[test]
    fn late_bytes_counts_what_arrives_in_one_more_timeout() {
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        line.reply_at(500, &[1, 2, 3]);
        line.write_all(&[0]).unwrap();
        line.flush().unwrap();
        let mut clock = time.ticker(100);
        let mut link = Link {
            transport: &mut line,
            clock: &mut clock,
            timeout: Duration::from_millis(1),
            request: Request::NONE,
        };
        assert_eq!(late_bytes(&mut link), Ok(3));
        assert_eq!(late_bytes(&mut link), Ok(0));
    }

    #[test]
    fn receive_reads_no_further_than_wanted() {
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        line.stale(&[1, 2, 3, 4, 5]);
        let mut clock = time.ticker(1);
        let mut link = Link {
            transport: &mut line,
            clock: &mut clock,
            timeout: Duration::from_millis(1),
            request: Request::NONE,
        };
        let deadline = link.deadline();
        assert!(receive(&mut link, &mut Take(3), deadline).is_ok());
        assert_eq!(line.unread(), 2);
    }

    #[test]
    fn transport_errors_are_mapped() {
        let time = MockTime::new();
        let mut clock = time.ticker(1);
        let request = Request::NONE.instruction(Instruction::Read).id(id(2));
        for (kind, expected) in [
            (
                ErrorKind::TimedOut,
                Error::from(TransportError::PortTimeout {
                    request,
                    direction: Direction::ReadReady,
                }),
            ),
            (
                ErrorKind::BrokenPipe,
                TransportError::Io {
                    request,
                    direction: Direction::ReadReady,
                    kind: ErrorKind::BrokenPipe,
                }
                .into(),
            ),
        ] {
            let mut line = MockTransport::new(&time);
            line.fail_with(kind);
            let mut link = Link {
                transport: &mut line,
                clock: &mut clock,
                timeout: Duration::from_millis(1),
                request,
            };
            let deadline = link.deadline();
            assert_eq!(receive(&mut link, &mut Take(1), deadline), Err(expected));
        }
    }

    #[test]
    fn end_of_stream_is_a_transport_error_not_a_timeout() {
        let time = MockTime::new();
        let mut clock = time.ticker(1);
        let mut line = MockTransport::new(&time);
        line.end_of_stream();
        let mut link = Link {
            transport: &mut line,
            clock: &mut clock,
            timeout: Duration::from_millis(1),
            request: Request::NONE,
        };
        let eof = Error::from(TransportError::EndOfStream {
            request: Request::NONE,
        });
        assert_eq!(drain(&mut link), Err(eof));
        let deadline = link.deadline();
        assert_eq!(receive(&mut link, &mut Take(1), deadline), Err(eof));
        assert!(!matches!(eof, Error::Transport(err) if err.is_timeout()));
    }

    #[test]
    fn drain_discards_pending_bytes() {
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        line.stale(&[0xAA; 100]);
        let mut clock = time.ticker(1);
        let mut link = Link {
            transport: &mut line,
            clock: &mut clock,
            timeout: Duration::from_millis(1),
            request: Request::NONE,
        };
        drain(&mut link).unwrap();
        assert_eq!(line.unread(), 0);
    }

    #[test]
    fn writer_streams_in_chunks_and_flushes() {
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        let mut w = Writer::new(&mut line, Request::NONE);
        for b in 0..100u8 {
            w.put(b).unwrap();
        }
        w.finish().unwrap();
        assert_eq!(line.sent(), (0..100u8).collect::<std::vec::Vec<_>>());
        assert!(line.writes() >= 4);
    }

    #[test]
    fn writer_maps_write_errors() {
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        line.fail_with(ErrorKind::BrokenPipe);
        let request = Request::NONE.instruction(Instruction::Write).id(id(3));
        let mut w = Writer::new(&mut line, request);
        w.put(1).unwrap();
        assert_eq!(
            w.finish(),
            Err(TransportError::Io {
                request,
                direction: Direction::Write,
                kind: ErrorKind::BrokenPipe,
            }
            .into())
        );
        let mut line = MockTransport::new(&time);
        line.fail_with(ErrorKind::TimedOut);
        let w = Writer::new(&mut line, request);
        assert_eq!(
            w.finish(),
            Err(TransportError::PortTimeout {
                request,
                direction: Direction::Flush,
            }
            .into())
        );
    }

    /// User Story 3: every error byte becomes a precise typed error.
    mod status {
        use super::*;
        use crate::error::{P1Flag, P1Flags, P2Code};
        use crate::protocol::{Exchange, P1, P2, p2};

        fn p2_reply(error: u8) -> std::vec::Vec<u8> {
            let time = MockTime::new();
            let mut clock = time.ticker(1);
            let mut line = MockTransport::new(&time);
            p2::send(
                &mut Link {
                    transport: &mut line,
                    clock: &mut clock,
                    timeout: Duration::from_millis(1),
                    request: Request::NONE,
                },
                1,
                0x55,
                [error].into_iter(),
            )
            .unwrap();
            line.sent().to_vec()
        }

        fn p2_ping_error(error: u8) -> Result<Pong, Error> {
            let time = MockTime::new();
            let mut clock = time.ticker(1);
            let mut line = MockTransport::new(&time);
            line.reply(&p2_reply(error));
            P2::write(
                &mut Link {
                    transport: &mut line,
                    clock: &mut clock,
                    timeout: Duration::from_millis(1),
                    request: Request::NONE,
                },
                id(1),
                0,
                &[],
            )
            .map(|()| Pong {
                id: id(1),
                model_number: None,
                firmware: None,
                fault: None,
            })
        }

        fn p2_status(error: u8) -> StatusError {
            match p2_ping_error(error) {
                Err(Error::Status(status)) => status,
                other => panic!("error byte {error:#04X} gave {other:?}"),
            }
        }

        #[test]
        fn p2_alert_is_a_hardware_fault() {
            let status = p2_status(0x80);
            assert_eq!(
                status.report,
                StatusReport::P2 {
                    alert: true,
                    code: None
                }
            );
            assert!(status.is_hardware_fault());
        }

        #[test]
        fn p2_codes_without_alert_are_not_hardware_faults() {
            let codes = [
                P2Code::ResultFail,
                P2Code::Instruction,
                P2Code::Crc,
                P2Code::DataRange,
                P2Code::DataLength,
                P2Code::DataLimit,
                P2Code::Access,
            ];
            for (byte, code) in (1..=7).zip(codes) {
                let status = p2_status(byte);
                assert_eq!(
                    status.report,
                    StatusReport::P2 {
                        alert: false,
                        code: Some(code)
                    }
                );
                assert!(!status.is_hardware_fault());
            }
        }

        #[test]
        fn p2_alert_and_code_together_and_unknown_codes() {
            assert_eq!(
                p2_status(0x85).report,
                StatusReport::P2 {
                    alert: true,
                    code: Some(P2Code::DataLength)
                }
            );
            assert_eq!(
                p2_status(0x09).report,
                StatusReport::P2 {
                    alert: false,
                    code: Some(P2Code::Unknown(9))
                }
            );
        }

        #[test]
        fn zero_error_byte_is_ok() {
            assert!(p2_ping_error(0).is_ok());
        }

        #[test]
        fn p1_reports_every_flag() {
            let report = <P1 as Exchange>::report(0x24);
            let StatusReport::P1(flags) = report else {
                panic!("expected P1 flags");
            };
            assert_eq!(
                flags.iter().collect::<std::vec::Vec<_>>(),
                [P1Flag::Overheating, P1Flag::Overload]
            );
            let fault = StatusError {
                id: id(1),
                report,
                request: Request::NONE,
            };
            assert!(fault.is_hardware_fault());
            assert_eq!(
                std::string::ToString::to_string(&fault),
                "actuator 1 reported overheating (temperature above its Temperature Limit), \
                 an overload (load above its maximum torque)"
            );
            let range = StatusError {
                id: id(1),
                report: StatusReport::P1(P1Flags(1 << 3)),
                request: Request::NONE,
            };
            assert!(!range.is_hardware_fault());
        }
    }

    /// User Story 4: drain before send, bounded waits.
    mod exchange {
        use super::*;
        use crate::protocol::vectors::p2 as v;
        use crate::protocol::{Exchange, P2};

        #[test]
        fn stale_reply_is_drained_not_taken_as_the_answer() {
            let time = MockTime::new();
            let mut clock = time.ticker(1);
            let mut line = MockTransport::new(&time);
            line.stale(v::READ_REPLY_1).reply(v::READ_REPLY_2);
            let mut out = [0; 4];
            let result = P2::read(
                &mut Link {
                    transport: &mut line,
                    clock: &mut clock,
                    timeout: Duration::from_millis(1),
                    request: Request::NONE,
                },
                id(2),
                132,
                &mut out,
            );
            assert_eq!(result, Ok(()));
            assert_eq!(out, [0x1F, 0x08, 0, 0]);
        }

        fn read_with_reply_at(at_us: u64) -> Result<(), Error> {
            let time = MockTime::new();
            let mut clock = time.ticker(1);
            let mut line = MockTransport::new(&time);
            line.reply_at(at_us, v::READ_REPLY_1);
            let mut out = [0; 4];
            P2::read(
                &mut Link {
                    transport: &mut line,
                    clock: &mut clock,
                    timeout: Duration::from_micros(1_000),
                    request: Request::NONE,
                },
                id(1),
                132,
                &mut out,
            )
        }

        #[test]
        fn silent_actuator_times_out() {
            assert_eq!(
                read_with_reply_at(u64::MAX),
                Err(TransportError::NoReply {
                    request: Request::NONE
                        .instruction(Instruction::Read)
                        .id(id(1))
                        .addr(132)
                        .len(4),
                    timeout_us: 1_000,
                }
                .into())
            );
        }

        #[test]
        fn reply_just_before_the_deadline_is_taken() {
            assert_eq!(read_with_reply_at(900), Ok(()));
        }

        #[test]
        fn reply_after_the_deadline_is_too_late() {
            assert_eq!(
                read_with_reply_at(5_000),
                Err(TransportError::NoReply {
                    request: Request::NONE
                        .instruction(Instruction::Read)
                        .id(id(1))
                        .addr(132)
                        .len(4),
                    timeout_us: 1_000,
                }
                .into())
            );
        }
    }

    /// SC-003: a million inputs, no panic, every wait bounded.
    mod robustness {
        use super::*;
        use crate::protocol::vectors::{p1 as v1, p2 as v2};
        use crate::protocol::{Exchange, P1, P2, p1, p2};

        struct XorShift(u64);

        impl XorShift {
            fn next(&mut self) -> u64 {
                self.0 ^= self.0 << 13;
                self.0 ^= self.0 >> 7;
                self.0 ^= self.0 << 17;
                self.0
            }

            fn below(&mut self, n: usize) -> usize {
                (self.next() % n as u64) as usize
            }
        }

        /// A random stream: pure noise, or an e-Manual status packet that
        /// was flipped, truncated, spliced or given an oversized length.
        fn input(rng: &mut XorShift, buf: &mut [u8; 128]) -> usize {
            let pool: [&[u8]; 9] = [
                v2::PING_REPLY,
                v2::BROADCAST_PING_REPLY_2,
                v2::READ_REPLY_1,
                v2::READ_REPLY_2,
                v2::OK_REPLY,
                v1::OK_REPLY,
                v1::READ_REPLY,
                v1::ERROR_REPLY,
                v1::FACTORY_RESET_REPLY,
            ];
            let mut push = |len: &mut usize, bytes: &[u8]| {
                let n = bytes.len().min(128 - *len);
                buf[*len..*len + n].copy_from_slice(&bytes[..n]);
                *len += n;
            };
            let mut len = 0;
            match rng.below(5) {
                0 => {
                    let n = rng.below(64);
                    for _ in 0..n {
                        let b = rng.next() as u8;
                        push(&mut len, &[b]);
                    }
                }
                1 => {
                    push(&mut len, pool[rng.below(pool.len())]);
                    let pos = rng.below(len);
                    buf[pos] ^= rng.next() as u8 | 1;
                }
                2 => {
                    push(&mut len, pool[rng.below(pool.len())]);
                    len = rng.below(len);
                }
                3 => {
                    let first = pool[rng.below(pool.len())];
                    push(&mut len, &first[..rng.below(first.len())]);
                    push(&mut len, pool[rng.below(pool.len())]);
                }
                _ => {
                    push(&mut len, pool[rng.below(5)]);
                    buf[5] = 0xFF;
                    buf[6] = 0xFF;
                }
            }
            len
        }

        fn seed() -> u64 {
            std::env::var("DXL_FUZZ_SEED")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0x9E37_79B9_7F4A_7C15)
        }

        #[test]
        fn robustness_1m() {
            let seed = seed();
            let mut rng = XorShift(seed);
            let mut buf = [0u8; 128];
            let mut out = [0u8; 8];
            let result = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
                for _ in 0..1_000_000 {
                    let len = input(&mut rng, &mut buf);
                    let mut a = p2::StatusParser::new(&mut out);
                    for &b in &buf[..len] {
                        let _ = a.push(b);
                        assert!(a.wanted() >= 1);
                    }
                    let mut b1 = p1::StatusParser::new(&mut out);
                    for &b in &buf[..len] {
                        let _ = b1.push(b);
                        assert!(b1.wanted() >= 1);
                    }
                }
            }));
            assert!(
                result.is_ok(),
                "panicked; reproduce with DXL_FUZZ_SEED={seed}"
            );
        }

        #[test]
        fn every_exchange_returns_by_its_deadline() {
            let seed = seed();
            let mut rng = XorShift(seed);
            let mut buf = [0u8; 128];
            for _ in 0..10_000 {
                let len = input(&mut rng, &mut buf);
                let time = MockTime::new();
                let mut clock = time.ticker(1);
                let mut line = MockTransport::new(&time);
                line.reply(&buf[..len]);
                let mut out = [0; 4];
                let mut link = Link {
                    transport: &mut line,
                    clock: &mut clock,
                    timeout: Duration::from_micros(500),
                    request: Request::NONE,
                };
                if rng.next() & 1 == 0 {
                    let _ = P2::read(&mut link, id(1), 132, &mut out);
                } else {
                    let _ = P1::read(&mut link, id(1), 43, &mut out[..1]);
                }
                assert!(
                    time.now() < 2_000,
                    "exchange ran past its deadline; DXL_FUZZ_SEED={seed}"
                );
            }
        }
    }
}
