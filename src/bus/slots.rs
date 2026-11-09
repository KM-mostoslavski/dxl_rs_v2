//! Where a bus keeps its actuators, their torque states, their goal bounds
//! and what it has confirmed about them since Setup (data-model.md
//! "Slots"). Setup and every lookup go through [`Slots`], so the engine does
//! not depend on the storage.

use crate::Id;
use crate::actuator::{Actuator, TorqueState};
use crate::error::SetupError;
use crate::models::ActuatorKind;

/// The EEPROM bound of the goal an actuator's operating mode uses, read at
/// Setup: Velocity Limit in velocity mode, Current Limit in current and
/// current-based position mode, in raw register units. `None` in the other
/// modes and where the model's table records no bound. Kept beside the
/// actuator's `Cache`, which stays 16 bytes.
pub(crate) type Bound = Option<u32>;

/// What the bus has confirmed about one actuator since Setup, beside its
/// torque state, from replies it got and writes it sent (plan 2026-11-09
/// D8, D9). Both marks are clear after Setup: the bus reads neither Torque
/// Enable nor Moving Speed there (FR-2.7, FR-9.6). The limit mode the
/// cache holds is the hardware's after Setup, until a raw write or a
/// reboot that may have changed the angle limits.
// DECISION: the D8/D9 marks live in a slot array of their own beside the torque states, not in TorqueState (pub, its slice lent by torque_states()) nor in Cache (no spare byte left), so both keep their size_of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Marks {
    /// Torque is confirmed off: a Torque Enable = 0 write ran, and nothing
    /// that may turn torque on was sent since (D8).
    torque_off: bool,
    /// Moving Speed is known to be 0: an acknowledged unicast write of 0,
    /// and no other write of it since (D9).
    speed_zero: bool,
    /// A raw write or a reboot reached the actuator since Setup or since
    /// the last acknowledged write of both position limits, so the limit
    /// mode the cache holds may not be the hardware's (D9).
    // DECISION: the flag is negative ("a raw write since"), so CLEAR stays the after-Setup state and Setup, clear() and DynBus need no change.
    raw_since: bool,
}

impl Marks {
    /// Nothing confirmed, as after Setup; the limit mode Setup read is the
    /// hardware's.
    pub(crate) const CLEAR: Self = Self {
        torque_off: false,
        speed_zero: false,
        raw_since: false,
    };

    /// Whether torque is confirmed off (D8).
    pub(crate) const fn torque_off(self) -> bool {
        self.torque_off
    }

    /// Whether Moving Speed is known to be 0 (D9).
    pub(crate) const fn speed_zero(self) -> bool {
        self.speed_zero
    }

    /// Whether the limit mode the cache holds is the hardware's: no raw
    /// write or reboot since Setup or since the last acknowledged write of
    /// both position limits (D9).
    pub(crate) const fn mode_trusted(self) -> bool {
        !self.raw_since
    }

    /// After a Torque Enable = 0 write: confirmed off when `ran`, the reply
    /// showing the instruction ran; else no longer confirmed.
    pub(crate) fn torque_off_written(&mut self, ran: bool) {
        self.torque_off = ran;
    }

    /// After anything that may turn torque on: an `enable_torque` that sent
    /// bytes, a reboot.
    pub(crate) fn forget_torque_off(&mut self) {
        self.torque_off = false;
    }

    /// After a write of Moving Speed: known to be 0 when `zero`, an
    /// acknowledged unicast write of 0; else no longer known.
    pub(crate) fn speed_written(&mut self, zero: bool) {
        self.speed_zero = zero;
    }

    /// After an acknowledged write of both position limits: the limit mode
    /// the cache follows them with is the hardware's again.
    pub(crate) fn limits_written(&mut self) {
        self.raw_since = false;
    }

    /// After a write that may touch any register (a raw write) or a
    /// reboot: nothing is confirmed any more, the limit mode the cache
    /// holds included.
    pub(crate) fn forget(&mut self) {
        *self = Self {
            raw_since: true,
            ..Self::CLEAR
        };
    }
}

/// The held actuators, borrowed for one multi-actuator call: the actuators,
/// their torque states, their goal bounds and their marks, same length and
/// order.
pub(crate) type Held<'s> = (
    &'s mut [ActuatorKind],
    &'s mut [TorqueState],
    &'s [Bound],
    &'s mut [Marks],
);

/// The held actuators with one torque state, one goal bound and one set of
/// marks per actuator, in the same order.
pub(crate) trait Slots {
    /// The held actuators, ascending by `Id`.
    fn kinds(&self) -> &[ActuatorKind];

    /// The held actuators, their states, their bounds and their marks, same
    /// length and order.
    fn split_mut(&mut self) -> Held<'_>;

    /// Empties the list. States become `Off`, bounds `None`, marks clear.
    fn clear(&mut self);

    /// Appends `kind` with torque state `state`, goal bound `bound` and
    /// clear marks. `TooManyActuators` past capacity.
    fn push(
        &mut self,
        kind: ActuatorKind,
        bound: Bound,
        state: TorqueState,
    ) -> Result<(), SetupError>;
}

