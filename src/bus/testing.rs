//! Packet builders and scripted actuators for the bus tests (research R-13).
//!
//! The protocol's own builders are private to `src/protocol/**`, so the bus
//! builds both directions here: status packets to script replies, and
//! instruction packets to check `sent()` byte for byte.

#![allow(dead_code, reason = "single-family test builds use only some fixtures")]

extern crate std;

use std::vec::Vec;

use crate::protocol::{Clock, Protocol};
use crate::transport::{MockTime, MockTransport};
use crate::{Bus, Config, Id};

/// A test clock on a shared [`MockTime`]: every reading moves time 10 µs
/// forward, like `MockTime::ticker(10)`, but with a type that can be named.
pub(crate) struct Ticker<'t>(pub(crate) &'t MockTime);

impl Clock for Ticker<'_> {
    fn now_us(&mut self) -> u64 {
        let now = self.0.now() + 10;
        self.0.set(now);
        now
    }
}

/// A bus on a scripted line.
pub(crate) type TestBus<'t, P, const N: usize = 32> = Bus<P, MockTransport<'t>, Ticker<'t>, N>;

/// A new bus with the default config on a silent line.
pub(crate) fn test_bus<P: Protocol, const N: usize>(time: &MockTime) -> TestBus<'_, P, N> {
    Bus::new(MockTransport::new(time), Ticker(time), Config::new())
}

/// How much a line had sent at one point.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Mark {
    sent: usize,
    flushes: usize,
}

/// The scripted line of `bus`, to add replies.
///
/// Free functions rather than methods on the bus type: an inherent impl on
/// `Bus<_, MockTransport, _>` trips the transport module's dead-code
/// expectation in non-`mock` test builds.
pub(crate) fn line<'a, 't, P: Protocol, const N: usize>(
    bus: &'a mut TestBus<'t, P, N>,
) -> &'a mut MockTransport<'t> {
    &mut bus.transport
}

/// What `bus` sent so far.
pub(crate) fn sent<'a, P: Protocol, const N: usize>(bus: &'a TestBus<'_, P, N>) -> &'a [u8] {
    bus.transport.sent()
}

/// How much `bus` has sent so far.
pub(crate) fn mark<P: Protocol, const N: usize>(bus: &TestBus<'_, P, N>) -> Mark {
    Mark {
        sent: bus.transport.sent().len(),
        flushes: bus.transport.flushes(),
    }
}

/// What `bus` sent since `mark`.
pub(crate) fn sent_since<'a, P: Protocol, const N: usize>(
    bus: &'a TestBus<'_, P, N>,
    mark: Mark,
) -> &'a [u8] {
    &bus.transport.sent()[mark.sent..]
}

/// Asserts that `bus` sent and flushed nothing since `mark`.
#[track_caller]
pub(crate) fn assert_quiet<P: Protocol, const N: usize>(bus: &TestBus<'_, P, N>, mark: Mark) {
    assert_eq!(sent_since(bus, mark), &[] as &[u8], "bytes were sent");
    assert_eq!(
        bus.transport.flushes(),
        mark.flushes,
        "the line was flushed"
    );
}

/// Asserts that `bus` read every scripted reply.
#[track_caller]
pub(crate) fn assert_done<P: Protocol, const N: usize>(bus: &TestBus<'_, P, N>) {
    bus.transport.assert_done();
}

/// Instruction codes of both protocols.
pub(crate) const PING: u8 = 0x01;
pub(crate) const READ: u8 = 0x02;
pub(crate) const WRITE: u8 = 0x03;

/// `Id` from a raw byte that is known to be valid.
pub(crate) fn id(n: u8) -> Id {
    Id::new(n).unwrap()
}

/// CRC-16 of Protocol 2: polynomial 0x8005, initial value 0, no reflection.
fn crc16(bytes: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &byte in bytes {
        crc ^= u16::from(byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x8005
            } else {
                crc << 1
            };
        }
    }
    crc
}

/// A Protocol 2 packet for `id` around `body` (instruction onwards): byte
/// stuffing, little-endian length and CRC.
fn p2_packet(id: u8, body: &[u8]) -> Vec<u8> {
    let mut stuffed = Vec::new();
    for &byte in body {
        stuffed.push(byte);
        if stuffed.ends_with(&[0xFF, 0xFF, 0xFD]) {
            stuffed.push(0xFD);
        }
    }
    let len = u16::try_from(stuffed.len() + 2).unwrap().to_le_bytes();
    let mut packet = std::vec![0xFF, 0xFF, 0xFD, 0x00, id, len[0], len[1]];
    packet.extend_from_slice(&stuffed);
    let crc = crc16(&packet).to_le_bytes();
    packet.extend_from_slice(&crc);
    packet
}

/// A Protocol 2 status packet (instruction `0x55`).
pub(crate) fn p2_status(id: u8, error: u8, params: &[u8]) -> Vec<u8> {
    let mut body = std::vec![0x55, error];
    body.extend_from_slice(params);
    p2_packet(id, &body)
}

