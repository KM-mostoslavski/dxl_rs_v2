//! FR-9 on the bus: typed registers, Setup bounds, multi-actuator fields,
//! Bulk Read/Write and Reboot, byte for byte on a scripted line.
//!
//! Packets come from the `testing` builders, which are checked against the
//! e-Manual packet examples (Protocol 1.0 and 2.0 pages, fetched
//! 2026-11-03). Register addresses are those of the models' control tables,
//! verified against their e-Manual pages on 2026-11-03: AX-12A
//! (<https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/>), RX-28
//! (<https://emanual.robotis.com/docs/en/dxl/rx/rx-28/>), MX-64 on
//! Protocol 1.0 (<https://emanual.robotis.com/docs/en/dxl/mx/mx-64/>) and
//! MX-64(2.0) (<https://emanual.robotis.com/docs/en/dxl/mx/mx-64-2/>).

extern crate std;

/// Whether `a` and `b` agree to 1e-4 relative.
/// `instruction` of `register` at `addr` (`len` bytes) on actuator `n`.
#[cfg(any(feature = "ax", feature = "rx", feature = "mx"))]
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

#[cfg(any(feature = "ax", feature = "rx", feature = "mx"))]
#[track_caller]
fn close(a: f32, b: f32) {
    assert!(
        (a - b).abs() <= 1e-4 * b.abs().max(1e-3),
        "{a} is not close to {b}"
    );
}

/// The scale `role` of model `kind` has in `mode`.
#[cfg(any(feature = "ax", feature = "rx", feature = "mx"))]
fn scale_of(
    kind: &crate::ActuatorKind,
    role: crate::actuator::Role,
    mode: crate::actuator::LimitMode,
) -> crate::units::RegisterScale {
    use crate::actuator::Describe;
    kind.data().role(role).unwrap().scale(mode).unwrap()
}

#[cfg(feature = "ax")]
mod ax12a {
    use super::*;
    use crate::actuator::{Actuator, LimitMode, ModelNumber, Role, TorqueState};
    use crate::bulk;
    use crate::bus::testing::*;
    use crate::error::ValidationError;
    use crate::field::{self, Raw};
    use crate::models::{Ax12a, Ax18a};
    use crate::protocol::P1;
    use crate::transport::MockTime;
    use crate::units::{ComplianceMargin, ComplianceSlope, Current, SpeedLimit, Velocity};

