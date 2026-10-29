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
mod vectors;

pub use clock::Clock;
#[cfg(feature = "std")]
pub use clock::StdClock;
pub use p1::P1;
pub use p2::P2;

use core::fmt;
use core::time::Duration;

use embedded_io::{ErrorKind, Write};

use crate::error::{Error, ProtocolError, StatusError, StatusReport, TransportError};
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
}

impl<T: Transport, C: Clock> Link<'_, T, C> {
    /// The instant, in clock microseconds, at which a wait starting now ends.
    pub fn deadline(&mut self) -> u64 {
        let timeout = u64::try_from(self.timeout.as_micros()).unwrap_or(u64::MAX);
        self.clock.now_us().saturating_add(timeout)
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
    fn push(&mut self, byte: u8) -> Option<Result<Status, ProtocolError>>;

    /// The error to report instead of a timeout when the deadline passes,
    /// e.g. because only malformed headers were seen.
    fn on_deadline(&self) -> Option<ProtocolError> {
        None
    }
}

/// Size of the stack buffer used for reading and writing.
pub(crate) const CHUNK: usize = 32;

/// What a `read` of zero bytes means: end of stream, the port is gone.
const END_OF_STREAM: ErrorKind = ErrorKind::NotConnected;

/// Maps a transport error; a transport-level timeout counts as ours.
pub(crate) fn io_error<E: embedded_io::Error>(err: E, id: Option<Id>) -> Error {
    match err.kind() {
        ErrorKind::TimedOut => TransportError::Timeout { id }.into(),
        kind => TransportError::Io(kind).into(),
    }
}

/// Discards whatever is waiting on the receive side, so stale bytes are
/// never taken for the reply to the next instruction. Gives up after one
/// timeout if the line keeps talking.
pub(crate) fn drain<T: Transport, C: Clock>(link: &mut Link<'_, T, C>) -> Result<(), Error> {
    let deadline = link.deadline();
    let mut chunk = [0u8; CHUNK];
    while link.transport.read_ready().map_err(|e| io_error(e, None))? {
        if link.clock.now_us() >= deadline {
            break;
        }
        let n = link
            .transport
            .read(&mut chunk)
            .map_err(|e| io_error(e, None))?;
        if n == 0 {
            return Err(TransportError::Io(END_OF_STREAM).into());
        }
    }
    Ok(())
}

