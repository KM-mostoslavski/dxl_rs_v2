//! Multi-actuator reads with one result per target (`partial-reads`;
//! research R-8 step 6, SRS FR-4.7 as decided by the architect on
//! 2026-10-28).

use crate::actuator::Describe;
use crate::error::Error;
use crate::protocol::{Clock, Protocol};
use crate::units::Position;
use crate::{Id, Transport};

use super::Bus;
use super::field::sealed::Value;
use super::field::{PresentPosition, Raw, ReadField};
use super::group::{self, OnFailedGroup, Stage};
use super::slots::Slots;
use super::wire::{AnyProtocol, Wire};

/// Reads field `F` of every target into `out[..targets]`, one result per
/// target: one `read_many` per register layout. A layout whose instruction
/// fails gives that error to each of its targets still waiting for a reply.
pub(super) fn read_partial<P, T, C, S, F, I>(
    wire: &mut Wire<'_, P, T, C>,
    slots: &mut S,
    stages: &mut [Stage],
    ids: I,
    out: &mut [Result<F::Value, Error>],
) -> Result<(), Error>
where
    P: Protocol,
    T: Transport,
    C: Clock,
    S: Slots,
    F: ReadField,
    I: IntoIterator<Item = Id>,
{
    let (kinds, states) = slots.split_mut();
    let stages = group::resolve(kinds, ids, stages)?;
    group::check_out(stages.len(), out.len())?;
    group::stage_reads::<F>(kinds, stages)?;
    group::read_each::<P, T, C, F, AnyProtocol, _>(
        wire,
        kinds,
        states,
        stages,
        OnFailedGroup::Share,
        |target, kind, _, result| {
            if let Some(slot) = out.get_mut(target) {
                *slot = result.map(|raw| F::Value::from_raw(raw, &kind.data().scale));
            }
        },
    )
}

/// Reads with one result per target (`partial-reads` feature).
impl<P: Protocol, T: Transport, C: Clock, const N: usize> Bus<P, T, C, N> {
    /// Reads Present Position of every target, in radians, with one result
    /// per target: `out[i]` is target `i`'s position or its own error.
    ///
    /// Protocol 2 sends one Sync Read per register layout, Protocol 1 one
    /// Read per target, as [`read_positions`](Bus::read_positions) does.
    /// Extra slots of `out` stay untouched.
    ///
    /// - Protocol 1: each target is independent; a silent or failing target
    ///   only fails its own slot.
    /// - Protocol 2: a target that stays silent, sends a corrupted reply or
    ///   reports an error only fails its own slot; the others keep their
    ///   values. When the instruction of a register layout fails (a failing
    ///   transport), each target of that layout still waiting for its reply
    ///   gets that error, and the next layout is still read.
    ///
    /// Every reply passes the fault check, so a target reporting a hardware
    /// fault holds [`Error::Status`] and is
    /// [`Faulted`](crate::TorqueState::Faulted) when the call returns. Not
    /// retried, whatever [`Config::retries`](crate::Config::retries) says.
    ///
    /// # Errors
    ///
    /// Only refusals, with nothing sent and `out` unchanged:
    /// - [`SetupError::NotOnBus`]: a target the bus does not hold.
    /// - [`ValidationError::DuplicateId`]: a target named twice.
    /// - [`ValidationError::BufferLength`]: `out` is shorter than the
    ///   target count.
    ///
    /// Everything that happens on the wire is in `out`.
    #[cfg_attr(
        all(feature = "mock", feature = "x-series"),
        doc = r#"
Two XM430-W350 on Protocol 2, the second one silent:

```
use dxl_rs_v2::{Bus, Config, Error, Id, MockTime, MockTransport, ModelNumber, P2, TransportError};

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
# line.reply_to(9, &with_crc(&[0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x08, 0x00, 0x55, 0x00, 0x00, 0x08, 0x00, 0x00]));
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

let mut out = [Ok(dxl_rs_v2::Position::ZERO); 2];
bus.read_positions_partial([a, b], &mut out)?;
assert!(out[0].is_ok());
assert_eq!(out[1], Err(TransportError::Timeout { id: Some(b) }.into()));
# Ok::<(), Error>(())
```
"#
    )]
    ///
    /// [`SetupError::NotOnBus`]: crate::SetupError::NotOnBus
    /// [`ValidationError::DuplicateId`]: crate::ValidationError::DuplicateId
    /// [`ValidationError::BufferLength`]: crate::ValidationError::BufferLength
    pub fn read_positions_partial<I: IntoIterator<Item = Id>>(
        &mut self,
        ids: I,
        out: &mut [Result<Position, Error>],
    ) -> Result<(), Error> {
        self.read_partial::<PresentPosition, I>(ids, out)
    }

    /// Reads Present Position of every target, in ticks, with one result
    /// per target: `out[i]` is target `i`'s tick or its own error. Same
    /// rules as [`read_positions_partial`](Bus::read_positions_partial).
    ///
    /// # Errors
    ///
    /// As [`read_positions_partial`](Bus::read_positions_partial): only
    /// refusals, with nothing sent.
    pub fn read_positions_raw_partial<I: IntoIterator<Item = Id>>(
        &mut self,
        ids: I,
        out: &mut [Result<i32, Error>],
    ) -> Result<(), Error> {
        self.read_partial::<Raw<PresentPosition>, I>(ids, out)
    }

    /// Reads field `F` of `ids` into `out`, one result per target.
    fn read_partial<F: ReadField, I: IntoIterator<Item = Id>>(
        &mut self,
        ids: I,
        out: &mut [Result<F::Value, Error>],
    ) -> Result<(), Error> {
        let mut stages = [Stage::EMPTY; N];
        let (mut wire, mut slots) = self.split();
        read_partial::<P, T, C, _, F, I>(&mut wire, &mut slots, &mut stages, ids, out)
    }
}

