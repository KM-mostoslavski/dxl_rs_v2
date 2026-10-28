//! Dynamixel Protocol 1.0: <https://emanual.robotis.com/docs/en/dxl/protocol1/>.
//!
//! Instruction packet: `FF FF | ID | LEN | INST | params | CHECKSUM`.
//! Status packet: `FF FF | ID | LEN | ERROR | params | CHECKSUM`.
//! `LEN` counts every byte after itself; the checksum is the inverted low
//! byte of the sum from `ID` to the last parameter.

use core::iter;

use super::{
    Clock, Exchange, Link, Parser, Pong, Protocol, ProtocolVersion, Status, Transport, Writer,
    check, drain, receive, sealed,
};
use crate::Id;
use crate::error::{
    Error, P1Flags, ProtocolError, StatusError, StatusReport, TransportError, ValidationError,
};

/// Dynamixel Protocol 1.0.
///
/// Has no Sync Read and its Ping reply carries no model number; those
/// operations are absent by design, so code using them with `P1` does not
/// compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct P1;

impl sealed::Sealed for P1 {}

impl Protocol for P1 {
    const VERSION: ProtocolVersion = ProtocolVersion::P1;
}

const BROADCAST: u8 = 0xFE;

const PING: u8 = 0x01;
const READ: u8 = 0x02;
const WRITE: u8 = 0x03;
const SYNC_WRITE: u8 = 0x83;

/// Largest `LEN` field value.
const MAX_LEN: usize = u8::MAX as usize;
/// Most data bytes a status packet can carry: `LEN` also counts the error
/// and checksum bytes.
const MAX_STATUS_DATA: usize = MAX_LEN - 2;

/// Encodes and sends one instruction packet.
fn send<T: embedded_io::Write, I>(
    transport: &mut T,
    id: u8,
    instruction: u8,
    params: I,
) -> Result<(), Error>
where
    I: Iterator<Item = u8> + Clone,
{
    let len = params.clone().count() + 2;
    let Ok(len_field) = u8::try_from(len) else {
        return Err(ValidationError::PacketTooLarge { len, max: MAX_LEN }.into());
    };
    let mut w = Writer::new(transport);
    w.put(0xFF)?;
    w.put(0xFF)?;
    let mut sum = 0u8;
    for b in [id, len_field, instruction].into_iter().chain(params) {
        sum = sum.wrapping_add(b);
        w.put(b)?;
    }
    w.put(!sum)?;
    w.finish()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    /// Searching for `FF FF`; holds how many matched.
    Hunt(u8),
    Len,
    Error,
    Data,
    Checksum,
}

/// Parses one status packet, writing its data into `out`.
pub(crate) struct StatusParser<'o> {
    out: &'o mut [u8],
    state: State,
    id: u8,
    sum: u8,
    /// Bytes left in the packet once `LEN` is known.
    remaining: usize,
    error: u8,
    count: usize,
    /// A header with an impossible `LEN` was seen and skipped.
    malformed: bool,
}

impl<'o> StatusParser<'o> {
    pub(crate) fn new(out: &'o mut [u8]) -> Self {
        Self {
            out,
            state: State::Hunt(0),
            id: 0,
            sum: 0,
            remaining: 0,
            error: 0,
            count: 0,
            malformed: false,
        }
    }

    fn restart(&mut self) {
        self.state = State::Hunt(0);
        self.count = 0;
    }
}

impl Parser for StatusParser<'_> {
    fn wanted(&self) -> usize {
        match self.state {
            State::Hunt(matched) => 4 - usize::from(matched),
            State::Len => 1,
            _ => self.remaining.max(1),
        }
    }

    fn on_deadline(&self) -> Option<ProtocolError> {
        self.malformed.then_some(ProtocolError::Malformed)
    }

    fn push(&mut self, byte: u8) -> Option<Result<Status, ProtocolError>> {
        match self.state {
            State::Hunt(matched) if matched < 2 => {
                self.state = State::Hunt(if byte == 0xFF { matched + 1 } else { 0 });
            }
            State::Hunt(_) => {
                if byte != 0xFF {
                    self.id = byte;
                    self.sum = byte;
                    self.state = State::Len;
                }
            }
            State::Len => {
                if byte < 2 {
                    self.malformed = true;
                    self.restart();
                    return None;
                }
                self.sum = self.sum.wrapping_add(byte);
                self.remaining = usize::from(byte);
                self.state = State::Error;
            }
            State::Error => {
                self.sum = self.sum.wrapping_add(byte);
                self.error = byte;
                self.remaining -= 1;
                self.state = if self.remaining == 1 {
                    State::Checksum
                } else {
                    State::Data
                };
            }
            State::Data => {
                self.sum = self.sum.wrapping_add(byte);
                if let Some(slot) = self.out.get_mut(self.count) {
                    *slot = byte;
                }
                self.count += 1;
                self.remaining -= 1;
                if self.remaining == 1 {
                    self.state = State::Checksum;
                }
            }
            State::Checksum => {
                let result = if byte == !self.sum {
                    Ok(Status {
                        id: self.id,
                        error: self.error,
                        len: self.count,
                    })
                } else {
                    Err(ProtocolError::Checksum { id: self.id })
                };
                self.restart();
                return Some(result);
            }
        }
        None
    }
}

