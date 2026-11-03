//! A bus with no capacity limit, for desktops (`std`, research R-17).
//!
//! `DynBus` keeps its actuators in Vecs that grow at Setup only, and runs
//! every call through the same engine as `Bus`.

use alloc::vec::Vec;
use core::marker::PhantomData;

use crate::actuator::{Actuator, ModelNumber, TorqueState};
use crate::error::{Error, SetupError};
use crate::models::ActuatorKind;
use crate::protocol::{Clock, P2, Protocol, StdClock};
use crate::units::Position;
use crate::{Id, Transport};

#[cfg(doc)]
use super::Bus;
use super::field::{GoalPosition, PresentPosition, Raw, ReadField, WriteField};
use super::group::Stage;
use super::slots::{Slots, find};
use super::wire::{AnyProtocol, ReadMany, SyncRead, Wire};
use super::{ActuatorRef, Config, IdSet, multi, setup};

/// Every `Id` there is: the most a Setup can find.
const ALL_IDS: usize = Id::MAX as usize + 1;

/// The slots of a `DynBus`: its actuator and torque-state Vecs, grown by
/// Setup.
struct DynSlots<'a> {
    actuators: &'a mut Vec<ActuatorKind>,
    states: &'a mut Vec<TorqueState>,
}

impl Slots for DynSlots<'_> {
    fn kinds(&self) -> &[ActuatorKind] {
        self.actuators
    }

    fn split_mut(&mut self) -> (&mut [ActuatorKind], &mut [TorqueState]) {
        (self.actuators, self.states)
    }

    fn clear(&mut self) {
        self.actuators.clear();
        self.states.clear();
    }

    fn push(&mut self, kind: ActuatorKind) -> Result<(), SetupError> {
        self.actuators.push(kind);
        self.states.push(TorqueState::Off);
        Ok(())
    }
}

/// A bus with no capacity limit. Storage grows at Setup only.
///
/// The same methods as [`Bus`], with the same signatures and behaviour,
/// without its capacity `N`: Setup holds as many actuators as answer (at
/// most 253, the number of IDs), and [`actuator`](DynBus::actuator) lends
/// the same [`ActuatorRef`]. Setup sizes the storage, so no later call
/// allocates. Only with the `std` feature.
#[cfg_attr(
    feature = "std",
    doc = r#"
A desktop chain of any length, read into a buffer sized once:

```no_run
use dxl_rs_v2::{Config, DynBus, P2, Position, SerialTransport, StdClock};

let port = SerialTransport::open("/dev/ttyUSB0", 1_000_000)?;
let mut bus = DynBus::<P2, _>::new(port, StdClock::default(), Config::new());
bus.autoscan()?;
let mut present = vec![Position::ZERO; bus.actuators().len()];
bus.enable_torque(bus.ids())?;
for _ in 0..1000 {
    bus.read_positions(bus.ids(), &mut present)?;
    bus.write_goal_positions(bus.ids(), &present)?;
}
# Ok::<(), Box<dyn std::error::Error>>(())
```
"#
)]
pub struct DynBus<P: Protocol, T: Transport, C = StdClock> {
    transport: T,
    clock: C,
    config: Config,
    /// Sorted by `Id`, no duplicates.
    actuators: Vec<ActuatorKind>,
    /// `states[i]` belongs to `actuators[i]`; same length.
    states: Vec<TorqueState>,
    /// Staging for multi-actuator calls; as long as `actuators` after Setup.
    stages: Vec<Stage>,
    protocol: PhantomData<P>,
}

