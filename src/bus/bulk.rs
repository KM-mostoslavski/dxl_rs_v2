//! Bulk Read and Bulk Write: several fields of several actuators in one
//! instruction (architect, 2026-11-03).
//!
//! Sync Read and Sync Write send one register layout per instruction, so
//! two fields, or one field on two models with different layouts, take two
//! instructions. Bulk Read and Bulk Write give each actuator its own address
//! and size, so one instruction carries them all:
//!
//! - [`Bus::bulk_read`](crate::Bus::bulk_read): Protocol 2, and Protocol 1
//!   on the firmware that has it (the e-Manual Protocol 1.0 page:
//!   "Supported products : DYNAMIXEL MX, MX(2.0) and X Series (excluding
//!   XL-320)"). On Protocol 1 each target's control table must list Bulk
//!   Read; AX and RX do not, and are refused with
//!   [`ValidationError::UnsupportedInstruction`] before anything is sent.
//! - [`Bus::bulk_write`](crate::Bus::bulk_write): Protocol 2 only. Protocol
//!   1 has no Bulk Write, so the method does not exist there.
//!
//! A call takes one part, or a tuple of up to eight parts, each made by
//! [`read()`] or [`write()`]: a field, its targets and the caller's buffer.
//! Every actuator appears once in a call (the e-Manual: "The same ID cannot
//! be used multiple times in the Parameter").
//!
//! The raw form (`raw-registers` feature) takes any address and length per
//! actuator, with no field checks: `Bus::bulk_read_raw` with one `RawRead`
//! per target, and `Bus::bulk_write_raw` (Protocol 2) with one `RawWrite`
//! per target.
//!
//! [`ValidationError::UnsupportedInstruction`]: crate::ValidationError::UnsupportedInstruction

use core::cell::Cell;
use core::marker::PhantomData;

use crate::actuator::{Actuator, Describe, TorqueState};
use crate::error::{Error, SetupError, ValidationError};
use crate::models::ActuatorKind;
use crate::protocol::{Clock, P2, Protocol, ProtocolVersion};
use crate::register::{Access, Area, Register};
use crate::{Id, Transport};

use super::Bus;
use super::field::{ReadField, WriteField, decode, encode_write, read_site};
use super::group::{self, FirstError, Stage};
use super::id_set::IdSet;
use super::observe::observe;
use super::slots::{Bound, Slots, find};
use super::wire::Wire;

#[cfg(feature = "raw-registers")]
pub(super) mod raw;

#[cfg(feature = "raw-registers")]
pub use raw::{RawRead, RawWrite};

pub(crate) mod sealed {
    use super::{ActuatorKind, Bound, Error, IdSet, Stage, TorqueState};

    /// The held actuators and the staging of one bulk call.
    pub struct Staging<'a> {
        /// The held actuators.
        pub(crate) kinds: &'a [ActuatorKind],
        /// Their torque states.
        pub(crate) states: &'a mut [TorqueState],
        /// Their goal bounds.
        pub(crate) bounds: &'a [Bound],
        /// One stage per target, in part order.
        pub(crate) stages: &'a mut [Stage],
        /// The targets staged so far.
        pub(crate) seen: IdSet,
        /// The next free stage.
        pub(crate) at: usize,
    }

    /// The read stages of one bulk call, handed back to the parts.
    pub struct Delivery<'a> {
        /// The held actuators.
        pub(crate) kinds: &'a [ActuatorKind],
        /// One read stage per target, in part order.
        pub(crate) stages: &'a [Stage],
        /// The next stage to deliver.
        pub(crate) at: usize,
    }

    /// The parts of a Bulk Read. Only this crate implements it.
    pub trait ReadParts {
        /// Stages every target of every part, checked before anything is
        /// sent.
        ///
        /// # Errors
        ///
        /// `NotOnBus`, `DuplicateId`, `BufferLength`, `Unsupported`,
        /// `NoUnit`.
        fn stage(&mut self, staging: &mut Staging<'_>) -> Result<(), Error>;

        /// Converts the read stages, from `delivery.at` on, into each
        /// part's buffer, advancing `delivery.at`.
        ///
        /// # Errors
        ///
        /// As [`stage`](ReadParts::stage); never once staging passed.
        fn deliver(&mut self, delivery: &mut Delivery<'_>) -> Result<(), Error>;
    }

    /// The parts of a Bulk Write. Only this crate implements it.
    pub trait WriteParts {
        /// Stages and encodes every target of every part, checked before
        /// anything is sent.
        ///
        /// # Errors
        ///
        /// `NotOnBus`, `DuplicateId`, `BufferLength`, and every refusal of
        /// a single write.
        fn stage(&mut self, staging: &mut Staging<'_>) -> Result<(), Error>;
    }
}

use sealed::{Delivery, Staging};

/// One or more [`read()`] parts: a [`ReadPart`] or a tuple of up to eight.
pub trait ReadParts: sealed::ReadParts {}

/// One or more [`write()`] parts: a [`WritePart`] or a tuple of up to eight.
pub trait WriteParts: sealed::WriteParts {}

/// One part of a Bulk Read: field `F` of targets `I`, into a buffer the
/// caller owns. Made by [`read()`].
pub struct ReadPart<'o, F: ReadField, I> {
    ids: Option<I>,
    out: &'o mut [F::Value],
    count: usize,
    field: PhantomData<F>,
}

/// One part of a Bulk Write: field `F` to targets `I`. Made by [`write()`].
pub struct WritePart<'v, F: WriteField, I> {
    ids: Option<I>,
    values: &'v [F::Value],
    field: PhantomData<F>,
}

