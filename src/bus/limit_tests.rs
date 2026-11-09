//! Typed EEPROM limits on the bus (plan 2026-11-09, D1 to D7), byte for
//! byte on a scripted line: the setters' packets, every refusal before a
//! byte, the cache that follows a write, and the getters.
//!
//! Register addresses, sizes, units and ranges are those of the models'
//! control tables, verified against their e-Manual pages: AX-12A
//! (<https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/>, 2026-11-03),
//! XM430-W350 units (<https://emanual.robotis.com/docs/en/dxl/x/xm430-w350/>,
//! 2026-11-09) and MX-64(2.0) Acceleration Limit
//! (<https://emanual.robotis.com/docs/en/dxl/mx/mx-64-2/>, 2026-11-09).

extern crate std;

/// `instruction` of `register` at `addr` (`len` bytes) on actuator `n`.
#[cfg(any(feature = "ax", feature = "mx", feature = "x-series"))]
fn on(
    instruction: crate::error::Instruction,
    n: u8,
    register: &'static str,
    addr: u16,
    len: u16,
) -> crate::error::Request {
    crate::error::Request::NONE
        .instruction(instruction)
        .id(crate::bus::testing::id(n))
        .register(register)
        .addr(addr)
        .len(len)
}

#[cfg(feature = "ax")]
mod ax12a {
    use super::on;
    use crate::actuator::{Actuator, LimitMode, ModelNumber};
    use crate::bus::testing::*;
    use crate::error::{
        Decivolts, Error, FaultError, Instruction, TransportError, ValidationError, VoltageError,
    };
    use crate::field::{
        GoalVelocity, MaxPositionLimit, MaxVoltageLimit, MinPositionLimit, MinVoltageLimit, Raw,
        SpeedLimit as SpeedLimitField, TemperatureLimit,
    };
    use crate::protocol::P1;
    use crate::transport::MockTime;
    use crate::units::{
        Acceleration, Current, HardwareErrors, Load, Position, Pwm, SpeedLimit, Temperature,
        Velocity, Voltage,
    };
    use crate::{IdSet, TorqueState};

