//! Crate errors, split by where they come from.

use core::fmt;

use crate::Id;
use crate::protocol::ProtocolVersion;

/// Every error this crate returns, by category.
///
/// Match on the category first, then on its inner error for the detail:
///
/// - [`Transport`](Error::Transport): the serial line failed, or a reply
///   did not arrive in time.
/// - [`Protocol`](Error::Protocol): bytes arrived but did not form the
///   expected status packet.
/// - [`Status`](Error::Status): an actuator answered, and its status packet
///   reports an error.
/// - [`Validation`](Error::Validation): a value was refused before anything
///   was sent.
/// - [`Setup`](Error::Setup): the bus could not be set up as declared.
///
/// ```
/// use dxl_rs_v2::{Error, Id, SetupError, ValidationError};
///
/// fn advice(err: Error) -> &'static str {
///     match err {
///         Error::Setup(SetupError::Missing { .. }) => "check the wiring and the baud rate",
///         Error::Setup(_) => "check the declared actuators",
///         Error::Validation(ValidationError::OutOfLimits { .. }) => "pick a goal inside the limits",
///         Error::Validation(_) => "fix the arguments",
///         _ => "try again",
///     }
/// }
///
/// let id = Id::new(5)?;
/// assert_eq!(
///     advice(SetupError::Missing { id }.into()),
///     "check the wiring and the baud rate"
/// );
/// # Ok::<(), Error>(())
/// ```
///
/// Categories and variants may be added later without a breaking change, so
/// a `match` outside the crate needs a `_` arm:
///
/// ```compile_fail,E0004
/// fn category(err: dxl_rs_v2::Error) -> &'static str {
///     match err {
///         dxl_rs_v2::Error::Transport(_) => "transport",
///         dxl_rs_v2::Error::Protocol(_) => "protocol",
///         dxl_rs_v2::Error::Status(_) => "status",
///         dxl_rs_v2::Error::Validation(_) => "validation",
///         dxl_rs_v2::Error::Setup(_) => "setup",
///     }
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A value was rejected before anything was sent on the bus.
    #[error(transparent)]
    Validation(#[from] ValidationError),

    /// The serial line failed, or a reply did not arrive in time.
    #[error(transparent)]
    Transport(#[from] TransportError),

    /// Bytes arrived but did not form the expected status packet.
    #[error(transparent)]
    Protocol(#[from] ProtocolError),

    /// An actuator answered, and its status packet reports an error.
    #[error(transparent)]
    Status(#[from] StatusError),

    /// The bus could not be set up as declared.
    #[error(transparent)]
    Setup(#[from] SetupError),
}

/// Formats the "enable this feature" hint of [`SetupError::UnsupportedModel`].
struct FeatureHint(Option<&'static str>);

impl fmt::Display for FeatureHint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(feature) => write!(f, " (enable the `{feature}` feature)"),
            None => Ok(()),
        }
    }
}

/// Formats where a [`ValidationError::DoesNotFit`] value had to fit.
struct FitsIn(u8);

impl fmt::Display for FitsIn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            0 => f.write_str("in 32 bits"),
            size => write!(f, "a {size}-byte register"),
        }
    }
}

