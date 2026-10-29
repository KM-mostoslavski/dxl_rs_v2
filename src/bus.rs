//! The bus: one wire, its actuators, and the rules every call follows.
//!
//! A [`Bus`] owns a transport, a clock and the actuators found at Setup.
//! [`Bus::declare`] sets it up from a list; [`Bus::actuator`] lends a
//! short-lived [`ActuatorRef`] to command one actuator.
//!
//! Rules that hold for every call:
//!
//! - Torque starts off. A goal is refused while torque is off or the
//!   actuator is faulted, and every refusal is decided before any byte is
//!   sent.
//! - Goals follow the limits and operating mode read at Setup: position
//!   mode applies the EEPROM limits, extended position modes only the
//!   register size, and other modes refuse goals.
//! - A reply reporting a hardware fault returns [`Error::Status`] and marks
//!   that actuator [`Faulted`](crate::TorqueState::Faulted) on the same call.
//!   Only [`ActuatorRef::clear_fault`] clears it, after asking the actuator.
//!   An X-series actuator needs a reboot outside this crate before its fault
//!   clears.
//! - Broadcast writes get no reply, so a loop that only writes sees a fault
//!   on its next read.
//! - Each unicast exchange waits at most [`Config::timeout`] for its reply
//!   and is tried again at most [`Config::retries`] times; see [`Config`]
//!   for the resulting time bounds.
#![cfg_attr(
    all(feature = "mock", feature = "x-series"),
    doc = r#"
One XM430-W350 at ID 1 on Protocol 2:

```
use dxl_rs_v2::{Actuator, Bus, Clock, Error, Id, ModelNumber, P2, Position, Transport};

fn move_one<T: Transport, C: Clock>(bus: &mut Bus<P2, T, C>, id: Id) -> Result<Position, Error> {
    bus.declare([(id, ModelNumber(1020))])?;
    let Some(mut xm) = bus.actuator(id) else {
        unreachable!("declared above");
    };
    assert_eq!(xm.name(), "XM430-W350");
    xm.enable_torque()?;
    xm.set_goal_position(Position::from_radians(0.5))?;
    xm.present_position()
}
# use dxl_rs_v2::{Config, MockTime, MockTransport};
# let time = MockTime::new();
# let mut line = MockTransport::new(&time);
# line.reply_to(1, &[0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x07, 0x00, 0x55, 0x00, 0xFC, 0x03, 0x34, 0x42, 0xC3])
#     .reply_to(2, &[0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x08, 0x00, 0x55, 0x00, 0x00, 0x00, 0x00, 0x00, 0xBF, 0xB8])
#     .reply_to(3, &[0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x08, 0x00, 0x55, 0x00, 0xFF, 0x0F, 0x00, 0x00, 0x5B, 0xB4])
#     .reply_to(4, &[0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x05, 0x00, 0x55, 0x00, 0x03, 0x59, 0x21])
#     .reply_to(5, &[0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x04, 0x00, 0x55, 0x00, 0xA1, 0x0C])
#     .reply_to(6, &[0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x04, 0x00, 0x55, 0x00, 0xA1, 0x0C])
#     .reply_to(7, &[0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x08, 0x00, 0x55, 0x00, 0x46, 0x09, 0x00, 0x00, 0x16, 0x40]);
# let mut bus = Bus::<P2, _, _>::new(line, time.ticker(10), Config::new());
let present = move_one(&mut bus, Id::new(1)?)?;
assert!((present.radians() - 0.5).abs() < 0.01);
# Ok::<(), Error>(())
```
"#
)]

mod actuator_ref;
mod observe;
#[cfg(feature = "std")]
mod serial;
mod setup;
mod slots;
#[cfg(test)]
mod testing;
mod wire;

use core::marker::PhantomData;
use core::time::Duration;

use crate::actuator::{ModelNumber, TorqueState};
use crate::error::Error;
use crate::models::ActuatorKind;
use crate::protocol::{Clock, Protocol};
use crate::{Id, Transport};

