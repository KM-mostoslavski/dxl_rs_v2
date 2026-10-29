//! A short-lived handle on one held actuator.

use crate::Transport;
use crate::actuator::{Actuator, Cache, Describe, ModelData, TorqueState};
use crate::error::Error;
use crate::models::ActuatorKind;
use crate::protocol::{Clock, Protocol};
use crate::units::Position;

use super::observe::{after_torque_write, check_clear, observe};
use super::wire::Wire;

/// One actuator of a [`Bus`](crate::Bus), borrowed from it by
/// [`Bus::actuator`](crate::Bus::actuator).
///
/// It borrows the whole bus mutably, so only one handle is live at a time,
/// and it cannot outlive the bus:
///
/// ```compile_fail,E0597
/// use dxl_rs_v2::{Bus, Clock, Config, Id, P2, Transport};
///
/// fn escape<T: Transport, C: Clock>(transport: T, clock: C, id: Id) {
///     let handle;
///     {
///         let mut bus = Bus::<P2, T, C>::new(transport, clock, Config::new());
///         handle = bus.actuator(id);
///     }
///     drop(handle);
/// }
/// ```
///
/// Every reply it receives passes the fault check: a reply reporting a
/// hardware fault returns [`Error::Status`] and marks the actuator
/// [`TorqueState::Faulted`] on that same call. Every refusal (torque off,
/// outside the limits, wrong operating mode, value too large) happens
/// before anything is sent.
///
/// It implements [`Actuator`] for what the bus read at Setup, and
/// [`kind`](ActuatorRef::kind) gives the model itself.
pub struct ActuatorRef<'b, P: Protocol, T: Transport, C: Clock> {
    kind: &'b mut ActuatorKind,
    state: &'b mut TorqueState,
    wire: Wire<'b, P, T, C>,
}

impl<'b, P: Protocol, T: Transport, C: Clock> ActuatorRef<'b, P, T, C> {
    /// A handle on `kind`, whose slot is `state`, talking over `wire`.
    pub(crate) fn new(
        kind: &'b mut ActuatorKind,
        state: &'b mut TorqueState,
        wire: Wire<'b, P, T, C>,
    ) -> Self {
        Self { kind, state, wire }
    }
}