    /// An AX-12A at ID 1 with these angle limits, torque `on` or off.
    fn ax12(time: &MockTime, cw: u16, ccw: u16, on: bool) -> TestBus<'_, P1> {
        let mut bus = test_bus(time);
        ax12_p1(line(&mut bus), 1, 1, cw, ccw);
        bus.declare([(id(1), ModelNumber(AX12))]).unwrap();
        if on {
            ack(&mut bus, 1);
            bus.actuator(id(1)).unwrap().enable_torque().unwrap();
        }
        bus
    }

    #[test]
    fn temperature_read_is_the_e_manual_read_example() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 1023, false);
        let mark = mark(&bus);
        line(&mut bus).reply(&[0xFF, 0xFF, 0x01, 0x03, 0x00, 0x20, 0xDB]);
        let celsius = bus.actuator(id(1)).unwrap().present_temperature().unwrap();
        assert_eq!(celsius.celsius(), 32.0);
        assert_eq!(
            sent_since(&bus, mark),
            [0xFF, 0xFF, 0x01, 0x04, 0x02, 0x2B, 0x01, 0xCC]
        );
    }

    #[test]
    fn reads_voltage_speed_load_moving_and_led() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 1023, false);
        let mark = mark(&bus);
        script(
            &mut bus,
            &[
                p1_status(1, 0, &[120]),
                p1_status(1, 0, &1124u16.to_le_bytes()),
                p1_status(1, 0, &1124u16.to_le_bytes()),
                p1_status(1, 0, &[1]),
                p1_status(1, 0, &[0]),
                p1_status(1, 0, &512u16.to_le_bytes()),
            ],
        );
        let mut ax = bus.actuator(id(1)).unwrap();
        close(ax.present_voltage().unwrap().volts(), 12.0);
        let speed = scale_of(ax.kind(), Role::PresentVelocity, LimitMode::Bounded);
        assert_eq!(ax.present_velocity().unwrap(), speed.velocity(1124));
        close(ax.present_load().unwrap().ratio(), -100.0 / 1023.0);
        assert!(ax.is_moving().unwrap());
        assert!(!ax.led().unwrap());
        assert_eq!(
            ax.speed_limit().unwrap(),
            SpeedLimit::Limited(
                scale_of(ax.kind(), Role::SpeedLimit, LimitMode::Bounded).velocity(512)
            )
        );
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[
                p1_read(1, 42, 1),
                p1_read(1, 38, 2),
                p1_read(1, 40, 2),
                p1_read(1, 46, 1),
                p1_read(1, 25, 1),
                p1_read(1, 32, 2),
            ])
        );
        assert_done(&bus);
    }

    #[test]
    fn registers_the_ax_lacks_are_unsupported_and_send_nothing() {
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
        assert_eq!(ax.present_current().map(drop), missing("Present Current"));
        assert_eq!(
            ax.hardware_errors().map(drop),
            missing("Hardware Error Status")
        );
        assert_eq!(ax.realtime_tick().map(drop), missing("Realtime Tick"));
        assert_eq!(ax.p_gain().map(drop), missing("Position P Gain"));
        assert_eq!(ax.d_gain().map(drop), missing("Position D Gain"));
        assert_eq!(ax.goal_current().map(drop), missing("Goal Current"));
        assert_eq!(
            ax.set_goal_current(Current::from_amps(0.1)),
            missing("Goal Current")
        );
        assert_eq!(
            ax.write(Raw(field::GoalCurrent), 5),
            missing("Goal Current")
        );
        assert_eq!(
            ax.read(field::TorqueCtrlModeEnable).map(drop),
            missing("Torque Ctrl Mode Enable")
        );
        assert_quiet(&bus, mark);
    }

    #[test]
    fn led_and_speed_limit_are_written_with_torque_off() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 1023, false);
        let mark = mark(&bus);
        let limit = scale_of(&bus.actuators()[0], Role::SpeedLimit, LimitMode::Bounded);
        script(&mut bus, &std::vec![p1_status(1, 0, &[]); 3]);
        let mut ax = bus.actuator(id(1)).unwrap();
        ax.set_led(true).unwrap();
        ax.set_speed_limit(SpeedLimit::Unlimited).unwrap();
        ax.set_speed_limit(SpeedLimit::Limited(limit.velocity(512)))
            .unwrap();
        assert_eq!(
            ax.write(Raw(field::SpeedLimit), 1024),
            Err(ValidationError::OutOfLimits {
                id: id(1),
                raw: 1024,
                min: 0,
                max: 1023
            }
            .into())
        );
        assert_eq!(
            ax.set_speed_limit(SpeedLimit::Limited(Velocity::from_rad_per_s(f32::NAN))),
            Err(ValidationError::NotFinite {
                request: on(crate::error::Instruction::Write, 1, "Moving Speed", 32, 2),
            }
            .into())
        );
        assert_eq!(ax.torque_state(), TorqueState::Off);
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[
                p1_write(1, 25, &[1]),
                p1_write(1, 32, &[0, 0]),
                p1_write(1, 32, &512u16.to_le_bytes()),
            ])
        );
    }

    #[test]
    fn wheel_mode_takes_only_a_raw_goal_velocity() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 0, true);
        assert_eq!(bus.actuators()[0].limit_mode(), LimitMode::Wheel);
        let mark = mark(&bus);
        let mut ax = bus.actuator(id(1)).unwrap();
        assert_eq!(ax.limit_mode(), LimitMode::Wheel);
        assert_eq!(
            ax.set_goal_velocity(Velocity::from_rad_per_s(1.0)),
            Err(ValidationError::NoUnit {
                id: id(1),
                register: "Moving Speed",
                mode: LimitMode::Wheel
            }
            .into())
        );
        assert_eq!(
            ax.present_velocity(),
            Err(ValidationError::NoUnit {
                id: id(1),
                register: "Present Speed",
                mode: LimitMode::Wheel
            }
            .into())
        );
        assert_eq!(
            ax.set_speed_limit(SpeedLimit::Unlimited),
            Err(ValidationError::NotPositionMode { id: id(1) }.into())
        );
        assert_quiet(&bus, mark);
        script(
            &mut bus,
            &[
                p1_status(1, 0, &[]),
                p1_status(1, 0, &1124u16.to_le_bytes()),
            ],
        );
        let mut ax = bus.actuator(id(1)).unwrap();
        ax.write(Raw(field::GoalVelocity), 1124).unwrap();
        assert_eq!(ax.read(Raw(field::PresentVelocity)), Ok(1124));
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[p1_write(1, 32, &[0x64, 0x04]), p1_read(1, 38, 2)])
        );
    }

    #[test]
    fn raw_wheel_speed_stays_in_the_e_manual_range() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 0, true);
        let mark = mark(&bus);
        let out_of_range = |raw| {
            Err(ValidationError::OutOfLimits {
                id: id(1),
                raw,
                min: 0,
                max: 2047,
            }
            .into())
        };
        let mut ax = bus.actuator(id(1)).unwrap();
        assert_eq!(ax.write(Raw(field::GoalVelocity), 2048), out_of_range(2048));
        assert_eq!(ax.write(Raw(field::GoalVelocity), -1), out_of_range(-1));
        assert_eq!(
            bus.sync_write(Raw(field::GoalVelocity), [id(1)], &[3000]),
            out_of_range(3000)
        );
        assert_quiet(&bus, mark);
        script(&mut bus, &[p1_status(1, 0, &[])]);
        bus.actuator(id(1))
            .unwrap()
            .write(Raw(field::GoalVelocity), 2047)
            .unwrap();
        assert_eq!(
            sent_since(&bus, mark),
            p1_write(1, 32, &2047u16.to_le_bytes())
        );
    }

    #[test]
    fn goal_velocity_needs_torque_and_wheel_mode() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 0, false);
        let off_mark = mark(&bus);
        assert_eq!(
            bus.actuator(id(1))
                .unwrap()
                .write(Raw(field::GoalVelocity), 100),
            Err(ValidationError::ActuatorIsOff { id: id(1) }.into())
        );
        let mut joint = ax12(&time, 0, 1023, true);
        let joint_mark = mark(&joint);
        assert_eq!(
            joint
                .actuator(id(1))
                .unwrap()
                .write(Raw(field::GoalVelocity), 100),
            Err(ValidationError::NotVelocityMode {
                id: id(1),
                mode: LimitMode::Bounded
            }
            .into())
        );
        assert_quiet(&bus, off_mark);
        assert_quiet(&joint, joint_mark);
    }

    #[test]
    fn tier2_compliance_on_the_ax12a() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 1023, false);
        let mark = mark(&bus);
        script(
            &mut bus,
            &[
                p1_status(1, 0, &[]),
                p1_status(1, 0, &[]),
                p1_status(1, 0, &[]),
                p1_status(1, 0, &[]),
                p1_status(1, 0, &[1]),
                p1_status(1, 0, &[1]),
                p1_status(1, 0, &[32]),
                p1_status(1, 0, &[32]),
            ],
        );
        let mut handle = bus.actuator(id(1)).unwrap();
        assert!(handle.model::<Ax18a>().is_none());
        let mut ax = handle.model::<Ax12a>().unwrap();
        assert_eq!(ax.id(), id(1));
        assert_eq!(ax.name(), "AX-12A");
        ax.set_cw_compliance_margin(ComplianceMargin::from_raw(2))
            .unwrap();
        ax.set_ccw_compliance_margin(ComplianceMargin::from_raw(3))
            .unwrap();
        ax.set_cw_compliance_slope(ComplianceSlope::new(5).unwrap())
            .unwrap();
        ax.set_ccw_compliance_slope(ComplianceSlope::new(7).unwrap())
            .unwrap();
        assert_eq!(ax.cw_compliance_margin(), Ok(ComplianceMargin::from_raw(1)));
        assert_eq!(
            ax.ccw_compliance_margin(),
            Ok(ComplianceMargin::from_raw(1))
        );
        assert_eq!(ax.cw_compliance_slope().map(ComplianceSlope::step), Ok(5));
        assert_eq!(ax.ccw_compliance_slope().map(ComplianceSlope::step), Ok(5));
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[
                p1_write(1, 26, &[2]),
                p1_write(1, 27, &[3]),
                p1_write(1, 28, &[32]),
                p1_write(1, 29, &[128]),
                p1_read(1, 26, 1),
                p1_read(1, 27, 1),
                p1_read(1, 28, 1),
                p1_read(1, 29, 1),
            ])
        );
        assert_eq!(bus.torque_state(id(1)), Some(TorqueState::Off));
    }

    #[test]
    fn compliance_slope_outside_the_e_manual_range_is_refused() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 1023, false);
        let mark = mark(&bus);
        let mut handle = bus.actuator(id(1)).unwrap();
        assert_eq!(
            handle.write(field::CwComplianceSlope, ComplianceSlope::from_raw(255)),
            Err(ValidationError::OutOfLimits {
                id: id(1),
                raw: 255,
                min: 0,
                max: 254
            }
            .into())
        );
        assert_quiet(&bus, mark);
    }

    #[test]
    fn ax_has_no_protocol_1_bulk_read() {
        let time = MockTime::new();
        let mut bus = ax12(&time, 0, 1023, false);
        let mark = mark(&bus);
        let mut out = [0; 1];
        assert_eq!(
            bus.bulk_read(bulk::read(
                Raw(field::PresentTemperature),
                [id(1)],
                &mut out
            )),
            Err(ValidationError::UnsupportedInstruction {
                id: id(1),
                instruction: "Bulk Read"
            }
            .into())
        );
        assert_quiet(&bus, mark);
    }
}

#[cfg(feature = "rx")]
mod rx28 {
    use super::*;
    use crate::actuator::{LimitMode, ModelNumber, Role};
    use crate::bulk;
    use crate::bus::testing::*;
    use crate::error::ValidationError;
    use crate::field::{self, Raw};
    use crate::models::Rx28;
    use crate::protocol::P1;
    use crate::transport::MockTime;
    use crate::units::{ComplianceSlope, Velocity};

