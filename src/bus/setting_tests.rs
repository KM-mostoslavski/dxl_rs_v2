//! Typed EEPROM settings on the bus (plan 2026-11-10, S2): Homing Offset,
//! Moving Threshold and Return Delay Time, byte for byte on a scripted
//! line: the setters' packets, every refusal before a byte, the cache they
//! leave alone, and the getters.
//!
//! Register addresses, sizes, units and ranges are those of the models'
//! control tables, verified against their e-Manual pages on 2026-11-10:
//! XM430-W350 (<https://emanual.robotis.com/docs/en/dxl/x/xm430-w350/>) and
//! AX-12A (<https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/>).

extern crate std;

/// `instruction` of `register` at `addr` (`len` bytes) on actuator `n`.
#[cfg(feature = "x-series")]
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

#[cfg(feature = "x-series")]
mod xm430 {
    use core::time::Duration;

    use super::on;
    use crate::actuator::{Actuator, Describe, LimitMode, ModelNumber, Role};
    use crate::bus::testing::*;
    use crate::error::{Error, Instruction, TransportError, ValidationError};
    use crate::field::{GoalPosition, HomingOffset, MovingThreshold, Raw, ReturnDelayTime};
    use crate::protocol::P2;
    use crate::transport::MockTime;
    use crate::units::{Position, Velocity};

    /// XM430-W210 model number: its table, unverified, has the three
    /// settings and no unit for them.
    const XM430_W210: u16 = 1030;

