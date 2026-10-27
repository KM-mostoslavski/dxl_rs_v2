//! `no_std`, heap-free HAL for Dynamixel actuators (Protocol 1 & 2).
//!
//! The design lives in `docs/ARCHITECTURE.md`.
#![no_std]
#![warn(clippy::mod_module_files)]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod actuator;
mod error;
mod id;
pub mod models;
mod register;
mod transport;

pub use actuator::Actuator;
/// The `embedded-io` crate this one builds on, so users can implement and
/// call [`Transport`] without a version-matched dependency of their own.
pub use embedded_io;
pub use error::{Error, ValidationError};
pub use id::Id;
pub use models::ActuatorKind;
/// The `serialport` crate behind [`SerialTransport`], for its port and error
/// types.
#[cfg(feature = "std")]
pub use serialport;
#[cfg(feature = "std")]
pub use transport::SerialTransport;
pub use transport::Transport;
#[cfg(feature = "mock")]
pub use transport::{Mismatch, MockTime, MockTransport};