    /// RX-28 at IDs `ids`, joint mode 0..=1023, torque off.
    fn rx28s<'t>(time: &'t MockTime, ids: &[u8]) -> TestBus<'t, P1> {
        let mut bus = test_bus(time);
        let mut at = 1;
        for &n in ids {
            at = angle_p1(line(&mut bus), at, n, RX28, 0, 1023);
        }
        bus.declare(ids.iter().map(|&n| (id(n), ModelNumber(RX28))))
            .unwrap();
        bus
    }

    #[test]
    fn telemetry_of_one_rx28() {
        let time = MockTime::new();
        let mut bus = rx28s(&time, &[7]);
        let mark = mark(&bus);
        script(
            &mut bus,
            &[
                p1_status(7, 0, &[40]),
                p1_status(7, 0, &[118]),
                p1_status(7, 0, &300u16.to_le_bytes()),
                p1_status(7, 0, &200u16.to_le_bytes()),
            ],
        );
        let mut rx = bus.actuator(id(7)).unwrap();
        assert_eq!(rx.present_temperature().unwrap().celsius(), 40.0);
        close(rx.present_voltage().unwrap().volts(), 11.8);
        let speed = scale_of(rx.kind(), Role::PresentVelocity, LimitMode::Bounded);
        assert_eq!(rx.present_velocity().unwrap(), speed.velocity(300));
        close(rx.present_load().unwrap().ratio(), 200.0 / 1023.0);
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[
                p1_read(7, 43, 1),
                p1_read(7, 42, 1),
                p1_read(7, 38, 2),
                p1_read(7, 40, 2),
            ])
        );
    }

    #[test]
    fn many_rx28_read_one_field_with_one_read_each() {
        let time = MockTime::new();
        let mut bus = rx28s(&time, &[1, 2]);
        let mark = mark(&bus);
        script(&mut bus, &[p1_status(1, 0, &[32]), p1_status(2, 0, &[33])]);
        let mut celsius = [crate::Temperature::default(); 2];
        bus.read_field(field::PresentTemperature, bus.ids(), &mut celsius)
            .unwrap();
        assert_eq!(celsius.map(crate::Temperature::celsius), [32.0, 33.0]);
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[p1_read(1, 43, 1), p1_read(2, 43, 1)])
        );
    }

    #[test]
    fn sync_write_of_led_and_speed_limit_on_protocol_1() {
        let time = MockTime::new();
        let mut bus = rx28s(&time, &[1, 2]);
        let mark = mark(&bus);
        bus.sync_write(field::Led, bus.ids(), &[true, false])
            .unwrap();
        bus.sync_write(Raw(field::SpeedLimit), bus.ids(), &[100, 0])
            .unwrap();
        assert_eq!(
            bus.sync_write(Raw(field::SpeedLimit), bus.ids(), &[100, 2000]),
            Err(ValidationError::OutOfLimits {
                id: id(2),
                raw: 2000,
                min: 0,
                max: 1023
            }
            .into())
        );
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[
                p1_sync_write(25, 1, &[(1, &[1]), (2, &[0])]),
                p1_sync_write(32, 2, &[(1, &100u16.to_le_bytes()), (2, &[0, 0])]),
            ])
        );
    }

    #[test]
    fn goal_velocity_sync_write_is_all_or_nothing() {
        let time = MockTime::new();
        let mut bus = test_bus::<P1, 32>(&time);
        hold(&mut bus, RX28, 1, (0, 0), LimitMode::Wheel, ON);
        hold(&mut bus, RX28, 2, (0, 1023), LimitMode::Bounded, ON);
        hold(&mut bus, RX28, 3, (0, 0), LimitMode::Wheel, OFF);
        let mark = mark(&bus);
        assert_eq!(
            bus.sync_write(Raw(field::GoalVelocity), [id(1), id(2)], &[100, 100]),
            Err(ValidationError::NotVelocityMode {
                id: id(2),
                mode: LimitMode::Bounded
            }
            .into())
        );
        assert_eq!(
            bus.sync_write(Raw(field::GoalVelocity), [id(1), id(3)], &[100, 100]),
            Err(ValidationError::ActuatorIsOff { id: id(3) }.into())
        );
        assert_eq!(
            bus.write_goal_velocities([id(1)], &[Velocity::from_rad_per_s(1.0)]),
            Err(ValidationError::NoUnit {
                id: id(1),
                register: "Moving Speed",
                mode: LimitMode::Wheel
            }
            .into())
        );
        assert_quiet(&bus, mark);
        bus.sync_write(Raw(field::GoalVelocity), [id(1)], &[1124])
            .unwrap();
        assert_eq!(
            sent_since(&bus, mark),
            p1_sync_write(32, 2, &[(1, &[0x64, 0x04])])
        );
    }

    #[test]
    fn tier2_compliance_on_the_rx28() {
        let time = MockTime::new();
        let mut bus = rx28s(&time, &[1]);
        let mark = mark(&bus);
        ack(&mut bus, 1);
        let mut handle = bus.actuator(id(1)).unwrap();
        let mut rx = handle.model::<Rx28>().unwrap();
        rx.set_ccw_compliance_slope(ComplianceSlope::new(1).unwrap())
            .unwrap();
        assert_eq!(sent_since(&bus, mark), p1_write(1, 29, &[2]));
    }

    #[test]
    fn rx28_has_no_protocol_1_bulk_read() {
        let time = MockTime::new();
        let mut bus = rx28s(&time, &[1, 2]);
        let mark = mark(&bus);
        let mut out = [0; 2];
        assert_eq!(
            bus.bulk_read(bulk::read(
                Raw(field::PresentPosition),
                [id(2), id(1)],
                &mut out
            )),
            Err(ValidationError::UnsupportedInstruction {
                id: id(2),
                instruction: "Bulk Read"
            }
            .into())
        );
        assert_quiet(&bus, mark);
    }

    #[cfg(feature = "partial-reads")]
    #[test]
    fn partial_field_read_keeps_each_result() {
        let time = MockTime::new();
        let mut bus = rx28s(&time, &[1, 2]);
        line(&mut bus).reply(&p1_status(1, 0, &[120]));
        let mut out = [Ok(crate::Voltage::default()); 2];
        bus.read_field_partial(field::PresentVoltage, bus.ids(), &mut out)
            .unwrap();
        close(out[0].unwrap().volts(), 12.0);
        assert_eq!(
            out[1],
            Err(crate::TransportError::NoReply {
                request: on(crate::error::Instruction::Read, 2, "Present Voltage", 42, 1),
                timeout_us: 10_000,
            }
            .into())
        );
    }
}

#[cfg(feature = "mx")]
mod mx64_p1 {
    use super::*;
    use crate::bus::testing::*;
    use std::vec::Vec;

    use crate::actuator::{LimitMode, ModelNumber, Role};
    use crate::bulk;
    use crate::error::ValidationError;
    use crate::field::{self, Raw};
    use crate::models::Mx64P1;
    use crate::protocol::P1;
    use crate::transport::MockTime;
    use crate::units::{Current, Gain, Velocity};