    /// An AX-12A at ID 1 with these angle limits, as Setup leaves it:
    /// torque `Off`, not confirmed off.
    fn declared(time: &MockTime, cw: u16, ccw: u16) -> TestBus<'_, P1> {
        let mut bus = test_bus(time);
        ax12_p1(line(&mut bus), 1, 1, cw, ccw);
        bus.declare([(id(1), ModelNumber(AX12))]).unwrap();
        bus
    }

    /// The same, torque `on`, or else confirmed off by an acknowledged
    /// `disable_torque()`, as a limit write needs (plan D8).
    fn ax12(time: &MockTime, cw: u16, ccw: u16, on: bool) -> TestBus<'_, P1> {
        let mut bus = declared(time, cw, ccw);
        ack(&mut bus, 1);
        let mut ax = bus.actuator(id(1)).unwrap();
        if on {
            ax.enable_torque().unwrap();
        } else {
            ax.disable_torque().unwrap();
        }
        bus
    }

    /// Writes Moving Speed = 0 with an acknowledged `set_speed_limit`, as a
    /// switch to wheel mode needs (plan D9).
    fn zero_speed(bus: &mut TestBus<'_, P1>) {
        ack(bus, 1);
        bus.actuator(id(1))
            .unwrap()
            .set_speed_limit(SpeedLimit::Unlimited)
            .unwrap();
    }

    /// The angle `raw` ticks of an AX-12A stand for.
    fn at(bus: &mut TestBus<'_, P1>, raw: i32) -> Position {
        bus.actuator(id(1))
            .unwrap()
            .position_scale()
            .to_position(raw)
    }

    /// Plan D8: the refusal of a limit write to `register` while torque is
    /// not confirmed off.
    fn unconfirmed(register: &'static str) -> Result<(), Error> {
        Err(ValidationError::TorqueUnconfirmed {
            id: id(1),
            register,
        }
        .into())
    }

    /// Plan D9: the refusal of a switch to wheel mode.
    fn speed_unknown() -> Result<(), Error> {
        Err(ValidationError::WheelSpeedUnknown { id: id(1) }.into())
    }

    /// Scripts an empty acknowledgement from actuator 1 to each of the
    /// next `count` packets.
    #[cfg(feature = "raw-registers")]
    fn acks(bus: &mut TestBus<'_, P1>, count: usize) {
        script(bus, &std::vec![p1_status(1, 0, &[]); count]);
    }

    /// Plan D3, D5: Min Voltage Limit(12) and Max Voltage Limit(13) are
    /// adjacent, so the pair is one Write; voltages are not cached.
    #[test]
    fn voltage_limits_are_one_write() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 1023, false);
        let mark = mark(&bus);
        ack(&mut bus, 1);
        let mut ax = bus.actuator(id(1)).unwrap();
        let before = ax.kind().clone();
        ax.set_voltage_limits(Voltage::from_volts(9.5), Voltage::from_volts(14.0))
            .unwrap();
        assert_eq!(*ax.kind(), before);
        assert_eq!(sent_since(&bus, mark), p1_write(1, 12, &[95, 140]));
        assert_done(&bus);
    }

    /// Plan D3: each single setter writes its register, in its table's unit.
    #[test]
    fn each_setter_writes_its_register() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 1023, false);
        let mark = mark(&bus);
        let mut sent = std::vec::Vec::new();
        for (call, packet) in [
            (0, p1_write(1, 11, &[70])),
            (1, p1_write(1, 14, &1023u16.to_le_bytes())),
            (2, p1_write(1, 14, &512u16.to_le_bytes())),
            (3, p1_write(1, 13, &[160])),
            (4, p1_write(1, 12, &[60])),
        ] {
            ack(&mut bus, 1);
            let mut ax = bus.actuator(id(1)).unwrap();
            let result = match call {
                0 => ax.set_temperature_limit(Temperature::from_celsius(70.0)),
                1 => ax.set_max_torque(Load::from_ratio(1.0)),
                2 => ax.set_max_torque(Load::from_ratio(0.5)),
                3 => ax.write_limit(Raw(MaxVoltageLimit), 160),
                _ => ax.write_limit(MinVoltageLimit, Voltage::from_volts(6.0)),
            };
            assert_eq!(result, Ok(()), "call {call}");
            sent.extend(packet);
        }
        assert_eq!(sent_since(&bus, mark), sent);
        assert_done(&bus);
    }

    /// Plan D4, D5: CW(6) and CCW(8) Angle Limit are one Write; the cache
    /// follows, and goals are checked against the new limits.
    #[test]
    fn position_limits_move_the_goal_check() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 1023, false);
        let (min, max) = (at(&mut bus, 200), at(&mut bus, 800));
        let mark = mark(&bus);
        ack(&mut bus, 1);
        let mut ax = bus.actuator(id(1)).unwrap();
        ax.set_position_limits(min, max).unwrap();
        assert_eq!(ax.position_limits_raw(), Some(200..=800));
        assert_eq!(ax.limit_mode(), LimitMode::Bounded);
        ack(&mut bus, 1);
        bus.actuator(id(1)).unwrap().enable_torque().unwrap();
        assert_eq!(
            bus.actuator(id(1)).unwrap().set_goal_position_raw(100),
            Err(ValidationError::OutOfLimits {
                id: id(1),
                raw: 100,
                min: 200,
                max: 800
            }
            .into())
        );
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[p1_write(1, 6, &[200, 0, 0x20, 0x03]), p1_write(1, 24, &[1]),])
        );
    }

    /// Plan D5, D9: on Protocol 1 the limit mode follows the angle limits as
    /// Setup would have recorded it: CW = CCW = 0 is wheel mode, entered
    /// after Moving Speed = 0, and back.
    #[test]
    fn angle_limits_switch_wheel_mode_and_back() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 1023, false);
        let mark = mark(&bus);
        script(&mut bus, &[p1_status(1, 0, &[]), p1_status(1, 0, &[])]);
        let mut ax = bus.actuator(id(1)).unwrap();
        ax.set_speed_limit(SpeedLimit::Unlimited).unwrap();
        ax.write_limit(Raw(MaxPositionLimit), 0).unwrap();
        assert_eq!(ax.limit_mode(), LimitMode::Wheel);
        assert_eq!(ax.position_limits_raw(), Some(0..=0));
        script(&mut bus, &[p1_status(1, 0, &[]), p1_status(1, 0, &[])]);
        let mut ax = bus.actuator(id(1)).unwrap();
        ax.enable_torque().unwrap();
        assert_eq!(
            ax.set_goal_position_raw(512),
            Err(ValidationError::NotPositionMode { id: id(1) }.into())
        );
        assert_eq!(ax.write(Raw(GoalVelocity), 1024 + 300), Ok(()));
        script(&mut bus, &[p1_status(1, 0, &[]), p1_status(1, 0, &[])]);
        let mut ax = bus.actuator(id(1)).unwrap();
        ax.disable_torque().unwrap();
        let (cw, ccw) = (
            ax.position_scale().to_position(0),
            ax.position_scale().to_position(1023),
        );
        ax.set_position_limits(cw, ccw).unwrap();
        assert_eq!(ax.limit_mode(), LimitMode::Bounded);
        assert_eq!(ax.position_limits_raw(), Some(0..=1023));
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[
                p1_write(1, 32, &[0, 0]),
                p1_write(1, 8, &[0, 0]),
                p1_write(1, 24, &[1]),
                p1_write(1, 32, &(1024u16 + 300).to_le_bytes()),
                p1_write(1, 24, &[0]),
                p1_write(1, 6, &[0, 0, 0xFF, 0x03]),
            ])
        );
        assert_done(&bus);
    }

    /// Plan D4: with torque on every limit write is refused, naming the
    /// register, and nothing is sent.
    #[test]
    fn torque_on_refuses_every_limit() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 1023, true);
        let (min, max) = (at(&mut bus, 200), at(&mut bus, 800));
        let mark = mark(&bus);
        let mut ax = bus.actuator(id(1)).unwrap();
        let on = |register| {
            Err(ValidationError::ActuatorIsOn {
                id: id(1),
                register,
            }
            .into())
        };
        assert_eq!(
            ax.set_voltage_limits(Voltage::from_volts(9.5), Voltage::from_volts(14.0)),
            on("Min Voltage Limit")
        );
        assert_eq!(ax.set_position_limits(min, max), on("CW Angle Limit"));
        assert_eq!(
            ax.set_temperature_limit(Temperature::from_celsius(70.0)),
            on("Temperature Limit")
        );
        assert_eq!(ax.set_max_torque(Load::from_ratio(1.0)), on("Max Torque"));
        assert_eq!(
            ax.write_limit(Raw(MaxPositionLimit), 900),
            on("CCW Angle Limit")
        );
        assert_eq!(ax.torque_state(), TorqueState::On);
        assert_quiet(&bus, mark);
    }

    /// Plan D8, FR-2.7: Setup reads no Torque Enable, so after it every
    /// limit write is refused, naming its register, with nothing sent,
    /// until an acknowledged `disable_torque()` confirms torque off.
    #[test]
    fn setup_leaves_torque_unconfirmed() {
        let time = MockTime::new();
        let mut bus = declared(&time, 0, 1023);
        let (min, max) = (at(&mut bus, 200), at(&mut bus, 800));
        let mark = mark(&bus);
        let mut ax = bus.actuator(id(1)).unwrap();
        assert_eq!(ax.torque_state(), TorqueState::Off);
        assert_eq!(
            ax.set_voltage_limits(Voltage::from_volts(9.5), Voltage::from_volts(14.0)),
            unconfirmed("Min Voltage Limit")
        );
        assert_eq!(
            ax.set_position_limits(min, max),
            unconfirmed("CW Angle Limit")
        );
        assert_eq!(
            ax.set_temperature_limit(Temperature::from_celsius(70.0)),
            unconfirmed("Temperature Limit")
        );
        assert_eq!(
            ax.set_max_torque(Load::from_ratio(1.0)),
            unconfirmed("Max Torque")
        );
        assert_eq!(
            ax.write_limit(Raw(MaxPositionLimit), 900),
            unconfirmed("CCW Angle Limit")
        );
        assert_quiet(&bus, mark);
        script(&mut bus, &[p1_status(1, 0, &[]), p1_status(1, 0, &[])]);
        let mut ax = bus.actuator(id(1)).unwrap();
        ax.disable_torque().unwrap();
        assert_eq!(
            ax.set_voltage_limits(Voltage::from_volts(9.5), Voltage::from_volts(14.0)),
            Ok(())
        );
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[p1_write(1, 24, &[0]), p1_write(1, 12, &[95, 140])])
        );
        assert_done(&bus);
    }

    /// Plan D8: a new Setup clears the mark: it reads no Torque Enable.
    #[test]
    fn setup_again_clears_the_mark() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 1023, false);
        let next = line(&mut bus).flushes() + 1;
        ax12_p1(line(&mut bus), next, 1, 0, 1023);
        bus.declare([(id(1), ModelNumber(AX12))]).unwrap();
        let quiet = mark(&bus);
        assert_eq!(
            bus.actuator(id(1))
                .unwrap()
                .set_temperature_limit(Temperature::from_celsius(70.0)),
            unconfirmed("Temperature Limit")
        );
        assert_quiet(&bus, quiet);
    }

    /// Plan D8: the multi-actuator `disable_torque` confirms torque off as
    /// the handle's does, one acknowledged write each.
    #[test]
    fn bus_disable_torque_confirms_torque_off() {
        let time = MockTime::new();
        let mut bus = declared(&time, 0, 1023);
        let mark = mark(&bus);
        script(&mut bus, &[p1_status(1, 0, &[]), p1_status(1, 0, &[])]);
        bus.disable_torque([id(1)]).unwrap();
        assert_eq!(
            bus.actuator(id(1))
                .unwrap()
                .set_temperature_limit(Temperature::from_celsius(70.0)),
            Ok(())
        );
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[p1_write(1, 24, &[0]), p1_write(1, 11, &[70])])
        );
    }

    /// Plan D8: every `enable_torque` that sent bytes clears the mark, on
    /// the handle and on the bus, even when it timed out and the bus holds
    /// `Off`.
    #[test]
    fn enable_attempt_clears_the_mark() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 1023, false);
        let mut ax = bus.actuator(id(1)).unwrap();
        assert!(matches!(ax.enable_torque(), Err(Error::Transport(_))));
        assert_eq!(ax.torque_state(), TorqueState::Off);
        let quiet = mark(&bus);
        assert_eq!(
            bus.actuator(id(1))
                .unwrap()
                .write_limit(Raw(MinVoltageLimit), 60),
            unconfirmed("Min Voltage Limit")
        );
        assert_quiet(&bus, quiet);

        let mut bus = ax12(&time, 0, 1023, false);
        assert!(bus.enable_torque([id(1)]).is_err());
        assert_eq!(bus.torque_state(id(1)), Some(TorqueState::Off));
        let quiet = mark(&bus);
        assert_eq!(
            bus.actuator(id(1))
                .unwrap()
                .write_limit(Raw(MinVoltageLimit), 60),
            unconfirmed("Min Voltage Limit")
        );
        assert_quiet(&bus, quiet);
    }

    /// Plan D8: a Torque Enable = 0 write whose reply was lost leaves the
    /// bus holding `Off` while the hardware's torque may still be on; the
    /// AX-12A e-Manual documents no EEPROM lock with torque on, so a limit
    /// write waits for an acknowledged torque-off.
    #[test]
    fn unconfirmed_torque_off_refuses_limit_writes() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 1023, true);
        assert!(bus.actuator(id(1)).unwrap().disable_torque().is_err());
        assert_eq!(bus.torque_state(id(1)), Some(TorqueState::Off));
        let quiet = mark(&bus);
        assert_eq!(
            bus.actuator(id(1))
                .unwrap()
                .write_limit(Raw(MaxPositionLimit), 0),
            unconfirmed("CCW Angle Limit")
        );
        assert_quiet(&bus, quiet);

        let mut bus = ax12(&time, 0, 1023, false);
        assert!(bus.actuator(id(1)).unwrap().disable_torque().is_err());
        let quiet = mark(&bus);
        assert_eq!(
            bus.actuator(id(1))
                .unwrap()
                .write_limit(Raw(MaxPositionLimit), 900),
            unconfirmed("CCW Angle Limit")
        );
        assert_quiet(&bus, quiet);
    }

    /// Plan D8: an AX-12A the bus turned on keeps its torque when it
    /// reports an Input Voltage error (not in the default Alarm Shutdown,
    /// 36), so `Faulted` reached from `On` does not pass the EEPROM guard.
    #[test]
    fn faulted_from_on_refuses_limit_writes() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 1023, true);
        let zero = at(&mut bus, 0);
        script(
            &mut bus,
            &[
                p1_status(1, 0x01, &[40]),
                p1_status(1, 0x01, &[70]),
                p1_status(1, 0x01, &[95]),
                p1_status(1, 0x01, &[140]),
            ],
        );
        assert!(
            bus.actuator(id(1))
                .unwrap()
                .read(crate::field::PresentTemperature)
                .is_err()
        );
        assert_eq!(bus.torque_state(id(1)), Some(FAULTED));
        let quiet = mark(&bus);
        assert_eq!(
            bus.actuator(id(1)).unwrap().set_position_limits(zero, zero),
            unconfirmed("CW Angle Limit")
        );
        assert_quiet(&bus, quiet);
    }

    /// Plan D8: a fault first reported on the reply to `disable_torque()`
    /// from `On` (Input Voltage only) still shows the write ran: the call
    /// returns the measured fault and the state becomes `Faulted`, yet
    /// torque is confirmed off and the voltage limits can be restored, as
    /// `examples/ax12_sine` does at the end of a run (NFR-11).
    #[test]
    fn fault_first_reported_on_disable_confirms_torque_off() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 1023, true);
        let mark = mark(&bus);
        script(
            &mut bus,
            &[
                p1_status(1, 0x01, &[]),
                p1_status(1, 0x01, &[95]),
                p1_status(1, 0x01, &[100]),
                p1_status(1, 0x01, &[140]),
                p1_status(1, 0, &[]),
            ],
        );
        let mut ax = bus.actuator(id(1)).unwrap();
        let off = ax.disable_torque();
        assert!(off.as_ref().is_err_and(Error::is_hardware_fault));
        assert_eq!(
            off,
            Err(FaultError::Voltage(VoltageError::BelowMin {
                id: id(1),
                measured: Decivolts(95),
                min: Decivolts(100),
                max: Decivolts(140),
                faults: HardwareErrors::from_raw(0x01),
                request: on(Instruction::Write, 1, "Torque Enable", 24, 1),
            })
            .into())
        );
        assert_eq!(ax.torque_state(), FAULTED);
        assert_eq!(
            ax.set_voltage_limits(Voltage::from_volts(6.0), Voltage::from_volts(16.0)),
            Ok(())
        );
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[
                p1_write(1, 24, &[0]),
                p1_read(1, 42, 1),
                p1_read(1, 12, 1),
                p1_read(1, 13, 1),
                p1_write(1, 12, &[60, 160]),
            ])
        );
        assert_done(&bus);
    }

    /// Plan D4: the register check comes first: a model without the
    /// register refuses whatever its torque state.
    #[test]
    fn registers_the_ax_lacks_are_unsupported() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 1023, true);
        let mark = mark(&bus);
        let mut ax = bus.actuator(id(1)).unwrap();
        let missing = |register| {
            Err(ValidationError::Unsupported {
                id: id(1),
                register,
            }
            .into())
        };
        assert_eq!(
            ax.set_velocity_limit(Velocity::from_rad_per_s(1.0)),
            missing("Velocity Limit")
        );
        assert_eq!(
            ax.set_current_limit(Current::from_amps(1.0)),
            missing("Current Limit")
        );
        assert_eq!(ax.set_pwm_limit(Pwm::from_ratio(0.5)), missing("PWM Limit"));
        assert_eq!(
            ax.set_acceleration_limit(Acceleration::from_rad_per_s2(1.0)),
            missing("Acceleration Limit")
        );
        assert_eq!(
            ax.acceleration_limit(),
            missing("Acceleration Limit").map(|()| Acceleration::ZERO)
        );
        assert_eq!(
            ax.velocity_limit(),
            missing("Velocity Limit").map(|()| Velocity::ZERO)
        );
        assert_quiet(&bus, mark);
    }

    /// Plan D4: values outside the e-Manual range, inverted pairs and
    /// values that convert to no register value send nothing.
    #[test]
    fn bad_values_send_nothing() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 200, 800, false);
        let (low, high) = (at(&mut bus, 900), at(&mut bus, 100));
        let mark = mark(&bus);
        let mut ax = bus.actuator(id(1)).unwrap();
        let v = Voltage::from_volts;
        let out = |raw, min, max| {
            Err(ValidationError::OutOfLimits {
                id: id(1),
                raw,
                min,
                max,
            }
            .into())
        };
        let inverted = |min, max| {
            Err(ValidationError::InvertedLimits {
                id: id(1),
                min,
                max,
            }
            .into())
        };
        assert_eq!(ax.set_voltage_limits(v(4.0), v(14.0)), out(40, 50, 160));
        assert_eq!(ax.set_voltage_limits(v(9.5), v(16.5)), out(165, 50, 160));
        assert_eq!(
            ax.set_temperature_limit(Temperature::from_celsius(101.0)),
            out(101, 0, 100)
        );
        assert_eq!(
            ax.write_limit(Raw(MaxPositionLimit), 1024),
            out(1024, 0, 1023)
        );
        assert_eq!(ax.set_voltage_limits(v(14.0), v(9.5)), inverted(140, 95));
        assert_eq!(ax.set_position_limits(low, high), inverted(900, 100));
        assert_eq!(
            ax.write_limit(Raw(MinPositionLimit), 900),
            inverted(900, 800)
        );
        assert_eq!(
            ax.write_limit(Raw(MaxPositionLimit), 100),
            inverted(200, 100)
        );
        assert_eq!(
            ax.set_voltage_limits(v(f32::NAN), v(14.0)),
            Err(ValidationError::NotFinite {
                request: on(Instruction::Write, 1, "Min Voltage Limit", 12, 1),
            }
            .into())
        );
        assert_eq!(
            ax.set_max_torque(Load::from_ratio(-0.5)),
            Err(ValidationError::DoesNotFit {
                raw: -512,
                size: 2,
                request: on(Instruction::Write, 1, "Max Torque", 14, 2),
            }
            .into())
        );
        assert_eq!(ax.position_limits_raw(), Some(200..=800));
        assert_quiet(&bus, mark);
    }

    /// Plan D4: a ratio is rounded to the nearest raw step of Max
    /// Torque(14) before it is checked, so a negative ratio within half a
    /// step is written as raw 0, and one below it does not fit.
    #[test]
    fn small_negative_ratio_rounds_to_zero() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 1023, false);
        let mark = mark(&bus);
        ack(&mut bus, 1);
        let mut ax = bus.actuator(id(1)).unwrap();
        assert_eq!(ax.set_max_torque(Load::from_ratio(-0.0004)), Ok(()));
        assert_eq!(
            ax.set_max_torque(Load::from_ratio(-0.0006)),
            Err(ValidationError::DoesNotFit {
                raw: -1,
                size: 2,
                request: on(Instruction::Write, 1, "Max Torque", 14, 2),
            }
            .into())
        );
        assert_eq!(sent_since(&bus, mark), p1_write(1, 14, &[0, 0]));
        assert_done(&bus);
    }

    /// Plan D8: `Faulted` passes the torque check once an acknowledged
    /// `disable_torque()` confirmed torque off, like `Off`; the fault
    /// stays.
    #[test]
    fn faulted_actuator_takes_limit_writes_once_confirmed_off() {
        let time = MockTime::new();
        let mut bus = test_bus::<P1, 32>(&time);
        hold(&mut bus, AX12, 1, (0, 1023), LimitMode::Bounded, FAULTED);
        let mut ax = bus.actuator(id(1)).unwrap();
        assert_eq!(
            ax.set_voltage_limits(Voltage::from_volts(6.0), Voltage::from_volts(16.0)),
            unconfirmed("Min Voltage Limit")
        );
        assert!(sent(&bus).is_empty());
        script(&mut bus, &[p1_status(1, 0, &[]), p1_status(1, 0, &[])]);
        let mut ax = bus.actuator(id(1)).unwrap();
        ax.disable_torque().unwrap();
        assert_eq!(
            ax.set_voltage_limits(Voltage::from_volts(6.0), Voltage::from_volts(16.0)),
            Ok(())
        );
        assert_eq!(ax.torque_state(), FAULTED);
        assert_eq!(
            sent(&bus),
            concat(&[p1_write(1, 24, &[0]), p1_write(1, 12, &[60, 160])])
        );
    }

    /// Plan D4, D8, FR-2.7, FR-8.1: an AX-12A held `Faulted` at Setup (its
    /// input voltage below Min Voltage Limit) answers `disable_torque()`
    /// with its Input Voltage flag only: the call returns the measured
    /// fault, yet the instruction ran, so torque is confirmed off and the
    /// typed setter fixes the limits; then the fault is cleared and torque
    /// enabled, as kmr needs (NFR-11).
    #[test]
    fn faulted_at_setup_is_fixed_with_set_voltage_limits() {
        let time = MockTime::new();
        let mut bus = test_bus::<P1, 32>(&time);
        line(&mut bus)
            .reply_to(1, &p1_status(1, 0x01, &[]))
            .reply_to(2, &p1_status(1, 0x01, &AX12.to_le_bytes()))
            .reply_to(3, &p1_status(1, 0x01, &0u16.to_le_bytes()))
            .reply_to(4, &p1_status(1, 0x01, &1023u16.to_le_bytes()))
            .reply_to(5, &p1_status(1, 0x01, &[]))
            .reply_to(6, &p1_status(1, 0x01, &[95]))
            .reply_to(7, &p1_status(1, 0x01, &[100]))
            .reply_to(8, &p1_status(1, 0x01, &[140]))
            .reply_to(9, &p1_status(1, 0, &[]))
            .reply_to(10, &p1_status(1, 0, &[]))
            .reply_to(11, &p1_status(1, 0, &[]));
        let report = bus.declare([(id(1), ModelNumber(AX12))]).unwrap();
        assert_eq!(report.faulted, [id(1)].into_iter().collect::<IdSet>());
        let mark = mark(&bus);
        let mut ax = bus.actuator(id(1)).unwrap();
        assert_eq!(
            ax.enable_torque(),
            Err(ValidationError::ActuatorFaulted { id: id(1) }.into())
        );
        assert_eq!(
            ax.disable_torque(),
            Err(FaultError::Voltage(VoltageError::BelowMin {
                id: id(1),
                measured: Decivolts(95),
                min: Decivolts(100),
                max: Decivolts(140),
                faults: HardwareErrors::from_raw(0x01),
                request: on(Instruction::Write, 1, "Torque Enable", 24, 1),
            })
            .into())
        );
        assert_eq!(
            ax.set_voltage_limits(Voltage::from_volts(6.0), Voltage::from_volts(16.0)),
            Ok(())
        );
        assert_eq!(ax.torque_state(), FAULTED);
        assert_eq!(ax.clear_fault(), Ok(()));
        assert_eq!(ax.enable_torque(), Ok(()));
        assert_eq!(ax.torque_state(), ON);
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[
                p1_write(1, 24, &[0]),
                p1_read(1, 42, 1),
                p1_read(1, 12, 1),
                p1_read(1, 13, 1),
                p1_write(1, 12, &[60, 160]),
                p1_instruction(1, PING, &[]),
                p1_write(1, 24, &[1]),
            ])
        );
    }

    /// Plan D8: `clear_fault` sends no reboot on Protocol 1 (a ping), so
    /// it leaves torque confirmed off.
    #[test]
    fn clear_fault_keeps_the_mark() {
        let time = MockTime::new();
        let mut bus = test_bus::<P1, 32>(&time);
        hold(&mut bus, AX12, 1, (0, 1023), LimitMode::Bounded, FAULTED);
        let mark = mark(&bus);
        script(
            &mut bus,
            &[
                p1_status(1, 0, &[]),
                p1_status(1, 0, &[]),
                p1_status(1, 0, &[]),
            ],
        );
        let mut ax = bus.actuator(id(1)).unwrap();
        ax.disable_torque().unwrap();
        assert_eq!(ax.clear_fault(), Ok(()));
        assert_eq!(ax.torque_state(), TorqueState::Off);
        assert_eq!(
            ax.set_temperature_limit(Temperature::from_celsius(70.0)),
            Ok(())
        );
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[
                p1_write(1, 24, &[0]),
                p1_instruction(1, PING, &[]),
                p1_write(1, 11, &[70]),
            ])
        );
    }

    /// Plan D8, D9 (`raw-registers`): a raw write may touch Torque Enable
    /// or Moving Speed, so it clears both marks.
    #[cfg(feature = "raw-registers")]
    #[test]
    fn write_raw_clears_both_marks() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 1023, false);
        zero_speed(&mut bus);
        let zero = at(&mut bus, 0);
        ack(&mut bus, 1);
        assert_eq!(bus.actuator(id(1)).unwrap().write_raw(25, &[1]), Ok(()));
        let quiet = mark(&bus);
        assert_eq!(
            bus.actuator(id(1))
                .unwrap()
                .write_limit(Raw(MaxVoltageLimit), 160),
            unconfirmed("Max Voltage Limit")
        );
        assert_quiet(&bus, quiet);
        ack(&mut bus, 1);
        bus.actuator(id(1)).unwrap().disable_torque().unwrap();
        let quiet = mark(&bus);
        assert_eq!(
            bus.actuator(id(1)).unwrap().set_position_limits(zero, zero),
            speed_unknown()
        );
        assert_quiet(&bus, quiet);
    }

    /// Plan D9 (`raw-registers`): raw writes put an AX-12A held in wheel
    /// mode back in joint mode with Moving Speed 512, which the bus does
    /// not see, so the wheel mode it holds no longer exempts a CW = CCW = 0
    /// write: pair or single, it counts as a switch and is refused with
    /// nothing sent. A non-wheel pair, then Moving Speed = 0, gets back.
    #[cfg(feature = "raw-registers")]
    #[test]
    fn raw_write_voids_the_held_wheel_mode() {
        let time = MockTime::new();
        let mut bus = declared(&time, 0, 0);
        let (zero, top) = (at(&mut bus, 0), at(&mut bus, 1023));
        acks(&mut bus, 3);
        let mut ax = bus.actuator(id(1)).unwrap();
        assert_eq!(ax.limit_mode(), LimitMode::Wheel);
        assert_eq!(ax.write_raw(6, &[0, 0, 0xFF, 0x03]), Ok(()));
        assert_eq!(ax.write_raw(32, &512u16.to_le_bytes()), Ok(()));
        assert_eq!(ax.disable_torque(), Ok(()));
        assert_eq!(ax.limit_mode(), LimitMode::Wheel);
        let quiet = mark(&bus);
        let mut ax = bus.actuator(id(1)).unwrap();
        assert_eq!(ax.set_position_limits(zero, zero), speed_unknown());
        assert_eq!(ax.write_limit(Raw(MaxPositionLimit), 0), speed_unknown());
        assert_eq!(ax.write_limit(MinPositionLimit, zero), speed_unknown());
        assert_quiet(&bus, quiet);
        acks(&mut bus, 3);
        let mut ax = bus.actuator(id(1)).unwrap();
        assert_eq!(ax.set_position_limits(zero, top), Ok(()));
        assert_eq!(ax.limit_mode(), LimitMode::Bounded);
        assert_eq!(ax.set_speed_limit(SpeedLimit::Unlimited), Ok(()));
        assert_eq!(ax.set_position_limits(zero, zero), Ok(()));
        assert_eq!(ax.limit_mode(), LimitMode::Wheel);
        assert_eq!(
            sent_since(&bus, quiet),
            concat(&[
                p1_write(1, 6, &[0, 0, 0xFF, 0x03]),
                p1_write(1, 32, &[0, 0]),
                p1_write(1, 6, &[0, 0, 0, 0]),
            ])
        );
        assert_done(&bus);
    }

    /// Plan D5, D9 (`raw-registers`): after a raw write, only an
    /// acknowledged write of both angle limits makes the wheel mode the
    /// bus holds the hardware's again. A single limit write does not, so a
    /// later CW = CCW = 0 pair with Moving Speed live is still refused;
    /// after the pair, staying in wheel mode passes again, as it does after
    /// Setup.
    #[cfg(feature = "raw-registers")]
    #[test]
    fn position_pair_trusts_the_wheel_mode_again() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 0, true);
        let zero = at(&mut bus, 0);
        /// Drives a wheel-mode goal velocity, so Moving Speed is live, and
        /// confirms torque off again.
        fn live(bus: &mut TestBus<'_, P1>) {
            acks(bus, 3);
            let mut ax = bus.actuator(id(1)).unwrap();
            ax.enable_torque().unwrap();
            ax.write(Raw(GoalVelocity), 300).unwrap();
            ax.disable_torque().unwrap();
        }
        acks(&mut bus, 4);
        let mut ax = bus.actuator(id(1)).unwrap();
        assert_eq!(ax.write_raw(25, &[1]), Ok(()));
        assert_eq!(ax.write(Raw(GoalVelocity), 0), Ok(()));
        assert_eq!(ax.disable_torque(), Ok(()));
        assert_eq!(ax.write_limit(Raw(MaxPositionLimit), 0), Ok(()));
        live(&mut bus);
        let quiet = mark(&bus);
        assert_eq!(
            bus.actuator(id(1)).unwrap().set_position_limits(zero, zero),
            speed_unknown()
        );
        assert_quiet(&bus, quiet);
        acks(&mut bus, 4);
        let mut ax = bus.actuator(id(1)).unwrap();
        ax.enable_torque().unwrap();
        assert_eq!(ax.write(Raw(GoalVelocity), 0), Ok(()));
        ax.disable_torque().unwrap();
        assert_eq!(ax.set_position_limits(zero, zero), Ok(()));
        live(&mut bus);
        acks(&mut bus, 1);
        let mut ax = bus.actuator(id(1)).unwrap();
        assert_eq!(ax.set_position_limits(zero, zero), Ok(()));
        assert_eq!(ax.limit_mode(), LimitMode::Wheel);
        assert_done(&bus);
    }

    /// Plan D9, AX-12A Moving Speed(32): a limit write that switches to
    /// wheel mode, pair or single, typed or raw, is refused with nothing
    /// sent until an acknowledged write of Moving Speed = 0.
    #[test]
    fn wheel_switch_needs_moving_speed_zero() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 1023, false);
        let zero = at(&mut bus, 0);
        let quiet = mark(&bus);
        let mut ax = bus.actuator(id(1)).unwrap();
        assert_eq!(ax.set_position_limits(zero, zero), speed_unknown());
        assert_eq!(ax.write_limit(MaxPositionLimit, zero), speed_unknown());
        assert_eq!(ax.write_limit(Raw(MaxPositionLimit), 0), speed_unknown());
        assert_eq!(ax.limit_mode(), LimitMode::Bounded);
        assert_quiet(&bus, quiet);
        script(&mut bus, &[p1_status(1, 0, &[]), p1_status(1, 0, &[])]);
        let mut ax = bus.actuator(id(1)).unwrap();
        ax.set_speed_limit(SpeedLimit::Unlimited).unwrap();
        assert_eq!(ax.set_position_limits(zero, zero), Ok(()));
        assert_eq!(ax.limit_mode(), LimitMode::Wheel);
        assert_eq!(
            sent_since(&bus, quiet),
            concat(&[p1_write(1, 32, &[0, 0]), p1_write(1, 6, &[0, 0, 0, 0])])
        );
        assert_done(&bus);
    }

    /// Plan D9: `write(Raw(SpeedLimit), 0)` acknowledged counts as the
    /// typed `SpeedLimit::Unlimited` does.
    #[test]
    fn raw_moving_speed_zero_allows_the_switch() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 1023, false);
        let mark = mark(&bus);
        script(&mut bus, &[p1_status(1, 0, &[]), p1_status(1, 0, &[])]);
        let mut ax = bus.actuator(id(1)).unwrap();
        ax.write(Raw(SpeedLimitField), 0).unwrap();
        assert_eq!(ax.write_limit(Raw(MaxPositionLimit), 0), Ok(()));
        assert_eq!(ax.limit_mode(), LimitMode::Wheel);
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[p1_write(1, 32, &[0, 0]), p1_write(1, 8, &[0, 0])])
        );
    }

    /// Plan D9: any other write of Moving Speed forgets that it is 0: a
    /// nonzero speed limit, a write of 0 whose reply was lost, and a Sync
    /// Write, which gets no reply, even of 0.
    #[test]
    fn other_moving_speed_writes_clear_the_mark() {
        let time = MockTime::new();
        let limited = SpeedLimit::Limited(Velocity::from_rad_per_s(5.0));
        for case in 0..3 {
            let mut bus = ax12(&time, 0, 1023, false);
            let zero = at(&mut bus, 0);
            zero_speed(&mut bus);
            match case {
                0 => {
                    ack(&mut bus, 1);
                    assert_eq!(
                        bus.actuator(id(1)).unwrap().set_speed_limit(limited),
                        Ok(())
                    );
                }
                1 => assert!(
                    bus.actuator(id(1))
                        .unwrap()
                        .set_speed_limit(SpeedLimit::Unlimited)
                        .is_err()
                ),
                _ => {
                    let before = mark(&bus);
                    assert_eq!(bus.sync_write(Raw(SpeedLimitField), [id(1)], &[0]), Ok(()));
                    assert_eq!(
                        sent_since(&bus, before),
                        p1_sync_write(32, 2, &[(1, &[0, 0])])
                    );
                }
            }
            let quiet = mark(&bus);
            assert_eq!(
                bus.actuator(id(1)).unwrap().set_position_limits(zero, zero),
                speed_unknown(),
                "case {case}"
            );
            assert_quiet(&bus, quiet);
        }
    }

    /// Plan D9: in wheel mode Moving Speed is the goal velocity, so a goal
    /// velocity write moves the mark too: 0 acknowledged makes it known,
    /// anything else unknown, and a later switch back to wheel mode
    /// follows the last one.
    #[test]
    fn goal_velocity_in_wheel_mode_moves_the_mark() {
        let time = MockTime::new();
        for (goal, back) in [(0, Ok(())), (300, speed_unknown())] {
            let mut bus = ax12(&time, 0, 0, true);
            let (zero, top) = (at(&mut bus, 0), at(&mut bus, 1023));
            script(
                &mut bus,
                &[
                    p1_status(1, 0, &[]),
                    p1_status(1, 0, &[]),
                    p1_status(1, 0, &[]),
                ],
            );
            let mut ax = bus.actuator(id(1)).unwrap();
            assert_eq!(ax.write(Raw(GoalVelocity), goal), Ok(()));
            ax.disable_torque().unwrap();
            assert_eq!(ax.set_position_limits(zero, top), Ok(()));
            assert_eq!(ax.limit_mode(), LimitMode::Bounded);
            if back.is_ok() {
                ack(&mut bus, 1);
            }
            let before = mark(&bus);
            assert_eq!(
                bus.actuator(id(1)).unwrap().set_position_limits(zero, zero),
                back,
                "goal {goal}"
            );
            if back.is_err() {
                assert_quiet(&bus, before);
            }
        }
    }

    /// Plan D9: leaving wheel mode, and staying in it, is never refused.
    #[test]
    fn leaving_or_staying_in_wheel_mode_passes() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 0, false);
        let (zero, top) = (at(&mut bus, 0), at(&mut bus, 1023));
        let mark = mark(&bus);
        script(
            &mut bus,
            &[
                p1_status(1, 0, &[]),
                p1_status(1, 0, &[]),
                p1_status(1, 0, &[]),
            ],
        );
        let mut ax = bus.actuator(id(1)).unwrap();
        assert_eq!(ax.set_position_limits(zero, zero), Ok(()));
        assert_eq!(ax.limit_mode(), LimitMode::Wheel);
        assert_eq!(ax.set_position_limits(zero, top), Ok(()));
        assert_eq!(ax.limit_mode(), LimitMode::Bounded);
        assert_eq!(ax.write_limit(Raw(MinPositionLimit), 10), Ok(()));
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[
                p1_write(1, 6, &[0, 0, 0, 0]),
                p1_write(1, 6, &[0, 0, 0xFF, 0x03]),
                p1_write(1, 6, &[10, 0]),
            ])
        );
    }

    /// Plan D4, D9: the wheel check comes after the torque and order
    /// checks, so a write refused for those says so first: torque not
    /// confirmed off, or the other limit unknown since a failed write.
    #[test]
    fn wheel_check_comes_last() {
        let time = MockTime::new();
        let mut bus = declared(&time, 0, 1023);
        let zero = at(&mut bus, 0);
        let quiet = mark(&bus);
        assert_eq!(
            bus.actuator(id(1)).unwrap().set_position_limits(zero, zero),
            unconfirmed("CW Angle Limit")
        );
        assert_quiet(&bus, quiet);
        let mut bus = ax12(&time, 0, 1023, false);
        let (lo, hi) = (at(&mut bus, 100), at(&mut bus, 900));
        assert!(
            bus.actuator(id(1))
                .unwrap()
                .set_position_limits(lo, hi)
                .is_err()
        );
        let quiet = mark(&bus);
        assert_eq!(
            bus.actuator(id(1))
                .unwrap()
                .write_limit(Raw(MaxPositionLimit), 0),
            Err(ValidationError::LimitsUnknown { id: id(1) }.into())
        );
        assert_quiet(&bus, quiet);
    }

    /// Plan D9: with the mode unknown after a failed angle-limit write,
    /// CW = CCW = 0 counts as a switch to wheel mode, even from wheel mode.
    #[test]
    fn unknown_mode_counts_as_a_switch() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 0, false);
        let zero = at(&mut bus, 0);
        assert!(matches!(
            bus.actuator(id(1)).unwrap().set_position_limits(zero, zero),
            Err(Error::Transport(_))
        ));
        let quiet = mark(&bus);
        assert_eq!(
            bus.actuator(id(1)).unwrap().set_position_limits(zero, zero),
            speed_unknown()
        );
        assert_quiet(&bus, quiet);
    }

    /// Plan D5: a position-limit write that times out leaves the limits and
    /// the wheel/joint mode unknown: every goal is refused, and a single
    /// limit write too, its order unknown.
    #[test]
    fn failed_angle_limit_write_refuses_goals() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 0, false);
        let (min, max) = (at(&mut bus, 0), at(&mut bus, 1023));
        let mut ax = bus.actuator(id(1)).unwrap();
        assert_eq!(
            ax.set_position_limits(min, max),
            Err(TransportError::NoReply {
                request: on(Instruction::Write, 1, "CW Angle Limit", 6, 4),
                timeout_us: 10_000,
            }
            .into())
        );
        assert_eq!(ax.position_limits_raw(), None);
        let unknown: Result<(), Error> = Err(ValidationError::LimitsUnknown { id: id(1) }.into());
        let quiet = mark(&bus);
        assert_eq!(
            bus.actuator(id(1))
                .unwrap()
                .write_limit(Raw(MinPositionLimit), 10),
            unknown
        );
        assert_quiet(&bus, quiet);
        ack(&mut bus, 1);
        bus.actuator(id(1)).unwrap().enable_torque().unwrap();
        let quiet = mark(&bus);
        let mut ax = bus.actuator(id(1)).unwrap();
        assert_eq!(ax.set_goal_position_raw(512), unknown);
        assert_eq!(ax.write(Raw(GoalVelocity), 300), unknown);
        assert_quiet(&bus, quiet);
    }

    /// Plan D5, AX-12A Moving Speed(32): in wheel mode Moving Speed is the
    /// goal velocity, so with the mode unknown after a failed angle-limit
    /// write it is refused on every tier, torque on or off, and nothing is
    /// sent.
    #[test]
    fn failed_angle_limit_write_refuses_moving_speed() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 1023, false);
        let zero = at(&mut bus, 0);
        zero_speed(&mut bus);
        assert!(matches!(
            bus.actuator(id(1)).unwrap().set_position_limits(zero, zero),
            Err(Error::Transport(_))
        ));
        let unknown: Result<(), Error> = Err(ValidationError::LimitsUnknown { id: id(1) }.into());
        let speed = SpeedLimit::Limited(Velocity::from_rad_per_s(5.0));
        for on in [false, true] {
            if on {
                ack(&mut bus, 1);
                bus.actuator(id(1)).unwrap().enable_torque().unwrap();
            }
            let quiet = mark(&bus);
            let mut ax = bus.actuator(id(1)).unwrap();
            assert_eq!(ax.set_speed_limit(speed), unknown, "torque on: {on}");
            assert_eq!(ax.write(Raw(SpeedLimitField), 1000), unknown, "{on}");
            assert_eq!(
                bus.sync_write(Raw(SpeedLimitField), [id(1)], &[1000]),
                unknown,
                "{on}"
            );
            assert_quiet(&bus, quiet);
        }
    }

    /// Plan D5, D9: after a failed angle-limit write, an acknowledged write
    /// of both determines them again: the limits and the wheel/joint mode
    /// are known, and goals are checked against them. With the mode
    /// unknown, CW = CCW = 0 counts as a switch to wheel mode, which
    /// Moving Speed = 0 allows.
    #[test]
    fn pair_write_after_a_failed_one_restores_limits_and_mode() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 1023, false);
        let (zero, lo, hi) = (at(&mut bus, 0), at(&mut bus, 100), at(&mut bus, 900));
        zero_speed(&mut bus);
        assert!(
            bus.actuator(id(1))
                .unwrap()
                .set_position_limits(zero, zero)
                .is_err()
        );
        let mark = mark(&bus);
        ack(&mut bus, 1);
        let mut ax = bus.actuator(id(1)).unwrap();
        ax.set_position_limits(zero, zero).unwrap();
        assert_eq!(ax.limit_mode(), LimitMode::Wheel);
        assert_eq!(ax.position_limits_raw(), Some(0..=0));
        assert_eq!(
            ax.set_speed_limit(SpeedLimit::Unlimited),
            Err(ValidationError::NotPositionMode { id: id(1) }.into())
        );
        ack(&mut bus, 1);
        let mut ax = bus.actuator(id(1)).unwrap();
        ax.set_position_limits(lo, hi).unwrap();
        assert_eq!(ax.limit_mode(), LimitMode::Bounded);
        assert_eq!(ax.position_limits_raw(), Some(100..=900));
        script(&mut bus, &[p1_status(1, 0, &[]), p1_status(1, 0, &[])]);
        let mut ax = bus.actuator(id(1)).unwrap();
        ax.enable_torque().unwrap();
        assert_eq!(
            ax.set_goal_position_raw(50),
            Err(ValidationError::OutOfLimits {
                id: id(1),
                raw: 50,
                min: 100,
                max: 900
            }
            .into())
        );
        assert_eq!(ax.set_goal_position_raw(500), Ok(()));
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[
                p1_write(1, 6, &[0, 0, 0, 0]),
                p1_write(1, 6, &[100, 0, 0x84, 0x03]),
                p1_write(1, 24, &[1]),
                p1_write(1, 30, &500u16.to_le_bytes()),
            ])
        );
        assert_done(&bus);
    }

    /// Plan D3: the getters read their registers, one read each, never
    /// cached; the voltage pair is one read of both.
    #[test]
    fn getters_read_their_registers() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 1023, false);
        let mark = mark(&bus);
        script(
            &mut bus,
            &[
                p1_status(1, 0, &[60, 140]),
                p1_status(1, 0, &[70]),
                p1_status(1, 0, &1023u16.to_le_bytes()),
                p1_status(1, 0, &200u16.to_le_bytes()),
            ],
        );
        let mut ax = bus.actuator(id(1)).unwrap();
        let (min, max) = ax.voltage_limits().unwrap();
        assert!((min.volts() - 6.0).abs() < 1e-5 && (max.volts() - 14.0).abs() < 1e-5);
        assert_eq!(ax.temperature_limit().unwrap().celsius(), 70.0);
        assert_eq!(ax.max_torque().unwrap().ratio(), 1.0);
        let cw = ax.read(MinPositionLimit).unwrap();
        assert_eq!(cw, ax.position_scale().to_position(200));
        assert_eq!(ax.position_limits_raw(), Some(0..=1023));
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[
                p1_read(1, 12, 2),
                p1_read(1, 11, 1),
                p1_read(1, 14, 2),
                p1_read(1, 6, 2),
            ])
        );
        assert_done(&bus);
    }

    /// Plan D2: limit fields are read fields, so a multi-actuator read
    /// saves them before a run (NFR-11).
    #[test]
    fn limits_read_on_many() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 1023, false);
        let mark = mark(&bus);
        line(&mut bus).reply(&p1_status(1, 0, &[70]));
        let mut out = [Temperature::default()];
        bus.read_field(TemperatureLimit, [id(1)], &mut out).unwrap();
        assert_eq!(out[0].celsius(), 70.0);
        assert_eq!(sent_since(&bus, mark), p1_read(1, 11, 1));
    }
}

