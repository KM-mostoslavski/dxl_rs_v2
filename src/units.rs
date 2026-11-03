//! Typed SI values for actuator quantities.
//!
//! [`Position`] is an angle in radians, held as an `f32`; [`Velocity`],
//! [`Current`], [`Voltage`], [`Temperature`] and [`Load`] are SI values
//! converted with a per-model [`RegisterScale`]. Each model's
//! [`PositionScale`] maps it to and from the actuator's raw ticks. Every typed
//! value keeps a raw form: the raw `i32` ticks stay available beside it, for
//! values the typed form cannot hold or registers it does not cover.

mod electrical;
mod load;
mod position;
mod scale;
mod status;
mod thermal;
mod tuning;
mod velocity;

pub use electrical::{Current, Voltage};
pub use load::Load;
pub(crate) use position::Limits;
pub use position::{Position, PositionScale};
pub use scale::{RegisterScale, Sign};
pub use status::{HardwareErrors, Tick};
pub use thermal::Temperature;
pub use tuning::{ComplianceMargin, ComplianceSlope, Gain};
pub use velocity::{SpeedLimit, Velocity};
