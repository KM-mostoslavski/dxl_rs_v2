//! Setup: find the actuators, read what the bus must know about each, and
//! hold them with torque off (research R-6, R-9).

use crate::actuator::{Describe, LimitMode, ModelNumber};
use crate::error::{Error, SetupError, StatusError, TransportError, ValidationError};
use crate::models::{self, ActuatorKind};
use crate::protocol::{Clock, Pong, Protocol};
use crate::{Id, Transport};

use super::slots::Slots;
use super::wire::Wire;

/// Declared Setup: pings only the listed actuators.
///
/// Replaces what `slots` held. The list is checked before any byte: more
/// than `N` entries, a repeated `Id` or a model this build cannot drive on
/// `P` fail at once. On any error `slots` is left empty.
pub(crate) fn declare<P, T, C, S, I, const N: usize>(
    wire: &mut Wire<'_, P, T, C>,
    slots: &mut S,
    list: I,
) -> Result<(), Error>
where
    P: Protocol,
    T: Transport,
    C: Clock,
    S: Slots,
    I: IntoIterator<Item = (Id, ModelNumber)>,
{
    slots.clear();
    let result = declare_all::<P, T, C, S, I, N>(wire, slots, list);
    if result.is_err() {
        slots.clear();
    }
    result
}

fn declare_all<P, T, C, S, I, const N: usize>(
    wire: &mut Wire<'_, P, T, C>,
    slots: &mut S,
    list: I,
) -> Result<(), Error>
where
    P: Protocol,
    T: Transport,
    C: Clock,
    S: Slots,
    I: IntoIterator<Item = (Id, ModelNumber)>,
{
    let mut entries = heapless::Vec::<(Id, ModelNumber), N>::new();
    for (id, model) in list {
        if entries.iter().any(|&(held, _)| held == id) {
            return Err(ValidationError::DuplicateId { id }.into());
        }
        entries
            .push((id, model))
            .map_err(|_| SetupError::TooManyActuators { capacity: N })?;
    }
    entries.sort_unstable_by_key(|&(id, _)| id);
    for &(id, model) in &entries {
        ActuatorKind::new(P::VERSION, model, 0, id).map(drop)?;
    }
    for &(id, declared) in &entries {
        let pong = match wire.ping(id) {
            Err(Error::Transport(TransportError::Timeout { .. })) => {
                return Err(SetupError::Missing { id }.into());
            }
            result => result?,
        };
        let found = match pong.model_number {
            Some(number) => number,
            None => read_model_number(wire, id)?,
        };
        if found != declared.get() {
            return Err(SetupError::WrongModel {
                id,
                declared: declared.get(),
                found,
            }
            .into());
        }
        init_one(wire, slots, id, declared, pong.firmware)?;
    }
    Ok(())
}

/// Setup by scanning: a P2 broadcast ping, or a P1 sweep of IDs 0..=252.
///
/// Replaces what `slots` held. Every reply is collected first; a corrupted
/// reply, a reply reporting an error, two replies with one `Id`, or more
/// than `N` replies fail Setup before anything else is sent. The replies are
/// then set up in `Id` order like declared actuators. On any error `slots`
/// is left empty.
pub(crate) fn autoscan<P, T, C, S, const N: usize>(
    wire: &mut Wire<'_, P, T, C>,
    slots: &mut S,
) -> Result<(), Error>
where
    P: Protocol,
    T: Transport,
    C: Clock,
    S: Slots,
{
    slots.clear();
    let result = autoscan_all::<P, T, C, S, N>(wire, slots);
    if result.is_err() {
        slots.clear();
    }
    result
}