    /// An XM430-W350 at ID 1, limits 1000..=3000, in Operating Mode `mode`,
    /// as Setup leaves it: torque `Off`, not confirmed off.
    fn declared(time: &MockTime, mode: u8) -> TestBus<'_, P2> {
        let mut bus = test_bus(time);
        xm430_p2(line(&mut bus), 1, 1, 1000, 3000, mode);
        bus.declare([(id(1), ModelNumber(XM430))]).unwrap();
        bus
    }

    /// The same, torque confirmed off by an acknowledged `disable_torque()`,
    /// as a setting write needs (plan 2026-11-09 D8).
    fn xm430(time: &MockTime, mode: u8) -> TestBus<'_, P2> {
        let mut bus = declared(time, mode);
        disable(&mut bus);
        bus
    }

    fn disable(bus: &mut TestBus<'_, P2>) {
        ack(bus, 1);
        bus.actuator(id(1)).unwrap().disable_torque().unwrap();
    }

    /// An angle of `ticks` position ticks of an XM430-W350, from no zero
    /// point: an offset.
    fn ticks(bus: &mut TestBus<'_, P2>, ticks: f32) -> Position {
        let per_tick = bus
            .actuator(id(1))
            .unwrap()
            .position_scale()
            .radians_per_tick();
        Position::from_radians(ticks * per_tick)
    }

    /// The Moving Threshold of raw `raw`, in the table's 0.229 rpm step.
    fn threshold(bus: &mut TestBus<'_, P2>, raw: u32) -> Velocity {
        bus.actuator(id(1))
            .unwrap()
            .kind()
            .data()
            .role(Role::MovingThreshold)
            .unwrap()
            .scale(LimitMode::Bounded)
            .unwrap()
            .velocity(raw)
    }

    /// S2: each setter writes its register in its table's unit: Return
    /// Delay Time(9) in 2 µs steps, Homing Offset(20) in position ticks
    /// with no zero point, Moving Threshold(24) in 0.229 rpm steps. The bus
    /// holds no copy of them, so nothing it caches changes.
    #[test]
    fn each_setter_writes_its_register() {
        let time = MockTime::new();
        let mut bus = xm430(&time, 3);
        let (forward, back) = (ticks(&mut bus, 1024.0), ticks(&mut bus, -1024.0));
        let speed = threshold(&mut bus, 10);
        let mark = mark(&bus);
        script(&mut bus, &std::vec![p2_status(1, 0, &[]); 7]);
        let mut xm = bus.actuator(id(1)).unwrap();
        let before = xm.kind().clone();
        assert_eq!(xm.set_return_delay_time(Duration::from_micros(500)), Ok(()));
        assert_eq!(
            xm.set_return_delay_time(Duration::from_nanos(19_400)),
            Ok(())
        );
        assert_eq!(xm.set_homing_offset(forward), Ok(()));
        assert_eq!(xm.set_homing_offset(back), Ok(()));
        assert_eq!(xm.set_homing_offset(Position::ZERO), Ok(()));
        assert_eq!(xm.set_moving_threshold(speed), Ok(()));
        assert_eq!(xm.write_setting(Raw(HomingOffset), 1_044_479), Ok(()));
        assert_eq!(*xm.kind(), before);
        assert_eq!(xm.position_limits_raw(), Some(1000..=3000));
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[
                p2_write(1, 9, &[250]),
                p2_write(1, 9, &[10]),
                p2_write(1, 20, &1024i32.to_le_bytes()),
                p2_write(1, 20, &(-1024i32).to_le_bytes()),
                p2_write(1, 20, &0i32.to_le_bytes()),
                p2_write(1, 24, &10u32.to_le_bytes()),
                p2_write(1, 20, &1_044_479i32.to_le_bytes()),
            ])
        );
        assert_done(&bus);
    }

    /// S2: each getter reads its register once, never cached; Return Delay
    /// Time comes back in whole 2 µs steps, Homing Offset as an angle from
    /// no zero point.
    #[test]
    fn getters_read_their_registers() {
        let time = MockTime::new();
        let mut bus = xm430(&time, 3);
        let (back, zero) = (ticks(&mut bus, -1024.0), ticks(&mut bus, 0.0));
        let speed = threshold(&mut bus, 10);
        let mark = mark(&bus);
        script(
            &mut bus,
            &[
                p2_status(1, 0, &[254]),
                p2_status(1, 0, &(-1024i32).to_le_bytes()),
                p2_status(1, 0, &0i32.to_le_bytes()),
                p2_status(1, 0, &10u32.to_le_bytes()),
                p2_status(1, 0, &[250]),
            ],
        );
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(xm.return_delay_time(), Ok(Duration::from_micros(508)));
        let offset = xm.homing_offset().unwrap();
        assert!(
            (offset.radians() - back.radians()).abs() < 1e-5,
            "{offset:?}"
        );
        assert_eq!(xm.homing_offset(), Ok(zero));
        assert_eq!(xm.moving_threshold(), Ok(speed));
        assert_eq!(xm.read(ReturnDelayTime), Ok(Duration::from_micros(500)));
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[
                p2_read(1, 9, 1),
                p2_read(1, 20, 4),
                p2_read(1, 20, 4),
                p2_read(1, 24, 4),
                p2_read(1, 9, 1),
            ])
        );
        assert_done(&bus);
    }

    /// S2, plan 2026-11-09 D4: outside the e-Manual range, not fitting the
    /// register, not finite: refused, and nothing is sent.
    #[test]
    fn bad_values_send_nothing() {
        let time = MockTime::new();
        let mut bus = xm430(&time, 3);
        let beyond = ticks(&mut bus, 1_044_481.0);
        let fast = threshold(&mut bus, 1024);
        let quiet = mark(&bus);
        let mut xm = bus.actuator(id(1)).unwrap();
        let out = |raw, min, max| -> Result<(), Error> {
            Err(ValidationError::OutOfLimits {
                id: id(1),
                raw,
                min,
                max,
            }
            .into())
        };
        assert_eq!(
            xm.set_return_delay_time(Duration::from_micros(510)),
            out(255, 0, 254)
        );
        assert_eq!(
            xm.set_return_delay_time(Duration::from_secs(1)),
            Err(ValidationError::DoesNotFit {
                raw: 500_000,
                size: 1,
                request: on(Instruction::Write, 1, "Return Delay Time", 9, 1),
            }
            .into())
        );
        assert_eq!(
            xm.set_homing_offset(beyond),
            out(1_044_481, -1_044_479, 1_044_479)
        );
        assert_eq!(
            xm.write_setting(Raw(HomingOffset), -1_044_480),
            out(-1_044_480, -1_044_479, 1_044_479)
        );
        assert_eq!(
            xm.set_homing_offset(Position::from_radians(f32::NAN)),
            Err(ValidationError::NotFinite {
                request: on(Instruction::Write, 1, "Homing Offset", 20, 4),
            }
            .into())
        );
        assert_eq!(xm.set_moving_threshold(fast), out(1024, 0, 1023));
        assert_eq!(
            xm.set_moving_threshold(Velocity::from_rad_per_s(-1.0)),
            Err(ValidationError::DoesNotFit {
                raw: -42,
                size: 4,
                request: on(Instruction::Write, 1, "Moving Threshold", 24, 4),
            }
            .into())
        );
        assert_quiet(&bus, quiet);
    }

    /// Plan 2026-11-09 D4, D8: torque on refuses every setting with
    /// `ActuatorIsOn`, torque not confirmed off since Setup with
    /// `TorqueUnconfirmed`; nothing is sent.
    #[test]
    fn torque_not_confirmed_off_refuses_every_setting() {
        let time = MockTime::new();
        let mut bus = declared(&time, 3);
        let quiet = mark(&bus);
        let unconfirmed = |register| -> Result<(), Error> {
            Err(ValidationError::TorqueUnconfirmed {
                id: id(1),
                register,
            }
            .into())
        };
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(
            xm.set_return_delay_time(Duration::ZERO),
            unconfirmed("Return Delay Time")
        );
        assert_eq!(
            xm.set_homing_offset(Position::ZERO),
            unconfirmed("Homing Offset")
        );
        assert_eq!(
            xm.set_moving_threshold(Velocity::ZERO),
            unconfirmed("Moving Threshold")
        );
        assert_eq!(
            xm.write_setting(Raw(MovingThreshold), 10),
            unconfirmed("Moving Threshold")
        );
        assert_quiet(&bus, quiet);

        ack(&mut bus, 1);
        bus.actuator(id(1)).unwrap().enable_torque().unwrap();
        let enabled = mark(&bus);
        let is_on = |register| -> Result<(), Error> {
            Err(ValidationError::ActuatorIsOn {
                id: id(1),
                register,
            }
            .into())
        };
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(
            xm.set_return_delay_time(Duration::ZERO),
            is_on("Return Delay Time")
        );
        assert_eq!(xm.set_homing_offset(Position::ZERO), is_on("Homing Offset"));
        assert_eq!(
            xm.set_moving_threshold(Velocity::ZERO),
            is_on("Moving Threshold")
        );
        assert_eq!(
            xm.write_setting(Raw(ReturnDelayTime), 0),
            is_on("Return Delay Time")
        );
        assert_quiet(&bus, enabled);
    }

    /// S2: Homing Offset shifts Present Position, not the limits the bus
    /// holds: a write of it, acknowledged or failed, leaves the position
    /// limits, the mode and the goal check as they were.
    #[test]
    fn homing_offset_keeps_the_limits_and_goals() {
        let time = MockTime::new();
        let mut bus = xm430(&time, 3);
        let offset = ticks(&mut bus, 512.0);
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(
            xm.set_homing_offset(offset),
            Err(TransportError::NoReply {
                request: on(Instruction::Write, 1, "Homing Offset", 20, 4),
                timeout_us: 10_000,
            }
            .into())
        );
        assert_eq!(xm.position_limits_raw(), Some(1000..=3000));
        assert_eq!(xm.limit_mode(), LimitMode::Bounded);
        script(&mut bus, &[p2_status(1, 0, &[]), p2_status(1, 0, &[])]);
        let mark = mark(&bus);
        let mut xm = bus.actuator(id(1)).unwrap();
        xm.enable_torque().unwrap();
        assert_eq!(xm.write(Raw(GoalPosition), 2000), Ok(()));
        assert_eq!(
            xm.write(Raw(GoalPosition), 3001),
            Err(ValidationError::OutOfLimits {
                id: id(1),
                raw: 3001,
                min: 1000,
                max: 3000,
            }
            .into())
        );
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[
                p2_write(1, 64, &[1]),
                p2_write(1, 116, &2000i32.to_le_bytes()),
            ])
        );
    }

    /// Plan 2026-11-09 D6: a model whose table gives the settings no unit
    /// (an unverified XM430-W210) refuses the typed forms, the getters
    /// included: a Homing Offset never falls back to the position scale
    /// and its zero point. The raw forms work.
    #[test]
    fn no_unit_refuses_the_typed_form_only() {
        let time = MockTime::new();
        let mut bus = test_bus::<P2, 32>(&time);
        hold(&mut bus, XM430_W210, 1, (0, 4095), LimitMode::Bounded, OFF);
        disable(&mut bus);
        let quiet = mark(&bus);
        let mut xm = bus.actuator(id(1)).unwrap();
        let no_unit = |register| -> Result<(), Error> {
            Err(ValidationError::NoUnit {
                id: id(1),
                register,
                mode: LimitMode::Bounded,
            }
            .into())
        };
        assert_eq!(
            xm.set_homing_offset(Position::ZERO),
            no_unit("Homing Offset")
        );
        assert_eq!(
            xm.homing_offset(),
            no_unit("Homing Offset").map(|()| Position::ZERO)
        );
        assert_eq!(
            xm.read(HomingOffset),
            no_unit("Homing Offset").map(|()| Position::ZERO)
        );
        assert_eq!(
            xm.set_moving_threshold(Velocity::ZERO),
            no_unit("Moving Threshold")
        );
        assert_eq!(
            xm.moving_threshold(),
            no_unit("Moving Threshold").map(|()| Velocity::ZERO)
        );
        assert_eq!(
            xm.set_return_delay_time(Duration::ZERO),
            no_unit("Return Delay Time")
        );
        assert_eq!(
            xm.return_delay_time(),
            no_unit("Return Delay Time").map(|()| Duration::ZERO)
        );
        assert_quiet(&bus, quiet);
        script(
            &mut bus,
            &[
                p2_status(1, 0, &[]),
                p2_status(1, 0, &(-10i32).to_le_bytes()),
                p2_status(1, 0, &[]),
            ],
        );
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(xm.write_setting(Raw(HomingOffset), -10), Ok(()));
        assert_eq!(xm.read(Raw(HomingOffset)), Ok(-10));
        assert_eq!(xm.write_setting(Raw(ReturnDelayTime), 0), Ok(()));
        assert_eq!(
            sent_since(&bus, quiet),
            concat(&[
                p2_write(1, 20, &(-10i32).to_le_bytes()),
                p2_read(1, 20, 4),
                p2_write(1, 9, &[0]),
            ])
        );
    }

    /// Plan 2026-11-09 D2: the settings are read fields, so a
    /// multi-actuator read (one Sync Read on Protocol 2) saves them before
    /// a run (NFR-11).
    #[test]
    fn settings_read_on_many() {
        let time = MockTime::new();
        let mut bus = xm430(&time, 3);
        let mark = mark(&bus);
        line(&mut bus).reply(&p2_status(1, 0, &[100]));
        let mut out = [Duration::ZERO];
        bus.read_field(ReturnDelayTime, [id(1)], &mut out).unwrap();
        assert_eq!(out, [Duration::from_micros(200)]);
        assert_eq!(sent_since(&bus, mark), p2_sync_read(9, 1, &[1]));
    }
}