/// A value rejected before anything was sent on the bus.
///
/// Variants may be added later, so a `match` outside the crate needs a `_`
/// arm:
///
/// ```compile_fail,E0004
/// use dxl_rs_v2::ValidationError as V;
///
/// fn is_about_limits(err: V) -> bool {
///     match err {
///         V::OutOfLimits { .. } | V::LimitsUnknown { .. } | V::InvertedLimits { .. } => true,
///         V::InvalidId(_)
///         | V::AddressOutOfRange { .. }
///         | V::PacketTooLarge { .. }
///         | V::DataLength { .. }
///         | V::ActuatorIsOff { .. }
///         | V::ActuatorFaulted { .. }
///         | V::NotFinite
///         | V::DoesNotFit { .. }
///         | V::BufferLength { .. }
///         | V::DuplicateId { .. }
///         | V::NotPositionMode { .. } => false,
///     }
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ValidationError {
    /// The ID is outside `0..=252`; see [`Id::MAX`](crate::Id::MAX).
    #[error("invalid actuator id {0}: must be in 0..=252")]
    InvalidId(u8),

    /// Protocol 1 addresses are one byte; `addr` does not fit.
    #[error("address {addr} does not fit Protocol 1's one-byte address field")]
    AddressOutOfRange {
        /// The address that was asked for.
        addr: u16,
    },

    /// The packet's length field cannot describe `len` bytes; `max` is the
    /// protocol's limit.
    #[error("packet of {len} bytes exceeds the protocol limit of {max}")]
    PacketTooLarge {
        /// The length that was asked for, in bytes.
        len: usize,
        /// The largest length the protocol allows, in bytes.
        max: usize,
    },

    /// A data slice does not have the size the instruction declares.
    #[error("data is {got} bytes, expected {expected}")]
    DataLength {
        /// The size the instruction declares, in bytes.
        expected: usize,
        /// The size of the slice that was passed, in bytes.
        got: usize,
    },

    /// A goal was given while the actuator's torque is off. A faulted
    /// actuator counts as off.
    #[error(
        "actuator {} has torque off; enable torque before setting a goal",
        id.get()
    )]
    ActuatorIsOff {
        /// The actuator the goal was for.
        id: Id,
    },

    /// Torque was to be enabled on a faulted actuator.
    #[error(
        "actuator {} is faulted; clear the fault before enabling torque",
        id.get()
    )]
    ActuatorFaulted {
        /// The faulted actuator.
        id: Id,
    },

    /// A goal tick lies outside the position limits read from the
    /// actuator's EEPROM. Both bounds are allowed.
    #[error("goal {raw} for actuator {} is outside its limits {min}..={max}", id.get())]
    OutOfLimits {
        /// The actuator the goal was for.
        id: Id,
        /// The goal, in raw ticks.
        raw: i32,
        /// The minimum position limit, in raw ticks.
        min: i32,
        /// The maximum position limit, in raw ticks.
        max: i32,
    },

    /// A goal was checked before Setup read the actuator's position limits.
    /// Limits are never taken from a control table's defaults.
    #[error("limits of actuator {} are not known; they are read at setup", id.get())]
    LimitsUnknown {
        /// The actuator the goal was for.
        id: Id,
    },

    /// The actuator reports a minimum position limit above its maximum.
    #[error(
        "actuator {} reports a minimum limit {min} above its maximum {max}",
        id.get()
    )]
    InvertedLimits {
        /// The actuator that reported the limits.
        id: Id,
        /// The minimum position limit it reports, in raw ticks.
        min: i32,
        /// The maximum position limit it reports, in raw ticks.
        max: i32,
    },

    /// An angle is NaN or infinite, so no tick matches it.
    #[error("angle is not a finite number")]
    NotFinite,

    /// A value does not fit a register of `size` bytes, or, when `size` is
    /// 0, does not fit an `i32`. Values are never truncated.
    #[error("value {raw} does not fit {}", FitsIn(*size))]
    DoesNotFit {
        /// The value that does not fit.
        raw: i64,
        /// The register size in bytes; `size` 0 = does not fit `i32`.
        size: u8,
    },

    /// A caller buffer or value slice does not have one slot per target.
    #[error("buffer has {got} slots for {targets} targets")]
    BufferLength {
        /// The number of target actuators.
        targets: usize,
        /// The number of slots that were passed.
        got: usize,
    },

    /// An actuator ID appears more than once in a target or declared list.
    #[error("actuator {} is listed more than once", id.get())]
    DuplicateId {
        /// The repeated ID.
        id: Id,
    },

    /// The actuator's operating mode ignores Goal Position: velocity,
    /// current, PWM or wheel mode.
    #[error(
        "actuator {} is not in a position mode; it ignores goal positions",
        id.get()
    )]
    NotPositionMode {
        /// The actuator the goal was for.
        id: Id,
    },
}