pub use actuator_ref::ActuatorRef;
#[cfg(feature = "std")]
pub use serial::SerialBus;

use slots::{FixedSlots, Slots, find};
use wire::Wire;

/// No clock. Implements no `Clock`, so a no_std bus must name its clock.
///
/// It is [`DefaultClock`] when the `std` feature is off: a `Bus` that does
/// not name its clock then has no constructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoClock {}

/// The clock a [`Bus`] uses when its type does not name one: `StdClock`
/// with the `std` feature, [`NoClock`] (so no constructor) without it.
#[cfg(feature = "std")]
pub type DefaultClock = crate::StdClock;

/// The clock a [`Bus`] uses when its type does not name one: `StdClock`
/// with the `std` feature, [`NoClock`] (so no constructor) without it.
#[cfg(not(feature = "std"))]
pub type DefaultClock = NoClock;

/// Timing of a [`Bus`]: how long to wait for replies and how often to try
/// again.
///
/// The defaults are a 10 ms reply timeout, a 10 ms scan window and no
/// retries:
///
/// ```
/// use core::time::Duration;
/// use dxl_rs_v2::Config;
///
/// let config = Config::new().timeout(Duration::from_millis(5)).retries(2);
/// assert_eq!(config.timeout, Duration::from_millis(5));
/// assert_eq!(config.scan_timeout, Duration::from_millis(10));
/// assert_eq!(config.retries, 2);
/// ```
///
/// Retries apply to unicast instructions that time out or get a corrupted
/// reply only. A status error, a refused value or a failing serial line is
/// returned at once.
///
/// A unicast exchange first drains stale input (it stops as soon as the line
/// is quiet and never runs past one timeout), then waits at most one timeout
/// for the reply. Worst case per unicast instruction:
///
/// - on a quiet line: `(retries + 1) × timeout`;
/// - on a line that never stops talking: `(retries + 1) × 2 × timeout`.
///
/// [`scan_timeout`](Config::scan_timeout) bounds [`Bus::autoscan`]: one
/// window for the Protocol 2 broadcast ping, one wait per ID for the
/// Protocol 1 sweep (253 IDs, about 2.5 s at the default when few answer).
/// Broadcast replies share one wire: each Protocol 2 ping reply is 14
/// bytes, about 140 bit-times plus the actuator's Return Delay Time, so a
/// long chain at a low baud rate needs a longer window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Config {
    /// Wait for one reply. Default 10 ms.
    pub timeout: Duration,
    /// P2 broadcast-ping window and P1 per-ID sweep wait. Default 10 ms.
    pub scan_timeout: Duration,
    /// Extra attempts of a unicast exchange after a timeout or corrupted
    /// reply. Default 0.
    pub retries: u8,
}

impl Config {
    /// The defaults: 10 ms timeout, 10 ms scan window, no retries.
    pub const fn new() -> Self {
        Self {
            timeout: Duration::from_millis(10),
            scan_timeout: Duration::from_millis(10),
            retries: 0,
        }
    }

    /// Sets the reply timeout.
    pub const fn timeout(self, timeout: Duration) -> Self {
        Self { timeout, ..self }
    }

    /// Sets the scan window.
    pub const fn scan_timeout(self, scan_timeout: Duration) -> Self {
        Self {
            scan_timeout,
            ..self
        }
    }

    /// Sets the number of extra attempts after a timeout or corrupted reply.
    pub const fn retries(self, retries: u8) -> Self {
        Self { retries, ..self }
    }
}

impl Default for Config {
    /// Same as [`Config::new`].
    fn default() -> Self {
        Self::new()
    }
}