    /// MX-64 (Protocol 1.0) at `ids` with these angle limits and torque
    /// switch, torque on.
    fn mx64s<'t>(time: &'t MockTime, ids: &[u8], cw: u16, ccw: u16, switch: u8) -> TestBus<'t, P1> {
        let mut bus = test_bus(time);
        let mut at = 1;
        for &n in ids {
            at = mx64_p1(line(&mut bus), at, n, cw, ccw, switch);
        }
        bus.declare(ids.iter().map(|&n| (id(n), ModelNumber(MX64_P1))))
            .unwrap();
        let acks: Vec<_> = ids.iter().map(|&n| p1_status(n, 0, &[])).collect();
        script(&mut bus, &acks);
        bus.enable_torque(bus.ids()).unwrap();
        bus
    }

    #[test]
    fn bulk_read_is_the_e_manual_example() {
        let time = MockTime::new();
        let mut bus = mx64s(&time, &[1, 2], 0, 4095, 0);
        let mark = mark(&bus);
        line(&mut bus)
            .reply(&[0xFF, 0xFF, 0x01, 0x04, 0x00, 0x00, 0x80, 0x7A])
            .reply(&[0xFF, 0xFF, 0x02, 0x04, 0x00, 0x00, 0x80, 0x79]);
        let (mut goal, mut present) = ([0; 1], [0; 1]);
        bus.bulk_read((
            bulk::read(Raw(field::GoalPosition), [id(1)], &mut goal),
            bulk::read(Raw(field::PresentPosition), [id(2)], &mut present),
        ))
        .unwrap();
        assert_eq!((goal, present), ([0x8000], [0x8000]));
        assert_eq!(
            sent_since(&bus, mark),
            [
                0xFF, 0xFF, 0xFE, 0x09, 0x92, 0x00, 0x02, 0x01, 0x1E, 0x02, 0x02, 0x24, 0x1D
            ]
        );
        assert_done(&bus);
    }

    #[test]
    fn torque_mode_takes_goal_currents() {
        let time = MockTime::new();
        let mut bus = mx64s(&time, &[1], 0, 4095, 1);
        let mark = mark(&bus);
        let torque = scale_of(&bus.actuators()[0], Role::GoalCurrent, LimitMode::Current);
        script(&mut bus, &std::vec![p1_status(1, 0, &[]); 2]);
        let mut mx = bus.actuator(id(1)).unwrap();
        assert_eq!(mx.goal_bound_raw(), None);
        mx.set_goal_current(torque.current(100)).unwrap();
        mx.set_goal_current(torque.current(1124)).unwrap();
        assert_eq!(
            mx.set_goal_position_raw(2048),
            Err(ValidationError::NotPositionMode { id: id(1) }.into())
        );
        assert_eq!(
            mx.set_goal_velocity(Velocity::ZERO),
            Err(ValidationError::NotVelocityMode {
                id: id(1),
                mode: LimitMode::Current
            }
            .into())
        );
        assert_eq!(
            mx.set_goal_current(Current::from_amps(10.0)),
            Err(ValidationError::DoesNotFit {
                raw: 2222,
                size: 2,
                request: on(crate::error::Instruction::Write, 1, "Goal Torque", 71, 2),
            }
            .into())
        );
        close(torque.current(100).amps(), 0.45);
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[
                p1_write(1, 71, &100u16.to_le_bytes()),
                p1_write(1, 71, &1124u16.to_le_bytes()),
            ])
        );
    }

    #[test]
    fn joint_mode_refuses_goal_currents() {
        let time = MockTime::new();
        let mut bus = mx64s(&time, &[1], 0, 4095, 0);
        let mark = mark(&bus);
        assert_eq!(
            bus.actuator(id(1)).unwrap().set_goal_current(Current::ZERO),
            Err(ValidationError::NotCurrentMode {
                id: id(1),
                mode: LimitMode::Bounded
            }
            .into())
        );
        assert_quiet(&bus, mark);
    }

    #[test]
    fn wheel_mode_takes_typed_goal_velocities() {
        let time = MockTime::new();
        let mut bus = mx64s(&time, &[1], 0, 0, 0);
        let mark = mark(&bus);
        let wheel = scale_of(&bus.actuators()[0], Role::GoalVelocity, LimitMode::Wheel);
        script(
            &mut bus,
            &[
                p1_status(1, 0, &[]),
                p1_status(1, 0, &1124u16.to_le_bytes()),
            ],
        );
        let mut mx = bus.actuator(id(1)).unwrap();
        mx.set_goal_velocity(wheel.velocity(1124)).unwrap();
        assert_eq!(mx.goal_velocity().unwrap(), wheel.velocity(1124));
        assert_eq!(
            mx.set_goal_velocity(Velocity::from_rad_per_s(100.0)),
            Err(ValidationError::DoesNotFit {
                raw: 8377,
                size: 2,
                request: on(crate::error::Instruction::Write, 1, "Moving Speed", 32, 2),
            }
            .into())
        );
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[p1_write(1, 32, &1124u16.to_le_bytes()), p1_read(1, 32, 2)])
        );
    }

    #[test]
    fn tier2_gains_tick_and_torque_switch() {
        let time = MockTime::new();
        let mut bus = mx64s(&time, &[1], 0, 4095, 0);
        let mark = mark(&bus);
        script(
            &mut bus,
            &[
                p1_status(1, 0, &[]),
                p1_status(1, 0, &[]),
                p1_status(1, 0, &[32]),
                p1_status(1, 0, &[0]),
                p1_status(1, 0, &[0]),
                p1_status(1, 0, &500u16.to_le_bytes()),
            ],
        );
        let mut handle = bus.actuator(id(1)).unwrap();
        let mut mx = handle.model::<Mx64P1>().unwrap();
        mx.set_p_gain(Gain::from_raw(32)).unwrap();
        mx.set_d_gain(Gain::from_raw(4)).unwrap();
        assert_eq!(
            mx.set_p_gain(Gain::from_raw(255)),
            Err(ValidationError::OutOfLimits {
                id: id(1),
                raw: 255,
                min: 0,
                max: 254
            }
            .into())
        );
        assert_eq!(mx.p_gain(), Ok(Gain::from_raw(32)));
        assert_eq!(mx.d_gain(), Ok(Gain::from_raw(0)));
        assert_eq!(mx.torque_ctrl_mode_enable(), Ok(false));
        assert_eq!(handle.realtime_tick().unwrap().raw(), 500);
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[
                p1_write(1, 28, &[32]),
                p1_write(1, 26, &[4]),
                p1_read(1, 28, 1),
                p1_read(1, 26, 1),
                p1_read(1, 70, 1),
                p1_read(1, 50, 2),
            ])
        );
    }

    #[test]
    fn realtime_tick_needs_firmware_40() {
        let time = MockTime::new();
        let mut bus = test_bus::<P1, 32>(&time);
        let at = mx64_p1_fw(line(&mut bus), 1, 1, 39, 0, 4095, 0);
        mx64_p1_fw(line(&mut bus), at, 2, 40, 0, 4095, 0);
        bus.declare([(id(1), ModelNumber(MX64_P1)), (id(2), ModelNumber(MX64_P1))])
            .unwrap();
        assert_eq!(sent(&bus), concat(&[mx64_p1_sent(1), mx64_p1_sent(2)]));
        let mark = mark(&bus);
        let old = Err(ValidationError::Unsupported {
            id: id(1),
            register: "Realtime Tick",
        }
        .into());
        assert_eq!(bus.actuator(id(1)).unwrap().realtime_tick(), old);
        let mut ticks = [crate::Tick::from_raw(0); 2];
        assert_eq!(
            bus.read_field(field::RealtimeTick, [id(2), id(1)], &mut ticks),
            old.map(drop)
        );
        assert_quiet(&bus, mark);
        script(&mut bus, &[p1_status(2, 0, &500u16.to_le_bytes())]);
        assert_eq!(
            bus.actuator(id(2)).unwrap().realtime_tick().unwrap().raw(),
            500
        );
        assert_eq!(sent_since(&bus, mark), p1_read(2, 50, 2));
    }

    #[cfg(feature = "ax")]
    #[test]
    fn mixed_bulk_read_refuses_the_ax() {
        let time = MockTime::new();
        let mut bus = test_bus::<P1, 32>(&time);
        hold(&mut bus, AX12, 1, (0, 1023), BOUNDED, OFF);
        hold(&mut bus, MX64_P1, 2, (0, 4095), BOUNDED, OFF);
        let mark = mark(&bus);
        let (mut a, mut b) = ([0; 1], [0; 1]);
        assert_eq!(
            bus.bulk_read((
                bulk::read(Raw(field::PresentPosition), [id(2)], &mut a),
                bulk::read(Raw(field::PresentPosition), [id(1)], &mut b),
            )),
            Err(ValidationError::UnsupportedInstruction {
                id: id(1),
                instruction: "Bulk Read"
            }
            .into())
        );
        assert_quiet(&bus, mark);
    }
}

#[cfg(feature = "mx")]
mod mx64_p2 {
    use super::*;
    use crate::bus::testing::*;
    use std::vec::Vec;

