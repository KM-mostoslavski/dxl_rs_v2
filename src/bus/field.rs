//! Fields of the explicit multi-actuator calls (research R-15).
//!
//! A field names one register of every model, and the type its values have:
//! [`Bus::sync_write`](crate::Bus::sync_write) takes a [`WriteField`],
//! [`Bus::sync_read`](crate::Bus::sync_read) (Protocol 2) a [`ReadField`].
//! The field is passed as a value, so no turbofish is needed:
//!
//! ```
//! use dxl_rs_v2::field::{GoalPosition, Raw, WriteField};
//! use dxl_rs_v2::Position;
//!
//! fn value_of<F: WriteField>(_: F, value: F::Value) -> F::Value {
//!     value
//! }
//! let typed: Position = value_of(GoalPosition, Position::ZERO);
//! let raw: i32 = value_of(Raw(GoalPosition), 2048);
//! # let _ = (typed, raw);
//! ```
//!
//! Torque Enable is not a field: a sync write gets no reply, so the bus
//! could not keep its torque state true to the hardware. Use
//! [`Bus::enable_torque`](crate::Bus::enable_torque) and
//! [`Bus::disable_torque`](crate::Bus::disable_torque).

use crate::actuator::ModelData;
use crate::error::ValidationError;
use crate::register::Register;
use crate::units::{Position, PositionScale};

pub(crate) mod sealed {
    use super::{ModelData, PositionScale, Register, ValidationError};

    /// A register every model has. Only this crate implements it.
    pub trait Field {
        /// The field's register in `data`.
        fn register(data: &ModelData) -> Register;
    }

    /// A value type of a field. Only this crate implements it.
    pub trait Value: Copy {
        /// The register value (tick) for `self`, given the model's `scale`.
        ///
        /// # Errors
        ///
        /// [`ValidationError::NotFinite`] or [`ValidationError::DoesNotFit`]
        /// for an angle that maps to no tick.
        fn to_raw(self, scale: &PositionScale) -> Result<i32, ValidationError>;

        /// The value of register value (tick) `raw`, given the model's
        /// `scale`.
        fn from_raw(raw: i32, scale: &PositionScale) -> Self;
    }
}

/// A field the bus can read from many actuators at once.
pub trait ReadField: sealed::Field {
    /// The type of one actuator's value.
    type Value: Copy + sealed::Value;
}

/// A field the bus can write to many actuators at once.
pub trait WriteField: sealed::Field {
    /// The type of one actuator's value.
    type Value: Copy + sealed::Value;
}

/// Goal Position, in radians ([`Position`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GoalPosition;

/// Present Position, in radians ([`Position`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PresentPosition;

/// The raw register value (`i32` ticks) of field `F`, e.g.
/// `Raw(GoalPosition)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Raw<F>(pub F);

impl sealed::Field for GoalPosition {
    fn register(data: &ModelData) -> Register {
        data.goal_position
    }
}

impl sealed::Field for PresentPosition {
    fn register(data: &ModelData) -> Register {
        data.present_position
    }
}

impl<F: sealed::Field> sealed::Field for Raw<F> {
    fn register(data: &ModelData) -> Register {
        F::register(data)
    }
}

impl WriteField for GoalPosition {
    type Value = Position;
}

impl ReadField for PresentPosition {
    type Value = Position;
}

impl<F: WriteField> WriteField for Raw<F> {
    type Value = i32;
}

impl<F: ReadField> ReadField for Raw<F> {
    type Value = i32;
}

impl sealed::Value for Position {
    fn to_raw(self, scale: &PositionScale) -> Result<i32, ValidationError> {
        scale.to_raw(self)
    }

    fn from_raw(raw: i32, scale: &PositionScale) -> Self {
        scale.to_position(raw)
    }
}

impl sealed::Value for i32 {
    fn to_raw(self, _: &PositionScale) -> Result<i32, ValidationError> {
        Ok(self)
    }

    fn from_raw(raw: i32, _: &PositionScale) -> Self {
        raw
    }
}