/// A Dynamixel bus: one wire, one protocol `P`, a transport `T`, a clock `C`
/// and at most `N` actuators (default 32, at most 253).
///
/// Nothing is sent until Setup ([`declare`](Bus::declare) or
/// [`autoscan`](Bus::autoscan)). Setup reads each
/// actuator's model, firmware variant, EEPROM limits and operating mode,
/// and holds it with torque off. Each actuator is then reached through a
/// short-lived [`ActuatorRef`] from [`actuator`](Bus::actuator).
///
/// A reply reporting a hardware fault returns [`Error::Status`] and marks
/// that actuator [`Faulted`](TorqueState::Faulted) on the same call; only
/// [`ActuatorRef::clear_fault`] clears it, after asking the actuator. An
/// X-series actuator must be rebooted (outside this crate) before its fault
/// clears. Broadcast writes get no reply, so a loop that only writes sees a
/// fault on its next read.
///
/// The clock bounds every wait for a reply. Any `FnMut() -> u64` closure
/// returning microseconds is a [`Clock`]. With the `std` feature the clock
/// defaults to `StdClock`; without it the bus must name one, so
/// [`DefaultClock`] is [`NoClock`] and a bus without a clock cannot be built.
#[cfg_attr(
    not(feature = "std"),
    doc = r#"
```compile_fail,E0277
use dxl_rs_v2::{Bus, Config, DefaultClock, P2, Transport};

fn open<T: Transport>(uart: T, clock: DefaultClock) -> Bus<P2, T> {
    Bus::new(uart, clock, Config::new())
}
```
"#
)]
pub struct Bus<P: Protocol, T: Transport, C = DefaultClock, const N: usize = 32> {
    transport: T,
    clock: C,
    config: Config,
    /// Sorted by `Id`, no duplicates.
    actuators: heapless::Vec<ActuatorKind, N>,
    /// `states[i]` belongs to `actuators[i]`. Entries at `i >= len` are
    /// unused and kept `Off`.
    states: [TorqueState; N],
    protocol: PhantomData<P>,
}

impl<P: Protocol, T: Transport, C: Clock, const N: usize> Bus<P, T, C, N> {
    /// A bus over `transport`, timed by `clock`. No I/O.
    ///
    /// Compile error if `N > 253`: only 253 IDs exist.
    pub fn new(transport: T, clock: C, config: Config) -> Self {
        const { assert!(N <= 253, "a bus holds at most 253 actuators") };
        Self {
            transport,
            clock,
            config,
            actuators: heapless::Vec::new(),
            states: [TorqueState::Off; N],
            protocol: PhantomData,
        }
    }

    /// Gives the transport and the clock back. No I/O.
    pub fn into_parts(self) -> (T, C) {
        (self.transport, self.clock)
    }

    /// The config the bus was built with.
    pub fn config(&self) -> Config {
        self.config
    }

    /// Setup from a list. Pings only the listed IDs. Replaces the held list;
    /// empty on `Err`.
    ///
    /// For each listed actuator, in `Id` order, Setup pings it, checks its
    /// model number (from the ping on Protocol 2, a Model Number read on
    /// Protocol 1), reads the firmware where the model's table depends on
    /// it, the EEPROM position limits and the operating mode (none on
    /// models whose mode follows from the angle limits), and holds it with
    /// torque `Off`. Nothing is written. It never sweeps the bus.
    ///
    /// # Errors
    ///
    /// Before anything is sent:
    /// - [`SetupError::TooManyActuators`]: more than `N` entries.
    /// - [`ValidationError::DuplicateId`]: an `Id` listed twice.
    /// - [`SetupError::UnsupportedModel`]: a model this build has no table
    ///   for on `P` (its `feature` names the Cargo feature that adds it).
    ///
    /// On the wire:
    /// - [`SetupError::Missing`]: a listed actuator does not answer.
    /// - [`SetupError::WrongModel`]: it answers with another model number.
    /// - [`SetupError::UnknownOperatingMode`]: its mode value is in none of
    ///   its model's lists.
    /// - [`ValidationError::InvertedLimits`]: its minimum limit is above its
    ///   maximum.
    /// - [`Error::Status`] (a hardware fault fails Setup), and any transport
    ///   or protocol error of a read.
    ///
    /// [`SetupError::TooManyActuators`]: crate::SetupError::TooManyActuators
    /// [`ValidationError::DuplicateId`]: crate::ValidationError::DuplicateId
    /// [`SetupError::UnsupportedModel`]: crate::SetupError::UnsupportedModel
    /// [`SetupError::Missing`]: crate::SetupError::Missing
    /// [`SetupError::WrongModel`]: crate::SetupError::WrongModel
    /// [`SetupError::UnknownOperatingMode`]: crate::SetupError::UnknownOperatingMode
    /// [`ValidationError::InvertedLimits`]: crate::ValidationError::InvertedLimits
    pub fn declare<I>(&mut self, list: I) -> Result<&[ActuatorKind], Error>
    where
        I: IntoIterator<Item = (Id, ModelNumber)>,
    {
        let (mut wire, mut slots) = self.split();
        setup::declare::<P, T, C, _, _, N>(&mut wire, &mut slots, list)?;
        Ok(self.actuators())
    }