/// The bus could not be set up as declared: an actuator is missing, unknown
/// to this build, or not what was declared.
///
/// Variants may be added later, so a `match` outside the crate needs a `_`
/// arm:
///
/// ```compile_fail,E0004
/// use dxl_rs_v2::SetupError as S;
///
/// fn names_an_actuator(err: S) -> bool {
///     match err {
///         S::TooManyActuators { .. } => false,
///         S::UnsupportedModel { .. }
///         | S::Missing { .. }
///         | S::WrongModel { .. }
///         | S::NotOnBus { .. }
///         | S::UnknownOperatingMode { .. } => true,
///     }
/// }
/// ```
///
/// An unsupported model is a setup error, matched as
/// `Error::Setup(SetupError::UnsupportedModel { .. })`; [`Error`] has no
/// variant of its own for it:
///
/// ```compile_fail,E0599
/// fn unsupported(err: dxl_rs_v2::Error) -> bool {
///     matches!(err, dxl_rs_v2::Error::UnsupportedModel { .. })
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SetupError {
    /// No model compiled into this build has this model number on this
    /// protocol.
    ///
    /// `feature` names the Cargo feature that would add the model, when the
    /// model is known but its family is off; `None` otherwise.
    #[error(
        "actuator {} reports model number {model_number}, not supported on {protocol}{}",
        id.get(),
        FeatureHint(*feature)
    )]
    UnsupportedModel {
        /// The actuator that reported the model number.
        id: Id,
        /// The model number it reported.
        model_number: u16,
        /// The protocol of the bus.
        protocol: ProtocolVersion,
        /// The Cargo feature that adds this model, if any.
        feature: Option<&'static str>,
    },

    /// More actuators answered than the bus can hold.
    #[error("more actuators answered than the bus capacity of {capacity}")]
    TooManyActuators {
        /// The number of actuators the bus can hold.
        capacity: usize,
    },

    /// A declared actuator did not answer.
    #[error("declared actuator {} did not answer", id.get())]
    Missing {
        /// The declared actuator.
        id: Id,
    },

    /// A declared actuator reports another model number than declared.
    #[error(
        "actuator {} was declared as model number {declared} but reports {found}",
        id.get()
    )]
    WrongModel {
        /// The declared actuator.
        id: Id,
        /// The model number it was declared with.
        declared: u16,
        /// The model number it reports.
        found: u16,
    },

    /// A target ID that the bus does not hold.
    #[error("actuator {} is not on this bus", id.get())]
    NotOnBus {
        /// The ID that was asked for.
        id: Id,
    },

    /// The actuator's Operating Mode (or XL-320 Control Mode) value is in
    /// none of its model's value lists, which come from its control table;
    /// no mode is guessed (decided 2026-10-28).
    #[error("actuator {} reports unknown operating mode {value}", id.get())]
    UnknownOperatingMode {
        /// The actuator that reported the value.
        id: Id,
        /// The mode register value it reported.
        value: u8,
    },
}

/// The serial line failed, or a reply did not arrive in time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum TransportError {
    /// The transport returned an error other than a timeout.
    #[error("serial transport error: {0}")]
    Io(embedded_io::ErrorKind),

    /// No complete reply arrived before the deadline. `id` is the actuator
    /// that was expected to answer, when there was exactly one.
    #[error("timed out waiting for {}", WaitingFor(*id))]
    Timeout {
        /// The actuator that was expected to answer, if only one was.
        id: Option<Id>,
    },
}

/// Formats who a [`TransportError::Timeout`] was waiting for.
struct WaitingFor(Option<Id>);

impl fmt::Display for WaitingFor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(id) => write!(f, "actuator {}", id.get()),
            None => f.write_str("a reply"),
        }
    }
}

/// Bytes arrived but did not form the expected status packet.
///
/// IDs are raw bytes here: a corrupted packet may carry an ID that is not a
/// valid [`Id`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ProtocolError {
    /// A Protocol 1 status packet failed its checksum.
    #[error("checksum mismatch in status packet from id {id}")]
    Checksum {
        /// The raw ID byte of the packet.
        id: u8,
    },

    /// A Protocol 2 status packet failed its CRC.
    #[error("CRC mismatch in status packet from id {id}")]
    Crc {
        /// The raw ID byte of the packet.
        id: u8,
    },

    /// A valid status packet came from another actuator than the one asked.
    #[error("expected a reply from actuator {}, got id {got}", expected.get())]
    WrongId {
        /// The actuator the instruction was sent to.
        expected: Id,
        /// The raw ID byte of the reply.
        got: u8,
    },

    /// A valid status packet carried a different amount of data than asked.
    #[error("expected {expected} data bytes in the reply, got {got}")]
    WrongLength {
        /// The number of data bytes asked for.
        expected: usize,
        /// The number of data bytes the reply carried.
        got: usize,
    },

    /// A length field that no valid packet can have, or one longer than
    /// the expected reply can be (a corrupted length that would otherwise
    /// swallow the packets after it). Also a scan reply carrying an ID no
    /// actuator can have.
    #[error("malformed status packet")]
    Malformed,
}

