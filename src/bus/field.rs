//! Fields: one register of every model, with the type its values have
//! (research R-15, SRS FR-9.5).
//!
//! A field names a register by what it holds, whatever its name and address
//! in a given model: [`PresentVelocity`] is Present Speed(38) on an AX-12A
//! and Present Velocity(128) on an XM430-W350. The same fields serve every
//! tier:
//!
//! - one actuator: [`ActuatorRef::read`](crate::ActuatorRef::read) and
//!   [`ActuatorRef::write`](crate::ActuatorRef::write), and for the EEPROM
//!   limits ([`LimitField`]) [`ActuatorRef::write_limit`](crate::ActuatorRef::write_limit);
//! - many actuators: [`Bus::read_field`](crate::Bus::read_field) (both
//!   protocols), [`Bus::sync_write`](crate::Bus::sync_write) (both
//!   protocols) and [`Bus::sync_read`](crate::Bus::sync_read) (Protocol 2);
//! - several registers in one packet: [`Bus::bulk_read`](crate::Bus::bulk_read)
//!   and [`Bus::bulk_write`](crate::Bus::bulk_write).
//!
//! The field is passed as a value, so no turbofish is needed, and
//! [`Raw`] gives the raw register value of any field (FR-3.6):
//!
//! ```
//! use dxl_rs_v2::field::{GoalPosition, PresentTemperature, Raw, ReadField, WriteField};
//! use dxl_rs_v2::{Position, Temperature};
//!
//! fn value_of<F: WriteField>(_: F, value: <F as WriteField>::Value) -> <F as WriteField>::Value {
//!     value
//! }
//! let typed: Position = value_of(GoalPosition, Position::ZERO);
//! let raw: i32 = value_of(Raw(GoalPosition), 2048);
//! fn read_type<F: ReadField>(_: F) -> &'static str {
//!     core::any::type_name::<F::Value>()
//! }
//! assert!(read_type(PresentTemperature).ends_with("Temperature"));
//! # let _ = (typed, raw);
//! ```
//!
//! What each field checks before anything is sent, on every tier:
//!
//! | Field | Value | Torque on? | Mode | Bound |
//! |---|---|---|---|---|
//! | [`GoalPosition`] | [`Position`] | yes | position modes | EEPROM position limits in position mode |
//! | [`GoalVelocity`] | [`Velocity`] | yes | velocity, wheel | Velocity Limit, read at Setup |
//! | [`GoalCurrent`] | [`Current`] | yes | current, current-based position | Current Limit, read at Setup |
//! | [`SpeedLimit`] | [`units::SpeedLimit`](crate::SpeedLimit) | no | not where it is the goal velocity | e-Manual range |
//! | [`Led`] | `bool` | no | any | — |
//! | [`PGain`], [`DGain`] | [`Gain`] | no | any | e-Manual range |
//! | compliance fields | [`ComplianceMargin`], [`ComplianceSlope`] | no | any | e-Manual range |
//! | [`LedRed`], [`LedGreen`], [`LedBlue`] | `u8` | no | any | — |
//! | [`MinVoltageLimit`], [`MaxVoltageLimit`] | [`Voltage`] | confirmed off | any | e-Manual range; a pair, min ≤ max |
//! | [`TemperatureLimit`] | [`Temperature`] | confirmed off | any | e-Manual range |
//! | [`MinPositionLimit`], [`MaxPositionLimit`] | [`Position`] | confirmed off | any | e-Manual range of the limit row (verified tables only); min ≤ max |
//! | [`VelocityLimit`] | [`Velocity`] | confirmed off | any | e-Manual range |
//! | [`CurrentLimit`] | [`Current`] | confirmed off | any | e-Manual range |
//! | [`PwmLimit`] | [`Pwm`] | confirmed off | any | e-Manual range |
//! | [`MaxTorque`] | [`Load`] | confirmed off | any | e-Manual range |
//! | [`AccelerationLimit`] | [`Acceleration`] | confirmed off | any | e-Manual range |
//!
//! Every value is also checked against the register size. "e-Manual
//! range" is the range the table records for the register: a register
//! whose table records none gets only the size check (e.g. MX-64 on
//! Protocol 1 Min/Max Voltage Limit, whose e-Manual range contradicts its
//! own initial value). A model whose table lacks the register refuses the
//! call with [`ValidationError::Unsupported`]; a typed value in a mode
//! where the table gives the register no unit refuses it with
//! [`ValidationError::NoUnit`] (the raw form still works). A typed position
//! limit needs no unit but the e-Manual range of its row: on a table that
//! records none (an unverified table) it is refused with `NoUnit` too.
//!
//! The limit fields are [`LimitField`]s, written only by
//! [`ActuatorRef::write_limit`](crate::ActuatorRef::write_limit) and its
//! setters, never by a sync or bulk write, so the bus keeps the position
//! limits, the limit mode and the goal bound it holds true to each write.
//! The bus writes EEPROM only with torque confirmed off, on every model: a
//! limit write while the torque state it holds is on is refused with
//! [`ValidationError::ActuatorIsOn`], and in any other state until an
//! acknowledged `disable_torque()` (whose reply shows the write ran, a
//! hardware fault flag included) with [`ValidationError::TorqueUnconfirmed`]:
//! after Setup, after an `enable_torque()` that sent bytes, a torque write
//! without such a reply, a reboot or a raw write. X-series, MX(2.0) and
//! XL-320 firmware also writes EEPROM only with Torque Enable 0
//! (XM430-W350: "Data in the EEPROM Area can only be written to if Torque
//! Enable(64) is cleared to '0'",
//! <https://emanual.robotis.com/docs/en/dxl/x/xm430-w350/>, 2026-11-09);
//! the AX, RX and MX Protocol 1 pages document no such lock, so there the
//! bus check is the only guard. Every read tier reads them.
//!
//! Torque Enable is not a field: a sync write gets no reply, so the bus
//! could not keep its torque state true to the hardware. Use
//! [`Bus::enable_torque`](crate::Bus::enable_torque) and
//! [`Bus::disable_torque`](crate::Bus::disable_torque). Nor are ID, baud
//! rate and the registers of one series (Position Limit Threshold, Y;
//! ...): those stay raw (`raw-registers`).

