//! Dynamixel Protocol 2.0: <https://emanual.robotis.com/docs/en/dxl/protocol2/>.
//!
//! Instruction packet: `FF FF FD 00 | ID | LEN_L LEN_H | INST | params | CRC_L CRC_H`.
//! Status packet: the same with `INST = 0x55` followed by an error byte.
//! `LEN` counts every byte after itself, stuffing included.

use core::iter;

use super::{
    Clock, Exchange, Link, Parser, Pong, Protocol, ProtocolVersion, Status, Transport, Writer,
    check, crc, drain, receive, sealed,
};
use crate::Id;
use crate::error::{
    Error, P2Code, ProtocolError, StatusError, StatusReport, TransportError, ValidationError,
};

/// Dynamixel Protocol 2.0.
///
/// Has two instructions Protocol 1 lacks: Sync Read and broadcast Ping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct P2;

impl sealed::Sealed for P2 {}

impl Protocol for P2 {
    const VERSION: ProtocolVersion = ProtocolVersion::P2;
}

const HEADER: [u8; 4] = [0xFF, 0xFF, 0xFD, 0x00];
const BROADCAST: u8 = 0xFE;

const PING: u8 = 0x01;
const READ: u8 = 0x02;
const WRITE: u8 = 0x03;
const STATUS: u8 = 0x55;
const SYNC_READ: u8 = 0x82;
const SYNC_WRITE: u8 = 0x83;

/// Largest `LEN` field value.
const MAX_LEN: usize = u16::MAX as usize;
/// Most data bytes a status packet can carry: `LEN` also counts the
/// instruction, error and CRC bytes.
const MAX_STATUS_DATA: usize = MAX_LEN - 4;

/// Tracks the byte-stuffing pattern over a stream of bytes.
///
/// The e-Manual (`#transmission-process`) adds `0xFD` after every
/// `FF FF FD` "in a packet" but does not say which fields. Stuffing is
/// applied from the instruction byte to the last parameter: the header must
/// stay a header, and the reserved byte, ID and length cannot start the
/// pattern.
#[derive(Default)]
struct Stuffing {
    last: [u8; 2],
}

impl Stuffing {
    /// Records `byte`; returns whether a stuffing `0xFD` follows it.
    fn push(&mut self, byte: u8) -> bool {
        let stuffed = self.last == [0xFF, 0xFF] && byte == 0xFD;
        self.last = if stuffed {
            [0, 0]
        } else {
            [self.last[1], byte]
        };
        stuffed
    }
}

/// Encodes and sends one instruction packet, stuffing and CRC included.
pub(super) fn send<T: embedded_io::Write, I>(
    transport: &mut T,
    id: u8,
    instruction: u8,
    params: I,
) -> Result<(), Error>
where
    I: Iterator<Item = u8> + Clone,
{
    let mut stuffing = Stuffing::default();
    let body: usize = iter::once(instruction)
        .chain(params.clone())
        .map(|b| 1 + usize::from(stuffing.push(b)))
        .sum();
    let len = body + 2;
    let Ok(len_field) = u16::try_from(len) else {
        return Err(ValidationError::PacketTooLarge { len, max: MAX_LEN }.into());
    };

    let mut w = Writer::new(transport);
    let mut crc = 0;
    let [len_l, len_h] = len_field.to_le_bytes();
    let mut stuffing = Stuffing::default();
    for (i, b) in HEADER
        .into_iter()
        .chain([id, len_l, len_h, instruction])
        .chain(params)
        .enumerate()
    {
        crc = crc::update_byte(crc, b);
        w.put(b)?;
        if i >= 7 && stuffing.push(b) {
            crc = crc::update_byte(crc, 0xFD);
            w.put(0xFD)?;
        }
    }
    let [crc_l, crc_h] = crc.to_le_bytes();
    w.put(crc_l)?;
    w.put(crc_h)?;
    w.finish()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    /// Searching for the header; holds how many header bytes matched.
    Hunt(u8),
    Id,
    LenL,
    LenH,
    Instruction,
    Error,
    Data,
    CrcL,
    CrcH,
    /// Skipping the rest of a packet that is not a status packet.
    Skip,
}

/// Parses one status packet, writing its unstuffed data into `out`.
pub(crate) struct StatusParser<'o> {
    out: &'o mut [u8],
    state: State,
    crc: u16,
    id: u8,
    len_l: u8,
    /// Raw bytes left in the packet once `LEN` is known.
    remaining: usize,
    error: u8,
    count: usize,
    received_crc_l: u8,
    stuffing: Stuffing,
    drop_next: bool,
    /// A header with an impossible `LEN` was seen and skipped.
    malformed: bool,
}

impl<'o> StatusParser<'o> {
    pub(crate) fn new(out: &'o mut [u8]) -> Self {
        Self {
            out,
            state: State::Hunt(0),
            crc: 0,
            id: 0,
            len_l: 0,
            remaining: 0,
            error: 0,
            count: 0,
            received_crc_l: 0,
            stuffing: Stuffing::default(),
            drop_next: false,
            malformed: false,
        }
    }