#[cfg(feature = "x-series")]
mod xm430 {
    use super::on;
    use crate::actuator::{Actuator, Describe, LimitMode, ModelNumber};
    use crate::bus::testing::*;
    use crate::error::{Error, Instruction, TransportError, ValidationError};
    use crate::field::{
        GoalCurrent, GoalVelocity, MaxPositionLimit, MaxVoltageLimit, MinPositionLimit, Raw,
        TemperatureLimit,
    };
    use crate::protocol::P2;
    use crate::transport::MockTime;
    use crate::units::{Current, Position, Pwm, Temperature, Velocity, Voltage};

    /// An XM430-W350 at ID 1, limits 1000..=3000, in Operating Mode `mode`,
    /// as Setup leaves it: torque `Off`, not confirmed off.
    fn declared(time: &MockTime, mode: u8) -> TestBus<'_, P2> {
        let mut bus = test_bus(time);
        xm430_p2(line(&mut bus), 1, 1, 1000, 3000, mode);
        bus.declare([(id(1), ModelNumber(XM430))]).unwrap();
        bus
    }

    /// The same, torque confirmed off by an acknowledged `disable_torque()`,
    /// as a limit write needs (plan D8).
    fn xm430(time: &MockTime, mode: u8) -> TestBus<'_, P2> {
        let mut bus = declared(time, mode);
        disable(&mut bus);
        bus
    }

    fn enable(bus: &mut TestBus<'_, P2>) {
        ack(bus, 1);
        bus.actuator(id(1)).unwrap().enable_torque().unwrap();
    }

    fn disable(bus: &mut TestBus<'_, P2>) {
        ack(bus, 1);
        bus.actuator(id(1)).unwrap().disable_torque().unwrap();
    }

    /// Plan D8: the refusal of a limit write to `register` while torque is
    /// not confirmed off.
    fn unconfirmed(register: &'static str) -> Result<(), Error> {
        Err(ValidationError::TorqueUnconfirmed {
            id: id(1),
            register,
        }
        .into())
    }

    fn at(bus: &mut TestBus<'_, P2>, raw: i32) -> Position {
        bus.actuator(id(1))
            .unwrap()
            .position_scale()
            .to_position(raw)
    }

    /// Plan D3: every setter's packet. Max Voltage Limit(32) comes before
    /// Min Voltage Limit(34), and Max Position Limit(48) before Min(52):
    /// each pair is one Write from the lower address.
    #[test]
    fn each_setter_writes_its_register() {
        let time = MockTime::new();
        let mut bus = xm430(&time, 3);
        let (min, max) = (at(&mut bus, 500), at(&mut bus, 3500));
        let velocity = bus
            .actuator(id(1))
            .unwrap()
            .kind()
            .data()
            .role(crate::actuator::Role::VelocityLimit)
            .unwrap()
            .scale(LimitMode::Bounded)
            .unwrap()
            .velocity(100);
        let mark = mark(&bus);
        let mut sent = std::vec::Vec::new();
        for (call, packet) in [
            (0, p2_write(1, 32, &[160, 0, 100, 0])),
            (1, p2_write(1, 31, &[75])),
            (2, p2_write(1, 36, &442u16.to_le_bytes())),
            (3, p2_write(1, 38, &372u16.to_le_bytes())),
            (4, p2_write(1, 44, &100u32.to_le_bytes())),
            (
                5,
                p2_write(
                    1,
                    48,
                    &[3500i32.to_le_bytes(), 500i32.to_le_bytes()].concat(),
                ),
            ),
        ] {
            ack(&mut bus, 1);
            let mut xm = bus.actuator(id(1)).unwrap();
            let result = match call {
                0 => xm.set_voltage_limits(Voltage::from_volts(10.0), Voltage::from_volts(16.0)),
                1 => xm.set_temperature_limit(Temperature::from_celsius(75.0)),
                2 => xm.set_pwm_limit(Pwm::from_ratio(0.5)),
                3 => xm.set_current_limit(Current::from_amps(1.0)),
                4 => xm.set_velocity_limit(velocity),
                _ => xm.set_position_limits(min, max),
            };
            assert_eq!(result, Ok(()), "call {call}");
            sent.extend(packet);
        }
        assert_eq!(sent_since(&bus, mark), sent);
        assert_eq!(
            bus.actuator(id(1)).unwrap().position_limits_raw(),
            Some(500..=3500)
        );
        assert_done(&bus);
    }

    /// Plan D4: torque on, out of range, inverted, not finite: nothing is
    /// sent.
    #[test]
    fn refusals_send_nothing() {
        let time = MockTime::new();
        let mut bus = xm430(&time, 3);
        enable(&mut bus);
        let before = mark(&bus);
        assert_eq!(
            bus.actuator(id(1))
                .unwrap()
                .set_pwm_limit(Pwm::from_ratio(0.5)),
            Err(ValidationError::ActuatorIsOn {
                id: id(1),
                register: "PWM Limit"
            }
            .into())
        );
        assert_quiet(&bus, before);
        ack(&mut bus, 1);
        bus.actuator(id(1)).unwrap().disable_torque().unwrap();
        let quiet = mark(&bus);
        let mut xm = bus.actuator(id(1)).unwrap();
        let v = Voltage::from_volts;
        let out = |raw, min, max| {
            Err(ValidationError::OutOfLimits {
                id: id(1),
                raw,
                min,
                max,
            }
            .into())
        };
        assert_eq!(xm.set_voltage_limits(v(9.0), v(16.0)), out(90, 95, 160));
        assert_eq!(xm.set_pwm_limit(Pwm::from_ratio(1.01)), out(894, 0, 885));
        assert_eq!(
            xm.set_current_limit(Current::from_amps(3.3)),
            out(1227, 0, 1193)
        );
        assert_eq!(
            xm.set_voltage_limits(v(16.0), v(10.0)),
            Err(ValidationError::InvertedLimits {
                id: id(1),
                min: 160,
                max: 100
            }
            .into())
        );
        assert_eq!(
            xm.set_velocity_limit(Velocity::from_rad_per_s(f32::INFINITY)),
            Err(ValidationError::NotFinite {
                request: on(Instruction::Write, 1, "Velocity Limit", 44, 4),
            }
            .into())
        );
        assert_eq!(
            xm.write_limit(Raw(MaxPositionLimit), 999),
            Err(ValidationError::InvertedLimits {
                id: id(1),
                min: 1000,
                max: 999
            }
            .into())
        );
        assert_eq!(
            xm.set_velocity_limit(Velocity::from_rad_per_s(1e8)),
            Err(ValidationError::DoesNotFit {
                raw: 4_169_998_592,
                size: 4,
                request: on(Instruction::Write, 1, "Velocity Limit", 44, 4),
            }
            .into())
        );
        assert_quiet(&bus, quiet);
    }

    /// NFR-12: the errors of the getters and of a raw limit that fits no
    /// register, with their requests.
    #[test]
    fn getter_and_encode_errors_carry_their_request() {
        let time = MockTime::new();
        let mut bus = xm430(&time, 3);
        assert_eq!(
            bus.actuator(id(1)).unwrap().voltage_limits(),
            Err(TransportError::NoReply {
                request: on(Instruction::Read, 1, "Max Voltage Limit", 32, 4),
                timeout_us: 10_000,
            }
            .into())
        );

        let mut bus = test_bus::<P2, 32>(&time);
        hold(&mut bus, 1030, 1, (0, 4095), LimitMode::Bounded, OFF);
        disable(&mut bus);
        let quiet = mark(&bus);
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(
            xm.voltage_limits(),
            Err(ValidationError::NoUnit {
                id: id(1),
                register: "Min Voltage Limit",
                mode: LimitMode::Bounded,
            }
            .into())
        );
        assert_eq!(
            xm.write_limit(Raw(TemperatureLimit), 300),
            Err(ValidationError::DoesNotFit {
                raw: 300,
                size: 1,
                request: on(Instruction::Write, 1, "Temperature Limit", 31, 1),
            }
            .into())
        );
        assert_quiet(&bus, quiet);
    }

    /// Plan D6: a model whose table has no unit for the register (an
    /// unverified XM430-W210) refuses the typed form; the raw form works.
    /// A typed position limit needs the e-Manual range a verified table
    /// records, though `Position` converts through `type_info`.
    #[test]
    fn no_unit_refuses_the_typed_form_only() {
        let time = MockTime::new();
        let mut bus = test_bus::<P2, 32>(&time);
        hold(&mut bus, 1030, 1, (0, 4095), LimitMode::Bounded, OFF);
        disable(&mut bus);
        let start = mark(&bus);
        let mut xm = bus.actuator(id(1)).unwrap();
        let scale = xm.position_scale();
        let no_unit = |register| {
            Err(ValidationError::NoUnit {
                id: id(1),
                register,
                mode: LimitMode::Bounded,
            }
            .into())
        };
        assert_eq!(
            xm.set_position_limits(Position::from_radians(-4.0), Position::from_radians(4.0)),
            no_unit("Min Position Limit")
        );
        assert_eq!(
            xm.write_limit(MinPositionLimit, scale.to_position(100)),
            no_unit("Min Position Limit")
        );
        assert_eq!(
            xm.write_limit(MaxPositionLimit, scale.to_position(4000)),
            no_unit("Max Position Limit")
        );
        assert_eq!(
            xm.set_voltage_limits(Voltage::from_volts(10.0), Voltage::from_volts(16.0)),
            Err(ValidationError::NoUnit {
                id: id(1),
                register: "Min Voltage Limit",
                mode: LimitMode::Bounded,
            }
            .into())
        );
        assert_eq!(
            xm.set_temperature_limit(Temperature::from_celsius(70.0)),
            Err(ValidationError::NoUnit {
                id: id(1),
                register: "Temperature Limit",
                mode: LimitMode::Bounded,
            }
            .into())
        );
        assert_quiet(&bus, start);
        script(&mut bus, &[p2_status(1, 0, &[]), p2_status(1, 0, &[])]);
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(xm.write_limit(Raw(MaxVoltageLimit), 160), Ok(()));
        assert_eq!(xm.write_limit(Raw(MinPositionLimit), 0), Ok(()));
        assert_eq!(
            sent_since(&bus, start),
            concat(&[
                p2_write(1, 32, &160u16.to_le_bytes()),
                p2_write(1, 52, &0i32.to_le_bytes()),
            ])
        );
    }

    /// Plan D5: a position-limit write moves the goal check.
    #[test]
    fn position_limits_move_the_goal_check() {
        let time = MockTime::new();
        let mut bus = xm430(&time, 3);
        let max = at(&mut bus, 3800);
        ack(&mut bus, 1);
        bus.actuator(id(1))
            .unwrap()
            .write_limit(MaxPositionLimit, max)
            .unwrap();
        enable(&mut bus);
        let mark = mark(&bus);
        ack(&mut bus, 1);
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(xm.position_limits_raw(), Some(1000..=3800));
        assert_eq!(xm.set_goal_position_raw(3700), Ok(()));
        assert_eq!(
            xm.set_goal_position_raw(3801),
            Err(ValidationError::OutOfLimits {
                id: id(1),
                raw: 3801,
                min: 1000,
                max: 3800
            }
            .into())
        );
        assert_eq!(
            sent_since(&bus, mark),
            p2_write(1, 116, &3700i32.to_le_bytes())
        );
    }

    /// Plan D5: in velocity mode a Velocity Limit write replaces the goal
    /// bound; in current mode a Current Limit write does; in position mode
    /// neither is a bound.
    #[test]
    fn bound_limits_replace_the_goal_bound() {
        let time = MockTime::new();
        let mut bus = xm430(&time, 1);
        assert_eq!(
            bus.actuator(id(1)).unwrap().goal_bound_raw(),
            Some(XM430_VELOCITY_LIMIT)
        );
        ack(&mut bus, 1);
        bus.actuator(id(1))
            .unwrap()
            .write_limit(Raw(crate::field::VelocityLimit), 100)
            .unwrap();
        assert_eq!(bus.actuator(id(1)).unwrap().goal_bound_raw(), Some(100));
        enable(&mut bus);
        let mark = mark(&bus);
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(
            xm.write(Raw(GoalVelocity), -101),
            Err(ValidationError::OutOfLimits {
                id: id(1),
                raw: -101,
                min: -100,
                max: 100
            }
            .into())
        );
        assert!(sent_since(&bus, mark).is_empty());

        let mut bus = xm430(&time, 0);
        ack(&mut bus, 1);
        bus.actuator(id(1))
            .unwrap()
            .write_limit(Raw(crate::field::CurrentLimit), 500)
            .unwrap();
        assert_eq!(bus.actuator(id(1)).unwrap().goal_bound_raw(), Some(500));
        enable(&mut bus);
        assert_eq!(
            bus.actuator(id(1)).unwrap().write(Raw(GoalCurrent), 501),
            Err(ValidationError::OutOfLimits {
                id: id(1),
                raw: 501,
                min: -500,
                max: 500
            }
            .into())
        );

        let mut bus = xm430(&time, 3);
        ack(&mut bus, 1);
        bus.actuator(id(1))
            .unwrap()
            .write_limit(Raw(crate::field::VelocityLimit), 100)
            .unwrap();
        assert_eq!(bus.actuator(id(1)).unwrap().goal_bound_raw(), None);
    }

    /// Plan D5: a Velocity Limit write that times out in velocity mode
    /// leaves the goal bound unknown: `goal_bound_raw` is `None` and goal
    /// velocities are refused, until an acknowledged Velocity Limit write
    /// sets the bound again.
    #[test]
    fn failed_bound_write_refuses_bounded_goals() {
        let time = MockTime::new();
        let mut bus = xm430(&time, 1);
        assert_eq!(
            bus.actuator(id(1))
                .unwrap()
                .write_limit(Raw(crate::field::VelocityLimit), 100),
            Err(TransportError::NoReply {
                request: on(Instruction::Write, 1, "Velocity Limit", 44, 4),
                timeout_us: 10_000,
            }
            .into())
        );
        assert_eq!(bus.actuator(id(1)).unwrap().goal_bound_raw(), None);
        enable(&mut bus);
        let quiet = mark(&bus);
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(
            xm.set_goal_velocity(Velocity::ZERO),
            Err(ValidationError::LimitsUnknown { id: id(1) }.into())
        );
        assert_quiet(&bus, quiet);

        ack(&mut bus, 1);
        bus.actuator(id(1)).unwrap().disable_torque().unwrap();
        ack(&mut bus, 1);
        bus.actuator(id(1))
            .unwrap()
            .write_limit(Raw(crate::field::VelocityLimit), 150)
            .unwrap();
        assert_eq!(bus.actuator(id(1)).unwrap().goal_bound_raw(), Some(150));
        enable(&mut bus);
        let mark = mark(&bus);
        ack(&mut bus, 1);
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(xm.write(Raw(GoalVelocity), -150), Ok(()));
        assert_eq!(
            xm.write(Raw(GoalVelocity), 151),
            Err(ValidationError::OutOfLimits {
                id: id(1),
                raw: 151,
                min: -150,
                max: 150
            }
            .into())
        );
        assert_eq!(
            sent_since(&bus, mark),
            p2_write(1, 104, &(-150i32).to_le_bytes())
        );
    }

    /// Plan D5: after a failed position-limit write, an acknowledged write
    /// of both limits determines them again.
    #[test]
    fn pair_write_after_a_failed_one_restores_the_limits() {
        let time = MockTime::new();
        let mut bus = xm430(&time, 3);
        let (min, max) = (at(&mut bus, 500), at(&mut bus, 3500));
        assert!(
            bus.actuator(id(1))
                .unwrap()
                .write_limit(MaxPositionLimit, max)
                .is_err()
        );
        assert_eq!(bus.actuator(id(1)).unwrap().position_limits_raw(), None);
        ack(&mut bus, 1);
        let mut xm = bus.actuator(id(1)).unwrap();
        xm.set_position_limits(min, max).unwrap();
        assert_eq!(xm.position_limits_raw(), Some(500..=3500));
        assert_eq!(xm.limit_mode(), LimitMode::Bounded);
        enable(&mut bus);
        ack(&mut bus, 1);
        assert_eq!(
            bus.actuator(id(1)).unwrap().set_goal_position_raw(3400),
            Ok(())
        );
    }

    /// Plan D5: a position-limit write that times out leaves the limits
    /// unknown: bounded goals are refused, extended-mode goals still pass.
    #[test]
    fn failed_position_write_refuses_bounded_goals() {
        let time = MockTime::new();
        for (mode, goal) in [
            (3, Err(ValidationError::LimitsUnknown { id: id(1) }.into())),
            (4, Ok(())),
        ] {
            let mut bus = xm430(&time, mode);
            let max = at(&mut bus, 3500);
            assert!(matches!(
                bus.actuator(id(1))
                    .unwrap()
                    .write_limit(MaxPositionLimit, max),
                Err(Error::Transport(_))
            ));
            assert_eq!(bus.actuator(id(1)).unwrap().position_limits_raw(), None);
            enable(&mut bus);
            if goal.is_ok() {
                ack(&mut bus, 1);
            }
            assert_eq!(
                bus.actuator(id(1)).unwrap().set_goal_position_raw(2000),
                goal,
                "mode {mode}"
            );
        }
    }

    /// Plan D3: the getters' reads and values.
    #[test]
    fn getters_read_their_registers() {
        let time = MockTime::new();
        let mut bus = xm430(&time, 3);
        let mark = mark(&bus);
        script(
            &mut bus,
            &[
                p2_status(1, 0, &[160, 0, 100, 0]),
                p2_status(1, 0, &[80]),
                p2_status(1, 0, &200u32.to_le_bytes()),
                p2_status(1, 0, &1000u16.to_le_bytes()),
                p2_status(1, 0, &885u16.to_le_bytes()),
            ],
        );
        let mut xm = bus.actuator(id(1)).unwrap();
        let (min, max) = xm.voltage_limits().unwrap();
        assert!((min.volts() - 10.0).abs() < 1e-5, "{min:?}");
        assert!((max.volts() - 16.0).abs() < 1e-5, "{max:?}");
        assert_eq!(xm.temperature_limit().unwrap().celsius(), 80.0);
        let scale = |role| {
            xm.kind()
                .data()
                .role(role)
                .unwrap()
                .scale(LimitMode::Bounded)
                .unwrap()
        };
        let velocity = scale(crate::actuator::Role::VelocityLimit).velocity(200);
        let current = scale(crate::actuator::Role::CurrentLimit).current(1000);
        assert_eq!(xm.velocity_limit(), Ok(velocity));
        assert_eq!(xm.current_limit(), Ok(current));
        assert!((xm.pwm_limit().unwrap().ratio() - 1.000_05).abs() < 1e-4);
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[
                p2_read(1, 32, 4),
                p2_read(1, 31, 1),
                p2_read(1, 44, 4),
                p2_read(1, 38, 2),
                p2_read(1, 36, 2),
            ])
        );
        assert_done(&bus);
    }

    /// Plan D8: after Setup a limit write is refused, with nothing sent,
    /// until an acknowledged `disable_torque()`; X-series firmware also
    /// writes EEPROM only with Torque Enable 0.
    #[test]
    fn setup_leaves_torque_unconfirmed() {
        let time = MockTime::new();
        let mut bus = declared(&time, 3);
        let start = mark(&bus);
        assert_eq!(
            bus.actuator(id(1))
                .unwrap()
                .set_pwm_limit(Pwm::from_ratio(0.5)),
            unconfirmed("PWM Limit")
        );
        assert_quiet(&bus, start);
        disable(&mut bus);
        ack(&mut bus, 1);
        assert_eq!(
            bus.actuator(id(1))
                .unwrap()
                .set_pwm_limit(Pwm::from_ratio(0.5)),
            Ok(())
        );
        assert_eq!(
            sent_since(&bus, start),
            concat(&[
                p2_write(1, 64, &[0]),
                p2_write(1, 36, &442u16.to_le_bytes())
            ])
        );
    }

    /// Plan D8: a reboot may come back with torque on (Startup
    /// Configuration bit 0), and an `enable_torque` that sent bytes may
    /// have turned it on even when its reply was lost: both clear the mark.
    #[test]
    fn reboot_and_enable_attempt_clear_the_mark() {
        let time = MockTime::new();
        let mut bus = xm430(&time, 3);
        ack(&mut bus, 1);
        bus.actuator(id(1)).unwrap().reboot().unwrap();
        let quiet = mark(&bus);
        assert_eq!(
            bus.actuator(id(1))
                .unwrap()
                .set_temperature_limit(Temperature::from_celsius(75.0)),
            unconfirmed("Temperature Limit")
        );
        assert_quiet(&bus, quiet);

        let mut bus = xm430(&time, 3);
        assert!(bus.actuator(id(1)).unwrap().enable_torque().is_err());
        assert_eq!(bus.torque_state(id(1)), Some(crate::TorqueState::Off));
        let quiet = mark(&bus);
        assert_eq!(
            bus.actuator(id(1))
                .unwrap()
                .set_temperature_limit(Temperature::from_celsius(75.0)),
            unconfirmed("Temperature Limit")
        );
        assert_quiet(&bus, quiet);
    }

    /// Plan D8, Protocol 2: a reply to Torque Enable = 0 with the alert bit
    /// and no error code shows the write ran: `disable_torque()` returns the
    /// measured fault, yet confirms torque off. A reply with an error code
    /// (Access Error) does not, and clears a mark set before.
    #[test]
    fn p2_replies_that_confirm_and_that_do_not() {
        let time = MockTime::new();
        let mut bus = declared(&time, 3);
        let start = mark(&bus);
        script(
            &mut bus,
            &[
                p2_status(1, 0x80, &[]),
                p2_status(1, 0x80, &[0x20]),
                p2_status(1, 0, &[]),
            ],
        );
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(
            xm.disable_torque(),
            Err(crate::error::FaultError::Overload {
                id: id(1),
                faults: crate::units::HardwareErrors::from_raw(0x20),
                request: on(Instruction::Write, 1, "Torque Enable", 64, 1),
            }
            .into())
        );
        assert_eq!(xm.torque_state(), crate::TorqueState::Faulted);
        assert_eq!(
            xm.set_temperature_limit(Temperature::from_celsius(75.0)),
            Ok(())
        );
        assert_eq!(
            sent_since(&bus, start),
            concat(&[
                p2_write(1, 64, &[0]),
                p2_read(1, 70, 1),
                p2_write(1, 31, &[75]),
            ])
        );

        let mut bus = xm430(&time, 3);
        line(&mut bus).reply(&p2_status(1, 0x07, &[]));
        assert!(matches!(
            bus.actuator(id(1)).unwrap().disable_torque(),
            Err(Error::Status(_))
        ));
        let quiet = mark(&bus);
        assert_eq!(
            bus.actuator(id(1))
                .unwrap()
                .set_temperature_limit(Temperature::from_celsius(75.0)),
            unconfirmed("Temperature Limit")
        );
        assert_quiet(&bus, quiet);
    }

    /// Plan D8 (`raw-registers`): a raw write, unicast or a raw Bulk Write
    /// part, may touch Torque Enable, so it clears the mark of the actuator
    /// it reaches, and only of that one.
    #[cfg(feature = "raw-registers")]
    #[test]
    fn raw_writes_clear_the_mark() {
        use crate::bulk::RawWrite;

        let time = MockTime::new();
        let mut bus = xm430(&time, 3);
        ack(&mut bus, 1);
        assert_eq!(bus.actuator(id(1)).unwrap().write_raw(65, &[1]), Ok(()));
        let quiet = mark(&bus);
        assert_eq!(
            bus.actuator(id(1))
                .unwrap()
                .set_pwm_limit(Pwm::from_ratio(0.5)),
            unconfirmed("PWM Limit")
        );
        assert_quiet(&bus, quiet);

        let mut bus = test_bus::<P2, 32>(&time);
        let next = xm430_p2(line(&mut bus), 1, 1, 1000, 3000, 3);
        let next = xm430_p2(line(&mut bus), next, 2, 1000, 3000, 3);
        line(&mut bus)
            .reply_to(next, &p2_status(1, 0, &[]))
            .reply_to(next + 1, &p2_status(2, 0, &[]))
            .reply_to(next + 3, &p2_status(2, 0, &[]));
        bus.declare([(id(1), ModelNumber(XM430)), (id(2), ModelNumber(XM430))])
            .unwrap();
        bus.disable_torque([id(1), id(2)]).unwrap();
        let start = mark(&bus);
        assert_eq!(
            bus.bulk_write_raw(&[RawWrite::new(id(1), 65, &[1])]),
            Ok(())
        );
        let pwm = Pwm::from_ratio(0.5);
        assert_eq!(
            bus.actuator(id(1)).unwrap().set_pwm_limit(pwm),
            unconfirmed("PWM Limit")
        );
        assert_eq!(bus.actuator(id(2)).unwrap().set_pwm_limit(pwm), Ok(()));
        assert_eq!(
            sent_since(&bus, start),
            concat(&[
                p2_bulk_write(&[(1, 65, &[1])]),
                p2_write(2, 36, &442u16.to_le_bytes()),
            ])
        );
    }

    /// Plan D9: the wheel check is for AX, RX and MX on Protocol 1 only;
    /// an X-series takes both position limits at 0 without a Moving Speed
    /// write (its mode is Operating Mode).
    #[test]
    fn wheel_check_is_protocol_1_only() {
        let time = MockTime::new();
        let mut bus = xm430(&time, 3);
        let zero = at(&mut bus, 0);
        let start = mark(&bus);
        ack(&mut bus, 1);
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(xm.set_position_limits(zero, zero), Ok(()));
        assert_eq!(xm.limit_mode(), LimitMode::Bounded);
        assert_eq!(sent_since(&bus, start), p2_write(1, 48, &[0; 8]));
    }

    /// Plan D3: `DynBus` lends the same handle, with the same setters.
    #[cfg(feature = "std")]
    #[test]
    fn dyn_bus_handle_sets_limits() {
        use crate::{Config, DynBus};
        let time = MockTime::new();
        let mut line = crate::transport::MockTransport::new(&time);
        let next = xm430_p2(&mut line, 1, 1, 1000, 3000, 3);
        line.reply_to(next + 1, &p2_status(1, 0, &[]))
            .reply_to(next + 2, &p2_status(1, 0, &[]));
        let mut bus = DynBus::<P2, _, _>::new(line, time.ticker(10), Config::new());
        bus.declare([(id(1), ModelNumber(XM430))]).unwrap();
        let mut xm = bus.actuator(id(1)).unwrap();
        let volts = (Voltage::from_volts(10.0), Voltage::from_volts(16.0));
        assert_eq!(
            xm.set_voltage_limits(volts.0, volts.1),
            unconfirmed("Min Voltage Limit")
        );
        assert!(xm.enable_torque().is_err());
        assert_eq!(
            xm.set_voltage_limits(volts.0, volts.1),
            unconfirmed("Min Voltage Limit")
        );
        xm.disable_torque().unwrap();
        assert_eq!(xm.set_voltage_limits(volts.0, volts.1), Ok(()));
        let (line, _) = bus.into_parts();
        assert!(line.sent().ends_with(&concat(&[
            p2_write(1, 64, &[1]),
            p2_write(1, 64, &[0]),
            p2_write(1, 32, &[160, 0, 100, 0]),
        ])));
        line.assert_done();
    }
}