/// Waits until `parser` completes a packet or `deadline` passes.
///
/// Reads only as many bytes as the parser wants, so bytes of the next packet
/// stay on the line for the next call.
pub(crate) fn receive<T: Transport, C: Clock, P: Parser>(
    link: &mut Link<'_, T, C>,
    parser: &mut P,
    deadline: u64,
    id: Option<Id>,
) -> Result<Status, Error> {
    let mut chunk = [0u8; CHUNK];
    loop {
        if link.clock.now_us() >= deadline {
            return Err(parser
                .on_deadline()
                .map_or(TransportError::Timeout { id }.into(), Error::from));
        }
        if !link.transport.read_ready().map_err(|e| io_error(e, id))? {
            continue;
        }
        let want = parser.wanted().clamp(1, CHUNK);
        let n = link
            .transport
            .read(&mut chunk[..want])
            .map_err(|e| io_error(e, id))?;
        if n == 0 {
            return Err(TransportError::Io(END_OF_STREAM).into());
        }
        let n = n.min(want);
        for &byte in &chunk[..n] {
            if let Some(result) = parser.push(byte) {
                return result.map_err(Error::from);
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
pub(crate) fn check<P: Exchange>(status: Status, id: Id, out: &mut [u8]) -> Result<(), Error> {
    let len = out.len();
    if status.id != id.get() {
        return Err(ProtocolError::WrongId {
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
        }
        .into());
    }
    if status.len != len {
        return Err(ProtocolError::WrongLength {
            expected: len,
            got: status.len,
        }
        .into());
    }
    Ok(())
}

/// Buffers outgoing bytes in a small stack chunk.
pub(crate) struct Writer<'t, T: Write> {
    transport: &'t mut T,
    buf: [u8; CHUNK],
    len: usize,
}

impl<'t, T: Write> Writer<'t, T> {
    pub(crate) fn new(transport: &'t mut T) -> Self {
        Self {
            transport,
            buf: [0; CHUNK],
            len: 0,
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
        self.transport.flush().map_err(|e| io_error(e, None))
    }

    fn write_buf(&mut self) -> Result<(), Error> {
        self.transport
            .write_all(&self.buf[..self.len])
            .map_err(|e| io_error(e, None))?;
        self.len = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use crate::transport::{MockTime, MockTransport};

    /// Accepts any `n` bytes as a packet with ID 1.
    struct Take(usize);

    impl Parser for Take {
        fn wanted(&self) -> usize {
            self.0
        }

        fn push(&mut self, _: u8) -> Option<Result<Status, ProtocolError>> {
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
        };
        let deadline = link.deadline();
        assert_eq!(
            receive(&mut link, &mut Take(3), deadline, Some(id(4))),
            Err(TransportError::Timeout { id: Some(id(4)) }.into())
        );
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
        };
        let deadline = link.deadline();
        assert!(receive(&mut link, &mut Take(3), deadline, None).is_ok());
        assert_eq!(line.unread(), 2);
    }

    #[test]
    fn transport_errors_are_mapped() {
        let time = MockTime::new();
        let mut clock = time.ticker(1);
        for (kind, expected) in [
            (
                ErrorKind::TimedOut,
                Error::from(TransportError::Timeout { id: Some(id(2)) }),
            ),
            (
                ErrorKind::BrokenPipe,
                TransportError::Io(ErrorKind::BrokenPipe).into(),
            ),
        ] {
            let mut line = MockTransport::new(&time);
            line.fail_with(kind);
            let mut link = Link {
                transport: &mut line,
                clock: &mut clock,
                timeout: Duration::from_millis(1),
            };
            let deadline = link.deadline();
            assert_eq!(
                receive(&mut link, &mut Take(1), deadline, Some(id(2))),
                Err(expected)
            );
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
        };
        let eof = Error::from(TransportError::Io(END_OF_STREAM));
        assert_eq!(drain(&mut link), Err(eof));
        let deadline = link.deadline();
        assert_eq!(receive(&mut link, &mut Take(1), deadline, None), Err(eof));
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
        };
        drain(&mut link).unwrap();
        assert_eq!(line.unread(), 0);
    }

    #[test]
    fn writer_streams_in_chunks_and_flushes() {
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        let mut w = Writer::new(&mut line);
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
        let mut w = Writer::new(&mut line);
        w.put(1).unwrap();
        assert_eq!(
            w.finish(),
            Err(TransportError::Io(ErrorKind::BrokenPipe).into())
        );
    }

    /// User Story 3: every error byte becomes a precise typed error.
    mod status {
        use super::*;
        use crate::error::{P1Flag, P1Flags, P2Code};
        use crate::protocol::{Exchange, P1, P2, p2};

        fn p2_reply(error: u8) -> std::vec::Vec<u8> {
            let mut buf = [0u8; 64];
            let mut w: &mut [u8] = &mut buf;
            p2::send(&mut w, 1, 0x55, [error].into_iter()).unwrap();
            let n = 64 - w.len();
            buf[..n].to_vec()
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
            let fault = StatusError { id: id(1), report };
            assert!(fault.is_hardware_fault());
            assert_eq!(
                std::string::ToString::to_string(&fault),
                "actuator 1 reported overheating, overload"
            );
            let range = StatusError {
                id: id(1),
                report: StatusReport::P1(P1Flags(1 << 3)),
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
                Err(TransportError::Timeout { id: Some(id(1)) }.into())
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
                Err(TransportError::Timeout { id: Some(id(1)) }.into())
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