use crate::Id;
use crate::actuator::{Actuator, Cache, Describe, LimitMode, Role};
use crate::error::{Error, Instruction, ValidationError};
use crate::models::ActuatorKind;
use crate::register::{Access, Register};
use crate::units::{
    Acceleration, ComplianceMargin, ComplianceSlope, Current, Gain, HardwareErrors, Load, Position,
    PositionScale, Pwm, RegisterScale, Temperature, Tick, Velocity, Voltage,
};

pub(crate) mod sealed {
    use super::{
        ActuatorKind, Cache, Id, LimitMode, PositionScale, Register, RegisterScale, ValidationError,
    };

    /// Where a field's value lives on one actuator, and how it converts.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Site {
        /// The actuator.
        pub(crate) id: Id,
        /// The register.
        pub(crate) reg: Register,
        /// The register's name for errors, e.g. `"Present Temperature"`.
        pub(crate) name: &'static str,
        /// The limit mode Setup recorded.
        pub(crate) mode: LimitMode,
        /// The model's position scale.
        pub(crate) position: PositionScale,
        /// The register's scale in `mode`; `None`: raw only (FR-9.2).
        pub(crate) scale: Option<RegisterScale>,
        /// The raw range the e-Manual gives for the register in `mode`.
        pub(crate) range: Option<(i64, i64)>,
    }

    impl Site {
        /// A call of `instruction` on this site, for errors.
        pub(crate) const fn request(
            &self,
            instruction: crate::error::Instruction,
        ) -> crate::error::Request {
            self.reg
                .request()
                .register(self.name)
                .instruction(instruction)
                .id(self.id)
        }
    }

    /// A register every model may have. Only this crate implements it.
    pub trait Field {
        /// Whether the register is the actuator's own fault report
        /// (Hardware Error Status): a single read of it whose reply has
        /// only the Protocol 2 alert bit set returns the non-zero value the
        /// reply carries, after marking the actuator `Faulted`.
        const FAULT_REPORT: bool = false;

        /// The field's site on `kind`.
        ///
        /// # Errors
        ///
        /// [`ValidationError::Unsupported`] when the model's table lacks
        /// the register.
        fn site(kind: &ActuatorKind) -> Result<Site, ValidationError>;
    }

    /// What a write of a field must pass besides its value's conversion.
    /// Only this crate implements it.
    pub trait Rule: Field {
        /// Whether the write moves the actuator, so torque must be on
        /// (FR-3.4, FR-9.3).
        const MOVES: bool;

        /// The operating-mode check, before the value is converted.
        ///
        /// # Errors
        ///
        /// The mode error of the field, e.g. `NotVelocityMode`.
        fn check_mode(site: &Site, kind: &ActuatorKind) -> Result<(), ValidationError>;

        /// The limit check of register value `raw`, with `bound` the EEPROM
        /// limit Setup read for the goal the mode uses. Returns the value to
        /// send.
        ///
        /// # Errors
        ///
        /// `OutOfLimits` or the field's own refusal.
        fn check_value(
            site: &Site,
            cache: &Cache,
            bound: Option<u32>,
            raw: i32,
        ) -> Result<i32, ValidationError>;
    }

    /// A value type a field reads into. Only this crate implements it.
    pub trait Decode: Copy {
        /// Checks that `site` gives the value what it needs (a unit),
        /// before anything is sent.
        ///
        /// # Errors
        ///
        /// [`ValidationError::NoUnit`] for a typed value without a scale.
        fn check_site(site: &Site) -> Result<(), ValidationError>;

        /// The value of register value `raw`.
        ///
        /// # Errors
        ///
        /// As [`check_site`](Decode::check_site).
        fn from_raw(raw: i32, site: &Site) -> Result<Self, ValidationError>;
    }

    /// A value type a field writes from. Only this crate implements it.
    pub trait Encode: Copy {
        /// Whether a limit write of this value needs the e-Manual range the
        /// table records for the register: a position converts through
        /// `type_info`, which no `unit_info` row backs, so the recorded
        /// range is what tells a verified table (plan 2026-11-09 D6).
        const NEEDS_RANGE: bool = false;

        /// The register value for `self`.
        ///
        /// # Errors
        ///
        /// [`ValidationError::NoUnit`], [`ValidationError::NotFinite`] or
        /// [`ValidationError::DoesNotFit`] for a value that maps to no
        /// register value.
        fn to_raw(self, site: &Site) -> Result<i32, ValidationError>;
    }
}

use sealed::{Decode, Encode, Field, Rule, Site};

/// A field the bus can read.
pub trait ReadField: Field {
    /// The type of one actuator's value.
    type Value: Copy + Decode;
}

/// A field the bus can write.
pub trait WriteField: Field + Rule {
    /// The type of one actuator's value.
    type Value: Copy + Encode;
}

/// An EEPROM limit field: written only by
/// [`ActuatorRef::write_limit`](crate::ActuatorRef::write_limit), one
/// acknowledged unicast write with torque confirmed off.
///
/// The limit fields are not [`WriteField`]s, so
/// [`Bus::sync_write`](crate::Bus::sync_write),
/// [`Bus::bulk_write`](crate::Bus::bulk_write) and
/// [`ActuatorRef::write`](crate::ActuatorRef::write) refuse them at compile
/// time: a broadcast write gets no reply, so the bus could neither confirm
/// it nor keep the limits it holds true to the actuator.
///
/// ```compile_fail,E0277
/// use dxl_rs_v2::field::MaxVoltageLimit;
/// use dxl_rs_v2::{Bus, Clock, Error, Id, P2, Transport, Voltage};
///
/// fn widen<T: Transport, C: Clock>(bus: &mut Bus<P2, T, C>, ids: [Id; 2]) -> Result<(), Error> {
///     bus.sync_write(MaxVoltageLimit, ids, &[Voltage::from_volts(16.0); 2])
/// }
/// ```
///
/// They are [`ReadField`]s, so every read tier saves them, e.g. before a
/// run that changes them (NFR-11). [`Raw`] of a limit field writes its raw
/// register value, with the same checks.
pub trait LimitField: Field {
    /// The type of one actuator's value.
    type Value: Copy + Encode;
}

