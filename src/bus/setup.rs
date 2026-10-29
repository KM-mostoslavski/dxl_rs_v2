//! Setup: find the actuators, read what the bus must know about each, and
//! hold them with torque off (research R-6, R-9).

use crate::actuator::{Describe, LimitMode, ModelNumber};
use crate::error::{Error, SetupError, TransportError, ValidationError};
use crate::models::{self, ActuatorKind};
use crate::protocol::{Clock, Protocol};
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
}
