//! Dynamixel Protocol 2.0: <https://emanual.robotis.com/docs/en/dxl/protocol2/>.
//!
//! Instruction packet: `FF FF FD 00 | ID | LEN_L LEN_H | INST | params | CRC_L CRC_H`.
//! Status packet: the same with `INST = 0x55` followed by an error byte.
//! `LEN` counts every byte after itself, stuffing included.

use core::iter;

use super::{
    Clock, Defect, Exchange, Link, Parser, Pong, Protocol, ProtocolVersion, Receive, Status,
    Transport, Writer, check, collect, crc, drain, receive, repeated_id, sealed,
};
use crate::Id;
use crate::error::{
    Error, Instruction, P2Code, ProtocolError, Request, StatusError, StatusReport, ValidationError,
};

/// Dynamixel Protocol 2.0.
///
/// Has instructions Protocol 1 lacks: Sync Read, Bulk Write, Reboot and
/// broadcast Ping.
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
const REBOOT: u8 = 0x08;
const BULK_READ: u8 = 0x92;
const BULK_WRITE: u8 = 0x93;

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
pub(super) fn send<T: Transport, C: Clock, I>(
    link: &mut Link<'_, T, C>,
    id: u8,
    instruction: u8,
    params: I,
) -> Result<(), Error>
where
    I: Iterator<Item = u8> + Clone,
{
    let request = link.request;
    let mut stuffing = Stuffing::default();
    let body: usize = iter::once(instruction)
        .chain(params.clone())
        .map(|b| 1 + usize::from(stuffing.push(b)))
        .sum();
    let len = body + 2;
    let Ok(len_field) = u16::try_from(len) else {
        return Err(ValidationError::PacketTooLarge {
            len,
            max: MAX_LEN,
            request,
        }
        .into());
    };

    let mut w = Writer::new(link.transport, request);
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
    /// The last header with an impossible `LEN` that was skipped.
    malformed: Option<Defect>,
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
            malformed: None,
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

    fn fail(&mut self, err: Defect) -> Option<Result<Status, Defect>> {
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

    fn on_deadline(&self) -> Option<Defect> {
        self.malformed
    }

    fn push(&mut self, byte: u8) -> Option<Result<Status, Defect>> {
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
                    self.malformed = Some(Defect::TooShort {
                        protocol: ProtocolVersion::P2,
                        length: u16::from_le_bytes([self.len_l, byte]),
                        min: 3,
                    });
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
                    let length = u16::try_from(self.remaining + 1).unwrap_or(u16::MAX);
                    self.malformed = Some(if self.remaining < 3 {
                        Defect::TooShort {
                            protocol: ProtocolVersion::P2,
                            length,
                            min: 3,
                        }
                    } else {
                        Defect::TooLong {
                            length,
                            max: u16::try_from(self.longest_body() + 1).unwrap_or(u16::MAX),
                        }
                    });
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
                    return self.fail(Defect::Crc {
                        packet_id: self.id,
                        computed: self.crc,
                        received,
                    });
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
///
/// A packet from another ID is a late reply to an earlier instruction (its
/// actuator answered after that exchange timed out, e.g. behind a USB
/// adapter's latency timer). It is dropped and the wait goes on until the
/// deadline, so it is never taken for this reply.
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
    send(link, id.get(), instruction, params)?;
    let deadline = link.deadline();
    let status = loop {
        let status = receive(link, &mut StatusParser::new(out), deadline)?;
        if status.id == id.get() {
            break status;
        }
    };
    check::<P2>(status, id, out, link.request)
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
fn scan_reply(status: Status, data: [u8; 3], request: Request) -> Result<Pong, Error> {
    let Ok(id) = Id::new(status.id) else {
        return Err(ProtocolError::InvalidReplyId {
            request,
            got: status.id,
        }
        .into());
    };
    let request = request.id(id);
    let fault = (status.error != 0).then(|| P2::report(status.error));
    match (status.len, fault) {
        (3, fault) => Ok(pong(id, data, fault)),
        (_, Some(report)) => Err(StatusError {
            id,
            report,
            request,
        }
        .into()),
        (got, None) => Err(ProtocolError::WrongLength {
            request,
            expected: 3,
            got,
        }
        .into()),
    }
}

/// `PacketTooLarge` for a reply of `len` data bytes, if no status packet
/// can carry it.
fn check_reply_len(len: usize, request: Request) -> Result<u16, Error> {
    u16::try_from(len)
        .ok()
        .filter(|_| len <= MAX_STATUS_DATA)
        .ok_or(
            ValidationError::PacketTooLarge {
                len,
                max: MAX_STATUS_DATA,
                request,
            }
            .into(),
        )
}

impl Exchange for P2 {
    fn ping<T: Transport, C: Clock>(link: &mut Link<'_, T, C>, id: Id) -> Result<Pong, Error> {
        let mut data = [0; 3];
        let request = Request::NONE.instruction(Instruction::Ping).id(id);
        link.with(request, |link| {
            unicast(link, id, PING, iter::empty(), &mut data)
        })?;
        Ok(pong(id, data, None))
    }

    fn read<T: Transport, C: Clock>(
        link: &mut Link<'_, T, C>,
        id: Id,
        addr: u16,
        out: &mut [u8],
    ) -> Result<(), Error> {
        let request = Request::NONE
            .instruction(Instruction::Read)
            .id(id)
            .addr(addr)
            .len(Request::len_of(out.len()));
        link.with(request, |link| {
            let len = check_reply_len(out.len(), link.request)?;
            unicast(link, id, READ, addr_len(addr, len).into_iter(), out)
        })
    }

    fn write<T: Transport, C: Clock>(
        link: &mut Link<'_, T, C>,
        id: Id,
        addr: u16,
        data: &[u8],
    ) -> Result<(), Error> {
        let request = Request::NONE
            .instruction(Instruction::Write)
            .id(id)
            .addr(addr)
            .len(Request::len_of(data.len()));
        let params = addr.to_le_bytes().into_iter().chain(data.iter().copied());
        link.with(request, |link| unicast(link, id, WRITE, params, &mut []))
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
        let request = Request::NONE
            .instruction(Instruction::SyncWrite)
            .addr(addr)
            .len(size);
        link.with(request, |link| {
            if let Some((id, data)) = entries.clone().find(|(_, d)| d.len() != usize::from(size)) {
                return Err(ValidationError::DataLength {
                    expected: usize::from(size),
                    got: data.len(),
                    request: link.request.id(id),
                }
                .into());
            }
            let params = addr_len(addr, size).into_iter().chain(
                entries.flat_map(|(id, data)| iter::once(id.get()).chain(data.iter().copied())),
            );
            drain(link)?;
            send(link, BROADCAST, SYNC_WRITE, params)
        })
    }

    fn scan<T: Transport, C: Clock, F>(
        link: &mut Link<'_, T, C>,
        mut on_reply: F,
    ) -> Result<(), Error>
    where
        F: FnMut(Result<Pong, Error>),
    {
        link.with(Request::NONE.instruction(Instruction::Ping), |link| {
            drain(link)?;
            send(link, BROADCAST, PING, iter::empty())?;
            let deadline = link.deadline();
            loop {
                let mut data = [0; 3];
                match receive(link, &mut StatusParser::new(&mut data), deadline) {
                    Ok(status) => on_reply(scan_reply(status, data, link.request)),
                    Err(Error::Transport(err)) if err.is_timeout() => return Ok(()),
                    Err(err @ Error::Transport(_)) => return Err(err),
                    Err(err) => on_reply(Err(err)),
                }
            }
        })
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

    fn bulk_read<T: Transport, C: Clock, I, F>(
        link: &mut Link<'_, T, C>,
        entries: I,
        scratch: &mut [u8],
        on_reply: F,
    ) -> Result<(), Error>
    where
        I: Iterator<Item = (Id, u16, u16)> + Clone,
        F: FnMut(Id, Result<&[u8], Error>),
    {
        if let Some(id) = repeated_id(entries.clone().map(|(id, _, _)| id)) {
            return Err(ValidationError::DuplicateId { id }.into());
        }
        link.with(Request::NONE.instruction(Instruction::BulkRead), |link| {
            for (id, addr, size) in entries.clone() {
                check_reply_size(
                    usize::from(size),
                    scratch,
                    link.request.id(id).addr(addr).len(size),
                )?;
            }
            let params = entries
                .clone()
                .flat_map(|(id, addr, size)| iter::once(id.get()).chain(addr_len(addr, size)));
            drain(link)?;
            send(link, BROADCAST, BULK_READ, params)?;
            collect::<Self, T, C, _, F>(
                link,
                entries.map(|(id, addr, size)| (id, addr, usize::from(size))),
                scratch,
                on_reply,
            )
        })
    }

    fn report(error: u8) -> StatusReport {
        StatusReport::P2 {
            alert: error & 0x80 != 0,
            code: P2Code::from_bits(error),
        }
    }
}

impl Receive for P2 {
    fn receive_status<T: Transport, C: Clock>(
        link: &mut Link<'_, T, C>,
        out: &mut [u8],
        deadline: u64,
    ) -> Result<Status, Error> {
        receive(link, &mut StatusParser::new(out), deadline)
    }
}

/// Checks that a reply of `size` data bytes fits a status packet and
/// `scratch`.
fn check_reply_size(size: usize, scratch: &[u8], request: Request) -> Result<(), Error> {
    check_reply_len(size, request)?;
    if scratch.len() < size {
        return Err(ValidationError::DataLength {
            expected: size,
            got: scratch.len(),
            request,
        }
        .into());
    }
    Ok(())
}

impl P2 {
    /// Reads `size` bytes at `addr` from every ID in `ids` with one Sync Read
    /// instruction.
    ///
    /// Replies come back in request order (e-Manual `#parameters-4`) and are
    /// collected as `protocol::collect` says: `on_reply` is called exactly
    /// once per ID with that actuator's data, or its own error. `scratch`
    /// holds one reply and must be at least `size` bytes. Returns `Err` only
    /// for an invalid request, before anything is sent, or when the
    /// transport itself fails.
    pub(crate) fn sync_read<T: Transport, C: Clock, I, F>(
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
        let request = Request::NONE
            .instruction(Instruction::SyncRead)
            .addr(addr)
            .len(size);
        link.with(request, |link| {
            let size_usize = usize::from(size);
            check_reply_size(size_usize, scratch, link.request)?;
            let params = addr_len(addr, size)
                .into_iter()
                .chain(ids.clone().map(Id::get));
            drain(link)?;
            send(link, BROADCAST, SYNC_READ, params)?;
            collect::<Self, T, C, _, F>(
                link,
                ids.map(|id| (id, addr, size_usize)),
                scratch,
                on_reply,
            )
        })
    }

    /// Writes each `(id, addr, data)` entry with one Bulk Write instruction
    /// (e-Manual `#bulk-write-0x93`): addresses and sizes may differ per
    /// actuator. Broadcast, so no reply and no wait.
    ///
    /// # Errors
    ///
    /// Before anything is sent: `DuplicateId` for an ID listed twice,
    /// `PacketTooLarge` for data no packet can carry. Then only a failing
    /// transport.
    pub(crate) fn bulk_write<'d, T: Transport, C: Clock, I>(
        link: &mut Link<'_, T, C>,
        entries: I,
    ) -> Result<(), Error>
    where
        I: Iterator<Item = (Id, u16, &'d [u8])> + Clone,
    {
        if let Some(id) = repeated_id(entries.clone().map(|(id, _, _)| id)) {
            return Err(ValidationError::DuplicateId { id }.into());
        }
        link.with(Request::NONE.instruction(Instruction::BulkWrite), |link| {
            if let Some((id, addr, data)) = entries
                .clone()
                .find(|(_, _, data)| u16::try_from(data.len()).is_err())
            {
                return Err(ValidationError::PacketTooLarge {
                    len: data.len(),
                    max: MAX_LEN,
                    request: link.request.id(id).addr(addr),
                }
                .into());
            }
            let params = entries.flat_map(|(id, addr, data)| {
                let len = u16::try_from(data.len()).unwrap_or(u16::MAX);
                iter::once(id.get())
                    .chain(addr_len(addr, len))
                    .chain(data.iter().copied())
            });
            drain(link)?;
            send(link, BROADCAST, BULK_WRITE, params)
        })
    }

    /// Reboots `id` (e-Manual `#reboot-0x08`) and waits for its status
    /// packet, sent before the actuator restarts.
    pub(crate) fn reboot<T: Transport, C: Clock>(
        link: &mut Link<'_, T, C>,
        id: Id,
    ) -> Result<(), Error> {
        let request = Request::NONE.instruction(Instruction::Reboot).id(id);
        link.with(request, |link| {
            unicast(link, id, REBOOT, iter::empty(), &mut [])
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
    use crate::error::{Direction, StatusError, TransportError};
    use crate::protocol::vectors::p2 as v;
    use crate::transport::{MockTime, MockTransport};

    fn id(raw: u8) -> Id {
        Id::new(raw).unwrap()
    }

    macro_rules! link {
        ($line:expr, $clock:expr) => {
            Link {
                transport: &mut $line,
                clock: &mut $clock,
                timeout: Duration::from_millis(10),
                request: Request::NONE,
            }
        };
    }

    /// The unicast timeout every `link!` uses, in microseconds.
    const TIMEOUT_US: u32 = 10_000;

    fn req(instruction: Instruction) -> Request {
        Request::NONE.instruction(instruction)
    }

    /// The request of a Sync Read of Present Position (132, 4 bytes) for
    /// target `raw`.
    fn sync_req(raw: u8) -> Request {
        req(Instruction::SyncRead).id(id(raw)).addr(132).len(4)
    }

    fn no_reply(request: Request) -> Error {
        TransportError::NoReply {
            request,
            timeout_us: TIMEOUT_US,
        }
        .into()
    }

    fn skipped(request: Request, answered: u8) -> Error {
        TransportError::Skipped {
            request,
            answered: id(answered),
        }
        .into()
    }

    fn broken_pipe(request: Request) -> Error {
        TransportError::Io {
            request,
            direction: Direction::ReadReady,
            kind: ErrorKind::BrokenPipe,
        }
        .into()
    }

    /// Encodes a packet into a vector (also builds status packets: a status
    /// packet is instruction 0x55 with the error byte as first parameter).
    pub(crate) fn packet(id: u8, instruction: u8, params: &[u8]) -> Vec<u8> {
        let time = MockTime::new();
        let mut clock = time.ticker(1);
        let mut line = MockTransport::new(&time);
        send(
            &mut link!(line, clock),
            id,
            instruction,
            params.iter().copied(),
        )
        .unwrap();
        line.sent().to_vec()
    }

    /// Feeds `bytes` to a parser; returns the last result, if any.
    fn parse(bytes: &[u8], out: &mut [u8]) -> Option<Result<Status, Defect>> {
        let mut parser = StatusParser::new(out);
        let mut last = None;
        for &b in bytes {
            if let Some(r) = parser.push(b) {
                last = Some(r);
            }
        }
        last
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
                            matches!(result, Some(Err(Defect::Crc { .. }))),
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
    fn only_impossible_lengths_report_a_short_length_at_the_deadline() {
        let time = MockTime::new();
        let mut clock = time.ticker(1);
        let mut line = MockTransport::new(&time);
        line.reply(&[0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x02, 0x00]);
        assert_eq!(
            P2::ping(&mut link!(line, clock), id(1)),
            Err(ProtocolError::LengthTooShort {
                request: req(Instruction::Ping).id(id(1)),
                protocol: ProtocolVersion::P2,
                length: 2,
                min: 3,
            }
            .into())
        );
    }

    #[test]
    fn short_and_long_length_fields_are_told_apart() {
        let mut out = [0; 1];
        let mut parser = StatusParser::new(&mut out);
        for &b in &[0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x02, 0x00] {
            assert_eq!(parser.push(b), None);
        }
        assert_eq!(
            parser.on_deadline(),
            Some(Defect::TooShort {
                protocol: ProtocolVersion::P2,
                length: 2,
                min: 3
            })
        );
        let mut out = [0; 1];
        let mut parser = StatusParser::new(&mut out);
        for &b in v::READ_REPLY_1 {
            assert_eq!(parser.push(b), None);
        }
        assert_eq!(
            parser.on_deadline(),
            Some(Defect::TooLong { length: 8, max: 6 })
        );
    }

    #[test]
    fn crc_mismatch_names_computed_and_received() {
        let mut bad = v::READ_REPLY_1.to_vec();
        let last = bad.len() - 1;
        bad[last] ^= 0xFF;
        let computed = crc::update(0, &bad[..last - 1]);
        let received = u16::from_le_bytes([bad[last - 1], bad[last]]);
        assert_ne!(computed, received);
        let mut out = [0; 4];
        assert_eq!(
            parse(&bad, &mut out),
            Some(Err(Defect::Crc {
                packet_id: 1,
                computed,
                received
            }))
        );
        let request = req(Instruction::Read).id(id(1)).addr(132).len(4);
        assert_eq!(
            Defect::Crc {
                packet_id: 1,
                computed,
                received
            }
            .error(request),
            ProtocolError::Crc {
                request,
                packet_id: 1,
                computed,
                received
            }
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
                (1, Err(skipped(sync_req(1), 2))),
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
                (3, Err(skipped(sync_req(3), 2))),
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
                    request: sync_req(1),
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
        assert_eq!(
            P2::sync_read(
                &mut link!(line, clock),
                0,
                u16::MAX,
                [id(1)].into_iter(),
                &mut scratch,
                |_, _| {}
            ),
            Err(ValidationError::PacketTooLarge {
                len: usize::from(u16::MAX),
                max: MAX_STATUS_DATA,
                request: req(Instruction::SyncRead).addr(0).len(u16::MAX),
            }
            .into())
        );
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
    fn replies_from_another_id_are_skipped_and_of_another_size_rejected() {
        let time = MockTime::new();
        let mut clock = time.ticker(1);

        let mut line = MockTransport::new(&time);
        line.reply(v::READ_REPLY_2);
        let mut out = [0; 4];
        let read_req = req(Instruction::Read).id(id(1)).addr(132).len(4);
        assert_eq!(
            P2::read(&mut link!(line, clock), id(1), 132, &mut out),
            Err(no_reply(read_req))
        );

        let mut line = MockTransport::new(&time);
        let mut stream = std::vec::Vec::from(v::READ_REPLY_2);
        stream.extend_from_slice(v::READ_REPLY_1);
        line.reply(&stream);
        let mut out = [0; 4];
        assert_eq!(
            P2::read(&mut link!(line, clock), id(1), 132, &mut out),
            Ok(())
        );

        let mut line = MockTransport::new(&time);
        line.reply(&packet(1, STATUS, &[0, 0xA6, 0]));
        let mut out = [0; 4];
        assert_eq!(
            P2::read(&mut link!(line, clock), id(1), 132, &mut out),
            Err(ProtocolError::WrongLength {
                request: read_req,
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
            Err(ProtocolError::LengthTooLong {
                request: req(Instruction::Read).id(id(1)).addr(132).len(1),
                length: 8,
                max: 6,
            }
            .into())
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
                report: alert,
                request: req(Instruction::Read).id(id(1)).addr(70).len(1),
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
                (1, Err(skipped(sync_req(1), 2))),
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
                (1, Err(no_reply(sync_req(1)))),
                (2, Ok(std::vec![0x1F, 0x08, 0, 0]))
            ]
        );
        assert_eq!(line.unread(), 0);
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
                },
                request: req(Instruction::Write).id(id(3)).addr(116).len(4),
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
                max: MAX_STATUS_DATA,
                request: req(Instruction::Read)
                    .id(id(1))
                    .addr(0)
                    .len(Request::len_of(MAX_STATUS_DATA + 1)),
            }
            .into())
        );
        let data = [0u8; MAX_LEN];
        assert!(matches!(
            P2::write(&mut link!(line, clock), id(1), 0, &data),
            Err(Error::Validation(ValidationError::PacketTooLarge {
                max: MAX_LEN,
                request: Request {
                    instruction: Some(Instruction::Write),
                    addr: Some(0),
                    ..
                },
                ..
            }))
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
                got: 2,
                request: req(Instruction::SyncWrite).id(id(1)).addr(116).len(4),
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
                (2, Err(no_reply(sync_req(2))))
            ]
        );
    }

    #[test]
    fn sync_read_fails_on_transport_error_and_small_scratch() {
        let time = MockTime::new();
        let mut clock = time.ticker(1);
        let mut line = MockTransport::new(&time);
        let mut scratch = [0; 2];
        assert_eq!(
            P2::sync_read(
                &mut link!(line, clock),
                132,
                4,
                [id(1)].into_iter(),
                &mut scratch,
                |_, _| {}
            ),
            Err(ValidationError::DataLength {
                expected: 4,
                got: 2,
                request: req(Instruction::SyncRead).addr(132).len(4),
            }
            .into())
        );
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
            Err(broken_pipe(req(Instruction::SyncRead).addr(132).len(4)))
        );
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
                Err(ProtocolError::Crc {
                    request: req(Instruction::Ping),
                    packet_id: 1,
                    computed: crc::update(0, &corrupted[..corrupted.len() - 2]),
                    received: u16::from_le_bytes([
                        corrupted[corrupted.len() - 2],
                        corrupted[corrupted.len() - 1]
                    ]),
                }
                .into()),
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
                    },
                    request: req(Instruction::Ping).id(id(3)),
                }
                .into()),
                Err(ProtocolError::WrongLength {
                    request: req(Instruction::Ping).id(id(4)),
                    expected: 3,
                    got: 1
                }
                .into()),
                Err(ProtocolError::InvalidReplyId {
                    request: req(Instruction::Ping),
                    got: 253
                }
                .into()),
                Err(ProtocolError::LengthTooShort {
                    request: req(Instruction::Ping),
                    protocol: ProtocolVersion::P2,
                    length: 2,
                    min: 3
                }
                .into()),
            ]
        );
    }

    #[test]
    fn scan_fails_on_the_transport() {
        let time = MockTime::new();
        let mut clock = time.ticker(10);
        let mut line = MockTransport::new(&time);
        line.fail_with(ErrorKind::BrokenPipe);
        assert_eq!(
            P2::scan(&mut link!(line, clock), |_| {}),
            Err(broken_pipe(req(Instruction::Ping)))
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
                (3, Err(skipped(sync_req(3), 2))),
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
            Err(broken_pipe(req(Instruction::SyncRead).addr(132).len(4)))
        );
    }

    /// What each ID got, in order.
    type Replies = Vec<(u8, Result<Vec<u8>, Error>)>;

    fn bulk_read_with(entries: &[(u8, u16, u16)], replies: &[&[u8]]) -> (Vec<u8>, Replies) {
        let time = MockTime::new();
        let mut clock = time.ticker(100);
        let mut line = MockTransport::new(&time);
        for reply in replies {
            line.reply(reply);
        }
        let mut got = Vec::new();
        let mut scratch = [0; 4];
        P2::bulk_read(
            &mut link!(line, clock),
            entries
                .iter()
                .map(|&(raw, addr, size)| (id(raw), addr, size)),
            &mut scratch,
            |id, r| got.push((id.get(), r.map(<[u8]>::to_vec))),
        )
        .unwrap();
        (line.sent().to_vec(), got)
    }

    #[test]
    fn bulk_read_matches_emanual_and_delivers_each_size() {
        let (sent, got) = bulk_read_with(
            &[(1, 144, 2), (2, 146, 1)],
            &[v::BULK_READ_REPLY_1, v::BULK_READ_REPLY_2],
        );
        assert_eq!(sent, v::BULK_READ);
        assert_eq!(
            got,
            [(1, Ok(std::vec![0x77, 0x00])), (2, Ok(std::vec![0x24]))]
        );
    }

    #[test]
    fn bulk_read_gives_a_silent_actuator_its_timeout() {
        let (_, got) = bulk_read_with(&[(1, 144, 2), (2, 146, 1)], &[v::BULK_READ_REPLY_2]);
        let bulk = |raw, addr, len| req(Instruction::BulkRead).id(id(raw)).addr(addr).len(len);
        assert_eq!(
            got,
            [
                (1, Err(skipped(bulk(1, 144, 2), 2))),
                (2, Ok(std::vec![0x24]))
            ]
        );
        let (_, got) = bulk_read_with(&[(1, 144, 1)], &[v::BULK_READ_REPLY_1]);
        assert_eq!(
            got,
            [(
                1,
                Err(ProtocolError::WrongLength {
                    request: bulk(1, 144, 1),
                    expected: 1,
                    got: 2
                }
                .into())
            )]
        );
    }

    #[test]
    fn bulk_read_errors_carry_each_entry_s_address_and_size() {
        let (_, got) = bulk_read_with(&[(1, 144, 2), (2, 146, 1)], &[]);
        let bulk = |raw, addr, len| req(Instruction::BulkRead).id(id(raw)).addr(addr).len(len);
        assert_eq!(
            got,
            [
                (1, Err(no_reply(bulk(1, 144, 2)))),
                (2, Err(no_reply(bulk(2, 146, 1))))
            ]
        );
    }

    #[test]
    fn bulk_read_refuses_bad_requests_before_sending() {
        let time = MockTime::new();
        let mut clock = time.ticker(1);
        let mut line = MockTransport::new(&time);
        let mut scratch = [0; 2];
        let mut run = |entries: &[(u8, u16, u16)], scratch: &mut [u8]| {
            P2::bulk_read(
                &mut link!(line, clock),
                entries
                    .iter()
                    .map(|&(raw, addr, size)| (id(raw), addr, size)),
                scratch,
                |_, _| {},
            )
        };
        assert_eq!(
            run(&[(1, 144, 2), (1, 146, 1)], &mut scratch),
            Err(ValidationError::DuplicateId { id: id(1) }.into())
        );
        assert_eq!(
            run(&[(1, 132, 4)], &mut scratch),
            Err(ValidationError::DataLength {
                expected: 4,
                got: 2,
                request: req(Instruction::BulkRead).id(id(1)).addr(132).len(4),
            }
            .into())
        );
        assert_eq!(
            run(&[(1, 0, u16::MAX)], &mut scratch),
            Err(ValidationError::PacketTooLarge {
                len: usize::from(u16::MAX),
                max: MAX_STATUS_DATA,
                request: req(Instruction::BulkRead).id(id(1)).addr(0).len(u16::MAX),
            }
            .into())
        );
        assert!(line.sent().is_empty());
    }

    #[test]
    fn bulk_write_matches_emanual_and_never_reads() {
        let time = MockTime::new();
        let mut clock = time.ticker(1);
        let mut line = MockTransport::new(&time);
        let a = 160u16.to_le_bytes();
        let b = [80u8];
        let entries = [(id(1), 32, &a[..]), (id(2), 31, &b[..])];
        P2::bulk_write(&mut link!(line, clock), entries.into_iter()).unwrap();
        assert_eq!((line.sent(), line.reads()), (v::BULK_WRITE, 0));

        let mut line = MockTransport::new(&time);
        let twice = [(id(1), 32, &a[..]), (id(1), 31, &b[..])];
        assert_eq!(
            P2::bulk_write(&mut link!(line, clock), twice.into_iter()),
            Err(ValidationError::DuplicateId { id: id(1) }.into())
        );
        let huge = std::vec![0; usize::from(u16::MAX) + 1];
        let too_long = [(id(1), 0, &huge[..])];
        assert_eq!(
            P2::bulk_write(&mut link!(line, clock), too_long.into_iter()),
            Err(ValidationError::PacketTooLarge {
                len: huge.len(),
                max: MAX_LEN,
                request: req(Instruction::BulkWrite).id(id(1)).addr(0),
            }
            .into())
        );
        assert!(line.sent().is_empty());
    }

    #[test]
    fn reboot_matches_emanual_and_waits_for_its_status() {
        let time = MockTime::new();
        let mut clock = time.ticker(10);
        let mut line = MockTransport::new(&time);
        line.reply(v::OK_REPLY);
        assert_eq!(P2::reboot(&mut link!(line, clock), id(1)), Ok(()));
        line.assert_sent(v::REBOOT);
        line.assert_done();

        let mut line = MockTransport::new(&time);
        assert_eq!(
            P2::reboot(&mut link!(line, clock), id(1)),
            Err(no_reply(req(Instruction::Reboot).id(id(1))))
        );
    }
}
