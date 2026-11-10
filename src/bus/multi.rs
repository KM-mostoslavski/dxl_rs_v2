//! Commands to many actuators in one call (research R-7, R-8, SRS FR-9.5):
//! the Easy tier (`enable_torque`, `disable_torque`,
//! `write_goal_positions[_raw]`, `write_goal_velocities`,
//! `write_goal_currents`, `read_positions[_raw]`, `read_field`) and the
//! explicit `sync_write` and, on Protocol 2, `sync_read`.

use crate::actuator::{Actuator, Describe, TorqueState};
use crate::error::{Error, Request};
use crate::protocol::{Clock, P2, Protocol};
use crate::units::{Current, Position, Velocity};
use crate::{Id, Transport};

use super::Bus;
use super::diagnose::diagnosed;
use super::field::{
    self, GoalCurrent, GoalPosition, GoalVelocity, PresentPosition, Raw, ReadField, WriteField,
};
use super::group::{self, FirstError, Stage};
use super::observe::{after_torque_write, ran, reached};
use super::slots::Slots;
use super::wire::{AnyProtocol, ReadMany, SyncRead, Wire};

/// Enables torque on every target, in target order, after checking that
/// none is faulted. Stops at the first error.
pub(super) fn torque_on<P, T, C, S, I>(
    wire: &mut Wire<'_, P, T, C>,
    slots: &mut S,
    stages: &mut [Stage],
    ids: I,
) -> Result<(), Error>
where
    P: Protocol,
    T: Transport,
    C: Clock,
    S: Slots,
    I: IntoIterator<Item = Id>,
{
    let (kinds, states, _, marks) = slots.split_mut();
    let stages = group::resolve(kinds, ids, stages)?;
    for stage in stages.iter() {
        let (kind, state) = group::held(kinds, states, stage)?;
        state.enable(kind.id())?;
    }
    for stage in stages.iter() {
        let (kind, state) = group::held(kinds, states, stage)?;
        let result = wire.write_value(kind.id(), kind.data().torque_enable, 1);
        if reached(&result)
            && let Some(marks) = marks.get_mut(usize::from(stage.slot))
        {
            marks.forget_torque_off();
        }
        let result = diagnosed(wire, kind, result);
        after_torque_write(state, TorqueState::On, &result);
        result?;
    }
    Ok(())
}

/// Disables torque on every target, in target order, even after an error.
/// Returns the first hardware fault, else the first error.
pub(super) fn torque_off<P, T, C, S, I>(
    wire: &mut Wire<'_, P, T, C>,
    slots: &mut S,
    stages: &mut [Stage],
    ids: I,
) -> Result<(), Error>
where
    P: Protocol,
    T: Transport,
    C: Clock,
    S: Slots,
    I: IntoIterator<Item = Id>,
{
    let (kinds, states, _, marks) = slots.split_mut();
    let stages = group::resolve(kinds, ids, stages)?;
    let mut first = FirstError::default();
    for (index, stage) in stages.iter().enumerate() {
        let (kind, state) = group::held(kinds, states, stage)?;
        let wanted = state.disable();
        let result = wire.write_value(kind.id(), kind.data().torque_enable, 0);
        if let Some(marks) = marks.get_mut(usize::from(stage.slot)) {
            marks.torque_off_written(ran(&result));
        }
        let result = diagnosed(wire, kind, result);
        after_torque_write(state, wanted, &result);
        first.note(index, &result);
    }
    first.into_diagnosed(wire, kinds)
}

/// Validates every target, then sends one Sync Write of field `F` per
/// register layout.
pub(super) fn write_field<P, T, C, S, F, I>(
    wire: &mut Wire<'_, P, T, C>,
    slots: &mut S,
    stages: &mut [Stage],
    ids: I,
    values: &[F::Value],
) -> Result<(), Error>
where
    P: Protocol,
    T: Transport,
    C: Clock,
    S: Slots,
    F: WriteField,
    I: IntoIterator<Item = Id>,
{
    let (kinds, states, bounds, marks) = slots.split_mut();
    let stages = group::resolve(kinds, ids, stages)?;
    group::check_values(stages.len(), values.len())?;
    group::validate_writes::<F>(kinds, states, bounds, stages, values)?;
    group::forget_speed(kinds, marks, stages);
    for (addr, size) in group::groups(stages) {
        wire.set_context(group::layout_context::<F>(kinds, stages, (addr, size)));
        let sent = wire.sync_write(addr, size, group::entries(kinds, stages, (addr, size)));
        wire.set_context(Request::NONE);
        sent?;
    }
    Ok(())
}

/// Reads field `F` of every target into `out[..targets]`, one `R::run` per
/// register layout. `out` changes only when every target answered.
pub(super) fn read_field<P, T, C, S, F, R, I>(
    wire: &mut Wire<'_, P, T, C>,
    slots: &mut S,
    stages: &mut [Stage],
    ids: I,
    out: &mut [F::Value],
) -> Result<(), Error>
where
    P: Protocol,
    T: Transport,
    C: Clock,
    S: Slots,
    F: ReadField,
    R: ReadMany<P, T, C>,
    I: IntoIterator<Item = Id>,
{
    let (kinds, states, _, _) = slots.split_mut();
    let stages = group::resolve(kinds, ids, stages)?;
    group::check_out(stages.len(), out.len())?;
    group::stage_reads::<F>(kinds, stages)?;
    group::read_stages::<P, T, C, F, R>(wire, kinds, states, stages)?;
    for (value, stage) in out.iter_mut().zip(stages.iter()) {
        if let Some(kind) = kinds.get(usize::from(stage.slot)) {
            *value = field::decode::<F>(kind, stage.raw())?;
        }
    }
    Ok(())
}

/// Many actuators in one call.
///
/// Targets are any `IntoIterator<Item = Id>` (an array, a slice iterator,
/// an [`IdSet`](crate::IdSet), [`Bus::ids`]) in the caller's order. The
/// whole call is validated before any byte: a target the bus does not hold
/// ([`SetupError::NotOnBus`]), a target named twice
/// ([`ValidationError::DuplicateId`]), a value count other than the target
/// count ([`ValidationError::BufferLength`]), and each target's torque state
/// and limit mode. A refusal sends nothing to anyone. An empty target list
/// sends nothing and returns `Ok`.
///
/// [`SetupError::NotOnBus`]: crate::SetupError::NotOnBus
/// [`ValidationError::DuplicateId`]: crate::ValidationError::DuplicateId
/// [`ValidationError::BufferLength`]: crate::ValidationError::BufferLength
impl<P: Protocol, T: Transport, C: Clock, const N: usize> Bus<P, T, C, N> {
    /// Turns torque on for every target: one acknowledged Torque Enable
    /// write each, in target order.
    ///
    /// Each target written to is no longer confirmed off, whatever its
    /// reply: limit writes to it are refused until an acknowledged
    /// [`disable_torque`](Bus::disable_torque).
    ///
    /// # Errors
    ///
    /// Refused with nothing sent:
    /// - [`SetupError::NotOnBus`], [`ValidationError::DuplicateId`]: see
    ///   the targets rules above.
    /// - [`ValidationError::ActuatorFaulted`]: a target is
    ///   [`Faulted`](TorqueState::Faulted), naming the first one; clear its
    ///   fault first.
    ///
    /// Otherwise the first error of a write, after which no further target
    /// is enabled. Targets written before it stay `On`; the failed one is
    /// `Off`, or `Faulted` if its reply reported a hardware fault.
    ///
    /// [`SetupError::NotOnBus`]: crate::SetupError::NotOnBus
    /// [`ValidationError::DuplicateId`]: crate::ValidationError::DuplicateId
    /// [`ValidationError::ActuatorFaulted`]: crate::ValidationError::ActuatorFaulted
    pub fn enable_torque<I: IntoIterator<Item = Id>>(&mut self, ids: I) -> Result<(), Error> {
        let mut stages = [Stage::EMPTY; N];
        let (mut wire, mut slots) = self.split();
        torque_on(&mut wire, &mut slots, &mut stages, ids)
    }