impl<P: Protocol, T: Transport, C: Clock> DynBus<P, T, C> {
    /// A bus over `transport`, timed by `clock`. No I/O, no allocation.
    pub fn new(transport: T, clock: C, config: Config) -> Self {
        Self {
            transport,
            clock,
            config,
            actuators: Vec::new(),
            states: Vec::new(),
            stages: Vec::new(),
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

    /// Setup from a list, as [`Bus::declare`] does, with no capacity limit.
    /// Pings only the listed IDs. Replaces the held list; empty on `Err`.
    ///
    /// # Errors
    ///
    /// As [`Bus::declare`], without `TooManyActuators`.
    pub fn declare<I>(&mut self, list: I) -> Result<&[ActuatorKind], Error>
    where
        I: IntoIterator<Item = (Id, ModelNumber)>,
    {
        self.stages.clear();
        let (mut wire, mut slots) = self.split();
        setup::declare::<P, T, C, _, _, ALL_IDS>(&mut wire, &mut slots, list)?;
        self.stages.resize(self.actuators.len(), Stage::EMPTY);
        Ok(self.actuators())
    }

    /// Setup by scanning (P2 broadcast ping, P1 sweep 0..=252), as
    /// [`Bus::autoscan`] does, holding every actuator that answers.
    /// Replaces the held list; empty on `Err`.
    ///
    /// # Errors
    ///
    /// As [`Bus::autoscan`], without `TooManyActuators`.
    pub fn autoscan(&mut self) -> Result<&[ActuatorKind], Error> {
        self.stages.clear();
        let (mut wire, mut slots) = self.split();
        setup::autoscan::<P, T, C, _, ALL_IDS>(&mut wire, &mut slots)?;
        self.stages.resize(self.actuators.len(), Stage::EMPTY);
        Ok(self.actuators())
    }

    /// Held actuators, ascending by `Id`.
    pub fn actuators(&self) -> &[ActuatorKind] {
        &self.actuators
    }

    /// Held IDs, as an owned set (so `bus.enable_torque(bus.ids())`
    /// compiles). Ascending, like [`actuators`](DynBus::actuators).
    pub fn ids(&self) -> IdSet {
        self.actuators.iter().map(Actuator::id).collect()
    }

    /// A handle on the actuator with `id`; `None` for an `Id` the bus does
    /// not hold. No I/O. The same handle as [`Bus::actuator`] lends.
    pub fn actuator(&mut self, id: Id) -> Option<ActuatorRef<'_, P, T, C>> {
        let index = find(&self.actuators, id)?;
        let kind = self.actuators.get_mut(index)?;
        let state = self.states.get_mut(index)?;
        let wire = Wire::new(&mut self.transport, &mut self.clock, &self.config);
        Some(ActuatorRef::new(kind, state, wire))
    }

    /// The torque state of the actuator with `id`; `None` for an `Id` the
    /// bus does not hold.
    pub fn torque_state(&self, id: Id) -> Option<TorqueState> {
        let index = find(&self.actuators, id)?;
        self.states.get(index).copied()
    }

    /// Torque states of the held actuators. Same order and length as
    /// [`actuators`](DynBus::actuators).
    pub fn torque_states(&self) -> &[TorqueState] {
        &self.states
    }

    /// Turns torque on for every target, as [`Bus::enable_torque`] does.
    ///
    /// # Errors
    ///
    /// As [`Bus::enable_torque`].
    pub fn enable_torque<I: IntoIterator<Item = Id>>(&mut self, ids: I) -> Result<(), Error> {
        let (mut wire, mut slots, stages) = self.split_staged();
        multi::torque_on(&mut wire, &mut slots, stages, ids)
    }

    /// Turns torque off for every target, as [`Bus::disable_torque`] does.
    ///
    /// # Errors
    ///
    /// As [`Bus::disable_torque`].
    pub fn disable_torque<I: IntoIterator<Item = Id>>(&mut self, ids: I) -> Result<(), Error> {
        let (mut wire, mut slots, stages) = self.split_staged();
        multi::torque_off(&mut wire, &mut slots, stages, ids)
    }

    /// Sends goal positions in radians, as [`Bus::write_goal_positions`]
    /// does: validated before any byte, one Sync Write per register layout.
    ///
    /// # Errors
    ///
    /// As [`Bus::write_goal_positions`].
    pub fn write_goal_positions<I: IntoIterator<Item = Id>>(
        &mut self,
        ids: I,
        goals: &[Position],
    ) -> Result<(), Error> {
        self.sync_write(GoalPosition, ids, goals)
    }

    /// Sends goal positions in ticks, as [`Bus::write_goal_positions_raw`]
    /// does.
    ///
    /// # Errors
    ///
    /// As [`Bus::write_goal_positions_raw`].
    pub fn write_goal_positions_raw<I: IntoIterator<Item = Id>>(
        &mut self,
        ids: I,
        goals: &[i32],
    ) -> Result<(), Error> {
        self.sync_write(Raw(GoalPosition), ids, goals)
    }

    /// Writes field `F` to every target, as [`Bus::sync_write`] does. Both
    /// protocols.
    ///
    /// # Errors
    ///
    /// As [`Bus::sync_write`].
    pub fn sync_write<F: WriteField, I: IntoIterator<Item = Id>>(
        &mut self,
        field: F,
        ids: I,
        values: &[F::Value],
    ) -> Result<(), Error> {
        let _ = field;
        let (mut wire, mut slots, stages) = self.split_staged();
        multi::write_field::<P, T, C, _, F, I>(&mut wire, &mut slots, stages, ids, values)
    }

    /// Reads Present Position of every target, in radians, as
    /// [`Bus::read_positions`] does. Strict: `out` changes only if every
    /// target answered.
    ///
    /// # Errors
    ///
    /// As [`Bus::read_positions`].
    pub fn read_positions<I: IntoIterator<Item = Id>>(
        &mut self,
        ids: I,
        out: &mut [Position],
    ) -> Result<(), Error> {
        self.read::<PresentPosition, AnyProtocol, I>(ids, out)
    }

    /// Reads Present Position of every target, in ticks, as
    /// [`Bus::read_positions_raw`] does.
    ///
    /// # Errors
    ///
    /// As [`Bus::read_positions_raw`].
    pub fn read_positions_raw<I: IntoIterator<Item = Id>>(
        &mut self,
        ids: I,
        out: &mut [i32],
    ) -> Result<(), Error> {
        self.read::<Raw<PresentPosition>, AnyProtocol, I>(ids, out)
    }

    /// Reads field `F` of `ids` into `out` through `R`.
    fn read<F: ReadField, R: ReadMany<P, T, C>, I: IntoIterator<Item = Id>>(
        &mut self,
        ids: I,
        out: &mut [F::Value],
    ) -> Result<(), Error> {
        let (mut wire, mut slots, stages) = self.split_staged();
        multi::read_field::<P, T, C, _, F, R, I>(&mut wire, &mut slots, stages, ids, out)
    }

    /// Splits the bus into its wire and its slots, which borrow disjoint
    /// fields.
    fn split(&mut self) -> (Wire<'_, P, T, C>, DynSlots<'_>) {
        (
            Wire::new(&mut self.transport, &mut self.clock, &self.config),
            DynSlots {
                actuators: &mut self.actuators,
                states: &mut self.states,
            },
        )
    }

    /// [`split`](DynBus::split), plus the staging sized at Setup.
    fn split_staged(&mut self) -> (Wire<'_, P, T, C>, DynSlots<'_>, &mut [Stage]) {
        (
            Wire::new(&mut self.transport, &mut self.clock, &self.config),
            DynSlots {
                actuators: &mut self.actuators,
                states: &mut self.states,
            },
            &mut self.stages,
        )
    }
}

/// The Protocol 2 explicit tier.
impl<T: Transport, C: Clock> DynBus<P2, T, C> {
    /// Reads field `F` of every target with Sync Read, as
    /// [`Bus::sync_read`] does. Protocol 2 only.
    ///
    /// # Errors
    ///
    /// As [`Bus::sync_read`].
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

/// Reads with one result per target (`partial-reads` feature).
#[cfg(feature = "partial-reads")]
impl<P: Protocol, T: Transport, C: Clock> DynBus<P, T, C> {
    /// Reads Present Position of every target, in radians, with one result
    /// per target, as [`Bus::read_positions_partial`] does: `out[i]` is
    /// target `i`'s position or its own error.
    ///
    /// # Errors
    ///
    /// As [`Bus::read_positions_partial`]: only refusals, with nothing
    /// sent.
    pub fn read_positions_partial<I: IntoIterator<Item = Id>>(
        &mut self,
        ids: I,
        out: &mut [Result<Position, Error>],
    ) -> Result<(), Error> {
        self.read_partial::<PresentPosition, I>(ids, out)
    }