#[cfg(test)]
mod tests {
    use crate::bus::testing::*;
    use crate::error::{Error, SetupError};
    use crate::transport::MockTime;

    const SENTINEL: Result<i32, Error> = Ok(-7);

    #[cfg(any(feature = "x-series", feature = "ax"))]
    fn timeout(n: u8) -> Result<i32, Error> {
        Err(crate::TransportError::Timeout { id: Some(id(n)) }.into())
    }

    #[test]
    fn empty_targets_send_nothing() {
        let time = MockTime::new();
        let mut bus = test_bus::<crate::protocol::P2, 32>(&time);
        let mark = mark(&bus);
        let mut out = [SENTINEL; 2];
        assert_eq!(bus.read_positions_raw_partial([], &mut out), Ok(()));
        assert_eq!(bus.read_positions_partial(bus.ids(), &mut []), Ok(()));
        assert_eq!(
            bus.read_positions_raw_partial([id(1)], &mut out),
            Err(SetupError::NotOnBus { id: id(1) }.into())
        );
        assert_eq!(out, [SENTINEL; 2]);
        assert_quiet(&bus, mark);
    }

    #[cfg(feature = "x-series")]
    mod x_series {
        extern crate std;

        use core::cell::Cell;

        use super::*;
        use crate::Transport;
        use crate::actuator::{Actuator, Describe};
        use crate::bus::group::{self, OnFailedGroup, Stage};
        use crate::bus::slots::Slots;
        use crate::bus::wire::{ReadMany, Wire};
        use crate::error::{StatusError, StatusReport, TransportError, ValidationError};
        use crate::field::{PresentPosition, Raw};
        use crate::models::ActuatorKind;
        use crate::protocol::{Clock, P2, Protocol};
        use crate::units::Position;
        use crate::{Id, TorqueState};

        /// XM430-W350 at IDs 1, 2 and 3, torque off.
        fn three_xm(time: &MockTime) -> TestBus<'_, P2> {
            let mut bus = test_bus(time);
            for n in [1, 2, 3] {
                hold(&mut bus, XM430, n, (0, 4095), BOUNDED, OFF);
            }
            bus
        }

        fn present(n: u8, raw: i32) -> std::vec::Vec<u8> {
            p2_status(n, 0, &raw.to_le_bytes())
        }

        fn alert(n: u8) -> Result<i32, Error> {
            Err(StatusError {
                id: id(n),
                report: StatusReport::P2 {
                    alert: true,
                    code: None,
                },
            }
            .into())
        }

