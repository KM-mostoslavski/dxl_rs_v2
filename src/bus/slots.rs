//! Where a bus keeps its actuators and their torque states (data-model.md
//! "Slots"). Setup and every lookup go through [`Slots`], so the engine does
//! not depend on the storage.

use crate::Id;
use crate::actuator::{Actuator, TorqueState};
use crate::error::SetupError;
use crate::models::ActuatorKind;

/// The held actuators and one torque state per actuator, in the same order.
pub(crate) trait Slots {
    /// The held actuators, ascending by `Id`.
    fn kinds(&self) -> &[ActuatorKind];

    /// The held actuators and their states, same length and order.
    fn split_mut(&mut self) -> (&mut [ActuatorKind], &mut [TorqueState]);

    /// Empties the list. States become `Off`.
    fn clear(&mut self);

    /// Appends `kind` with state `Off`. `TooManyActuators` past capacity.
    fn push(&mut self, kind: ActuatorKind) -> Result<(), SetupError>;
}

/// The slots of a `Bus`: a list of at most `N` actuators and `N` states.
/// States at `i >= len` are unused and kept `Off`.
pub(crate) struct FixedSlots<'a, const N: usize> {
    actuators: &'a mut heapless::Vec<ActuatorKind, N>,
    states: &'a mut [TorqueState; N],
}

impl<'a, const N: usize> FixedSlots<'a, N> {
    /// Borrows a bus's two storage fields.
    pub(crate) fn new(
        actuators: &'a mut heapless::Vec<ActuatorKind, N>,
        states: &'a mut [TorqueState; N],
    ) -> Self {
        Self { actuators, states }
    }

    /// The held actuators and their states, same length and order, for as
    /// long as the bus is borrowed.
    pub(crate) fn into_split(self) -> (&'a mut [ActuatorKind], &'a mut [TorqueState]) {
        let len = self.actuators.len();
        let states = self.states.get_mut(..len).unwrap_or_default();
        (self.actuators, states)
    }
}

impl<const N: usize> Slots for FixedSlots<'_, N> {
    fn kinds(&self) -> &[ActuatorKind] {
        self.actuators
    }

    fn split_mut(&mut self) -> (&mut [ActuatorKind], &mut [TorqueState]) {
        let len = self.actuators.len();
        let states = self.states.get_mut(..len).unwrap_or_default();
        (self.actuators, states)
    }

    fn clear(&mut self) {
        self.actuators.clear();
        self.states.fill(TorqueState::Off);
    }

    fn push(&mut self, kind: ActuatorKind) -> Result<(), SetupError> {
        let index = self.actuators.len();
        self.actuators
            .push(kind)
            .map_err(|_| SetupError::TooManyActuators { capacity: N })?;
        if let Some(state) = self.states.get_mut(index) {
            *state = TorqueState::Off;
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
        let mut slots = FixedSlots::new(&mut actuators, &mut states);
        slots.clear();
        assert!(slots.kinds().is_empty());
        assert_eq!(find(slots.kinds(), crate::bus::testing::id(1)), None);
        let (kinds, held) = slots.into_split();
        assert!(kinds.is_empty() && held.is_empty());
        assert_eq!(states, [TorqueState::Off; 2]);
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
            let mut slots = FixedSlots::new(&mut actuators, &mut states);
            for n in [1, 4, 7] {
                slots.push(xm430(n)).unwrap();
            }
            let (kinds, held) = slots.split_mut();
            assert_eq!((kinds.len(), held.len()), (3, 3));
            assert_eq!(held, [TorqueState::Off; 3]);
            assert_eq!(find(slots.kinds(), id(4)), Some(1));
            assert_eq!(find(slots.kinds(), id(5)), None);
            let (kinds, held) = slots.into_split();
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
            let mut slots = FixedSlots::new(&mut actuators, &mut states);
            slots.push(xm430(1)).unwrap();
            slots.push(xm430(2)).unwrap();
            assert_eq!(
                slots.push(xm430(3)),
                Err(SetupError::TooManyActuators { capacity: 2 })
            );
            assert_eq!(slots.kinds().len(), 2);
        }

        #[test]
        fn clear_empties_and_resets_every_state() {
            let mut actuators = heapless::Vec::<ActuatorKind, 2>::new();
            let mut states = [TorqueState::Faulted; 2];
            let mut slots = FixedSlots::new(&mut actuators, &mut states);
            slots.push(xm430(1)).unwrap();
            slots.push(xm430(2)).unwrap();
            slots.clear();
            assert!(slots.kinds().is_empty());
            assert_eq!(states, [TorqueState::Off; 2]);
        }
    }
}