    /// Turns torque off for every target: one acknowledged Torque Enable
    /// write each, in target order, sent to every target even after an
    /// error.
    ///
    /// Each target's state follows its own write, as
    /// [`ActuatorRef::disable_torque`](crate::ActuatorRef::disable_torque)
    /// does: `Off`, except that a `Faulted` target stays `Faulted` and a
    /// reply reporting a hardware fault gives `Faulted`. A reply that shows
    /// the write ran (no error, or only a hardware fault) confirms that
    /// target's torque off for limit writes, as there.
    ///
    /// # Errors
    ///
    /// - [`SetupError::NotOnBus`], [`ValidationError::DuplicateId`], with
    ///   nothing sent.
    /// - Otherwise the first reply reporting a hardware fault, else the
    ///   first error of any write.
    ///
    /// [`SetupError::NotOnBus`]: crate::SetupError::NotOnBus
    /// [`ValidationError::DuplicateId`]: crate::ValidationError::DuplicateId
    pub fn disable_torque<I: IntoIterator<Item = Id>>(&mut self, ids: I) -> Result<(), Error> {
        let mut stages = [Stage::EMPTY; N];
        let (mut wire, mut slots) = self.split();
        torque_off(&mut wire, &mut slots, &mut stages, ids)
    }

    /// Sends goal positions in radians: `goals[i]` to target `i`, one Sync
    /// Write per register layout.
    ///
    /// Each goal is converted to the nearest tick of its model and checked
    /// like [`ActuatorRef::set_goal_position`](crate::ActuatorRef::set_goal_position)
    /// does. Limits and operating mode are those read at Setup, or written
    /// since through the bus (`set_position_limits`, `set_operating_mode`).
    /// Position mode applies the EEPROM limits. Extended and current-based
    /// position modes and MX multi-turn apply none, as the firmware does;
    /// only the register size is checked. Velocity, current, PWM and wheel
    /// modes refuse every goal (`NotPositionMode`). A mode changed otherwise
    /// is not seen until the next Setup.
    ///
    /// A Sync Write gets no reply, so a fault shows on the next read. The
    /// call is all-or-nothing at validation only: targets with different
    /// register layouts (e.g. an XM430 and an XL-320) get one instruction
    /// each, and a transport failure between two leaves the first sent.
    ///
    /// # Errors
    ///
    /// Refused with nothing sent, naming the first failing target in
    /// target order:
    /// - [`SetupError::NotOnBus`], [`ValidationError::DuplicateId`],
    ///   [`ValidationError::BufferLength`] (`goals.len()` is not the target
    ///   count).
    /// - [`ActuatorIsOff`]: a target's torque is not
    ///   [`On`](TorqueState::On).
    /// - [`NotFinite`], [`OutOfLimits`], [`NotPositionMode`],
    ///   [`DoesNotFit`].
    ///
    /// Otherwise a transport error of a Sync Write.
    #[cfg_attr(
        all(feature = "mock", feature = "rx"),
        doc = r#"
Two RX-28 on Protocol 1, moved and read together:

```
use dxl_rs_v2::{Bus, Config, Error, Id, MockTime, MockTransport, ModelNumber, P1, Position};

# let time = MockTime::new();
# let mut line = MockTransport::new(&time);
# line.reply_to(1, &[0xFF, 0xFF, 0x01, 0x02, 0x00, 0xFC])
#     .reply_to(2, &[0xFF, 0xFF, 0x01, 0x04, 0x00, 0x1C, 0x00, 0xDE])
#     .reply_to(3, &[0xFF, 0xFF, 0x01, 0x04, 0x00, 0x00, 0x00, 0xFA])
#     .reply_to(4, &[0xFF, 0xFF, 0x01, 0x04, 0x00, 0xFF, 0x03, 0xF8])
#     .reply_to(5, &[0xFF, 0xFF, 0x02, 0x02, 0x00, 0xFB])
#     .reply_to(6, &[0xFF, 0xFF, 0x02, 0x04, 0x00, 0x1C, 0x00, 0xDD])
#     .reply_to(7, &[0xFF, 0xFF, 0x02, 0x04, 0x00, 0x00, 0x00, 0xF9])
#     .reply_to(8, &[0xFF, 0xFF, 0x02, 0x04, 0x00, 0xFF, 0x03, 0xF7])
#     .reply_to(9, &[0xFF, 0xFF, 0x01, 0x02, 0x00, 0xFC])
#     .reply_to(10, &[0xFF, 0xFF, 0x02, 0x02, 0x00, 0xFB])
#     .reply_to(12, &[0xFF, 0xFF, 0x01, 0x04, 0x00, 0x00, 0x02, 0xF8])
#     .reply_to(13, &[0xFF, 0xFF, 0x02, 0x04, 0x00, 0x64, 0x02, 0x93]);
let mut bus = Bus::<P1, _, _>::new(line, time.ticker(10), Config::new());
let (left, right) = (Id::new(1)?, Id::new(2)?);
bus.declare([(left, ModelNumber(28)), (right, ModelNumber(28))])?;

// Refused while torque is off: nothing is sent.
let goals = [Position::ZERO, Position::from_radians(0.5)];
assert!(bus.write_goal_positions([left, right], &goals).is_err());

bus.enable_torque(bus.ids())?;
bus.write_goal_positions([left, right], &goals)?;

// One Read per actuator on Protocol 1, into a buffer the caller owns.
let mut present = [0; 2];
bus.read_positions_raw(bus.ids(), &mut present)?;
assert_eq!(present, [512, 612]);
# let (line, _) = bus.into_parts();
# line.assert_done();
# Ok::<(), Error>(())
```
"#
    )]
    ///
    /// [`SetupError::NotOnBus`]: crate::SetupError::NotOnBus
    /// [`ValidationError::DuplicateId`]: crate::ValidationError::DuplicateId
    /// [`ValidationError::BufferLength`]: crate::ValidationError::BufferLength
    /// [`ActuatorIsOff`]: crate::ValidationError::ActuatorIsOff
    /// [`NotFinite`]: crate::ValidationError::NotFinite
    /// [`OutOfLimits`]: crate::ValidationError::OutOfLimits
    /// [`NotPositionMode`]: crate::ValidationError::NotPositionMode
    /// [`DoesNotFit`]: crate::ValidationError::DoesNotFit
    pub fn write_goal_positions<I: IntoIterator<Item = Id>>(
        &mut self,
        ids: I,
        goals: &[Position],
    ) -> Result<(), Error> {
        self.sync_write(GoalPosition, ids, goals)
    }

    /// Sends goal positions in ticks: `goals[i]` to target `i`, one Sync
    /// Write per register layout.
    ///
    /// Same rules as [`write_goal_positions`](Bus::write_goal_positions)
    /// without the conversion: limits and operating mode are those read at
    /// Setup, position mode applies the EEPROM limits, extended position
    /// modes only the register size, other modes refuse. Not atomic across
    /// layouts on the wire; a fault shows on the next read.
    ///
    /// # Errors
    ///
    /// As [`write_goal_positions`](Bus::write_goal_positions), without
    /// `NotFinite`.
    pub fn write_goal_positions_raw<I: IntoIterator<Item = Id>>(
        &mut self,
        ids: I,
        goals: &[i32],
    ) -> Result<(), Error> {
        self.sync_write(Raw(GoalPosition), ids, goals)
    }

    /// Sends goal velocities in rad/s: `goals[i]` to target `i`, one Sync
    /// Write per register layout (Goal Velocity, or Moving Speed in wheel
    /// mode on Protocol 1).
    ///
    /// Each target is checked as
    /// [`ActuatorRef::set_goal_velocity`](crate::ActuatorRef::set_goal_velocity)
    /// checks it: torque on, velocity or wheel mode, within the Velocity
    /// Limit read at Setup. A refusal sends nothing to anyone. Not atomic
    /// across layouts on the wire; a fault shows on the next read.
    ///
    /// # Errors
    ///
    /// As [`sync_write`](Bus::sync_write).
    pub fn write_goal_velocities<I: IntoIterator<Item = Id>>(
        &mut self,
        ids: I,
        goals: &[Velocity],
    ) -> Result<(), Error> {
        self.sync_write(GoalVelocity, ids, goals)
    }

    /// Sends goal currents in amperes: `goals[i]` to target `i`, one Sync
    /// Write per register layout (Goal Current, or Goal Torque on MX
    /// Protocol 1).
    ///
    /// Each target is checked as
    /// [`ActuatorRef::set_goal_current`](crate::ActuatorRef::set_goal_current)
    /// checks it: torque on, current or current-based position mode, within
    /// the Current Limit read at Setup. A refusal sends nothing to anyone.
    ///
    /// # Errors
    ///
    /// As [`sync_write`](Bus::sync_write).
    pub fn write_goal_currents<I: IntoIterator<Item = Id>>(
        &mut self,
        ids: I,
        goals: &[Current],
    ) -> Result<(), Error> {
        self.sync_write(GoalCurrent, ids, goals)
    }

    /// Writes field `F` to every target: `values[i]` to target `i`, one Sync
    /// Write per register layout. Both protocols; any writable
    /// [`field`](crate::field).
    ///
    /// Each target is checked as a single
    /// [`ActuatorRef::write`](crate::ActuatorRef::write) checks it, all of
    /// it before any byte, and a refusal sends nothing to anyone:
    /// `bus.sync_write(GoalPosition, ids, &goals)` is
    /// [`write_goal_positions`](Bus::write_goal_positions), and
    /// `bus.sync_write(Raw(GoalPosition), ids, &ticks)` is
    /// [`write_goal_positions_raw`](Bus::write_goal_positions_raw). Targets
    /// whose models put the field at different addresses get one
    /// instruction each, with each model's own scale. Not atomic across
    /// layouts on the wire; a fault shows on the next read.
    ///
    /// A Sync Write gets no reply, so one of Moving Speed (the speed limit,
    /// or the goal velocity in wheel mode, on AX, RX and MX on Protocol 1)
    /// leaves the bus not knowing it is 0, even when it writes 0: a later
    /// switch to wheel mode needs an acknowledged
    /// [`set_speed_limit`](crate::ActuatorRef::set_speed_limit) first.
    ///
    /// # Errors
    ///
    /// Refused with nothing sent, naming the first failing target in
    /// target order: the target errors of
    /// [`write_goal_positions`](Bus::write_goal_positions), and every
    /// refusal of [`ActuatorRef::write`](crate::ActuatorRef::write)
    /// (`Unsupported`, `ActuatorIsOff`, the mode errors, `NoUnit`,
    /// `NotFinite`, `DoesNotFit`, `OutOfLimits`). Otherwise a transport
    /// error of a Sync Write.
    pub fn sync_write<F: WriteField, I: IntoIterator<Item = Id>>(
        &mut self,
        field: F,
        ids: I,
        values: &[F::Value],
    ) -> Result<(), Error> {
        let _ = field;
        let mut stages = [Stage::EMPTY; N];
        let (mut wire, mut slots) = self.split();
        write_field::<P, T, C, _, F, I>(&mut wire, &mut slots, &mut stages, ids, values)
    }

    /// Reads Present Position of every target, in radians: `out[i]` is
    /// target `i`. Protocol 2 sends one Sync Read per register layout,
    /// Protocol 1 one Read per target.
    ///
    /// Strict: `out` changes only if every target answered. Extra slots of
    /// `out` stay untouched. Every reply passes the fault check, so a target
    /// reporting a hardware fault is [`Faulted`](TorqueState::Faulted) when
    /// the call returns. Not retried, whatever [`Config::retries`] says.
    ///
    /// # Errors
    ///
    /// Refused with nothing sent:
    /// - [`SetupError::NotOnBus`], [`ValidationError::DuplicateId`].
    /// - [`ValidationError::BufferLength`]: `out` is shorter than the
    ///   target count.
    ///
    /// On the wire, with `out` unchanged: the first reply reporting a
    /// hardware fault in target order, else the first error in target order
    /// (a timeout, a corrupted reply, a status error). A failing transport
    /// stops the call at once.
    ///
    /// [`Config::retries`]: crate::Config::retries
    /// [`SetupError::NotOnBus`]: crate::SetupError::NotOnBus
    /// [`ValidationError::DuplicateId`]: crate::ValidationError::DuplicateId
    /// [`ValidationError::BufferLength`]: crate::ValidationError::BufferLength
    pub fn read_positions<I: IntoIterator<Item = Id>>(
        &mut self,
        ids: I,
        out: &mut [Position],
    ) -> Result<(), Error> {
        self.read::<PresentPosition, AnyProtocol, I>(ids, out)
    }

    /// Reads Present Position of every target, in ticks: `out[i]` is target
    /// `i`. Same rules as [`read_positions`](Bus::read_positions).
    ///
    /// # Errors
    ///
    /// As [`read_positions`](Bus::read_positions).
    pub fn read_positions_raw<I: IntoIterator<Item = Id>>(
        &mut self,
        ids: I,
        out: &mut [i32],
    ) -> Result<(), Error> {
        self.read::<Raw<PresentPosition>, AnyProtocol, I>(ids, out)
    }

    /// Reads field `F` of every target: `out[i]` is target `i`. Both
    /// protocols; any readable [`field`](crate::field). Protocol 2 sends one
    /// Sync Read per register layout, Protocol 1 one Read per target.
    ///
    /// Each value is converted with its own model's scale. The same rules
    /// as [`read_positions`](Bus::read_positions): strict, `out` changes
    /// only if every target answered, a hardware fault marks its target
    /// `Faulted`, not retried.
    #[cfg_attr(
        all(feature = "mock", feature = "rx"),
        doc = r#"