    /// The most bytes that can follow the instruction byte in a status
    /// packet with `out.len()` data bytes: error byte, data, CRC, and one
    /// stuffing byte per three stuffed bytes. A longer declared length is a
    /// corrupted LEN; trusting it would swallow the packets after it.
    fn longest_body(&self) -> usize {
        let data = self.out.len();
        1 + data + 2 + data.div_ceil(3)
    }

    fn restart(&mut self) {
        self.state = State::Hunt(0);
        self.count = 0;
        self.stuffing = Stuffing::default();
        self.drop_next = false;
    }

    fn fail(&mut self, err: ProtocolError) -> Option<Result<Status, ProtocolError>> {
        self.restart();
        Some(Err(err))
    }

    /// Advances past one body byte, removing stuffing. Returns the byte, or
    /// `None` when it was a stuffing byte.
    fn unstuff(&mut self, byte: u8) -> Option<u8> {
        if self.drop_next {
            self.drop_next = false;
            return None;
        }
        self.drop_next = self.stuffing.push(byte);
        Some(byte)
    }
}

impl Parser for StatusParser<'_> {
    fn wanted(&self) -> usize {
        match self.state {
            State::Hunt(matched) => HEADER.len() - usize::from(matched) + 3,
            State::Id => 3,
            State::LenL => 2,
            State::LenH => 1,
            _ => self.remaining.max(1),
        }
    }

    fn on_deadline(&self) -> Option<ProtocolError> {
        self.malformed.then_some(ProtocolError::Malformed)
    }

    fn push(&mut self, byte: u8) -> Option<Result<Status, ProtocolError>> {
        if !matches!(
            self.state,
            State::Hunt(_) | State::Id | State::LenL | State::LenH
        ) {
            self.remaining -= 1;
        }
        if !matches!(self.state, State::CrcL | State::CrcH | State::Skip) {
            self.crc = crc::update_byte(self.crc, byte);
        }
        match self.state {
            State::Hunt(matched) => {
                let matched = usize::from(matched);
                self.state = if byte == HEADER[matched] {
                    State::Hunt(matched as u8 + 1)
                } else if byte == 0xFF {
                    State::Hunt(if matched == 3 {
                        1
                    } else {
                        matched.max(1) as u8
                    })
                } else {
                    State::Hunt(0)
                };
                if self.state == State::Hunt(4) {
                    self.state = State::Id;
                    self.crc = crc::update(0, &HEADER);
                }
            }
            State::Id => {
                self.id = byte;
                self.state = State::LenL;
            }
            State::LenL => {
                self.len_l = byte;
                self.state = State::LenH;
            }
            State::LenH => {
                self.remaining = usize::from(u16::from_le_bytes([self.len_l, byte]));
                if self.remaining < 3 {
                    self.malformed = true;
                    self.restart();
                } else {
                    self.state = State::Instruction;
                }
            }
            State::Instruction => {
                self.unstuff(byte);
                if byte != STATUS {
                    self.state = State::Skip;
                } else if self.remaining < 3 || self.remaining > self.longest_body() {
                    self.malformed = true;
                    self.restart();
                } else {
                    self.state = State::Error;
                }
            }
            State::Error => {
                self.unstuff(byte);
                self.error = byte;
                self.state = if self.remaining == 2 {
                    State::CrcL
                } else {
                    State::Data
                };
            }
            State::Data => {
                if let Some(b) = self.unstuff(byte) {
                    if let Some(slot) = self.out.get_mut(self.count) {
                        *slot = b;
                    }
                    self.count += 1;
                }
                if self.remaining == 2 {
                    self.state = State::CrcL;
                }
            }
            State::CrcL => {
                self.received_crc_l = byte;
                self.state = State::CrcH;
            }
            State::CrcH => {
                let received = u16::from_le_bytes([self.received_crc_l, byte]);
                if received != self.crc {
                    return self.fail(ProtocolError::Crc { id: self.id });
                }
                let status = Status {
                    id: self.id,
                    error: self.error,
                    len: self.count,
                };
                self.restart();
                return Some(Ok(status));
            }
            State::Skip => {
                if self.remaining == 0 {
                    self.restart();
                }
            }
        }
        None
    }
}

/// Address and length parameters, little-endian.
fn addr_len(addr: u16, len: u16) -> [u8; 4] {
    let [a_l, a_h] = addr.to_le_bytes();
    let [l_l, l_h] = len.to_le_bytes();
    [a_l, a_h, l_l, l_h]
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
    check::<P2>(status, id, out)
}

fn pong(id: Id, data: [u8; 3], fault: Option<StatusReport>) -> Pong {
    Pong {
        id,
        model_number: Some(u16::from_le_bytes([data[0], data[1]])),
        firmware: Some(data[2]),
        fault,
    }
}

