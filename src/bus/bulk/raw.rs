//! Raw Bulk Read and Bulk Write (`raw-registers` feature): any address and
//! any length on each actuator, in one instruction.
//!
//! What is sent bypasses the field checks, as
//! [`ActuatorRef::read_raw`](crate::ActuatorRef::read_raw) and
//! [`write_raw`](crate::ActuatorRef::write_raw) do; what comes back still
//! passes the fault check. The instruction checks stay: every target is
//! held, appears once, and on Protocol 1 has Bulk Read in its control
//! table.

use core::mem;

use crate::actuator::{Actuator, Describe};
use crate::error::{Error, SetupError, ValidationError};
use crate::protocol::{Clock, P2, Protocol, ProtocolVersion};
use crate::{Id, Transport};

use super::super::Bus;
use super::super::group::{FirstError, Stage};
use super::super::id_set::IdSet;
use super::super::observe::observe;
use super::super::slots::{Slots, find};
use super::super::wire::Wire;
use super::lost;

/// One target of a raw Bulk Read: `out.len()` bytes from `addr` on actuator
/// `id`.
#[derive(Debug)]
pub struct RawRead<'o> {
    /// The actuator to read.
    pub id: Id,
    /// The first address to read.
    pub addr: u16,
    /// Receives the bytes; its length is the number of bytes read.
    pub out: &'o mut [u8],
}

impl<'o> RawRead<'o> {
    /// Reads `out.len()` bytes from `addr` on `id`.
    pub fn new(id: Id, addr: u16, out: &'o mut [u8]) -> Self {
        Self { id, addr, out }
    }
}

/// One target of a raw Bulk Write: `data` from `addr` on actuator `id`.
#[derive(Debug, Clone, Copy)]
pub struct RawWrite<'d> {
    /// The actuator to write.
    pub id: Id,
    /// The first address to write.
    pub addr: u16,
    /// The bytes, sent as given.
    pub data: &'d [u8],
}

impl<'d> RawWrite<'d> {
    /// Writes `data` from `addr` on `id`.
    pub fn new(id: Id, addr: u16, data: &'d [u8]) -> Self {
        Self { id, addr, data }
    }
}

/// The target listed at `at` in the instruction: the targets in order,
/// except `last`, which goes at the end.
fn target_at(at: usize, count: usize, last: usize) -> usize {
    if at + 1 == count {
        last
    } else if at < last {
        at
    } else {
        at + 1
    }
}