/// An actuator's status packet reported an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("actuator {} reported {report}", id.get())]
#[non_exhaustive]
pub struct StatusError {
    /// The actuator that reported it.
    pub id: Id,
    /// What its error byte says, decoded per protocol.
    pub report: StatusReport,
}

impl StatusError {
    /// Whether the report is a hardware fault, after which the actuator
    /// should be treated as faulted and its torque as off.
    ///
    /// - Protocol 2: the alert bit (bit 7) is set. The e-Manual defines it as
    ///   "a problem with the device", detailed in Hardware Error Status
    ///   (<https://emanual.robotis.com/docs/en/dxl/protocol2/#error>).
    /// - Protocol 1: Overload, Overheating or Input Voltage is set. The other
    ///   flags report a rejected instruction, not the hardware
    ///   (<https://emanual.robotis.com/docs/en/dxl/protocol1/#error>).
    pub fn is_hardware_fault(&self) -> bool {
        match self.report {
            StatusReport::P1(flags) => {
                flags.contains(P1Flag::Overload)
                    || flags.contains(P1Flag::Overheating)
                    || flags.contains(P1Flag::InputVoltage)
            }
            StatusReport::P2 { alert, .. } => alert,
        }
    }
}

/// The error byte of a status packet, decoded per protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StatusReport {
    /// Protocol 1: one flag per bit.
    P1(P1Flags),
    /// Protocol 2: the alert bit and an optional error code.
    P2 {
        /// Bit 7: a hardware fault; Hardware Error Status says which.
        alert: bool,
        /// Bits 0 to 6, or `None` when they are all zero.
        code: Option<P2Code>,
    },
}

impl fmt::Display for StatusReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            StatusReport::P1(flags) => write!(f, "{flags}"),
            StatusReport::P2 { alert, code } => match (alert, code) {
                (true, Some(code)) => write!(f, "a hardware alert and {code}"),
                (true, None) => f.write_str("a hardware alert"),
                (false, Some(code)) => write!(f, "{code}"),
                (false, None) => f.write_str("no error"),
            },
        }
    }
}

/// A Protocol 2 error code (bits 0..=6 of the error byte).
///
/// Source: <https://emanual.robotis.com/docs/en/dxl/protocol2/#error>.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum P2Code {
    /// 0x01: the instruction packet could not be processed.
    ResultFail,
    /// 0x02: undefined instruction, or Action without Reg Write.
    Instruction,
    /// 0x03: the actuator saw a CRC mismatch in our packet.
    Crc,
    /// 0x04: the value is outside the register's range.
    DataRange,
    /// 0x05: the data is shorter than the register.
    DataLength,
    /// 0x06: the value is outside the configured limit.
    DataLimit,
    /// 0x07: read-only, write-only or undefined address, or an EEPROM write
    /// while torque is on.
    Access,
    /// A code the e-Manual does not define.
    Unknown(u8),
}

impl P2Code {
    /// Decodes the error code from a Protocol 2 error byte, ignoring the
    /// alert bit. `None` when the code is 0.
    pub const fn from_bits(byte: u8) -> Option<Self> {
        Some(match byte & 0x7F {
            0 => return None,
            1 => Self::ResultFail,
            2 => Self::Instruction,
            3 => Self::Crc,
            4 => Self::DataRange,
            5 => Self::DataLength,
            6 => Self::DataLimit,
            7 => Self::Access,
            other => Self::Unknown(other),
        })
    }
}

impl fmt::Display for P2Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResultFail => f.write_str("result fail"),
            Self::Instruction => f.write_str("instruction error"),
            Self::Crc => f.write_str("CRC error"),
            Self::DataRange => f.write_str("data range error"),
            Self::DataLength => f.write_str("data length error"),
            Self::DataLimit => f.write_str("data limit error"),
            Self::Access => f.write_str("access error"),
            Self::Unknown(code) => write!(f, "unknown error code {code}"),
        }
    }
}