impl<P: Protocol, T: Transport, C: Clock> ActuatorRef<'_, P, T, C> {
    /// The model, to `match` on for what only one model has.
    pub fn kind(&self) -> &ActuatorKind {
        self.kind
    }

    /// The torque state the bus holds for this actuator.
    pub fn torque_state(&self) -> TorqueState {
        *self.state
    }

    /// Turns torque on: one acknowledged write of Torque Enable = 1.
    ///
    /// # Errors
    ///
    /// - [`ValidationError::ActuatorFaulted`] while
    ///   [`Faulted`](TorqueState::Faulted), with nothing sent; call
    ///   [`clear_fault`](Self::clear_fault) first.
    /// - Any error of the write. The state is then `Off`, or `Faulted` if
    ///   the reply reported a hardware fault; never `On`.
    ///
    /// [`ValidationError::ActuatorFaulted`]: crate::ValidationError::ActuatorFaulted
    pub fn enable_torque(&mut self) -> Result<(), Error> {
        let id = self.kind.id();
        let wanted = self.state.enable(id)?;
        let result = self.wire.write_value(id, self.data().torque_enable, 1);
        after_torque_write(self.state, wanted, &result);
        result
    }

    /// Turns torque off: one acknowledged write of Torque Enable = 0, sent
    /// in every state.
    ///
    /// # Errors
    ///
    /// Any error of the write. The state then becomes `Off`, except that a
    /// `Faulted` actuator stays `Faulted` (the fault is never lost) and a
    /// reply reporting a hardware fault gives `Faulted`.
    pub fn disable_torque(&mut self) -> Result<(), Error> {
        let id = self.kind.id();
        let wanted = self.state.disable();
        let result = self.wire.write_value(id, self.data().torque_enable, 0);
        after_torque_write(self.state, wanted, &result);
        result
    }

    /// Sends goal position `goal`, in radians: one acknowledged write of
    /// Goal Position, no read-back.
    ///
    /// The angle is converted to the nearest tick, then checked like
    /// [`set_goal_position_raw`](Self::set_goal_position_raw).
    ///
    /// Limits and operating mode are those read at Setup. Position mode
    /// applies the EEPROM limits. Extended and current-based position modes
    /// and MX multi-turn apply none, as the firmware does; only the register
    /// size is checked. Velocity, current, PWM and wheel modes refuse every
    /// goal (`NotPositionMode`). A mode changed after Setup is not seen until
    /// the next Setup.
    ///
    /// # Errors
    ///
    /// Refused with nothing sent:
    /// - [`ActuatorIsOff`]: torque is not [`On`](TorqueState::On).
    /// - [`NotFinite`]: `goal` is NaN or infinite.
    /// - [`OutOfLimits`]: position mode, and the tick is outside the limits.
    /// - [`NotPositionMode`]: the operating mode ignores goal positions.
    /// - [`DoesNotFit`]: the tick does not fit the register.
    ///
    /// Otherwise any error of the write.
    ///
    /// [`ActuatorIsOff`]: crate::ValidationError::ActuatorIsOff
    /// [`NotFinite`]: crate::ValidationError::NotFinite
    /// [`OutOfLimits`]: crate::ValidationError::OutOfLimits
    /// [`NotPositionMode`]: crate::ValidationError::NotPositionMode
    /// [`DoesNotFit`]: crate::ValidationError::DoesNotFit
    pub fn set_goal_position(&mut self, goal: Position) -> Result<(), Error> {
        self.state.check_goal(self.kind.id())?;
        let raw = self.data().scale.to_raw(goal)?;
        self.send_goal(raw)
    }

    /// Sends goal position `raw`, in ticks: one acknowledged write of Goal
    /// Position, no read-back.
    ///
    /// Limits and operating mode are those read at Setup. Position mode
    /// applies the EEPROM limits. Extended and current-based position modes
    /// and MX multi-turn apply none, as the firmware does; only the register
    /// size is checked. Velocity, current, PWM and wheel modes refuse every
    /// goal (`NotPositionMode`). A mode changed after Setup is not seen until
    /// the next Setup.
    ///
    /// # Errors
    ///
    /// Refused with nothing sent:
    /// - [`ActuatorIsOff`]: torque is not [`On`](TorqueState::On).
    /// - [`OutOfLimits`]: position mode, and `raw` is outside the limits.
    /// - [`NotPositionMode`]: the operating mode ignores goal positions.
    /// - [`DoesNotFit`]: `raw` does not fit the register.
    ///
    /// Otherwise any error of the write.
    ///
    /// [`ActuatorIsOff`]: crate::ValidationError::ActuatorIsOff
    /// [`OutOfLimits`]: crate::ValidationError::OutOfLimits
    /// [`NotPositionMode`]: crate::ValidationError::NotPositionMode
    /// [`DoesNotFit`]: crate::ValidationError::DoesNotFit
    pub fn set_goal_position_raw(&mut self, raw: i32) -> Result<(), Error> {
        self.state.check_goal(self.kind.id())?;
        self.send_goal(raw)
    }

    /// The limit-mode check, then the write. The torque guard has passed.
    fn send_goal(&mut self, raw: i32) -> Result<(), Error> {
        let raw = self.kind.cache().check_goal(raw)?;
        let id = self.kind.id();
        let result = self.wire.write_value(id, self.data().goal_position, raw);
        observe(self.state, &result);
        result
    }

    /// Reads Goal Position back from the actuator, in radians.
    ///
    /// # Errors
    ///
    /// Any error of the read.
    pub fn goal_position(&mut self) -> Result<Position, Error> {
        let raw = self.goal_position_raw()?;
        Ok(self.data().scale.to_position(raw))
    }

    /// Reads Goal Position back from the actuator, in ticks.
    ///
    /// # Errors
    ///
    /// Any error of the read.
    pub fn goal_position_raw(&mut self) -> Result<i32, Error> {
        self.read(self.data().goal_position)
    }

    /// Reads Present Position, in radians.
    ///
    /// # Errors
    ///
    /// Any error of the read.
    pub fn present_position(&mut self) -> Result<Position, Error> {
        let raw = self.present_position_raw()?;
        Ok(self.data().scale.to_position(raw))
    }

    /// Reads Present Position, in ticks.
    ///
    /// # Errors
    ///
    /// Any error of the read.
    pub fn present_position_raw(&mut self) -> Result<i32, Error> {
        self.read(self.data().present_position)
    }

    /// Reads and decodes `reg`; a hardware fault in the reply marks the
    /// actuator `Faulted`.
    fn read(&mut self, reg: crate::register::Register) -> Result<i32, Error> {
        let result = self.wire.read_value(self.kind.id(), reg);
        observe(self.state, &result);
        result
    }

    /// Asks the actuator (Hardware Error Status, or a ping where the model
    /// has none). Clears to `Off` only if it reports no fault; otherwise
    /// returns its `Status` error.
    ///
    /// The check is sent in every state. On success the state is `Off` if it
    /// was `Faulted`, and unchanged otherwise; torque stays off until
    /// [`enable_torque`](Self::enable_torque).
    ///
    /// An X-series actuator (and any model with Hardware Error Status) keeps
    /// reporting its fault until it is rebooted, which this crate does not
    /// do: reboot it (or power-cycle it), then call `clear_fault`.
    ///
    /// # Errors
    ///
    /// - [`Error::Status`] while the actuator still reports a fault: the
    ///   reply's own error, or, when Hardware Error Status reads non-zero, a
    ///   report with the alert bit set. The state stays `Faulted`.
    /// - Any other error of the exchange; the state does not change.
    pub fn clear_fault(&mut self) -> Result<(), Error> {
        check_clear(&mut self.wire, self.kind, self.state)
    }
}