    use crate::actuator::{LimitMode, ModelNumber, Role, TorqueState};
    use crate::error::{Error, SetupError, ValidationError};
    use crate::field::{self, Raw};
    use crate::models::{Mx64, Mx64P1};
    use crate::protocol::P2;
    use crate::transport::MockTime;
    use crate::units::{Current, Gain, SpeedLimit, Velocity};
    use crate::{HardwareErrors, Temperature};
    use crate::{TransportError, bulk};

    /// MX-64(2.0) at `ids`, limits 0..=4095, in Operating Mode `mode`, with
    /// torque on where `on`.
    fn mx64s<'t>(time: &'t MockTime, ids: &[u8], mode: u8, on: bool) -> TestBus<'t, P2> {
        let mut bus = test_bus(time);
        let mut at = 1;
        for &n in ids {
            at = mx64_p2(line(&mut bus), at, n, 0, 4095, mode);
        }
        bus.declare(ids.iter().map(|&n| (id(n), ModelNumber(MX64))))
            .unwrap();
        if on {
            let acks: Vec<_> = ids.iter().map(|&n| p2_status(n, 0, &[])).collect();
            script(&mut bus, &acks);
            bus.enable_torque(bus.ids()).unwrap();
        }
        bus
    }

    fn velocity(bus: &TestBus<'_, P2>, raw: i32) -> Velocity {
        scale_of(&bus.actuators()[0], Role::GoalVelocity, LimitMode::Velocity)
            .velocity(raw.cast_unsigned())
    }

    fn current(bus: &TestBus<'_, P2>, raw: i32) -> Current {
        scale_of(&bus.actuators()[0], Role::GoalCurrent, LimitMode::Current)
            .current(raw.cast_unsigned())
    }

    #[test]
    fn setup_reads_the_bound_of_the_mode() {
        let time = MockTime::new();
        for (mode, bound, read) in [
            (1, Some(XM430_VELOCITY_LIMIT), Some(p2_read(1, 44, 4))),
            (
                0,
                Some(u32::from(XM430_CURRENT_LIMIT)),
                Some(p2_read(1, 38, 2)),
            ),
            (
                5,
                Some(u32::from(XM430_CURRENT_LIMIT)),
                Some(p2_read(1, 38, 2)),
            ),
            (3, None, None),
            (4, None, None),
            (16, None, None),
        ] {
            let mut bus = mx64s(&time, &[1], mode, false);
            let mut expected = concat(&[
                p2_instruction(1, PING, &[]),
                p2_read(1, 52, 4),
                p2_read(1, 48, 4),
                p2_read(1, 11, 1),
            ]);
            expected.extend(read.unwrap_or_default());
            assert_eq!(sent(&bus), expected, "mode {mode}");
            let mut fixture = p2_instruction(1, PING, &[]);
            fixture.extend(xm430_p2_reads_sent_in(1, mode));
            assert_eq!(fixture, expected, "mode {mode}");
            assert_eq!(
                bus.actuator(id(1)).unwrap().goal_bound_raw(),
                bound,
                "mode {mode}"
            );
            assert_done(&bus);
        }
    }

    #[test]
    fn a_failed_bound_read_fails_setup() {
        let time = MockTime::new();
        let mut bus = test_bus::<P2, 32>(&time);
        line(&mut bus)
            .reply_to(1, &p2_pong(1, MX64, MX64_FW))
            .reply_to(2, &p2_status(1, 0, &0i32.to_le_bytes()))
            .reply_to(3, &p2_status(1, 0, &4095i32.to_le_bytes()))
            .reply_to(4, &p2_status(1, 0, &[1]));
        assert_eq!(
            bus.declare([(id(1), ModelNumber(MX64))]),
            Err(TransportError::NoReply {
                request: on(crate::error::Instruction::Read, 1, "Velocity Limit", 44, 4)
                    .step(crate::error::Step::GoalBound),
                timeout_us: 10_000,
            }
            .into())
        );
        assert!(bus.actuators().is_empty());
    }

    #[test]
    fn telemetry_reads() {
        let time = MockTime::new();
        let mut bus = mx64s(&time, &[1], 3, false);
        let mark = mark(&bus);
        script(
            &mut bus,
            &[
                p2_status(1, 0, &[36]),
                p2_status(1, 0, &120u16.to_le_bytes()),
                p2_status(1, 0, &(-50i32).to_le_bytes()),
                p2_status(1, 0, &(-100i16).to_le_bytes()),
                p2_status(1, 0, &[0x04]),
                p2_status(1, 0, &1000u16.to_le_bytes()),
                p2_status(1, 0, &[1]),
                p2_status(1, 0, &[1]),
                p2_status(1, 0, &0u32.to_le_bytes()),
            ],
        );
        let mut mx = bus.actuator(id(1)).unwrap();
        assert_eq!(mx.present_temperature().unwrap().celsius(), 36.0);
        close(mx.present_voltage().unwrap().volts(), 12.0);
        close(
            mx.present_velocity().unwrap().rad_per_s(),
            -50.0 * 0.229 * core::f32::consts::TAU / 60.0,
        );
        close(mx.present_current().unwrap().amps(), -0.336);
        let errors = mx.hardware_errors().unwrap();
        assert_eq!(errors, HardwareErrors::from_raw(0x04));
        assert!(errors.overheating());
        assert_eq!(mx.realtime_tick().unwrap().raw(), 1000);
        assert!(mx.is_moving().unwrap());
        assert!(mx.led().unwrap());
        assert_eq!(mx.goal_velocity().unwrap(), Velocity::ZERO);
        assert_eq!(mx.torque_state(), TorqueState::Off);
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[
                p2_read(1, 146, 1),
                p2_read(1, 144, 2),
                p2_read(1, 128, 4),
                p2_read(1, 126, 2),
                p2_read(1, 70, 1),
                p2_read(1, 120, 2),
                p2_read(1, 122, 1),
                p2_read(1, 65, 1),
                p2_read(1, 104, 4),
            ])
        );
        assert_done(&bus);
    }

    #[test]
    fn velocity_mode_checks_torque_mode_and_velocity_limit() {
        let time = MockTime::new();
        let mut bus = mx64s(&time, &[1], 1, true);
        let start = mark(&bus);
        script(&mut bus, &std::vec![p2_status(1, 0, &[]); 2]);
        let (down, top, over) = (
            velocity(&bus, -100),
            velocity(&bus, 200),
            velocity(&bus, 201),
        );
        let mut mx = bus.actuator(id(1)).unwrap();
        assert_eq!(mx.goal_bound_raw(), Some(XM430_VELOCITY_LIMIT));
        mx.set_goal_velocity(down).unwrap();
        mx.set_goal_velocity(top).unwrap();
        assert_eq!(
            mx.set_goal_velocity(over),
            Err(ValidationError::OutOfLimits {
                id: id(1),
                raw: 201,
                min: -200,
                max: 200
            }
            .into())
        );
        assert_eq!(
            mx.write(Raw(field::GoalVelocity), -201),
            Err(ValidationError::OutOfLimits {
                id: id(1),
                raw: -201,
                min: -200,
                max: 200
            }
            .into())
        );
        assert_eq!(
            mx.set_goal_current(Current::ZERO),
            Err(ValidationError::NotCurrentMode {
                id: id(1),
                mode: LimitMode::Velocity
            }
            .into())
        );
        assert_eq!(
            sent_since(&bus, start),
            concat(&[
                p2_write(1, 104, &(-100i32).to_le_bytes()),
                p2_write(1, 104, &200i32.to_le_bytes()),
            ])
        );
        ack(&mut bus, 1);
        bus.actuator(id(1)).unwrap().disable_torque().unwrap();
        let quiet = mark(&bus);
        assert_eq!(
            bus.actuator(id(1)).unwrap().set_goal_velocity(down),
            Err(ValidationError::ActuatorIsOff { id: id(1) }.into())
        );
        assert_quiet(&bus, quiet);
    }

    #[test]
    fn current_mode_checks_the_current_limit() {
        let time = MockTime::new();
        let mut bus = mx64s(&time, &[1], 0, true);
        let mark = mark(&bus);
        ack(&mut bus, 1);
        let (ok, over) = (current(&bus, -100), current(&bus, 1001));
        let mut mx = bus.actuator(id(1)).unwrap();
        mx.set_goal_current(ok).unwrap();
        assert_eq!(
            mx.set_goal_current(over),
            Err(ValidationError::OutOfLimits {
                id: id(1),
                raw: 1001,
                min: -1000,
                max: 1000
            }
            .into())
        );
        assert_eq!(
            mx.set_goal_position_raw(2048),
            Err(ValidationError::NotPositionMode { id: id(1) }.into())
        );
        assert_eq!(
            sent_since(&bus, mark),
            p2_write(1, 102, &(-100i16).to_le_bytes())
        );
    }

    #[test]
    fn profile_velocity_led_and_gains_with_torque_off() {
        let time = MockTime::new();
        let mut bus = mx64s(&time, &[1], 3, false);
        let mark = mark(&bus);
        script(&mut bus, &std::vec![p2_status(1, 0, &[]); 4]);
        let mut handle = bus.actuator(id(1)).unwrap();
        handle.set_speed_limit(SpeedLimit::Unlimited).unwrap();
        handle.set_led(true).unwrap();
        assert_eq!(
            handle.write(Raw(field::SpeedLimit), 40_000),
            Err(ValidationError::OutOfLimits {
                id: id(1),
                raw: 40_000,
                min: 0,
                max: 32_767
            }
            .into())
        );
        assert!(handle.model::<Mx64P1>().is_none());
        let mut mx = handle.model::<Mx64>().unwrap();
        mx.set_p_gain(Gain::from_raw(850)).unwrap();
        mx.set_d_gain(Gain::from_raw(0)).unwrap();
        assert_eq!(
            mx.set_d_gain(Gain::from_raw(16_384)),
            Err(ValidationError::OutOfLimits {
                id: id(1),
                raw: 16_384,
                min: 0,
                max: 16_383
            }
            .into())
        );
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[
                p2_write(1, 112, &0u32.to_le_bytes()),
                p2_write(1, 65, &[1]),
                p2_write(1, 84, &850u16.to_le_bytes()),
                p2_write(1, 80, &0u16.to_le_bytes()),
            ])
        );
        assert_eq!(bus.torque_state(id(1)), Some(TorqueState::Off));
    }

    #[test]
    fn mx64_has_no_present_load() {
        let time = MockTime::new();
        let mut bus = mx64s(&time, &[1], 3, false);
        let mark = mark(&bus);
        assert_eq!(
            bus.actuator(id(1)).unwrap().present_load(),
            Err(ValidationError::Unsupported {
                id: id(1),
                register: "Present Load"
            }
            .into())
        );
        assert_quiet(&bus, mark);
    }

    #[test]
    fn reboot_is_the_e_manual_example_and_turns_torque_off() {
        let time = MockTime::new();
        let mut bus = mx64s(&time, &[1], 3, true);
        let mark = mark(&bus);
        line(&mut bus).reply(&[
            0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x04, 0x00, 0x55, 0x00, 0xA1, 0x0C,
        ]);
        let mut mx = bus.actuator(id(1)).unwrap();
        assert_eq!(mx.torque_state(), TorqueState::On);
        mx.reboot().unwrap();
        assert_eq!(mx.torque_state(), TorqueState::Off);
        assert_eq!(
            sent_since(&bus, mark),
            [0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x03, 0x00, 0x08, 0x2F, 0x4E]
        );
        let mut bus = mx64s(&time, &[1], 3, true);
        let mut mx = bus.actuator(id(1)).unwrap();
        assert!(mx.reboot().is_err());
        assert_eq!(mx.torque_state(), TorqueState::Off);
    }

    #[test]
    fn reboot_keeps_a_fault_until_clear_fault_asks() {
        let time = MockTime::new();
        let mut bus = mx64s(&time, &[1], 3, true);
        script(
            &mut bus,
            &[
                p2_status(1, 0x80, &[0x04]),
                p2_status(1, 0, &[]),
                p2_status(1, 0, &[0]),
            ],
        );
        let mut mx = bus.actuator(id(1)).unwrap();
        assert_eq!(mx.hardware_errors(), Ok(HardwareErrors::from_raw(0x04)));
        assert_eq!(mx.torque_state(), TorqueState::Faulted);
        mx.reboot().unwrap();
        assert_eq!(mx.torque_state(), TorqueState::Faulted);
        mx.clear_fault().unwrap();
        assert_eq!(mx.torque_state(), TorqueState::Off);
    }

    #[test]
    fn hardware_errors_names_the_fault_the_alert_reports() {
        use crate::error::{P2Code, StatusError, StatusReport};
        let time = MockTime::new();
        let mut bus = mx64s(&time, &[1, 2], 3, false);
        let mark = mark(&bus);
        script(
            &mut bus,
            &[
                p2_status(1, 0x80, &[0x24]),
                p2_status(1, 0x80, &[0x24]),
                p2_status(1, 0x80, &[0]),
                p2_status(1, 0x84, &[0x24]),
                p2_status(2, 0, &[0]),
            ],
        );
        let alert = |code| -> Result<HardwareErrors, Error> {
            Err(StatusError {
                id: id(1),
                report: StatusReport::P2 { alert: true, code },
                request: on(
                    crate::error::Instruction::Read,
                    1,
                    "Hardware Error Status",
                    70,
                    1,
                ),
            }
            .into())
        };
        let mut mx = bus.actuator(id(1)).unwrap();
        assert_eq!(mx.hardware_errors(), Ok(HardwareErrors::from_raw(0x24)));
        assert_eq!(mx.torque_state(), TorqueState::Faulted);
        assert_eq!(mx.read(Raw(field::HardwareErrorStatus)), Ok(0x24));
        assert_eq!(mx.hardware_errors(), alert(None));
        assert_eq!(mx.hardware_errors(), alert(Some(P2Code::DataRange)));
        let mut clean = bus.actuator(id(2)).unwrap();
        assert_eq!(clean.hardware_errors(), Ok(HardwareErrors::from_raw(0)));
        assert_eq!(clean.torque_state(), TorqueState::Off);
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[
                p2_read(1, 70, 1),
                p2_read(1, 70, 1),
                p2_read(1, 70, 1),
                p2_read(1, 70, 1),
                p2_read(2, 70, 1),
            ])
        );

        let next = line(&mut bus).flushes() + 1;
        line(&mut bus)
            .reply_to(
                next,
                &concat(&[p2_status(1, 0x80, &[0x24]), p2_status(2, 0, &[0])]),
            )
            .reply_to(next + 1, &p2_status(1, 0x80, &[0x24]))
            .reply_to(next + 2, &p2_status(1, 0x80, &[75]))
            .reply_to(next + 3, &p2_status(1, 0x80, &[80]));
        let mut out = [HardwareErrors::from_raw(0); 2];
        let err = bus
            .sync_read(field::HardwareErrorStatus, [id(1), id(2)], &mut out)
            .unwrap_err();
        assert_eq!(
            err,
            Error::Fault(crate::error::FaultError::Temperature(
                crate::error::TemperatureError::WithinLimit {
                    id: id(1),
                    measured: crate::error::Celsius(75),
                    limit: crate::error::Celsius(80),
                    faults: HardwareErrors::from_raw(0x24),
                    request: on(
                        crate::error::Instruction::SyncRead,
                        1,
                        "Hardware Error Status",
                        70,
                        1
                    ),
                }
            ))
        );
        assert_eq!(
            std::string::ToString::to_string(&err),
            "id 1: overheated, now 75 °C (limit 80 °C), also overload"
        );
    }

    #[test]
    fn sync_read_and_sync_write_of_fr9_fields() {
        let time = MockTime::new();
        let mut bus = mx64s(&time, &[1, 2], 1, true);
        let mark = mark(&bus);
        line(&mut bus)
            .reply(&p2_status(1, 0, &[36]))
            .reply(&p2_status(2, 0, &[37]));
        let mut celsius = [Temperature::default(); 2];
        bus.sync_read(field::PresentTemperature, bus.ids(), &mut celsius)
            .unwrap();
        assert_eq!(celsius.map(Temperature::celsius), [36.0, 37.0]);
        let goals = [velocity(&bus, 100), velocity(&bus, -100)];
        bus.write_goal_velocities(bus.ids(), &goals).unwrap();
        assert_eq!(
            bus.write_goal_velocities(bus.ids(), &[goals[0], velocity(&bus, 300)]),
            Err(ValidationError::OutOfLimits {
                id: id(2),
                raw: 300,
                min: -200,
                max: 200
            }
            .into())
        );
        assert_eq!(
            bus.write_goal_currents(bus.ids(), &[Current::ZERO; 2]),
            Err(ValidationError::NotCurrentMode {
                id: id(1),
                mode: LimitMode::Velocity
            }
            .into())
        );
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[
                p2_sync_read(146, 1, &[1, 2]),
                p2_sync_write(
                    104,
                    4,
                    &[(1, &100i32.to_le_bytes()), (2, &(-100i32).to_le_bytes())]
                ),
            ])
        );
    }

    #[test]
    fn write_goal_currents_in_current_mode() {
        let time = MockTime::new();
        let mut bus = mx64s(&time, &[1, 2], 0, true);
        let mark = mark(&bus);
        let goals = [current(&bus, 10), current(&bus, -10)];
        bus.write_goal_currents(bus.ids(), &goals).unwrap();
        assert_eq!(
            sent_since(&bus, mark),
            p2_sync_write(
                102,
                2,
                &[(1, &10i16.to_le_bytes()), (2, &(-10i16).to_le_bytes())]
            )
        );
    }

    #[test]
    fn bulk_read_is_the_e_manual_example() {
        let time = MockTime::new();
        let mut bus = mx64s(&time, &[1, 2], 3, false);
        let mark = mark(&bus);
        line(&mut bus)
            .reply(&[
                0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x06, 0x00, 0x55, 0x00, 0x77, 0x00, 0xC3, 0x69,
            ])
            .reply(&[
                0xFF, 0xFF, 0xFD, 0x00, 0x02, 0x05, 0x00, 0x55, 0x00, 0x24, 0x8B, 0xA9,
            ]);
        let mut volts = [crate::Voltage::default(); 1];
        let mut celsius = [Temperature::default(); 1];
        bus.bulk_read((
            bulk::read(field::PresentVoltage, [id(1)], &mut volts),
            bulk::read(field::PresentTemperature, [id(2)], &mut celsius),
        ))
        .unwrap();
        close(volts[0].volts(), 11.9);
        assert_eq!(celsius[0].celsius(), 36.0);
        assert_eq!(
            sent_since(&bus, mark),
            [
                0xFF, 0xFF, 0xFD, 0x00, 0xFE, 0x0D, 0x00, 0x92, 0x01, 0x90, 0x00, 0x02, 0x00, 0x02,
                0x92, 0x00, 0x01, 0x00, 0x1A, 0x05
            ]
        );
        assert_done(&bus);
    }

    #[test]
    fn bulk_read_is_strict_and_marks_faults() {
        let time = MockTime::new();
        let mut bus = mx64s(&time, &[1, 2], 3, true);
        let next = line(&mut bus).flushes() + 1;
        line(&mut bus)
            .reply(&p2_status(1, 0, &[36]))
            .reply(&p2_status(2, 0x80, &[37]))
            .reply_to(next + 1, &p2_status(2, 0x80, &[0x20]));
        let mut a = [Temperature::default(); 1];
        let mut b = [Temperature::default(); 1];
        let result = bus.bulk_read((
            bulk::read(field::PresentTemperature, [id(1)], &mut a),
            bulk::read(field::PresentTemperature, [id(2)], &mut b),
        ));
        assert_eq!(
            result,
            Err(crate::error::FaultError::Overload {
                id: id(2),
                faults: HardwareErrors::from_raw(0x20),
                request: crate::error::Request::NONE
                    .instruction(crate::error::Instruction::BulkRead)
                    .id(id(2))
                    .addr(146)
                    .len(1),
            }
            .into())
        );
        assert_eq!(a[0], Temperature::default());
        assert_eq!(bus.torque_states(), [ON, FAULTED]);
    }

    #[test]
    fn bulk_calls_refuse_before_sending() {
        let time = MockTime::new();
        let mut bus = mx64s(&time, &[1, 2], 1, false);
        let mark = mark(&bus);
        let (mut a, mut b) = ([0; 1], [0; 1]);
        assert_eq!(
            bus.bulk_read((
                bulk::read(Raw(field::PresentVoltage), [id(1)], &mut a),
                bulk::read(Raw(field::PresentTemperature), [id(1)], &mut b),
            )),
            Err(ValidationError::DuplicateId { id: id(1) }.into())
        );
        assert_eq!(
            bus.bulk_read(bulk::read(
                Raw(field::PresentVoltage),
                [id(1), id(2)],
                &mut a
            )),
            Err(ValidationError::BufferLength {
                targets: 2,
                got: 1,
                request: crate::error::Request::NONE,
            }
            .into())
        );
        assert_eq!(
            bus.bulk_read(bulk::read(Raw(field::PresentVoltage), [id(9)], &mut a)),
            Err(SetupError::NotOnBus { id: id(9) }.into())
        );
        assert_eq!(
            bus.bulk_read(bulk::read(
                field::PresentLoad,
                [id(1)],
                &mut [crate::Load::default()]
            )),
            Err(ValidationError::Unsupported {
                id: id(1),
                register: "Present Load"
            }
            .into())
        );
        assert_eq!(
            bus.bulk_write((
                bulk::write(field::Led, [id(1)], &[true]),
                bulk::write(Raw(field::GoalVelocity), [id(2)], &[10]),
            )),
            Err(ValidationError::ActuatorIsOff { id: id(2) }.into())
        );
        assert_eq!(
            bus.bulk_write(bulk::write(field::Led, [id(1), id(2)], &[true])),
            Err(ValidationError::BufferLength {
                targets: 2,
                got: 1,
                request: crate::error::Request::NONE,
            }
            .into())
        );
        let mut none: [i32; 0] = [];
        assert_eq!(
            bus.bulk_read(bulk::read(Raw(field::PresentVoltage), [], &mut none)),
            Ok(())
        );
        assert_eq!(bus.bulk_write(bulk::write(field::Led, [], &[])), Ok(()));
        assert_quiet(&bus, mark);
        assert_eq!(a, [0]);
    }

    #[test]
    fn bulk_write_carries_different_fields() {
        let time = MockTime::new();
        let mut bus = mx64s(&time, &[1, 2], 1, true);
        let mark = mark(&bus);
        let reads = line(&mut bus).reads();
        let slow = velocity(&bus, -5);
        bus.bulk_write((
            bulk::write(field::Led, [id(1)], &[true]),
            bulk::write(field::GoalVelocity, [id(2)], &[slow]),
        ))
        .unwrap();
        assert_eq!(
            sent_since(&bus, mark),
            p2_bulk_write(&[(1, 65, &[1]), (2, 104, &(-5i32).to_le_bytes())])
        );
        assert_eq!(line(&mut bus).reads(), reads);
    }
}

