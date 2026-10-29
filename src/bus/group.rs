//! The multi-actuator engine (research R-8): resolve the targets, check the
//! value count, validate and stage every target, then group the stages by
//! register layout. One N-free core over slices; `Bus` passes a stack
//! `[Stage; N]`.

use crate::Id;
use crate::actuator::{Actuator, Describe, TorqueState};
use crate::error::{Error, SetupError, ValidationError};
use crate::models::ActuatorKind;

use core::cell::Cell;

use super::field::WriteField;
use super::field::sealed::{Field, Value};
use super::id_set::IdSet;
use super::observe::{is_hardware_fault, observe};
use super::slots::find;
use super::wire::{ReadMany, Wire};

/// One target of a multi-actuator call, staged before anything is sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Stage {
    /// Index into the slot list.
    pub(crate) slot: u8,
    /// Register address of this target's field.
    pub(crate) addr: u16,
    /// Register size: 1, 2 or 4.
    pub(crate) size: u8,
    /// Encoded value (writes) or received bytes (reads).
    pub(crate) bytes: [u8; 4],
}

impl Stage {
    /// An unused stage.
    pub(crate) const EMPTY: Self = Self {
        slot: 0,
        addr: 0,
        size: 0,
        bytes: [0; 4],
    };

    /// The register layout this stage is sent with.
    fn key(&self) -> (u16, u8) {
        (self.addr, self.size)
    }

    /// The tick a read staged.
    pub(crate) fn raw(&self) -> i32 {
        i32::from_le_bytes(self.bytes)
    }

    /// The `size` staged bytes.
    pub(crate) fn data(&self) -> &[u8] {
        self.bytes.get(..usize::from(self.size)).unwrap_or_default()
    }
}

/// The error for a slot index that is not in the list. `resolve` only
/// stages slots it found, so it never happens.
fn lost(kinds: &[ActuatorKind]) -> Error {
    SetupError::TooManyActuators {
        capacity: kinds.len(),
    }
    .into()
}

/// The actuator of `stage` and its torque state.
pub(crate) fn held<'k>(
    kinds: &'k [ActuatorKind],
    states: &'k mut [TorqueState],
    stage: &Stage,
) -> Result<(&'k ActuatorKind, &'k mut TorqueState), Error> {
    let slot = usize::from(stage.slot);
    match (kinds.get(slot), states.get_mut(slot)) {
        (Some(kind), Some(state)) => Ok((kind, state)),
        _ => Err(lost(kinds)),
    }
}

/// Looks up each target in the caller's order and stages its slot.
///
/// Returns the staged targets, `stages[..targets]`.
///
/// # Errors
///
/// - [`SetupError::NotOnBus`]: a target the bus does not hold.
/// - [`ValidationError::DuplicateId`]: a target named twice.
pub(crate) fn resolve<'s, I: IntoIterator<Item = Id>>(
    kinds: &[ActuatorKind],
    ids: I,
    stages: &'s mut [Stage],
) -> Result<&'s mut [Stage], Error> {
    let mut seen = IdSet::new();
    let mut count = 0;
    for id in ids {
        let index = find(kinds, id).ok_or(SetupError::NotOnBus { id })?;
        if !seen.insert(id) {
            return Err(ValidationError::DuplicateId { id }.into());
        }
        let slot = u8::try_from(index).ok();
        let (Some(slot), Some(stage)) = (slot, stages.get_mut(count)) else {
            return Err(lost(kinds));
        };
        *stage = Stage {
            slot,
            ..Stage::EMPTY
        };
        count += 1;
    }
    Ok(stages.get_mut(..count).unwrap_or_default())
}

/// Checks that a write has one value per target.
///
/// # Errors
///
/// [`ValidationError::BufferLength`] when `got != targets`.
pub(crate) fn check_values(targets: usize, got: usize) -> Result<(), ValidationError> {
    if got == targets {
        Ok(())
    } else {
        Err(ValidationError::BufferLength { targets, got })
    }
}