Temperatures of two RX-28 on Protocol 1, one Read each:

```
use dxl_rs_v2::field::PresentTemperature;
use dxl_rs_v2::{Bus, Config, Error, Id, MockTime, MockTransport, ModelNumber, P1, Temperature};

# let time = MockTime::new();
# let mut line = MockTransport::new(&time);
# line.reply_to(1, &[0xFF, 0xFF, 0x01, 0x02, 0x00, 0xFC])
#     .reply_to(2, &[0xFF, 0xFF, 0x01, 0x04, 0x00, 0x1C, 0x00, 0xDE])
#     .reply_to(3, &[0xFF, 0xFF, 0x01, 0x04, 0x00, 0x00, 0x00, 0xFA])
#     .reply_to(4, &[0xFF, 0xFF, 0x01, 0x04, 0x00, 0xFF, 0x03, 0xF8])
#     .reply_to(5, &[0xFF, 0xFF, 0x02, 0x02, 0x00, 0xFB])
#     .reply_to(6, &[0xFF, 0xFF, 0x02, 0x04, 0x00, 0x1C, 0x00, 0xDD])
#     .reply_to(7, &[0xFF, 0xFF, 0x02, 0x04, 0x00, 0x00, 0x00, 0xF9])
#     .reply_to(8, &[0xFF, 0xFF, 0x02, 0x04, 0x00, 0xFF, 0x03, 0xF7])
#     .reply_to(9, &[0xFF, 0xFF, 0x01, 0x03, 0x00, 0x20, 0xDB])
#     .reply_to(10, &[0xFF, 0xFF, 0x02, 0x03, 0x00, 0x21, 0xD9]);
let mut bus = Bus::<P1, _, _>::new(line, time.ticker(10), Config::new());
bus.declare([(Id::new(1)?, ModelNumber(28)), (Id::new(2)?, ModelNumber(28))])?;
let mut celsius = [Temperature::default(); 2];
bus.read_field(PresentTemperature, bus.ids(), &mut celsius)?;
assert_eq!(celsius.map(Temperature::celsius), [32.0, 33.0]);
# Ok::<(), Error>(())
```
"#
    )]
    ///
    /// # Errors
    ///
    /// As [`read_positions`](Bus::read_positions), and refused with nothing
    /// sent for a target whose model lacks the register
    /// ([`ValidationError::Unsupported`]) or gives it no unit in its mode
    /// ([`ValidationError::NoUnit`]).
    ///
    /// [`ValidationError::Unsupported`]: crate::ValidationError::Unsupported
    /// [`ValidationError::NoUnit`]: crate::ValidationError::NoUnit
    pub fn read_field<F: ReadField, I: IntoIterator<Item = Id>>(
        &mut self,
        field: F,
        ids: I,
        out: &mut [F::Value],
    ) -> Result<(), Error> {
        let _ = field;
        self.read::<F, AnyProtocol, I>(ids, out)
    }

    /// Reads field `F` of `ids` into `out` through `R`.
    fn read<F: ReadField, R: ReadMany<P, T, C>, I: IntoIterator<Item = Id>>(
        &mut self,
        ids: I,
        out: &mut [F::Value],
    ) -> Result<(), Error> {
        let mut stages = [Stage::EMPTY; N];
        let (mut wire, mut slots) = self.split();
        read_field::<P, T, C, _, F, R, I>(&mut wire, &mut slots, &mut stages, ids, out)
    }
}