/// The raw register value (`i32`) of field `F`, e.g. `Raw(GoalPosition)`.
///
/// Sizes 1 and 2 read unsigned, size 4 as an `i32`, as the register holds
/// them: a negative Goal Current of an XM430-W350 reads as its 16-bit two's
/// complement, and a CW wheel speed of an AX-12A has bit 10 set. A raw
/// write passes the same torque, mode and limit checks as the typed one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Raw<F>(pub F);

impl<F: Field> Field for Raw<F> {
    const FAULT_REPORT: bool = F::FAULT_REPORT;

    fn site(kind: &ActuatorKind) -> Result<Site, ValidationError> {
        F::site(kind)
    }
}

impl<F: Rule> Rule for Raw<F> {
    const MOVES: bool = F::MOVES;

    fn check_mode(site: &Site, kind: &ActuatorKind) -> Result<(), ValidationError> {
        F::check_mode(site, kind)
    }

    fn check_value(
        site: &Site,
        cache: &Cache,
        bound: Option<u32>,
        raw: i32,
    ) -> Result<i32, ValidationError> {
        F::check_value(site, cache, bound, raw)
    }
}

impl<F: ReadField> ReadField for Raw<F> {
    type Value = i32;
}

impl<F: WriteField> WriteField for Raw<F> {
    type Value = i32;
}

impl<F: LimitField> LimitField for Raw<F> {
    type Value = i32;
}

/// The site of `reg`, named `name`, on `kind`, with no register scale.
fn plain_site(kind: &ActuatorKind, reg: Register, name: &'static str) -> Site {
    let data = kind.data();
    Site {
        id: kind.id(),
        reg,
        name,
        mode: kind.cache().mode(),
        position: data.scale,
        scale: None,
        range: None,
    }
}

/// The site of `role` on `kind`, with its scale and range in the recorded
/// mode. A register the table has only from a later firmware than the one
/// Setup recorded is missing too.
fn role_site(kind: &ActuatorKind, role: Role) -> Result<Site, ValidationError> {
    let id = kind.id();
    let found = kind
        .data()
        .role(role)
        .filter(|found| found.from_firmware() <= kind.cache().firmware())
        .ok_or(ValidationError::Unsupported {
            id,
            register: role.name(),
        })?;
    let mode = kind.cache().mode();
    Ok(Site {
        scale: found.scale(mode),
        range: found
            .raw_range(mode)
            .map(|range| (*range.start(), *range.end())),
        ..plain_site(kind, found.register(), found.name())
    })
}

/// The range check every non-goal write passes: the register value must lie
/// in the e-Manual range, where the table records one.
fn in_range(site: &Site, raw: i32) -> Result<i32, ValidationError> {
    match site.range {
        Some((min, max)) if i64::from(raw) < min || i64::from(raw) > max => {
            Err(ValidationError::OutOfLimits {
                id: site.id,
                raw,
                min: saturate(min),
                max: saturate(max),
            })
        }
        _ => Ok(raw),
    }
}

/// `v` as an `i32`, saturated, for an error message.
fn saturate(v: i64) -> i32 {
    i32::try_from(v).unwrap_or(if v < 0 { i32::MIN } else { i32::MAX })
}

/// The bound check of a goal velocity or current: the signed value's
/// magnitude must not pass `bound`, the EEPROM limit read at Setup.
fn within_bound(site: &Site, bound: Option<u32>, raw: i32) -> Result<i32, ValidationError> {
    let Some(bound) = bound else {
        return in_range(site, raw);
    };
    let signed = site
        .scale
        .map_or(i64::from(raw), |scale| scale.signed_of(raw.cast_unsigned()));
    let limit = i64::from(bound);
    if signed.abs() > limit {
        Err(ValidationError::OutOfLimits {
            id: site.id,
            raw: saturate(signed),
            min: saturate(-limit),
            max: saturate(limit),
        })
    } else {
        in_range(site, raw)
    }
}