#[cfg(feature = "mx")]
mod mx64_p1 {
    use crate::actuator::{Actuator, LimitMode};
    use crate::bus::testing::*;
    use crate::field::{Raw, SpeedLimit};
    use crate::protocol::P1;
    use crate::transport::MockTime;

    /// Plan D5, MX-64 (Protocol 1.0) CW/CCW Angle Limit(6, 8): "Multi-turn
    /// Mode | Condition that Both CW and CCW are 4095"
    /// (<https://emanual.robotis.com/docs/en/dxl/mx/mx-64/>). Both limits at
    /// 4095 switch the bus to multi-turn mode, where goals are checked only
    /// against the register size.
    #[test]
    fn angle_limits_at_4095_switch_multi_turn() {
        let time = MockTime::new();
        let mut bus = test_bus::<P1, 32>(&time);
        hold(&mut bus, MX64_P1, 1, (0, 4095), BOUNDED, OFF);
        let top = bus
            .actuator(id(1))
            .unwrap()
            .position_scale()
            .to_position(4095);
        script(
            &mut bus,
            &[
                p1_status(1, 0, &[]),
                p1_status(1, 0, &[]),
                p1_status(1, 0, &[]),
                p1_status(1, 0, &[]),
            ],
        );
        let mut mx = bus.actuator(id(1)).unwrap();
        mx.disable_torque().unwrap();
        mx.set_position_limits(top, top).unwrap();
        assert_eq!(mx.limit_mode(), LimitMode::Unbounded);
        assert_eq!(mx.position_limits_raw(), Some(4095..=4095));
        mx.enable_torque().unwrap();
        assert_eq!(mx.set_goal_position_raw(20_000), Ok(()));
        assert_eq!(
            sent(&bus),
            concat(&[
                p1_write(1, 24, &[0]),
                p1_write(1, 6, &[0xFF, 0x0F, 0xFF, 0x0F]),
                p1_write(1, 24, &[1]),
                p1_write(1, 30, &20_000u16.to_le_bytes()),
            ])
        );
    }