/// The Protocol 2 explicit tier.
impl<T: Transport, C: Clock, const N: usize> Bus<P2, T, C, N> {
    /// Reads field `F` of every target with Sync Read: `out[i]` is target
    /// `i`, one instruction per register layout. Protocol 2 only; any
    /// readable [`field`](crate::field).
    ///
    /// The same rules as [`read_field`](Bus::read_field) and
    /// [`read_positions`](Bus::read_positions):
    /// `bus.sync_read(PresentPosition, ids, &mut out)` is `read_positions`,
    /// and `bus.sync_read(Raw(PresentPosition), ids, &mut ticks)` is
    /// `read_positions_raw`. Strict: `out` changes only if every target
    /// answered; a hardware fault marks its target `Faulted`.
    ///
    /// Protocol 1 has no Sync Read, so the method does not exist there:
    ///
    /// ```compile_fail,E0599
    /// use dxl_rs_v2::field::PresentPosition;
    /// use dxl_rs_v2::{Bus, Clock, Id, P1, Position, Transport};
    ///
    /// fn read<T: Transport, C: Clock>(bus: &mut Bus<P1, T, C>, ids: [Id; 2]) {
    ///     let mut out = [Position::ZERO; 2];
    ///     let _ = bus.sync_read(PresentPosition, ids, &mut out);
    /// }
    /// ```
    ///
    /// Use [`read_positions`](Bus::read_positions) on either protocol.
    ///
    /// # Errors
    ///
    /// As [`read_field`](Bus::read_field).
    pub fn sync_read<F: ReadField, I: IntoIterator<Item = Id>>(
        &mut self,
        field: F,
        ids: I,
        out: &mut [F::Value],
    ) -> Result<(), Error> {
        let _ = field;
        self.read::<F, SyncRead, I>(ids, out)
    }
}

#[cfg(test)]
mod tests {
    use crate::bus::testing::*;
    use crate::error::{Error, FaultError, Instruction, Request, SetupError, ValidationError};
    use crate::protocol::P2;
    use crate::transport::MockTime;
    use crate::units::HardwareErrors;

    /// `instruction` of `register` at `addr` (`len` bytes) on `n`.
    fn on(instruction: Instruction, n: u8, register: &'static str, addr: u16, len: u16) -> Request {
        Request::NONE
            .instruction(instruction)
            .id(id(n))
            .register(register)
            .addr(addr)
            .len(len)
    }

    /// The overload `n` reports in reply to `request`, measured.
    fn overload(n: u8, request: Request) -> Error {
        FaultError::Overload {
            id: id(n),
            faults: HardwareErrors::from_raw(0x20),
            request,
        }
        .into()
    }

    fn no_reply(request: Request) -> Error {
        crate::TransportError::NoReply {
            request,
            timeout_us: 10_000,
        }
        .into()
    }