/// A Bulk Read part: field `field` of every target in `ids`, `out[i]`
/// being target `i`. `out` changes only if the whole call succeeds; extra
/// slots stay untouched.
pub fn read<F, I>(field: F, ids: I, out: &mut [F::Value]) -> ReadPart<'_, F, I::IntoIter>
where
    F: ReadField,
    I: IntoIterator<Item = Id>,
{
    let _ = field;
    ReadPart {
        ids: Some(ids.into_iter()),
        out,
        count: 0,
        field: PhantomData,
    }
}

/// A Bulk Write part: `values[i]` of field `field` to target `i` of `ids`,
/// checked like a single write.
pub fn write<F, I>(field: F, ids: I, values: &[F::Value]) -> WritePart<'_, F, I::IntoIter>
where
    F: WriteField,
    I: IntoIterator<Item = Id>,
{
    let _ = field;
    WritePart {
        ids: Some(ids.into_iter()),
        values,
        field: PhantomData,
    }
}

/// The error for a stage that does not exist; `Staging` holds a stage per
/// held actuator and each target once, so it never happens.
fn lost(kinds: &[ActuatorKind]) -> Error {
    SetupError::TooManyActuators {
        capacity: kinds.len(),
    }
    .into()
}

/// Stages every ID of `ids` from `staging.at` on: held, named once in the
/// whole call. Returns the first stage and the count.
fn resolve<I: Iterator<Item = Id>>(
    staging: &mut Staging<'_>,
    ids: Option<I>,
) -> Result<(usize, usize), Error> {
    let start = staging.at;
    for id in ids.into_iter().flatten() {
        let index = find(staging.kinds, id).ok_or(SetupError::NotOnBus { id })?;
        if !staging.seen.insert(id) {
            return Err(ValidationError::DuplicateId { id }.into());
        }
        let slot = u8::try_from(index).ok();
        let (Some(slot), Some(stage)) = (slot, staging.stages.get_mut(staging.at)) else {
            return Err(lost(staging.kinds));
        };
        *stage = Stage {
            slot,
            ..Stage::EMPTY
        };
        staging.at += 1;
    }
    Ok((start, staging.at - start))
}

impl<F: ReadField, I: Iterator<Item = Id>> sealed::ReadParts for ReadPart<'_, F, I> {
    fn stage(&mut self, staging: &mut Staging<'_>) -> Result<(), Error> {
        let (start, count) = resolve(staging, self.ids.take())?;
        self.count = count;
        group::check_out(count, self.out.len())?;
        let kinds = staging.kinds;
        for stage in staging.stages.iter_mut().skip(start).take(count) {
            let kind = kinds
                .get(usize::from(stage.slot))
                .ok_or_else(|| lost(kinds))?;
            let reg = read_site::<F>(kind)?.reg;
            stage.addr = reg.addr;
            stage.size = reg.size;
        }
        Ok(())
    }

    fn deliver(&mut self, delivery: &mut Delivery<'_>) -> Result<(), Error> {
        let kinds = delivery.kinds;
        let mine = delivery.stages.iter().skip(delivery.at).take(self.count);
        for (value, stage) in self.out.iter_mut().zip(mine) {
            let kind = kinds
                .get(usize::from(stage.slot))
                .ok_or_else(|| lost(kinds))?;
            *value = decode::<F>(kind, stage.raw())?;
        }
        delivery.at += self.count;
        Ok(())
    }
}