/// Declares a field backed by an FR-9 role.
macro_rules! role_field {
    ($(#[$doc:meta])* $name:ident, $role:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
        pub struct $name;

        impl Field for $name {
            fn site(kind: &ActuatorKind) -> Result<Site, ValidationError> {
                role_site(kind, Role::$role)
            }
        }
    };
}

/// Declares the write rule of a field that does not move the actuator:
/// allowed with torque off, in any mode, within the e-Manual range.
macro_rules! settings_rule {
    ($name:ident) => {
        impl Rule for $name {
            const MOVES: bool = false;

            fn check_mode(_: &Site, _: &ActuatorKind) -> Result<(), ValidationError> {
                Ok(())
            }

            fn check_value(
                site: &Site,
                _: &Cache,
                _: Option<u32>,
                raw: i32,
            ) -> Result<i32, ValidationError> {
                in_range(site, raw)
            }
        }
    };
}

/// Goal Position, in radians ([`Position`]). Readable (the goal sent last)
/// and writable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GoalPosition;

/// Present Position, in radians ([`Position`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PresentPosition;

impl Field for GoalPosition {
    fn site(kind: &ActuatorKind) -> Result<Site, ValidationError> {
        Ok(plain_site(kind, kind.data().goal_position, "Goal Position"))
    }
}

impl Field for PresentPosition {
    fn site(kind: &ActuatorKind) -> Result<Site, ValidationError> {
        Ok(plain_site(
            kind,
            kind.data().present_position,
            "Present Position",
        ))
    }
}

impl Rule for GoalPosition {
    const MOVES: bool = true;

    fn check_mode(_: &Site, _: &ActuatorKind) -> Result<(), ValidationError> {
        Ok(())
    }

    fn check_value(
        _: &Site,
        cache: &Cache,
        _: Option<u32>,
        raw: i32,
    ) -> Result<i32, ValidationError> {
        cache.check_goal(raw)
    }
}

impl ReadField for GoalPosition {
    type Value = Position;
}

impl WriteField for GoalPosition {
    type Value = Position;
}

impl ReadField for PresentPosition {
    type Value = Position;
}

role_field!(
    /// Present Temperature, in °C ([`Temperature`]). Every model.
    PresentTemperature,
    PresentTemperature
);
role_field!(
    /// Present Voltage, or Present Input Voltage, in volts ([`Voltage`]).
    PresentVoltage,
    PresentVoltage
);
role_field!(
    /// Present Speed (Protocol 1) or Present Velocity, in rad/s
    /// ([`Velocity`]). AX and RX in wheel mode give it in % of output, so
    /// only its raw form is read there.
    PresentVelocity,
    PresentVelocity
);
role_field!(
    /// Present Load, a ratio of the maximum torque from −1 to 1 ([`Load`]).
    /// AX, RX, MX on Protocol 1.
    PresentLoad,
    PresentLoad
);
role_field!(
    /// Present Current, in amperes ([`Current`]). MX(2.0), X, Y, PRO.
    PresentCurrent,
    PresentCurrent
);
role_field!(
    /// Goal Velocity, or Moving Speed in wheel mode (Protocol 1), in rad/s
    /// ([`Velocity`]). Written only in velocity or wheel mode, with torque
    /// on, and within the Velocity Limit read at Setup.
    ///
    /// AX and RX wheel-mode Moving Speed is a share of output (about 0.1 %
    /// per step, bit 10 the direction), not a velocity: only its raw form
    /// is written there, within the e-Manual's 0 to 2047.
    GoalVelocity,
    GoalVelocity
);
role_field!(
    /// A speed limit: Profile Velocity, or Moving Speed in joint and
    /// multi-turn mode (Protocol 1), as a
    /// [`units::SpeedLimit`](crate::SpeedLimit). Allowed with torque off.
    ///
    /// On Protocol 1 in wheel mode Moving Speed is the goal velocity, so
    /// the write is refused there with `NotPositionMode`: use
    /// [`GoalVelocity`].
    SpeedLimit,
    SpeedLimit
);
role_field!(
    /// Goal Current, or Goal Torque (MX on Protocol 1), in amperes
    /// ([`Current`]). Written only in current or current-based position
    /// mode, with torque on, and within the Current Limit read at Setup.
    GoalCurrent,
    GoalCurrent
);
role_field!(
    /// Moving: whether the actuator is moving (`bool`). Read only.
    Moving,
    Moving
);
role_field!(
    /// LED, on or off (`bool`). Allowed with torque off. PRO and RH-P12-RN
    /// have [`LedRed`], [`LedGreen`] and [`LedBlue`] instead.
    Led,
    Led
);
role_field!(
    /// LED Red brightness (`u8`), PRO and RH-P12-RN. Allowed with torque
    /// off.
    LedRed,
    LedRed
);
role_field!(
    /// LED Green brightness (`u8`), PRO and RH-P12-RN. Allowed with torque
    /// off.
    LedGreen,
    LedGreen
);
role_field!(
    /// LED Blue brightness (`u8`), PRO and RH-P12-RN. Allowed with torque
    /// off.
    LedBlue,
    LedBlue
);
/// Hardware Error Status, a set of fault flags ([`HardwareErrors`]).
/// MX(2.0), X, Y, PRO. Read only.
///
/// An actuator with a fault answers every instruction with the alert bit
/// set. A single read of this field
/// ([`ActuatorRef::hardware_errors`](crate::ActuatorRef::hardware_errors))
/// still returns the flags that reply carries, after marking the actuator
/// `Faulted`, so the caller learns which fault it is. The multi-actuator
/// reads stay strict: there the reply is the `Status` error of that target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HardwareErrorStatus;

impl Field for HardwareErrorStatus {
    const FAULT_REPORT: bool = true;

    fn site(kind: &ActuatorKind) -> Result<Site, ValidationError> {
        role_site(kind, Role::HardwareErrorStatus)
    }
}
role_field!(
    /// Realtime Tick, the actuator's wrapping millisecond clock ([`Tick`]).
    /// Read only.
    RealtimeTick,
    RealtimeTick
);
role_field!(
    /// Position P Gain, or P Gain (MX on Protocol 1), in register units
    /// ([`Gain`]). Allowed with torque off.
    PGain,
    PGain
);
role_field!(
    /// Position D Gain, or D Gain (MX on Protocol 1), in register units
    /// ([`Gain`]). Allowed with torque off.
    DGain,
    DGain
);
role_field!(
    /// CW Compliance Margin, AX and RX ([`ComplianceMargin`]). Allowed with
    /// torque off.
    CwComplianceMargin,
    CwComplianceMargin
);
role_field!(
    /// CCW Compliance Margin, AX and RX ([`ComplianceMargin`]). Allowed with
    /// torque off.
    CcwComplianceMargin,
    CcwComplianceMargin
);
role_field!(
    /// CW Compliance Slope, AX and RX ([`ComplianceSlope`]). Allowed with
    /// torque off.
    CwComplianceSlope,
    CwComplianceSlope
);
role_field!(
    /// CCW Compliance Slope, AX and RX ([`ComplianceSlope`]). Allowed with
    /// torque off.
    CcwComplianceSlope,
    CcwComplianceSlope
);
role_field!(
    /// Torque Ctrl Mode Enable, MX-64 on Protocol 1 (`bool`). Read only:
    /// writing it changes the operating mode, which Setup records once.
    TorqueCtrlModeEnable,
    TorqueCtrlModeEnable
);
role_field!(
    /// Min Voltage Limit, in volts ([`Voltage`]): the EEPROM bound below
    /// which the actuator reports an input voltage fault. Every model. A
    /// [`LimitField`].
    MinVoltageLimit,
    MinVoltageLimit
);
role_field!(
    /// Max Voltage Limit, in volts ([`Voltage`]): the EEPROM bound above
    /// which the actuator reports an input voltage fault. Every model. A
    /// [`LimitField`].
    MaxVoltageLimit,
    MaxVoltageLimit
);
role_field!(
    /// Temperature Limit, in °C ([`Temperature`]): the EEPROM bound above
    /// which the actuator reports overheating. A [`LimitField`].
    TemperatureLimit,
    TemperatureLimit
);
role_field!(
    /// Min Position Limit, or CW Angle Limit (AX, RX, MX on Protocol 1,
    /// XL-320), in radians ([`Position`]): the lower EEPROM position limit,
    /// which goals are checked against in position mode. Every model. A
    /// [`LimitField`].
    MinPositionLimit,
    MinPositionLimit
);
role_field!(
    /// Max Position Limit, or CCW Angle Limit (AX, RX, MX on Protocol 1,
    /// XL-320), in radians ([`Position`]): the upper EEPROM position limit,
    /// which goals are checked against in position mode. Every model. A
    /// [`LimitField`].
    MaxPositionLimit,
    MaxPositionLimit
);
role_field!(
    /// Velocity Limit, in rad/s ([`Velocity`]): the EEPROM bound of Goal
    /// Velocity's magnitude. MX(2.0), X, Y, PRO. A [`LimitField`].
    VelocityLimit,
    VelocityLimit
);
role_field!(
    /// Current Limit, in amperes ([`Current`]): the EEPROM bound of Goal
    /// Current's magnitude. MX(2.0), X, Y, PRO. A [`LimitField`].
    CurrentLimit,
    CurrentLimit
);
role_field!(
    /// PWM Limit, a ratio of the full PWM output ([`Pwm`]). MX(2.0), X, Y,
    /// PRO. A [`LimitField`].
    PwmLimit,
    PwmLimit
);
role_field!(
    /// Max Torque, a ratio of the maximum torque ([`Load`]): the torque
    /// limit the actuator starts with at power-on. AX, RX, MX on Protocol 1,
    /// XL-320. A [`LimitField`].
    ///
    /// The running bound is Torque Limit (RAM), which power-on resets to
    /// Max Torque (AX-12A: "When the power is turned on, Torque Limit(34) is
    /// reset to the value of Max Torque(14)",
    /// <https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/>, 2026-11-09), so
    /// a new Max Torque bounds the output from the next power-on.
    MaxTorque,
    MaxTorque
);
role_field!(
    /// Acceleration Limit, in rad/s² ([`Acceleration`]): the EEPROM bound of
    /// Profile Acceleration, which the firmware enforces (MX-64(2.0):
    /// "Profile Acceleration(108) can’t be configured with any values
    /// exceeding Acceleration Limit(40)",
    /// <https://emanual.robotis.com/docs/en/dxl/mx/mx-64-2/>, 2026-11-09).
    /// MX(2.0), PRO, PRO+, Y, RH-P12-RN. A [`LimitField`].
    AccelerationLimit,
    AccelerationLimit
);

impl Rule for GoalVelocity {
    const MOVES: bool = true;

    fn check_mode(_: &Site, kind: &ActuatorKind) -> Result<(), ValidationError> {
        kind.cache().check_goal_velocity()
    }

    fn check_value(
        site: &Site,
        _: &Cache,
        bound: Option<u32>,
        raw: i32,
    ) -> Result<i32, ValidationError> {
        within_bound(site, bound, raw)
    }
}

impl Rule for GoalCurrent {
    const MOVES: bool = true;

    fn check_mode(_: &Site, kind: &ActuatorKind) -> Result<(), ValidationError> {
        kind.cache().check_goal_current()
    }

    fn check_value(
        site: &Site,
        _: &Cache,
        bound: Option<u32>,
        raw: i32,
    ) -> Result<i32, ValidationError> {
        within_bound(site, bound, raw)
    }
}

impl Rule for SpeedLimit {
    const MOVES: bool = false;

    fn check_mode(site: &Site, kind: &ActuatorKind) -> Result<(), ValidationError> {
        let is_goal = kind
            .data()
            .role(Role::GoalVelocity)
            .is_some_and(|goal| goal.register() == site.reg);
        if !is_goal {
            return Ok(());
        }
        kind.cache().check_mode_known()?;
        if site.mode.allows_goal_velocity() {
            Err(ValidationError::NotPositionMode { id: site.id })
        } else {
            Ok(())
        }
    }

    fn check_value(
        site: &Site,
        _: &Cache,
        _: Option<u32>,
        raw: i32,
    ) -> Result<i32, ValidationError> {
        in_range(site, raw)
    }
}

settings_rule!(Led);
settings_rule!(LedRed);
settings_rule!(LedGreen);
settings_rule!(LedBlue);
settings_rule!(PGain);
settings_rule!(DGain);
settings_rule!(CwComplianceMargin);
settings_rule!(CcwComplianceMargin);
settings_rule!(CwComplianceSlope);
settings_rule!(CcwComplianceSlope);

/// Declares the value type of each readable field.
macro_rules! read_values {
    ($($field:ident: $value:ty;)*) => {$(
        impl ReadField for $field {
            type Value = $value;
        }
    )*};
}

/// Declares the writable fields; a field writes the type it reads.
macro_rules! write_values {
    ($($field:ident),* $(,)?) => {$(
        impl WriteField for $field {
            type Value = <$field as ReadField>::Value;
        }
    )*};
}

read_values! {
    PresentTemperature: Temperature;
    PresentVoltage: Voltage;
    PresentVelocity: Velocity;
    PresentLoad: Load;
    PresentCurrent: Current;
    GoalVelocity: Velocity;
    SpeedLimit: crate::units::SpeedLimit;
    GoalCurrent: Current;
    Moving: bool;
    Led: bool;
    LedRed: u8;
    LedGreen: u8;
    LedBlue: u8;
    HardwareErrorStatus: HardwareErrors;
    RealtimeTick: Tick;
    PGain: Gain;
    DGain: Gain;
    CwComplianceMargin: ComplianceMargin;
    CcwComplianceMargin: ComplianceMargin;
    CwComplianceSlope: ComplianceSlope;
    CcwComplianceSlope: ComplianceSlope;
    TorqueCtrlModeEnable: bool;
    MinVoltageLimit: Voltage;
    MaxVoltageLimit: Voltage;
    TemperatureLimit: Temperature;
    MinPositionLimit: Position;
    MaxPositionLimit: Position;
    VelocityLimit: Velocity;
    CurrentLimit: Current;
    PwmLimit: Pwm;
    MaxTorque: Load;
    AccelerationLimit: Acceleration;
}

/// Declares the limit fields; a limit writes the type it reads.
macro_rules! limit_values {
    ($($field:ident),* $(,)?) => {$(
        impl LimitField for $field {
            type Value = <$field as ReadField>::Value;
        }
    )*};
}

limit_values! {
    MinVoltageLimit,
    MaxVoltageLimit,
    TemperatureLimit,
    MinPositionLimit,
    MaxPositionLimit,
    VelocityLimit,
    CurrentLimit,
    PwmLimit,
    MaxTorque,
    AccelerationLimit,
}

write_values! {
    GoalVelocity,
    SpeedLimit,
    GoalCurrent,
    Led,
    LedRed,
    LedGreen,
    LedBlue,
    PGain,
    DGain,
    CwComplianceMargin,
    CcwComplianceMargin,
    CwComplianceSlope,
    CcwComplianceSlope,
}

/// The scale of `site`, or why the typed value has none there.
fn scale_of(site: &Site) -> Result<RegisterScale, ValidationError> {
    site.scale.ok_or(ValidationError::NoUnit {
        id: site.id,
        register: site.name,
        mode: site.mode,
    })
}

/// A register value as the scales take it: the register's bits.
fn bits(raw: i32) -> u32 {
    raw.cast_unsigned()
}

/// Declares a typed value read through the register's scale.
macro_rules! scaled_decode {
    ($($value:ty => $method:ident;)*) => {$(
        impl Decode for $value {
            fn check_site(site: &Site) -> Result<(), ValidationError> {
                scale_of(site).map(drop)
            }

            fn from_raw(raw: i32, site: &Site) -> Result<Self, ValidationError> {
                Ok(scale_of(site)?.$method(bits(raw)))
            }
        }
    )*};
}

scaled_decode! {
    Temperature => temperature;
    Voltage => voltage;
    Velocity => velocity;
    Acceleration => acceleration;
    Load => load;
    Current => current;
    Pwm => pwm;
    crate::units::SpeedLimit => speed_limit;
}

/// Register bits `raw` from `scale` as the `i32` register value the checks
/// take.
///
/// # Errors
///
/// [`ValidationError::DoesNotFit`] with the true raw value when an
/// unsigned 4-byte value lies above `i32::MAX`, so no error reports a
/// wrapped value.
fn register_value(scale: RegisterScale, raw: u32) -> Result<i32, ValidationError> {
    let value = scale.signed_of(raw);
    if value > i64::from(i32::MAX) {
        Err(ValidationError::DoesNotFit {
            raw: value,
            size: scale.size(),
            request: crate::error::Request::NONE,
        })
    } else {
        Ok(raw.cast_signed())
    }
}

/// The register value of `si` (SI units) through the scale of `site`.
///
/// # Errors
///
/// `NoUnit`, `NotFinite`, `DoesNotFit`.
fn scaled(site: &Site, si: f32) -> Result<i32, ValidationError> {
    let scale = scale_of(site)?;
    register_value(scale, scale.raw_of_si(si)?)
}

impl Encode for Velocity {
    fn to_raw(self, site: &Site) -> Result<i32, ValidationError> {
        let scale = scale_of(site)?;
        register_value(scale, scale.velocity_to_raw(self)?)
    }
}

impl Encode for Acceleration {
    fn to_raw(self, site: &Site) -> Result<i32, ValidationError> {
        let scale = scale_of(site)?;
        register_value(scale, scale.acceleration_to_raw(self)?)
    }
}

impl Encode for Current {
    fn to_raw(self, site: &Site) -> Result<i32, ValidationError> {
        let scale = scale_of(site)?;
        register_value(scale, scale.current_to_raw(self)?)
    }
}

impl Encode for crate::units::SpeedLimit {
    fn to_raw(self, site: &Site) -> Result<i32, ValidationError> {
        let scale = scale_of(site)?;
        register_value(scale, scale.speed_limit_to_raw(self)?)
    }
}

impl Encode for Pwm {
    fn to_raw(self, site: &Site) -> Result<i32, ValidationError> {
        scaled(site, self.ratio())
    }
}

impl Encode for Voltage {
    fn to_raw(self, site: &Site) -> Result<i32, ValidationError> {
        scaled(site, self.volts())
    }
}

impl Encode for Temperature {
    fn to_raw(self, site: &Site) -> Result<i32, ValidationError> {
        scaled(site, self.celsius())
    }
}

impl Encode for Load {
    fn to_raw(self, site: &Site) -> Result<i32, ValidationError> {
        scaled(site, self.ratio())
    }
}

impl Decode for Position {
    fn check_site(_: &Site) -> Result<(), ValidationError> {
        Ok(())
    }

    fn from_raw(raw: i32, site: &Site) -> Result<Self, ValidationError> {
        Ok(site.position.to_position(raw))
    }
}

impl Encode for Position {
    const NEEDS_RANGE: bool = true;

    fn to_raw(self, site: &Site) -> Result<i32, ValidationError> {
        site.position.to_raw(self)
    }
}

impl Decode for i32 {
    fn check_site(_: &Site) -> Result<(), ValidationError> {
        Ok(())
    }

    fn from_raw(raw: i32, _: &Site) -> Result<Self, ValidationError> {
        Ok(raw)
    }
}

impl Encode for i32 {
    fn to_raw(self, _: &Site) -> Result<i32, ValidationError> {
        Ok(self)
    }
}

/// Declares a value that is the register value, whatever the mode.
macro_rules! plain_decode {
    ($($value:ty => |$raw:ident| $from:expr;)*) => {$(
        impl Decode for $value {
            fn check_site(_: &Site) -> Result<(), ValidationError> {
                Ok(())
            }

            fn from_raw($raw: i32, _: &Site) -> Result<Self, ValidationError> {
                Ok($from)
            }
        }
    )*};
}

plain_decode! {
    bool => |raw| raw != 0;
    u8 => |raw| raw.to_le_bytes()[0];
    HardwareErrors => |raw| HardwareErrors::from_raw(raw.to_le_bytes()[0]);
    Tick => |raw| {
        let [b0, b1, _, _] = raw.to_le_bytes();
        Tick::from_raw(u16::from_le_bytes([b0, b1]))
    };
    Gain => |raw| {
        let [b0, b1, _, _] = raw.to_le_bytes();
        Gain::from_raw(u16::from_le_bytes([b0, b1]))
    };
    ComplianceMargin => |raw| ComplianceMargin::from_raw(raw.to_le_bytes()[0]);
    ComplianceSlope => |raw| ComplianceSlope::from_raw(raw.to_le_bytes()[0]);
}

impl Encode for bool {
    fn to_raw(self, _: &Site) -> Result<i32, ValidationError> {
        Ok(i32::from(self))
    }
}

impl Encode for u8 {
    fn to_raw(self, _: &Site) -> Result<i32, ValidationError> {
        Ok(i32::from(self))
    }
}

impl Encode for Gain {
    fn to_raw(self, _: &Site) -> Result<i32, ValidationError> {
        Ok(i32::from(self.raw()))
    }
}

impl Encode for ComplianceMargin {
    fn to_raw(self, _: &Site) -> Result<i32, ValidationError> {
        Ok(i32::from(self.raw()))
    }
}

impl Encode for ComplianceSlope {
    fn to_raw(self, _: &Site) -> Result<i32, ValidationError> {
        Ok(i32::from(self.raw()))
    }
}

/// The site of a read of `F` on `kind`, checked before anything is sent.
///
/// # Errors
///
/// [`ValidationError::Unsupported`], [`ValidationError::NoUnit`].
pub(crate) fn read_site<F: ReadField>(kind: &ActuatorKind) -> Result<Site, ValidationError> {
    let site = F::site(kind)?;
    F::Value::check_site(&site)?;
    Ok(site)
}

/// The value of a read of `F` on `kind` from its decoded register value.
///
/// # Errors
///
/// As [`read_site`].
pub(crate) fn decode<F: ReadField>(kind: &ActuatorKind, raw: i32) -> Result<F::Value, Error> {
    let site = read_site::<F>(kind)?;
    F::Value::from_raw(raw, &site).map_err(|err| err.within(site.request(Instruction::Read)).into())
}

/// Validates a write of `value` to field `F` of `kind` and encodes it into
/// `out`, all before anything is sent: the register, the torque guard of a
/// goal, the operating mode, the value's conversion, the limits and the
/// register size, in that order. Returns the register and its bytes.
///
/// # Errors
///
/// [`ValidationError::Unsupported`], `ActuatorIsOff`, the field's mode
/// error, the value's conversion error, `OutOfLimits`, `DoesNotFit`.
pub(crate) fn encode_write<'o, F: WriteField>(
    kind: &ActuatorKind,
    on: Result<(), ValidationError>,
    bound: Option<u32>,
    value: F::Value,
    out: &'o mut [u8; 4],
) -> Result<(Register, &'o [u8]), ValidationError> {
    let site = F::site(kind)?;
    if site.reg.access != Access::RW {
        return Err(ValidationError::Unsupported {
            id: site.id,
            register: site.name,
        });
    }
    if F::MOVES {
        on?;
    }
    F::check_mode(&site, kind)?;
    let request = site.request(Instruction::Write);
    let raw = value.to_raw(&site).map_err(|err| err.within(request))?;
    let raw = F::check_value(&site, kind.cache(), bound, raw).map_err(|err| err.within(request))?;
    let bytes = site
        .reg
        .encode(raw, out)
        .map_err(|err| err.within(request))?;
    Ok((site.reg, bytes))
}