    #[test]
    fn empty_bus_and_empty_targets_send_nothing() {
        let time = MockTime::new();
        let mut bus = test_bus::<P2, 32>(&time);
        let mark = mark(&bus);
        assert_eq!(bus.enable_torque([]), Ok(()));
        assert_eq!(bus.disable_torque([]), Ok(()));
        assert_eq!(bus.write_goal_positions([], &[]), Ok(()));
        assert_eq!(bus.write_goal_positions_raw(bus.ids(), &[]), Ok(()));
        assert_eq!(
            bus.write_goal_positions_raw([id(1)], &[0]),
            Err(SetupError::NotOnBus { id: id(1) }.into())
        );
        assert_eq!(
            bus.write_goal_positions_raw([], &[0]),
            Err(ValidationError::BufferLength {
                targets: 0,
                got: 1,
                request: Request::NONE,
            }
            .into())
        );
        assert_quiet(&bus, mark);
    }

    #[test]
    fn ids_lets_a_call_name_every_held_actuator() {
        let time = MockTime::new();
        let mut bus = test_bus::<P2, 32>(&time);
        assert!(bus.ids().is_empty());
        assert_eq!(bus.enable_torque(bus.ids()), Ok(()));
        assert_eq!(bus.disable_torque(bus.ids()), Ok(()));
    }

    #[cfg(feature = "x-series")]
    mod x_series {
        use super::super::*;
        use super::*;
        use crate::actuator::LimitMode;
        use crate::error::TransportError;
        use crate::field::{GoalPosition, Raw};

        /// XM430-W350 at IDs 1 and 2 (limits 1000..=3000) and an XL-320 at
        /// ID 3 (limits 0..=1023), in position mode, with torque `states`.
        fn three(time: &MockTime, states: [TorqueState; 3]) -> TestBus<'_, P2> {
            let mut bus = test_bus(time);
            hold(&mut bus, XM430, 1, (1000, 3000), BOUNDED, states[0]);
            hold(&mut bus, XM430, 2, (1000, 3000), BOUNDED, states[1]);
            hold(&mut bus, XL320, 3, (0, 1023), BOUNDED, states[2]);
            bus
        }

