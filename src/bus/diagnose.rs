//! Measures a hardware fault a reply reported (audit E13): the error byte
//! says only that a fault exists, so the bus reads what it is and how far
//! off its limit, and returns a [`FaultError`] instead of the bare
//! [`StatusError`].
//!
//! Decided 2026-11-04: the measurement runs automatically, only on the
//! fault path (no traffic otherwise). Reads during it tolerate the fault
//! bit the actuator keeps setting in every reply, and use the data the
//! reply carries.

use crate::actuator::{Actuator, Describe, Role, RoleRegister};
use crate::error::{
    Celsius, Decivolts, Error, FaultError, Request, StatusError, StatusReport, Step,
    TemperatureError, VoltageError,
};
use crate::models::{self, ActuatorKind};
use crate::protocol::{Clock, Protocol};
use crate::register::Register;
use crate::units::HardwareErrors;
use crate::{Id, ModelNumber, Transport};

use super::wire::Wire;

/// `result` with a hardware-fault [`StatusError`] replaced by its measured
/// [`FaultError`]; any other result as it is.
pub(crate) fn diagnosed<V, P: Protocol, T: Transport, C: Clock>(
    wire: &mut Wire<'_, P, T, C>,
    kind: &ActuatorKind,
    result: Result<V, Error>,
) -> Result<V, Error> {
    result.map_err(|err| diagnose(wire, kind, err))
}

/// `err` measured into a [`FaultError`] when it is a hardware-fault
/// [`StatusError`] of `kind`; any other error as it is.
///
/// If a measurement read fails, `err` is returned as it is: the fault is
/// never lost (the actuator still counts as faulted), only left unmeasured.
pub(crate) fn diagnose<P: Protocol, T: Transport, C: Clock>(
    wire: &mut Wire<'_, P, T, C>,
    kind: &ActuatorKind,
    err: Error,
) -> Error {
    match err {
        Error::Status(status) if status.is_hardware_fault() && status.id == kind.id() => {
            let saved = wire.context();
            wire.set_context(Request::NONE.step(Step::FaultDiagnosis));
            let measured = measure(wire, kind, status);
            wire.set_context(saved);
            measured.unwrap_or(err)
        }
        other => other,
    }
}

/// As [`diagnose`], for an actuator Setup has not built yet (a scan reply
/// reporting a fault): its model is read first. If the model is unknown to
/// this build, `err` is returned as it is.
pub(crate) fn diagnose_unknown<P: Protocol, T: Transport, C: Clock>(
    wire: &mut Wire<'_, P, T, C>,
    id: Id,
    model: Option<u16>,
    err: Error,
) -> Error {
    if !err.is_hardware_fault() {
        return err;
    }
    let saved = wire.context();
    wire.set_context(Request::NONE.step(Step::FaultDiagnosis));
    let model = match model {
        Some(model) => Ok(model),
        None => read_tolerant(wire, id, models::MODEL_NUMBER)
            .map(|raw| u16::try_from(raw).unwrap_or(u16::MAX)),
    };
    wire.set_context(saved);
    match model {
        Ok(model) => match ActuatorKind::new(P::VERSION, ModelNumber(model), 0, id) {
            Ok(kind) => diagnose(wire, &kind, err),
            Err(_) => err,
        },
        Err(_) => err,
    }
}

/// Reads `reg` of `id`, accepting a reply that reports a hardware fault:
/// its data is the register's value.
fn read_tolerant<P: Protocol, T: Transport, C: Clock>(
    wire: &mut Wire<'_, P, T, C>,
    id: Id,
    reg: Register,
) -> Result<i32, Error> {
    let mut buf = [0; 4];
    let bytes = reg.buf(&mut buf)?;
    match wire.read(id, reg, bytes) {
        Err(err) if !err.is_hardware_fault() => return Err(err),
        _ => {}
    }
    Ok(reg.decode(bytes)?)
}

