//! Typed SI values for actuator quantities.
//!
//! [`Position`] is an angle in radians, held as an `f32`. Each model's
//! [`PositionScale`] maps it to and from the actuator's raw ticks. Every typed
//! value keeps a raw form: the raw `i32` ticks stay available beside it, for
//! values the typed form cannot hold or registers it does not cover.

mod position;

pub(crate) use position::Limits;
pub use position::{Position, PositionScale};