        fn ids(raw: &[u8]) -> impl Iterator<Item = Id> + '_ {
            raw.iter().map(|&n| id(n))
        }

        #[test]
        fn ids_are_the_held_ids() {
            let time = MockTime::new();
            let bus = three(&time, [ON; 3]);
            assert_eq!(bus.ids(), crate::IdSet::range(id(1), id(3)));
        }

        #[test]
        fn one_layout_is_one_sync_write_and_no_reply() {
            let time = MockTime::new();
            let mut bus = three(&time, [ON; 3]);
            let reads = line(&mut bus).reads();
            bus.write_goal_positions_raw(ids(&[2, 1]), &[1500, 2500])
                .unwrap();
            assert_eq!(
                sent(&bus),
                p2_sync_write(
                    116,
                    4,
                    &[(2, &1500i32.to_le_bytes()), (1, &2500i32.to_le_bytes())]
                )
            );
            assert_eq!(line(&mut bus).reads(), reads);
            assert_eq!(line(&mut bus).flushes(), 1);
        }

        #[test]
        fn two_layouts_are_two_sync_writes() {
            let time = MockTime::new();
            let mut bus = three(&time, [ON; 3]);
            bus.write_goal_positions_raw(ids(&[1, 3, 2]), &[1500, 600, 2500])
                .unwrap();
            assert_eq!(
                sent(&bus),
                concat(&[
                    p2_sync_write(
                        116,
                        4,
                        &[(1, &1500i32.to_le_bytes()), (2, &2500i32.to_le_bytes())]
                    ),
                    p2_sync_write(30, 2, &[(3, &600u16.to_le_bytes())]),
                ])
            );
            assert_eq!(line(&mut bus).flushes(), 2);
            assert_done(&bus);
        }

        #[test]
        fn typed_goals_are_the_register_values() {
            let time = MockTime::new();
            let mut bus = three(&time, [ON; 3]);
            let half = Position::from_radians(0.5);
            let xm = bus.actuators()[0].position_scale().to_raw(half).unwrap();
            bus.write_goal_positions(ids(&[1, 3]), &[half, Position::ZERO])
                .unwrap();
            assert_eq!(
                sent(&bus),
                concat(&[
                    p2_sync_write(116, 4, &[(1, &xm.to_le_bytes())]),
                    p2_sync_write(30, 2, &[(3, &512u16.to_le_bytes())]),
                ])
            );
        }

        #[test]
        fn one_target_off_refuses_everyone() {
            let time = MockTime::new();
            for (states, off) in [([ON, OFF, ON], 2), ([ON, ON, FAULTED], 3)] {
                let mut bus = three(&time, states);
                let mark = mark(&bus);
                assert_eq!(
                    bus.write_goal_positions_raw(ids(&[1, 2, 3]), &[2000, 2000, 500]),
                    Err(ValidationError::ActuatorIsOff { id: id(off) }.into())
                );
                assert_eq!(
                    bus.write_goal_positions(ids(&[3, 2, 1]), &[Position::ZERO; 3]),
                    Err(ValidationError::ActuatorIsOff { id: id(off) }.into())
                );
                assert_quiet(&bus, mark);
            }
        }

        #[test]
        fn mode_decides_per_target() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            hold(&mut bus, XM430, 1, (1000, 3000), LimitMode::Unbounded, ON);
            hold(&mut bus, XM430, 2, (1000, 3000), LimitMode::Pwm, ON);
            hold(&mut bus, XM430, 3, (1000, 3000), BOUNDED, ON);
            let mark = mark(&bus);
            assert_eq!(
                bus.write_goal_positions_raw(ids(&[1, 2]), &[9000, 2000]),
                Err(ValidationError::NotPositionMode { id: id(2) }.into())
            );
            assert_eq!(
                bus.write_goal_positions_raw(ids(&[1, 3]), &[9000, 9000]),
                Err(ValidationError::OutOfLimits {
                    id: id(3),
                    raw: 9000,
                    min: 1000,
                    max: 3000
                }
                .into())
            );
            assert_quiet(&bus, mark);
            bus.write_goal_positions_raw(ids(&[1, 3]), &[9000, 2000])
                .unwrap();
            assert_eq!(
                sent_since(&bus, mark),
                p2_sync_write(
                    116,
                    4,
                    &[(1, &9000i32.to_le_bytes()), (3, &2000i32.to_le_bytes())]
                )
            );
        }

        #[test]
        fn bad_targets_and_counts_send_nothing() {
            let time = MockTime::new();
            let mut bus = three(&time, [ON; 3]);
            let mark = mark(&bus);
            assert_eq!(
                bus.write_goal_positions_raw(ids(&[1, 2]), &[2000]),
                Err(ValidationError::BufferLength {
                    targets: 2,
                    got: 1,
                    request: Request::NONE,
                }
                .into())
            );
            assert_eq!(
                bus.write_goal_positions(ids(&[1]), &[Position::ZERO; 2]),
                Err(ValidationError::BufferLength {
                    targets: 1,
                    got: 2,
                    request: Request::NONE,
                }
                .into())
            );
            assert_eq!(
                bus.write_goal_positions_raw(ids(&[1, 9]), &[2000, 2000]),
                Err(SetupError::NotOnBus { id: id(9) }.into())
            );
            assert_eq!(
                bus.write_goal_positions_raw(ids(&[2, 1, 2]), &[2000; 3]),
                Err(ValidationError::DuplicateId { id: id(2) }.into())
            );
            assert_eq!(
                bus.write_goal_positions(ids(&[1]), &[Position::from_radians(f32::NAN)]),
                Err(ValidationError::NotFinite {
                    request: on(Instruction::Write, 1, "Goal Position", 116, 4),
                }
                .into())
            );
            assert_eq!(
                bus.enable_torque(ids(&[1, 9])),
                Err(SetupError::NotOnBus { id: id(9) }.into())
            );
            assert_eq!(
                bus.disable_torque(ids(&[1, 1])),
                Err(ValidationError::DuplicateId { id: id(1) }.into())
            );
            assert_quiet(&bus, mark);
        }

        #[test]
        fn explicit_sync_write_is_the_easy_tier() {
            let time = MockTime::new();
            let mut bus = three(&time, [ON; 3]);
            bus.sync_write(GoalPosition, ids(&[2]), &[Position::ZERO])
                .unwrap();
            bus.sync_write(Raw(GoalPosition), ids(&[3]), &[700])
                .unwrap();
            let mark = mark(&bus);
            assert_eq!(
                bus.sync_write(Raw(GoalPosition), ids(&[3]), &[1024]),
                Err(ValidationError::OutOfLimits {
                    id: id(3),
                    raw: 1024,
                    min: 0,
                    max: 1023
                }
                .into())
            );
            assert_quiet(&bus, mark);
            assert_eq!(
                sent(&bus),
                concat(&[
                    p2_sync_write(116, 4, &[(2, &2048i32.to_le_bytes())]),
                    p2_sync_write(30, 2, &[(3, &700u16.to_le_bytes())]),
                ])
            );
        }

        #[test]
        fn a_transport_error_aborts_between_layouts() {
            let time = MockTime::new();
            let mut bus = three(&time, [ON; 3]);
            line(&mut bus).fail_with(embedded_io::ErrorKind::BrokenPipe);
            assert_eq!(
                bus.write_goal_positions_raw(ids(&[1, 3]), &[2000, 500]),
                Err(TransportError::Io {
                    request: Request::NONE
                        .instruction(Instruction::SyncWrite)
                        .register("Goal Position")
                        .addr(116)
                        .len(4),
                    direction: crate::error::Direction::ReadReady,
                    kind: embedded_io::ErrorKind::BrokenPipe,
                }
                .into())
            );
        }

        #[test]
        fn enable_writes_each_target_in_order() {
            let time = MockTime::new();
            let mut bus = three(&time, [OFF; 3]);
            ack(&mut bus, 3);
            line(&mut bus).reply_to(2, &p2_status(1, 0, &[]));
            bus.enable_torque(ids(&[3, 1])).unwrap();
            assert_eq!(
                sent(&bus),
                concat(&[p2_write(3, 24, &[1]), p2_write(1, 64, &[1])])
            );
            assert_eq!(bus.torque_states(), [ON, OFF, ON]);
            assert_done(&bus);
        }

        #[test]
        fn enable_is_refused_if_any_target_is_faulted() {
            let time = MockTime::new();
            let mut bus = three(&time, [OFF, FAULTED, OFF]);
            let mark = mark(&bus);
            assert_eq!(
                bus.enable_torque(bus.ids()),
                Err(ValidationError::ActuatorFaulted { id: id(2) }.into())
            );
            assert_quiet(&bus, mark);
            assert_eq!(bus.torque_states(), [OFF, FAULTED, OFF]);
        }

        #[test]
        fn enable_stops_at_the_first_error() {
            let time = MockTime::new();
            let mut bus = three(&time, [OFF; 3]);
            ack(&mut bus, 1);
            assert_eq!(
                bus.enable_torque(bus.ids()),
                Err(no_reply(on(Instruction::Write, 2, "Torque Enable", 64, 1)))
            );
            assert_eq!(
                sent(&bus),
                concat(&[p2_write(1, 64, &[1]), p2_write(2, 64, &[1])])
            );
            assert_eq!(bus.torque_states(), [ON, OFF, OFF]);
        }

        #[test]
        fn enable_with_a_hardware_fault_faults_that_target() {
            let time = MockTime::new();
            let mut bus = three(&time, [OFF; 3]);
            line(&mut bus)
                .reply_to(1, &p2_status(1, 0x80, &[]))
                .reply_to(2, &p2_status(1, 0x80, &[0x20]));
            assert_eq!(
                bus.enable_torque(bus.ids()),
                Err(overload(
                    1,
                    on(Instruction::Write, 1, "Torque Enable", 64, 1)
                ))
            );
            assert_eq!(bus.torque_states(), [FAULTED, OFF, OFF]);
            assert_eq!(line(&mut bus).flushes(), 2);
        }

        #[test]
        fn disable_tries_every_target_and_prefers_a_fault() {
            let time = MockTime::new();
            let mut bus = three(&time, [ON, ON, FAULTED]);
            line(&mut bus)
                .reply_to(2, &p2_status(2, 0x80, &[]))
                .reply_to(3, &p2_status(2, 0x80, &[0x20]))
                .reply_to(4, &p2_status(3, 0, &[]));
            let result = bus.disable_torque(bus.ids());
            assert_eq!(
                result,
                Err(overload(
                    2,
                    on(Instruction::Write, 2, "Torque Enable", 64, 1)
                ))
            );
            assert_eq!(
                sent(&bus),
                concat(&[
                    p2_write(1, 64, &[0]),
                    p2_write(2, 64, &[0]),
                    p2_read(2, 70, 1),
                    p2_write(3, 24, &[0]),
                ])
            );
            assert_eq!(bus.torque_states(), [OFF, FAULTED, FAULTED]);
        }

        #[test]
        fn disable_returns_the_first_error_and_turns_off_the_rest() {
            let time = MockTime::new();
            let mut bus = three(&time, [ON; 3]);
            line(&mut bus).reply_to(3, &p2_status(3, 0, &[]));
            assert_eq!(
                bus.disable_torque(bus.ids()),
                Err(no_reply(on(Instruction::Write, 1, "Torque Enable", 64, 1)))
            );
            assert_eq!(bus.torque_states(), [OFF; 3]);
            assert_eq!(line(&mut bus).flushes(), 3);
        }

        mod prop {
            extern crate std;

            use super::*;
            use proptest::prelude::*;

            fn state() -> impl Strategy<Value = TorqueState> {
                prop_oneof![Just(ON), Just(OFF), Just(FAULTED)]
            }

            proptest! {
                #[test]
                fn no_goal_byte_for_a_slot_not_on(
                    targets in proptest::collection::vec((state(), -4.0f32..4.0), 1..=8),
                ) {
                    let time = MockTime::new();
                    let mut bus = test_bus::<P2, 32>(&time);
                    for (n, &(state, _)) in (1u8..).zip(&targets) {
                        hold(&mut bus, XM430, n, (0, 4095), BOUNDED, state);
                    }
                    let goals: std::vec::Vec<Position> =
                        targets.iter().map(|&(_, rad)| Position::from_radians(rad)).collect();
                    let all_on = targets.iter().all(|&(state, _)| state == ON);
                    let result = bus.write_goal_positions(bus.ids(), &goals);
                    if all_on {
                        prop_assert_eq!(result.is_ok(), !sent(&bus).is_empty());
                    } else {
                        prop_assert!(result.is_err());
                        prop_assert!(sent(&bus).is_empty());
                        prop_assert_eq!(line(&mut bus).flushes(), 0);
                    }
                }
            }
        }
    }

    #[cfg(feature = "ax")]
    mod ax {
        use super::super::*;
        use super::*;
        use crate::field::{GoalPosition, Raw};
        use crate::protocol::P1;

        #[test]
        fn p1_sync_write_and_torque() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            hold(&mut bus, AX12, 1, (0, 1023), BOUNDED, OFF);
            hold(&mut bus, AX12, 4, (0, 1023), BOUNDED, OFF);
            line(&mut bus)
                .reply_to(1, &p1_status(1, 0, &[]))
                .reply_to(2, &p1_status(4, 0, &[]));
            bus.enable_torque(bus.ids()).unwrap();
            bus.sync_write(GoalPosition, [id(4), id(1)], &[Position::ZERO; 2])
                .unwrap();
            bus.sync_write(Raw(GoalPosition), [id(1)], &[100]).unwrap();
            ack(&mut bus, 1);
            bus.disable_torque([id(1)]).unwrap();
            let center = 512u16.to_le_bytes();
            assert_eq!(
                sent(&bus),
                concat(&[
                    p1_write(1, 24, &[1]),
                    p1_write(4, 24, &[1]),
                    p1_sync_write(30, 2, &[(4, &center), (1, &center)]),
                    p1_sync_write(30, 2, &[(1, &100u16.to_le_bytes())]),
                    p1_write(1, 24, &[0]),
                ])
            );
            assert_eq!(bus.torque_states(), [OFF, ON]);
        }
    }

    #[cfg(feature = "rx")]
    mod rx {
        use super::*;
        use crate::protocol::P1;

        #[test]
        fn rx28_chain_moves_together() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            for n in [1, 2, 3] {
                hold(&mut bus, RX28, n, (0, 1023), BOUNDED, OFF);
            }
            for n in [1, 2, 3] {
                line(&mut bus).reply_to(usize::from(n), &p1_status(n, 0, &[]));
            }
            bus.enable_torque(bus.ids()).unwrap();
            assert_eq!(bus.torque_states(), [ON; 3]);
            let mark = mark(&bus);
            bus.write_goal_positions_raw(bus.ids(), &[100, 512, 900])
                .unwrap();
            assert_eq!(
                sent_since(&bus, mark),
                p1_sync_write(
                    30,
                    2,
                    &[
                        (1, &100u16.to_le_bytes()),
                        (2, &512u16.to_le_bytes()),
                        (3, &900u16.to_le_bytes()),
                    ]
                )
            );
            line(&mut bus)
                .reply_to(5, &p1_status(1, 0, &[]))
                .reply_to(6, &p1_status(2, 0x20, &[]))
                .reply_to(7, &p1_status(3, 0, &[]));
            assert_eq!(
                bus.disable_torque(bus.ids()),
                Err(overload(
                    2,
                    on(Instruction::Write, 2, "Torque Enable", 24, 1)
                ))
            );
            assert_eq!(bus.torque_states(), [OFF, FAULTED, OFF]);
            assert_eq!(
                bus.enable_torque(bus.ids()),
                Err(crate::ValidationError::ActuatorFaulted { id: id(2) }.into())
            );
        }
    }

    #[cfg(feature = "x-series")]
    mod x_series_reads {
        use super::super::*;
        use super::*;
        use crate::error::TransportError;
        use crate::field::{PresentPosition, Raw};

        const SENTINEL: i32 = -7;

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

        extern crate std;

        #[test]
        fn one_sync_read_fills_out_in_target_order() {
            let time = MockTime::new();
            let mut bus = three_xm(&time);
            line(&mut bus)
                .reply_to(1, &present(3, 300))
                .reply_to(1, &present(1, -100))
                .reply_to(1, &present(2, 4000));
            let mut out = [SENTINEL; 4];
            bus.read_positions_raw([id(3), id(1), id(2)], &mut out)
                .unwrap();
            assert_eq!(out, [300, -100, 4000, SENTINEL]);
            assert_eq!(sent(&bus), p2_sync_read(132, 4, &[3, 1, 2]));
            assert_done(&bus);
        }

        #[test]
        fn typed_reads_are_radians() {
            let time = MockTime::new();
            let mut bus = three_xm(&time);
            line(&mut bus)
                .reply_to(1, &present(1, 2048))
                .reply_to(1, &present(2, 3072));
            let scale = bus.actuators()[0].position_scale();
            let mut out = [Position::from_radians(9.0); 2];
            bus.read_positions([id(1), id(2)], &mut out).unwrap();
            assert_eq!(out, [scale.to_position(2048), scale.to_position(3072)]);
        }

        #[test]
        fn explicit_sync_read_is_the_easy_tier() {
            let time = MockTime::new();
            let mut bus = three_xm(&time);
            line(&mut bus)
                .reply_to(1, &present(2, 1000))
                .reply_to(2, &present(2, 1000));
            let mut typed = [Position::ZERO; 1];
            bus.sync_read(PresentPosition, [id(2)], &mut typed).unwrap();
            let mut raw = [0; 1];
            bus.sync_read(Raw(PresentPosition), [id(2)], &mut raw)
                .unwrap();
            assert_eq!(raw, [1000]);
            assert_eq!(
                typed,
                [bus.actuators()[1].position_scale().to_position(1000)]
            );
            let one = p2_sync_read(132, 4, &[2]);
            assert_eq!(sent(&bus), concat(&[one.clone(), one]));
        }

        #[test]
        fn mixed_layouts_are_one_sync_read_each() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            hold(&mut bus, XM430, 1, (0, 4095), BOUNDED, ON);
            hold(&mut bus, XL320, 3, (0, 1023), BOUNDED, ON);
            line(&mut bus)
                .reply_to(1, &p2_status(3, 0, &700u16.to_le_bytes()))
                .reply_to(2, &present(1, 2100));
            let mut out = [SENTINEL; 2];
            bus.read_positions_raw([id(3), id(1)], &mut out).unwrap();
            assert_eq!(out, [700, 2100]);
            assert_eq!(
                sent(&bus),
                concat(&[p2_sync_read(37, 2, &[3]), p2_sync_read(132, 4, &[1])])
            );
        }

        #[test]
        fn a_timeout_leaves_out_unchanged() {
            let time = MockTime::new();
            let mut bus = three_xm(&time);
            line(&mut bus)
                .reply_to(1, &present(1, 10))
                .reply_to(1, &present(3, 30));
            let mut out = [SENTINEL; 3];
            assert_eq!(
                bus.read_positions_raw(bus.ids(), &mut out),
                Err(TransportError::Skipped {
                    request: on(Instruction::SyncRead, 2, "Present Position", 132, 4),
                    answered: id(3),
                }
                .into())
            );
            assert_eq!(out, [SENTINEL; 3]);
            assert_eq!(bus.torque_states(), [OFF; 3]);
        }

        #[test]
        fn a_hardware_fault_wins_over_an_earlier_timeout() {
            let time = MockTime::new();
            let mut bus = three_xm(&time);
            line(&mut bus)
                .reply_to(1, &present(2, 20))
                .reply_to(1, &p2_status(3, 0x80, &30i32.to_le_bytes()))
                .reply_to(2, &p2_status(3, 0x80, &[0x20]));
            let mut out = [SENTINEL; 3];
            assert_eq!(
                bus.read_positions_raw(bus.ids(), &mut out),
                Err(overload(
                    3,
                    on(Instruction::SyncRead, 3, "Present Position", 132, 4)
                ))
            );
            assert_eq!(out, [SENTINEL; 3]);
            assert_eq!(bus.torque_states(), [OFF, OFF, FAULTED]);
        }

        #[test]
        fn faults_are_noted_across_layouts_in_target_order() {
            let time = MockTime::new();
            let mut bus = test_bus::<P2, 32>(&time);
            hold(&mut bus, XM430, 1, (0, 4095), BOUNDED, ON);
            hold(&mut bus, XL320, 2, (0, 1023), BOUNDED, ON);
            hold(&mut bus, XM430, 3, (0, 4095), BOUNDED, ON);
            line(&mut bus)
                .reply_to(1, &present(1, 1))
                .reply_to(1, &p2_status(3, 0x80, &3i32.to_le_bytes()))
                .reply_to(2, &p2_status(2, 0x80, &2u16.to_le_bytes()))
                .reply_to(3, &p2_status(2, 0x80, &[0x20]));
            let mut out = [SENTINEL; 3];
            let result = bus.read_positions_raw(bus.ids(), &mut out);
            assert_eq!(
                sent(&bus),
                concat(&[
                    p2_sync_read(132, 4, &[1, 3]),
                    p2_sync_read(37, 2, &[2]),
                    p2_read(2, 50, 1),
                ])
            );
            assert_eq!(
                result,
                Err(overload(
                    2,
                    on(Instruction::SyncRead, 2, "Present Position", 37, 2)
                ))
            );
            assert_eq!(bus.torque_states(), [ON, FAULTED, FAULTED]);
            assert_eq!(out, [SENTINEL; 3]);
        }

        #[test]
        fn refusals_send_nothing() {
            let time = MockTime::new();
            let mut bus = three_xm(&time);
            let mark = mark(&bus);
            let mut out = [SENTINEL; 2];
            assert_eq!(
                bus.read_positions_raw(bus.ids(), &mut out),
                Err(ValidationError::BufferLength {
                    targets: 3,
                    got: 2,
                    request: Request::NONE,
                }
                .into())
            );
            assert_eq!(
                bus.read_positions_raw([id(1), id(9)], &mut out),
                Err(SetupError::NotOnBus { id: id(9) }.into())
            );
            assert_eq!(
                bus.sync_read(PresentPosition, [id(1), id(1)], &mut [Position::ZERO; 2]),
                Err(ValidationError::DuplicateId { id: id(1) }.into())
            );
            assert_eq!(bus.read_positions([], &mut []), Ok(()));
            assert_eq!(bus.read_positions_raw([], &mut out), Ok(()));
            assert_eq!(out, [SENTINEL; 2]);
            assert_quiet(&bus, mark);
        }

        #[test]
        fn a_failing_transport_aborts() {
            let time = MockTime::new();
            let mut bus = three_xm(&time);
            line(&mut bus).fail_with(embedded_io::ErrorKind::BrokenPipe);
            let mut out = [SENTINEL; 3];
            assert_eq!(
                bus.read_positions_raw(bus.ids(), &mut out),
                Err(TransportError::Io {
                    request: Request::NONE
                        .instruction(Instruction::SyncRead)
                        .register("Present Position")
                        .addr(132)
                        .len(4),
                    direction: crate::error::Direction::ReadReady,
                    kind: embedded_io::ErrorKind::BrokenPipe,
                }
                .into())
            );
            assert_eq!(out, [SENTINEL; 3]);
        }
    }

    #[cfg(feature = "ax")]
    mod ax_reads {
        use super::super::*;
        use super::*;
        use crate::protocol::P1;

        #[test]
        fn p1_reads_each_target_in_order() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            for n in [1, 2, 3] {
                hold(&mut bus, AX12, n, (0, 1023), BOUNDED, ON);
            }
            line(&mut bus)
                .reply_to(1, &p1_status(3, 0, &300u16.to_le_bytes()))
                .reply_to(2, &p1_status(1, 0, &100u16.to_le_bytes()))
                .reply_to(3, &p1_status(2, 0, &200u16.to_le_bytes()));
            let mut out = [0; 3];
            bus.read_positions_raw([id(3), id(1), id(2)], &mut out)
                .unwrap();
            assert_eq!(out, [300, 100, 200]);
            assert_eq!(
                sent(&bus),
                concat(&[p1_read(3, 36, 2), p1_read(1, 36, 2), p1_read(2, 36, 2)])
            );
        }

        #[test]
        fn p1_silent_target_then_overload() {
            let time = MockTime::new();
            let mut bus = test_bus::<P1, 32>(&time);
            for n in [1, 2, 3] {
                hold(&mut bus, AX12, n, (0, 1023), BOUNDED, ON);
            }
            line(&mut bus)
                .reply_to(2, &p1_status(2, 0, &200u16.to_le_bytes()))
                .reply_to(3, &p1_status(3, 0x20, &300u16.to_le_bytes()));
            let mut out = [Position::ZERO; 3];
            let result = bus.read_positions(bus.ids(), &mut out);
            assert_eq!(
                result,
                Err(overload(
                    3,
                    on(Instruction::Read, 3, "Present Position", 36, 2)
                ))
            );
            assert_eq!(bus.torque_states(), [ON, ON, FAULTED]);
            assert_eq!(out, [Position::ZERO; 3]);
            assert_eq!(line(&mut bus).flushes(), 3);

            line(&mut bus).reply_to(5, &p1_status(2, 0, &200u16.to_le_bytes()));
            assert_eq!(
                bus.read_positions(bus.ids(), &mut out),
                Err(no_reply(on(
                    Instruction::Read,
                    1,
                    "Present Position",
                    36,
                    2
                )))
            );
        }
    }
}