/// Checks and stages every target of a raw Bulk Read on `P`, then sends it.
///
/// The replies come back one at a time through one buffer, which must hold
/// the longest. No buffer of the call's own is that long, so the longest
/// `out` is lent as that buffer and its target listed last: every earlier
/// reply is copied out of it before the last one lands in it.
pub(in crate::bus) fn bulk_read_raw<P, T, C, S>(
    wire: &mut Wire<'_, P, T, C>,
    slots: &mut S,
    stages: &mut [Stage],
    targets: &mut [RawRead<'_>],
) -> Result<(), Error>
where
    P: Protocol,
    T: Transport,
    C: Clock,
    S: Slots,
{
    let (kinds, states, _) = slots.split_mut();
    let kinds = &*kinds;
    let mut seen = IdSet::new();
    for (at, target) in targets.iter().enumerate() {
        let id = target.id;
        let index = find(kinds, id).ok_or(SetupError::NotOnBus { id })?;
        if !seen.insert(id) {
            return Err(ValidationError::DuplicateId { id }.into());
        }
        let kind = kinds.get(index).ok_or_else(|| lost(kinds))?;
        if P::VERSION == ProtocolVersion::P1 && !kind.data().bulk_read_p1 {
            return Err(ValidationError::UnsupportedInstruction {
                id,
                instruction: "Bulk Read",
            }
            .into());
        }
        let len = target.out.len();
        let len = u16::try_from(len).map_err(|_| ValidationError::PacketTooLarge {
            len,
            max: usize::from(u16::MAX),
        })?;
        let (Some(slot), Some(stage)) = (u8::try_from(index).ok(), stages.get_mut(at)) else {
            return Err(lost(kinds));
        };
        *stage = Stage::raw_read(slot, target.addr, len);
    }
    let count = targets.len();
    let Some(last) = targets
        .iter()
        .enumerate()
        .max_by_key(|(_, target)| target.out.len())
        .map(|(at, _)| at)
    else {
        return Ok(());
    };
    let stages = stages.get(..count).unwrap_or_default();
    let entries = (0..count).filter_map(|at| {
        let stage = stages.get(target_at(at, count, last))?;
        let kind = kinds.get(usize::from(stage.slot))?;
        Some((kind.id(), stage.addr, stage.raw_len()))
    });
    let scratch = targets
        .get_mut(last)
        .map(|target| mem::take(&mut target.out))
        .unwrap_or_default();
    let mut first = FirstError::default();
    let mut next = 0;
    let sent = wire.bulk_read(entries, scratch, |_, reply| {
        let at = target_at(next, count, last);
        next += 1;
        let result = reply.map(|bytes| {
            let out = targets
                .get_mut(at)
                .filter(|_| at != last)
                .and_then(|target| target.out.get_mut(..bytes.len()));
            if let Some(out) = out {
                out.copy_from_slice(bytes);
            }
        });
        let slot = stages.get(at).map(|stage| usize::from(stage.slot));
        if let Some(state) = slot.and_then(|slot| states.get_mut(slot)) {
            observe(state, &result);
        }
        first.note(at, &result);
    });
    if let Some(target) = targets.get_mut(last) {
        target.out = scratch;
    }
    sent?;
    first.into_result()
}

/// Checks every target of a raw Bulk Write, then sends it.
pub(in crate::bus) fn bulk_write_raw<T, C, S>(
    wire: &mut Wire<'_, P2, T, C>,
    slots: &S,
    targets: &[RawWrite<'_>],
) -> Result<(), Error>
where
    T: Transport,
    C: Clock,
    S: Slots,
{
    let kinds = slots.kinds();
    let mut seen = IdSet::new();
    for &RawWrite { id, .. } in targets {
        find(kinds, id).ok_or(SetupError::NotOnBus { id })?;
        if !seen.insert(id) {
            return Err(ValidationError::DuplicateId { id }.into());
        }
    }
    if targets.is_empty() {
        return Ok(());
    }
    wire.bulk_write(
        targets
            .iter()
            .map(|target| (target.id, target.addr, target.data)),
    )
}

/// Raw Bulk Read, on both protocols (`raw-registers` feature).
impl<P: Protocol, T: Transport, C: Clock, const N: usize> Bus<P, T, C, N> {
    /// Reads `out.len()` bytes from `addr` of every target with one Bulk
    /// Read: each actuator its own address and length.
    ///
    /// No field check: any address, any length, in every torque state, as
    /// [`ActuatorRef::read_raw`](crate::ActuatorRef::read_raw). Every reply
    /// passes the fault check; a reply reporting a hardware fault marks its
    /// actuator [`Faulted`](crate::TorqueState::Faulted). Not retried. An
    /// empty call sends nothing.
    ///
    /// The bytes go straight into each `out`, so the call is not strict
    /// like [`bulk_read`](Bus::bulk_read): every `out` holds this call's
    /// bytes only on `Ok`.
    ///
    /// The instruction lists the targets in order, except the one with the
    /// longest `out` (the last of them on a tie), which goes last: its
    /// buffer receives every reply in turn, so no buffer of the bus's own
    /// has to be that long.
    ///
    /// On Protocol 1, Bulk Read exists on MX, MX(2.0) and X series firmware
    /// except XL-320 (e-Manual Protocol 1.0): each target's control table
    /// must list it (MX-64 does; AX and RX do not).
    #[cfg_attr(
        all(feature = "mock", feature = "x-series"),
        doc = r#"
Present Velocity and Present Position of ID 1 (8 bytes from 128) and
Present Temperature of ID 2, two XM430-W350, in one instruction. ID 1's
buffer is the longest, so ID 1 is listed last and answers last:

```
use dxl_rs_v2::bulk::RawRead;
use dxl_rs_v2::{Bus, Config, Error, Id, MockTime, MockTransport, ModelNumber, P2};

# let time = MockTime::new();
# let mut line = MockTransport::new(&time);
# for (n, at) in [(1u8, 1), (2, 5)] {
#     let pong = [0xFF, 0xFF, 0xFD, 0x00, n, 0x07, 0x00, 0x55, 0x00, 0xFC, 0x03, 0x34];
#     let min = [0xFF, 0xFF, 0xFD, 0x00, n, 0x08, 0x00, 0x55, 0x00, 0x00, 0x00, 0x00, 0x00];
#     let max = [0xFF, 0xFF, 0xFD, 0x00, n, 0x08, 0x00, 0x55, 0x00, 0xFF, 0x0F, 0x00, 0x00];
#     let mode = [0xFF, 0xFF, 0xFD, 0x00, n, 0x05, 0x00, 0x55, 0x00, 0x03];
#     for (i, body) in [&pong[..], &min, &max, &mode].into_iter().enumerate() {
#         line.reply_to(at + i, &with_crc(body));
#     }
# }
# let temp = [0xFF, 0xFF, 0xFD, 0x00, 0x02, 0x05, 0x00, 0x55, 0x00, 0x24];
# let both = [0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x0C, 0x00, 0x55, 0x00,
#     0x0A, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00];
# line.reply_to(9, &with_crc(&temp)).reply_to(9, &with_crc(&both));
# fn with_crc(body: &[u8]) -> Vec<u8> {
#     let mut crc: u16 = 0;
#     for &byte in body {
#         crc ^= u16::from(byte) << 8;
#         for _ in 0..8 {
#             crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x8005 } else { crc << 1 };
#         }
#     }
#     let mut packet = body.to_vec();
#     packet.extend_from_slice(&crc.to_le_bytes());
#     packet
# }
let mut bus = Bus::<P2, _, _>::new(line, time.ticker(10), Config::new());
let (a, b) = (Id::new(1)?, Id::new(2)?);
bus.declare([(a, ModelNumber(1020)), (b, ModelNumber(1020))])?;

let mut motion = [0; 8];
let mut celsius = [0; 1];
bus.bulk_read_raw(&mut [
    RawRead::new(a, 128, &mut motion),
    RawRead::new(b, 146, &mut celsius),
])?;
assert_eq!(i32::from_le_bytes(motion[..4].try_into().unwrap()), 10);
assert_eq!(i32::from_le_bytes(motion[4..].try_into().unwrap()), 2048);
assert_eq!(celsius, [36]);
# Ok::<(), Error>(())
```
"#
    )]
    ///
    /// # Errors
    ///
    /// Refused with nothing sent:
    /// - [`SetupError::NotOnBus`], [`ValidationError::DuplicateId`].
    /// - [`ValidationError::UnsupportedInstruction`]: Protocol 1, and a
    ///   target's table does not list Bulk Read.
    /// - [`ValidationError::PacketTooLarge`]: an `out` longer than one
    ///   status packet carries.
    /// - [`ValidationError::AddressOutOfRange`]: on Protocol 1, an address
    ///   above 255.
    ///
    /// On the wire: the first reply reporting a hardware fault in target
    /// order, else the first error in target order; a failing transport
    /// stops the call at once.
    pub fn bulk_read_raw(&mut self, targets: &mut [RawRead<'_>]) -> Result<(), Error> {
        let mut stages = [Stage::EMPTY; N];
        let (mut wire, mut slots) = self.split();
        bulk_read_raw(&mut wire, &mut slots, &mut stages, targets)
    }
}

/// Raw Bulk Write, Protocol 2 only (`raw-registers` feature).
impl<T: Transport, C: Clock, const N: usize> Bus<P2, T, C, N> {
    /// Writes `data` from `addr` of every target with one Bulk Write: each
    /// actuator its own address and bytes. Protocol 2 only.
    ///
    /// No torque guard, no limit check, no operating-mode check, no field
    /// check: the bytes are sent as given, as
    /// [`ActuatorRef::write_raw`](crate::ActuatorRef::write_raw) sends
    /// them, and never update the bus's view of the actuators. A Bulk Write
    /// gets no reply, so a fault shows on the next read. An empty call
    /// sends nothing.
    ///
    /// Protocol 1 has no Bulk Write, so the method does not exist there:
    ///
    /// ```compile_fail,E0599
    /// use dxl_rs_v2::bulk::RawWrite;
    /// use dxl_rs_v2::{Bus, Clock, Id, P1, Transport};
    ///
    /// fn blink<T: Transport, C: Clock>(bus: &mut Bus<P1, T, C>, id: Id) {
    ///     let _ = bus.bulk_write_raw(&[RawWrite::new(id, 25, &[1])]);
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Refused with nothing sent: [`SetupError::NotOnBus`],
    /// [`ValidationError::DuplicateId`], [`ValidationError::PacketTooLarge`]
    /// (data longer than one packet carries). Otherwise a transport error.
    pub fn bulk_write_raw(&mut self, targets: &[RawWrite<'_>]) -> Result<(), Error> {
        let (mut wire, slots) = self.split();
        bulk_write_raw(&mut wire, &slots, targets)
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use std::vec::Vec;

    use core::iter;

    use super::*;

    /// The instruction order of `count` targets whose longest is `last`.
    fn order(count: usize, last: usize) -> impl Iterator<Item = usize> {
        (0..count)
            .filter(move |&at| at != last)
            .chain(iter::once(last))
    }

    #[test]
    fn instruction_order_puts_the_longest_last() {
        for count in 1..6 {
            for last in 0..count {
                let listed: Vec<_> = (0..count).map(|at| target_at(at, count, last)).collect();
                assert_eq!(listed, order(count, last).collect::<Vec<_>>());
            }
        }
    }

    #[test]
    fn raw_stage_keeps_its_length() {
        let stage = Stage::raw_read(3, 132, 0x1234);
        assert_eq!((stage.slot, stage.addr, stage.raw_len()), (3, 132, 0x1234));
    }

    #[cfg(feature = "x-series")]
    mod p2 {
        use super::*;
        use crate::bus::testing::*;
        use crate::error::{Error, StatusError, StatusReport, TransportError};
        use crate::protocol::P2;
        use crate::protocol::vectors::p2 as v;
        use crate::transport::MockTime;

        /// XM430-W350s at `ids`, limits 0..=4095, torque `state`.
        fn xm430s<'t>(
            time: &'t MockTime,
            ids: &[u8],
            state: crate::TorqueState,
        ) -> TestBus<'t, P2> {
            let mut bus = test_bus(time);
            for &n in ids {
                hold(&mut bus, XM430, n, (0, 4095), BOUNDED, state);
            }
            bus
        }

        #[test]
        fn bulk_read_raw_moves_only_a_longest_target_that_is_not_last() {
            let time = MockTime::new();
            let mut bus = xm430s(&time, &[1, 2], OFF);
            let mark = mark(&bus);
            line(&mut bus)
                .reply(&p2_status(1, 0, &[36]))
                .reply(&p2_status(2, 0, &[0x77, 0x00]));
            let (mut celsius, mut volts) = ([0; 1], [0; 2]);
            bus.bulk_read_raw(&mut [
                RawRead::new(id(1), 146, &mut celsius),
                RawRead::new(id(2), 144, &mut volts),
            ])
            .unwrap();
            assert_eq!((celsius, volts), ([36], [0x77, 0x00]));
            assert_eq!(
                sent_since(&bus, mark),
                p2_bulk_read(&[(1, 146, 1), (2, 144, 2)])
            );
            assert_done(&bus);

            let mut bus = xm430s(&time, &[1, 2], OFF);
            let mark = crate::bus::testing::mark(&bus);
            line(&mut bus)
                .reply(v::BULK_READ_REPLY_2)
                .reply(v::BULK_READ_REPLY_1);
            let (mut volts, mut celsius) = ([0; 2], [0; 1]);
            bus.bulk_read_raw(&mut [
                RawRead::new(id(1), 144, &mut volts),
                RawRead::new(id(2), 146, &mut celsius),
            ])
            .unwrap();
            assert_eq!((volts, celsius), ([0x77, 0x00], [0x24]));
            assert_eq!(
                sent_since(&bus, mark),
                p2_bulk_read(&[(2, 146, 1), (1, 144, 2)])
            );
            assert_done(&bus);
        }

        #[test]
        fn bulk_read_raw_lends_the_longest_buffer_and_fills_every_one() {
            let time = MockTime::new();
            let mut bus = xm430s(&time, &[1, 2, 3], ON);
            let mark = mark(&bus);
            let block = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
            line(&mut bus)
                .reply(&p2_status(2, 0, &[36]))
                .reply(&p2_status(3, 0, &[0x77, 0x00]))
                .reply(&p2_status(1, 0, &block));
            let (mut a, mut b, mut c) = ([0; 10], [0; 1], [0; 2]);
            bus.bulk_read_raw(&mut [
                RawRead::new(id(1), 124, &mut a),
                RawRead::new(id(2), 146, &mut b),
                RawRead::new(id(3), 144, &mut c),
            ])
            .unwrap();
            assert_eq!((a, b, c), (block, [36], [0x77, 0x00]));
            assert_eq!(
                sent_since(&bus, mark),
                p2_bulk_read(&[(2, 146, 1), (3, 144, 2), (1, 124, 10)])
            );
            assert_eq!(bus.torque_states(), [ON; 3]);
        }

        #[test]
        fn bulk_read_raw_reports_the_fault_first_and_marks_it() {
            let time = MockTime::new();
            let mut bus = xm430s(&time, &[1, 2, 3], ON);
            line(&mut bus)
                .reply(&p2_status(1, 0, &[36]))
                .reply(&p2_status(3, 0x80, &[37, 0]));
            let (mut a, mut b, mut c) = ([0; 1], [0; 1], [0; 2]);
            let mut targets = [
                RawRead::new(id(1), 146, &mut a),
                RawRead::new(id(2), 146, &mut b),
                RawRead::new(id(3), 144, &mut c),
            ];
            assert_eq!(
                bus.bulk_read_raw(&mut targets),
                Err(StatusError {
                    id: id(3),
                    report: StatusReport::P2 {
                        alert: true,
                        code: None
                    }
                }
                .into())
            );
            assert_eq!(targets[2].out.len(), 2);
            assert_eq!(bus.torque_states(), [ON, ON, FAULTED]);

            let mut bus = xm430s(&time, &[1, 2], ON);
            line(&mut bus).reply(&p2_status(2, 0, &[37]));
            let (mut a, mut b) = ([0; 1], [0; 1]);
            assert_eq!(
                bus.bulk_read_raw(&mut [
                    RawRead::new(id(1), 146, &mut a),
                    RawRead::new(id(2), 146, &mut b),
                ]),
                Err(TransportError::Timeout { id: Some(id(1)) }.into())
            );
            assert_eq!(b, [37]);
        }

        #[test]
        fn bulk_read_raw_refuses_before_sending() {
            let time = MockTime::new();
            let mut bus = xm430s(&time, &[1, 2], OFF);
            let mark = mark(&bus);
            let (mut a, mut b) = ([0; 1], [0; 1]);
            assert_eq!(
                bus.bulk_read_raw(&mut [RawRead::new(id(9), 146, &mut a)]),
                Err(SetupError::NotOnBus { id: id(9) }.into())
            );
            assert_eq!(
                bus.bulk_read_raw(&mut [
                    RawRead::new(id(1), 146, &mut a),
                    RawRead::new(id(1), 144, &mut b),
                ]),
                Err(ValidationError::DuplicateId { id: id(1) }.into())
            );
            let mut huge = std::vec![0; usize::from(u16::MAX) + 1];
            assert_eq!(
                bus.bulk_read_raw(&mut [RawRead::new(id(1), 0, &mut huge)]),
                Err(ValidationError::PacketTooLarge {
                    len: usize::from(u16::MAX) + 1,
                    max: usize::from(u16::MAX)
                }
                .into())
            );
            let mut big = std::vec![0; usize::from(u16::MAX)];
            let mut targets = [
                RawRead::new(id(1), 0, &mut big),
                RawRead::new(id(2), 146, &mut a),
            ];
            assert!(matches!(
                bus.bulk_read_raw(&mut targets),
                Err(Error::Validation(ValidationError::PacketTooLarge { .. }))
            ));
            assert_eq!(targets[0].out.len(), usize::from(u16::MAX));
            assert_eq!(bus.bulk_read_raw(&mut []), Ok(()));
            assert_quiet(&bus, mark);
        }

        #[test]
        fn bulk_write_raw_is_the_e_manual_example_and_skips_every_guard() {
            let time = MockTime::new();
            let mut bus = xm430s(&time, &[1, 2], OFF);
            let mark = mark(&bus);
            let reads = line(&mut bus).reads();
            bus.bulk_write_raw(&[
                RawWrite::new(id(1), 32, &160u16.to_le_bytes()),
                RawWrite::new(id(2), 31, &[80]),
            ])
            .unwrap();
            assert_eq!(sent_since(&bus, mark), v::BULK_WRITE);
            assert_eq!(line(&mut bus).reads(), reads);

            let mark = crate::bus::testing::mark(&bus);
            bus.bulk_write_raw(&[RawWrite::new(id(2), 116, &9000i32.to_le_bytes())])
                .unwrap();
            assert_eq!(
                sent_since(&bus, mark),
                p2_bulk_write(&[(2, 116, &9000i32.to_le_bytes())])
            );
            assert_eq!(bus.torque_states(), [OFF; 2]);
        }

        #[test]
        fn bulk_write_raw_refuses_before_sending() {
            let time = MockTime::new();
            let mut bus = xm430s(&time, &[1, 2], ON);
            let mark = mark(&bus);
            assert_eq!(
                bus.bulk_write_raw(&[RawWrite::new(id(7), 65, &[1])]),
                Err(SetupError::NotOnBus { id: id(7) }.into())
            );
            assert_eq!(
                bus.bulk_write_raw(&[
                    RawWrite::new(id(2), 65, &[1]),
                    RawWrite::new(id(2), 64, &[0])
                ]),
                Err(ValidationError::DuplicateId { id: id(2) }.into())
            );
            let huge = std::vec![0; usize::from(u16::MAX) + 1];
            assert!(matches!(
                bus.bulk_write_raw(&[RawWrite::new(id(1), 0, &huge)]),
                Err(Error::Validation(ValidationError::PacketTooLarge { .. }))
            ));
            assert_eq!(bus.bulk_write_raw(&[]), Ok(()));
            assert_quiet(&bus, mark);
        }

        #[cfg(feature = "std")]
        #[test]
        fn dyn_bus_has_the_raw_bulk_calls_of_bus() {
            use crate::Config;
            use crate::actuator::ModelNumber;
            use crate::bus::DynBus;
            use crate::transport::MockTransport;

            let time = MockTime::new();
            let mut line = MockTransport::new(&time);
            let next = xm430_p2(&mut line, 1, 1, 0, 4095, 3);
            let next = xm430_p2(&mut line, next, 2, 0, 4095, 3);
            line.reply_to(next, &p2_status(1, 0, &[0x77, 0x00]))
                .reply_to(next, &p2_status(2, 0, &[1, 2, 3, 4]));
            let mut bus = DynBus::<P2, _, _>::new(line, Ticker(&time), Config::new());
            bus.declare([(id(1), ModelNumber(XM430)), (id(2), ModelNumber(XM430))])
                .unwrap();
            let (mut volts, mut ticks) = ([0; 2], [0; 4]);
            bus.bulk_read_raw(&mut [
                RawRead::new(id(1), 144, &mut volts),
                RawRead::new(id(2), 132, &mut ticks),
            ])
            .unwrap();
            assert_eq!((volts, ticks), ([0x77, 0x00], [1, 2, 3, 4]));
            bus.bulk_write_raw(&[RawWrite::new(id(2), 65, &[1])])
                .unwrap();
            assert_eq!(
                bus.bulk_write_raw(&[RawWrite::new(id(3), 65, &[1])]),
                Err(SetupError::NotOnBus { id: id(3) }.into())
            );
            let (line, _) = bus.into_parts();
            let mut expected = Vec::new();
            for n in [1, 2] {
                expected.extend(p2_instruction(n, PING, &[]));
                expected.extend(xm430_p2_reads_sent_in(n, 3));
            }
            expected.extend(concat(&[
                p2_bulk_read(&[(1, 144, 2), (2, 132, 4)]),
                p2_bulk_write(&[(2, 65, &[1])]),
            ]));
            assert_eq!(line.sent(), expected);
        }
    }

    #[cfg(all(feature = "mx", feature = "ax"))]
    mod p1 {
        use super::*;
        use crate::bus::testing::*;
        use crate::protocol::P1;
        use crate::protocol::vectors::p1 as v;
        use crate::transport::MockTime;

        #[test]
        fn bulk_read_raw_is_the_e_manual_example_on_mx() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            hold(&mut bus, MX64_P1, 1, (0, 4095), BOUNDED, OFF);
            hold(&mut bus, MX64_P1, 2, (0, 4095), BOUNDED, OFF);
            let mark = mark(&bus);
            line(&mut bus)
                .reply(v::BULK_READ_REPLY_1)
                .reply(v::BULK_READ_REPLY_2);
            let (mut goal, mut present) = ([0; 2], [0; 2]);
            bus.bulk_read_raw(&mut [
                RawRead::new(id(1), 30, &mut goal),
                RawRead::new(id(2), 36, &mut present),
            ])
            .unwrap();
            assert_eq!((goal, present), ([0x00, 0x80], [0x00, 0x80]));
            assert_eq!(sent_since(&bus, mark), v::BULK_READ);
            assert_done(&bus);
        }

        #[test]
        fn bulk_read_raw_refuses_ax_and_long_addresses_before_sending() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            hold(&mut bus, MX64_P1, 1, (0, 4095), BOUNDED, OFF);
            hold(&mut bus, AX12, 2, (0, 1023), BOUNDED, OFF);
            let mark = mark(&bus);
            let (mut a, mut b) = ([0; 2], [0; 2]);
            assert_eq!(
                bus.bulk_read_raw(&mut [
                    RawRead::new(id(1), 36, &mut a),
                    RawRead::new(id(2), 36, &mut b),
                ]),
                Err(ValidationError::UnsupportedInstruction {
                    id: id(2),
                    instruction: "Bulk Read"
                }
                .into())
            );
            assert_eq!(
                bus.bulk_read_raw(&mut [RawRead::new(id(1), 256, &mut a)]),
                Err(ValidationError::AddressOutOfRange { addr: 256 }.into())
            );
            assert_quiet(&bus, mark);
        }
    }
}