/// Protocol 1 addresses are one byte.
fn short_addr(addr: u16) -> Result<u8, Error> {
    u8::try_from(addr).map_err(|_| ValidationError::AddressOutOfRange { addr }.into())
}

/// Sends an instruction to `id` and parses its status packet into `out`,
/// checking it carries exactly `out.len()` data bytes.
fn unicast<T: Transport, C: Clock, I>(
    link: &mut Link<'_, T, C>,
    id: Id,
    instruction: u8,
    params: I,
    out: &mut [u8],
) -> Result<(), Error>
where
    I: Iterator<Item = u8> + Clone,
{
    drain(link)?;
    send(link.transport, id.get(), instruction, params)?;
    let deadline = link.deadline();
    let status = receive(link, &mut StatusParser::new(out), deadline, Some(id))?;
    check::<P1>(status, id, out)
}

impl Exchange for P1 {
    fn ping<T: Transport, C: Clock>(link: &mut Link<'_, T, C>, id: Id) -> Result<Pong, Error> {
        unicast(link, id, PING, iter::empty(), &mut [])?;
        Ok(Pong {
            id,
            model_number: None,
            firmware: None,
            fault: None,
        })
    }

    fn read<T: Transport, C: Clock>(
        link: &mut Link<'_, T, C>,
        id: Id,
        addr: u16,
        out: &mut [u8],
    ) -> Result<(), Error> {
        let addr = short_addr(addr)?;
        let Some(len) = u8::try_from(out.len())
            .ok()
            .filter(|_| out.len() <= MAX_STATUS_DATA)
        else {
            return Err(ValidationError::PacketTooLarge {
                len: out.len(),
                max: MAX_STATUS_DATA,
            }
            .into());
        };
        unicast(link, id, READ, [addr, len].into_iter(), out)
    }

    fn write<T: Transport, C: Clock>(
        link: &mut Link<'_, T, C>,
        id: Id,
        addr: u16,
        data: &[u8],
    ) -> Result<(), Error> {
        let params = iter::once(short_addr(addr)?).chain(data.iter().copied());
        unicast(link, id, WRITE, params, &mut [])
    }