/// The site of limit field `F` on `kind`, checked before anything is
/// sent: the model has the register, writable.
///
/// # Errors
///
/// [`ValidationError::Unsupported`].
pub(crate) fn limit_site<F: LimitField>(kind: &ActuatorKind) -> Result<Site, ValidationError> {
    let site = F::site(kind)?;
    if site.reg.access == Access::RW {
        Ok(site)
    } else {
        Err(ValidationError::Unsupported {
            id: site.id,
            register: site.name,
        })
    }
}

/// The register value of limit `value` at `site`, checked against the
/// e-Manual range the table records for it.
///
/// # Errors
///
/// The value's conversion error (`NoUnit`, `NotFinite`, `DoesNotFit`),
/// then `OutOfLimits`. A typed position on a site with no recorded range
/// (an unverified table) is `NoUnit` (plan D6).
pub(crate) fn limit_raw<V: Encode>(site: &Site, value: V) -> Result<i32, ValidationError> {
    if V::NEEDS_RANGE && site.range.is_none() {
        return Err(ValidationError::NoUnit {
            id: site.id,
            register: site.name,
            mode: site.mode,
        });
    }
    let raw = value
        .to_raw(site)
        .map_err(|err| err.within(site.request(Instruction::Write)))?;
    in_range(site, raw)
}