        #[test]
        fn one_silent_target_fails_only_its_slot() {
            let time = MockTime::new();
            let mut bus = three_xm(&time);
            line(&mut bus)
                .reply_to(1, &present(3, 300))
                .reply_to(1, &present(1, 100));
            let mut out = [SENTINEL; 4];
            assert_eq!(
                bus.read_positions_raw_partial([id(3), id(2), id(1)], &mut out),
                Ok(())
            );
            assert_eq!(out, [Ok(300), timeout(2), Ok(100), SENTINEL]);
            assert_eq!(sent(&bus), p2_sync_read(132, 4, &[3, 2, 1]));
            assert_eq!(bus.torque_states(), [OFF; 3]);
        }

        #[test]
        fn an_alert_fails_its_slot_and_faults_it() {
            let time = MockTime::new();
            let mut bus = three_xm(&time);
            line(&mut bus)
                .reply_to(1, &present(1, 10))
                .reply_to(1, &p2_status(2, 0x80, &20i32.to_le_bytes()))
                .reply_to(1, &present(3, 30));
            let mut out = [SENTINEL; 3];
            bus.read_positions_raw_partial(bus.ids(), &mut out).unwrap();
            assert_eq!(out, [Ok(10), alert(2), Ok(30)]);
            assert_eq!(bus.torque_states(), [OFF, FAULTED, OFF]);
            assert_done(&bus);
        }

        #[test]
        fn a_failing_instruction_fails_every_slot_of_its_group() {
            let time = MockTime::new();
            let mut bus = three_xm(&time);
            line(&mut bus).fail_with(embedded_io::ErrorKind::BrokenPipe);
            let mut out = [SENTINEL; 3];
            assert_eq!(bus.read_positions_raw_partial(bus.ids(), &mut out), Ok(()));
            let io: Result<i32, Error> =
                Err(TransportError::Io(embedded_io::ErrorKind::BrokenPipe).into());
            assert_eq!(out, [io; 3]);
            assert_eq!(bus.torque_states(), [OFF; 3]);
        }

        #[test]
        fn typed_reads_are_radians() {
            let time = MockTime::new();
            let mut bus = three_xm(&time);
            line(&mut bus).reply_to(1, &present(2, 3072));
            let scale = bus.actuators()[0].position_scale();
            let mut out = [Ok(Position::from_radians(9.0)); 2];
            bus.read_positions_partial([id(2), id(3)], &mut out)
                .unwrap();
            assert_eq!(
                out,
                [
                    Ok(scale.to_position(3072)),
                    Err(TransportError::Timeout { id: Some(id(3)) }.into())
                ]
            );
        }

