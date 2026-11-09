//! Typed SI values for actuator quantities.
//!
//! [`Position`] is an angle in radians, held as an `f32`; [`Velocity`],
//! [`Acceleration`], [`Current`], [`Voltage`], [`Temperature`], [`Load`] and
//! [`Pwm`] are SI values or ratios converted with a per-model
//! [`RegisterScale`]. Each model's
//! [`PositionScale`] maps it to and from the actuator's raw ticks. Every typed
//! value keeps a raw form: the raw `i32` ticks stay available beside it, for
//! values the typed form cannot hold or registers it does not cover.

// DECISION: Acceleration gets its own file beside velocity.rs, one quantity per file, as Pwm beside load.rs.
mod acceleration;
mod electrical;
mod load;
mod position;
// DECISION: Pwm gets its own file beside load.rs, one ratio type per file.
mod pwm;
mod scale;
mod status;
mod thermal;
mod tuning;
mod velocity;

pub use acceleration::Acceleration;
pub use electrical::{Current, Voltage};
pub use load::Load;
pub(crate) use position::Limits;
pub use position::{Position, PositionScale};
pub use pwm::Pwm;
pub use scale::{RegisterScale, Sign};
pub use status::{HardwareErrors, Tick};
pub use thermal::Temperature;
pub use tuning::{ComplianceMargin, ComplianceSlope, Gain};
pub use velocity::{SpeedLimit, Velocity};