    /// Plan D9: on MX-64 Protocol 1 with Torque Ctrl Mode Enable on, CW =
    /// CCW = 0 still counts as a switch to wheel mode, which takes over
    /// once the switch is off: refused until Moving Speed(32) is 0. The
    /// typed speed limit has no unit in that mode, so the raw form writes
    /// the 0, as the docs of `write_limit` say.
    #[test]
    fn torque_control_mode_still_guards_the_wheel_switch() {
        let time = MockTime::new();
        let mut bus = test_bus::<P1, 32>(&time);
        hold(&mut bus, MX64_P1, 1, (0, 4095), LimitMode::Current, OFF);
        let zero = bus.actuator(id(1)).unwrap().position_scale().to_position(0);
        ack(&mut bus, 1);
        bus.actuator(id(1)).unwrap().disable_torque().unwrap();
        let start = mark(&bus);
        let mut mx = bus.actuator(id(1)).unwrap();
        assert_eq!(
            mx.set_position_limits(zero, zero),
            Err(crate::ValidationError::WheelSpeedUnknown { id: id(1) }.into())
        );
        assert!(matches!(
            mx.set_speed_limit(crate::units::SpeedLimit::Unlimited),
            Err(crate::Error::Validation(
                crate::ValidationError::NoUnit { .. }
            ))
        ));
        assert_quiet(&bus, start);
        script(&mut bus, &[p1_status(1, 0, &[]), p1_status(1, 0, &[])]);
        let mut mx = bus.actuator(id(1)).unwrap();
        assert_eq!(mx.write(Raw(SpeedLimit), 0), Ok(()));
        assert_eq!(mx.set_position_limits(zero, zero), Ok(()));
        assert_eq!(mx.limit_mode(), LimitMode::Current);
        assert_eq!(
            sent_since(&bus, start),
            concat(&[p1_write(1, 32, &[0, 0]), p1_write(1, 6, &[0, 0, 0, 0])])
        );
    }
}