/// The fault `status` reports, measured.
fn measure<P: Protocol, T: Transport, C: Clock>(
    wire: &mut Wire<'_, P, T, C>,
    kind: &ActuatorKind,
    status: StatusError,
) -> Result<Error, Error> {
    let StatusError { id, request, .. } = status;
    let data = kind.data();
    let faults = match status.report {
        StatusReport::P1(flags) => flags.hardware(),
        StatusReport::P2 { .. } => match data.hardware_error {
            Some(reg) => HardwareErrors::from_raw(
                u8::try_from(read_tolerant(wire, id, reg)?).unwrap_or(u8::MAX),
            ),
            None => {
                return Ok(FaultError::Unmeasurable {
                    id,
                    faults: HardwareErrors::default(),
                    register: "Hardware Error Status",
                    request,
                }
                .into());
            }
        },
    };
    let unmeasurable = |register| {
        Ok(FaultError::Unmeasurable {
            id,
            faults,
            register,
            request,
        }
        .into())
    };
    let role = |role: Role| data.role(role).copied().ok_or(role.name());
    let fault = if faults.input_voltage() {
        let registers = (
            role(Role::PresentVoltage),
            role(Role::MinVoltageLimit),
            role(Role::MaxVoltageLimit),
        );
        let (present, min, max) = match registers {
            (Ok(present), Ok(min), Ok(max)) => (present, min, max),
            (Err(name), _, _) | (_, Err(name), _) | (_, _, Err(name)) => {
                return unmeasurable(name);
            }
        };
        let mut volts = |reg: RoleRegister| {
            read_tolerant(wire, id, reg.register()).map(|raw| decivolts(kind, &reg, raw))
        };
        let measured = volts(present)?;
        let (min, max) = (volts(min)?, volts(max)?);
        FaultError::Voltage(VoltageError::classify(
            id, measured, min, max, faults, request,
        ))
    } else if faults.overheating() {
        let (present, limit) = match (role(Role::PresentTemperature), role(Role::TemperatureLimit))
        {
            (Ok(present), Ok(limit)) => (present, limit),
            (Err(name), _) | (_, Err(name)) => return unmeasurable(name),
        };
        let mut degrees = |reg: RoleRegister| {
            read_tolerant(wire, id, reg.register()).map(|raw| celsius(kind, &reg, raw))
        };
        let measured = degrees(present)?;
        let limit = degrees(limit)?;
        FaultError::Temperature(TemperatureError::classify(
            id, measured, limit, faults, request,
        ))
    } else if faults.overload() {
        FaultError::Overload {
            id,
            faults,
            request,
        }
    } else if faults.electrical_shock() {
        FaultError::ElectricalShock {
            id,
            faults,
            request,
        }
    } else if faults.encoder() {
        FaultError::Encoder {
            id,
            faults,
            request,
        }
    } else {
        FaultError::Unidentified {
            id,
            faults,
            request,
        }
    };
    Ok(fault.into())
}

/// `raw` of voltage register `reg` in tenths of a volt, through its scale
/// (0.1 V per raw step in every table today).
fn decivolts(kind: &ActuatorKind, reg: &RoleRegister, raw: i32) -> Decivolts {
    let tenths = scaled(kind, reg, raw, 0.1) * 10.0;
    Decivolts(if tenths <= 0.0 {
        0
    } else {
        (tenths + 0.5) as u16
    })
}

/// `raw` of temperature register `reg` in whole degrees Celsius, through
/// its scale (1 °C per raw step in every table today).
fn celsius(kind: &ActuatorKind, reg: &RoleRegister, raw: i32) -> Celsius {
    let degrees = scaled(kind, reg, raw, 1.0);
    Celsius(if degrees < 0.0 {
        (degrees - 0.5) as i16
    } else {
        (degrees + 0.5) as i16
    })
}

/// `raw` in SI units through `reg`'s scale in the recorded mode, or
/// `fallback` per raw step without one.
fn scaled(kind: &ActuatorKind, reg: &RoleRegister, raw: i32, fallback: f32) -> f32 {
    let per_raw = reg
        .scale(kind.cache().mode())
        .map_or(fallback, |scale| scale.si_per_raw());
    raw as f32 * per_raw
}
