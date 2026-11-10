//! A short-lived handle on one held actuator.

use core::time::Duration;

use crate::Transport;
use crate::actuator::{Actuator, Cache, Describe, ModelData, OperatingMode, TorqueState};
use crate::error::{Error, StatusError, StatusReport};
use crate::field::{
    AccelerationLimit, CurrentLimit, DGain, GoalCurrent, GoalPosition, GoalVelocity,
    HardwareErrorStatus, HomingOffset, Led, MaxPositionLimit, MaxTorque, MaxVoltageLimit,
    MinPositionLimit, MinVoltageLimit, Moving, MovingThreshold, PGain, PresentCurrent, PresentLoad,
    PresentPosition, PresentTemperature, PresentVelocity, PresentVoltage, PwmLimit, Raw, ReadField,
    RealtimeTick, ReturnDelayTime, SettingField, SpeedLimit as SpeedLimitField, TemperatureLimit,
    VelocityLimit, WriteField,
};
use crate::models::ActuatorKind;
use crate::protocol::{Clock, P2, Protocol};
use crate::units::{
    Acceleration, Current, Gain, HardwareErrors, Load, Position, Pwm, SpeedLimit, Temperature,
    Tick, Velocity, Voltage,
};

use super::diagnose::diagnosed;
use super::field::{self as fields, sealed::Decode};
use super::limits;
use super::observe::{after_torque_write, check_clear, observe, ran, reached};
use super::slots::{Bound, Marks};
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
///
/// The EEPROM settings several series share are read the same way and
/// written with [`write_setting`](ActuatorRef::write_setting) and its
/// setters: the limits (voltage, temperature, position, velocity,
/// acceleration, current and PWM limits, Max Torque:
/// [`set_voltage_limits`](ActuatorRef::set_voltage_limits),
/// [`set_position_limits`](ActuatorRef::set_position_limits), ...), Homing
/// Offset ([`set_homing_offset`](ActuatorRef::set_homing_offset)), Moving
/// Threshold ([`set_moving_threshold`](ActuatorRef::set_moving_threshold))
/// and Return Delay Time
/// ([`set_return_delay_time`](ActuatorRef::set_return_delay_time)). Each is
/// one acknowledged write once torque is confirmed off (an acknowledged
/// [`disable_torque`](ActuatorRef::disable_torque) since Setup, and nothing
/// since that may turn torque on), checked against the e-Manual range
/// where the table records one and, for a pair, its order. The bus follows
/// each write: goals are then checked against the new position limits,
/// limit mode and goal bound. The operating mode is set the same way, with
/// [`set_operating_mode`](ActuatorRef::set_operating_mode), and goals then
/// follow the new mode; [`operating_mode`](ActuatorRef::operating_mode)
/// reads it. ID, baud rate and the registers of one series stay raw
/// (`raw-registers`).
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
    pub(super) bound: &'b mut Bound,
    pub(super) marks: &'b mut Marks,
    pub(super) wire: Wire<'b, P, T, C>,
}