    /// Reads Present Position of every target, in ticks, with one result
    /// per target, as [`Bus::read_positions_raw_partial`] does.
    ///
    /// # Errors
    ///
    /// As [`Bus::read_positions_raw_partial`]: only refusals, with nothing
    /// sent.
    pub fn read_positions_raw_partial<I: IntoIterator<Item = Id>>(
        &mut self,
        ids: I,
        out: &mut [Result<i32, Error>],
    ) -> Result<(), Error> {
        self.read_partial::<Raw<PresentPosition>, I>(ids, out)
    }

    /// Reads field `F` of `ids` into `out`, one result per target.
    fn read_partial<F: ReadField, I: IntoIterator<Item = Id>>(
        &mut self,
        ids: I,
        out: &mut [Result<F::Value, Error>],
    ) -> Result<(), Error> {
        let (mut wire, mut slots, stages) = self.split_staged();
        super::partial::read_partial::<P, T, C, _, F, I>(&mut wire, &mut slots, stages, ids, out)
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    #[cfg(any(feature = "ax", feature = "x-series"))]
    use std::{format, string::String, vec::Vec};

    use super::*;
    use crate::bus::testing::*;
    use crate::transport::{MockTime, MockTransport};

    /// A `DynBus` on a scripted line.
    type TestDynBus<'t, P, L = MockTransport<'t>> = DynBus<P, L, Ticker<'t>>;

    fn test_dyn_bus<P: Protocol>(time: &MockTime) -> TestDynBus<'_, P> {
        DynBus::new(MockTransport::new(time), Ticker(time), Config::new())
    }

