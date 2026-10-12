//! `no_std`, heap-free HAL for Dynamixel actuators (Protocol 1 & 2).
//!
//! The design lives in `docs/ARCHITECTURE.md`.
#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod error;
mod id;

pub use error::{Error, ValidationError};
pub use id::Id;