impl<'b, P: Protocol, T: Transport, C: Clock> ActuatorRef<'b, P, T, C> {
    /// A handle on `kind`, whose slot is `state` with goal bound `bound`
    /// and marks `marks`, talking over `wire`.
    pub(crate) fn new(
        kind: &'b mut ActuatorKind,
        state: &'b mut TorqueState,
        bound: &'b mut Bound,
        marks: &'b mut Marks,
        wire: Wire<'b, P, T, C>,
    ) -> Self {
        Self {
            kind,
            state,
            bound,
            marks,
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

    /// The EEPROM bound of the goal this actuator's operating mode uses, in
    /// raw register units: Velocity Limit in velocity mode, Current Limit
    /// in current and current-based position mode. Setup reads it, an
    /// acknowledged [`set_operating_mode`](Self::set_operating_mode) into
    /// such a mode reads it again, and an acknowledged write of it sets it.
    /// Goal velocities and currents are checked against it (FR-9.3).
    ///
    /// `None` in the other modes, on models whose table records no bound
    /// (AX, RX, MX on Protocol 1), and while it is unknown:
    ///
    /// - after a failed write of it, or a failed bound read of
    ///   `set_operating_mode`, goals it bounds are refused with
    ///   [`LimitsUnknown`](crate::ValidationError::LimitsUnknown) until the
    ///   next Setup, an acknowledged write of it, or an acknowledged
    ///   `set_operating_mode` whose bound read succeeds;
    /// - after a failed write of the mode, the mode is unknown too: every
    ///   goal is refused with `LimitsUnknown`, and the bound stays `None`,
    ///   an acknowledged write of it included, until an acknowledged
    ///   `set_operating_mode` or the next Setup.
    pub fn goal_bound_raw(&self) -> Option<u32> {
        *self.bound
    }

    /// Turns torque on: one acknowledged write of Torque Enable = 1.
    ///
    /// Once the write is sent, whatever its reply, torque is no longer
    /// confirmed off: setting writes are refused with
    /// [`TorqueUnconfirmed`](crate::ValidationError::TorqueUnconfirmed)
    /// until the next acknowledged [`disable_torque`](Self::disable_torque).
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
        if reached(&result) {
            self.marks.forget_torque_off();
        }
        let result = diagnosed(&mut self.wire, self.kind, result);
        after_torque_write(self.state, wanted, &result);
        result
    }

    /// Turns torque off: one acknowledged write of Torque Enable = 0, sent
    /// in every state.
    ///
    /// A reply that shows the actuator ran the write confirms torque off,
    /// which [`write_setting`](Self::write_setting) and its setters need:
    /// a reply with no error, or one reporting only a hardware fault
    /// (Protocol 1: Input Voltage, Overheating, Overload; Protocol 2: the
    /// alert bit with no error code). So an actuator held `Faulted` at
    /// Setup is confirmed off by this call although it returns the fault.
    /// Any other result (no reply, a corrupted one, another error) leaves
    /// torque unconfirmed.
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
        self.marks.torque_off_written(ran(&result));
        let result = diagnosed(&mut self.wire, self.kind, result);
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
            *self.bound,
            value,
            &mut buf,
        )?;
        let result = self.wire.write(id, reg, bytes);
        if limits::writes_moving_speed(self.kind, reg.addr, usize::from(reg.size)) {
            // DECISION: every unicast write of Moving Speed moves the D9 mark, whichever field wrote it (speed limit, or goal velocity in wheel mode): known 0 when the reply shows the write of 0 ran (as D8 reads a torque write's reply), else no longer known.
            self.marks
                .speed_written(ran(&result) && bytes.iter().all(|&byte| byte == 0));
        }
        let result = diagnosed(&mut self.wire, self.kind, result);
        observe(self.state, &result);
        result
    }

    /// Sends goal position `goal`, in radians: one acknowledged write of
    /// Goal Position, no read-back.
    ///
    /// The angle is converted to the nearest tick, then checked like
    /// [`set_goal_position_raw`](Self::set_goal_position_raw).
    ///
    /// Limits and operating mode are those read at Setup, or written since
    /// through the bus ([`set_position_limits`](Self::set_position_limits),
    /// [`set_operating_mode`](Self::set_operating_mode)). Position mode
    /// applies the EEPROM limits. Extended and current-based position modes
    /// and MX multi-turn apply none, as the firmware does; only the register
    /// size is checked. Velocity, current, PWM and wheel modes refuse every
    /// goal (`NotPositionMode`). A mode changed otherwise (a raw write,
    /// another program) is not seen until the next Setup.
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
    /// Limits and operating mode are those read at Setup, or written since
    /// through the bus ([`set_position_limits`](Self::set_position_limits),
    /// [`set_operating_mode`](Self::set_operating_mode)). Position mode
    /// applies the EEPROM limits. Extended and current-based position modes
    /// and MX multi-turn apply none, as the firmware does; only the register
    /// size is checked. Velocity, current, PWM and wheel modes refuse every
    /// goal (`NotPositionMode`). A mode changed otherwise (a raw write,
    /// another program) is not seen until the next Setup.
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
    /// On AX, RX and MX on Protocol 1, Moving Speed stays in the register
    /// when angle limits switch the actuator to wheel mode, and there it is
    /// the goal velocity that the next [`enable_torque`](Self::enable_torque)
    /// drives (AX-12A Moving Speed(32), wheel mode: "If a value in the range
    /// of 0 ~ 1,023 is used, it is stopped by setting to 0 while rotating
    /// to CCW direction", <https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/>).
    /// So a limit write that switches to wheel mode is refused with
    /// [`WheelSpeedUnknown`](crate::ValidationError::WheelSpeedUnknown)
    /// unless the bus knows Moving Speed is 0: write
    /// [`SpeedLimit::Unlimited`] (raw 0, stop in wheel mode) first (or
    /// `write(Raw(SpeedLimit), 0)` where this returns `NoUnit`). Only an
    /// acknowledged unicast write of Moving Speed = 0 makes it known: this
    /// call with [`SpeedLimit::Unlimited`], `write(Raw(SpeedLimit), 0)`, or
    /// a goal velocity of 0 in wheel mode (`write(Raw(GoalVelocity), 0)`);
    /// any other write of Moving Speed, on any tier (a Sync Write, which
    /// gets no reply, even of 0), a raw write and a reboot make it unknown
    /// again.
    ///
    /// # Errors
    ///
    /// As [`write`](Self::write). `NotPositionMode` on Protocol 1 in wheel
    /// mode, where Moving Speed is the goal velocity: use
    /// [`set_goal_velocity`](Self::set_goal_velocity) there.
    /// `LimitsUnknown` on Protocol 1 while the wheel/joint mode is unknown,
    /// after a failed angle-limit write.
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

    /// Writes EEPROM setting `F`: one acknowledged unicast write of its
    /// register, with torque confirmed off, no read-back. `Raw(F)` writes
    /// the register value, with the same checks. The pair setters
    /// ([`set_voltage_limits`](Self::set_voltage_limits),
    /// [`set_position_limits`](Self::set_position_limits)) and the named
    /// setters below call it.
    ///
    /// Everything is checked before a byte is sent, in this order: the
    /// model has the register; torque is confirmed off (below); the value
    /// converts to a register value with the unit its control table gives
    /// (a position: the table records the limit's e-Manual range); it lies
    /// in the e-Manual range the table records, where it records one (else
    /// only the register size checks it, e.g. MX-64 on Protocol 1 voltage
    /// limits); a position limit keeps the limits ordered against the
    /// other limit the bus holds; on AX, RX and MX on Protocol 1 it does not
    /// switch to wheel mode while Moving Speed may be live (below); it fits
    /// the register. A single voltage limit cannot be checked against the
    /// other one, which the bus does not hold: use
    /// [`set_voltage_limits`](Self::set_voltage_limits) to check both.
    ///
    /// The torque check is the bus's rule, on every model: X-series,
    /// MX(2.0) and XL-320 firmware also writes EEPROM only with Torque
    /// Enable 0 (XM430-W350 Torque Enable(64),
    /// <https://emanual.robotis.com/docs/en/dxl/x/xm430-w350/>), but the AX,
    /// RX and MX Protocol 1 pages document no such lock, so there it is the
    /// only guard. The bus never reads the hardware's Torque Enable and
    /// never writes it on its own, so it passes only once it has confirmed
    /// torque off: an acknowledged [`disable_torque`](Self::disable_torque)
    /// (or [`Bus::disable_torque`](crate::Bus::disable_torque)), whose reply
    /// shows the write ran, also when that reply carries only a hardware
    /// fault, so an actuator held faulted at Setup can be fixed. Setup,
    /// every [`enable_torque`](Self::enable_torque) that sent bytes, a torque
    /// write without such a reply, [`reboot`] (Startup Configuration may
    /// turn torque on) and a raw write (`write_raw`, a raw Bulk Write part)
    /// leave torque unconfirmed, whatever the torque state.
    /// [`clear_fault`](Self::clear_fault) sends no reboot and keeps it.
    ///
    /// On AX, RX and MX on Protocol 1, a write that leaves CW = CCW Angle
    /// Limit = 0 switches the actuator to wheel mode, where Moving Speed,
    /// the last speed limit, is the goal velocity the next
    /// [`enable_torque`](Self::enable_torque) drives. From any other mode
    /// it is refused unless the bus knows Moving Speed is 0: write
    /// [`SpeedLimit::Unlimited`] first
    /// ([`set_speed_limit`](Self::set_speed_limit)), or
    /// `write(Raw(SpeedLimit), 0)` where the table gives Moving Speed no
    /// unit in the recorded mode (unverified tables; MX-64 on Protocol 1
    /// with torque control on). With the mode unknown after a failed
    /// angle-limit write, Moving Speed cannot be written
    /// ([`LimitsUnknown`]): first write a non-wheel pair with
    /// [`set_position_limits`](Self::set_position_limits), which makes the
    /// mode known again, or declare again. A Moving Speed = 0 acknowledged
    /// before the failed write still counts. After a raw write
    /// (`raw-registers`) the bus no longer trusts the wheel mode it holds,
    /// so there too CW = CCW = 0 counts as a switch, until an acknowledged
    /// `set_position_limits`. Leaving wheel mode is never refused for it.
    ///
    /// The bus follows the write. Acknowledged, a position limit replaces
    /// the limit Setup read, so goals are checked against it; on AX, RX and
    /// MX on Protocol 1 the limit mode follows the angle limits as Setup
    /// would have found it: CW = CCW = 0 is wheel mode, and on MX CW = CCW
    /// = 4095 (raw) is multi-turn mode, where goal positions are no longer
    /// checked against the angle limits, only against the register size. A
    /// Velocity or Current Limit the operating mode bounds its goal with
    /// replaces that bound. The other settings (voltage, temperature,
    /// acceleration and PWM limits, Max Torque, Homing Offset, Moving
    /// Threshold, Return Delay Time) are not held, so writing them changes
    /// no check.
    ///
    /// Failed, whatever it was written to is unknown: every goal that
    /// depends on it is refused with [`LimitsUnknown`], until the next Setup
    /// or an acknowledged write that determines it again: both position
    /// limits ([`set_position_limits`](Self::set_position_limits)), which on
    /// Protocol 1 also give the limit mode, or the bounding Velocity or
    /// Current Limit. A single position limit stays refused while the other
    /// is unknown. On a faulted X-series (or other Protocol 2) actuator
    /// every reply carries the alert bit until it is rebooted, so a limit
    /// write there returns the fault and leaves what it wrote unknown:
    /// after [`reboot`], [`clear_fault`](Self::clear_fault) and
    /// [`disable_torque`](Self::disable_torque), write it again, or declare
    /// again to re-read it.
    ///
    /// ```
    /// use dxl_rs_v2::field::{MaxVoltageLimit, Raw, TemperatureLimit};
    /// use dxl_rs_v2::{ActuatorRef, Clock, Error, P1, Temperature, Transport};
    ///
    /// fn widen<T: Transport, C: Clock>(ax: &mut ActuatorRef<'_, P1, T, C>) -> Result<(), Error> {
    ///     ax.disable_torque()?;
    ///     ax.write_setting(TemperatureLimit, Temperature::from_celsius(75.0))?;
    ///     ax.write_setting(Raw(MaxVoltageLimit), 160)
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Refused with nothing sent:
    /// - [`Unsupported`]: the model's table lacks the register.
    /// - [`ActuatorIsOn`]: torque is [`On`](TorqueState::On).
    /// - [`TorqueUnconfirmed`]: torque is not `On`, but not confirmed off
    ///   either: call [`disable_torque`](Self::disable_torque) first.
    /// - [`NoUnit`]: a typed value, and the table gives the register no
    ///   unit, or for a position limit no e-Manual range (an unverified
    ///   table): use `Raw(F)`.
    /// - [`NotFinite`], [`DoesNotFit`]: the value maps to no register
    ///   value.
    /// - [`OutOfLimits`]: outside the e-Manual range.
    /// - [`InvertedLimits`]: a position limit on the wrong side of the
    ///   other one.
    /// - [`LimitsUnknown`]: a position limit, and the other one is unknown
    ///   since a failed limit write.
    /// - [`WheelSpeedUnknown`]: an angle limit that switches an AX, RX or
    ///   MX on Protocol 1 to wheel mode while Moving Speed is not known to
    ///   be 0.
    ///
    /// Otherwise any error of the write; a reply reporting a hardware fault
    /// marks the actuator `Faulted`.
    ///
    /// [`Unsupported`]: crate::ValidationError::Unsupported
    /// [`ActuatorIsOn`]: crate::ValidationError::ActuatorIsOn
    /// [`TorqueUnconfirmed`]: crate::ValidationError::TorqueUnconfirmed
    /// [`WheelSpeedUnknown`]: crate::ValidationError::WheelSpeedUnknown
    /// [`NoUnit`]: crate::ValidationError::NoUnit
    /// [`NotFinite`]: crate::ValidationError::NotFinite
    /// [`DoesNotFit`]: crate::ValidationError::DoesNotFit
    /// [`OutOfLimits`]: crate::ValidationError::OutOfLimits
    /// [`InvertedLimits`]: crate::ValidationError::InvertedLimits
    /// [`LimitsUnknown`]: crate::ValidationError::LimitsUnknown
    /// [`reboot`]: ActuatorRef::reboot
    pub fn write_setting<F: SettingField>(
        &mut self,
        field: F,
        value: F::Value,
    ) -> Result<(), Error> {
        let _ = field;
        let write = limits::check_single::<F>(self.kind, (*self.state, *self.marks), value)?;
        self.send_limits(write, None)
    }

    /// Sets Min and Max Voltage Limit, in volts: the input voltages outside
    /// which the actuator reports a fault. One acknowledged write of both
    /// where the registers are adjacent (every model), with torque
    /// confirmed off.
    ///
    /// Checked as [`write_setting`](Self::write_setting), both values before a
    /// byte is sent, each against the e-Manual range where the table
    /// records one, and `min` must not lie above `max`.
    ///
    /// # Errors
    ///
    /// As [`write_setting`](Self::write_setting); [`InvertedLimits`] when `min`
    /// is above `max`, in raw units; [`ActuatorIsOn`] and
    /// [`TorqueUnconfirmed`] name Min Voltage Limit.
    ///
    /// [`InvertedLimits`]: crate::ValidationError::InvertedLimits
    /// [`ActuatorIsOn`]: crate::ValidationError::ActuatorIsOn
    /// [`TorqueUnconfirmed`]: crate::ValidationError::TorqueUnconfirmed
    pub fn set_voltage_limits(&mut self, min: Voltage, max: Voltage) -> Result<(), Error> {
        let (lo, hi) = limits::check_pair::<MinVoltageLimit, MaxVoltageLimit>(
            self.kind,
            (*self.state, *self.marks),
            min,
            max,
        )?;
        self.send_limits(lo, Some(hi))
    }

    /// Sets the EEPROM position limits, in radians: Min and Max Position
    /// Limit, or CW and CCW Angle Limit (AX, RX, MX on Protocol 1,
    /// XL-320). One acknowledged write of both where the registers are
    /// adjacent, else one each, with torque confirmed off.
    ///
    /// Checked as [`write_setting`](Self::write_setting), both values before a
    /// byte is sent, and `min` must not lie above `max`. Acknowledged, they
    /// replace the limits the bus holds
    /// ([`position_limits`](Actuator::position_limits)), also limits a
    /// failed write had left unknown, and on AX, RX and MX on Protocol 1 the
    /// limit mode follows them: both at raw tick 0 (CW = CCW = 0) is wheel
    /// mode, and on MX both at raw 4095 is multi-turn mode, where goal
    /// positions are checked only against the register size. In wheel mode
    /// Moving Speed, the last speed limit, is the goal velocity the next
    /// [`enable_torque`](Self::enable_torque) drives, so the switch is
    /// refused with [`WheelSpeedUnknown`] unless an acknowledged write of
    /// Moving Speed = 0 came first
    /// ([`set_speed_limit(SpeedLimit::Unlimited)`](Self::set_speed_limit),
    /// or `write(Raw(SpeedLimit), 0)` where that returns `NoUnit`); with
    /// the mode unknown after a failed angle-limit write, write a non-wheel
    /// pair first, as [`write_setting`](Self::write_setting) says. Leaving
    /// wheel mode is not refused. After a raw write, CW = CCW = 0 counts as
    /// a switch even where the bus holds wheel mode, until an acknowledged
    /// call of this. Where the two registers are apart (Y series),
    /// the one that keeps the pair ordered is written first.
    ///
    /// Failed, the limits (and on Protocol 1 the limit mode) are unknown
    /// until the next Setup or an acknowledged call of this. On a faulted
    /// X-series actuator the reply carries the alert bit until a reboot,
    /// so the write returns the fault: after
    /// [`reboot`](ActuatorRef::reboot), [`clear_fault`](Self::clear_fault)
    /// and [`disable_torque`](Self::disable_torque), call this again.
    ///
    /// # Errors
    ///
    /// As [`write_setting`](Self::write_setting); [`InvertedLimits`] when `min`
    /// is above `max`; [`ActuatorIsOn`] and [`TorqueUnconfirmed`] name the
    /// minimum register; [`WheelSpeedUnknown`] for a switch to wheel mode
    /// with Moving Speed not known to be 0.
    ///
    /// [`InvertedLimits`]: crate::ValidationError::InvertedLimits
    /// [`ActuatorIsOn`]: crate::ValidationError::ActuatorIsOn
    /// [`TorqueUnconfirmed`]: crate::ValidationError::TorqueUnconfirmed
    /// [`WheelSpeedUnknown`]: crate::ValidationError::WheelSpeedUnknown
    pub fn set_position_limits(&mut self, min: Position, max: Position) -> Result<(), Error> {
        let (lo, hi) = limits::check_pair::<MinPositionLimit, MaxPositionLimit>(
            self.kind,
            (*self.state, *self.marks),
            min,
            max,
        )?;
        let (first, second) = limits::ordered(self.kind, lo, hi);
        self.send_limits(first, Some(second))
    }

    /// Sets Temperature Limit, in °C: above it the actuator reports
    /// overheating. One acknowledged write, with torque confirmed off.
    ///
    /// # Errors
    ///
    /// As [`write_setting`](Self::write_setting).
    pub fn set_temperature_limit(&mut self, limit: Temperature) -> Result<(), Error> {
        self.write_setting(TemperatureLimit, limit)
    }

    /// Sets Velocity Limit, in rad/s (MX(2.0), X, Y, PRO). One acknowledged
    /// write, with torque confirmed off. In velocity mode it then bounds
    /// goal velocities, as the value Setup read did.
    ///
    /// # Errors
    ///
    /// As [`write_setting`](Self::write_setting); `Unsupported` on models
    /// without it.
    pub fn set_velocity_limit(&mut self, limit: Velocity) -> Result<(), Error> {
        self.write_setting(VelocityLimit, limit)
    }

    /// Sets Current Limit, in amperes (MX(2.0), X, Y, PRO). One
    /// acknowledged write, with torque confirmed off. In current and
    /// current-based position mode it then bounds goal currents, as the
    /// value Setup read did.
    ///
    /// # Errors
    ///
    /// As [`write_setting`](Self::write_setting); `Unsupported` on models
    /// without it.
    pub fn set_current_limit(&mut self, limit: Current) -> Result<(), Error> {
        self.write_setting(CurrentLimit, limit)
    }

    /// Sets PWM Limit, a ratio of the full PWM output (MX(2.0), X, Y, PRO).
    /// One acknowledged write, with torque confirmed off.
    ///
    /// # Errors
    ///
    /// As [`write_setting`](Self::write_setting); `Unsupported` on models
    /// without it.
    pub fn set_pwm_limit(&mut self, limit: Pwm) -> Result<(), Error> {
        self.write_setting(PwmLimit, limit)
    }

    /// Sets Max Torque, a ratio of the maximum torque from 0 to 1 (AX, RX,
    /// MX on Protocol 1, XL-320). One acknowledged write, with torque
    /// confirmed off.
    ///
    /// Max Torque is the value the running Torque Limit (RAM) is reset to
    /// at power-on (AX-12A: "When the power is turned on, Torque Limit(34)
    /// is reset to the value of Max Torque(14)",
    /// <https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/>): writing it
    /// does not change the running torque limit until the next power-on.
    ///
    /// # Errors
    ///
    /// As [`write_setting`](Self::write_setting); `Unsupported` on models
    /// without it; `DoesNotFit` for a ratio below minus half a raw step (a
    /// smaller negative one is written as 0).
    pub fn set_max_torque(&mut self, limit: Load) -> Result<(), Error> {
        self.write_setting(MaxTorque, limit)
    }

    /// Sets Acceleration Limit, in rad/s² (MX(2.0), PRO, PRO+, Y,
    /// RH-P12-RN): the bound of Profile Acceleration, which the firmware
    /// enforces (MX-64(2.0): "Profile Acceleration(108) can’t be configured
    /// with any values exceeding Acceleration Limit(40)",
    /// <https://emanual.robotis.com/docs/en/dxl/mx/mx-64-2/>). One
    /// acknowledged write, with torque confirmed off. The bus holds no copy
    /// of it, so nothing else it checks changes.
    ///
    /// # Errors
    ///
    /// As [`write_setting`](Self::write_setting); `Unsupported` on models
    /// without it; `NoUnit` where the table gives it no unit (write
    /// `Raw(AccelerationLimit)`); `DoesNotFit` for an acceleration below
    /// minus half a raw step (a smaller negative one is written as 0).
    pub fn set_acceleration_limit(&mut self, limit: Acceleration) -> Result<(), Error> {
        self.write_setting(AccelerationLimit, limit)
    }

    /// Sets Homing Offset, an angle offset in radians (MX(2.0), X, Y, PRO):
    /// the actuator adds it to Present Position ("Present Position(132) =
    /// Actual Position + Homing Offset(20)", XM430-W350,
    /// <https://emanual.robotis.com/docs/en/dxl/x/xm430-w350/>). One
    /// acknowledged write, with torque confirmed off.
    ///
    /// `offset` is an angle, not a position: one raw step is one position
    /// tick of [`position_scale`](Actuator::position_scale), with no zero
    /// point, so [`Position::ZERO`] writes raw 0 and an offset of `n` ticks
    /// shifts Present Position by `n` ticks. The bus holds no copy of it:
    /// the position limits it holds stay the raw values Setup read, and the
    /// firmware still applies them to goal positions in the shifted frame,
    /// so a goal is checked against the same raw limits after the write.
    /// The XM430-W350 ignores an offset outside ±1,024 ticks in position
    /// mode (e-Manual); the bus checks only the table's range.
    ///
    /// # Errors
    ///
    /// As [`write_setting`](Self::write_setting); `Unsupported` on models
    /// without it; `NoUnit` where the table gives it no unit (write
    /// `Raw(HomingOffset)`).
    pub fn set_homing_offset(&mut self, offset: Position) -> Result<(), Error> {
        self.write_setting(HomingOffset, offset)
    }

    /// Sets Moving Threshold, in rad/s (MX(2.0), X, Y, PRO): above it the
    /// actuator reports Moving ([`is_moving`](Self::is_moving)). One
    /// acknowledged write, with torque confirmed off.
    ///
    /// # Errors
    ///
    /// As [`write_setting`](Self::write_setting); `Unsupported` on models
    /// without it; `NoUnit` where the table gives it no unit (write
    /// `Raw(MovingThreshold)`); `DoesNotFit` for a speed below minus half a
    /// raw step (a smaller negative one is written as 0).
    pub fn set_moving_threshold(&mut self, threshold: Velocity) -> Result<(), Error> {
        self.write_setting(MovingThreshold, threshold)
    }

    /// Sets Return Delay Time: how long the actuator waits after an
    /// instruction packet before its status packet, rounded to the nearest
    /// raw step (2 µs on every page seen, 0 to 254 steps). One acknowledged
    /// write, with torque confirmed off.
    ///
    /// The delay adds to the latency of every reply, so the bus's reply
    /// timeout ([`Config::timeout`](crate::Config::timeout)) must stay
    /// above it plus the reply's transfer time; the default 10 ms covers
    /// the largest delay the register holds, 508 µs.
    ///
    /// # Errors
    ///
    /// As [`write_setting`](Self::write_setting); `NoUnit` where the table
    /// gives it no unit (write `Raw(ReturnDelayTime)`); `DoesNotFit` for a
    /// delay the register cannot hold.
    pub fn set_return_delay_time(&mut self, delay: Duration) -> Result<(), Error> {
        self.write_setting(ReturnDelayTime, delay)
    }

    /// Reads Min and Max Voltage Limit, in volts, `(min, max)`: one read
    /// of both where the registers are adjacent (every model). Never
    /// cached: each call asks the actuator.
    ///
    /// # Errors
    ///
    /// As [`read`](Self::read); `NoUnit` where the table gives no unit (read
    /// `Raw(MinVoltageLimit)` and `Raw(MaxVoltageLimit)`).
    pub fn voltage_limits(&mut self) -> Result<(Voltage, Voltage), Error> {
        self.read_pair::<MinVoltageLimit, MaxVoltageLimit>()
    }

    /// Reads Temperature Limit, in °C. Never cached.
    ///
    /// # Errors
    ///
    /// As [`read`](Self::read).
    pub fn temperature_limit(&mut self) -> Result<Temperature, Error> {
        self.read(TemperatureLimit)
    }

    /// Reads Velocity Limit, in rad/s (MX(2.0), X, Y, PRO). Never cached;
    /// in velocity mode [`goal_bound_raw`](Self::goal_bound_raw) is the
    /// bound the bus holds, `None` while it is unknown.
    ///
    /// # Errors
    ///
    /// As [`read`](Self::read); `Unsupported` on models without it.
    pub fn velocity_limit(&mut self) -> Result<Velocity, Error> {
        self.read(VelocityLimit)
    }

    /// Reads Current Limit, in amperes (MX(2.0), X, Y, PRO). Never cached;
    /// in a current mode [`goal_bound_raw`](Self::goal_bound_raw) is the
    /// bound the bus holds, `None` while it is unknown.
    ///
    /// # Errors
    ///
    /// As [`read`](Self::read); `Unsupported` on models without it.
    pub fn current_limit(&mut self) -> Result<Current, Error> {
        self.read(CurrentLimit)
    }

    /// Reads PWM Limit, a ratio of the full PWM output (MX(2.0), X, Y,
    /// PRO). Never cached.
    ///
    /// # Errors
    ///
    /// As [`read`](Self::read); `Unsupported` on models without it.
    pub fn pwm_limit(&mut self) -> Result<Pwm, Error> {
        self.read(PwmLimit)
    }

    /// Reads Max Torque, a ratio of the maximum torque (AX, RX, MX on
    /// Protocol 1, XL-320): the torque limit the actuator starts with at
    /// power-on. Never cached.
    ///
    /// # Errors
    ///
    /// As [`read`](Self::read); `Unsupported` on models without it.
    pub fn max_torque(&mut self) -> Result<Load, Error> {
        self.read(MaxTorque)
    }

    /// Reads Acceleration Limit, in rad/s² (MX(2.0), PRO, PRO+, Y,
    /// RH-P12-RN): the bound of Profile Acceleration. Never cached.
    ///
    /// # Errors
    ///
    /// As [`read`](Self::read); `Unsupported` on models without it;
    /// `NoUnit` where the table gives it no unit (read
    /// `Raw(AccelerationLimit)`).
    pub fn acceleration_limit(&mut self) -> Result<Acceleration, Error> {
        self.read(AccelerationLimit)
    }

    /// Reads Homing Offset, an angle offset in radians with no zero point
    /// (MX(2.0), X, Y, PRO). Never cached.
    ///
    /// # Errors
    ///
    /// As [`read`](Self::read); `Unsupported` on models without it;
    /// `NoUnit` where the table gives it no unit (read `Raw(HomingOffset)`).
    pub fn homing_offset(&mut self) -> Result<Position, Error> {
        self.read(HomingOffset)
    }

    /// Reads Moving Threshold, in rad/s (MX(2.0), X, Y, PRO). Never cached.
    ///
    /// # Errors
    ///
    /// As [`read`](Self::read); `Unsupported` on models without it;
    /// `NoUnit` where the table gives it no unit (read
    /// `Raw(MovingThreshold)`).
    pub fn moving_threshold(&mut self) -> Result<Velocity, Error> {
        self.read(MovingThreshold)
    }

    /// Reads Return Delay Time, to the nanosecond. Never cached.
    ///
    /// # Errors
    ///
    /// As [`read`](Self::read); `NoUnit` where the table gives it no unit
    /// (read `Raw(ReturnDelayTime)`).
    pub fn return_delay_time(&mut self) -> Result<Duration, Error> {
        self.read(ReturnDelayTime)
    }

    /// Sets the operating mode: one acknowledged write of the mode register
    /// (Operating Mode, XL-320 Control Mode), with torque confirmed off,
    /// then, in a mode that bounds its goal, one read of that bound.
    ///
    /// The value written is the one the model's control table lists for
    /// `mode` ([`OperatingMode`]); a mode it lists no single value for is
    /// refused. Checked as [`write_setting`](Self::write_setting) checks a
    /// setting, before a byte is sent: the model has a mode register, then
    /// torque is confirmed off, then the mode has its value. AX, RX and MX
    /// on Protocol 1 have no mode register: their mode follows the angle
    /// limits, so [`set_position_limits`](Self::set_position_limits)
    /// switches them between joint, wheel (both limits 0) and, on MX,
    /// multi-turn mode.
    ///
    /// The bus follows the write as Setup would have read the new mode.
    /// Acknowledged, goals are checked in the new mode from then on
    /// ([`limit_mode`](Actuator::limit_mode)); a mode that bounds its goal
    /// (velocity: Velocity Limit; current and current-based position:
    /// Current Limit) then reads that bound once, where the table has the
    /// register, and checks goals against it
    /// ([`goal_bound_raw`](Self::goal_bound_raw)): that read is part of
    /// this call. If the read fails, the mode is set and recorded but the
    /// bound is unknown: goals it bounds are refused with
    /// [`LimitsUnknown`] until the next Setup or an acknowledged write of
    /// the bound ([`set_velocity_limit`](Self::set_velocity_limit),
    /// [`set_current_limit`](Self::set_current_limit)), and the call
    /// returns the read's error. A write that fails once bytes went out
    /// (no reply, a corrupted one, a hardware fault) leaves the mode
    /// unknown: every goal is refused with [`LimitsUnknown`] until an
    /// acknowledged `set_operating_mode` or the next Setup. The position
    /// limits the bus holds do not change.
    ///
    /// The firmware resets other registers on a mode change (XM430-W350
    /// Operating Mode(11),
    /// <https://emanual.robotis.com/docs/en/dxl/x/xm430-w350/>): "value of
    /// Gains, such as Velocity PI(76, 78); Position PID(80, 82, 84);
    /// Feedforward(88, 90), will be reset fitting to a selected Operating
    /// Mode(11)"; Profile Velocity(112) and Profile Acceleration(108) are
    /// reset to 0; "The Goal PWM(100) and Goal Current(102) are reset to
    /// the value of PWM Limit(36) and Current Limit(38) respectively"; in
    /// Current-based Position Control Mode, Position PID(80, 82, 84) and
    /// PWM Limit(36) are reset too. Write gains and profiles again after
    /// the call, where they matter.
    ///
    /// Every other goal register keeps its last value, and the actuator
    /// acts on its goals as soon as torque is on: in current mode
    /// [`enable_torque`](Self::enable_torque) drives Goal Current, now
    /// Current Limit; in velocity mode, the last Goal Velocity. The typed
    /// goal setters need torque on ([`ActuatorIsOff`]), so to start from a
    /// goal of your own, write it before `enable_torque` with `write_raw`
    /// (`raw-registers`), or set the bound it starts from first
    /// ([`set_current_limit`](Self::set_current_limit) before switching to
    /// current mode).
    ///
    /// ```
    /// use dxl_rs_v2::{ActuatorRef, Clock, Error, OperatingMode, P2, Transport, Velocity};
    ///
    /// fn spin<T: Transport, C: Clock>(xm: &mut ActuatorRef<'_, P2, T, C>) -> Result<(), Error> {
    ///     xm.disable_torque()?;
    ///     xm.set_operating_mode(OperatingMode::Velocity)?;
    ///     xm.enable_torque()?;
    ///     xm.set_goal_velocity(Velocity::from_rad_per_s(1.0))
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Refused with nothing sent:
    /// - [`Unsupported`]: no mode register (AX, RX, MX on Protocol 1),
    ///   naming Operating Mode.
    /// - [`ActuatorIsOn`]: torque is [`On`](TorqueState::On).
    /// - [`TorqueUnconfirmed`]: torque is not `On`, but not confirmed off
    ///   either: call [`disable_torque`](Self::disable_torque) first.
    /// - [`UnsupportedMode`]: the table lists no single value for `mode`:
    ///   the model lacks it, or the table does not single it out yet.
    ///
    /// Otherwise any error of the write, or of the bound read after an
    /// acknowledged write; a reply reporting a hardware fault marks the
    /// actuator `Faulted`.
    ///
    /// [`Unsupported`]: crate::ValidationError::Unsupported
    /// [`ActuatorIsOn`]: crate::ValidationError::ActuatorIsOn
    /// [`ActuatorIsOff`]: crate::ValidationError::ActuatorIsOff
    /// [`TorqueUnconfirmed`]: crate::ValidationError::TorqueUnconfirmed
    /// [`UnsupportedMode`]: crate::ValidationError::UnsupportedMode
    /// [`LimitsUnknown`]: crate::ValidationError::LimitsUnknown
    pub fn set_operating_mode(&mut self, mode: OperatingMode) -> Result<(), Error> {
        // DECISION: the plan's "write the goal before enable_torque" cannot hold on the typed tier, which refuses goals with torque off (Principle III), so the docs give the two ways that can: write_raw before enable_torque, or the bound the firmware resets the goal to.
        let write = super::mode::check_mode(self.kind, (*self.state, *self.marks), mode)?;
        self.send_mode(write)
    }

    /// Reads the operating mode: one read of the mode register (Operating
    /// Mode, XL-320 Control Mode), mapped back to the [`OperatingMode`]
    /// whose value the control table lists. Never cached: the limit mode
    /// the bus checks goals in ([`limit_mode`](Actuator::limit_mode)) does
    /// not change.
    ///
    /// # Errors
    ///
    /// - [`Unsupported`] with nothing sent: no mode register (AX, RX, MX on
    ///   Protocol 1), naming Operating Mode.
    /// - Any error of the read; a reply reporting a hardware fault marks
    ///   the actuator `Faulted`.
    /// - [`UnlistedMode`]: the value read is the value of no named mode in
    ///   the table, carried in the error.
    ///
    /// [`Unsupported`]: crate::ValidationError::Unsupported
    /// [`UnlistedMode`]: crate::ValidationError::UnlistedMode
    pub fn operating_mode(&mut self) -> Result<OperatingMode, Error> {
        self.read_mode()
    }

    /// Reads and decodes `reg`, the actuator's fault report, as
    /// [`read_reg`](Self::read_reg) does, except that a reply whose only
    /// error is the Protocol 2 alert bit returns the non-zero value it
    /// carries. The actuator is marked `Faulted` either way.
    fn read_fault_report(&mut self, reg: crate::register::Register) -> Result<i32, Error> {
        let mut buf = [0; 4];
        let bytes = reg.buf(&mut buf)?;
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
    pub(super) fn read_reg(&mut self, reg: crate::register::Register) -> Result<i32, Error> {
        let result = self.wire.read_value(self.kind.id(), reg);
        let result = diagnosed(&mut self.wire, self.kind, result);
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
    /// It sends no reboot on either protocol (a read or a ping), so it
    /// leaves torque confirmed off if it was: a setting write needs no new
    /// [`disable_torque`](Self::disable_torque) after it.
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
    /// The torque state becomes `Off`, whatever the reply, as at Setup:
    /// the hardware's Torque Enable is not read. A `Faulted` actuator stays
    /// `Faulted` until [`clear_fault`](Self::clear_fault) asks it.
    ///
    /// With factory settings the actuator comes back with torque off and
    /// its RAM registers (goals, LED, gains) at their initial values, as
    /// after power-on. A model with Startup Configuration (address 60, e.g.
    /// XM430-W350 from firmware V45, PH54-200-S500-R from V12) can be set
    /// to come back with torque on (bit 0) or with its RAM restored from
    /// the backup (bit 1). The bus does not read it: `Off` then still
    /// means goal writes are refused until
    /// [`enable_torque`](Self::enable_torque). Torque is no longer
    /// confirmed off, and Moving Speed no longer known to be 0, once the
    /// instruction is sent: [`write_setting`](Self::write_setting) and its
    /// setters are refused with
    /// [`TorqueUnconfirmed`](crate::ValidationError::TorqueUnconfirmed)
    /// until an acknowledged [`disable_torque`](Self::disable_torque).
    ///
    /// Setup is not repeated, and needs not be: what it read (position
    /// limits, operating mode, the goal's Velocity or Current Limit,
    /// firmware) lives in EEPROM, which a reboot leaves unchanged, so the
    /// bus's cache stays valid. A limit a failed write left unknown stays
    /// unknown: a limit write to a faulted X-series actuator returns its
    /// fault, so after the reboot, [`clear_fault`](Self::clear_fault) and
    /// [`disable_torque`](Self::disable_torque) write it again (both
    /// position limits with
    /// [`set_position_limits`](Self::set_position_limits)), or declare
    /// again. There is no factory reset.
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
        if reached(&result) {
            self.marks.forget();
        }
        let result = diagnosed(&mut self.wire, self.kind, result);
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
/// `position_limits[_raw]` of the held actuator, as read at Setup and kept
/// true by its limit writes (`None` after a failed one, until the next
/// Setup or an acknowledged write of both).
impl<P: Protocol, T: Transport, C: Clock> Actuator for ActuatorRef<'_, P, T, C> {}

#[cfg(test)]
mod tests {

    #[cfg(feature = "x-series")]
    mod x_series {
        extern crate std;

        use super::super::*;
        use crate::actuator::ModelNumber;
        use crate::bus::testing::*;
        use crate::error::{
            Celsius, FaultError, Instruction, Request, TemperatureError, TransportError,
            ValidationError,
        };
        use crate::protocol::P2;
        use crate::transport::MockTime;
        use crate::units::HardwareErrors;

        /// `instruction` of `register` at `addr` (`len` bytes) on actuator 1.
        fn on_1(instruction: Instruction, register: &'static str, addr: u16, len: u16) -> Request {
            Request::NONE
                .instruction(instruction)
                .id(id(1))
                .register(register)
                .addr(addr)
                .len(len)
        }

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
            script(
                &mut bus,
                &[
                    p2_status(1, 0x80, &2000i32.to_le_bytes()),
                    p2_status(1, 0x80, &[0x04]),
                    p2_status(1, 0x80, &[80]),
                    p2_status(1, 0x80, &[72]),
                ],
            );
            let mark = mark(&bus);
            let mut xm = bus.actuator(id(1)).unwrap();
            let err = xm.present_position_raw().unwrap_err();
            assert_eq!(
                err,
                Error::Fault(FaultError::Temperature(TemperatureError::AboveLimit {
                    id: id(1),
                    measured: Celsius(80),
                    limit: Celsius(72),
                    faults: HardwareErrors::from_raw(0x04),
                    request: on_1(Instruction::Read, "Present Position", 132, 4),
                }))
            );
            assert_eq!(
                std::string::ToString::to_string(&err),
                "id 1: temperature 80 °C above 72 °C"
            );
            assert_eq!(xm.torque_state(), TorqueState::Faulted);
            assert_eq!(
                sent_since(&bus, mark),
                concat(&[
                    p2_read(1, 132, 4),
                    p2_read(1, 70, 1),
                    p2_read(1, 146, 1),
                    p2_read(1, 31, 1),
                ])
            );
        }

        #[test]
        fn failed_diagnosis_still_faults() {
            let time = MockTime::new();
            let mut bus = xm430_on(&time, 3);
            line(&mut bus).reply(&p2_status(1, 0x80, &2000i32.to_le_bytes()));
            let mut xm = bus.actuator(id(1)).unwrap();
            assert_eq!(
                xm.present_position_raw(),
                Err(StatusError {
                    id: id(1),
                    report: StatusReport::P2 {
                        alert: true,
                        code: None,
                    },
                    request: on_1(Instruction::Read, "Present Position", 132, 4),
                }
                .into())
            );
            assert_eq!(xm.torque_state(), TorqueState::Faulted);
            let mut bus = xm430(&time, 3);
            line(&mut bus).reply(&p2_status(1, 0x80, &[]));
            let mut xm = bus.actuator(id(1)).unwrap();
            assert!(xm.enable_torque().is_err());
            assert_eq!(xm.torque_state(), TorqueState::Faulted);
        }

        #[test]
        fn hardware_fault_on_torque_write_faults() {
            let time = MockTime::new();
            let mut bus = xm430(&time, 3);
            script(
                &mut bus,
                &[p2_status(1, 0x80, &[]), p2_status(1, 0x80, &[0x20])],
            );
            let mut xm = bus.actuator(id(1)).unwrap();
            assert_eq!(
                xm.enable_torque(),
                Err(FaultError::Overload {
                    id: id(1),
                    faults: HardwareErrors::from_raw(0x20),
                    request: on_1(Instruction::Write, "Torque Enable", 64, 1),
                }
                .into())
            );
            assert_eq!(xm.torque_state(), TorqueState::Faulted);
        }

        fn faulted(time: &MockTime) -> TestBus<'_, P2> {
            let mut bus = xm430_on(time, 3);
            script(
                &mut bus,
                &[
                    p2_status(1, 0x80, &2000i32.to_le_bytes()),
                    p2_status(1, 0x80, &[0x20]),
                ],
            );
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
            assert_eq!(
                xm.enable_torque(),
                Err(TransportError::NoReply {
                    request: on_1(Instruction::Write, "Torque Enable", 64, 1),
                    timeout_us: 10_000,
                }
                .into())
            );
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
                Err(ValidationError::NotFinite {
                    request: on_1(Instruction::Write, "Goal Position", 116, 4),
                }
                .into())
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
            assert_eq!(
                ax.present_position_raw(),
                Err(crate::error::FaultError::Overload {
                    id: id(1),
                    faults: crate::units::HardwareErrors::from_raw(0x20),
                    request: crate::error::Request::NONE
                        .instruction(crate::error::Instruction::Read)
                        .id(id(1))
                        .register("Present Position")
                        .addr(36)
                        .len(2),
                }
                .into())
            );
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
