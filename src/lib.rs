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
pub use error::{Error, ValidationError};
pub use id::Id;
pub use models::ActuatorKind;
#[cfg(feature = "std")]
pub use transport::SerialTransport;
pub use transport::Transport;
#[cfg(feature = "mock")]
pub use transport::{Mismatch, MockTime, MockTransport};