        #[test]
        fn mixed_layouts_are_one_sync_read_each() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            hold(&mut bus, XM430, 1, (0, 4095), BOUNDED, ON);
            hold(&mut bus, XL320, 2, (0, 1023), BOUNDED, ON);
            hold(&mut bus, XM430, 3, (0, 4095), BOUNDED, ON);
            line(&mut bus)
                .reply_to(1, &present(3, 33))
                .reply_to(2, &p2_status(2, 0x80, &700u16.to_le_bytes()));
            let mut out = [SENTINEL; 3];
            bus.read_positions_raw_partial(bus.ids(), &mut out).unwrap();
            assert_eq!(
                sent(&bus),
                concat(&[p2_sync_read(132, 4, &[1, 3]), p2_sync_read(37, 2, &[2])])
            );
            assert_eq!(out, [timeout(1), alert(2), Ok(33)]);
            assert_eq!(bus.torque_states(), [ON, FAULTED, ON]);
        }

        #[test]
        fn refusals_send_nothing_and_leave_out() {
            let time = MockTime::new();
            let mut bus = three_xm(&time);
            let mark = mark(&bus);
            let mut out = [SENTINEL; 2];
            assert_eq!(
                bus.read_positions_raw_partial(bus.ids(), &mut out),
                Err(ValidationError::BufferLength { targets: 3, got: 2 }.into())
            );
            assert_eq!(
                bus.read_positions_raw_partial([id(1), id(9)], &mut out),
                Err(SetupError::NotOnBus { id: id(9) }.into())
            );
            assert_eq!(
                bus.read_positions_partial([id(2), id(2)], &mut [Ok(Position::ZERO); 2]),
                Err(ValidationError::DuplicateId { id: id(2) }.into())
            );
            assert_eq!(out, [SENTINEL; 2]);
            assert_quiet(&bus, mark);
        }

        /// Answers the XM430 layout `(132, 4)` for every ID; for any other
        /// layout, answers the first ID and then fails like a broken line.
        struct XlLayoutBreaks;

        impl<P: Protocol, T: Transport, C: Clock> ReadMany<P, T, C> for XlLayoutBreaks {
            fn run<I, F>(
                _: &mut Wire<'_, P, T, C>,
                addr: u16,
                _: u8,
                ids: I,
                _: &mut [u8],
                mut on_reply: F,
            ) -> Result<(), Error>
            where
                I: Iterator<Item = Id> + Clone,
                F: FnMut(Id, Result<&[u8], Error>),
            {
                let mut ids = ids;
                if addr == 132 {
                    for id in ids {
                        on_reply(id, Ok(&[id.get(), 0, 0, 0]));
                    }
                    return Ok(());
                }
                if let Some(first) = ids.next() {
                    on_reply(first, Ok(&[0x80, 0]));
                }
                Err(TransportError::Io(embedded_io::ErrorKind::BrokenPipe).into())
            }
        }

        #[test]
        fn a_group_failing_midway_keeps_the_replies_already_read() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            hold(&mut bus, XL320, 1, (0, 1023), BOUNDED, ON);
            hold(&mut bus, XM430, 2, (0, 4095), BOUNDED, ON);
            hold(&mut bus, XL320, 3, (0, 1023), BOUNDED, ON);
            let mut stages = [Stage::EMPTY; 32];
            let mut got: [Option<Result<i32, Error>>; 3] = [None; 3];
            let (mut wire, mut slots) = bus.split();
            let (kinds, states) = slots.split_mut();
            let staged = group::resolve(kinds, [id(1), id(2), id(3)], &mut stages).unwrap();
            group::stage_reads::<Raw<PresentPosition>>(kinds, staged).unwrap();
            group::read_each::<P2, _, _, Raw<PresentPosition>, XlLayoutBreaks, _>(
                &mut wire,
                kinds,
                states,
                staged,
                OnFailedGroup::Share,
                |target, kind: &ActuatorKind, _: &Cell<Stage>, result| {
                    assert_eq!(kind.data().present_position.size, [2, 4, 2][target]);
                    got[target] = Some(result);
                },
            )
            .unwrap();
            let io = Err(TransportError::Io(embedded_io::ErrorKind::BrokenPipe).into());
            assert_eq!(got, [Some(Ok(0x80)), Some(Ok(2)), Some(io)]);
            assert_eq!(
                group::read_stages::<P2, _, _, Raw<PresentPosition>, XlLayoutBreaks>(
                    &mut wire, kinds, states, staged
                ),
                io.map(drop)
            );
            assert_eq!(states, [TorqueState::On; 3]);
        }
    }

    #[cfg(feature = "ax")]
    mod ax {
        use super::*;
        use crate::error::StatusError;
        use crate::protocol::P1;

        #[test]
        fn p1_slots_are_independent() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            for n in [1, 2, 3] {
                hold(&mut bus, AX12, n, (0, 1023), BOUNDED, ON);
            }
            line(&mut bus)
                .reply_to(1, &p1_status(1, 0, &100u16.to_le_bytes()))
                .reply_to(3, &p1_status(3, 0x20, &300u16.to_le_bytes()));
            let mut out = [SENTINEL; 3];
            bus.read_positions_raw_partial(bus.ids(), &mut out).unwrap();
            assert_eq!(out[..2], [Ok(100), timeout(2)]);
            assert!(
                matches!(out[2], Err(Error::Status(StatusError { id, .. })) if id.get() == 3),
                "{:?}",
                out[2]
            );
            assert_eq!(bus.torque_states(), [ON, ON, FAULTED]);
            assert_eq!(
                sent(&bus),
                concat(&[p1_read(1, 36, 2), p1_read(2, 36, 2), p1_read(3, 36, 2)])
            );
        }
    }
}