#[cfg(feature = "pro")]
mod pro {
    use super::*;
    use crate::bus::testing::*;
    use crate::error::{Error, ValidationError};
    use crate::models::Ph54200S500;
    use crate::protocol::P2;
    use crate::transport::MockTime;

    #[test]
    fn rgb_led_is_tier2_and_unscaled_registers_are_raw_only() {
        let time = MockTime::new();
        let mut bus = test_bus::<P2, 32>(&time);
        hold(&mut bus, 2020, 1, (-1000, 1000), BOUNDED, OFF);
        let mark = mark(&bus);
        let mut handle = bus.actuator(id(1)).unwrap();
        assert_eq!(
            handle.set_led(true),
            Err(ValidationError::Unsupported {
                id: id(1),
                register: "LED"
            }
            .into())
        );
        assert!(matches!(
            handle.present_temperature(),
            Err(Error::Validation(ValidationError::NoUnit { .. }))
        ));
        assert_quiet(&bus, mark);
        let mut replies = std::vec![p2_status(1, 0, &[]); 3];
        replies.push(p2_status(1, 0, &[9]));
        script(&mut bus, &replies);
        let mut handle = bus.actuator(id(1)).unwrap();
        let mut pro = handle.model::<Ph54200S500>().unwrap();
        pro.set_led_red(255).unwrap();
        pro.set_led_green(0).unwrap();
        pro.set_led_blue(9).unwrap();
        assert_eq!(pro.led_red(), Ok(9));
        assert_eq!(
            sent_since(&bus, mark),
            concat(&[
                p2_write(1, 513, &[255]),
                p2_write(1, 514, &[0]),
                p2_write(1, 515, &[9]),
                p2_read(1, 513, 1),
            ])
        );
    }
}

