//! `no_std`, heap-free HAL for Dynamixel actuators (Protocol 1 & 2).
//!
//! The design lives in `docs/ARCHITECTURE.md`.
#![no_std]
#![warn(clippy::mod_module_files)]
#![warn(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod actuator;
mod bus;
mod error;
mod id;
pub mod models;
mod protocol;
mod register;
mod transport;
mod units;

pub use actuator::{Actuator, LimitMode, Model, ModelNumber, TorqueState};
pub use bus::{
    ActuatorRef, Bus, Config, DefaultClock, IdSet, IdSetIter, ModelRef, NoClock, bulk, field,
};
#[cfg(feature = "std")]
pub use bus::{DynBus, SerialBus};
/// The `embedded-io` crate this one builds on, so users can implement and
/// call [`Transport`] without a version-matched dependency of their own.
pub use embedded_io;
pub use error::{
    Celsius, Decivolts, Direction, Error, FaultError, Instruction, P1Flag, P1Flags, P2Code,
    ProtocolError, Request, SetupError, StatusError, StatusReport, Step, TemperatureError,
    TransportError, ValidationError, VoltageError,
};
pub use id::Id;
pub use models::ActuatorKind;
#[cfg(feature = "std")]
pub use protocol::StdClock;
pub use protocol::{Clock, P1, P2, Protocol, ProtocolVersion};
/// The `serialport` crate behind [`SerialTransport`], for its port and error
/// types.
#[cfg(feature = "std")]
pub use serialport;
#[cfg(feature = "std")]
pub use transport::SerialTransport;
pub use transport::Transport;
#[cfg(feature = "mock")]
pub use transport::{Mismatch, MockTime, MockTransport};
pub use units::{
    ComplianceMargin, ComplianceSlope, Current, Gain, HardwareErrors, Load, Position,
    PositionScale, RegisterScale, Sign, SpeedLimit, Temperature, Tick, Velocity, Voltage,
};
