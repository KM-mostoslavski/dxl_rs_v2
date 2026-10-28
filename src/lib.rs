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
mod error;
mod id;
pub mod models;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "exchanges are called by the bus module")
)]
mod protocol;
mod register;
mod transport;

pub use actuator::Actuator;
/// The `embedded-io` crate this one builds on, so users can implement and
/// call [`Transport`] without a version-matched dependency of their own.
pub use embedded_io;
pub use error::{
    Error, P1Flag, P1Flags, P2Code, ProtocolError, StatusError, StatusReport, TransportError,
    ValidationError,
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