impl<F: WriteField, I: Iterator<Item = Id>> sealed::WriteParts for WritePart<'_, F, I> {
    fn stage(&mut self, staging: &mut Staging<'_>) -> Result<(), Error> {
        let (start, count) = resolve(staging, self.ids.take())?;
        group::check_values(count, self.values.len())?;
        let (kinds, states, bounds) = (staging.kinds, &mut *staging.states, staging.bounds);
        let mine = staging.stages.iter_mut().skip(start).take(count);
        for (stage, &value) in mine.zip(self.values) {
            let (kind, state) = group::held(kinds, states, stage)?;
            let on = state.check_goal(kind.id());
            let bound = bounds.get(usize::from(stage.slot)).copied().flatten();
            let reg = encode_write::<F>(kind, on, bound, value, &mut stage.bytes)?.0;
            stage.addr = reg.addr;
            stage.size = reg.size;
        }
        Ok(())
    }
}

impl<F: ReadField, I: Iterator<Item = Id>> ReadParts for ReadPart<'_, F, I> {}
impl<F: WriteField, I: Iterator<Item = Id>> WriteParts for WritePart<'_, F, I> {}

/// Implements the part traits for tuples of parts, staged and delivered in
/// order.
macro_rules! tuples {
    ($(($($part:ident),+))*) => {$(
        impl<$($part: sealed::ReadParts),+> sealed::ReadParts for ($($part,)+) {
            fn stage(&mut self, staging: &mut Staging<'_>) -> Result<(), Error> {
                #[allow(non_snake_case, reason = "one binding per tuple element")]
                let ($($part,)+) = self;
                $($part.stage(staging)?;)+
                Ok(())
            }

            fn deliver(&mut self, delivery: &mut Delivery<'_>) -> Result<(), Error> {
                #[allow(non_snake_case, reason = "one binding per tuple element")]
                let ($($part,)+) = self;
                $($part.deliver(delivery)?;)+
                Ok(())
            }
        }

        impl<$($part: sealed::ReadParts),+> ReadParts for ($($part,)+) {}

        impl<$($part: sealed::WriteParts),+> sealed::WriteParts for ($($part,)+) {
            fn stage(&mut self, staging: &mut Staging<'_>) -> Result<(), Error> {
                #[allow(non_snake_case, reason = "one binding per tuple element")]
                let ($($part,)+) = self;
                $($part.stage(staging)?;)+
                Ok(())
            }
        }

        impl<$($part: sealed::WriteParts),+> WriteParts for ($($part,)+) {}
    )*};
}

tuples! {
    (A)
    (A, B)
    (A, B, C)
    (A, B, C, D)
    (A, B, C, D, E)
    (A, B, C, D, E, F)
    (A, B, C, D, E, F, G)
    (A, B, C, D, E, F, G, H)
}

/// A register of `size` bytes at `addr`, to decode a staged reply.
fn layout(addr: u16, size: u8) -> Register {
    Register {
        addr,
        size,
        access: Access::R,
        area: Area::Ram,
    }
}

/// Stages every part, checks the targets take Bulk Read on `P`, sends one
/// Bulk Read and delivers the values. Each part's buffer changes only if
/// every target answered.
pub(super) fn bulk_read<P, T, C, S, B>(
    wire: &mut Wire<'_, P, T, C>,
    slots: &mut S,
    stages: &mut [Stage],
    parts: &mut B,
) -> Result<(), Error>
where
    P: Protocol,
    T: Transport,
    C: Clock,
    S: Slots,
    B: ReadParts,
{
    let (kinds, states, bounds) = slots.split_mut();
    let mut staging = Staging {
        kinds,
        states,
        bounds,
        stages,
        seen: IdSet::new(),
        at: 0,
    };
    parts.stage(&mut staging)?;
    let Staging {
        states, stages, at, ..
    } = staging;
    let stages = stages.get_mut(..at).unwrap_or_default();
    if P::VERSION == ProtocolVersion::P1 {
        for stage in stages.iter() {
            let kind = kinds
                .get(usize::from(stage.slot))
                .ok_or_else(|| lost(kinds))?;
            if !kind.data().bulk_read_p1 {
                return Err(ValidationError::UnsupportedInstruction {
                    id: kind.id(),
                    instruction: "Bulk Read",
                }
                .into());
            }
        }
    }
    if stages.is_empty() {
        return Ok(());
    }
    let cells = Cell::from_mut(&mut *stages).as_slice_of_cells();
    let entries = cells.iter().filter_map(|cell| {
        let stage = cell.get();
        let kind = kinds.get(usize::from(stage.slot))?;
        Some((kind.id(), stage.addr, u16::from(stage.size)))
    });
    let mut first = FirstError::default();
    let mut next = 0;
    let mut scratch = [0; 4];
    wire.bulk_read(entries, &mut scratch, |_, reply| {
        let target = next;
        next += 1;
        let Some(cell) = cells.get(target) else {
            return;
        };
        let mut stage = cell.get();
        let result = reply.and_then(|bytes| Ok(layout(stage.addr, stage.size).decode(bytes)?));
        if let Some(state) = states.get_mut(usize::from(stage.slot)) {
            observe(state, &result);
        }
        if let Ok(raw) = result {
            stage.bytes = raw.to_le_bytes();
            cell.set(stage);
        }
        first.note(target, &result);
    })?;
    first.into_result()?;
    parts.deliver(&mut Delivery {
        kinds,
        stages,
        at: 0,
    })
}