    /// Setup by scanning (P2 broadcast ping, P1 sweep 0..=252). Available
    /// without `std`.
    ///
    /// Replaces the held list; empty on `Err`. Protocol 2 sends one
    /// broadcast Ping and collects every reply within
    /// [`Config::scan_timeout`]. Protocol 1 pings IDs 0..=252 in turn, each
    /// waiting `scan_timeout`, which takes about 2.5 s at the default when
    /// few IDs answer. Each actuator found is then set up like
    /// [`declare`](Bus::declare) does, in `Id` order, and held with torque
    /// `Off`.
    ///
    /// # Errors
    ///
    /// - [`Error::Protocol`]: a corrupted reply to the scan.
    /// - [`Error::Status`]: a reply reporting an error (a hardware fault
    ///   fails Setup).
    /// - [`ValidationError::DuplicateId`]: two replies with one `Id`.
    /// - [`SetupError::TooManyActuators`]: more than `N` replies.
    /// - [`SetupError::UnsupportedModel`]: a model this build has no table
    ///   for on `P` (its `feature` names the Cargo feature that adds it).
    /// - The other Setup errors of [`declare`](Bus::declare), after the scan.
    ///
    /// [`ValidationError::DuplicateId`]: crate::ValidationError::DuplicateId
    /// [`SetupError::TooManyActuators`]: crate::SetupError::TooManyActuators
    /// [`SetupError::UnsupportedModel`]: crate::SetupError::UnsupportedModel
    pub fn autoscan(&mut self) -> Result<&[ActuatorKind], Error> {
        let (mut wire, mut slots) = self.split();
        setup::autoscan::<P, T, C, _, N>(&mut wire, &mut slots)?;
        Ok(self.actuators())
    }

    /// Held actuators, ascending by `Id`.
    pub fn actuators(&self) -> &[ActuatorKind] {
        &self.actuators
    }

