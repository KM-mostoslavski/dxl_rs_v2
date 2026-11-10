//! Operating mode writes and reads (plan 2026-11-10, S3): the checks a mode
//! write passes before a byte is sent, its packet, the cache that follows
//! it, and the read back.

use crate::Transport;
use crate::actuator::{
    Actuator, Describe, LimitMode, ModeSource, ModeValues, OperatingMode, TorqueState,
};
use crate::error::{Error, ValidationError};
use crate::models::ActuatorKind;
use crate::protocol::{Clock, Protocol};
use crate::register::Register;

use super::actuator_ref::ActuatorRef;
use super::diagnose::diagnosed;
use super::limits::{bound_register, check_off};
use super::observe::{observe, reached};
use super::slots::Marks;

/// The register name a model without a mode register is refused with.
const OPERATING_MODE: &str = "Operating Mode";

/// One mode write, checked: the register, its value, and the limit mode
/// Setup would record for that value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ModeWrite {
    /// The mode register.
    register: Register,
    /// The value of the mode asked for, from the model's lists.
    value: u8,
    /// The limit mode of that value.
    mode: LimitMode,
}

/// The mode register of `kind` and what its values mean.
///
/// # Errors
///
/// `Unsupported`, naming Operating Mode, on the models whose mode follows
/// their angle limits (AX, RX, MX on Protocol 1).
fn mode_register(kind: &ActuatorKind) -> Result<(Register, ModeValues), ValidationError> {
    match kind.data().mode {
        ModeSource::Register { register, values } => Ok((register, values)),
        // DECISION: the MX-64 on Protocol 1 (angle limits overridden by its Torque Ctrl Mode Enable switch) is an angle-limit model too: it has no Operating Mode, and its switch is a RAM register that changes the mode Setup recorded, left out of S3.
        ModeSource::AngleLimits | ModeSource::TorqueControl { .. } => {
            Err(ValidationError::Unsupported {
                id: kind.id(),
                register: OPERATING_MODE,
            })
        }
    }
}

/// A write of named mode `mode` to `kind`, whose torque state is `state`
/// and marks `marks`, checked as every setting is (plan 2026-11-09 D4,
/// D8): the register, the torque, then the value.
///
/// # Errors
///
/// `Unsupported`, `ActuatorIsOn`, `TorqueUnconfirmed`, `UnsupportedMode`.
pub(super) fn check_mode(
    kind: &ActuatorKind,
    (state, marks): (TorqueState, Marks),
    mode: OperatingMode,
) -> Result<ModeWrite, ValidationError> {
    let (register, values) = mode_register(kind)?;
    check_off(kind.id(), state, marks, register.name)?;
    // DECISION: a mode the table does not single out is refused after the torque check, as a setting whose value does not convert (NoUnit) is: the order of plan 2026-11-09 D4.
    let (value, limit) = values
        .value_of(mode)
        .ok_or(ValidationError::UnsupportedMode {
            id: kind.id(),
            mode,
        })?;
    Ok(ModeWrite {
        register,
        value,
        mode: limit,
    })
}

impl<P: Protocol, T: Transport, C: Clock> ActuatorRef<'_, P, T, C> {
    /// Sends checked mode write `write`: one acknowledged Write. The cache
    /// then follows it (plan 2026-11-10 S3): acknowledged, the new limit
    /// mode is recorded and known, and where it bounds its goal with a
    /// register the table has, one Read of that register caches the bound;
    /// failed after bytes went out, the mode is unknown.
    ///
    /// # Errors
    ///
    /// Any error of the write, or of the bound read, whose bound is then
    /// unknown.
    pub(super) fn send_mode(&mut self, write: ModeWrite) -> Result<(), Error> {
        let result = self.exchange(write.register, &[write.value]);
        if result.is_err() {
            // DECISION: every error of the exchange that may have reached the actuator leaves the mode unknown (plan 2026-11-09 D5), also a reply carrying only a hardware fault, after which the instruction ran: a faulted X-series answers every request with the alert bit until a reboot, so its bound could not be read either.
            if reached(&result) {
                self.kind.cache_mut().forget_mode();
                *self.bound = None;
            }
            return result;
        }
        self.kind.cache_mut().know_mode(write.mode);
        self.kind.cache_mut().know_bound();
        *self.bound = None;
        let Some(reg) = bound_register(self.kind) else {
            return Ok(());
        };
        // DECISION: the bound read is a runtime read like every other, so a reply carrying a hardware fault fails it (bound unknown) where Setup's read would tolerate it.
        match self.read_reg(reg) {
            Ok(raw) => {
                *self.bound = Some(raw.cast_unsigned());
                Ok(())
            }
            Err(err) => {
                self.kind.cache_mut().forget_bound();
                Err(err)
            }
        }
    }

    /// Reads the mode register once and maps its value back to the named
    /// mode. Nothing is cached.
    ///
    /// # Errors
    ///
    /// `Unsupported` with nothing sent on the models whose mode follows
    /// their angle limits; any error of the read; `UnlistedMode` for a
    /// value no named mode takes.
    pub(super) fn read_mode(&mut self) -> Result<OperatingMode, Error> {
        let id = self.kind.id();
        let (register, values) = mode_register(self.kind)?;
        let mut value = [0];
        let result = self.wire.read(id, register, &mut value);
        let result = diagnosed(&mut self.wire, self.kind, result);
        observe(self.state, &result);
        result?;
        let [value] = value;
        values.mode_of(value).ok_or_else(|| {
            ValidationError::UnlistedMode {
                id,
                register: register.name,
                value,
            }
            .into()
        })
    }
}