/// Stages and validates every part, then sends one Bulk Write.
pub(super) fn bulk_write<T, C, S, B>(
    wire: &mut Wire<'_, P2, T, C>,
    slots: &mut S,
    stages: &mut [Stage],
    parts: &mut B,
) -> Result<(), Error>
where
    T: Transport,
    C: Clock,
    S: Slots,
    B: WriteParts,
{
    let (kinds, states, bounds) = slots.split_mut();
    let mut staging = Staging {
        kinds,
        states,
        bounds,
        stages,
        seen: IdSet::new(),
        at: 0,
    };
    parts.stage(&mut staging)?;
    let Staging { stages, at, .. } = staging;
    let stages = stages.get(..at).unwrap_or_default();
    if stages.is_empty() {
        return Ok(());
    }
    let entries = stages.iter().filter_map(|stage| {
        let kind = kinds.get(usize::from(stage.slot))?;
        Some((kind.id(), stage.addr, stage.data()))
    });
    wire.bulk_write(entries)
}

/// Bulk Read, on both protocols.
impl<P: Protocol, T: Transport, C: Clock, const N: usize> Bus<P, T, C, N> {
    /// Reads every part with one Bulk Read: each part's field of each of its
    /// targets, into that part's buffer. Several fields, and models with
    /// different register layouts, share one instruction.
    ///
    /// Strict, like [`read_positions`](Bus::read_positions): the buffers
    /// change only if every target answered; every reply passes the fault
    /// check. Not retried. An empty call sends nothing.
    ///
    /// On Protocol 1, Bulk Read exists on MX, MX(2.0) and X series firmware
    /// except XL-320 (e-Manual Protocol 1.0): each target's control table
    /// must list it (MX-64 does; AX and RX do not).
    #[cfg_attr(
        all(feature = "mock", feature = "x-series"),
        doc = r#"
The e-Manual example: Present Input Voltage of ID 1 and Present
Temperature of ID 2, two XM430-W350, in one instruction:

```
use dxl_rs_v2::field::{PresentTemperature, PresentVoltage, Raw};
use dxl_rs_v2::{Bus, Config, Error, Id, MockTime, MockTransport, ModelNumber, P2, bulk};

# let time = MockTime::new();
# let mut line = MockTransport::new(&time);
# for (n, at) in [(1u8, 1), (2, 5)] {
#     let pong = [0xFF, 0xFF, 0xFD, 0x00, n, 0x07, 0x00, 0x55, 0x00, 0xFC, 0x03, 0x34];
#     let min = [0xFF, 0xFF, 0xFD, 0x00, n, 0x08, 0x00, 0x55, 0x00, 0x00, 0x00, 0x00, 0x00];
#     let max = [0xFF, 0xFF, 0xFD, 0x00, n, 0x08, 0x00, 0x55, 0x00, 0xFF, 0x0F, 0x00, 0x00];
#     let mode = [0xFF, 0xFF, 0xFD, 0x00, n, 0x05, 0x00, 0x55, 0x00, 0x03];
#     for (i, body) in [&pong[..], &min, &max, &mode].into_iter().enumerate() {
#         line.reply_to(at + i, &with_crc(body));
#     }
# }
# line.reply_to(9, &[0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x06, 0x00, 0x55, 0x00, 0x77, 0x00, 0xC3, 0x69])
#     .reply_to(9, &[0xFF, 0xFF, 0xFD, 0x00, 0x02, 0x05, 0x00, 0x55, 0x00, 0x24, 0x8B, 0xA9]);
# fn with_crc(body: &[u8]) -> Vec<u8> {
#     let mut crc: u16 = 0;
#     for &byte in body {
#         crc ^= u16::from(byte) << 8;
#         for _ in 0..8 {
#             crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x8005 } else { crc << 1 };
#         }
#     }
#     let mut packet = body.to_vec();
#     packet.extend_from_slice(&crc.to_le_bytes());
#     packet
# }
let mut bus = Bus::<P2, _, _>::new(line, time.ticker(10), Config::new());
let (a, b) = (Id::new(1)?, Id::new(2)?);
bus.declare([(a, ModelNumber(1020)), (b, ModelNumber(1020))])?;

let mut volts = [0; 1];
let mut celsius = [0; 1];
bus.bulk_read((
    bulk::read(Raw(PresentVoltage), [a], &mut volts),
    bulk::read(Raw(PresentTemperature), [b], &mut celsius),
))?;
assert_eq!((volts, celsius), ([119], [36]));
# Ok::<(), Error>(())
```
"#
    )]
    ///
    /// # Errors
    ///
    /// Refused with nothing sent, every buffer unchanged:
    /// - [`SetupError::NotOnBus`], [`ValidationError::DuplicateId`] (an
    ///   actuator in two places of the call), [`ValidationError::BufferLength`].
    /// - [`ValidationError::Unsupported`], [`ValidationError::NoUnit`].
    /// - [`ValidationError::UnsupportedInstruction`]: Protocol 1, and a
    ///   target's table does not list Bulk Read.
    ///
    /// On the wire, buffers unchanged: the first reply reporting a hardware
    /// fault in target order, else the first error in target order; a
    /// failing transport stops the call at once.
    ///
    /// [`SetupError::NotOnBus`]: crate::SetupError::NotOnBus
    /// [`ValidationError::DuplicateId`]: crate::ValidationError::DuplicateId
    /// [`ValidationError::BufferLength`]: crate::ValidationError::BufferLength
    /// [`ValidationError::Unsupported`]: crate::ValidationError::Unsupported
    /// [`ValidationError::NoUnit`]: crate::ValidationError::NoUnit
    /// [`ValidationError::UnsupportedInstruction`]: crate::ValidationError::UnsupportedInstruction
    pub fn bulk_read<B: ReadParts>(&mut self, mut parts: B) -> Result<(), Error> {
        let mut stages = [Stage::EMPTY; N];
        let (mut wire, mut slots) = self.split();
        bulk_read(&mut wire, &mut slots, &mut stages, &mut parts)
    }
}