    /// A handle on the actuator with `id`; `None` for an `Id` the bus does
    /// not hold. No I/O.
    ///
    /// The handle borrows the bus, so two cannot be live at once:
    ///
    /// ```compile_fail,E0499
    /// use dxl_rs_v2::{Bus, Clock, Id, P2, Transport};
    ///
    /// fn two<T: Transport, C: Clock>(bus: &mut Bus<P2, T, C>, a: Id, b: Id) {
    ///     let first = bus.actuator(a);
    ///     let second = bus.actuator(b);
    ///     drop((first, second));
    /// }
    /// ```
    pub fn actuator(&mut self, id: Id) -> Option<ActuatorRef<'_, P, T, C>> {
        let (wire, slots) = self.split();
        let index = find(slots.kinds(), id)?;
        let (kinds, states) = slots.into_split();
        let kind = kinds.get_mut(index)?;
        let state = states.get_mut(index)?;
        Some(ActuatorRef::new(kind, state, wire))
    }

    /// The torque state of the actuator with `id`; `None` for an `Id` the
    /// bus does not hold.
    pub fn torque_state(&self, id: Id) -> Option<TorqueState> {
        let index = find(&self.actuators, id)?;
        self.states.get(index).copied()
    }

    /// Torque states of the held actuators. Same order and length as
    /// [`actuators`](Bus::actuators).
    pub fn torque_states(&self) -> &[TorqueState] {
        self.states.get(..self.actuators.len()).unwrap_or_default()
    }

    /// Splits the bus into its wire and its slots, which borrow disjoint
    /// fields.
    fn split(&mut self) -> (Wire<'_, P, T, C>, FixedSlots<'_, N>) {
        (
            Wire::new(&mut self.transport, &mut self.clock, &self.config),
            FixedSlots::new(&mut self.actuators, &mut self.states),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::testing::*;
    use crate::protocol::P2;
    use crate::transport::{MockTime, MockTransport};

    #[test]
    fn config_defaults() {
        let config = Config::new();
        assert_eq!(config.timeout, Duration::from_millis(10));
        assert_eq!(config.scan_timeout, Duration::from_millis(10));
        assert_eq!(config.retries, 0);
        assert_eq!(config, Config::default());
    }

    #[test]
    fn config_builders_set_each_field() {
        const CONFIG: Config = Config::new()
            .timeout(Duration::from_millis(3))
            .scan_timeout(Duration::from_millis(7))
            .retries(2);
        assert_eq!(CONFIG.timeout, Duration::from_millis(3));
        assert_eq!(CONFIG.scan_timeout, Duration::from_millis(7));
        assert_eq!(CONFIG.retries, 2);
    }

    #[test]
    fn new_then_into_parts_sends_nothing() {
        let time = MockTime::new();
        let line = MockTransport::new(&time);
        let config = Config::new().retries(1);
        let bus = Bus::<P2, _, _>::new(line, time.ticker(10), config);
        assert_eq!(bus.config(), config);
        assert!(bus.actuators().is_empty());
        assert!(bus.torque_states().is_empty());
        let (line, mut clock) = bus.into_parts();
        assert!(line.sent().is_empty());
        assert_eq!(line.flushes(), 0);
        assert_eq!(clock(), 10);
    }

    #[test]
    fn unknown_id_gives_none() {
        let time = MockTime::new();
        let line = MockTransport::new(&time);
        let mut bus = Bus::<P2, _, _>::new(line, time.ticker(10), Config::new());
        assert!(bus.actuator(id(9)).is_none());
        assert_eq!(bus.torque_state(id(9)), None);
        let (line, _) = bus.into_parts();
        assert!(line.sent().is_empty());
        assert_eq!(line.flushes(), 0);
    }

    #[test]
    fn clock_may_be_a_plain_fn() {
        fn zero() -> u64 {
            0
        }
        let time = MockTime::new();
        let line = MockTransport::new(&time);
        let bus: Bus<P2, _, fn() -> u64, 4> = Bus::new(line, zero, Config::new());
        assert_eq!(bus.config(), Config::new());
    }

    #[cfg(feature = "x-series")]
    #[test]
    fn known_id_gives_a_ref() {
        use crate::actuator::Actuator;

        let time = MockTime::new();
        let mut line = MockTransport::new(&time);
        xm430_p2(&mut line, 1, 4, 0, 4095, 3);
        let mut bus = Bus::<P2, _, _>::new(line, time.ticker(10), Config::new());
        bus.declare([(id(4), ModelNumber(XM430))]).unwrap();
        let handle = bus.actuator(id(4)).unwrap();
        assert_eq!(handle.kind().id(), id(4));
        assert_eq!(handle.id(), id(4));
        assert_eq!(handle.torque_state(), TorqueState::Off);
        assert_eq!(bus.torque_state(id(4)), Some(TorqueState::Off));
        assert_eq!(bus.torque_state(id(5)), None);
    }
}
