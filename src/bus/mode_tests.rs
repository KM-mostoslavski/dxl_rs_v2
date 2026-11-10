//! The operating mode on the bus (plan 2026-11-10, S3), byte for byte on a
//! scripted line: `set_operating_mode` and its goal-bound read, every
//! refusal before a byte, the goals the bus accepts after each write, a
//! failed write or read, and `operating_mode`.
//!
//! Register addresses, sizes and mode values are those of the models'
//! control tables: XM430-W350 Operating Mode(11), Velocity Limit(44),
//! Current Limit(38), Goal Current(102), Goal Velocity(104) and Goal
//! Position(116) (<https://emanual.robotis.com/docs/en/dxl/x/xm430-w350/>,
//! modes 0 Current, 1 Velocity, 3 Position, 4 Extended Position, 5
//! Current-based Position, 16 PWM).

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
    use super::on;
    use crate::actuator::{Actuator, Describe, LimitMode, ModelNumber, OperatingMode};
    use crate::bus::testing::*;
    use crate::error::{Error, Instruction, TransportError, ValidationError};
    use crate::field::{GoalCurrent, GoalPosition, GoalVelocity, Raw};
    use crate::protocol::P2;
    use crate::transport::MockTime;

    /// XM430-W210 model number: its table, unverified, gives only the
    /// coarse mode lists (`bounded` 3, `unbounded` 4 and 5, `no_position`
    /// 0, 1 and 16).
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

    fn enable(bus: &mut TestBus<'_, P2>) {
        ack(bus, 1);
        bus.actuator(id(1)).unwrap().enable_torque().unwrap();
    }

    fn out(raw: i32, bound: i32) -> Result<(), Error> {
        Err(ValidationError::OutOfLimits {
            id: id(1),
            raw,
            min: -bound,
            max: bound,
        }
        .into())
    }

    fn unknown() -> Result<(), Error> {
        Err(ValidationError::LimitsUnknown { id: id(1) }.into())
    }

    /// S3: position → velocity → current. Each write is one Write of
    /// Operating Mode(11); a mode that bounds its goal then reads that
    /// bound once (Velocity Limit(44), Current Limit(38)), and goals are
    /// checked as Setup would have recorded the mode.
    #[test]
    fn position_to_velocity_to_current() {
        let time = MockTime::new();
        let mut bus = xm430(&time, 3);
        let start = mark(&bus);
        script(
            &mut bus,
            &[p2_status(1, 0, &[]), p2_status(1, 0, &100u32.to_le_bytes())],
        );
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(xm.set_operating_mode(OperatingMode::Velocity), Ok(()));
        assert_eq!(xm.limit_mode(), LimitMode::Velocity);
        assert_eq!(xm.goal_bound_raw(), Some(100));
        assert_eq!(xm.position_limits_raw(), Some(1000..=3000));
        enable(&mut bus);
        let enabled = mark(&bus);
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(xm.write(Raw(GoalVelocity), 101), out(101, 100));
        assert_eq!(xm.write(Raw(GoalVelocity), -101), out(-101, 100));
        assert_eq!(
            xm.write(Raw(GoalPosition), 2000),
            Err(ValidationError::NotPositionMode { id: id(1) }.into())
        );
        assert_quiet(&bus, enabled);
        ack(&mut bus, 1);
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(xm.write(Raw(GoalVelocity), -100), Ok(()));
        disable(&mut bus);

        script(
            &mut bus,
            &[p2_status(1, 0, &[]), p2_status(1, 0, &500u16.to_le_bytes())],
        );
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(xm.set_operating_mode(OperatingMode::Current), Ok(()));
        assert_eq!(xm.limit_mode(), LimitMode::Current);
        assert_eq!(xm.goal_bound_raw(), Some(500));
        enable(&mut bus);
        let enabled = mark(&bus);
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(xm.write(Raw(GoalCurrent), 501), out(501, 500));
        assert_eq!(
            xm.write(Raw(GoalVelocity), 0),
            Err(ValidationError::NotVelocityMode {
                id: id(1),
                mode: LimitMode::Current,
            }
            .into())
        );
        assert_quiet(&bus, enabled);
        ack(&mut bus, 1);
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(xm.write(Raw(GoalCurrent), 500), Ok(()));

        assert_eq!(
            sent_since(&bus, start),
            concat(&[
                p2_write(1, 11, &[1]),
                p2_read(1, 44, 4),
                p2_write(1, 64, &[1]),
                p2_write(1, 104, &(-100i32).to_le_bytes()),
                p2_write(1, 64, &[0]),
                p2_write(1, 11, &[0]),
                p2_read(1, 38, 2),
                p2_write(1, 64, &[1]),
                p2_write(1, 102, &500u16.to_le_bytes()),
            ])
        );
        assert_done(&bus);
    }

    /// S3: the position modes bound no goal, so their write reads nothing
    /// and the bound held for the old mode is dropped; position mode checks
    /// goals against the position limits again, extended and current-based
    /// position only against the register size, PWM refuses every goal.
    #[test]
    fn each_mode_checks_goals_as_setup_would() {
        let time = MockTime::new();
        let mut bus = xm430(&time, 1);
        assert_eq!(
            bus.actuator(id(1)).unwrap().goal_bound_raw(),
            Some(XM430_VELOCITY_LIMIT)
        );
        let start = mark(&bus);
        let cases = [
            (OperatingMode::Position, 3, LimitMode::Bounded),
            (OperatingMode::ExtendedPosition, 4, LimitMode::Unbounded),
            (OperatingMode::Pwm, 16, LimitMode::Pwm),
        ];
        let mut sent = std::vec::Vec::new();
        for (mode, value, limit) in cases {
            ack(&mut bus, 1);
            let mut xm = bus.actuator(id(1)).unwrap();
            assert_eq!(xm.set_operating_mode(mode), Ok(()), "{mode:?}");
            assert_eq!(xm.limit_mode(), limit);
            assert_eq!(xm.goal_bound_raw(), None);
            sent.push(p2_write(1, 11, &[value]));
        }
        assert_eq!(sent_since(&bus, start), concat(&sent));

        script(
            &mut bus,
            &[p2_status(1, 0, &[]), p2_status(1, 0, &700u16.to_le_bytes())],
        );
        let mut xm = bus.actuator(id(1)).unwrap();
        let mode = OperatingMode::CurrentBasedPosition;
        assert_eq!(xm.set_operating_mode(mode), Ok(()));
        assert_eq!(xm.limit_mode(), LimitMode::UnboundedCurrent);
        assert_eq!(xm.goal_bound_raw(), Some(700));
        enable(&mut bus);
        script(&mut bus, &[p2_status(1, 0, &[]), p2_status(1, 0, &[])]);
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(xm.write(Raw(GoalPosition), 5000), Ok(()));
        assert_eq!(xm.write(Raw(GoalCurrent), 701), out(701, 700));
        assert_eq!(xm.write(Raw(GoalCurrent), 700), Ok(()));
        disable(&mut bus);

        ack(&mut bus, 1);
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(xm.set_operating_mode(OperatingMode::Position), Ok(()));
        assert_eq!(xm.goal_bound_raw(), None);
        enable(&mut bus);
        ack(&mut bus, 1);
        let mut xm = bus.actuator(id(1)).unwrap();
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
        assert_eq!(xm.write(Raw(GoalPosition), 3000), Ok(()));
        assert_done(&bus);
    }

    /// S3: a write that fails after bytes went out leaves the mode unknown:
    /// every goal is refused with `LimitsUnknown`, also after a limit
    /// write, until a confirmed `set_operating_mode`.
    #[test]
    fn failed_write_refuses_every_goal() {
        let time = MockTime::new();
        let mut bus = xm430(&time, 1);
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(
            xm.set_operating_mode(OperatingMode::Position),
            Err(TransportError::NoReply {
                request: on(Instruction::Write, 1, "Operating Mode", 11, 1),
                timeout_us: 10_000,
            }
            .into())
        );
        assert_eq!(xm.goal_bound_raw(), None);
        script(&mut bus, &[p2_status(1, 0, &[]), p2_status(1, 0, &[])]);
        let mut xm = bus.actuator(id(1)).unwrap();
        xm.set_velocity_limit(crate::units::Velocity::ZERO).unwrap();
        xm.enable_torque().unwrap();
        let enabled = mark(&bus);
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(xm.write(Raw(GoalPosition), 2000), unknown());
        assert_eq!(xm.write(Raw(GoalVelocity), 0), unknown());
        assert_eq!(xm.write(Raw(GoalCurrent), 0), unknown());
        assert_quiet(&bus, enabled);
        disable(&mut bus);

        ack(&mut bus, 1);
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(xm.set_operating_mode(OperatingMode::Position), Ok(()));
        assert_eq!(xm.limit_mode(), LimitMode::Bounded);
        enable(&mut bus);
        ack(&mut bus, 1);
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(xm.write(Raw(GoalPosition), 2000), Ok(()));
        assert_done(&bus);
    }

    /// S3: a confirmed write whose bound read fails records the new mode
    /// and returns the read's error; the bound is unknown, so goals it
    /// bounds are refused with `LimitsUnknown` until a Velocity Limit write
    /// determines it.
    #[test]
    fn failed_bound_read_leaves_the_bound_unknown() {
        let time = MockTime::new();
        let mut bus = xm430(&time, 3);
        let start = mark(&bus);
        ack(&mut bus, 1);
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(
            xm.set_operating_mode(OperatingMode::Velocity),
            Err(TransportError::NoReply {
                request: on(Instruction::Read, 1, "Velocity Limit", 44, 4),
                timeout_us: 10_000,
            }
            .into())
        );
        assert_eq!(xm.limit_mode(), LimitMode::Velocity);
        assert_eq!(xm.goal_bound_raw(), None);
        assert_eq!(
            sent_since(&bus, start),
            concat(&[p2_write(1, 11, &[1]), p2_read(1, 44, 4)])
        );
        enable(&mut bus);
        let enabled = mark(&bus);
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(xm.write(Raw(GoalVelocity), 0), unknown());
        assert_eq!(
            xm.write(Raw(GoalPosition), 2000),
            Err(ValidationError::NotPositionMode { id: id(1) }.into())
        );
        assert_quiet(&bus, enabled);
        disable(&mut bus);

        script(&mut bus, &[p2_status(1, 0, &[]), p2_status(1, 0, &[])]);
        let mut xm = bus.actuator(id(1)).unwrap();
        xm.write_setting(Raw(crate::field::VelocityLimit), 50)
            .unwrap();
        assert_eq!(xm.goal_bound_raw(), Some(50));
        xm.enable_torque().unwrap();
        assert_eq!(xm.write(Raw(GoalVelocity), 51), out(51, 50));
        ack(&mut bus, 1);
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(xm.write(Raw(GoalVelocity), 50), Ok(()));
        assert_done(&bus);
    }

    /// S3, plan 2026-11-09 D8: torque on, or not confirmed off: refused
    /// with nothing sent, naming the mode register.
    #[test]
    fn torque_not_confirmed_off_sends_nothing() {
        let time = MockTime::new();
        let mut bus = declared(&time, 3);
        let quiet = mark(&bus);
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(
            xm.set_operating_mode(OperatingMode::Velocity),
            Err(ValidationError::TorqueUnconfirmed {
                id: id(1),
                register: "Operating Mode",
            }
            .into())
        );
        assert_quiet(&bus, quiet);
        enable(&mut bus);
        let enabled = mark(&bus);
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(
            xm.set_operating_mode(OperatingMode::Velocity),
            Err(ValidationError::ActuatorIsOn {
                id: id(1),
                register: "Operating Mode",
            }
            .into())
        );
        assert_eq!(xm.limit_mode(), LimitMode::Bounded);
        assert_quiet(&bus, enabled);
    }

    /// S3: a table that lists no single value for a mode refuses it with
    /// `UnsupportedMode`, nothing sent; the mode it does single out works.
    #[test]
    fn mode_without_one_value_is_unsupported() {
        let time = MockTime::new();
        let mut bus = test_bus(&time);
        hold(&mut bus, XM430_W210, 1, (0, 4095), LimitMode::Bounded, OFF);
        disable(&mut bus);
        let quiet = mark(&bus);
        let mut xm = bus.actuator(id(1)).unwrap();
        for mode in [
            OperatingMode::Current,
            OperatingMode::Velocity,
            OperatingMode::ExtendedPosition,
            OperatingMode::CurrentBasedPosition,
            OperatingMode::Pwm,
        ] {
            assert_eq!(
                xm.set_operating_mode(mode),
                Err(ValidationError::UnsupportedMode { id: id(1), mode }.into())
            );
        }
        assert_quiet(&bus, quiet);
        ack(&mut bus, 1);
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(xm.set_operating_mode(OperatingMode::Position), Ok(()));
        assert_eq!(sent_since(&bus, quiet), p2_write(1, 11, &[3]));
    }

    /// S3: `operating_mode` reads Operating Mode(11) once, maps it back to
    /// the named mode, and caches nothing; a value no named mode takes is
    /// `UnlistedMode`.
    #[test]
    fn operating_mode_reads_and_decodes() {
        let time = MockTime::new();
        let mut bus = declared(&time, 3);
        let start = mark(&bus);
        script(
            &mut bus,
            &[
                p2_status(1, 0, &[0]),
                p2_status(1, 0, &[1]),
                p2_status(1, 0, &[3]),
                p2_status(1, 0, &[4]),
                p2_status(1, 0, &[5]),
                p2_status(1, 0, &[16]),
                p2_status(1, 0, &[2]),
            ],
        );
        let mut xm = bus.actuator(id(1)).unwrap();
        for mode in [
            OperatingMode::Current,
            OperatingMode::Velocity,
            OperatingMode::Position,
            OperatingMode::ExtendedPosition,
            OperatingMode::CurrentBasedPosition,
            OperatingMode::Pwm,
        ] {
            assert_eq!(xm.operating_mode(), Ok(mode));
        }
        assert_eq!(
            xm.operating_mode(),
            Err(ValidationError::UnlistedMode {
                id: id(1),
                register: "Operating Mode",
                value: 2,
            }
            .into())
        );
        assert_eq!(xm.limit_mode(), LimitMode::Bounded);
        assert_eq!(sent_since(&bus, start), p2_read(1, 11, 1).repeat(7));
        assert_done(&bus);
    }

    /// S3: on a table with only the coarse lists, a mode it does not single
    /// out reads back as `UnlistedMode`.
    #[test]
    fn coarse_table_reads_unnamed_modes_as_unlisted() {
        let time = MockTime::new();
        let mut bus = test_bus::<P2, 32>(&time);
        hold(&mut bus, XM430_W210, 1, (0, 4095), LimitMode::Bounded, OFF);
        script(&mut bus, &[p2_status(1, 0, &[3]), p2_status(1, 0, &[1])]);
        let mut xm = bus.actuator(id(1)).unwrap();
        assert_eq!(xm.operating_mode(), Ok(OperatingMode::Position));
        assert_eq!(
            xm.operating_mode(),
            Err(ValidationError::UnlistedMode {
                id: id(1),
                register: "Operating Mode",
                value: 1,
            }
            .into())
        );
    }

    /// S3: the XL-320's mode register is Control Mode(11); joint mode is
    /// its `Position`, the only mode its table singles out.
    #[test]
    fn xl320_control_mode() {
        let time = MockTime::new();
        let mut bus = test_bus(&time);
        xl320_p2(line(&mut bus), 1, 1, 0, 1023, 2);
        bus.declare([(id(1), ModelNumber(XL320))]).unwrap();
        let quiet = mark(&bus);
        let mut xl = bus.actuator(id(1)).unwrap();
        assert_eq!(
            xl.set_operating_mode(OperatingMode::Position),
            Err(ValidationError::TorqueUnconfirmed {
                id: id(1),
                register: "Control Mode",
            }
            .into())
        );
        assert_quiet(&bus, quiet);
        disable(&mut bus);
        let confirmed = mark(&bus);
        let mut xl = bus.actuator(id(1)).unwrap();
        assert_eq!(
            xl.set_operating_mode(OperatingMode::Velocity),
            Err(ValidationError::UnsupportedMode {
                id: id(1),
                mode: OperatingMode::Velocity,
            }
            .into())
        );
        assert_quiet(&bus, confirmed);
        script(&mut bus, &[p2_status(1, 0, &[]), p2_status(1, 0, &[1])]);
        let mut xl = bus.actuator(id(1)).unwrap();
        assert_eq!(xl.set_operating_mode(OperatingMode::Position), Ok(()));
        assert_eq!(
            xl.operating_mode(),
            Err(ValidationError::UnlistedMode {
                id: id(1),
                register: "Control Mode",
                value: 1,
            }
            .into())
        );
        assert_eq!(
            sent_since(&bus, confirmed),
            concat(&[p2_write(1, 11, &[2]), p2_read(1, 11, 1)])
        );
    }

    /// S3: a reply reporting a hardware fault to the write marks the
    /// actuator `Faulted` and leaves the mode unknown.
    #[test]
    fn faulted_reply_leaves_the_mode_unknown() {
        let time = MockTime::new();
        let mut bus = xm430(&time, 3);
        line(&mut bus).reply(&p2_status(1, 0x80, &[]));
        let mut xm = bus.actuator(id(1)).unwrap();
        assert!(
            xm.set_operating_mode(OperatingMode::Velocity)
                .is_err_and(|err| err.is_hardware_fault())
        );
        assert_eq!(xm.torque_state(), FAULTED);
        assert_eq!(xm.limit_mode(), LimitMode::Bounded);
        assert_eq!(xm.goal_bound_raw(), None);
        assert_eq!(
            xm.kind().cache().check_goal(2000),
            Err(ValidationError::LimitsUnknown { id: id(1) })
        );
    }

    /// Plan 2026-11-09 D3: `DynBus` lends the same handle, with the same
    /// mode calls and bytes.
    #[cfg(feature = "std")]
    #[test]
    fn dyn_bus_handle_sets_the_mode() {
        use crate::{Config, DynBus};
        let time = MockTime::new();
        let mut line = crate::transport::MockTransport::new(&time);
        let next = xm430_p2(&mut line, 1, 1, 1000, 3000, 3);
        line.reply_to(next, &p2_status(1, 0, &[]))
            .reply_to(next + 1, &p2_status(1, 0, &[]))
            .reply_to(next + 2, &p2_status(1, 0, &1000u16.to_le_bytes()))
            .reply_to(next + 3, &p2_status(1, 0, &[0]));
        let mut bus = DynBus::<P2, _, _>::new(line, time.ticker(10), Config::new());
        bus.declare([(id(1), ModelNumber(XM430))]).unwrap();
        let mut xm = bus.actuator(id(1)).unwrap();
        xm.disable_torque().unwrap();
        assert_eq!(xm.set_operating_mode(OperatingMode::Current), Ok(()));
        assert_eq!(xm.goal_bound_raw(), Some(1000));
        assert_eq!(xm.operating_mode(), Ok(OperatingMode::Current));
        let (line, _) = bus.into_parts();
        assert!(line.sent().ends_with(&concat(&[
            p2_write(1, 64, &[0]),
            p2_write(1, 11, &[0]),
            p2_read(1, 38, 2),
            p2_read(1, 11, 1),
        ])));
        line.assert_done();
    }
}