/// A Protocol 2 instruction packet.
pub(crate) fn p2_instruction(id: u8, inst: u8, params: &[u8]) -> Vec<u8> {
    let mut body = std::vec![inst];
    body.extend_from_slice(params);
    p2_packet(id, &body)
}

/// A Protocol 2 Ping reply: model number and firmware version.
pub(crate) fn p2_pong(id: u8, model: u16, fw: u8) -> Vec<u8> {
    let [lo, hi] = model.to_le_bytes();
    p2_status(id, 0, &[lo, hi, fw])
}

/// A Protocol 2 Read instruction for `len` bytes at `addr`.
pub(crate) fn p2_read(id: u8, addr: u16, len: u16) -> Vec<u8> {
    let [a0, a1] = addr.to_le_bytes();
    let [l0, l1] = len.to_le_bytes();
    p2_instruction(id, READ, &[a0, a1, l0, l1])
}

/// A Protocol 2 Write instruction of `data` at `addr`.
pub(crate) fn p2_write(id: u8, addr: u16, data: &[u8]) -> Vec<u8> {
    let mut params = addr.to_le_bytes().to_vec();
    params.extend_from_slice(data);
    p2_instruction(id, WRITE, &params)
}

/// A Protocol 1 packet: `FF FF ID LEN <head> params CHECKSUM`.
fn p1_packet(id: u8, head: u8, params: &[u8]) -> Vec<u8> {
    let len = u8::try_from(params.len() + 2).unwrap();
    let mut packet = std::vec![0xFF, 0xFF, id, len, head];
    packet.extend_from_slice(params);
    let sum = packet[2..]
        .iter()
        .fold(0u8, |sum, &byte| sum.wrapping_add(byte));
    packet.push(!sum);
    packet
}

/// A Protocol 1 status packet.
pub(crate) fn p1_status(id: u8, error: u8, params: &[u8]) -> Vec<u8> {
    p1_packet(id, error, params)
}

/// A Protocol 1 instruction packet.
pub(crate) fn p1_instruction(id: u8, inst: u8, params: &[u8]) -> Vec<u8> {
    p1_packet(id, inst, params)
}

/// A Protocol 1 Read instruction for `len` bytes at `addr`.
pub(crate) fn p1_read(id: u8, addr: u8, len: u8) -> Vec<u8> {
    p1_instruction(id, READ, &[addr, len])
}

/// A Protocol 1 Write instruction of `data` at `addr`.
pub(crate) fn p1_write(id: u8, addr: u8, data: &[u8]) -> Vec<u8> {
    let mut params = std::vec![addr];
    params.extend_from_slice(data);
    p1_instruction(id, WRITE, &params)
}

/// The concatenation of `packets`, for `assert_sent`.
pub(crate) fn concat(packets: &[Vec<u8>]) -> Vec<u8> {
    packets.concat()
}

/// XM430-W350 model number.
pub(crate) const XM430: u16 = 1020;
/// XL-320 model number.
pub(crate) const XL320: u16 = 350;
/// AX-12A model number.
pub(crate) const AX12: u16 = 12;
/// RX-28 model number.
pub(crate) const RX28: u16 = 28;
/// XM430-W350 firmware used by the fixtures.
pub(crate) const XM430_FW: u8 = 52;

/// Scripts an XM430-W350 at `id` on Protocol 2 answering Setup from flush
/// `at`: ping, Min Position Limit (52), Max Position Limit (48), Operating
/// Mode (11). Returns the next free flush number.
pub(crate) fn xm430_p2(
    line: &mut MockTransport<'_>,
    at: usize,
    id: u8,
    min: i32,
    max: i32,
    mode: u8,
) -> usize {
    line.reply_to(at, &p2_pong(id, XM430, XM430_FW));
    xm430_p2_reads(line, at + 1, id, min, max, mode)
}

/// Scripts the Setup reads of an XM430-W350 at `id` after its ping, from
/// flush `at`: Min and Max Position Limit, Operating Mode. Returns the next
/// free flush number.
pub(crate) fn xm430_p2_reads(
    line: &mut MockTransport<'_>,
    at: usize,
    id: u8,
    min: i32,
    max: i32,
    mode: u8,
) -> usize {
    line.reply_to(at, &p2_status(id, 0, &min.to_le_bytes()))
        .reply_to(at + 1, &p2_status(id, 0, &max.to_le_bytes()))
        .reply_to(at + 2, &p2_status(id, 0, &[mode]));
    at + 3
}

/// What the Setup reads of [`xm430_p2_reads`] send.
pub(crate) fn xm430_p2_reads_sent(id: u8) -> Vec<u8> {
    concat(&[p2_read(id, 52, 4), p2_read(id, 48, 4), p2_read(id, 11, 1)])
}

/// What Setup of [`xm430_p2`] sends.
pub(crate) fn xm430_p2_sent(id: u8) -> Vec<u8> {
    concat(&[
        p2_instruction(id, PING, &[]),
        p2_read(id, 52, 4),
        p2_read(id, 48, 4),
        p2_read(id, 11, 1),
    ])
}