#[cfg(feature = "mx")]
mod mx64 {
    use super::on;
    use crate::actuator::{Actuator, LimitMode, ModelNumber};
    use crate::bus::testing::*;
    use crate::error::{Error, Instruction, TransportError, ValidationError};
    use crate::field::{AccelerationLimit, GoalVelocity, Raw};
    use crate::protocol::P2;
    use crate::transport::MockTime;
    use crate::units::Acceleration;

    /// MX-28(2.0) model number: its table, unverified, has Acceleration
    /// Limit(40) and no unit for it.
    const MX28: u16 = 30;

    /// One raw step of MX-64(2.0) Acceleration Limit(40), "214.577
    /// Rev/min²", in rad/s².
    const STEP: f32 = 214.577 * core::f32::consts::TAU / 3600.0;

    /// Disables torque on actuator 1 with an acknowledged write, as
    /// [`write_limit`](crate::ActuatorRef::write_limit) asks before EEPROM
    /// writes where torque may be on.
    fn disable(bus: &mut TestBus<'_, P2>) {
        ack(bus, 1);
        bus.actuator(id(1)).unwrap().disable_torque().unwrap();
    }

    /// An MX-64(2.0) at ID 1, limits 1000..=3000, in Operating Mode `mode`,
    /// torque disabled.
    fn mx64(time: &MockTime, mode: u8) -> TestBus<'_, P2> {
        let mut bus = test_bus(time);
        mx64_p2(line(&mut bus), 1, 1, 1000, 3000, mode);
        bus.declare([(id(1), ModelNumber(MX64))]).unwrap();
        disable(&mut bus);
        bus
    }

    /// Plan D10: the setter writes Acceleration Limit(40), 4 bytes, in the
    /// table's step of 214.577 rev/min²; the getter reads it back; the bus
    /// holds no copy, so nothing it caches changes.
    #[test]
    fn acceleration_limit_is_written_and_read() {
        let time = MockTime::new();
        let mut bus = mx64(&time, 3);
        let mark = mark(&bus);
        script(
            &mut bus,
            &[
                p2_status(1, 0, &[]),
                p2_status(1, 0, &[]),
                p2_status(1, 0, &[]),
                p2_status(1, 0, &100u32.to_le_bytes()),
            ],
        );
        let mut mx = bus.actuator(id(1)).unwrap();
        let before = mx.kind().clone();
        assert_eq!(
            mx.set_acceleration_limit(Acceleration::from_rad_per_s2(37.45)),
            Ok(())
        );
        assert_eq!(
            mx.set_acceleration_limit(Acceleration::from_rad_per_s2(32_767.0 * STEP)),
            Ok(())
        );
        assert_eq!(mx.write_limit(Raw(AccelerationLimit), 0), Ok(()));
        let read = mx.acceleration_limit().unwrap();
        assert!((read.rad_per_s2() - 100.0 * STEP).abs() < 1e-4, "{read:?}");
        assert!((read.rad_per_s2() - 37.450_75).abs() < 1e-4, "{read:?}");
        assert_eq!(*mx.kind(), before);
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[
                p2_write(1, 40, &100u32.to_le_bytes()),
                p2_write(1, 40, &32_767u32.to_le_bytes()),
                p2_write(1, 40, &0u32.to_le_bytes()),
                p2_read(1, 40, 4),
            ])
        );
        assert_done(&bus);
    }

    /// Plan D4: outside the e-Manual's 0 ~ 32,767, negative, not finite,
    /// torque on: refused, and nothing is sent.
    #[test]
    fn acceleration_limit_refusals_send_nothing() {
        let time = MockTime::new();
        let mut bus = mx64(&time, 3);
        let quiet = mark(&bus);
        let mut mx = bus.actuator(id(1)).unwrap();
        let request = on(Instruction::Write, 1, "Acceleration Limit", 40, 4);
        let out = |raw| {
            Err(ValidationError::OutOfLimits {
                id: id(1),
                raw,
                min: 0,
                max: 32_767,
            }
            .into())
        };
        assert_eq!(
            mx.set_acceleration_limit(Acceleration::from_rad_per_s2(32_768.0 * STEP)),
            out(32_768)
        );
        assert_eq!(mx.write_limit(Raw(AccelerationLimit), 40_000), out(40_000));
        assert_eq!(
            mx.set_acceleration_limit(Acceleration::from_rad_per_s2(-1.0)),
            Err(ValidationError::DoesNotFit {
                raw: -3,
                size: 4,
                request,
            }
            .into())
        );
        assert_eq!(
            mx.set_acceleration_limit(Acceleration::from_rad_per_s2(f32::NAN)),
            Err(ValidationError::NotFinite { request }.into())
        );
        assert_quiet(&bus, quiet);

        ack(&mut bus, 1);
        bus.actuator(id(1)).unwrap().enable_torque().unwrap();
        let enabled = mark(&bus);
        let mut mx = bus.actuator(id(1)).unwrap();
        let is_on = || -> Result<(), Error> {
            Err(ValidationError::ActuatorIsOn {
                id: id(1),
                register: "Acceleration Limit",
            }
            .into())
        };
        assert_eq!(mx.set_acceleration_limit(Acceleration::ZERO), is_on());
        assert_eq!(mx.write_limit(Raw(AccelerationLimit), 100), is_on());
        assert_quiet(&bus, enabled);
    }

    /// Plan D4, D10: the value is rounded to the nearest raw step before
    /// it is checked, as every typed value is, so a negative acceleration
    /// within half a step is written as raw 0, and one below it does not
    /// fit the unsigned register.
    #[test]
    fn small_negative_acceleration_rounds_to_zero() {
        let time = MockTime::new();
        let mut bus = mx64(&time, 3);
        let mark = mark(&bus);
        ack(&mut bus, 1);
        let mut mx = bus.actuator(id(1)).unwrap();
        assert_eq!(
            mx.set_acceleration_limit(Acceleration::from_rad_per_s2(-0.5 * STEP + 0.01)),
            Ok(())
        );
        assert_eq!(
            mx.set_acceleration_limit(Acceleration::from_rad_per_s2(-0.5 * STEP - 0.01)),
            Err(ValidationError::DoesNotFit {
                raw: -1,
                size: 4,
                request: on(Instruction::Write, 1, "Acceleration Limit", 40, 4),
            }
            .into())
        );
        assert_eq!(sent_since(&bus, mark), p2_write(1, 40, &0u32.to_le_bytes()));
        assert_done(&bus);
    }

    /// Plan D5, D10: Acceleration Limit is not cached, so a write of it
    /// that fails leaves the position limits, the mode and the goal bound
    /// known, and goals pass as before.
    #[test]
    fn failed_acceleration_write_keeps_the_cache() {
        let time = MockTime::new();
        let mut bus = mx64(&time, 1);
        let mut mx = bus.actuator(id(1)).unwrap();
        assert_eq!(
            mx.set_acceleration_limit(Acceleration::from_rad_per_s2(37.45)),
            Err(TransportError::NoReply {
                request: on(Instruction::Write, 1, "Acceleration Limit", 40, 4),
                timeout_us: 10_000,
            }
            .into())
        );
        assert_eq!(mx.goal_bound_raw(), Some(XM430_VELOCITY_LIMIT));
        assert_eq!(mx.position_limits_raw(), Some(1000..=3000));
        assert_eq!(mx.limit_mode(), LimitMode::Velocity);
        script(&mut bus, &[p2_status(1, 0, &[]), p2_status(1, 0, &[])]);
        let mark = mark(&bus);
        let mut mx = bus.actuator(id(1)).unwrap();
        mx.enable_torque().unwrap();
        assert_eq!(mx.write(Raw(GoalVelocity), 200), Ok(()));
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[
                p2_write(1, 64, &[1]),
                p2_write(1, 104, &200i32.to_le_bytes()),
            ])
        );
    }

    /// Plan D6: MX-28(2.0)'s table gives Acceleration Limit no unit, so the
    /// typed setter and getter are refused; the raw forms work.
    #[test]
    fn no_unit_refuses_the_typed_form_only() {
        let time = MockTime::new();
        let mut bus = test_bus::<P2, 32>(&time);
        hold(&mut bus, MX28, 1, (0, 4095), LimitMode::Bounded, OFF);
        disable(&mut bus);
        let quiet = mark(&bus);
        let mut mx = bus.actuator(id(1)).unwrap();
        let no_unit = || -> Result<(), Error> {
            Err(ValidationError::NoUnit {
                id: id(1),
                register: "Acceleration Limit",
                mode: LimitMode::Bounded,
            }
            .into())
        };
        assert_eq!(
            mx.set_acceleration_limit(Acceleration::from_rad_per_s2(37.45)),
            no_unit()
        );
        assert_eq!(
            mx.acceleration_limit(),
            no_unit().map(|()| Acceleration::ZERO)
        );
        assert_quiet(&bus, quiet);
        script(
            &mut bus,
            &[p2_status(1, 0, &[]), p2_status(1, 0, &100u32.to_le_bytes())],
        );
        let mut mx = bus.actuator(id(1)).unwrap();
        assert_eq!(mx.write_limit(Raw(AccelerationLimit), 100), Ok(()));
        assert_eq!(mx.read(Raw(AccelerationLimit)), Ok(100));
        assert_eq!(
            sent_since(&bus, quiet),
            concat(&[p2_write(1, 40, &100u32.to_le_bytes()), p2_read(1, 40, 4),])
        );
    }
}