fn autoscan_all<P, T, C, S, const N: usize>(
    wire: &mut Wire<'_, P, T, C>,
    slots: &mut S,
) -> Result<(), Error>
where
    P: Protocol,
    T: Transport,
    C: Clock,
    S: Slots,
{
    let mut pongs = heapless::Vec::<Pong, N>::new();
    let mut failure = None;
    wire.scan(|reply| {
        if failure.is_none() {
            failure = collect::<N>(&mut pongs, reply).err();
        }
    })?;
    if let Some(err) = failure {
        return Err(err);
    }
    pongs.sort_unstable_by_key(|pong| pong.id);
    for pong in &pongs {
        let model = match pong.model_number {
            Some(number) => number,
            None => read_model_number(wire, pong.id)?,
        };
        init_one(wire, slots, pong.id, ModelNumber(model), pong.firmware)?;
    }
    Ok(())
}

/// Adds one scan reply to `pongs`, or says why Setup fails.
fn collect<const N: usize>(
    pongs: &mut heapless::Vec<Pong, N>,
    reply: Result<Pong, Error>,
) -> Result<(), Error> {
    let pong = reply?;
    let id = pong.id;
    if let Some(report) = pong.fault {
        return Err(StatusError { id, report }.into());
    }
    if pongs.iter().any(|held| held.id == id) {
        return Err(ValidationError::DuplicateId { id }.into());
    }
    pongs
        .push(pong)
        .map_err(|_| SetupError::TooManyActuators { capacity: N }.into())
}

/// Reads Model Number, which every table has at the same place.
fn read_model_number<P: Protocol, T: Transport, C: Clock>(
    wire: &mut Wire<'_, P, T, C>,
    id: Id,
) -> Result<u16, Error> {
    let raw = wire.read_value(id, models::MODEL_NUMBER)?;
    Ok(u16::try_from(raw).unwrap_or(u16::MAX))
}