/// The goal role whose EEPROM bound Setup reads in limit mode `mode`: Goal
/// Velocity in velocity and wheel mode, Goal Current in current and
/// current-based position mode.
pub(crate) fn bounded_goal(mode: LimitMode) -> Option<Role> {
    if mode.allows_goal_velocity() {
        Some(Role::GoalVelocity)
    } else if mode.allows_goal_current() {
        Some(Role::GoalCurrent)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::testing::id;
    use crate::register::Area;
    use crate::units::Sign;

    fn site(scale: Option<RegisterScale>, range: Option<(i64, i64)>) -> Site {
        Site {
            id: id(1),
            reg: Register {
                name: "Goal Velocity",
                addr: 104,
                size: 4,
                access: Access::RW,
                area: Area::Ram,
            },
            name: "Goal Velocity",
            mode: LimitMode::Velocity,
            position: PositionScale::new(2048, 0, 4095, -3.0, 3.0),
            scale,
            range,
        }
    }

    const SIGNED: RegisterScale = RegisterScale::new(1.0, Sign::TwosComplement, 4, false);

    #[test]
    fn bounds_and_ranges() {
        let signed = site(Some(SIGNED), None);
        assert_eq!(within_bound(&signed, Some(10), -10), Ok(-10));
        assert_eq!(
            within_bound(&signed, Some(10), -11),
            Err(ValidationError::OutOfLimits {
                id: id(1),
                raw: -11,
                min: -10,
                max: 10
            })
        );
        assert_eq!(
            within_bound(&signed, Some(u32::MAX), i32::MIN),
            Ok(i32::MIN)
        );
        let raw = site(None, Some((0, 5)));
        assert_eq!(
            within_bound(&raw, Some(3), 4),
            Err(ValidationError::OutOfLimits {
                id: id(1),
                raw: 4,
                min: -3,
                max: 3
            })
        );
        assert_eq!(
            within_bound(&raw, None, 6),
            Err(ValidationError::OutOfLimits {
                id: id(1),
                raw: 6,
                min: 0,
                max: 5
            })
        );
        let wide = site(None, Some((1 << 40, 1 << 41)));
        assert_eq!(
            in_range(&wide, 0),
            Err(ValidationError::OutOfLimits {
                id: id(1),
                raw: 0,
                min: i32::MAX,
                max: i32::MAX
            })
        );
        assert_eq!(saturate(-(1 << 40)), i32::MIN);
    }

    #[test]
    fn bounded_goal_per_mode() {
        for (mode, role) in [
            (LimitMode::Velocity, Some(Role::GoalVelocity)),
            (LimitMode::Wheel, Some(Role::GoalVelocity)),
            (LimitMode::Current, Some(Role::GoalCurrent)),
            (LimitMode::UnboundedCurrent, Some(Role::GoalCurrent)),
            (LimitMode::Bounded, None),
            (LimitMode::Unbounded, None),
            (LimitMode::Pwm, None),
        ] {
            assert_eq!(bounded_goal(mode), role, "{mode:?}");
        }
    }

    #[test]
    fn plain_values_are_the_register_value() {
        let at = site(None, None);
        assert_eq!(bool::from_raw(2, &at), Ok(true));
        assert_eq!(u8::from_raw(0x1FF, &at), Ok(0xFF));
        assert_eq!(
            <Tick as Decode>::from_raw(0x1_2345, &at),
            Ok(Tick::from_raw(0x2345))
        );
        assert_eq!(
            <Gain as Decode>::from_raw(850, &at),
            Ok(Gain::from_raw(850))
        );
        assert_eq!(
            <HardwareErrors as Decode>::from_raw(0x24, &at),
            Ok(HardwareErrors::from_raw(0x24))
        );
        assert_eq!(
            <ComplianceMargin as Decode>::from_raw(3, &at),
            Ok(ComplianceMargin::from_raw(3))
        );
        assert_eq!(
            <ComplianceSlope as Decode>::from_raw(64, &at),
            Ok(ComplianceSlope::from_raw(64))
        );
        assert_eq!(true.to_raw(&at), Ok(1));
        assert_eq!(7u8.to_raw(&at), Ok(7));
        assert_eq!(Gain::from_raw(9).to_raw(&at), Ok(9));
        assert_eq!(ComplianceMargin::from_raw(4).to_raw(&at), Ok(4));
        assert_eq!(ComplianceSlope::from_raw(32).to_raw(&at), Ok(32));
        assert_eq!(i32::from_raw(-5, &at), Ok(-5));
        assert_eq!((-5i32).to_raw(&at), Ok(-5));
    }

    #[test]
    fn typed_values_need_a_scale() {
        let none = site(None, None);
        let missing = Err(ValidationError::NoUnit {
            id: id(1),
            register: "Goal Velocity",
            mode: LimitMode::Velocity,
        });
        assert_eq!(Velocity::check_site(&none), missing);
        assert_eq!(
            Velocity::from_raw(1, &none),
            missing.map(|()| Velocity::ZERO)
        );
        assert_eq!(Velocity::ZERO.to_raw(&none), missing.map(|()| 0));
        assert_eq!(Current::ZERO.to_raw(&none), missing.map(|()| 0));
        assert_eq!(Acceleration::ZERO.to_raw(&none), missing.map(|()| 0));
        assert_eq!(
            Acceleration::from_raw(1, &none),
            missing.map(|()| Acceleration::ZERO)
        );
        assert_eq!(
            crate::units::SpeedLimit::Unlimited.to_raw(&none),
            missing.map(|()| 0)
        );
        let scaled = site(Some(SIGNED), None);
        assert_eq!(Velocity::check_site(&scaled), Ok(()));
        assert_eq!(
            Velocity::from_raw(-3, &scaled),
            Ok(Velocity::from_rad_per_s(-3.0))
        );
        assert_eq!(Velocity::from_rad_per_s(-3.0).to_raw(&scaled), Ok(-3));
        assert_eq!(Current::from_amps(2.0).to_raw(&scaled), Ok(2));
        assert_eq!(Acceleration::from_rad_per_s2(-4.0).to_raw(&scaled), Ok(-4));
        assert_eq!(
            Acceleration::from_raw(5, &scaled),
            Ok(Acceleration::from_rad_per_s2(5.0))
        );
        assert_eq!(Position::check_site(&none), Ok(()));
        assert_eq!(Position::from_raw(2048, &none), Ok(Position::ZERO));
        assert_eq!(Position::ZERO.to_raw(&none), Ok(2048));
    }
}