#[cfg(feature = "ax")]
mod ax12a {
    use crate::actuator::{ModelNumber, OperatingMode};
    use crate::bus::testing::*;
    use crate::error::ValidationError;
    use crate::protocol::P1;
    use crate::transport::MockTime;

    /// S3: an AX-12A has no mode register (its mode follows the angle
    /// limits): both calls are refused with `Unsupported`, nothing sent.
    #[test]
    fn angle_limit_model_is_unsupported() {
        let time = MockTime::new();
        let mut bus = test_bus::<P1, 32>(&time);
        ax12_p1(line(&mut bus), 1, 1, 0, 1023);
        bus.declare([(id(1), ModelNumber(AX12))]).unwrap();
        ack(&mut bus, 1);
        bus.actuator(id(1)).unwrap().disable_torque().unwrap();
        let quiet = mark(&bus);
        let mut ax = bus.actuator(id(1)).unwrap();
        let unsupported = ValidationError::Unsupported {
            id: id(1),
            register: "Operating Mode",
        };
        for mode in [OperatingMode::Position, OperatingMode::Velocity] {
            assert_eq!(ax.set_operating_mode(mode), Err(unsupported.into()));
        }
        assert_eq!(ax.operating_mode(), Err(unsupported.into()));
        assert_quiet(&bus, quiet);
    }
}

#[cfg(feature = "mx")]
mod mx64_p1 {
    use crate::actuator::{ModelNumber, OperatingMode};
    use crate::bus::testing::*;
    use crate::error::ValidationError;
    use crate::protocol::P1;
    use crate::transport::MockTime;

    /// S3: an MX-64 on Protocol 1 follows its angle limits and a torque
    /// control switch, no Operating Mode: refused with `Unsupported`.
    #[test]
    fn torque_switch_model_is_unsupported() {
        let time = MockTime::new();
        let mut bus = test_bus::<P1, 32>(&time);
        mx64_p1(line(&mut bus), 1, 1, 0, 4095, 0);
        bus.declare([(id(1), ModelNumber(MX64_P1))]).unwrap();
        let quiet = mark(&bus);
        let mut mx = bus.actuator(id(1)).unwrap();
        let unsupported = ValidationError::Unsupported {
            id: id(1),
            register: "Operating Mode",
        };
        assert_eq!(
            mx.set_operating_mode(OperatingMode::Current),
            Err(unsupported.into())
        );
        assert_eq!(mx.operating_mode(), Err(unsupported.into()));
        assert_quiet(&bus, quiet);
    }
}