/// Builds one actuator of `model` at `id` from what it reports, and pushes
/// it with torque `Off`. `firmware` is the version a P2 ping returned.
///
/// 1. Firmware, read only where the model's table depends on it.
/// 2. The model variant for this protocol and firmware.
/// 3. The EEPROM limits (CW is the minimum, CCW the maximum).
/// 4. The limit mode: the mode register's value through the model's own
///    lists, or the angle limits where the model has no mode register.
/// 5. The slot, `Off`. No torque write.
fn init_one<P, T, C, S>(
    wire: &mut Wire<'_, P, T, C>,
    slots: &mut S,
    id: Id,
    model: ModelNumber,
    firmware: Option<u8>,
) -> Result<(), Error>
where
    P: Protocol,
    T: Transport,
    C: Clock,
    S: Slots,
{
    let firmware = match (firmware, ActuatorKind::firmware_register(P::VERSION, model)) {
        (Some(firmware), _) => firmware,
        (None, Some(reg)) => u8::try_from(wire.read_value(id, reg)?).unwrap_or(u8::MAX),
        (None, None) => 0,
    };
    let mut kind = ActuatorKind::new(P::VERSION, model, firmware, id)?;
    let data = kind.data();
    let min = wire.read_value(id, data.limits.min())?;
    let max = wire.read_value(id, data.limits.max())?;
    let limits = kind.cache_mut().record_limits(min, max)?;
    let mode = match data.mode.register() {
        Some(reg) => {
            let mut value = [0];
            wire.read(id, reg, &mut value)?;
            let [value] = value;
            data.mode
                .limit_mode(value)
                .ok_or(SetupError::UnknownOperatingMode { id, value })?
        }
        None => LimitMode::from_angle_limits(limits),
    };
    kind.cache_mut().record_mode(mode);
    slots.push(kind)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::testing::*;
    use crate::protocol::{P1, P2, ProtocolVersion};
    use crate::transport::MockTime;

    fn model(n: u16) -> ModelNumber {
        ModelNumber(n)
    }

    #[test]
    fn unsupported_declared_model_sends_nothing() {
        let time = MockTime::new();
        let mut bus = test_bus::<P2, 32>(&time);
        let mark = mark(&bus);
        assert_eq!(
            bus.declare([(id(1), model(9999))]),
            Err(SetupError::UnsupportedModel {
                id: id(1),
                model_number: 9999,
                protocol: ProtocolVersion::P2,
                feature: None,
            }
            .into())
        );
        assert_quiet(&bus, mark);

        let mut bus = test_bus::<P1, 32>(&time);
        assert_eq!(
            bus.declare([(id(3), model(XL320))]),
            Err(SetupError::UnsupportedModel {
                id: id(3),
                model_number: XL320,
                protocol: ProtocolVersion::P1,
                feature: None,
            }
            .into())
        );
        assert_quiet(&bus, mark);
        assert!(bus.actuators().is_empty());
    }

    #[test]
    fn too_many_declared() {
        let time = MockTime::new();
        let mut bus = test_bus::<P2, 2>(&time);
        let mark = mark(&bus);
        let list = [1, 2, 3].map(|n| (id(n), model(XM430)));
        assert_eq!(
            bus.declare(list),
            Err(SetupError::TooManyActuators { capacity: 2 }.into())
        );
        assert_quiet(&bus, mark);
    }

    #[test]
    fn duplicate_declared_id() {
        let time = MockTime::new();
        let mut bus = test_bus::<P2, 32>(&time);
        let mark = mark(&bus);
        assert_eq!(
            bus.declare([(id(4), model(XM430)), (id(4), model(XM430))]),
            Err(ValidationError::DuplicateId { id: id(4) }.into())
        );
        assert_quiet(&bus, mark);
    }

    #[test]
    fn empty_list_sets_up_nothing() {
        let time = MockTime::new();
        let mut bus = test_bus::<P2, 32>(&time);
        let mark = mark(&bus);
        assert_eq!(bus.declare([]), Ok(&[] as &[ActuatorKind]));
        assert_quiet(&bus, mark);
    }

    #[cfg(feature = "x-series")]
    mod x_series {
        use super::*;
        use crate::actuator::{Actuator, TorqueState};
        use crate::error::{StatusError, StatusReport};

        fn xm430_bus(time: &MockTime, mode: u8) -> TestBus<'_, P2> {
            let mut bus = test_bus(time);
            xm430_p2(line(&mut bus), 1, 1, 1000, 3000, mode);
            bus.declare([(id(1), model(XM430))]).unwrap();
            bus
        }

        #[test]
        fn declare_reads_limits_and_starts_off() {
            let time = MockTime::new();
            let bus = xm430_bus(&time, 3);
            assert_eq!(sent(&bus), xm430_p2_sent(1));
            assert_done(&bus);
            assert_eq!(bus.torque_states(), [TorqueState::Off]);
            let held = &bus.actuators()[0];
            assert!(matches!(held, ActuatorKind::Xm430W350(_)));
            assert_eq!(held.id(), id(1));
            assert_eq!(held.position_limits_raw(), Some(1000..=3000));
        }

        #[test]
        fn declare_xl320_reads_control_mode() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            xl320_p2(line(&mut bus), 1, 2, 0, 1023, 2);
            bus.declare([(id(2), model(XL320))]).unwrap();
            assert_eq!(sent(&bus), xl320_p2_sent(2));
            assert_eq!(bus.actuators()[0].cache().check_goal(512), Ok(512));

            let mut bus = test_bus::<P2, 32>(&time);
            xl320_p2(line(&mut bus), 1, 2, 0, 1023, 1);
            bus.declare([(id(2), model(XL320))]).unwrap();
            assert_eq!(
                bus.actuators()[0].cache().check_goal(512),
                Err(ValidationError::NotPositionMode { id: id(2) })
            );
        }

        #[test]
        fn silent_declared_id_is_missing() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            assert_eq!(
                bus.declare([(id(5), model(XM430))]),
                Err(SetupError::Missing { id: id(5) }.into())
            );
            assert!(bus.actuators().is_empty());
        }

        #[test]
        fn other_model_is_wrong_model() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            line(&mut bus).reply_to(1, &p2_pong(5, 1060, 45));
            assert_eq!(
                bus.declare([(id(5), model(XM430))]),
                Err(SetupError::WrongModel {
                    id: id(5),
                    declared: XM430,
                    found: 1060
                }
                .into())
            );
            assert!(bus.actuators().is_empty());
        }

        #[test]
        fn limit_read_timeout_fails_setup() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            line(&mut bus)
                .reply_to(1, &p2_pong(1, XM430, XM430_FW))
                .reply_to(2, &p2_status(1, 0, &1000i32.to_le_bytes()));
            assert!(matches!(
                bus.declare([(id(1), model(XM430))]),
                Err(Error::Transport(TransportError::Timeout { .. }))
            ));
            assert!(bus.actuators().is_empty());
            assert!(bus.torque_states().is_empty());
        }

        #[test]
        fn inverted_limits_fail_setup() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            xm430_p2(line(&mut bus), 1, 1, 3000, 1000, 3);
            assert_eq!(
                bus.declare([(id(1), model(XM430))]),
                Err(ValidationError::InvertedLimits {
                    id: id(1),
                    min: 3000,
                    max: 1000
                }
                .into())
            );
            assert!(bus.actuators().is_empty());
        }

        #[test]
        fn fault_at_setup_fails_setup() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            line(&mut bus).reply_to(1, &p2_status(1, 0x80, &[0xFC, 0x03, XM430_FW]));
            assert_eq!(
                bus.declare([(id(1), model(XM430))]),
                Err(StatusError {
                    id: id(1),
                    report: StatusReport::P2 {
                        alert: true,
                        code: None
                    }
                }
                .into())
            );
            assert!(bus.actuators().is_empty());
        }

        #[test]
        fn second_declare_replaces_and_failed_one_empties() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            let next = xm430_p2(line(&mut bus), 1, 1, 0, 4095, 3);
            bus.declare([(id(1), model(XM430))]).unwrap();
            xm430_p2(line(&mut bus), next, 2, 0, 4095, 3);
            let held = bus.declare([(id(2), model(XM430))]).unwrap();
            assert_eq!(held.len(), 1);
            assert_eq!(held[0].id(), id(2));
            assert_eq!(
                bus.declare([(id(3), model(XM430))]),
                Err(SetupError::Missing { id: id(3) }.into())
            );
            assert!(bus.actuators().is_empty());
        }

        #[test]
        fn declared_list_is_held_in_id_order() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            let mut at = 1;
            for n in [1, 2, 3] {
                at = xm430_p2(line(&mut bus), at, n, 0, 4095, 3);
            }
            let list = [3, 1, 2].map(|n| (id(n), model(XM430)));
            let held = bus.declare(list).unwrap();
            let ids: [u8; 3] = core::array::from_fn(|i| held[i].id().get());
            assert_eq!(ids, [1, 2, 3]);
            assert_eq!(
                sent(&bus),
                concat(&[xm430_p2_sent(1), xm430_p2_sent(2), xm430_p2_sent(3)])
            );
            assert_eq!(bus.torque_states(), [TorqueState::Off; 3]);
        }

        #[test]
        fn mode_is_recorded_at_setup() {
            for (mode, expected) in [
                (
                    3,
                    Err(ValidationError::OutOfLimits {
                        id: id(1),
                        raw: 3500,
                        min: 1000,
                        max: 3000,
                    }),
                ),
                (4, Ok(3500)),
                (5, Ok(3500)),
                (1, Err(ValidationError::NotPositionMode { id: id(1) })),
                (0, Err(ValidationError::NotPositionMode { id: id(1) })),
                (16, Err(ValidationError::NotPositionMode { id: id(1) })),
            ] {
                let time = MockTime::new();
                let bus = xm430_bus(&time, mode);
                assert_eq!(
                    bus.actuators()[0].cache().check_goal(3500),
                    expected,
                    "mode {mode}"
                );
            }
        }

        #[test]
        fn unknown_mode_fails_setup() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            xm430_p2(line(&mut bus), 1, 1, 0, 4095, 2);
            assert_eq!(
                bus.declare([(id(1), model(XM430))]),
                Err(SetupError::UnknownOperatingMode {
                    id: id(1),
                    value: 2
                }
                .into())
            );
            assert!(bus.actuators().is_empty());
        }

        #[test]
        fn mode_read_timeout_fails_setup() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            line(&mut bus)
                .reply_to(1, &p2_pong(1, XM430, XM430_FW))
                .reply_to(2, &p2_status(1, 0, &0i32.to_le_bytes()))
                .reply_to(3, &p2_status(1, 0, &4095i32.to_le_bytes()));
            assert!(matches!(
                bus.declare([(id(1), model(XM430))]),
                Err(Error::Transport(TransportError::Timeout { .. }))
            ));
            assert_eq!(sent(&bus), xm430_p2_sent(1));
            assert!(bus.actuators().is_empty());
        }

        #[test]
        fn p1_model_read_fault_fails_setup() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            line(&mut bus)
                .reply_to(1, &p1_status(1, 0, &[]))
                .reply_to(2, &p1_status(1, 0x20, &XM430.to_le_bytes()));
            assert!(matches!(
                bus.declare([(id(1), model(XM430))]),
                Err(Error::Status(_))
            ));
            assert!(bus.actuators().is_empty());
        }
    }

    #[cfg(feature = "ax")]
    mod ax {
        use super::*;
        use crate::actuator::{Actuator, TorqueState};

        #[test]
        fn declare_ax12_reads_model_and_angle_limits_without_mode() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            ax12_p1(line(&mut bus), 1, 1, 0, 1023);
            bus.declare([(id(1), model(AX12))]).unwrap();
            assert_eq!(sent(&bus), ax12_p1_sent(1));
            assert_done(&bus);
            assert_eq!(bus.torque_states(), [TorqueState::Off]);
            assert_eq!(bus.actuators()[0].position_limits_raw(), Some(0..=1023));
            assert_eq!(bus.actuators()[0].cache().check_goal(512), Ok(512));
        }

        #[test]
        fn ax12_wheel_mode_is_recorded() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            ax12_p1(line(&mut bus), 1, 1, 0, 0);
            bus.declare([(id(1), model(AX12))]).unwrap();
            assert_eq!(sent(&bus), ax12_p1_sent(1));
            assert_eq!(
                bus.actuators()[0].cache().check_goal(0),
                Err(ValidationError::NotPositionMode { id: id(1) })
            );
        }

        #[test]
        fn p1_model_mismatch_is_wrong_model() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            angle_p1(line(&mut bus), 1, 1, 18, 0, 1023);
            assert_eq!(
                bus.declare([(id(1), model(AX12))]),
                Err(SetupError::WrongModel {
                    id: id(1),
                    declared: AX12,
                    found: 18
                }
                .into())
            );
        }
    }

    #[cfg(feature = "rx")]
    mod rx {
        use super::*;

        #[test]
        fn declare_eight_rx28_on_p1() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            let mut at = 1;
            for n in 1..=8 {
                at = angle_p1(line(&mut bus), at, n, RX28, 0, 1023);
            }
            let held = bus.declare((1..=8).map(|n| (id(n), model(RX28)))).unwrap();
            assert_eq!(held.len(), 8);
            assert!(
                held.iter()
                    .all(|kind| matches!(kind, ActuatorKind::Rx28(_)))
            );
            assert_eq!(bus.actuators()[7].cache().check_goal(1023), Ok(1023));
            assert_done(&bus);
        }
    }

    mod scan {
        extern crate std;

        use super::*;
        use crate::error::{ProtocolError, StatusError, StatusReport};

        fn p1_sweep() -> std::vec::Vec<u8> {
            concat(
                &(0..=Id::MAX)
                    .map(|n| p1_instruction(n, PING, &[]))
                    .collect::<std::vec::Vec<_>>(),
            )
        }

        fn broadcast_ping() -> std::vec::Vec<u8> {
            p2_instruction(0xFE, PING, &[])
        }

        #[test]
        fn p2_autoscan_empty_is_bounded() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            assert_eq!(bus.autoscan(), Ok(&[] as &[ActuatorKind]));
            assert_eq!(
                sent(&bus),
                [0xFF, 0xFF, 0xFD, 0x00, 0xFE, 0x03, 0x00, 0x01, 0x31, 0x42]
            );
            assert!(time.now() <= 10_000 + 1_000, "took {} us", time.now());
        }

        #[test]
        fn p1_autoscan_empty_is_bounded() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            assert_eq!(bus.autoscan(), Ok(&[] as &[ActuatorKind]));
            assert_eq!(sent(&bus), p1_sweep());
            assert!(
                time.now() <= 253 * 10_000 + 10_000,
                "took {} us",
                time.now()
            );
        }

        #[test]
        fn corrupt_scan_reply_fails() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            let mut pong = p2_pong(3, XM430, XM430_FW);
            let last = pong.len() - 1;
            pong[last] ^= 0xFF;
            line(&mut bus).reply_to(1, &pong);
            assert!(matches!(
                bus.autoscan(),
                Err(Error::Protocol(ProtocolError::Crc { .. }))
            ));
            assert_eq!(sent(&bus), broadcast_ping());
            assert!(bus.actuators().is_empty());
        }

        #[test]
        fn two_pongs_one_id() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            line(&mut bus)
                .reply_to(1, &p2_pong(3, XM430, XM430_FW))
                .reply_to(1, &p2_pong(3, XL320, 37));
            assert_eq!(
                bus.autoscan(),
                Err(ValidationError::DuplicateId { id: id(3) }.into())
            );
            assert_eq!(sent(&bus), broadcast_ping());
            assert!(bus.actuators().is_empty());
        }

        #[test]
        fn pong_with_alert_fails() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            let [lo, hi] = XM430.to_le_bytes();
            line(&mut bus).reply_to(1, &p2_status(3, 0x80, &[lo, hi, XM430_FW]));
            assert_eq!(
                bus.autoscan(),
                Err(StatusError {
                    id: id(3),
                    report: StatusReport::P2 {
                        alert: true,
                        code: None
                    }
                }
                .into())
            );
            assert!(bus.actuators().is_empty());
        }

        #[test]
        fn scan_past_capacity() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 2>(&time);
            for n in [1, 2, 3] {
                line(&mut bus).reply_to(1, &p2_pong(n, XM430, XM430_FW));
            }
            assert_eq!(
                bus.autoscan(),
                Err(SetupError::TooManyActuators { capacity: 2 }.into())
            );
            assert_eq!(sent(&bus), broadcast_ping());
            assert!(bus.actuators().is_empty());
        }

        #[test]
        fn p2_only_model_on_p1_fails_setup() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            line(&mut bus)
                .reply_to(5, &p1_status(4, 0, &[]))
                .reply_to(254, &p1_status(4, 0, &XL320.to_le_bytes()));
            assert_eq!(
                bus.autoscan(),
                Err(SetupError::UnsupportedModel {
                    id: id(4),
                    model_number: XL320,
                    protocol: ProtocolVersion::P1,
                    feature: None,
                }
                .into())
            );
            assert_eq!(sent(&bus), concat(&[p1_sweep(), p1_read(4, 0, 2)]));
            assert!(bus.actuators().is_empty());
        }

        #[cfg(not(feature = "pro"))]
        #[test]
        fn family_off_names_feature() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            line(&mut bus).reply_to(1, &p2_pong(7, 2000, 1));
            assert_eq!(
                bus.autoscan(),
                Err(SetupError::UnsupportedModel {
                    id: id(7),
                    model_number: 2000,
                    protocol: ProtocolVersion::P2,
                    feature: Some("pro"),
                }
                .into())
            );
            assert!(bus.actuators().is_empty());
        }

        #[test]
        fn autoscan_replaces_a_declared_list() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            assert_eq!(bus.declare([]), Ok(&[] as &[ActuatorKind]));
            line(&mut bus).reply_to(1, &p2_pong(1, 9999, 1));
            assert!(bus.autoscan().is_err());
            assert!(bus.actuators().is_empty());
        }

        #[cfg(feature = "x-series")]
        mod x_series {
            use super::*;
            use crate::actuator::{Actuator, TorqueState};

            #[test]
            fn p2_autoscan_sorts_three() {
                let time = MockTime::new();
                let mut bus = test_bus::<P2, 32>(&time);
                for n in [3, 1, 2] {
                    line(&mut bus).reply_to(1, &p2_pong(n, XM430, XM430_FW));
                }
                let mut at = 2;
                for n in [1, 2, 3] {
                    at = xm430_p2_reads(line(&mut bus), at, n, 0, 4095, 3);
                }
                let held = bus.autoscan().unwrap();
                let ids: [u8; 3] = core::array::from_fn(|i| held[i].id().get());
                assert_eq!(ids, [1, 2, 3]);
                assert_eq!(
                    sent(&bus),
                    concat(&[
                        broadcast_ping(),
                        xm430_p2_reads_sent(1),
                        xm430_p2_reads_sent(2),
                        xm430_p2_reads_sent(3),
                    ])
                );
                assert_eq!(bus.torque_states(), [TorqueState::Off; 3]);
                assert_done(&bus);
            }

            #[test]
            fn firmware_picks_the_table() {
                for (fw, fw52) in [(52, true), (53, false)] {
                    let time = MockTime::new();
                    let mut bus = test_bus::<P2, 32>(&time);
                    line(&mut bus).reply_to(1, &p2_pong(4, 1190, fw));
                    xm430_p2_reads(line(&mut bus), 2, 4, 0, 4095, 3);
                    let held = bus.autoscan().unwrap();
                    assert_eq!(
                        matches!(held[0], ActuatorKind::Xl330M077Fw52(_)),
                        fw52,
                        "firmware {fw}"
                    );
                    assert_eq!(
                        matches!(held[0], ActuatorKind::Xl330M077(_)),
                        !fw52,
                        "firmware {fw}"
                    );
                }
            }

            #[test]
            fn autoscan_reads_the_mode_like_declare() {
                let time = MockTime::new();
                let mut bus = test_bus::<P2, 32>(&time);
                line(&mut bus).reply_to(1, &p2_pong(1, XM430, XM430_FW));
                xm430_p2_reads(line(&mut bus), 2, 1, 0, 4095, 2);
                assert_eq!(
                    bus.autoscan(),
                    Err(SetupError::UnknownOperatingMode {
                        id: id(1),
                        value: 2
                    }
                    .into())
                );
                assert!(bus.actuators().is_empty());
            }
        }

        #[cfg(feature = "rx")]
        mod rx {
            use super::*;
            use crate::actuator::TorqueState;

            #[test]
            fn p1_autoscan_some_answer() {
                let time = MockTime::new();
                let mut bus = test_bus::<P1, 32>(&time);
                line(&mut bus)
                    .reply_to(2, &p1_status(1, 0, &[]))
                    .reply_to(6, &p1_status(5, 0, &[]));
                let at = angle_p1_reads(line(&mut bus), 254, 1, RX28, 0, 1023);
                angle_p1_reads(line(&mut bus), at, 5, RX28, 0, 1023);
                let held = bus.autoscan().unwrap();
                assert_eq!(held.len(), 2);
                assert!(matches!(held[0], ActuatorKind::Rx28(_)));
                assert!(matches!(held[1], ActuatorKind::Rx28(_)));
                assert_eq!(
                    sent(&bus),
                    concat(&[
                        p1_sweep(),
                        p1_read(1, 0, 2),
                        p1_read(1, 6, 2),
                        p1_read(1, 8, 2),
                        p1_read(5, 0, 2),
                        p1_read(5, 6, 2),
                        p1_read(5, 8, 2),
                    ])
                );
                assert_eq!(bus.torque_states(), [TorqueState::Off; 2]);
                assert_done(&bus);
            }
        }
    }
}