#[cfg(feature = "y-series")]
mod ym070 {
    use crate::actuator::{Actuator, LimitMode};
    use crate::bus::field::{limit_site, sealed::Site};
    use crate::bus::limits::{Checked, ordered};
    use crate::bus::testing::*;
    use crate::field::{LimitField, MaxPositionLimit, MinPositionLimit, Raw};
    use crate::protocol::P2;
    use crate::transport::MockTime;

    /// YM070-210-M001 model number.
    const YM070: u16 = 4000;

    /// A YM070-210-M001 at ID 1 held with limits -1000..=1000, torque
    /// confirmed off by an acknowledged `disable_torque()` (plan D8).
    fn ym070(time: &MockTime) -> TestBus<'_, P2> {
        let mut bus = test_bus::<P2, 32>(time);
        hold(&mut bus, YM070, 1, (-1000, 1000), LimitMode::Bounded, OFF);
        ack(&mut bus, 1);
        bus.actuator(id(1)).unwrap().disable_torque().unwrap();
        bus
    }

    /// Plan D6: the YM070 table is unverified, so it records no e-Manual
    /// range for its position limits and the typed form is refused; single
    /// raw writes keep the order check against the other limit.
    #[test]
    fn unverified_y_takes_raw_position_limits_only() {
        let time = MockTime::new();
        let mut bus = ym070(&time);
        let start = mark(&bus);
        let scale = bus.actuator(id(1)).unwrap().position_scale();
        assert_eq!(
            bus.actuator(id(1))
                .unwrap()
                .set_position_limits(scale.to_position(-500), scale.to_position(500)),
            Err(crate::ValidationError::NoUnit {
                id: id(1),
                register: "Min Position Limit",
                mode: LimitMode::Bounded,
            }
            .into())
        );
        assert_quiet(&bus, start);
        ack(&mut bus, 1);
        let mut ym = bus.actuator(id(1)).unwrap();
        ym.write_limit(Raw(MinPositionLimit), -500).unwrap();
        assert_eq!(ym.position_limits_raw(), Some(-500..=1000));
        assert_eq!(
            ym.write_limit(Raw(MaxPositionLimit), -600),
            Err(crate::ValidationError::InvertedLimits {
                id: id(1),
                min: -500,
                max: -600
            }
            .into())
        );
        assert_eq!(
            sent_since(&bus, start),
            p2_write(1, 84, &(-500i32).to_le_bytes())
        );
    }

    /// Limit `F` of the YM070 at `raw`, checked as a verified table would
    /// allow it: the whole `i32` range, as its e-Manual gives for Max/Min
    /// Position Limit(76, 84).
    fn checked<F: LimitField>(bus: &mut TestBus<'_, P2>, raw: i32) -> Checked {
        let ym = bus.actuator(id(1)).unwrap();
        let site = Site {
            range: Some((i64::from(i32::MIN), i64::from(i32::MAX))),
            ..limit_site::<F>(ym.kind()).unwrap()
        };
        Checked::new(site, raw)
    }

    /// Sends the pair `min`, `max` to the YM070 as `set_position_limits`
    /// does.
    fn send_pair(bus: &mut TestBus<'_, P2>, min: i32, max: i32) -> Result<(), crate::Error> {
        let lo = checked::<MinPositionLimit>(bus, min);
        let hi = checked::<MaxPositionLimit>(bus, max);
        let mut ym = bus.actuator(id(1)).unwrap();
        let (first, second) = ordered(ym.kind(), lo, hi);
        ym.send_limits(first, Some(second))
    }

    /// Plan D3: Max Position Limit(76) and Min Position Limit(84) are apart:
    /// one Write each, the minimum first unless it lies above the maximum
    /// the bus holds.
    #[test]
    fn apart_position_limits_keep_the_pair_ordered() {
        let time = MockTime::new();
        let mut bus = ym070(&time);
        let start = mark(&bus);
        script(&mut bus, &std::vec![p2_status(1, 0, &[]); 5]);
        send_pair(&mut bus, -500, 500).unwrap();
        assert_eq!(
            bus.actuator(id(1)).unwrap().position_limits_raw(),
            Some(-500..=500)
        );
        send_pair(&mut bus, 2000, 3000).unwrap();
        let mut ym = bus.actuator(id(1)).unwrap();
        assert_eq!(ym.position_limits_raw(), Some(2000..=3000));
        ym.write_limit(Raw(MinPositionLimit), 2500).unwrap();
        assert_eq!(ym.position_limits_raw(), Some(2500..=3000));
        assert_eq!(
            sent_since(&bus, start),
            concat(&[
                p2_write(1, 84, &(-500i32).to_le_bytes()),
                p2_write(1, 76, &500i32.to_le_bytes()),
                p2_write(1, 76, &3000i32.to_le_bytes()),
                p2_write(1, 84, &2000i32.to_le_bytes()),
                p2_write(1, 84, &2500i32.to_le_bytes()),
            ])
        );
        assert_eq!(
            ym_refuses(&mut bus),
            Err(crate::ValidationError::InvertedLimits {
                id: id(1),
                min: 2500,
                max: 2000
            }
            .into())
        );
    }

    /// A Max Position Limit below the held minimum, refused.
    fn ym_refuses(bus: &mut TestBus<'_, P2>) -> Result<(), crate::Error> {
        bus.actuator(id(1))
            .unwrap()
            .write_limit(Raw(MaxPositionLimit), 2000)
    }

    /// Plan D5: the second write of an apart pair failing leaves the limits
    /// unknown; both acknowledged next time, they are known again.
    #[test]
    fn apart_pair_failing_halfway_forgets_the_limits() {
        let time = MockTime::new();
        let mut bus = ym070(&time);
        let start = mark(&bus);
        ack(&mut bus, 1);
        assert!(send_pair(&mut bus, -500, 500).is_err());
        assert_eq!(bus.actuator(id(1)).unwrap().position_limits_raw(), None);
        let mark = mark(&bus);
        script(&mut bus, &[p2_status(1, 0, &[]), p2_status(1, 0, &[])]);
        send_pair(&mut bus, -400, 400).unwrap();
        assert_eq!(
            bus.actuator(id(1)).unwrap().position_limits_raw(),
            Some(-400..=400)
        );
        assert_eq!(
            sent_since(&bus, start),
            concat(&[
                p2_write(1, 84, &(-500i32).to_le_bytes()),
                p2_write(1, 76, &500i32.to_le_bytes()),
                p2_write(1, 84, &(-400i32).to_le_bytes()),
                p2_write(1, 76, &400i32.to_le_bytes()),
            ])
        );
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[
                p2_write(1, 84, &(-400i32).to_le_bytes()),
                p2_write(1, 76, &400i32.to_le_bytes()),
            ])
        );
    }
}