/// One Protocol 1 error flag.
///
/// Source: <https://emanual.robotis.com/docs/en/dxl/protocol1/#error>.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum P1Flag {
    /// Bit 0: input voltage out of range.
    InputVoltage,
    /// Bit 1: goal position outside the angle limits.
    AngleLimit,
    /// Bit 2: internal temperature too high.
    Overheating,
    /// Bit 3: instruction value out of range.
    Range,
    /// Bit 4: the actuator saw a checksum mismatch in our packet.
    Checksum,
    /// Bit 5: the load cannot be controlled with the set torque.
    Overload,
    /// Bit 6: undefined instruction, or Action without Reg Write.
    Instruction,
}

impl P1Flag {
    const ALL: [Self; 7] = [
        Self::InputVoltage,
        Self::AngleLimit,
        Self::Overheating,
        Self::Range,
        Self::Checksum,
        Self::Overload,
        Self::Instruction,
    ];

    const fn bit(self) -> u8 {
        1 << self as u8
    }

    const fn name(self) -> &'static str {
        match self {
            Self::InputVoltage => "input voltage error",
            Self::AngleLimit => "angle limit error",
            Self::Overheating => "overheating",
            Self::Range => "range error",
            Self::Checksum => "checksum error",
            Self::Overload => "overload",
            Self::Instruction => "instruction error",
        }
    }
}

impl fmt::Display for P1Flag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A Protocol 1 error byte: any combination of [`P1Flag`]s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct P1Flags(pub u8);

impl P1Flags {
    /// Whether `flag` is set.
    pub const fn contains(self, flag: P1Flag) -> bool {
        self.0 & flag.bit() != 0
    }

    /// Every set flag, lowest bit first.
    pub fn iter(self) -> impl Iterator<Item = P1Flag> {
        P1Flag::ALL
            .into_iter()
            .filter(move |&flag| self.contains(flag))
    }
}