/// The slots of a `Bus`: a list of at most `N` actuators, `N` states, `N`
/// bounds and `N` marks. Entries at `i >= len` are unused and kept `Off`,
/// `None` and clear.
pub(crate) struct FixedSlots<'a, const N: usize> {
    actuators: &'a mut heapless::Vec<ActuatorKind, N>,
    states: &'a mut [TorqueState; N],
    bounds: &'a mut [Bound; N],
    marks: &'a mut [Marks; N],
}

/// One handle's view of the slots: the actuators, their states, bounds and
/// marks, all mutable, same length and order.
pub(crate) type Split<'a> = (
    &'a mut [ActuatorKind],
    &'a mut [TorqueState],
    &'a mut [Bound],
    &'a mut [Marks],
);

impl<'a, const N: usize> FixedSlots<'a, N> {
    /// Borrows a bus's four storage fields.
    pub(crate) fn new(
        actuators: &'a mut heapless::Vec<ActuatorKind, N>,
        states: &'a mut [TorqueState; N],
        bounds: &'a mut [Bound; N],
        marks: &'a mut [Marks; N],
    ) -> Self {
        Self {
            actuators,
            states,
            bounds,
            marks,
        }
    }

    /// The held actuators, their states, bounds and marks, same length and
    /// order, for as long as the bus is borrowed. The bounds and marks are
    /// mutable: a handle's limit write keeps its actuator's bound true, and
    /// its torque and speed writes move its marks.
    pub(crate) fn into_split(self) -> Split<'a> {
        let len = self.actuators.len();
        let states = self.states.get_mut(..len).unwrap_or_default();
        let bounds = self.bounds.get_mut(..len).unwrap_or_default();
        let marks = self.marks.get_mut(..len).unwrap_or_default();
        (self.actuators, states, bounds, marks)
    }
}

impl<const N: usize> Slots for FixedSlots<'_, N> {
    fn kinds(&self) -> &[ActuatorKind] {
        self.actuators
    }

    fn split_mut(&mut self) -> Held<'_> {
        let len = self.actuators.len();
        let states = self.states.get_mut(..len).unwrap_or_default();
        let bounds = self.bounds.get(..len).unwrap_or_default();
        let marks = self.marks.get_mut(..len).unwrap_or_default();
        (self.actuators, states, bounds, marks)
    }

    fn clear(&mut self) {
        self.actuators.clear();
        self.states.fill(TorqueState::Off);
        self.bounds.fill(None);
        self.marks.fill(Marks::CLEAR);
    }

    fn push(
        &mut self,
        kind: ActuatorKind,
        bound: Bound,
        state: TorqueState,
    ) -> Result<(), SetupError> {
        let index = self.actuators.len();
        self.actuators
            .push(kind)
            .map_err(|kind| SetupError::TooManyActuators {
                capacity: N,
                ids: self
                    .actuators
                    .iter()
                    .map(|kind| kind.id())
                    .chain([kind.id()])
                    .collect(),
                declared: false,
            })?;
        if let Some(slot) = self.states.get_mut(index) {
            *slot = state;
        }
        if let Some(slot) = self.bounds.get_mut(index) {
            *slot = bound;
        }
        if let Some(slot) = self.marks.get_mut(index) {
            *slot = Marks::CLEAR;
        }
        Ok(())
    }
}