    /// Runs `$body` on a `Bus` and on a `DynBus`, each on a line scripted
    /// by `$script`, and checks that both return the same and send the same
    /// bytes.
    #[cfg(any(feature = "ax", feature = "x-series"))]
    macro_rules! on_both {
        ($p:ty, $script:expr, |$bus:ident| $body:block) => {{
            let script: fn(&mut MockTransport<'_>) = $script;
            let time = MockTime::new();
            let mut line = MockTransport::new(&time);
            script(&mut line);
            let mut $bus = TestBus::<$p, 32>::new(line, Ticker(&time), Config::new());
            let fixed: Vec<String> = $body;
            let (fixed_line, _) = $bus.into_parts();

            let time = MockTime::new();
            let mut line = MockTransport::new(&time);
            script(&mut line);
            let mut $bus = TestDynBus::<$p>::new(line, Ticker(&time), Config::new());
            let dynamic: Vec<String> = $body;
            let (dyn_line, _) = $bus.into_parts();

            assert_eq!(fixed, dynamic);
            assert_eq!(fixed_line.sent(), dyn_line.sent());
            assert_eq!(fixed_line.flushes(), dyn_line.flushes());
            fixed_line.assert_done();
            dynamic
        }};
    }

    #[cfg(any(feature = "ax", feature = "x-series"))]
    fn show<V: core::fmt::Debug>(value: V) -> String {
        format!("{value:?}")
    }

    /// The three Vecs of `bus` are empty.
    #[track_caller]
    fn assert_empty<P: Protocol, L: Transport>(bus: &TestDynBus<'_, P, L>) {
        assert!(bus.actuators.is_empty());
        assert!(bus.states.is_empty());
        assert!(bus.stages.is_empty());
    }

    #[test]
    fn new_then_into_parts_sends_nothing() {
        let time = MockTime::new();
        let config = Config::new().retries(2);
        let bus = DynBus::<P2, _, _>::new(MockTransport::new(&time), Ticker(&time), config);
        assert_eq!(bus.config(), config);
        assert!(bus.actuators().is_empty());
        assert!(bus.torque_states().is_empty());
        assert!(bus.ids().is_empty());
        assert_empty(&bus);
        let (line, _) = bus.into_parts();
        assert!(line.sent().is_empty());
        assert_eq!(line.flushes(), 0);
    }

    #[test]
    fn unknown_id_and_empty_targets_send_nothing() {
        let time = MockTime::new();
        let mut bus = test_dyn_bus::<P2>(&time);
        assert!(bus.actuator(id(9)).is_none());
        assert_eq!(bus.torque_state(id(9)), None);
        assert_eq!(bus.enable_torque(bus.ids()), Ok(()));
        assert_eq!(bus.disable_torque([]), Ok(()));
        assert_eq!(bus.write_goal_positions([], &[]), Ok(()));
        assert_eq!(bus.read_positions_raw([], &mut []), Ok(()));
        assert_eq!(
            bus.write_goal_positions_raw([id(1)], &[0]),
            Err(SetupError::NotOnBus { id: id(1) }.into())
        );
        let (line, _) = bus.into_parts();
        assert!(line.sent().is_empty());
        assert_eq!(line.flushes(), 0);
    }

    #[cfg(feature = "x-series")]
    mod x_series {
        use super::*;
        use crate::error::ValidationError;
        use crate::field::{GoalPosition, PresentPosition, Raw};

        /// How many actuators the uncapped test holds: more than `Bus`'s
        /// default 32.
        const MANY: u8 = 40;

        /// Scripts the Setup reads of an XM430-W350 at `n` (limits
        /// 0..=4095, position mode) on a bigger line, as `xm430_p2_reads`.
        fn setup_reads<const RX: usize, const TX: usize, const CHUNKS: usize>(
            line: &mut MockTransport<'_, RX, TX, CHUNKS>,
            at: usize,
            n: u8,
        ) -> usize {
            line.reply_to(at, &p2_status(n, 0, &0i32.to_le_bytes()))
                .reply_to(at + 1, &p2_status(n, 0, &4095i32.to_le_bytes()))
                .reply_to(at + 2, &p2_status(n, 0, &[3]));
            at + 3
        }

        #[test]
        fn dyn_bus_holds_40() {
            let time = MockTime::new();
            let mut line = MockTransport::<'_, 4096, 4096, 128>::with_capacity(&time);
            let pongs: Vec<Vec<u8>> = (1..=MANY)
                .rev()
                .map(|n| p2_pong(n, XM430, XM430_FW))
                .collect();
            line.reply_to(1, &concat(&pongs));
            let mut at = 2;
            for n in 1..=MANY {
                at = setup_reads(&mut line, at, n);
            }
            let mut bus = DynBus::<P2, _, _>::new(line, Ticker(&time), Config::new());

            let held = bus.autoscan().unwrap();
            assert_eq!(held.len(), usize::from(MANY));
            let ids: Vec<u8> = held.iter().map(|kind| kind.id().get()).collect();
            assert_eq!(ids, (1..=MANY).collect::<Vec<_>>());
            assert_eq!(bus.torque_states(), [OFF; MANY as usize]);
            assert_eq!(bus.stages.len(), usize::from(MANY));
            assert_eq!(bus.ids().len(), usize::from(MANY));

            let mut expected = std::vec![p2_instruction(BROADCAST, PING, &[])];
            expected.extend((1..=MANY).map(xm430_p2_reads_sent));
            assert_eq!(bus.transport.sent(), concat(&expected));
            bus.transport.assert_done();
        }

        #[test]
        fn many_actuators_in_one_call() {
            let time = MockTime::new();
            let mut line = MockTransport::<'_, 4096, 8192, 256>::with_capacity(&time);
            let mut at = 1;
            for n in 1..=MANY {
                line.reply_to(at, &p2_pong(n, XM430, XM430_FW));
                at = setup_reads(&mut line, at + 1, n);
            }
            for n in 1..=MANY {
                line.reply_to(at, &p2_status(n, 0, &[]));
                at += 1;
            }
            let replies: Vec<Vec<u8>> = (1..=MANY)
                .map(|n| p2_status(n, 0, &i32::from(n).to_le_bytes()))
                .collect();
            line.reply_to(at + 1, &concat(&replies));
            let mut bus = DynBus::<P2, _, _>::new(line, Ticker(&time), Config::new());
            let list: Vec<(Id, ModelNumber)> =
                (1..=MANY).map(|n| (id(n), ModelNumber(XM430))).collect();
            bus.declare(list).unwrap();

            bus.enable_torque(bus.ids()).unwrap();
            assert_eq!(bus.torque_states(), [ON; MANY as usize]);
            let goals: Vec<i32> = (1..=MANY).map(|n| 100 * i32::from(n)).collect();
            bus.write_goal_positions_raw(bus.ids(), &goals).unwrap();
            let mut present = std::vec![0; usize::from(MANY)];
            bus.sync_read(Raw(PresentPosition), bus.ids(), &mut present)
                .unwrap();
            assert_eq!(present, (1..=MANY).map(i32::from).collect::<Vec<_>>());
            bus.transport.assert_done();
        }

        /// XM430-W350 at IDs 1 and 2 (limits 0..=4095, position mode) and
        /// an XL-320 at ID 3 (0..=1023, joint mode): Setup, then the replies
        /// of [`cycle`].
        fn three(line: &mut MockTransport<'_>) {
            let at = xm430_p2(line, 1, 1, 0, 4095, 3);
            let at = xm430_p2(line, at, 2, 0, 4095, 3);
            let at = xl320_p2(line, at, 3, 0, 1023, 2);
            assert_eq!(at, 13);
            let ack = |n| p2_status(n, 0, &[]);
            let present = |n, tick: i32| p2_status(n, 0, &tick.to_le_bytes());
            line.reply_to(13, &ack(1))
                .reply_to(14, &ack(1))
                .reply_to(15, &present(1, 2000))
                .reply_to(16, &ack(1))
                .reply_to(17, &ack(2))
                .reply_to(18, &ack(3))
                .reply_to(24, &concat(&[present(1, 1000), present(2, 3000)]))
                .reply_to(25, &p2_status(3, 0, &512u16.to_le_bytes()))
                .reply_to(26, &concat(&[present(2, 2048), present(1, 1024)]))
                .reply_to(27, &p2_status(3, 0, &600u16.to_le_bytes()))
                .reply_to(28, &concat(&[present(1, 1), p2_status(2, 0x80, &[0; 4])]))
                .reply_to(29, &ack(1))
                .reply_to(30, &ack(2))
                .reply_to(31, &ack(3));
        }

        #[test]
        fn same_calls_same_bytes_as_bus() {
            let steps = on_both!(P2, three, |bus| {
                let (a, b, c) = (id(1), id(2), id(3));
                let mut steps = Vec::new();
                steps.push(show(bus.declare([
                    (c, ModelNumber(XL320)),
                    (a, ModelNumber(XM430)),
                    (b, ModelNumber(XM430)),
                ])));
                steps.push(show(bus.ids()));
                steps.push(show(bus.write_goal_positions_raw([a], &[100])));
                {
                    let mut xm: ActuatorRef<'_, P2, _, _> = bus.actuator(a).unwrap();
                    steps.push(show(xm.enable_torque()));
                    steps.push(show(xm.set_goal_position_raw(2000)));
                    steps.push(show(xm.present_position_raw()));
                }
                steps.push(show(bus.torque_state(a)));
                steps.push(show(bus.torque_state(id(9))));
                steps.push(show(bus.enable_torque(bus.ids())));
                steps.push(show(bus.torque_states()));
                steps.push(show(
                    bus.write_goal_positions_raw([a, b, c], &[1000, 3000, 512]),
                ));
                let goals = [Position::ZERO, Position::from_radians(0.5)];
                steps.push(show(bus.write_goal_positions([c, a], &goals)));
                steps.push(show(bus.sync_write(GoalPosition, [b], &[Position::ZERO])));
                steps.push(show(bus.write_goal_positions_raw([a, a], &[1, 1])));
                steps.push(show(bus.write_goal_positions_raw([a, c], &[1])));
                steps.push(show(bus.write_goal_positions_raw([c], &[5000])));
                let mut raw = [7; 4];
                steps.push(show(bus.read_positions_raw([a, b, c], &mut raw)));
                steps.push(show(raw));
                let mut typed = [Position::ZERO; 2];
                steps.push(show(bus.read_positions([b, a], &mut typed)));
                steps.push(show(typed));
                let mut one = [0];
                steps.push(show(bus.sync_read(Raw(PresentPosition), [c], &mut one)));
                steps.push(show(one));
                let mut two = [Position::ZERO; 1];
                steps.push(show(bus.read_positions([a, b], &mut two)));
                steps.push(show(bus.sync_read(PresentPosition, [a, b], &mut typed)));
                steps.push(show(typed));
                steps.push(show(bus.torque_states()));
                steps.push(show(bus.enable_torque([b])));
                steps.push(show(bus.disable_torque(bus.ids())));
                steps.push(show(bus.torque_states()));
                steps
            });
            assert!(steps[0].starts_with("Ok"));
            assert_eq!(
                steps[2],
                show(Err::<(), Error>(
                    ValidationError::ActuatorIsOff { id: id(1) }.into()
                ))
            );
            assert_eq!(steps[5], "Ok(2000)");
            assert_eq!(
                steps[13],
                show(Err::<(), Error>(
                    ValidationError::DuplicateId { id: id(1) }.into()
                ))
            );
            assert_eq!(steps[17], "[1000, 3000, 512, 7]");
            assert_eq!(steps[21], "[600]");
            assert!(steps[23].starts_with("Err(Status"));
            assert_eq!(steps[25], show([ON, FAULTED, ON]));
            assert_eq!(steps[28], show([OFF, FAULTED, OFF]));
        }

        fn two_scanned(line: &mut MockTransport<'_>) {
            line.reply_to(
                1,
                &concat(&[p2_pong(2, XM430, XM430_FW), p2_pong(1, XM430, XM430_FW)]),
            );
            let at = xm430_p2_reads(line, 2, 1, 0, 4095, 3);
            xm430_p2_reads(line, at, 2, 0, 4095, 4);
        }

        #[test]
        fn autoscan_same_as_bus() {
            let steps = on_both!(P2, two_scanned, |bus| {
                std::vec![show(bus.autoscan()), show(bus.torque_states())]
            });
            assert!(steps[0].starts_with("Ok"));
        }

        /// Two XM430-W350 at IDs 1 and 2, then the replies of the partial
        /// reads: ID 2 silent, then ID 1 reporting a hardware fault.
        #[cfg(feature = "partial-reads")]
        fn two_partial(line: &mut MockTransport<'_>) {
            let at = xm430_p2(line, 1, 1, 0, 4095, 3);
            let at = xm430_p2(line, at, 2, 0, 4095, 3);
            line.reply_to(at, &p2_status(1, 0, &2048i32.to_le_bytes()))
                .reply_to(
                    at + 1,
                    &concat(&[
                        p2_status(1, 0x80, &[0; 4]),
                        p2_status(2, 0, &100i32.to_le_bytes()),
                    ]),
                );
        }

        #[cfg(feature = "partial-reads")]
        #[test]
        fn partial_reads_same_as_bus() {
            let steps = on_both!(P2, two_partial, |bus| {
                let (a, b) = (id(1), id(2));
                let list = [(a, ModelNumber(XM430)), (b, ModelNumber(XM430))];
                let mut typed = [Ok(Position::ZERO); 2];
                let mut raw = [Ok(-7); 3];
                std::vec![
                    show(bus.declare(list)),
                    show(bus.read_positions_partial([a, b], &mut typed)),
                    show(typed),
                    show(bus.read_positions_raw_partial(bus.ids(), &mut raw)),
                    show(raw),
                    show(bus.read_positions_raw_partial([a, a], &mut raw)),
                    show(bus.read_positions_raw_partial([a, b], &mut [])),
                    show(bus.torque_states()),
                ]
            });
            assert!(steps[2].contains("Ok(") && steps[2].contains("Timeout"));
            assert!(steps[4].starts_with("[Err(Status") && steps[4].ends_with("Ok(100), Ok(-7)]"));
            assert!(steps[5].contains("DuplicateId"));
            assert!(steps[6].contains("BufferLength"));
            assert_eq!(steps[7], show([FAULTED, OFF]));
        }

        #[test]
        fn failed_declare_leaves_every_vec_empty() {
            let time = MockTime::new();
            let mut bus = test_dyn_bus::<P2>(&time);
            xm430_p2(&mut bus.transport, 1, 1, 0, 4095, 3);
            bus.declare([(id(1), ModelNumber(XM430))]).unwrap();
            assert_eq!(bus.stages.len(), 1);
            assert_eq!(
                bus.declare([(id(1), ModelNumber(XM430))]),
                Err(SetupError::Missing { id: id(1) }.into())
            );
            assert_empty(&bus);
            assert_eq!(
                bus.declare([(id(1), ModelNumber(XM430)), (id(1), ModelNumber(XM430))]),
                Err(ValidationError::DuplicateId { id: id(1) }.into())
            );
            assert_empty(&bus);
        }

        #[test]
        fn failed_autoscan_leaves_every_vec_empty() {
            let time = MockTime::new();
            let mut bus = test_dyn_bus::<P2>(&time);
            bus.transport.reply_to(1, &p2_pong(1, XM430, XM430_FW));
            xm430_p2_reads(&mut bus.transport, 2, 1, 0, 4095, 3);
            bus.autoscan().unwrap();
            assert_eq!(bus.stages.len(), 1);
            let mut corrupted = p2_pong(1, XM430, XM430_FW);
            if let Some(crc) = corrupted.last_mut() {
                *crc ^= 0xFF;
            }
            bus.transport.reply(&corrupted);
            assert!(matches!(bus.autoscan(), Err(Error::Protocol(_))));
            assert_empty(&bus);
        }
    }

    #[cfg(feature = "ax")]
    mod ax {
        use super::*;

        /// Two AX-12A at IDs 1 and 2 (limits 0..=1023): Setup, then the
        /// replies of the cycle.
        fn two(line: &mut MockTransport<'_>) {
            let at = ax12_p1(line, 1, 1, 0, 1023);
            let at = ax12_p1(line, at, 2, 0, 1023);
            assert_eq!(at, 9);
            line.reply_to(9, &p1_status(1, 0, &[]))
                .reply_to(10, &p1_status(2, 0, &[]))
                .reply_to(12, &p1_status(1, 0, &600u16.to_le_bytes()))
                .reply_to(13, &p1_status(2, 0, &700u16.to_le_bytes()))
                .reply_to(14, &p1_status(1, 0, &[]))
                .reply_to(15, &p1_status(2, 0, &[]));
        }

        #[test]
        fn same_calls_same_bytes_as_bus_on_p1() {
            let steps = on_both!(crate::protocol::P1, two, |bus| {
                let (a, b) = (id(1), id(2));
                let mut raw = [0; 2];
                std::vec![
                    show(bus.declare([(a, ModelNumber(AX12)), (b, ModelNumber(AX12))])),
                    show(bus.enable_torque([a, b])),
                    show(bus.write_goal_positions_raw([b, a], &[100, 900])),
                    show(bus.read_positions_raw(bus.ids(), &mut raw)),
                    show(raw),
                    show(bus.disable_torque(bus.ids())),
                    show(bus.torque_states()),
                ]
            });
            assert_eq!(steps[4], "[600, 700]");
            assert_eq!(steps[6], show([OFF, OFF]));
        }
    }
}