/// Turns one reply to a broadcast Ping into a [`Pong`], or into the error
/// that says why it cannot be one. A set error byte wins over a wrong
/// length, as in [`check`].
fn scan_reply(status: Status, data: [u8; 3]) -> Result<Pong, Error> {
    let Ok(id) = Id::new(status.id) else {
        return Err(ProtocolError::Malformed.into());
    };
    let fault = (status.error != 0).then(|| P2::report(status.error));
    match (status.len, fault) {
        (3, fault) => Ok(pong(id, data, fault)),
        (_, Some(report)) => Err(StatusError { id, report }.into()),
        (got, None) => Err(ProtocolError::WrongLength { expected: 3, got }.into()),
    }
}

impl Exchange for P2 {
    fn ping<T: Transport, C: Clock>(link: &mut Link<'_, T, C>, id: Id) -> Result<Pong, Error> {
        let mut data = [0; 3];
        unicast(link, id, PING, iter::empty(), &mut data)?;
        Ok(pong(id, data, None))
    }

    fn read<T: Transport, C: Clock>(
        link: &mut Link<'_, T, C>,
        id: Id,
        addr: u16,
        out: &mut [u8],
    ) -> Result<(), Error> {
        let Some(len) = u16::try_from(out.len())
            .ok()
            .filter(|_| out.len() <= MAX_STATUS_DATA)
        else {
            return Err(ValidationError::PacketTooLarge {
                len: out.len(),
                max: MAX_STATUS_DATA,
            }
            .into());
        };
        unicast(link, id, READ, addr_len(addr, len).into_iter(), out)
    }

    fn write<T: Transport, C: Clock>(
        link: &mut Link<'_, T, C>,
        id: Id,
        addr: u16,
        data: &[u8],
    ) -> Result<(), Error> {
        let params = addr.to_le_bytes().into_iter().chain(data.iter().copied());
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
        if let Some((_, data)) = entries.clone().find(|(_, d)| d.len() != usize::from(size)) {
            return Err(ValidationError::DataLength {
                expected: usize::from(size),
                got: data.len(),
            }
            .into());
        }
        let params = addr_len(addr, size)
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
        drain(link)?;
        send(link.transport, BROADCAST, PING, iter::empty())?;
        let deadline = link.deadline();
        loop {
            let mut data = [0; 3];
            match receive(link, &mut StatusParser::new(&mut data), deadline, None) {
                Ok(status) => on_reply(scan_reply(status, data)),
                Err(Error::Transport(TransportError::Timeout { .. })) => return Ok(()),
                Err(err @ Error::Transport(_)) => return Err(err),
                Err(err) => on_reply(Err(err)),
            }
        }
    }

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
        F: FnMut(Id, Result<&[u8], Error>),
    {
        Self::sync_read(link, addr, size, ids, scratch, on_reply)
    }

    fn report(error: u8) -> StatusReport {
        StatusReport::P2 {
            alert: error & 0x80 != 0,
            code: P2Code::from_bits(error),
        }
    }
}

