//! Where a bus keeps its actuators, their torque states and their goal
//! bounds (data-model.md "Slots"). Setup and every lookup go through [`Slots`], so the engine does
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

/// The held actuators, borrowed for one multi-actuator call: the actuators,
/// their torque states and their goal bounds, same length and order.
pub(crate) type Held<'s> = (&'s mut [ActuatorKind], &'s mut [TorqueState], &'s [Bound]);

/// The held actuators with one torque state and one goal bound per
/// actuator, in the same order.
pub(crate) trait Slots {
    /// The held actuators, ascending by `Id`.
    fn kinds(&self) -> &[ActuatorKind];

    /// The held actuators, their states and their bounds, same length and
    /// order.
    fn split_mut(&mut self) -> Held<'_>;

    /// Empties the list. States become `Off`, bounds `None`.
    fn clear(&mut self);

    /// Appends `kind` with state `Off` and goal bound `bound`.
    /// `TooManyActuators` past capacity.
    fn push(&mut self, kind: ActuatorKind, bound: Bound) -> Result<(), SetupError>;
}

/// The slots of a `Bus`: a list of at most `N` actuators, `N` states and
/// `N` bounds. Entries at `i >= len` are unused and kept `Off` and `None`.
pub(crate) struct FixedSlots<'a, const N: usize> {
    actuators: &'a mut heapless::Vec<ActuatorKind, N>,
    states: &'a mut [TorqueState; N],
    bounds: &'a mut [Bound; N],
}

impl<'a, const N: usize> FixedSlots<'a, N> {
    /// Borrows a bus's three storage fields.
    pub(crate) fn new(
        actuators: &'a mut heapless::Vec<ActuatorKind, N>,
        states: &'a mut [TorqueState; N],
        bounds: &'a mut [Bound; N],
    ) -> Self {
        Self {
            actuators,
            states,
            bounds,
        }
    }

    /// The held actuators, their states and bounds, same length and order,
    /// for as long as the bus is borrowed.
    pub(crate) fn into_split(self) -> Held<'a> {
        let len = self.actuators.len();
        let states = self.states.get_mut(..len).unwrap_or_default();
        let bounds = self.bounds.get(..len).unwrap_or_default();
        (self.actuators, states, bounds)
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
        (self.actuators, states, bounds)
    }

    fn clear(&mut self) {
        self.actuators.clear();
        self.states.fill(TorqueState::Off);
        self.bounds.fill(None);
    }

    fn push(&mut self, kind: ActuatorKind, bound: Bound) -> Result<(), SetupError> {
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
        if let Some(state) = self.states.get_mut(index) {
            *state = TorqueState::Off;
        }
        if let Some(slot) = self.bounds.get_mut(index) {
            *slot = bound;
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

    #[test]
    fn empty_slots() {
        let mut actuators = heapless::Vec::<ActuatorKind, 2>::new();
        let mut states = [TorqueState::Faulted; 2];
        let mut bounds = [Some(7); 2];
        let mut slots = FixedSlots::new(&mut actuators, &mut states, &mut bounds);
        slots.clear();
        assert!(slots.kinds().is_empty());
        assert_eq!(find(slots.kinds(), crate::bus::testing::id(1)), None);
        let (kinds, held, limits) = slots.into_split();
        assert!(kinds.is_empty() && held.is_empty() && limits.is_empty());
        assert_eq!(states, [TorqueState::Off; 2]);
        assert_eq!(bounds, [None; 2]);
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
            let mut slots = FixedSlots::new(&mut actuators, &mut states, &mut bounds);
            for n in [1, 4, 7] {
                slots.push(xm430(n), Some(u32::from(n))).unwrap();
            }
            let (kinds, held, limits) = slots.split_mut();
            assert_eq!((kinds.len(), held.len()), (3, 3));
            assert_eq!(held, [TorqueState::Off; 3]);
            assert_eq!(limits, [Some(1), Some(4), Some(7)]);
            assert_eq!(find(slots.kinds(), id(4)), Some(1));
            assert_eq!(find(slots.kinds(), id(5)), None);
            let (kinds, held, _) = slots.into_split();
            assert_eq!(kinds.len(), 3);
            assert_eq!(held.len(), 3);
            held[1] = TorqueState::On;
            let ids: [u8; 3] = core::array::from_fn(|i| kinds[i].id().get());
            assert_eq!(ids, [1, 4, 7]);
            assert_eq!(
                states,
                [TorqueState::Off, TorqueState::On, TorqueState::Off]
            );
        }

        #[test]
        fn push_past_capacity_is_too_many() {
            let mut actuators = heapless::Vec::<ActuatorKind, 2>::new();
            let mut states = [TorqueState::Off; 2];
            let mut bounds = [None; 2];
            let mut slots = FixedSlots::new(&mut actuators, &mut states, &mut bounds);
            slots.push(xm430(1), None).unwrap();
            slots.push(xm430(2), None).unwrap();
            assert_eq!(
                slots.push(xm430(3), None),
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
            let mut slots = FixedSlots::new(&mut actuators, &mut states, &mut bounds);
            slots.push(xm430(1), Some(9)).unwrap();
            slots.push(xm430(2), Some(9)).unwrap();
            slots.clear();
            assert!(slots.kinds().is_empty());
            assert_eq!(states, [TorqueState::Off; 2]);
            assert_eq!(bounds, [None; 2]);
        }
    }
}