impl<P: Protocol, T: Transport, C: Clock> Describe for ActuatorRef<'_, P, T, C> {
    fn data(&self) -> &'static ModelData {
        self.kind.data()
    }

    fn cache(&self) -> &Cache {
        self.kind.cache()
    }

    fn cache_mut(&mut self) -> &mut Cache {
        self.kind.cache_mut()
    }
}

/// `id`, `model_number`, `name`, `position_scale` and
/// `position_limits[_raw]` of the held actuator, as read at Setup.
impl<P: Protocol, T: Transport, C: Clock> Actuator for ActuatorRef<'_, P, T, C> {}

#[cfg(test)]
mod tests {

    #[cfg(feature = "x-series")]
    mod x_series {
        extern crate std;

        use super::super::*;
        use crate::actuator::ModelNumber;
        use crate::bus::testing::*;
        use crate::error::{StatusError, TransportError, ValidationError};
        use crate::protocol::P2;
        use crate::transport::MockTime;

        /// An XM430-W350 at ID 1, limits 1000..=3000, in Operating Mode
        /// `mode`, torque off.
        fn xm430(time: &MockTime, mode: u8) -> TestBus<'_, P2> {
            let mut bus = test_bus(time);
            xm430_p2(line(&mut bus), 1, 1, 1000, 3000, mode);
            bus.declare([(id(1), ModelNumber(XM430))]).unwrap();
            bus
        }