#[cfg(feature = "ax")]
mod ax12a {
    use core::time::Duration;

    use crate::actuator::ModelNumber;
    use crate::bus::testing::*;
    use crate::error::{Error, ValidationError};
    use crate::field::{HomingOffset, Raw, ReturnDelayTime};
    use crate::protocol::P1;
    use crate::transport::MockTime;
    use crate::units::{Position, Velocity};

    /// An AX-12A at ID 1, angle limits 0..=1023, torque confirmed off by an
    /// acknowledged `disable_torque()`.
    fn ax12(time: &MockTime) -> TestBus<'_, P1> {
        let mut bus = test_bus(time);
        ax12_p1(line(&mut bus), 1, 1, 0, 1023);
        bus.declare([(id(1), ModelNumber(AX12))]).unwrap();
        ack(&mut bus, 1);
        bus.actuator(id(1)).unwrap().disable_torque().unwrap();
        bus
    }

    /// S2: Return Delay Time(5), 1 byte, in 2 µs steps; the getter reads
    /// it back.
    #[test]
    fn return_delay_time_is_written_and_read() {
        let time = MockTime::new();
        let mut bus = ax12(&time);
        let mark = mark(&bus);
        script(
            &mut bus,
            &[
                p1_status(1, 0, &[]),
                p1_status(1, 0, &[]),
                p1_status(1, 0, &[250]),
            ],
        );
        let mut ax = bus.actuator(id(1)).unwrap();
        assert_eq!(ax.set_return_delay_time(Duration::from_micros(20)), Ok(()));
        assert_eq!(ax.write_setting(Raw(ReturnDelayTime), 254), Ok(()));
        assert_eq!(ax.return_delay_time(), Ok(Duration::from_micros(500)));
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[
                p1_write(1, 5, &[10]),
                p1_write(1, 5, &[254]),
                p1_read(1, 5, 1)
            ])
        );
        assert_done(&bus);
    }

    /// S2: the AX-12A has no Homing Offset and no Moving Threshold: the
    /// setters and getters are refused with `Unsupported`, nothing sent;
    /// a Return Delay Time out of range or with torque on is refused too.
    #[test]
    fn refusals_send_nothing() {
        let time = MockTime::new();
        let mut bus = ax12(&time);
        let quiet = mark(&bus);
        let mut ax = bus.actuator(id(1)).unwrap();
        let unsupported = |register| -> Result<(), Error> {
            Err(ValidationError::Unsupported {
                id: id(1),
                register,
            }
            .into())
        };
        assert_eq!(
            ax.set_homing_offset(Position::ZERO),
            unsupported("Homing Offset")
        );
        assert_eq!(
            ax.homing_offset(),
            unsupported("Homing Offset").map(|()| Position::ZERO)
        );
        assert_eq!(
            ax.write_setting(Raw(HomingOffset), 0),
            unsupported("Homing Offset")
        );
        assert_eq!(
            ax.set_moving_threshold(Velocity::ZERO),
            unsupported("Moving Threshold")
        );
        assert_eq!(
            ax.moving_threshold(),
            unsupported("Moving Threshold").map(|()| Velocity::ZERO)
        );
        assert_eq!(
            ax.write_setting(Raw(ReturnDelayTime), 255),
            Err(ValidationError::OutOfLimits {
                id: id(1),
                raw: 255,
                min: 0,
                max: 254,
            }
            .into())
        );
        assert_quiet(&bus, quiet);

        ack(&mut bus, 1);
        bus.actuator(id(1)).unwrap().enable_torque().unwrap();
        let enabled = mark(&bus);
        assert_eq!(
            bus.actuator(id(1))
                .unwrap()
                .set_return_delay_time(Duration::from_micros(20)),
            Err(ValidationError::ActuatorIsOn {
                id: id(1),
                register: "Return Delay Time",
            }
            .into())
        );
        assert_quiet(&bus, enabled);
    }
}