/// The slot of `id` in `kinds`, which is ascending by `Id`.
pub(crate) fn find(kinds: &[ActuatorKind], id: Id) -> Option<usize> {
    kinds.binary_search_by_key(&id, Actuator::id).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn torque_state_is_one_byte() {
        assert_eq!(size_of::<TorqueState>(), 1);
        assert_eq!(size_of::<[TorqueState; 32]>(), 32);
    }

    /// Every flag set, as no Setup leaves it.
    const ALL: Marks = Marks {
        torque_off: true,
        speed_zero: true,
        raw_since: true,
    };

    #[test]
    fn empty_slots() {
        let mut actuators = heapless::Vec::<ActuatorKind, 2>::new();
        let mut states = [TorqueState::Faulted; 2];
        let mut bounds = [Some(7); 2];
        let mut marks = [ALL; 2];
        let mut slots = FixedSlots::new(&mut actuators, &mut states, &mut bounds, &mut marks);
        slots.clear();
        assert!(slots.kinds().is_empty());
        assert_eq!(find(slots.kinds(), crate::bus::testing::id(1)), None);
        let (kinds, held, limits, confirmed) = slots.into_split();
        assert!(kinds.is_empty() && held.is_empty() && limits.is_empty() && confirmed.is_empty());
        assert_eq!(states, [TorqueState::Off; 2]);
        assert_eq!(bounds, [None; 2]);
        assert_eq!(marks, [Marks::CLEAR; 2]);
    }

    /// Plan D8, D9: each mark moves on its own; a raw write or a reboot
    /// forgets both and the limit mode, which a write of both position
    /// limits trusts again.
    #[test]
    fn marks_move_one_at_a_time() {
        let mut marks = Marks::default();
        assert_eq!(marks, Marks::CLEAR);
        assert!(marks.mode_trusted());
        marks.torque_off_written(true);
        marks.speed_written(true);
        assert!(marks.torque_off() && marks.speed_zero());
        marks.forget_torque_off();
        assert!(!marks.torque_off() && marks.speed_zero());
        marks.torque_off_written(true);
        marks.speed_written(false);
        assert!(marks.torque_off() && !marks.speed_zero());
        marks.torque_off_written(false);
        assert!(!marks.torque_off());
        let mut marks = ALL;
        marks.limits_written();
        assert!(marks.mode_trusted() && marks.torque_off() && marks.speed_zero());
        marks.forget();
        assert!(!marks.torque_off() && !marks.speed_zero() && !marks.mode_trusted());
        marks.torque_off_written(true);
        marks.speed_written(true);
        assert!(!marks.mode_trusted());
        marks.limits_written();
        assert_eq!(
            marks,
            Marks {
                raw_since: false,
                ..ALL
            }
        );
        let mut marks = Marks::CLEAR;
        marks.forget();
        marks.limits_written();
        assert_eq!(marks, Marks::CLEAR);
    }

    #[cfg(feature = "x-series")]
    mod with_models {
        use super::*;
        use crate::actuator::ModelNumber;
        use crate::bus::testing::id;
        use crate::protocol::ProtocolVersion;

        fn xm430(n: u8) -> ActuatorKind {
            ActuatorKind::new(ProtocolVersion::P2, ModelNumber(1020), 0, id(n)).unwrap()
        }

        #[test]
        fn push_keeps_kinds_and_states_aligned() {
            let mut actuators = heapless::Vec::<ActuatorKind, 3>::new();
            let mut states = [TorqueState::Off; 3];
            let mut bounds = [None; 3];
            let mut marks = [ALL; 3];
            let mut slots = FixedSlots::new(&mut actuators, &mut states, &mut bounds, &mut marks);
            for n in [1, 4, 7] {
                slots
                    .push(xm430(n), Some(u32::from(n)), TorqueState::Off)
                    .unwrap();
            }
            let (kinds, held, limits, confirmed) = slots.split_mut();
            assert_eq!((kinds.len(), held.len(), confirmed.len()), (3, 3, 3));
            assert_eq!(held, [TorqueState::Off; 3]);
            assert_eq!(limits, [Some(1), Some(4), Some(7)]);
            assert_eq!(confirmed, [Marks::CLEAR; 3]);
            assert_eq!(find(slots.kinds(), id(4)), Some(1));
            assert_eq!(find(slots.kinds(), id(5)), None);
            let (kinds, held, _, confirmed) = slots.into_split();
            assert_eq!(kinds.len(), 3);
            assert_eq!(held.len(), 3);
            held[1] = TorqueState::On;
            confirmed[2].torque_off_written(true);
            let ids: [u8; 3] = core::array::from_fn(|i| kinds[i].id().get());
            assert_eq!(ids, [1, 4, 7]);
            assert_eq!(
                states,
                [TorqueState::Off, TorqueState::On, TorqueState::Off]
            );
            assert!(marks[2].torque_off() && !marks[1].torque_off());
        }

        #[test]
        fn push_past_capacity_is_too_many() {
            let mut actuators = heapless::Vec::<ActuatorKind, 2>::new();
            let mut states = [TorqueState::Off; 2];
            let mut bounds = [None; 2];
            let mut marks = [Marks::CLEAR; 2];
            let mut slots = FixedSlots::new(&mut actuators, &mut states, &mut bounds, &mut marks);
            slots.push(xm430(1), None, TorqueState::Off).unwrap();
            slots.push(xm430(2), None, TorqueState::Off).unwrap();
            assert_eq!(
                slots.push(xm430(3), None, TorqueState::Off),
                Err(SetupError::TooManyActuators {
                    capacity: 2,
                    ids: [1, 2, 3].map(crate::bus::testing::id).into_iter().collect(),
                    declared: false,
                })
            );
            assert_eq!(slots.kinds().len(), 2);
        }

        #[test]
        fn clear_empties_and_resets_every_state() {
            let mut actuators = heapless::Vec::<ActuatorKind, 2>::new();
            let mut states = [TorqueState::Faulted; 2];
            let mut bounds = [None; 2];
            let mut marks = [Marks::CLEAR; 2];
            let mut slots = FixedSlots::new(&mut actuators, &mut states, &mut bounds, &mut marks);
            slots.push(xm430(1), Some(9), TorqueState::Off).unwrap();
            slots.push(xm430(2), Some(9), TorqueState::Off).unwrap();
            let (_, _, _, confirmed) = slots.split_mut();
            confirmed.fill(ALL);
            slots.clear();
            assert!(slots.kinds().is_empty());
            assert_eq!(states, [TorqueState::Off; 2]);
            assert_eq!(bounds, [None; 2]);
            assert_eq!(marks, [Marks::CLEAR; 2]);
        }
    }
}