        /// The same, with torque on.
        fn xm430_on(time: &MockTime, mode: u8) -> TestBus<'_, P2> {
            let mut bus = xm430(time, mode);
            line(&mut bus).reply(&p2_status(1, 0, &[]));
            bus.actuator(id(1)).unwrap().enable_torque().unwrap();
            bus
        }

        fn ack(bus: &mut TestBus<'_, P2>) {
            line(bus).reply(&p2_status(1, 0, &[]));
        }

        fn goal_write(raw: i32) -> std::vec::Vec<u8> {
            p2_write(1, 116, &raw.to_le_bytes())
        }

        #[test]
        fn goal_while_off_is_refused() {
            let time = MockTime::new();
            let mut bus = xm430(&time, 3);
            let mark = mark(&bus);
            assert_eq!(
                bus.actuator(id(1)).unwrap().set_goal_position_raw(2000),
                Err(ValidationError::ActuatorIsOff { id: id(1) }.into())
            );
            assert_quiet(&bus, mark);
        }

        #[test]
        fn goal_outside_limits_is_refused() {
            let time = MockTime::new();
            let mut bus = xm430_on(&time, 3);
            let mark = mark(&bus);
            for raw in [999, 3001] {
                assert_eq!(
                    bus.actuator(id(1)).unwrap().set_goal_position_raw(raw),
                    Err(ValidationError::OutOfLimits {
                        id: id(1),
                        raw,
                        min: 1000,
                        max: 3000
                    }
                    .into())
                );
            }
            assert_quiet(&bus, mark);
            for raw in [1000, 3000] {
                ack(&mut bus);
                assert_eq!(
                    bus.actuator(id(1)).unwrap().set_goal_position_raw(raw),
                    Ok(())
                );
            }
            assert_eq!(
                sent_since(&bus, mark),
                concat(&[goal_write(1000), goal_write(3000)])
            );
        }

        #[test]
        fn goal_inside_limits_sends_one_write() {
            let time = MockTime::new();
            let mut bus = xm430_on(&time, 3);
            let mark = mark(&bus);
            ack(&mut bus);
            bus.actuator(id(1))
                .unwrap()
                .set_goal_position_raw(2000)
                .unwrap();
            assert_eq!(sent_since(&bus, mark), goal_write(2000));
            assert_done(&bus);
        }

        #[test]
        fn enable_torque_sends_one_acknowledged_write() {
            let time = MockTime::new();
            let mut bus = xm430(&time, 3);
            let mark = mark(&bus);
            ack(&mut bus);
            let mut xm = bus.actuator(id(1)).unwrap();
            xm.enable_torque().unwrap();
            assert_eq!(xm.torque_state(), TorqueState::On);
            assert_eq!(sent_since(&bus, mark), p2_write(1, 64, &[1]));
            assert_eq!(bus.torque_state(id(1)), Some(TorqueState::On));
        }

        #[test]
        fn disable_torque_turns_off() {
            let time = MockTime::new();
            let mut bus = xm430_on(&time, 3);
            let mark = mark(&bus);
            ack(&mut bus);
            bus.actuator(id(1)).unwrap().disable_torque().unwrap();
            assert_eq!(sent_since(&bus, mark), p2_write(1, 64, &[0]));
            assert_eq!(bus.torque_state(id(1)), Some(TorqueState::Off));
        }

        #[test]
        fn present_position_raw_decodes() {
            let time = MockTime::new();
            let mut bus = xm430(&time, 4);
            let mark = mark(&bus);
            line(&mut bus).reply(&p2_status(1, 0, &(-300i32).to_le_bytes()));
            assert_eq!(
                bus.actuator(id(1)).unwrap().present_position_raw(),
                Ok(-300)
            );
            assert_eq!(sent_since(&bus, mark), p2_read(1, 132, 4));
        }

        #[test]
        fn goal_position_raw_reads_back() {
            let time = MockTime::new();
            let mut bus = xm430(&time, 3);
            let mark = mark(&bus);
            line(&mut bus).reply(&p2_status(1, 0, &1500i32.to_le_bytes()));
            assert_eq!(bus.actuator(id(1)).unwrap().goal_position_raw(), Ok(1500));
            assert_eq!(sent_since(&bus, mark), p2_read(1, 116, 4));
        }

        #[test]
        fn extended_mode_accepts_goal_outside_limits() {
            let time = MockTime::new();
            let mut bus = xm430_on(&time, 4);
            let mark = mark(&bus);
            ack(&mut bus);
            bus.actuator(id(1))
                .unwrap()
                .set_goal_position_raw(-5000)
                .unwrap();
            assert_eq!(sent_since(&bus, mark), goal_write(-5000));
        }

        #[test]
        fn velocity_mode_is_not_position_mode() {
            let time = MockTime::new();
            let mut bus = xm430_on(&time, 1);
            let mark = mark(&bus);
            assert_eq!(
                bus.actuator(id(1)).unwrap().set_goal_position_raw(2000),
                Err(ValidationError::NotPositionMode { id: id(1) }.into())
            );
            assert_quiet(&bus, mark);
        }

        #[test]
        fn alert_on_present_read_faults() {
            let time = MockTime::new();
            let mut bus = xm430_on(&time, 3);
            line(&mut bus).reply(&p2_status(1, 0x80, &2000i32.to_le_bytes()));
            let mut xm = bus.actuator(id(1)).unwrap();
            assert!(matches!(
                xm.present_position_raw(),
                Err(Error::Status(StatusError { .. }))
            ));
            assert_eq!(xm.torque_state(), TorqueState::Faulted);
        }

        #[test]
        fn hardware_fault_on_torque_write_faults() {
            let time = MockTime::new();
            let mut bus = xm430(&time, 3);
            line(&mut bus).reply(&p2_status(1, 0x80, &[]));
            let mut xm = bus.actuator(id(1)).unwrap();
            assert!(matches!(xm.enable_torque(), Err(Error::Status(_))));
            assert_eq!(xm.torque_state(), TorqueState::Faulted);
        }

        fn faulted(time: &MockTime) -> TestBus<'_, P2> {
            let mut bus = xm430_on(time, 3);
            line(&mut bus).reply(&p2_status(1, 0x80, &2000i32.to_le_bytes()));
            let _ = bus.actuator(id(1)).unwrap().present_position_raw();
            assert_eq!(bus.torque_state(id(1)), Some(TorqueState::Faulted));
            bus
        }

        #[test]
        fn goal_while_faulted_is_refused() {
            let time = MockTime::new();
            let mut bus = faulted(&time);
            let mark = mark(&bus);
            assert_eq!(
                bus.actuator(id(1)).unwrap().set_goal_position_raw(2000),
                Err(ValidationError::ActuatorIsOff { id: id(1) }.into())
            );
            assert_eq!(
                bus.actuator(id(1))
                    .unwrap()
                    .set_goal_position(Position::ZERO),
                Err(ValidationError::ActuatorIsOff { id: id(1) }.into())
            );
            assert_quiet(&bus, mark);
        }

        #[test]
        fn enable_while_faulted_is_refused() {
            let time = MockTime::new();
            let mut bus = faulted(&time);
            let mark = mark(&bus);
            assert_eq!(
                bus.actuator(id(1)).unwrap().enable_torque(),
                Err(ValidationError::ActuatorFaulted { id: id(1) }.into())
            );
            assert_quiet(&bus, mark);
        }

        #[test]
        fn enable_timeout_leaves_off() {
            let time = MockTime::new();
            let mut bus = xm430(&time, 3);
            let mut xm = bus.actuator(id(1)).unwrap();
            assert!(matches!(
                xm.enable_torque(),
                Err(Error::Transport(TransportError::Timeout { .. }))
            ));
            assert_eq!(xm.torque_state(), TorqueState::Off);

            let mut bus = xm430_on(&time, 3);
            let mut xm = bus.actuator(id(1)).unwrap();
            assert!(xm.enable_torque().is_err());
            assert_eq!(xm.torque_state(), TorqueState::Off);
        }

        #[test]
        fn enable_bad_crc_leaves_off() {
            let time = MockTime::new();
            let mut bus = xm430(&time, 3);
            let mut reply = p2_status(1, 0, &[]);
            let last = reply.len() - 1;
            reply[last] ^= 0x55;
            line(&mut bus).reply(&reply);
            let mut xm = bus.actuator(id(1)).unwrap();
            assert!(matches!(xm.enable_torque(), Err(Error::Protocol(_))));
            assert_eq!(xm.torque_state(), TorqueState::Off);
        }

        #[test]
        fn faulted_disable_timeout_stays_faulted() {
            let time = MockTime::new();
            let mut bus = faulted(&time);
            let mut xm = bus.actuator(id(1)).unwrap();
            assert!(xm.disable_torque().is_err());
            assert_eq!(xm.torque_state(), TorqueState::Faulted);
            let mark = mark(&bus);
            assert_eq!(
                bus.actuator(id(1)).unwrap().enable_torque(),
                Err(ValidationError::ActuatorFaulted { id: id(1) }.into())
            );
            assert_quiet(&bus, mark);
        }

        #[test]
        fn faulted_disable_ok_stays_faulted() {
            let time = MockTime::new();
            let mut bus = faulted(&time);
            ack(&mut bus);
            bus.actuator(id(1)).unwrap().disable_torque().unwrap();
            assert_eq!(bus.torque_state(id(1)), Some(TorqueState::Faulted));
        }

        #[test]
        fn typed_goal_is_the_register_value() {
            let time = MockTime::new();
            let mut bus = xm430_on(&time, 4);
            let scale = bus.actuator(id(1)).unwrap().position_scale();
            let mark = mark(&bus);
            for goal in [Position::ZERO, Position::from_radians(0.5)] {
                ack(&mut bus);
                bus.actuator(id(1))
                    .unwrap()
                    .set_goal_position(goal)
                    .unwrap();
            }
            let half = scale.to_raw(Position::from_radians(0.5)).unwrap();
            assert_eq!(
                sent_since(&bus, mark),
                concat(&[goal_write(2048), goal_write(half)])
            );
        }

        #[test]
        fn typed_goal_rounding_outside_is_refused() {
            let time = MockTime::new();
            let mut bus = xm430_on(&time, 3);
            let mark = mark(&bus);
            let mut xm = bus.actuator(id(1)).unwrap();
            let outside = xm.position_scale().to_position(3001);
            assert_eq!(
                xm.set_goal_position(outside),
                Err(ValidationError::OutOfLimits {
                    id: id(1),
                    raw: 3001,
                    min: 1000,
                    max: 3000
                }
                .into())
            );
            assert_eq!(
                xm.set_goal_position(Position::from_radians(f32::NAN)),
                Err(ValidationError::NotFinite.into())
            );
            assert_quiet(&bus, mark);
        }

        #[test]
        fn typed_goal_follows_limit_mode() {
            let time = MockTime::new();
            let mut bus = xm430_on(&time, 4);
            let scale = bus.actuator(id(1)).unwrap().position_scale();
            let before = mark(&bus);
            ack(&mut bus);
            bus.actuator(id(1))
                .unwrap()
                .set_goal_position(scale.to_position(5000))
                .unwrap();
            assert_eq!(sent_since(&bus, before), goal_write(5000));

            let mut bus = xm430_on(&time, 1);
            let quiet = mark(&bus);
            assert_eq!(
                bus.actuator(id(1))
                    .unwrap()
                    .set_goal_position(Position::ZERO),
                Err(ValidationError::NotPositionMode { id: id(1) }.into())
            );
            assert_quiet(&bus, quiet);
        }

        #[test]
        fn present_position_typed_and_raw() {
            let time = MockTime::new();
            let mut bus = xm430(&time, 3);
            let reply = p2_status(1, 0, &2500i32.to_le_bytes());
            line(&mut bus).reply(&reply);
            let raw = bus.actuator(id(1)).unwrap().present_position_raw().unwrap();
            line(&mut bus).reply(&reply);
            let mut xm = bus.actuator(id(1)).unwrap();
            let typed = xm.present_position().unwrap();
            assert_eq!(raw, 2500);
            assert_eq!(typed, xm.position_scale().to_position(2500));
        }

        #[test]
        fn goal_position_typed_reads_back() {
            let time = MockTime::new();
            let mut bus = xm430(&time, 3);
            line(&mut bus).reply(&p2_status(1, 0, &2048i32.to_le_bytes()));
            assert_eq!(
                bus.actuator(id(1)).unwrap().goal_position(),
                Ok(Position::ZERO)
            );
        }

        #[test]
        fn limits_typed_and_raw_agree() {
            let time = MockTime::new();
            let mut bus = xm430(&time, 3);
            let xm = bus.actuator(id(1)).unwrap();
            assert_eq!(xm.position_limits_raw(), Some(1000..=3000));
            let scale = xm.position_scale();
            assert_eq!(
                xm.position_limits(),
                Some(scale.to_position(1000)..=scale.to_position(3000))
            );
            assert_eq!(xm.id(), id(1));
            assert_eq!(xm.name(), "XM430-W350");
            assert_eq!(xm.model_number(), XM430);
            assert!(matches!(xm.kind(), ActuatorKind::Xm430W350(_)));
        }

        #[test]
        fn handle_cache_is_the_held_cache() {
            let time = MockTime::new();
            let mut bus = xm430(&time, 3);
            let mut xm = bus.actuator(id(1)).unwrap();
            assert_eq!(xm.cache().limits(), xm.kind().cache().limits());
            xm.cache_mut().record_limits(0, 10).unwrap();
            assert_eq!(bus.actuators()[0].position_limits_raw(), Some(0..=10));
        }
    }

    #[cfg(feature = "ax")]
    mod ax {
        use super::super::*;
        use crate::actuator::ModelNumber;
        use crate::bus::testing::*;
        use crate::error::ValidationError;
        use crate::protocol::P1;
        use crate::transport::MockTime;

        fn ax12_on(time: &MockTime, cw: u16, ccw: u16) -> TestBus<'_, P1> {
            let mut bus = test_bus(time);
            ax12_p1(line(&mut bus), 1, 1, cw, ccw);
            bus.declare([(id(1), ModelNumber(AX12))]).unwrap();
            line(&mut bus).reply(&p1_status(1, 0, &[]));
            bus.actuator(id(1)).unwrap().enable_torque().unwrap();
            bus
        }

        #[test]
        fn ax_wheel_mode_is_not_position_mode() {
            let time = MockTime::new();
            let mut bus = ax12_on(&time, 0, 0);
            let mark = mark(&bus);
            assert_eq!(
                bus.actuator(id(1)).unwrap().set_goal_position_raw(512),
                Err(ValidationError::NotPositionMode { id: id(1) }.into())
            );
            assert_quiet(&bus, mark);
        }

        #[test]
        fn p1_overload_on_present_read_faults() {
            let time = MockTime::new();
            let mut bus = ax12_on(&time, 0, 1023);
            line(&mut bus).reply(&p1_status(1, 0x20, &512u16.to_le_bytes()));
            let mut ax = bus.actuator(id(1)).unwrap();
            assert!(matches!(ax.present_position_raw(), Err(Error::Status(_))));
            assert_eq!(ax.torque_state(), TorqueState::Faulted);
        }

        #[test]
        fn angle_limit_flag_is_not_a_fault() {
            let time = MockTime::new();
            let mut bus = ax12_on(&time, 0, 1023);
            line(&mut bus).reply(&p1_status(1, 0x02, &512u16.to_le_bytes()));
            let mut ax = bus.actuator(id(1)).unwrap();
            assert!(matches!(ax.present_position_raw(), Err(Error::Status(_))));
            assert_eq!(ax.torque_state(), TorqueState::On);
        }

        #[test]
        fn typed_goal_is_the_register_value() {
            let time = MockTime::new();
            let mut bus = ax12_on(&time, 0, 1023);
            let scale = bus.actuator(id(1)).unwrap().position_scale();
            let mark = mark(&bus);
            for goal in [Position::ZERO, Position::from_radians(-1.0)] {
                line(&mut bus).reply(&p1_status(1, 0, &[]));
                bus.actuator(id(1))
                    .unwrap()
                    .set_goal_position(goal)
                    .unwrap();
            }
            let raw = u16::try_from(scale.to_raw(Position::from_radians(-1.0)).unwrap()).unwrap();
            assert_eq!(
                sent_since(&bus, mark),
                concat(&[
                    p1_write(1, 30, &512u16.to_le_bytes()),
                    p1_write(1, 30, &raw.to_le_bytes())
                ])
            );
        }
    }

    #[cfg(feature = "rx")]
    mod rx {
        use super::super::*;
        use crate::actuator::ModelNumber;
        use crate::bus::testing::*;
        use crate::protocol::P1;
        use crate::transport::MockTime;

        #[test]
        fn rx28_moves_like_an_ax() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            angle_p1(line(&mut bus), 1, 7, RX28, 0, 1023);
            bus.declare([(id(7), ModelNumber(RX28))]).unwrap();
            let mark = mark(&bus);
            line(&mut bus).reply(&p1_status(7, 0, &[]));
            let mut rx = bus.actuator(id(7)).unwrap();
            rx.enable_torque().unwrap();
            line(&mut bus).reply(&p1_status(7, 0, &[]));
            let mut rx = bus.actuator(id(7)).unwrap();
            rx.set_goal_position(Position::ZERO).unwrap();
            line(&mut bus).reply(&p1_status(7, 0, &600u16.to_le_bytes()));
            assert_eq!(bus.actuator(id(7)).unwrap().present_position_raw(), Ok(600));
            assert_eq!(
                sent_since(&bus, mark),
                concat(&[
                    p1_write(7, 24, &[1]),
                    p1_write(7, 30, &512u16.to_le_bytes()),
                    p1_read(7, 36, 2),
                ])
            );
        }
    }
}