    fn sync_write<'d, T: Transport, C: Clock, I>(
        link: &mut Link<'_, T, C>,
        addr: u16,
        size: u16,
        entries: I,
    ) -> Result<(), Error>
    where
        I: Iterator<Item = (Id, &'d [u8])> + Clone,
    {
        let addr = short_addr(addr)?;
        let Ok(size_byte) = u8::try_from(size) else {
            return Err(ValidationError::PacketTooLarge {
                len: usize::from(size),
                max: MAX_LEN,
            }
            .into());
        };
        if let Some((_, data)) = entries.clone().find(|(_, d)| d.len() != usize::from(size)) {
            return Err(ValidationError::DataLength {
                expected: usize::from(size),
                got: data.len(),
            }
            .into());
        }
        let params = [addr, size_byte]
            .into_iter()
            .chain(entries.flat_map(|(id, data)| iter::once(id.get()).chain(data.iter().copied())));
        drain(link)?;
        send(link.transport, BROADCAST, SYNC_WRITE, params)
    }

    fn scan<T: Transport, C: Clock, F>(
        link: &mut Link<'_, T, C>,
        mut on_reply: F,
    ) -> Result<(), Error>
    where
        F: FnMut(Result<Pong, Error>),
    {
        for id in (0..=Id::MAX).filter_map(|raw| Id::new(raw).ok()) {
            match Self::ping(link, id) {
                Ok(pong) => on_reply(Ok(pong)),
                Err(Error::Status(StatusError { id, report })) => on_reply(Ok(Pong {
                    id,
                    model_number: None,
                    firmware: None,
                    fault: Some(report),
                })),
                Err(Error::Transport(TransportError::Timeout { .. })) => {}
                Err(err @ Error::Transport(_)) => return Err(err),
                Err(err) => on_reply(Err(err)),
            }
        }
        Ok(())
    }

    fn read_many<T: Transport, C: Clock, I, F>(
        link: &mut Link<'_, T, C>,
        addr: u16,
        size: u16,
        ids: I,
        scratch: &mut [u8],
        mut on_reply: F,
    ) -> Result<(), Error>
    where
        I: Iterator<Item = Id> + Clone,
        F: FnMut(Id, Result<&[u8], Error>),
    {
        short_addr(addr)?;
        let size = usize::from(size);
        if size > MAX_STATUS_DATA {
            return Err(ValidationError::PacketTooLarge {
                len: size,
                max: MAX_STATUS_DATA,
            }
            .into());
        }
        let Some(buf) = scratch.get_mut(..size) else {
            return Err(ValidationError::DataLength {
                expected: size,
                got: scratch.len(),
            }
            .into());
        };
        for id in ids {
            match Self::read(link, id, addr, buf) {
                Ok(()) => on_reply(id, Ok(&*buf)),
                Err(err @ Error::Transport(TransportError::Io(_))) => return Err(err),
                Err(err) => on_reply(id, Err(err)),
            }
        }
        Ok(())
    }

    fn report(error: u8) -> StatusReport {
        StatusReport::P1(P1Flags(error))
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use core::time::Duration;

    use std::vec::Vec;

    use super::*;
    use crate::error::{P1Flag, StatusError};
    use crate::protocol::vectors::p1 as v;
    use crate::transport::{MockTime, MockTransport};

    fn id(raw: u8) -> Id {
        Id::new(raw).unwrap()
    }

    fn packet(id: u8, instruction: u8, params: &[u8]) -> Vec<u8> {
        let mut buf = [0u8; 300];
        let mut w: &mut [u8] = &mut buf;
        send(&mut w, id, instruction, params.iter().copied()).unwrap();
        let n = 300 - w.len();
        buf[..n].to_vec()
    }

    fn parse(bytes: &[u8], out: &mut [u8]) -> Option<Result<Status, ProtocolError>> {
        let mut parser = StatusParser::new(out);
        let mut last = None;
        for &b in bytes {
            if let Some(r) = parser.push(b) {
                last = Some(r);
            }
        }
        last
    }

    macro_rules! link {
        ($line:expr, $clock:expr) => {
            Link {
                transport: &mut $line,
                clock: &mut $clock,
                timeout: Duration::from_millis(10),
            }
        };
    }

    #[test]
    fn encode_matches_emanual() {
        assert_eq!(packet(1, PING, &[]), v::PING);
        assert_eq!(packet(1, READ, &[43, 1]), v::READ);
        assert_eq!(packet(1, WRITE, &[0x0C, 0x64, 0xAA]), v::WRITE);
        assert_eq!(packet(BROADCAST, WRITE, &[0x03, 0x01]), v::WRITE_BROADCAST);
    }

    #[test]
    fn decode_matches_emanual() {
        let status = parse(v::OK_REPLY, &mut []).unwrap().unwrap();
        assert_eq!(
            status,
            Status {
                id: 1,
                error: 0,
                len: 0
            }
        );
        let mut out = [0; 1];
        let status = parse(v::READ_REPLY, &mut out).unwrap().unwrap();
        assert_eq!((status.len, out), (1, [0x20]));
        let status = parse(v::ERROR_REPLY, &mut []).unwrap().unwrap();
        assert_eq!(status.error, 0x24);
    }

    #[test]
    fn every_single_byte_corruption_is_rejected() {
        for packet in v::STATUS {
            for pos in 2..packet.len() {
                for flip in [0x01, 0x80, 0xFF] {
                    let mut bad = packet.to_vec();
                    bad[pos] ^= flip;
                    let mut out = [0; 8];
                    let result = parse(&bad, &mut out);
                    assert!(
                        !matches!(result, Some(Ok(_))),
                        "accepted {bad:02X?} (pos {pos})"
                    );
                    if pos != 3 && bad[2] != 0xFF {
                        assert!(
                            matches!(result, Some(Err(ProtocolError::Checksum { .. }))),
                            "{bad:02X?} (pos {pos}) gave {result:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn resyncs_after_garbage() {
        let mut stream = std::vec![0x00, 0xFF, 0x12, 0xFF, 0xFF, 0xFF];
        stream.extend_from_slice(&v::READ_REPLY[2..]);
        let mut out = [0; 1];
        let status = parse(&stream, &mut out).unwrap().unwrap();
        assert_eq!((status.id, out), (1, [0x20]));
    }

    #[test]
    fn impossible_length_resyncs_to_the_real_reply() {
        let mut stream = std::vec![0xFF, 0xFF, 0x01, 0x01];
        stream.extend_from_slice(v::READ_REPLY);
        let mut out = [0; 1];
        let status = parse(&stream, &mut out).unwrap().unwrap();
        assert_eq!((status.id, out), (1, [0x20]));
    }

    #[test]
    fn only_impossible_lengths_report_malformed_at_the_deadline() {
        let time = MockTime::new();
        let mut clock = time.ticker(1);
        let mut line = MockTransport::new(&time);
        line.reply(&[0xFF, 0xFF, 0x01, 0x00]);
        assert_eq!(
            P1::ping(&mut link!(line, clock), id(1)),
            Err(ProtocolError::Malformed.into())
        );
    }

    #[test]
    fn ping_read_write_exchanges() {
        let time = MockTime::new();
        let mut clock = time.ticker(1);

        let mut line = MockTransport::new(&time);
        line.reply(v::OK_REPLY);
        let pong = P1::ping(&mut link!(line, clock), id(1)).unwrap();
        assert_eq!(
            pong,
            Pong {
                id: id(1),
                model_number: None,
                firmware: None,
                fault: None
            }
        );
        assert_eq!(line.sent(), v::PING);

        let mut line = MockTransport::new(&time);
        line.reply(v::READ_REPLY);
        let mut out = [0; 1];
        P1::read(&mut link!(line, clock), id(1), 43, &mut out).unwrap();
        assert_eq!((out, line.sent()), ([0x20], v::READ));

        let mut line = MockTransport::new(&time);
        line.reply(v::OK_REPLY);
        P1::write(&mut link!(line, clock), id(1), 12, &[0x64, 0xAA]).unwrap();
        assert_eq!(line.sent(), v::WRITE);
    }

    #[test]
    fn status_error_reports_every_flag() {
        let time = MockTime::new();
        let mut clock = time.ticker(1);
        let mut line = MockTransport::new(&time);
        line.reply(v::ERROR_REPLY);
        let err = P1::ping(&mut link!(line, clock), id(1)).unwrap_err();
        let Error::Status(status @ StatusError { report, .. }) = err else {
            panic!("expected a status error, got {err:?}");
        };
        assert!(status.is_hardware_fault());
        let StatusReport::P1(flags) = report else {
            panic!("expected P1 flags");
        };
        assert_eq!(
            flags.iter().collect::<Vec<_>>(),
            [P1Flag::Overheating, P1Flag::Overload]
        );
    }

    #[test]
    fn sync_write_matches_emanual_and_never_reads() {
        let time = MockTime::new();
        let mut clock = time.ticker(1);
        let mut line = MockTransport::new(&time);
        let a = [0x10, 0x00, 0x50, 0x01];
        let b = [0x20, 0x02, 0x60, 0x03];
        let entries = [(id(0), &a[..]), (id(1), &b[..])];
        P1::sync_write(&mut link!(line, clock), 30, 4, entries.into_iter()).unwrap();
        assert_eq!((line.sent(), line.reads()), (v::SYNC_WRITE, 0));
    }

    #[test]
    fn out_of_range_requests_are_refused_before_sending() {
        let time = MockTime::new();
        let mut clock = time.ticker(1);
        let mut line = MockTransport::new(&time);
        let mut out = [0; 1];
        assert_eq!(
            P1::read(&mut link!(line, clock), id(1), 256, &mut out),
            Err(ValidationError::AddressOutOfRange { addr: 256 }.into())
        );
        let mut huge = [0; MAX_STATUS_DATA + 1];
        assert_eq!(
            P1::read(&mut link!(line, clock), id(1), 0, &mut huge),
            Err(ValidationError::PacketTooLarge {
                len: MAX_STATUS_DATA + 1,
                max: MAX_STATUS_DATA
            }
            .into())
        );
        assert!(matches!(
            P1::write(&mut link!(line, clock), id(1), 0, &[0; 300]),
            Err(Error::Validation(ValidationError::PacketTooLarge { .. }))
        ));
        let empty: [(Id, &[u8]); 0] = [];
        assert!(matches!(
            P1::sync_write(&mut link!(line, clock), 0, 256, empty.into_iter()),
            Err(Error::Validation(ValidationError::PacketTooLarge { .. }))
        ));
        let short = [(id(1), &[0u8][..])];
        assert!(matches!(
            P1::sync_write(&mut link!(line, clock), 0, 2, short.into_iter()),
            Err(Error::Validation(ValidationError::DataLength { .. }))
        ));
        assert!(line.sent().is_empty());
    }

    #[test]
    fn scan_pings_every_id_and_reports_answers_faults_and_corruption() {
        let time = MockTime::new();
        let mut clock = time.ticker(100);
        let mut line = MockTransport::new(&time);
        let mut corrupted = packet(5, 0, &[]);
        *corrupted.last_mut().unwrap() ^= 0xFF;
        line.reply_to(2, v::OK_REPLY)
            .reply_to(4, &packet(3, 0x20, &[]))
            .reply_to(6, &corrupted)
            .reply_to(201, &packet(200, 0, &[]));
        let mut got = Vec::new();
        P1::scan(&mut link!(line, clock), |r| got.push(r)).unwrap();
        let pong = |raw, fault| Pong {
            id: id(raw),
            model_number: None,
            firmware: None,
            fault,
        };
        assert_eq!(
            got,
            [
                Ok(pong(1, None)),
                Ok(pong(3, Some(StatusReport::P1(P1Flags(0x20))))),
                Err(ProtocolError::Checksum { id: 5 }.into()),
                Ok(pong(200, None)),
            ]
        );
        assert_eq!(line.flushes(), usize::from(Id::MAX) + 1);
        line.assert_done();
    }

    #[test]
    fn scan_fails_only_when_the_transport_does() {
        let time = MockTime::new();
        let mut clock = time.ticker(1);
        let mut line = MockTransport::new(&time);
        line.fail_with(embedded_io::ErrorKind::BrokenPipe);
        let mut calls = 0;
        assert_eq!(
            P1::scan(&mut link!(line, clock), |_| calls += 1),
            Err(TransportError::Io(embedded_io::ErrorKind::BrokenPipe).into())
        );
        assert_eq!(calls, 0);
    }

    fn read_many_with(
        line: &mut MockTransport<'_>,
        ids: &[u8],
    ) -> Vec<(u8, Result<Vec<u8>, Error>)> {
        let time = MockTime::new();
        let mut clock = time.ticker(100);
        let mut got = Vec::new();
        let mut scratch = [0; 4];
        P1::read_many(
            &mut link!(*line, clock),
            43,
            1,
            ids.iter().map(|&raw| id(raw)),
            &mut scratch,
            |id, r| got.push((id.get(), r.map(<[u8]>::to_vec))),
        )
        .unwrap();
        got
    }

    #[test]
    fn read_many_reads_each_id_in_order_and_reports_each_error_to_its_id() {
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        line.reply_to(1, v::READ_REPLY)
            .reply_to(3, &packet(3, 0, &[0x30]))
            .reply_to(4, &packet(4, 0x20, &[0x40]));
        let got = read_many_with(&mut line, &[1, 2, 3, 4]);
        assert_eq!(
            got,
            [
                (1, Ok(std::vec![0x20])),
                (2, Err(TransportError::Timeout { id: Some(id(2)) }.into())),
                (3, Ok(std::vec![0x30])),
                (
                    4,
                    Err(StatusError {
                        id: id(4),
                        report: StatusReport::P1(P1Flags(0x20))
                    }
                    .into())
                ),
            ]
        );
        let mut expected = Vec::new();
        for raw in 1..=4 {
            expected.extend(packet(raw, READ, &[43, 1]));
        }
        line.assert_sent(&expected);
    }

    #[test]
    fn read_many_refuses_bad_requests_and_fails_on_the_transport() {
        let time = MockTime::new();
        let mut clock = time.ticker(1);
        let mut line = MockTransport::new(&time);
        let ids = [id(1)];
        let mut small = [0; 1];
        let mut calls = 0;
        for (addr, size, expected) in [
            (
                43,
                2,
                ValidationError::DataLength {
                    expected: 2,
                    got: 1,
                },
            ),
            (256, 1, ValidationError::AddressOutOfRange { addr: 256 }),
            (
                0,
                MAX_STATUS_DATA as u16 + 1,
                ValidationError::PacketTooLarge {
                    len: MAX_STATUS_DATA + 1,
                    max: MAX_STATUS_DATA,
                },
            ),
        ] {
            assert_eq!(
                P1::read_many(
                    &mut link!(line, clock),
                    addr,
                    size,
                    ids.into_iter(),
                    &mut small,
                    |_, _| calls += 1
                ),
                Err(expected.into())
            );
        }
        assert!(line.sent().is_empty());
        line.fail_with(embedded_io::ErrorKind::BrokenPipe);
        assert_eq!(
            P1::read_many(
                &mut link!(line, clock),
                43,
                1,
                ids.into_iter(),
                &mut small,
                |_, _| calls += 1
            ),
            Err(TransportError::Io(embedded_io::ErrorKind::BrokenPipe).into())
        );
        assert_eq!(calls, 0);
    }
}