#[cfg(all(feature = "x-series", feature = "std"))]
mod dyn_bus {
    use super::*;
    use crate::bus::testing::*;
    use std::vec::Vec;

    use crate::actuator::{Actuator, LimitMode, ModelNumber, TorqueState};
    use crate::bulk;
    use crate::bus::DynBus;
    use crate::error::ValidationError;
    use crate::field;
    use crate::protocol::P2;
    use crate::transport::MockTime;
    use crate::transport::MockTransport;
    use crate::units::{Current, Velocity};
    use crate::{Config, Temperature};

    #[test]
    fn dyn_bus_has_the_fr9_calls_of_bus() {
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        let next = xm430_p2(&mut line, 1, 1, 0, 4095, 1);
        let next = xm430_p2(&mut line, next, 2, 0, 4095, 1);
        line.reply_to(next, &p2_status(1, 0, &[]))
            .reply_to(next + 1, &p2_status(2, 0, &[]))
            .reply_to(next + 2, &p2_status(1, 0, &[30]))
            .reply_to(next + 2, &p2_status(2, 0, &[31]))
            .reply_to(next + 3, &p2_status(1, 0, &[32]))
            .reply_to(next + 3, &p2_status(2, 0, &[33]))
            .reply_to(next + 4, &p2_status(1, 0, &[34]))
            .reply_to(next + 4, &p2_status(2, 0, &[35]));
        let mut bus = DynBus::<P2, _, _>::new(line, Ticker(&time), Config::new());
        bus.declare([(id(1), ModelNumber(XM430)), (id(2), ModelNumber(XM430))])
            .unwrap();
        bus.enable_torque(bus.ids()).unwrap();
        assert_eq!(
            bus.actuator(id(1)).unwrap().goal_bound_raw(),
            Some(XM430_VELOCITY_LIMIT)
        );
        let mut celsius = [Temperature::default(); 2];
        bus.read_field(field::PresentTemperature, bus.ids(), &mut celsius)
            .unwrap();
        assert_eq!(celsius.map(Temperature::celsius), [30.0, 31.0]);
        bus.sync_read(field::PresentTemperature, bus.ids(), &mut celsius)
            .unwrap();
        let (mut a, mut b) = ([Temperature::default(); 1], [Temperature::default(); 1]);
        bus.bulk_read((
            bulk::read(field::PresentTemperature, [id(1)], &mut a),
            bulk::read(field::PresentTemperature, [id(2)], &mut b),
        ))
        .unwrap();
        assert_eq!((a[0].celsius(), b[0].celsius()), (34.0, 35.0));
        bus.write_goal_velocities(bus.ids(), &[Velocity::ZERO; 2])
            .unwrap();
        assert_eq!(
            bus.write_goal_currents(bus.ids(), &[Current::ZERO; 2]),
            Err(ValidationError::NotCurrentMode {
                id: id(1),
                mode: LimitMode::Velocity
            }
            .into())
        );
        bus.bulk_write(bulk::write(field::Led, [id(2)], &[true]))
            .unwrap();
        #[cfg(feature = "partial-reads")]
        {
            let mut out = [Ok(Temperature::default()); 2];
            bus.read_field_partial(field::PresentTemperature, bus.ids(), &mut out)
                .unwrap();
            assert!(out.iter().all(Result::is_err));
        }
        let (line, _) = bus.into_parts();
        let mut expected = Vec::new();
        for n in [1, 2] {
            expected.extend(p2_instruction(n, PING, &[]));
            expected.extend(xm430_p2_reads_sent_in(n, 1));
        }
        expected.extend(concat(&[
            p2_write(1, 64, &[1]),
            p2_write(2, 64, &[1]),
            p2_sync_read(146, 1, &[1, 2]),
            p2_sync_read(146, 1, &[1, 2]),
            p2_bulk_read(&[(1, 146, 1), (2, 146, 1)]),
            p2_sync_write(104, 4, &[(1, &[0; 4]), (2, &[0; 4])]),
            p2_bulk_write(&[(2, 65, &[1])]),
        ]));
        #[cfg(feature = "partial-reads")]
        expected.extend(p2_sync_read(146, 1, &[1, 2]));
        assert_eq!(line.sent(), expected);
    }

    #[test]
    fn dyn_bus_reboots_on_protocol_2_and_keeps_its_cache() {
        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        let next = xm430_p2(&mut line, 1, 1, 0, 4095, 1);
        line.reply_to(next, &p2_status(1, 0, &[]))
            .reply_to(next + 1, &p2_status(1, 0, &[]));
        let mut bus = DynBus::<P2, _, _>::new(line, Ticker(&time), Config::new());
        bus.declare([(id(1), ModelNumber(XM430))]).unwrap();
        bus.enable_torque([id(1)]).unwrap();
        let mut xm = bus.actuator(id(1)).unwrap();
        xm.reboot().unwrap();
        assert_eq!(xm.torque_state(), TorqueState::Off);
        assert_eq!(xm.limit_mode(), LimitMode::Velocity);
        assert_eq!(xm.goal_bound_raw(), Some(XM430_VELOCITY_LIMIT));
        let (line, _) = bus.into_parts();
        let mut expected = p2_instruction(1, PING, &[]);
        expected.extend(xm430_p2_reads_sent_in(1, 1));
        expected.extend(concat(&[
            p2_write(1, 64, &[1]),
            p2_instruction(1, REBOOT, &[]),
        ]));
        assert_eq!(line.sent(), expected);
    }
}