/// Scripts an XL-320 at `id` on Protocol 2 answering Setup from flush `at`:
/// ping, CW Angle Limit (6), CCW Angle Limit (8), Control Mode (11).
/// Returns the next free flush number.
pub(crate) fn xl320_p2(
    line: &mut MockTransport<'_>,
    at: usize,
    id: u8,
    cw: u16,
    ccw: u16,
    control_mode: u8,
) -> usize {
    line.reply_to(at, &p2_pong(id, XL320, 37))
        .reply_to(at + 1, &p2_status(id, 0, &cw.to_le_bytes()))
        .reply_to(at + 2, &p2_status(id, 0, &ccw.to_le_bytes()))
        .reply_to(at + 3, &p2_status(id, 0, &[control_mode]));
    at + 4
}

/// What Setup of [`xl320_p2`] sends.
pub(crate) fn xl320_p2_sent(id: u8) -> Vec<u8> {
    concat(&[
        p2_instruction(id, PING, &[]),
        p2_read(id, 6, 2),
        p2_read(id, 8, 2),
        p2_read(id, 11, 1),
    ])
}

/// Scripts a Protocol 1 actuator of `model` at `id` answering Setup from
/// flush `at`: ping, Model Number (0), CW Angle Limit (6), CCW Angle Limit
/// (8). No mode read. Returns the next free flush number.
pub(crate) fn angle_p1(
    line: &mut MockTransport<'_>,
    at: usize,
    id: u8,
    model: u16,
    cw: u16,
    ccw: u16,
) -> usize {
    line.reply_to(at, &p1_status(id, 0, &[]));
    angle_p1_reads(line, at + 1, id, model, cw, ccw)
}

/// Scripts the Setup reads of a Protocol 1 actuator of `model` at `id`
/// after its ping, from flush `at`: Model Number, CW and CCW Angle Limit.
/// Returns the next free flush number.
pub(crate) fn angle_p1_reads(
    line: &mut MockTransport<'_>,
    at: usize,
    id: u8,
    model: u16,
    cw: u16,
    ccw: u16,
) -> usize {
    line.reply_to(at, &p1_status(id, 0, &model.to_le_bytes()))
        .reply_to(at + 1, &p1_status(id, 0, &cw.to_le_bytes()))
        .reply_to(at + 2, &p1_status(id, 0, &ccw.to_le_bytes()));
    at + 3
}

/// Scripts an AX-12A at `id` on Protocol 1, as [`angle_p1`].
pub(crate) fn ax12_p1(line: &mut MockTransport<'_>, at: usize, id: u8, cw: u16, ccw: u16) -> usize {
    angle_p1(line, at, id, AX12, cw, ccw)
}

/// What Setup of [`ax12_p1`] sends.
pub(crate) fn ax12_p1_sent(id: u8) -> Vec<u8> {
    concat(&[
        p1_instruction(id, PING, &[]),
        p1_read(id, 0, 2),
        p1_read(id, 6, 2),
        p1_read(id, 8, 2),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn p2_status_matches_the_e_manual_ping_reply() {
        assert_eq!(
            p2_status(1, 0, &[0x06, 0x04, 0x26]),
            [
                0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x07, 0x00, 0x55, 0x00, 0x06, 0x04, 0x26, 0x65, 0x5D
            ]
        );
    }

    #[test]
    fn p2_instruction_matches_the_e_manual_ping() {
        assert_eq!(
            p2_instruction(1, PING, &[]),
            [0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x03, 0x00, 0x01, 0x19, 0x4E]
        );
    }

    #[test]
    fn p2_broadcast_ping_matches_the_e_manual() {
        assert_eq!(
            p2_instruction(0xFE, PING, &[]),
            [0xFF, 0xFF, 0xFD, 0x00, 0xFE, 0x03, 0x00, 0x01, 0x31, 0x42]
        );
    }

    #[test]
    fn p2_bytes_are_stuffed() {
        let packet = p2_status(1, 0, &[0xFF, 0xFF, 0xFD, 0x01]);
        assert_eq!(&packet[5..7], &[0x09, 0x00]);
        assert_eq!(&packet[9..14], &[0xFF, 0xFF, 0xFD, 0xFD, 0x01]);
    }

    #[test]
    fn p1_status_matches_the_e_manual() {
        assert_eq!(p1_status(1, 0, &[]), [0xFF, 0xFF, 0x01, 0x02, 0x00, 0xFC]);
        assert_eq!(
            p1_status(1, 0, &[0x20]),
            [0xFF, 0xFF, 0x01, 0x03, 0x00, 0x20, 0xDB]
        );
    }

    #[test]
    fn p1_instruction_matches_the_e_manual_ping() {
        assert_eq!(
            p1_instruction(1, PING, &[]),
            [0xFF, 0xFF, 0x01, 0x02, 0x01, 0xFB]
        );
    }
}