impl P2 {
    /// Reads `size` bytes at `addr` from every ID in `ids` with one Sync Read
    /// instruction.
    ///
    /// Replies come back in request order (e-Manual `#parameters-4`).
    /// `on_reply` is called exactly once per ID with that actuator's data,
    /// or its own error; each reply gets a full timeout. A reply from an ID
    /// later in the list means the IDs before it are not answering: they
    /// get a timeout and the reply goes to its own ID. A late reply from an
    /// ID that already timed out is discarded. `scratch` holds one
    /// reply and must be at least `size` bytes. Returns `Err` only when the
    /// transport itself fails.
    pub(crate) fn sync_read<T: Transport, C: Clock, I, F>(
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
        let size_usize = usize::from(size);
        if size_usize > MAX_STATUS_DATA {
            return Err(ValidationError::PacketTooLarge {
                len: size_usize,
                max: MAX_STATUS_DATA,
            }
            .into());
        }
        let Some(buf) = scratch.get_mut(..size_usize) else {
            return Err(ValidationError::DataLength {
                expected: size_usize,
                got: scratch.len(),
            }
            .into());
        };
        let params = addr_len(addr, size)
            .into_iter()
            .chain(ids.clone().map(Id::get));
        drain(link)?;
        send(link.transport, BROADCAST, SYNC_READ, params)?;
        let all = ids.clone();
        let mut held = None;
        let mut pending = ids;
        let mut done = 0;
        while let Some(id) = pending.next() {
            let already_handled = |raw: u8| {
                all.clone()
                    .take(done)
                    .any(|earlier| earlier != id && earlier.get() == raw)
            };
            let received = match held.take() {
                Some(status) => Ok(status),
                None => {
                    let deadline = link.deadline();
                    loop {
                        match receive(link, &mut StatusParser::new(buf), deadline, Some(id)) {
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
                        && pending.clone().any(|later| later.get() == status.id) =>
                {
                    held = Some(status);
                    on_reply(id, Err(TransportError::Timeout { id: Some(id) }.into()));
                }
                Ok(status) => on_reply(id, check::<P2>(status, id, buf).map(|()| &*buf)),
                Err(err @ Error::Transport(TransportError::Io(_))) => return Err(err),
                Err(err) => on_reply(id, Err(err)),
            }
        }
        Ok(())
    }

    /// Pings every actuator at once and calls `on_reply` for each answer
    /// that arrives within one timeout.
    ///
    /// The e-Manual does not specify reply order or timing (`#ping-0x01`),
    /// so all replies inside the window are collected. A reply whose error
    /// byte is set is still reported, since the actuator is present, with
    /// the error in [`Pong::fault`]. Corrupted replies are skipped; use
    /// [`Exchange::scan`] to see them. Returns `Err` only when the
    /// transport fails.
    pub(crate) fn broadcast_ping<T: Transport, C: Clock, F>(
        link: &mut Link<'_, T, C>,
        mut on_reply: F,
    ) -> Result<(), Error>
    where
        F: FnMut(Pong),
    {
        Self::scan(link, |reply| {
            if let Ok(pong) = reply {
                on_reply(pong);
            }
        })
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use core::time::Duration;

    use embedded_io::ErrorKind;
    use proptest::prelude::*;
    use std::vec::Vec;

    use super::*;
    use crate::error::StatusError;
    use crate::protocol::vectors::p2 as v;
    use crate::transport::{MockTime, MockTransport};

    fn id(raw: u8) -> Id {
        Id::new(raw).unwrap()
    }

    /// Encodes a packet into a vector (also builds status packets: a status
    /// packet is instruction 0x55 with the error byte as first parameter).
    pub(crate) fn packet(id: u8, instruction: u8, params: &[u8]) -> Vec<u8> {
        let mut buf = [0u8; 512];
        let mut w: &mut [u8] = &mut buf;
        send(&mut w, id, instruction, params.iter().copied()).unwrap();
        let n = 512 - w.len();
        buf[..n].to_vec()
    }

    /// Feeds `bytes` to a parser; returns the last result, if any.
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
        assert_eq!(packet(1, READ, &addr_len(132, 4)), v::READ);
        assert_eq!(
            packet(1, WRITE, &[0x74, 0x00, 0x00, 0x02, 0x00, 0x00]),
            v::WRITE
        );
        assert_eq!(packet(BROADCAST, PING, &[]), v::BROADCAST_PING);
        assert_eq!(packet(1, STATUS, &[0]), v::OK_REPLY);
    }

    #[test]
    fn decode_matches_emanual() {
        let mut out = [0; 3];
        let status = parse(v::PING_REPLY, &mut out).unwrap().unwrap();
        assert_eq!((status.id, status.error, status.len), (1, 0, 3));
        assert_eq!(pong(id(1), out, None).model_number, Some(1030));
        assert_eq!(pong(id(1), out, None).firmware, Some(38));

        let mut out = [0; 4];
        let status = parse(v::READ_REPLY_1, &mut out).unwrap().unwrap();
        assert_eq!((status.len, out), (4, [0xA6, 0, 0, 0]));

        let status = parse(v::OK_REPLY, &mut []).unwrap().unwrap();
        assert_eq!(
            status,
            Status {
                id: 1,
                error: 0,
                len: 0
            }
        );
    }

    #[test]
    fn stuffing_is_added_counted_and_removed() {
        let data = [0x01, 0xFF, 0xFF, 0xFD, 0x02];
        let sent = packet(1, WRITE, &data);
        let body = &sent[7..sent.len() - 2];
        assert_eq!(body, [WRITE, 0x01, 0xFF, 0xFF, 0xFD, 0xFD, 0x02]);
        assert_eq!(
            u16::from_le_bytes([sent[5], sent[6]]) as usize,
            body.len() + 2
        );
        let (head, tail) = sent.split_at(sent.len() - 2);
        assert_eq!(crc::update(0, head).to_le_bytes(), tail);

        let reply = packet(1, STATUS, &[0, 0xFF, 0xFF, 0xFD, 0x07]);
        let mut out = [0; 4];
        let status = parse(&reply, &mut out).unwrap().unwrap();
        assert_eq!((status.len, out), (4, [0xFF, 0xFF, 0xFD, 0x07]));
    }

    fn tricky_byte() -> impl Strategy<Value = u8> {
        prop_oneof![Just(0xFF), Just(0xFD), Just(0x00), any::<u8>()]
    }

    proptest! {
        #[test]
        fn status_round_trips(data in proptest::collection::vec(tricky_byte(), 0..64), err in any::<u8>()) {
            let mut params = std::vec![err];
            params.extend_from_slice(&data);
            let reply = packet(7, STATUS, &params);
            let mut out = [0; 64];
            let status = parse(&reply, &mut out).unwrap().unwrap();
            prop_assert_eq!((status.id, status.error, status.len), (7, err, data.len()));
            prop_assert_eq!(&out[..data.len()], &data[..]);
        }
    }

    #[test]
    fn every_single_byte_corruption_is_rejected() {
        for packet in v::STATUS {
            for pos in 4..packet.len() {
                for flip in [0x01, 0x80, 0xFF] {
                    let mut bad = packet.to_vec();
                    bad[pos] ^= flip;
                    let mut out = [0; 64];
                    let result = parse(&bad, &mut out);
                    assert!(
                        !matches!(result, Some(Ok(_))),
                        "accepted {bad:02X?} (pos {pos})"
                    );
                    if !(5..=7).contains(&pos) {
                        assert!(
                            matches!(result, Some(Err(ProtocolError::Crc { .. }))),
                            "{bad:02X?} (pos {pos}) gave {result:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn resyncs_after_garbage_and_skips_echoed_instructions() {
        let mut stream = std::vec![0x00, 0xFF, 0xFF, 0xFF, 0x12];
        stream.extend_from_slice(v::READ);
        stream.extend_from_slice(v::READ_REPLY_1);
        let mut out = [0; 4];
        let status = parse(&stream, &mut out).unwrap().unwrap();
        assert_eq!((status.id, out), (1, [0xA6, 0, 0, 0]));
    }

    #[test]
    fn impossible_length_resyncs_to_the_real_reply() {
        let mut stream = std::vec![0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x02, 0x00];
        stream.extend_from_slice(v::READ_REPLY_1);
        let mut out = [0; 4];
        let status = parse(&stream, &mut out).unwrap().unwrap();
        assert_eq!((status.id, out), (1, [0xA6, 0, 0, 0]));
        let short_status = [0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x03, 0x00, 0x55, 0x00, 0x00];
        assert_eq!(parse(&short_status, &mut []), None);
    }

    #[test]
    fn only_impossible_lengths_report_malformed_at_the_deadline() {
        let time = MockTime::new();
        let mut clock = time.ticker(1);
        let mut line = MockTransport::new(&time);
        line.reply(&[0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x02, 0x00]);
        assert_eq!(
            P2::ping(&mut link!(line, clock), id(1)),
            Err(ProtocolError::Malformed.into())
        );
    }

    #[test]
    fn echoed_ping_is_skipped() {
        let time = MockTime::new();
        let mut clock = time.ticker(1);
        let mut line = MockTransport::new(&time);
        line.reply(v::PING).reply(v::PING_REPLY);
        let pong = P2::ping(&mut link!(line, clock), id(1)).unwrap();
        assert_eq!(pong.model_number, Some(1030));
    }

    fn sync_read_with(ids: &[u8], replies: &[&[u8]]) -> Vec<(u8, Result<Vec<u8>, Error>)> {
        let time = MockTime::new();
        let mut clock = time.ticker(100);
        let mut line = MockTransport::new(&time);
        for reply in replies {
            line.reply(reply);
        }
        let mut got = Vec::new();
        let mut scratch = [0; 4];
        P2::sync_read(
            &mut link!(line, clock),
            132,
            4,
            ids.iter().map(|&raw| id(raw)),
            &mut scratch,
            |id, r| got.push((id.get(), r.map(<[u8]>::to_vec))),
        )
        .unwrap();
        got
    }

    #[test]
    fn sync_read_gives_a_silent_first_actuator_a_timeout_not_the_next_reply() {
        assert_eq!(
            sync_read_with(&[1, 2], &[v::READ_REPLY_2]),
            [
                (1, Err(TransportError::Timeout { id: Some(id(1)) }.into())),
                (2, Ok(std::vec![0x1F, 0x08, 0, 0]))
            ]
        );
    }

    #[test]
    fn sync_read_gives_a_silent_middle_actuator_a_timeout() {
        assert_eq!(
            sync_read_with(&[1, 3, 2], &[v::READ_REPLY_1, v::READ_REPLY_2]),
            [
                (1, Ok(std::vec![0xA6, 0, 0, 0])),
                (3, Err(TransportError::Timeout { id: Some(id(3)) }.into())),
                (2, Ok(std::vec![0x1F, 0x08, 0, 0]))
            ]
        );
    }

    #[test]
    fn sync_read_reports_a_reply_from_an_unlisted_id_as_wrong_id() {
        assert_eq!(
            sync_read_with(&[1], &[v::READ_REPLY_2]),
            [(
                1,
                Err(ProtocolError::WrongId {
                    expected: id(1),
                    got: 2
                }
                .into())
            )]
        );
    }

    #[test]
    fn sync_read_refuses_sizes_no_status_packet_can_carry() {
        let time = MockTime::new();
        let mut clock = time.ticker(1);
        let mut line = MockTransport::new(&time);
        let mut scratch = [0; 4];
        assert!(matches!(
            P2::sync_read(
                &mut link!(line, clock),
                0,
                u16::MAX,
                [id(1)].into_iter(),
                &mut scratch,
                |_, _| {}
            ),
            Err(Error::Validation(ValidationError::PacketTooLarge { .. }))
        ));
        assert!(line.sent().is_empty());
    }

    #[test]
    fn ping_read_write_exchanges() {
        let time = MockTime::new();
        let mut clock = time.ticker(1);

        let mut line = MockTransport::new(&time);
        line.reply(v::PING_REPLY);
        let pong = P2::ping(&mut link!(line, clock), id(1)).unwrap();
        assert_eq!((pong.model_number, pong.firmware), (Some(1030), Some(38)));
        assert_eq!(line.sent(), v::PING);

        let mut line = MockTransport::new(&time);
        line.reply(v::READ_REPLY_1);
        let mut out = [0; 4];
        P2::read(&mut link!(line, clock), id(1), 132, &mut out).unwrap();
        assert_eq!((out, line.sent()), ([0xA6, 0, 0, 0], v::READ));

        let mut line = MockTransport::new(&time);
        line.reply(v::OK_REPLY);
        P2::write(&mut link!(line, clock), id(1), 116, &512u32.to_le_bytes()).unwrap();
        assert_eq!(line.sent(), v::WRITE);
    }

    #[test]
    fn replies_from_another_id_or_of_another_size_are_rejected() {
        let time = MockTime::new();
        let mut clock = time.ticker(1);

        let mut line = MockTransport::new(&time);
        line.reply(v::READ_REPLY_2);
        let mut out = [0; 4];
        assert_eq!(
            P2::read(&mut link!(line, clock), id(1), 132, &mut out),
            Err(ProtocolError::WrongId {
                expected: id(1),
                got: 2
            }
            .into())
        );

        let mut line = MockTransport::new(&time);
        line.reply(&packet(1, STATUS, &[0, 0xA6, 0]));
        let mut out = [0; 4];
        assert_eq!(
            P2::read(&mut link!(line, clock), id(1), 132, &mut out),
            Err(ProtocolError::WrongLength {
                expected: 4,
                got: 2
            }
            .into())
        );

        let mut line = MockTransport::new(&time);
        line.reply(v::READ_REPLY_1);
        let mut out = [0; 1];
        assert_eq!(
            P2::read(&mut link!(line, clock), id(1), 132, &mut out),
            Err(ProtocolError::Malformed.into())
        );
    }

    #[test]
    fn alert_reply_keeps_its_data() {
        let time = MockTime::new();
        let mut clock = time.ticker(1);
        let mut line = MockTransport::new(&time);
        line.reply(&packet(1, STATUS, &[0x80, 0x20]));
        let mut out = [0];
        let alert = StatusReport::P2 {
            alert: true,
            code: None,
        };
        assert_eq!(
            P2::read(&mut link!(line, clock), id(1), 70, &mut out),
            Err(StatusError {
                id: id(1),
                report: alert
            }
            .into())
        );
        assert_eq!(out, [0x20]);
    }

    #[test]
    fn error_reply_without_data_wins_over_its_length_and_zeroes_out() {
        let time = MockTime::new();
        let mut clock = time.ticker(1);
        let mut line = MockTransport::new(&time);
        line.reply(&packet(1, STATUS, &[0x07]));
        let mut out = [0xAA; 4];
        let err = P2::read(&mut link!(line, clock), id(1), 132, &mut out).unwrap_err();
        assert!(matches!(err, Error::Status(_)), "{err:?}");
        assert_eq!(out, [0; 4]);
    }

    #[test]
    fn corrupted_length_does_not_swallow_the_next_reply() {
        let mut bad = v::READ_REPLY_1.to_vec();
        bad[6] ^= 0x80;
        assert_eq!(
            sync_read_with(&[1, 2], &[&bad, v::READ_REPLY_2]),
            [
                (1, Err(TransportError::Timeout { id: Some(id(1)) }.into())),
                (2, Ok(std::vec![0x1F, 0x08, 0, 0]))
            ]
        );
    }

    #[test]
    fn sync_read_discards_a_late_reply_from_a_timed_out_id() {
        let time = MockTime::new();
        let mut clock = time.ticker(100);
        let mut line = MockTransport::new(&time);
        line.reply_at(15_000, v::READ_REPLY_1)
            .reply_at(15_000, v::READ_REPLY_2);
        let mut got = Vec::new();
        let mut scratch = [0; 4];
        P2::sync_read(
            &mut link!(line, clock),
            132,
            4,
            [id(1), id(2)].into_iter(),
            &mut scratch,
            |id, r| got.push((id.get(), r.map(<[u8]>::to_vec))),
        )
        .unwrap();
        assert_eq!(
            got,
            [
                (1, Err(TransportError::Timeout { id: Some(id(1)) }.into())),
                (2, Ok(std::vec![0x1F, 0x08, 0, 0]))
            ]
        );
        assert_eq!(line.unread(), 0);
    }

    #[test]
    fn broadcast_ping_reports_a_set_error_byte_as_a_fault() {
        let time = MockTime::new();
        let mut clock = time.ticker(10);
        let mut line = MockTransport::new(&time);
        line.reply(&packet(1, STATUS, &[0x80, 0x06, 0x04, 0x26]));
        let mut got = Vec::new();
        P2::broadcast_ping(&mut link!(line, clock), |p| got.push(p)).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(
            got[0].fault,
            Some(StatusReport::P2 {
                alert: true,
                code: None
            })
        );
        assert_eq!(got[0].model_number, Some(1030));
    }

    #[test]
    fn status_error_byte_becomes_a_typed_error() {
        let time = MockTime::new();
        let mut clock = time.ticker(1);
        let mut line = MockTransport::new(&time);
        line.reply(&packet(3, STATUS, &[0x87]));
        let err = P2::write(&mut link!(line, clock), id(3), 116, &[0; 4]).unwrap_err();
        assert_eq!(
            err,
            StatusError {
                id: id(3),
                report: StatusReport::P2 {
                    alert: true,
                    code: Some(P2Code::Access)
                }
            }
            .into()
        );
    }

    #[test]
    fn oversized_packets_are_refused_before_sending() {
        let time = MockTime::new();
        let mut clock = time.ticker(1);
        let mut line = MockTransport::new(&time);
        let mut huge = [0u8; MAX_STATUS_DATA + 1];
        assert_eq!(
            P2::read(&mut link!(line, clock), id(1), 0, &mut huge),
            Err(ValidationError::PacketTooLarge {
                len: MAX_STATUS_DATA + 1,
                max: MAX_STATUS_DATA
            }
            .into())
        );
        let data = [0u8; MAX_LEN];
        assert!(matches!(
            P2::write(&mut link!(line, clock), id(1), 0, &data),
            Err(Error::Validation(ValidationError::PacketTooLarge { .. }))
        ));
        assert!(line.sent().is_empty());
    }

    #[test]
    fn sync_write_matches_emanual_and_never_reads() {
        let time = MockTime::new();
        let mut clock = time.ticker(1);
        let mut line = MockTransport::new(&time);
        let a = 150u32.to_le_bytes();
        let b = 170u32.to_le_bytes();
        let entries = [(id(1), &a[..]), (id(2), &b[..])];
        P2::sync_write(&mut link!(line, clock), 116, 4, entries.into_iter()).unwrap();
        assert_eq!((line.sent(), line.reads()), (v::SYNC_WRITE, 0));

        let short = [(id(1), &a[..2])];
        let mut line = MockTransport::new(&time);
        assert_eq!(
            P2::sync_write(&mut link!(line, clock), 116, 4, short.into_iter()),
            Err(ValidationError::DataLength {
                expected: 4,
                got: 2
            }
            .into())
        );
        assert!(line.sent().is_empty());
    }

    #[test]
    fn sync_read_delivers_each_reply_in_order() {
        let time = MockTime::new();
        let mut clock = time.ticker(1);
        let mut line = MockTransport::new(&time);
        line.reply(v::READ_REPLY_1).reply(v::READ_REPLY_2);
        let mut got = Vec::new();
        let mut scratch = [0; 4];
        P2::sync_read(
            &mut link!(line, clock),
            132,
            4,
            [id(1), id(2)].into_iter(),
            &mut scratch,
            |id, r| got.push((id.get(), r.map(<[u8]>::to_vec))),
        )
        .unwrap();
        assert_eq!(line.sent(), v::SYNC_READ);
        assert_eq!(
            got,
            [
                (1, Ok(std::vec![0xA6, 0, 0, 0])),
                (2, Ok(std::vec![0x1F, 0x08, 0, 0]))
            ]
        );
    }

    #[test]
    fn sync_read_reports_a_silent_actuator_and_carries_on() {
        let time = MockTime::new();
        let mut clock = time.ticker(100);
        let mut line = MockTransport::new(&time);
        line.reply(v::READ_REPLY_1);
        let mut got = Vec::new();
        let mut scratch = [0; 4];
        P2::sync_read(
            &mut link!(line, clock),
            132,
            4,
            [id(1), id(2)].into_iter(),
            &mut scratch,
            |id, r| got.push((id.get(), r.map(<[u8]>::to_vec))),
        )
        .unwrap();
        assert_eq!(
            got,
            [
                (1, Ok(std::vec![0xA6, 0, 0, 0])),
                (2, Err(TransportError::Timeout { id: Some(id(2)) }.into()))
            ]
        );
    }

    #[test]
    fn sync_read_fails_on_transport_error_and_small_scratch() {
        let time = MockTime::new();
        let mut clock = time.ticker(1);
        let mut line = MockTransport::new(&time);
        let mut scratch = [0; 2];
        assert!(matches!(
            P2::sync_read(
                &mut link!(line, clock),
                132,
                4,
                [id(1)].into_iter(),
                &mut scratch,
                |_, _| {}
            ),
            Err(Error::Validation(ValidationError::DataLength { .. }))
        ));
        line.fail_with(ErrorKind::BrokenPipe);
        let mut scratch = [0; 4];
        assert_eq!(
            P2::sync_read(
                &mut link!(line, clock),
                132,
                4,
                [id(1)].into_iter(),
                &mut scratch,
                |_, _| {}
            ),
            Err(TransportError::Io(ErrorKind::BrokenPipe).into())
        );
    }

    #[test]
    fn broadcast_ping_collects_every_reply_in_the_window() {
        let time = MockTime::new();
        let mut clock = time.ticker(10);
        let mut line = MockTransport::new(&time);
        line.reply(v::PING_REPLY)
            .reply(&[
                0xFF, 0xFF, 0xFD, 0x00, 0x09, 0x07, 0x00, 0x55, 0x00, 1, 2, 3, 0, 0,
            ])
            .reply(v::BROADCAST_PING_REPLY_2);
        let mut got = Vec::new();
        P2::broadcast_ping(&mut link!(line, clock), |p| got.push(p)).unwrap();
        assert_eq!(line.sent(), v::BROADCAST_PING);
        let ids: Vec<_> = got.iter().map(|p| p.id.get()).collect();
        assert_eq!(ids, [1, 2]);
        assert!(got.iter().all(|p| p.model_number == Some(1030)));

        let mut line = MockTransport::new(&time);
        let mut count = 0;
        P2::broadcast_ping(&mut link!(line, clock), |_| count += 1).unwrap();
        assert_eq!(count, 0);

        let mut line = MockTransport::new(&time);
        line.fail_with(ErrorKind::BrokenPipe);
        assert!(P2::broadcast_ping(&mut link!(line, clock), |_| {}).is_err());
    }

    fn scan_with(replies: &[&[u8]]) -> Vec<Result<Pong, Error>> {
        let time = MockTime::new();
        let mut clock = time.ticker(10);
        let mut line = MockTransport::new(&time);
        for reply in replies {
            line.reply(reply);
        }
        let mut got = Vec::new();
        P2::scan(&mut link!(line, clock), |r| got.push(r)).unwrap();
        assert_eq!(line.sent(), v::BROADCAST_PING);
        got
    }

    #[test]
    fn scan_sends_one_broadcast_ping_and_reports_every_reply() {
        assert_eq!(
            scan_with(&[v::PING_REPLY, v::BROADCAST_PING_REPLY_2]),
            [
                Ok(pong(id(1), [0x06, 0x04, 0x26], None)),
                Ok(pong(id(2), [0x06, 0x04, 0x26], None))
            ]
        );
        assert_eq!(scan_with(&[]), []);
    }

    #[test]
    fn scan_reports_a_set_error_byte_as_a_fault() {
        let alert = StatusReport::P2 {
            alert: true,
            code: None,
        };
        assert_eq!(
            scan_with(&[
                &packet(1, STATUS, &[0x80, 0x06, 0x04, 0x26]),
                v::BROADCAST_PING_REPLY_2
            ]),
            [
                Ok(pong(id(1), [0x06, 0x04, 0x26], Some(alert))),
                Ok(pong(id(2), [0x06, 0x04, 0x26], None))
            ]
        );
    }

    #[test]
    fn scan_passes_corrupted_replies_on_and_carries_on() {
        let mut corrupted = v::PING_REPLY.to_vec();
        *corrupted.last_mut().unwrap() ^= 0xFF;
        assert_eq!(
            scan_with(&[&corrupted, v::BROADCAST_PING_REPLY_2]),
            [
                Err(ProtocolError::Crc { id: 1 }.into()),
                Ok(pong(id(2), [0x06, 0x04, 0x26], None))
            ]
        );
    }

    #[test]
    fn scan_reports_replies_that_cannot_be_a_pong() {
        assert_eq!(
            scan_with(&[
                &packet(3, STATUS, &[0x01]),
                &packet(4, STATUS, &[0x00, 0x06]),
                &packet(253, STATUS, &[0x00, 0x06, 0x04, 0x26]),
                &[0xFF, 0xFF, 0xFD, 0x00, 0x05, 0x02, 0x00],
            ]),
            [
                Err(StatusError {
                    id: id(3),
                    report: StatusReport::P2 {
                        alert: false,
                        code: Some(P2Code::ResultFail)
                    }
                }
                .into()),
                Err(ProtocolError::WrongLength {
                    expected: 3,
                    got: 1
                }
                .into()),
                Err(ProtocolError::Malformed.into()),
                Err(ProtocolError::Malformed.into()),
            ]
        );
    }

    #[test]
    fn read_many_is_one_sync_read() {
        let time = MockTime::new();
        let mut clock = time.ticker(100);
        let mut line = MockTransport::new(&time);
        line.reply(v::READ_REPLY_1).reply(v::READ_REPLY_2);
        let mut got = Vec::new();
        let mut scratch = [0; 4];
        P2::read_many(
            &mut link!(line, clock),
            132,
            4,
            [id(1), id(3), id(2)].into_iter(),
            &mut scratch,
            |id, r| got.push((id.get(), r.map(<[u8]>::to_vec))),
        )
        .unwrap();
        assert_eq!(
            got,
            [
                (1, Ok(std::vec![0xA6, 0, 0, 0])),
                (3, Err(TransportError::Timeout { id: Some(id(3)) }.into())),
                (2, Ok(std::vec![0x1F, 0x08, 0, 0]))
            ]
        );
        assert_eq!(
            line.sent(),
            packet(BROADCAST, SYNC_READ, &[0x84, 0, 4, 0, 1, 3, 2])
        );

        let mut line = MockTransport::new(&time);
        line.fail_with(ErrorKind::BrokenPipe);
        assert_eq!(
            P2::read_many(
                &mut link!(line, clock),
                132,
                4,
                [id(1)].into_iter(),
                &mut scratch,
                |_, _| panic!("no reply expected")
            ),
            Err(TransportError::Io(ErrorKind::BrokenPipe).into())
        );
    }
}