/// Validates every staged target in order and encodes its value: torque on,
/// the value converted to a tick, the target's limit mode, the register
/// size. The first failure refuses the whole call.
///
/// `values` has one value per stage (see [`check_values`]).
pub(crate) fn validate_writes<F: WriteField>(
    kinds: &[ActuatorKind],
    states: &mut [TorqueState],
    stages: &mut [Stage],
    values: &[F::Value],
) -> Result<(), Error> {
    for (stage, &value) in stages.iter_mut().zip(values) {
        let (kind, state) = held(kinds, states, stage)?;
        state.check_goal(kind.id())?;
        let data = kind.data();
        let raw = kind.cache().check_goal(value.to_raw(&data.scale)?)?;
        let reg = F::register(data);
        reg.encode(raw, &mut stage.bytes)?;
        stage.addr = reg.addr;
        stage.size = reg.size;
    }
    Ok(())
}

/// The distinct register layouts `(addr, size)` of `stages`, in
/// first-appearance order: one instruction per layout.
pub(crate) fn groups(stages: &[Stage]) -> impl Iterator<Item = (u16, u8)> + '_ {
    stages
        .iter()
        .enumerate()
        .filter(|&(i, stage)| !stages.iter().take(i).any(|seen| seen.key() == stage.key()))
        .map(|(_, stage)| stage.key())
}

