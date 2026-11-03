//! A short-lived handle on one held actuator.

use crate::Transport;
use crate::actuator::{Actuator, Cache, Describe, ModelData, TorqueState};
use crate::error::{Error, StatusError, StatusReport};
use crate::field::{
    DGain, GoalCurrent, GoalPosition, GoalVelocity, HardwareErrorStatus, Led, Moving, PGain,
    PresentCurrent, PresentLoad, PresentPosition, PresentTemperature, PresentVelocity,
    PresentVoltage, Raw, ReadField, RealtimeTick, SpeedLimit as SpeedLimitField, WriteField,
};
use crate::models::ActuatorKind;
use crate::protocol::{Clock, P2, Protocol};
use crate::units::{
    Current, Gain, HardwareErrors, Load, Position, SpeedLimit, Temperature, Tick, Velocity, Voltage,
};

use super::field::{self as fields, sealed::Decode};
use super::observe::{after_torque_write, check_clear, observe};
use super::slots::Bound;
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
#[cfg_attr(
    not(feature = "raw-registers"),
    doc = r#"
Reading or writing a register by address needs the `raw-registers`
feature:

```compile_fail,E0599
use dxl_rs_v2::{ActuatorRef, Clock, P2, Transport};

fn peek<T: Transport, C: Clock>(xm: &mut ActuatorRef<'_, P2, T, C>) {
    let mut out = [0; 4];
    let _ = xm.read_raw(132, &mut out);
}
```

```compile_fail,E0599
use dxl_rs_v2::{ActuatorRef, Clock, P2, Transport};

fn poke<T: Transport, C: Clock>(xm: &mut ActuatorRef<'_, P2, T, C>) {
    let _ = xm.write_raw(116, &[0x00, 0x08, 0x00, 0x00]);
}
```
"#
)]
///
/// Beside position it reads and writes the typed registers of FR-9:
/// temperature, voltage, velocity, load, current, LED, hardware errors,
/// realtime tick, speed limit and gains, each in SI units through the
/// model's own scale. [`read`](ActuatorRef::read) and
/// [`write`](ActuatorRef::write) take any [`field`](crate::field), and
/// [`Raw`](crate::field::Raw) of it for the raw register value. A model
/// whose table lacks the register refuses the call with
/// [`ValidationError::Unsupported`](crate::ValidationError::Unsupported)
/// and sends nothing; [`model`](ActuatorRef::model) gives the Tier 2
/// handle, where such a call does not compile.
#[cfg_attr(
    all(feature = "mock", feature = "rx"),
    doc = r#"
An RX-28 on Protocol 1, its temperature and load read with torque off:

```
use dxl_rs_v2::{Bus, Config, Error, Id, MockTime, MockTransport, ModelNumber, P1};

# let time = MockTime::new();
# let mut line = MockTransport::new(&time);
# line.reply_to(1, &[0xFF, 0xFF, 0x01, 0x02, 0x00, 0xFC])
#     .reply_to(2, &[0xFF, 0xFF, 0x01, 0x04, 0x00, 0x1C, 0x00, 0xDE])
#     .reply_to(3, &[0xFF, 0xFF, 0x01, 0x04, 0x00, 0x00, 0x00, 0xFA])
#     .reply_to(4, &[0xFF, 0xFF, 0x01, 0x04, 0x00, 0xFF, 0x03, 0xF8])
#     .reply_to(5, &[0xFF, 0xFF, 0x01, 0x03, 0x00, 0x20, 0xDB])
#     .reply_to(6, &[0xFF, 0xFF, 0x01, 0x04, 0x00, 0x64, 0x04, 0x92]);
let mut bus = Bus::<P1, _, _>::new(line, time.ticker(10), Config::new());
let id = Id::new(1)?;
bus.declare([(id, ModelNumber(28))])?;
let mut rx = bus.actuator(id).ok_or(Error::Setup(dxl_rs_v2::SetupError::NotOnBus { id }))?;
assert_eq!(rx.present_temperature()?.celsius(), 32.0);
let load = rx.present_load()?.ratio();
assert!(load < 0.0, "bit 10 set: CW load");
# let (line, _) = bus.into_parts();
# line.assert_done();
# Ok::<(), Error>(())
```
"#
)]
pub struct ActuatorRef<'b, P: Protocol, T: Transport, C: Clock> {
    pub(super) kind: &'b mut ActuatorKind,
    pub(super) state: &'b mut TorqueState,
    pub(super) bound: Bound,
    pub(super) wire: Wire<'b, P, T, C>,
}