impl fmt::Display for P1Flags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unknown = self.0 & !P1Flag::ALL.iter().fold(0, |bits, flag| bits | flag.bit());
        let mut flags = self.iter();
        match flags.next() {
            None if unknown == 0 => return f.write_str("no error"),
            None => return write!(f, "unknown error bits {unknown:#04X}"),
            Some(first) => {
                write!(f, "{first}")?;
                flags.try_for_each(|flag| write!(f, ", {flag}"))?;
            }
        }
        if unknown != 0 {
            write!(f, ", unknown error bits {unknown:#04X}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use std::string::ToString;

    fn id(raw: u8) -> Id {
        Id::new(raw).unwrap()
    }

    #[test]
    fn validation_messages() {
        let cases = [
            (
                ValidationError::AddressOutOfRange { addr: 300 },
                "address 300 does not fit Protocol 1's one-byte address field",
            ),
            (
                ValidationError::PacketTooLarge { len: 300, max: 255 },
                "packet of 300 bytes exceeds the protocol limit of 255",
            ),
            (
                ValidationError::DataLength {
                    expected: 4,
                    got: 2,
                },
                "data is 2 bytes, expected 4",
            ),
        ];
        for (err, msg) in cases {
            assert_eq!(Error::from(err).to_string(), msg);
        }
    }

    fn core_validation_cases() -> [(ValidationError, &'static str); 11] {
        [
            (
                ValidationError::ActuatorIsOff { id: id(4) },
                "actuator 4 has torque off; enable torque before setting a goal",
            ),
            (
                ValidationError::ActuatorFaulted { id: id(4) },
                "actuator 4 is faulted; clear the fault before enabling torque",
            ),
            (
                ValidationError::OutOfLimits {
                    id: id(3),
                    raw: 3001,
                    min: 1000,
                    max: 3000,
                },
                "goal 3001 for actuator 3 is outside its limits 1000..=3000",
            ),
            (
                ValidationError::LimitsUnknown { id: id(3) },
                "limits of actuator 3 are not known; they are read at setup",
            ),
            (
                ValidationError::InvertedLimits {
                    id: id(3),
                    min: 3000,
                    max: 1000,
                },
                "actuator 3 reports a minimum limit 3000 above its maximum 1000",
            ),
            (ValidationError::NotFinite, "angle is not a finite number"),
            (
                ValidationError::DoesNotFit {
                    raw: 70000,
                    size: 2,
                },
                "value 70000 does not fit a 2-byte register",
            ),
            (
                ValidationError::DoesNotFit {
                    raw: 3_000_000_000,
                    size: 0,
                },
                "value 3000000000 does not fit in 32 bits",
            ),
            (
                ValidationError::BufferLength { targets: 3, got: 2 },
                "buffer has 2 slots for 3 targets",
            ),
            (
                ValidationError::DuplicateId { id: id(5) },
                "actuator 5 is listed more than once",
            ),
            (
                ValidationError::NotPositionMode { id: id(4) },
                "actuator 4 is not in a position mode; it ignores goal positions",
            ),
        ]
    }

    #[test]
    fn core_validation_messages() {
        for (err, msg) in core_validation_cases() {
            assert_eq!(Error::from(err).to_string(), msg);
        }
    }

    #[test]
    fn validation_converts_under_its_category() {
        for (err, _) in core_validation_cases() {
            assert_eq!(Error::from(err), Error::Validation(err));
        }
    }

    #[test]
    fn setup_messages() {
        let unsupported = |feature| SetupError::UnsupportedModel {
            id: id(3),
            model_number: 1020,
            protocol: ProtocolVersion::P1,
            feature,
        };
        let cases = [
            (
                unsupported(Some("x-series")),
                "actuator 3 reports model number 1020, not supported on Protocol 1 \
                 (enable the `x-series` feature)",
            ),
            (
                unsupported(None),
                "actuator 3 reports model number 1020, not supported on Protocol 1",
            ),
            (
                SetupError::TooManyActuators { capacity: 32 },
                "more actuators answered than the bus capacity of 32",
            ),
            (
                SetupError::Missing { id: id(5) },
                "declared actuator 5 did not answer",
            ),
            (
                SetupError::WrongModel {
                    id: id(5),
                    declared: 1020,
                    found: 12,
                },
                "actuator 5 was declared as model number 1020 but reports 12",
            ),
            (
                SetupError::NotOnBus { id: id(7) },
                "actuator 7 is not on this bus",
            ),
            (
                SetupError::UnknownOperatingMode {
                    id: id(5),
                    value: 7,
                },
                "actuator 5 reports unknown operating mode 7",
            ),
        ];
        for (err, msg) in cases {
            assert_eq!(Error::from(err).to_string(), msg);
            assert_eq!(Error::from(err), Error::Setup(err));
        }
    }

    #[test]
    fn setup_is_its_own_category() {
        assert!(matches!(
            Error::from(SetupError::Missing { id: id(5) }),
            Error::Setup(SetupError::Missing { id: missing }) if missing == id(5)
        ));
    }

    #[test]
    fn unsupported_model_is_under_setup() {
        let err = Error::from(SetupError::UnsupportedModel {
            id: id(3),
            model_number: 1020,
            protocol: ProtocolVersion::P2,
            feature: Some("x-series"),
        });
        assert!(matches!(
            err,
            Error::Setup(SetupError::UnsupportedModel {
                model_number: 1020,
                protocol: ProtocolVersion::P2,
                ..
            })
        ));
    }

    #[test]
    fn errors_are_copy_eq_and_alloc_free() {
        const fn assert_traits<T: Copy + Eq + core::error::Error>() {}
        assert_traits::<Error>();
        assert_traits::<ValidationError>();
        assert_traits::<SetupError>();
        let err = Error::from(SetupError::Missing { id: id(5) });
        let copy = err;
        assert_eq!(err, copy);
        assert_ne!(err, Error::from(ValidationError::NotFinite));
    }

    #[test]
    fn transport_messages() {
        assert_eq!(
            Error::from(TransportError::Timeout { id: Some(id(3)) }).to_string(),
            "timed out waiting for actuator 3"
        );
        assert_eq!(
            TransportError::Timeout { id: None }.to_string(),
            "timed out waiting for a reply"
        );
        assert!(
            TransportError::Io(embedded_io::ErrorKind::BrokenPipe)
                .to_string()
                .starts_with("serial transport error: ")
        );
    }

    #[test]
    fn protocol_messages() {
        let cases = [
            (
                ProtocolError::Checksum { id: 1 },
                "checksum mismatch in status packet from id 1",
            ),
            (
                ProtocolError::Crc { id: 2 },
                "CRC mismatch in status packet from id 2",
            ),
            (
                ProtocolError::WrongId {
                    expected: id(1),
                    got: 7,
                },
                "expected a reply from actuator 1, got id 7",
            ),
            (
                ProtocolError::WrongLength {
                    expected: 4,
                    got: 0,
                },
                "expected 4 data bytes in the reply, got 0",
            ),
            (ProtocolError::Malformed, "malformed status packet"),
        ];
        for (err, msg) in cases {
            assert_eq!(Error::from(err).to_string(), msg);
        }
    }

    #[test]
    fn p2_codes_decode_and_ignore_alert() {
        assert_eq!(P2Code::from_bits(0), None);
        assert_eq!(P2Code::from_bits(0x80), None);
        assert_eq!(P2Code::from_bits(0x01), Some(P2Code::ResultFail));
        assert_eq!(P2Code::from_bits(0x85), Some(P2Code::DataLength));
        assert_eq!(P2Code::from_bits(0x07), Some(P2Code::Access));
        assert_eq!(P2Code::from_bits(0x09), Some(P2Code::Unknown(9)));
    }

    #[test]
    fn p1_flags_iterate_every_set_bit() {
        let flags = P1Flags(0x24);
        let set: std::vec::Vec<_> = flags.iter().collect();
        assert_eq!(set, [P1Flag::Overheating, P1Flag::Overload]);
        assert_eq!(P1Flags(0).iter().count(), 0);
        assert_eq!(P1Flags(0x7F).iter().count(), 7);
    }

    #[test]
    fn hardware_faults_per_protocol() {
        let p1 = |byte| StatusError {
            id: id(1),
            report: StatusReport::P1(P1Flags(byte)),
        };
        assert!(p1(0x20).is_hardware_fault());
        assert!(p1(0x04).is_hardware_fault());
        assert!(p1(0x01).is_hardware_fault());
        assert!(!p1(0x08).is_hardware_fault());
        assert!(!p1(0x40).is_hardware_fault());

        let p2 = |alert, code| StatusError {
            id: id(1),
            report: StatusReport::P2 { alert, code },
        };
        assert!(p2(true, None).is_hardware_fault());
        assert!(!p2(false, Some(P2Code::Access)).is_hardware_fault());
    }

    #[test]
    fn status_messages_name_id_and_every_condition() {
        let p1 = StatusError {
            id: id(1),
            report: StatusReport::P1(P1Flags(0x24)),
        };
        assert_eq!(
            Error::from(p1).to_string(),
            "actuator 1 reported overheating, overload"
        );
        let p2 = |alert, code| {
            StatusError {
                id: id(5),
                report: StatusReport::P2 { alert, code },
            }
            .to_string()
        };
        assert_eq!(
            p2(true, Some(P2Code::DataLength)),
            "actuator 5 reported a hardware alert and data length error"
        );
        assert_eq!(p2(true, None), "actuator 5 reported a hardware alert");
        assert_eq!(
            p2(false, Some(P2Code::Unknown(9))),
            "actuator 5 reported unknown error code 9"
        );
        assert_eq!(p2(false, None), "actuator 5 reported no error");
        assert_eq!(P1Flags(0).to_string(), "no error");
        assert_eq!(P1Flags(0x80).to_string(), "unknown error bits 0x80");
        assert_eq!(
            P1Flags(0xA0).to_string(),
            "overload, unknown error bits 0x80"
        );
        for (code, name) in [
            (P2Code::ResultFail, "result fail"),
            (P2Code::Instruction, "instruction error"),
            (P2Code::Crc, "CRC error"),
            (P2Code::DataRange, "data range error"),
            (P2Code::DataLimit, "data limit error"),
            (P2Code::Access, "access error"),
        ] {
            assert_eq!(code.to_string(), name);
        }
        for flag in P1Flag::ALL {
            assert_eq!(flag.to_string(), flag.name());
        }
    }
}