/// The targets of layout `key`, in target order, with their staged bytes.
pub(crate) fn entries<'s>(
    kinds: &'s [ActuatorKind],
    stages: &'s [Stage],
    key: (u16, u8),
) -> impl Iterator<Item = (Id, &'s [u8])> + Clone + 's {
    stages
        .iter()
        .filter(move |stage| stage.key() == key)
        .filter_map(|stage| Some((kinds.get(usize::from(stage.slot))?.id(), stage.data())))
}

/// The error a call over several targets returns: the first hardware fault
/// in target order, else the first error in target order.
#[derive(Debug, Default)]
pub(crate) struct FirstError(Option<(usize, Error)>);

impl FirstError {
    /// Notes the result of target `index`.
    pub(crate) fn note<V>(&mut self, index: usize, result: &Result<V, Error>) {
        let Err(err) = result else { return };
        let better = match &self.0 {
            None => true,
            Some((kept_at, kept)) => match (is_hardware_fault(kept), is_hardware_fault(err)) {
                (false, true) => true,
                (true, false) => false,
                _ => index < *kept_at,
            },
        };
        if better {
            self.0 = Some((index, *err));
        }
    }

    /// `Ok` if no target failed, else the kept error.
    pub(crate) fn into_result(self) -> Result<(), Error> {
        self.0.map_or(Ok(()), |(_, err)| Err(err))
    }
}

/// Checks that a read has a slot of `out` for every target; extra slots
/// stay untouched.
///
/// # Errors
///
/// [`ValidationError::BufferLength`] when `got < targets`.
pub(crate) fn check_out(targets: usize, got: usize) -> Result<(), ValidationError> {
    if got >= targets {
        Ok(())
    } else {
        Err(ValidationError::BufferLength { targets, got })
    }
}

/// Stages a read of field `F`: each target's register layout.
pub(crate) fn stage_reads<F: Field>(
    kinds: &[ActuatorKind],
    stages: &mut [Stage],
) -> Result<(), Error> {
    for stage in stages {
        let kind = kinds
            .get(usize::from(stage.slot))
            .ok_or_else(|| lost(kinds))?;
        let reg = F::register(kind.data());
        stage.addr = reg.addr;
        stage.size = reg.size;
    }
    Ok(())
}

/// Reads field `F` of every staged target: one `R::run` per register
/// layout, replies mapped to their stages in target order (the e-Manual
/// Sync Read reply order). Each decoded tick goes into its stage, and every
/// target's result passes `observe`.
///
/// Returns the first hardware fault in target order, else the first error.
/// A failing `R::run` (an invalid request or a failing transport) aborts
/// at once.
pub(crate) fn read_stages<P, T, C, F, R>(
    wire: &mut Wire<'_, P, T, C>,
    kinds: &[ActuatorKind],
    states: &mut [TorqueState],
    stages: &mut [Stage],
) -> Result<(), Error>
where
    F: Field,
    R: ReadMany<P, T, C>,
{
    let cells = Cell::from_mut(stages).as_slice_of_cells();
    let mut first = FirstError::default();
    for (index, cell) in cells.iter().enumerate() {
        let key = cell.get().key();
        if cells.iter().take(index).any(|seen| seen.get().key() == key) {
            continue;
        }
        let members = cells
            .iter()
            .enumerate()
            .filter(move |(_, member)| member.get().key() == key);
        let ids = members
            .clone()
            .filter_map(|(_, member)| Some(kinds.get(usize::from(member.get().slot))?.id()));
        let mut cursor = members;
        let mut scratch = [0; 4];
        R::run(wire, key.0, key.1, ids, &mut scratch, |_, reply| {
            let Some((target, member)) = cursor.next() else {
                return;
            };
            let mut stage = member.get();
            let slot = usize::from(stage.slot);
            let Some(kind) = kinds.get(slot) else {
                return;
            };
            let result = reply.and_then(|bytes| Ok(F::register(kind.data()).decode(bytes)?));
            if let Ok(raw) = result {
                stage.bytes = raw.to_le_bytes();
                member.set(stage);
            }
            if let Some(state) = states.get_mut(slot) {
                observe(state, &result);
            }
            first.note(target, &result);
        })?;
    }
    first.into_result()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::testing::id;
    use crate::error::{StatusError, StatusReport, TransportError};

    #[test]
    fn stage_is_8_bytes() {
        assert_eq!(size_of::<Stage>(), 8);
        assert_eq!(Stage::default(), Stage::EMPTY);
    }

    #[test]
    fn empty_targets_stage_nothing() {
        let mut stages = [Stage::EMPTY; 4];
        let staged = resolve(&[], [], &mut stages).unwrap();
        assert!(staged.is_empty());
        assert_eq!(groups(staged).count(), 0);
    }

    #[test]
    fn unknown_target_is_not_on_bus() {
        let mut stages = [Stage::EMPTY; 4];
        assert_eq!(
            resolve(&[], [id(3)], &mut stages),
            Err(SetupError::NotOnBus { id: id(3) }.into())
        );
    }

    #[test]
    fn value_count_must_match() {
        assert_eq!(check_values(2, 2), Ok(()));
        assert_eq!(
            check_values(2, 1),
            Err(ValidationError::BufferLength { targets: 2, got: 1 })
        );
        assert_eq!(
            check_values(0, 1),
            Err(ValidationError::BufferLength { targets: 0, got: 1 })
        );
    }

    #[test]
    fn groups_come_in_first_appearance_order() {
        let stage = |slot, addr, size| Stage {
            slot,
            addr,
            size,
            bytes: [slot; 4],
        };
        let stages = [
            stage(0, 30, 2),
            stage(1, 116, 4),
            stage(2, 30, 2),
            stage(3, 116, 4),
            stage(4, 30, 4),
        ];
        let keys: [(u16, u8); 3] = {
            let mut found = groups(&stages);
            core::array::from_fn(|_| found.next().unwrap())
        };
        assert_eq!(keys, [(30, 2), (116, 4), (30, 4)]);
        assert_eq!(groups(&stages).count(), 3);
        assert_eq!(stages[0].data(), &[0, 0]);
        assert_eq!(stages[1].data(), &[1, 1, 1, 1]);
    }

    fn timeout() -> Result<(), Error> {
        Err(TransportError::Timeout { id: Some(id(1)) }.into())
    }

    fn alert(n: u8) -> Result<(), Error> {
        Err(StatusError {
            id: id(n),
            report: StatusReport::P2 {
                alert: true,
                code: None,
            },
        }
        .into())
    }

    #[test]
    fn first_error_prefers_a_hardware_fault() {
        let mut first = FirstError::default();
        first.note(0, &Ok::<(), Error>(()));
        assert_eq!(FirstError::default().into_result(), Ok(()));
        first.note(1, &timeout());
        first.note(3, &alert(3));
        first.note(2, &alert(2));
        first.note(4, &alert(4));
        first.note(0, &timeout());
        assert_eq!(first.into_result(), alert(2));

        let mut first = FirstError::default();
        first.note(5, &timeout());
        first.note(
            6,
            &Err::<(), _>(TransportError::Timeout { id: None }.into()),
        );
        assert_eq!(first.into_result(), timeout());

        let mut first = FirstError::default();
        first.note(
            5,
            &Err::<(), _>(TransportError::Timeout { id: None }.into()),
        );
        first.note(2, &timeout());
        assert_eq!(first.into_result(), timeout());
        assert_eq!(check_out(2, 3), Ok(()));
        assert_eq!(
            check_out(2, 1),
            Err(ValidationError::BufferLength { targets: 2, got: 1 })
        );
    }

    #[cfg(feature = "x-series")]
    mod with_models {
        use super::*;
        use crate::actuator::{LimitMode, ModelNumber};
        use crate::bus::field::{GoalPosition, Raw};
        use crate::protocol::ProtocolVersion;
        use crate::units::Position;

        /// Held actuators: an XM430-W350 at IDs 1 and 2 and an XL-320 at
        /// ID 3, limits 1000..=3000 (XL-320: 0..=1023), in `modes`.
        fn held(modes: [LimitMode; 3]) -> [ActuatorKind; 3] {
            let kind = |model, n, min, max| {
                let mut kind =
                    ActuatorKind::new(ProtocolVersion::P2, ModelNumber(model), 52, id(n)).unwrap();
                kind.cache_mut().record_limits(min, max).unwrap();
                kind
            };
            let mut kinds = [
                kind(1020, 1, 1000, 3000),
                kind(1020, 2, 1000, 3000),
                kind(350, 3, 0, 1023),
            ];
            for (kind, mode) in kinds.iter_mut().zip(modes) {
                kind.cache_mut().record_mode(mode);
            }
            kinds
        }

        const BOUNDED: [LimitMode; 3] = [LimitMode::Bounded; 3];
        fn on() -> [TorqueState; 3] {
            [TorqueState::On; 3]
        }

        #[test]
        fn resolve_stages_slots_in_target_order() {
            let kinds = held(BOUNDED);
            let mut stages = [Stage::EMPTY; 3];
            let staged = resolve(&kinds, [id(3), id(1)], &mut stages).unwrap();
            let slots: [u8; 2] = core::array::from_fn(|i| staged[i].slot);
            assert_eq!(slots, [2, 0]);
        }

        #[test]
        fn repeated_target_is_a_duplicate() {
            let kinds = held(BOUNDED);
            let mut stages = [Stage::EMPTY; 3];
            assert_eq!(
                resolve(&kinds, [id(1), id(3), id(1)], &mut stages),
                Err(ValidationError::DuplicateId { id: id(1) }.into())
            );
            assert_eq!(
                resolve(&kinds, [id(1), id(9), id(1)], &mut stages),
                Err(SetupError::NotOnBus { id: id(9) }.into())
            );
        }

        fn validate_raw(
            kinds: &[ActuatorKind],
            states: &mut [TorqueState],
            targets: &[u8],
            values: &[i32],
        ) -> Result<[Stage; 3], Error> {
            let mut stages = [Stage::EMPTY; 3];
            let staged = resolve(kinds, targets.iter().map(|&n| id(n)), &mut stages)?;
            check_values(staged.len(), values.len())?;
            validate_writes::<Raw<GoalPosition>>(kinds, states, staged, values)?;
            Ok(stages)
        }

        #[test]
        fn validate_encodes_each_target_with_its_layout() {
            let kinds = held(BOUNDED);
            let stages = validate_raw(&kinds, &mut on(), &[3, 1], &[512, 2048]).unwrap();
            assert_eq!((stages[0].addr, stages[0].size), (30, 2));
            assert_eq!(stages[0].data(), &512u16.to_le_bytes());
            assert_eq!((stages[1].addr, stages[1].size), (116, 4));
            assert_eq!(stages[1].data(), &2048i32.to_le_bytes());
        }

        #[test]
        fn first_failure_in_target_order_wins() {
            let kinds = held(BOUNDED);
            let mut states = [TorqueState::On, TorqueState::Off, TorqueState::Faulted];
            assert_eq!(
                validate_raw(&kinds, &mut states, &[1, 3, 2], &[0, 512, 2048]),
                Err(ValidationError::OutOfLimits {
                    id: id(1),
                    raw: 0,
                    min: 1000,
                    max: 3000
                }
                .into())
            );
            assert_eq!(
                validate_raw(&kinds, &mut states, &[1, 3, 2], &[2048, 512, 2048]),
                Err(ValidationError::ActuatorIsOff { id: id(3) }.into())
            );
            assert_eq!(
                validate_raw(&kinds, &mut states, &[2, 1], &[2048, 2048]),
                Err(ValidationError::ActuatorIsOff { id: id(2) }.into())
            );
        }

        #[test]
        fn limit_mode_decides_per_target() {
            let kinds = held([
                LimitMode::Unbounded,
                LimitMode::NoPosition,
                LimitMode::Bounded,
            ]);
            let stages = validate_raw(&kinds, &mut on(), &[1], &[100_000]).unwrap();
            assert_eq!(stages[0].data(), &100_000i32.to_le_bytes());
            assert_eq!(
                validate_raw(&kinds, &mut on(), &[1, 2], &[0, 2048]),
                Err(ValidationError::NotPositionMode { id: id(2) }.into())
            );
            assert_eq!(
                validate_raw(&kinds, &mut on(), &[3], &[70_000]),
                Err(ValidationError::OutOfLimits {
                    id: id(3),
                    raw: 70_000,
                    min: 0,
                    max: 1023
                }
                .into())
            );
        }

        #[test]
        fn typed_values_convert_and_nan_is_not_finite() {
            let kinds = held(BOUNDED);
            let mut stages = [Stage::EMPTY; 3];
            let staged = resolve(&kinds, [id(1)], &mut stages).unwrap();
            validate_writes::<GoalPosition>(&kinds, &mut on(), staged, &[Position::ZERO]).unwrap();
            assert_eq!(staged[0].data(), &2048i32.to_le_bytes());
            assert_eq!(
                validate_writes::<GoalPosition>(
                    &kinds,
                    &mut on(),
                    staged,
                    &[Position::from_radians(f32::NAN)]
                ),
                Err(ValidationError::NotFinite.into())
            );
        }

        #[test]
        fn entries_follow_target_order_and_clone() {
            let kinds = held(BOUNDED);
            let stages = validate_raw(&kinds, &mut on(), &[2, 3, 1], &[1500, 600, 2500]).unwrap();
            let keys: [(u16, u8); 2] = {
                let mut found = groups(&stages);
                core::array::from_fn(|_| found.next().unwrap())
            };
            assert_eq!(keys, [(116, 4), (30, 2)]);
            let xm = entries(&kinds, &stages, (116, 4));
            let ids: [u8; 2] = {
                let mut each = xm.clone().map(|(id, _)| id.get());
                core::array::from_fn(|_| each.next().unwrap())
            };
            assert_eq!(ids, [2, 1]);
            assert_eq!(xm.count(), 2);
            let mut xl = entries(&kinds, &stages, (30, 2));
            assert_eq!(xl.next(), Some((id(3), &600u16.to_le_bytes()[..])));
            assert_eq!(xl.next(), None);
        }

        #[test]
        fn lost_slot_is_an_error() {
            let kinds = held(BOUNDED);
            let stage = Stage {
                slot: 7,
                ..Stage::EMPTY
            };
            assert!(super::super::held(&kinds, &mut [TorqueState::On; 3], &stage).is_err());
        }
    }
}