impl<'b, P: Protocol, T: Transport, C: Clock> ActuatorRef<'b, P, T, C> {
    /// A handle on `kind`, whose slot is `state` with goal bound `bound`,
    /// talking over `wire`.
    pub(crate) fn new(
        kind: &'b mut ActuatorKind,
        state: &'b mut TorqueState,
        bound: Bound,
        wire: Wire<'b, P, T, C>,
    ) -> Self {
        Self {
            kind,
            state,
            bound,
            wire,
        }
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

    /// The EEPROM bound Setup read for the goal this actuator's operating
    /// mode uses, in raw register units: Velocity Limit in velocity mode,
    /// Current Limit in current and current-based position mode. `None` in
    /// the other modes and on models whose table records no bound (AX, RX,
    /// MX on Protocol 1). Goal velocities and currents are checked against
    /// it (FR-9.3).
    pub fn goal_bound_raw(&self) -> Option<u32> {
        self.bound
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

    /// Reads field `F`: one unicast read of its register, converted with the
    /// model's scale in the operating mode Setup recorded. `Raw(F)` gives
    /// the register value.
    ///
    /// ```
    /// use dxl_rs_v2::field::{PresentTemperature, Raw};
    /// use dxl_rs_v2::{ActuatorRef, Clock, Error, P2, Temperature, Transport};
    ///
    /// fn temperature<T: Transport, C: Clock>(
    ///     xm: &mut ActuatorRef<'_, P2, T, C>,
    /// ) -> Result<(Temperature, i32), Error> {
    ///     Ok((xm.read(PresentTemperature)?, xm.read(Raw(PresentTemperature))?))
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Refused with nothing sent:
    /// - [`Unsupported`]: the model's table lacks the register.
    /// - [`NoUnit`]: a typed value, and the table gives the register no
    ///   unit in this mode (AX wheel-mode Present Speed is in %).
    ///
    /// Otherwise any error of the read; a reply reporting a hardware fault
    /// marks the actuator `Faulted`. A read of
    /// [`HardwareErrorStatus`](crate::field::HardwareErrorStatus) returns
    /// the flags such a reply carries instead of its error, as
    /// [`hardware_errors`](Self::hardware_errors) says.
    ///
    /// [`Unsupported`]: crate::ValidationError::Unsupported
    /// [`NoUnit`]: crate::ValidationError::NoUnit
    pub fn read<F: ReadField>(&mut self, field: F) -> Result<F::Value, Error> {
        let _ = field;
        let site = fields::read_site::<F>(self.kind)?;
        let raw = if F::FAULT_REPORT {
            self.read_fault_report(site.reg)?
        } else {
            self.read_reg(site.reg)?
        };
        Ok(F::Value::from_raw(raw, &site)?)
    }

    /// Writes field `F`: one acknowledged write of its register, no
    /// read-back. `Raw(F)` writes the register value.
    ///
    /// Everything is checked before a byte is sent, in this order: the
    /// model has the register; torque is on, for a goal (position,
    /// velocity, current); the operating mode Setup recorded uses the goal;
    /// the value converts to a register value; it lies in the limits (the
    /// EEPROM position limits, Velocity Limit or Current Limit read at
    /// Setup, or the e-Manual range); it fits the register. LED, speed
    /// limit, gains and compliance do not move the actuator and are
    /// written with torque off too (FR-9.3). See the table in
    /// [`field`](crate::field).
    ///
    /// # Errors
    ///
    /// Refused with nothing sent:
    /// - [`Unsupported`]: the model's table lacks the register.
    /// - [`ActuatorIsOff`]: a goal while torque is not
    ///   [`On`](TorqueState::On).
    /// - [`NotPositionMode`], [`NotVelocityMode`], [`NotCurrentMode`]: the
    ///   operating mode ignores the goal.
    /// - [`NoUnit`], [`NotFinite`], [`DoesNotFit`]: the value maps to no
    ///   register value.
    /// - [`OutOfLimits`]: outside the limits that apply.
    ///
    /// Otherwise any error of the write.
    ///
    /// [`Unsupported`]: crate::ValidationError::Unsupported
    /// [`ActuatorIsOff`]: crate::ValidationError::ActuatorIsOff
    /// [`NotPositionMode`]: crate::ValidationError::NotPositionMode
    /// [`NotVelocityMode`]: crate::ValidationError::NotVelocityMode
    /// [`NotCurrentMode`]: crate::ValidationError::NotCurrentMode
    /// [`NoUnit`]: crate::ValidationError::NoUnit
    /// [`NotFinite`]: crate::ValidationError::NotFinite
    /// [`DoesNotFit`]: crate::ValidationError::DoesNotFit
    /// [`OutOfLimits`]: crate::ValidationError::OutOfLimits
    pub fn write<F: WriteField>(&mut self, field: F, value: F::Value) -> Result<(), Error> {
        let _ = field;
        let id = self.kind.id();
        let mut buf = [0; 4];
        let (reg, bytes) = fields::encode_write::<F>(
            self.kind,
            self.state.check_goal(id),
            self.bound,
            value,
            &mut buf,
        )?;
        let result = self.wire.write(id, reg, bytes);
        observe(self.state, &result);
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
        self.write(GoalPosition, goal)
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
        self.write(Raw(GoalPosition), raw)
    }

    /// Reads Goal Position back from the actuator, in radians.
    ///
    /// # Errors
    ///
    /// Any error of the read.
    pub fn goal_position(&mut self) -> Result<Position, Error> {
        self.read(GoalPosition)
    }

    /// Reads Goal Position back from the actuator, in ticks.
    ///
    /// # Errors
    ///
    /// Any error of the read.
    pub fn goal_position_raw(&mut self) -> Result<i32, Error> {
        self.read(Raw(GoalPosition))
    }

    /// Reads Present Position, in radians.
    ///
    /// # Errors
    ///
    /// Any error of the read.
    pub fn present_position(&mut self) -> Result<Position, Error> {
        self.read(PresentPosition)
    }

    /// Reads Present Position, in ticks.
    ///
    /// # Errors
    ///
    /// Any error of the read.
    pub fn present_position_raw(&mut self) -> Result<i32, Error> {
        self.read(Raw(PresentPosition))
    }

    /// Sends goal velocity `goal`, in rad/s: one acknowledged write of Goal
    /// Velocity, or of Moving Speed in wheel mode (Protocol 1). No
    /// read-back. Positive is CCW.
    ///
    /// Only velocity mode (Protocol 2) and wheel mode (Protocol 1, CW = CCW
    /// Angle Limit = 0) use it (FR-9.4), and only with torque on. The
    /// magnitude must not pass the Velocity Limit read at Setup. AX and RX
    /// give wheel-mode Moving Speed in % of output, not as a velocity, so
    /// they take only `write(Raw(GoalVelocity), raw)` (bit 10 the
    /// direction).
    ///
    /// # Errors
    ///
    /// As [`write`](Self::write): `Unsupported`, `ActuatorIsOff`,
    /// `NotVelocityMode`, `NoUnit`, `NotFinite`, `DoesNotFit`,
    /// `OutOfLimits`, then any error of the write.
    pub fn set_goal_velocity(&mut self, goal: Velocity) -> Result<(), Error> {
        self.write(GoalVelocity, goal)
    }

    /// Sends goal current `goal`, in amperes: one acknowledged write of Goal
    /// Current, or of Goal Torque (MX on Protocol 1). No read-back.
    ///
    /// Only current mode and current-based position mode use it (FR-9.4;
    /// MX-64 on Protocol 1 with Torque Ctrl Mode Enable on), and only with
    /// torque on. The magnitude must not pass the Current Limit read at
    /// Setup. In current-based position mode it limits the current of the
    /// position goal.
    ///
    /// # Errors
    ///
    /// As [`write`](Self::write): `Unsupported`, `ActuatorIsOff`,
    /// `NotCurrentMode`, `NoUnit`, `NotFinite`, `DoesNotFit`,
    /// `OutOfLimits`, then any error of the write.
    pub fn set_goal_current(&mut self, goal: Current) -> Result<(), Error> {
        self.write(GoalCurrent, goal)
    }

    /// Sets the speed limit: Profile Velocity, or Moving Speed in joint and
    /// multi-turn mode (Protocol 1). One acknowledged write, allowed with
    /// torque off; it moves nothing by itself. [`SpeedLimit::Unlimited`]
    /// is raw 0 where the table says 0 means no limit.
    ///
    /// # Errors
    ///
    /// As [`write`](Self::write). `NotPositionMode` on Protocol 1 in wheel
    /// mode, where Moving Speed is the goal velocity: use
    /// [`set_goal_velocity`](Self::set_goal_velocity) there.
    pub fn set_speed_limit(&mut self, limit: SpeedLimit) -> Result<(), Error> {
        self.write(SpeedLimitField, limit)
    }

    /// Turns the LED on or off: one acknowledged write, allowed with torque
    /// off.
    ///
    /// # Errors
    ///
    /// `Unsupported` on PRO and RH-P12-RN, whose LED is three colour
    /// registers (Tier 2, [`model`](Self::model)); otherwise any error of
    /// the write.
    pub fn set_led(&mut self, on: bool) -> Result<(), Error> {
        self.write(Led, on)
    }

    /// Reads Present Temperature, in °C.
    ///
    /// # Errors
    ///
    /// As [`read`](Self::read).
    pub fn present_temperature(&mut self) -> Result<Temperature, Error> {
        self.read(PresentTemperature)
    }

    /// Reads Present Voltage (Present Input Voltage on Protocol 2 models),
    /// in volts.
    ///
    /// # Errors
    ///
    /// As [`read`](Self::read).
    pub fn present_voltage(&mut self) -> Result<Voltage, Error> {
        self.read(PresentVoltage)
    }

    /// Reads Present Velocity (Present Speed on Protocol 1), in rad/s.
    ///
    /// # Errors
    ///
    /// As [`read`](Self::read); `NoUnit` on AX and RX in wheel mode, where
    /// Present Speed is in % of output (read `Raw(PresentVelocity)`).
    pub fn present_velocity(&mut self) -> Result<Velocity, Error> {
        self.read(PresentVelocity)
    }

    /// Reads Present Load, a ratio of the maximum torque from −1 to 1
    /// (AX, RX, MX on Protocol 1).
    ///
    /// # Errors
    ///
    /// As [`read`](Self::read); `Unsupported` on models without it.
    pub fn present_load(&mut self) -> Result<Load, Error> {
        self.read(PresentLoad)
    }

    /// Reads Present Current, in amperes (MX(2.0), X, Y, PRO).
    ///
    /// # Errors
    ///
    /// As [`read`](Self::read); `Unsupported` on models without it.
    pub fn present_current(&mut self) -> Result<Current, Error> {
        self.read(PresentCurrent)
    }

    /// Reads the goal velocity back (Goal Velocity, or Moving Speed in
    /// wheel mode), in rad/s.
    ///
    /// # Errors
    ///
    /// As [`read`](Self::read); `NoUnit` where the table gives the
    /// register no velocity unit in this mode.
    pub fn goal_velocity(&mut self) -> Result<Velocity, Error> {
        self.read(GoalVelocity)
    }

    /// Reads the goal current back (Goal Current, or Goal Torque), in
    /// amperes.
    ///
    /// # Errors
    ///
    /// As [`read`](Self::read).
    pub fn goal_current(&mut self) -> Result<Current, Error> {
        self.read(GoalCurrent)
    }

    /// Reads the speed limit back (Profile Velocity, or Moving Speed in
    /// joint and multi-turn mode).
    ///
    /// # Errors
    ///
    /// As [`read`](Self::read); `NoUnit` in wheel mode on Protocol 1.
    pub fn speed_limit(&mut self) -> Result<SpeedLimit, Error> {
        self.read(SpeedLimitField)
    }

    /// Reads Moving: whether the actuator is moving.
    ///
    /// # Errors
    ///
    /// As [`read`](Self::read).
    pub fn is_moving(&mut self) -> Result<bool, Error> {
        self.read(Moving)
    }

    /// Reads LED: whether it is on.
    ///
    /// # Errors
    ///
    /// As [`read`](Self::read); `Unsupported` on PRO and RH-P12-RN.
    pub fn led(&mut self) -> Result<bool, Error> {
        self.read(Led)
    }

    /// Reads Hardware Error Status (MX(2.0), X, Y, PRO): which fault the
    /// actuator reports.
    ///
    /// An actuator with a fault answers with the alert bit set. The read
    /// then returns the flags that reply carries and, like every reply
    /// reporting a fault, marks the actuator
    /// [`Faulted`](TorqueState::Faulted); a clean answer leaves the torque
    /// state as it is. Only [`clear_fault`](Self::clear_fault) clears
    /// `Faulted`, after a [`reboot`](Self::reboot) on X-series.
    ///
    /// # Errors
    ///
    /// As [`read`](Self::read); `Unsupported` on AX, RX and MX on Protocol
    /// 1, whose status packets carry their faults. A reply with the alert
    /// bit and no flag, or with another error, is returned as an error.
    pub fn hardware_errors(&mut self) -> Result<HardwareErrors, Error> {
        self.read(HardwareErrorStatus)
    }

    /// Reads Realtime Tick, the actuator's wrapping millisecond clock.
    ///
    /// # Errors
    ///
    /// As [`read`](Self::read); `Unsupported` on models without it, and on
    /// MX-64 Protocol 1 firmware older than 40, the first with it (Setup
    /// reads the firmware of an MX-64 on Protocol 1).
    pub fn realtime_tick(&mut self) -> Result<Tick, Error> {
        self.read(RealtimeTick)
    }

    /// Reads Position P Gain (P Gain on MX Protocol 1), in register units.
    /// Writing it is Tier 2 ([`model`](Self::model)).
    ///
    /// # Errors
    ///
    /// As [`read`](Self::read); `Unsupported` on models without it.
    pub fn p_gain(&mut self) -> Result<Gain, Error> {
        self.read(PGain)
    }

    /// Reads Position D Gain (D Gain on MX Protocol 1), in register units.
    /// Writing it is Tier 2 ([`model`](Self::model)).
    ///
    /// # Errors
    ///
    /// As [`read`](Self::read); `Unsupported` on models without it.
    pub fn d_gain(&mut self) -> Result<Gain, Error> {
        self.read(DGain)
    }

    /// Reads and decodes `reg`, the actuator's fault report, as
    /// [`read_reg`](Self::read_reg) does, except that a reply whose only
    /// error is the Protocol 2 alert bit returns the non-zero value it
    /// carries. The actuator is marked `Faulted` either way.
    fn read_fault_report(&mut self, reg: crate::register::Register) -> Result<i32, Error> {
        let mut buf = [0; 4];
        let bytes =
            buf.get_mut(..usize::from(reg.size))
                .ok_or(crate::ValidationError::DoesNotFit {
                    raw: 0,
                    size: reg.size,
                })?;
        let result = self.wire.read(self.kind.id(), reg, bytes);
        observe(self.state, &result);
        match result {
            Ok(()) => Ok(reg.decode(bytes)?),
            Err(Error::Status(StatusError {
                report:
                    StatusReport::P2 {
                        alert: true,
                        code: None,
                    },
                ..
            })) if bytes.iter().any(|&byte| byte != 0) => Ok(reg.decode(bytes)?),
            Err(err) => Err(err),
        }
    }

    /// Reads and decodes `reg`; a hardware fault in the reply marks the
    /// actuator `Faulted`.
    fn read_reg(&mut self, reg: crate::register::Register) -> Result<i32, Error> {
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
    /// reporting its fault until it is rebooted: on Protocol 2 call
    /// [`reboot`](Self::reboot) (or power-cycle it), then `clear_fault`.
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

/// Protocol 2 only.
impl<T: Transport, C: Clock> ActuatorRef<'_, P2, T, C> {
    /// Reboots the actuator (Reboot instruction, `0x08`) and waits for its
    /// status packet, which it sends before restarting. Never retried.
    ///
    /// The actuator comes back with torque off, so the torque state becomes
    /// `Off`, whatever the reply; a `Faulted` actuator stays `Faulted`
    /// until [`clear_fault`](Self::clear_fault) asks it.
    ///
    /// Setup is not repeated, and needs not be: what it read (position
    /// limits, operating mode, the goal's Velocity or Current Limit,
    /// firmware) lives in EEPROM, which a reboot leaves unchanged, so the
    /// bus's cache stays valid. RAM registers (goals, LED, gains) start
    /// again from their initial values, as after power-on. There is no
    /// factory reset.
    ///
    /// Protocol 1 has no typed reboot:
    ///
    /// ```compile_fail,E0599
    /// use dxl_rs_v2::{ActuatorRef, Clock, P1, Transport};
    ///
    /// fn restart<T: Transport, C: Clock>(ax: &mut ActuatorRef<'_, P1, T, C>) {
    ///     let _ = ax.reboot();
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Any error of the exchange; a reply reporting a hardware fault gives
    /// `Faulted`.
    pub fn reboot(&mut self) -> Result<(), Error> {
        let id = self.kind.id();
        let wanted = self.state.disable();
        let result = self.wire.reboot(id);
        after_torque_write(self.state, wanted, &result);
        result
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