/// Bulk Write, Protocol 2 only.
impl<T: Transport, C: Clock, const N: usize> Bus<P2, T, C, N> {
    /// Writes every part with one Bulk Write: each part's field to each of
    /// its targets. Protocol 2 only.
    ///
    /// Every target is checked as a single
    /// [`ActuatorRef::write`](crate::ActuatorRef::write) checks it (torque
    /// on for a goal, operating mode, limits, register size) before any
    /// byte, so a refusal sends nothing to anyone. A Bulk Write gets no
    /// reply, so a fault shows on the next read. An empty call sends
    /// nothing.
    ///
    /// Protocol 1 has no Bulk Write, so the method does not exist there:
    ///
    /// ```compile_fail,E0599
    /// use dxl_rs_v2::field::Led;
    /// use dxl_rs_v2::{Bus, Clock, Id, P1, Transport, bulk};
    ///
    /// fn blink<T: Transport, C: Clock>(bus: &mut Bus<P1, T, C>, id: Id) {
    ///     let _ = bus.bulk_write(bulk::write(Led, [id], &[true]));
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Refused with nothing sent: [`SetupError::NotOnBus`],
    /// [`ValidationError::DuplicateId`], [`ValidationError::BufferLength`],
    /// and every refusal of a single write. Otherwise a transport error.
    ///
    /// [`SetupError::NotOnBus`]: crate::SetupError::NotOnBus
    /// [`ValidationError::DuplicateId`]: crate::ValidationError::DuplicateId
    /// [`ValidationError::BufferLength`]: crate::ValidationError::BufferLength
    pub fn bulk_write<B: WriteParts>(&mut self, mut parts: B) -> Result<(), Error> {
        let mut stages = [Stage::EMPTY; N];
        let (mut wire, mut slots) = self.split();
        bulk_write(&mut wire, &mut slots, &mut stages, &mut parts)
    }
}
