//! Crate errors, split by where they come from.
//!
//! Every error says what failed and why in its `Display` text: which
//! actuator, which instruction and register (a [`Request`]), which setup
//! step, and the values involved. `Debug` on [`Error`] prints the same text,
//! so `?` out of `main` reads as a sentence; `{:#?}` prints the fields.

use core::fmt;

use crate::Id;
use crate::actuator::LimitMode;
use crate::bus::IdSet;
use crate::protocol::ProtocolVersion;
use crate::units::HardwareErrors;

/// Every error this crate returns, by category.
///
/// Match on the category first, then on its inner error for the detail:
///
/// - [`Transport`](Error::Transport): the serial line failed, or a reply
///   did not arrive in time.
/// - [`Protocol`](Error::Protocol): bytes arrived but did not form the
///   expected status packet.
/// - [`Status`](Error::Status): an actuator answered, and its status packet
///   reports an error that is not a hardware fault.
/// - [`Fault`](Error::Fault): an actuator reports a hardware fault, measured
///   against its limits (input voltage below its minimum, temperature above
///   its limit, ...).
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
///     advice(SetupError::Missing { id, timeout_us: 10_000 }.into()),
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
///         dxl_rs_v2::Error::Fault(_) => "fault",
///         dxl_rs_v2::Error::Validation(_) => "validation",
///         dxl_rs_v2::Error::Setup(_) => "setup",
///     }
/// }
/// ```
#[derive(Clone, Copy, PartialEq, Eq, thiserror::Error)]
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

    /// An actuator reports a hardware fault, measured against its limits.
    #[error(transparent)]
    Fault(#[from] FaultError),

    /// The bus could not be set up as declared.
    #[error(transparent)]
    Setup(#[from] SetupError),
}

/// Prints the `Display` text, so `fn main() -> Result<(), Error>` reports a
/// sentence. The alternate form `{:#?}` prints the variant and its fields.
impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !f.alternate() {
            return fmt::Display::fmt(self, f);
        }
        match self {
            Self::Validation(err) => f.debug_tuple("Validation").field(err).finish(),
            Self::Transport(err) => f.debug_tuple("Transport").field(err).finish(),
            Self::Protocol(err) => f.debug_tuple("Protocol").field(err).finish(),
            Self::Status(err) => f.debug_tuple("Status").field(err).finish(),
            Self::Fault(err) => f.debug_tuple("Fault").field(err).finish(),
            Self::Setup(err) => f.debug_tuple("Setup").field(err).finish(),
        }
    }
}

impl Error {
    /// The request the error happened in, for the categories that carry
    /// one: the instruction, the actuator, the register and the setup step.
    pub const fn request(&self) -> Option<&Request> {
        match self {
            Self::Validation(err) => err.request(),
            Self::Transport(err) => Some(err.request()),
            Self::Protocol(err) => Some(err.request()),
            Self::Status(err) => Some(&err.request),
            Self::Fault(err) => Some(err.request()),
            Self::Setup(_) => None,
        }
    }

    /// Whether the error is a hardware fault, after which the actuator is
    /// treated as faulted and its torque as off: a [`FaultError`], or a
    /// [`StatusError`] whose report is one (before it is measured).
    pub fn is_hardware_fault(&self) -> bool {
        match self {
            Self::Fault(_) => true,
            Self::Status(status) => status.is_hardware_fault(),
            _ => false,
        }
    }
}

/// A Dynamixel instruction, by its e-Manual name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Instruction {
    /// Ping (0x01).
    Ping,
    /// Read (0x02).
    Read,
    /// Write (0x03).
    Write,
    /// Reboot (0x08), Protocol 2.
    Reboot,
    /// Sync Read (0x82), Protocol 2.
    SyncRead,
    /// Sync Write (0x83).
    SyncWrite,
    /// Bulk Read (0x92).
    BulkRead,
    /// Bulk Write (0x93), Protocol 2.
    BulkWrite,
}

impl Instruction {
    /// Whether the instruction brings data back, so its target is the
    /// actuator data comes "from".
    const fn reads(self) -> bool {
        matches!(
            self,
            Self::Ping | Self::Read | Self::SyncRead | Self::BulkRead
        )
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Ping => "Ping",
            Self::Read => "Read",
            Self::Write => "Write",
            Self::Reboot => "Reboot",
            Self::SyncRead => "Sync Read",
            Self::SyncWrite => "Sync Write",
            Self::BulkRead => "Bulk Read",
            Self::BulkWrite => "Bulk Write",
        })
    }
}

/// The step of Setup (or of a fault diagnosis) an error happened in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Step {
    /// Autoscan: the Protocol 2 broadcast Ping, or the Protocol 1 sweep of
    /// IDs 0..=252.
    Scan,
    /// Declared Setup: the Ping of a declared actuator.
    Ping,
    /// Reading Model Number.
    ModelNumber,
    /// Reading Firmware Version, where the model's table depends on it.
    Firmware,
    /// Reading the EEPROM position (angle) limits.
    PositionLimits,
    /// Reading the operating mode register.
    OperatingMode,
    /// Reading the EEPROM bound of the goal the mode uses (Velocity Limit,
    /// Current Limit).
    GoalBound,
    /// Reading Hardware Error Status, present voltage or temperature, and
    /// their limits, after a reply reported a hardware fault.
    FaultDiagnosis,
}

impl fmt::Display for Step {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Scan => "setup, scanning for actuators",
            Self::Ping => "setup, pinging a declared actuator",
            Self::ModelNumber => "setup, reading the model number",
            Self::Firmware => "setup, reading the firmware version",
            Self::PositionLimits => "setup, reading the position limits",
            Self::OperatingMode => "setup, reading the operating mode",
            Self::GoalBound => "setup, reading the goal's EEPROM bound",
            Self::FaultDiagnosis => "the diagnosis of a hardware fault",
        })
    }
}

/// What a failed call was doing: the instruction, its target, the register
/// and the setup step. Fields are `None` where they do not apply or are not
/// known (a raw read has an address but no register name).
///
/// ```
/// use dxl_rs_v2::{Id, Instruction, Request};
///
/// let request = Request::NONE
///     .instruction(Instruction::Read)
///     .id(Id::new(1)?)
///     .register("Model Number")
///     .addr(0)
///     .len(2);
/// assert_eq!(
///     request.to_string(),
///     "Read of Model Number (address 0, 2 bytes) from actuator 1"
/// );
/// # Ok::<(), dxl_rs_v2::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub struct Request {
    /// The instruction.
    pub instruction: Option<Instruction>,
    /// The actuator it addressed; `None` for a broadcast to all.
    pub id: Option<Id>,
    /// The register's name, as the control table writes it.
    pub register: Option<&'static str>,
    /// The control-table address.
    pub addr: Option<u16>,
    /// The number of data bytes read or written.
    pub len: Option<u16>,
    /// The setup step, during Setup or a fault diagnosis.
    pub step: Option<Step>,
}

impl Request {
    /// Nothing known.
    pub const NONE: Self = Self {
        instruction: None,
        id: None,
        register: None,
        addr: None,
        len: None,
        step: None,
    };

    /// With `instruction`.
    pub const fn instruction(mut self, instruction: Instruction) -> Self {
        self.instruction = Some(instruction);
        self
    }

    /// With target `id`.
    pub const fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// With register name `register`.
    pub const fn register(mut self, register: &'static str) -> Self {
        self.register = Some(register);
        self
    }

    /// With address `addr`.
    pub const fn addr(mut self, addr: u16) -> Self {
        self.addr = Some(addr);
        self
    }

    /// With `len` data bytes.
    pub const fn len(mut self, len: u16) -> Self {
        self.len = Some(len);
        self
    }

    /// With setup step `step`.
    pub const fn step(mut self, step: Step) -> Self {
        self.step = Some(step);
        self
    }

    /// Whether nothing is known.
    pub const fn is_empty(&self) -> bool {
        self.instruction.is_none()
            && self.id.is_none()
            && self.register.is_none()
            && self.addr.is_none()
            && self.len.is_none()
            && self.step.is_none()
    }

    /// Fills the unknown fields from `outer`.
    pub(crate) const fn fill(&mut self, outer: Self) {
        if self.instruction.is_none() {
            self.instruction = outer.instruction;
        }
        if self.id.is_none() {
            self.id = outer.id;
        }
        if self.register.is_none() {
            self.register = outer.register;
        }
        if self.addr.is_none() {
            self.addr = outer.addr;
        }
        if self.len.is_none() {
            self.len = outer.len;
        }
        if self.step.is_none() {
            self.step = outer.step;
        }
    }

    /// This request with its unknown fields filled from `outer`.
    pub(crate) const fn filled(mut self, outer: Self) -> Self {
        self.fill(outer);
        self
    }

    /// `len` as a request field: lengths above `u16::MAX` saturate.
    pub(crate) fn len_of(len: usize) -> u16 {
        u16::try_from(len).unwrap_or(u16::MAX)
    }
}

/// E.g. "Read of Model Number (address 0, 2 bytes) from actuator 1, during
/// setup, reading the model number"; "the request" when nothing is known.
impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return f.write_str("the request");
        }
        let mut written = false;
        if let Some(instruction) = self.instruction {
            write!(f, "{instruction}")?;
            written = true;
        }
        if let Some(register) = self.register {
            if written {
                f.write_str(" of ")?;
            }
            f.write_str(register)?;
            written = true;
        }
        match (self.addr, self.len) {
            (Some(addr), Some(len)) => {
                write!(f, "{}(address {addr}, {})", Sep(written), Bytes(len))?
            }
            (Some(addr), None) => write!(f, "{}(address {addr})", Sep(written))?,
            (None, Some(len)) => write!(f, "{}({})", Sep(written), Bytes(len))?,
            (None, None) => {}
        }
        written |= self.addr.is_some() || self.len.is_some();
        if let Some(id) = self.id {
            let preposition = match self.instruction {
                Some(instruction) if instruction.reads() => "from",
                Some(_) => "to",
                None => "on",
            };
            if written {
                f.write_str(" ")?;
            }
            write!(f, "{preposition} actuator {}", id.get())?;
            written = true;
        } else if self.instruction.is_some() && self.instruction != Some(Instruction::Ping) {
            f.write_str(" (broadcast)")?;
        }
        if let Some(step) = self.step {
            if written {
                f.write_str(", ")?;
            }
            write!(f, "during {step}")?;
        }
        Ok(())
    }
}

/// A space when something was written before.
struct Sep(bool);

impl fmt::Display for Sep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 { f.write_str(" ") } else { Ok(()) }
    }
}

/// "1 byte", "4 bytes".
struct Bytes(u16);

impl fmt::Display for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            1 => f.write_str("1 byte"),
            n => write!(f, "{n} bytes"),
        }
    }
}

/// " (in <request>)", or nothing for an empty request.
struct In<'a>(&'a Request);

impl fmt::Display for In<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            Ok(())
        } else {
            write!(f, " (in {})", self.0)
        }
    }
}

/// " in reply to <request>", or nothing for an empty request.
struct ReplyTo<'a>(&'a Request);

impl fmt::Display for ReplyTo<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            Ok(())
        } else {
            write!(f, " in reply to {}", self.0)
        }
    }
}

/// A duration in microseconds, printed in milliseconds: "10 ms", "1.5 ms".
struct Ms(u32);

impl fmt::Display for Ms {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (ms, rest) = (self.0 / 1000, self.0 % 1000);
        if rest == 0 {
            write!(f, "{ms} ms")
        } else {
            let mut frac = rest;
            let mut digits = 3;
            while frac % 10 == 0 {
                frac /= 10;
                digits -= 1;
            }
            write!(f, "{ms}.{frac:0digits$} ms")
        }
    }
}

/// FTDI adapters (the U2D2 included) hold received bytes for up to their
/// latency timer: 16 ms by default on Linux.
const FTDI_DEFAULT_LATENCY_US: u32 = 16_000;

/// The likely causes of a missing reply, naming the adapter latency timer
/// when the timeout is not above the FTDI default.
struct SilenceHint(u32);

impl fmt::Display for SilenceHint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("; check power, wiring, the baud rate and the ID")?;
        if self.0 <= FTDI_DEFAULT_LATENCY_US {
            write!(
                f,
                ", and that the USB adapter's latency timer is below {} (FTDI default: 16 ms)",
                Ms(self.0)
            )?;
        }
        Ok(())
    }
}

/// The transport call that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Direction {
    /// [`Write::write`](embedded_io::Write::write).
    Write,
    /// [`Write::flush`](embedded_io::Write::flush).
    Flush,
    /// [`ReadReady::read_ready`](embedded_io::ReadReady::read_ready).
    ReadReady,
    /// [`Read::read`](embedded_io::Read::read).
    Read,
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Write => "writing",
            Self::Flush => "flushing",
            Self::ReadReady => "checking for input",
            Self::Read => "reading",
        })
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
///         | V::NotFinite { .. }
///         | V::DoesNotFit { .. }
///         | V::BufferLength { .. }
///         | V::DuplicateId { .. }
///         | V::NotPositionMode { .. }
///         | V::NotVelocityMode { .. }
///         | V::NotCurrentMode { .. }
///         | V::Unsupported { .. }
///         | V::NoUnit { .. }
///         | V::UnsupportedInstruction { .. }
///         | V::InvalidSlopeStep { .. } => false,
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
    #[error(
        "address {addr} does not fit Protocol 1's one-byte address field{}",
        In(request)
    )]
    AddressOutOfRange {
        /// The address that was asked for.
        addr: u16,
        /// The refused request.
        request: Request,
    },

    /// The packet's length field cannot describe `len` bytes; `max` is the
    /// protocol's limit.
    #[error(
        "packet of {len} bytes exceeds the protocol limit of {max}{}",
        In(request)
    )]
    PacketTooLarge {
        /// The length that was asked for, in bytes.
        len: usize,
        /// The largest length the protocol allows, in bytes.
        max: usize,
        /// The refused request.
        request: Request,
    },

    /// A data slice does not have the size the instruction declares.
    #[error("data is {got} bytes, expected {expected}{}", In(request))]
    DataLength {
        /// The size the instruction declares, in bytes.
        expected: usize,
        /// The size of the slice that was passed, in bytes.
        got: usize,
        /// The refused request; `id` is the entry with the wrong size.
        request: Request,
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

    /// A value is NaN or infinite, so no raw value matches it.
    #[error("value is not a finite number{}", In(request))]
    NotFinite {
        /// The refused request.
        request: Request,
    },

    /// A value does not fit a register of `size` bytes, or, when `size` is
    /// 0, does not fit an `i32`. Values are never truncated.
    #[error("value {raw} does not fit {}{}", FitsIn(*size), In(request))]
    DoesNotFit {
        /// The value that does not fit.
        raw: i64,
        /// The register size in bytes; `size` 0 = does not fit `i32`.
        size: u8,
        /// The refused request.
        request: Request,
    },

    /// A caller buffer or value slice does not have one slot per target.
    #[error("buffer has {got} slots for {targets} targets{}", In(request))]
    BufferLength {
        /// The number of target actuators.
        targets: usize,
        /// The number of slots that were passed.
        got: usize,
        /// The refused request.
        request: Request,
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

    /// The actuator's operating mode ignores Goal Velocity: only velocity
    /// mode (Protocol 2) and wheel mode (Protocol 1) use it.
    #[error(
        "actuator {} is in {mode:?} mode; only velocity or wheel mode uses a goal velocity",
        id.get()
    )]
    NotVelocityMode {
        /// The actuator the goal was for.
        id: Id,
        /// The mode recorded at Setup.
        mode: LimitMode,
    },

    /// The actuator's operating mode ignores Goal Current: only current and
    /// current-based position mode use it.
    #[error(
        "actuator {} is in {mode:?} mode; only current or current-based position mode uses a goal current",
        id.get()
    )]
    NotCurrentMode {
        /// The actuator the goal was for.
        id: Id,
        /// The mode recorded at Setup.
        mode: LimitMode,
    },

    /// The actuator's model has no such register, so a typed call for it
    /// sends nothing.
    #[error("actuator {} has no {register} register", id.get())]
    Unsupported {
        /// The actuator the call was for.
        id: Id,
        /// The register's name, as the control table writes it.
        register: &'static str,
    },

    /// A compliance slope step outside the e-Manual's `1..=7`.
    #[error("compliance slope step {step} is outside 1..=7{}", In(request))]
    InvalidSlopeStep {
        /// The step that was asked for.
        step: u8,
        /// The refused request.
        request: Request,
    },

    /// The register has no typed form in the actuator's operating mode:
    /// its control table gives no unit for that mode, or one that is not
    /// the value's quantity (AX-12A wheel-mode Moving Speed is a share of
    /// output in %, not a velocity). The raw form still works.
    #[error(
        "{register} of actuator {} has no typed form in {mode:?} mode; use its raw form",
        id.get()
    )]
    NoUnit {
        /// The actuator the call was for.
        id: Id,
        /// The register's name, as the control table writes it.
        register: &'static str,
        /// The mode recorded at Setup.
        mode: LimitMode,
    },

    /// The actuator's control table does not list the instruction for the
    /// bus protocol (AX and RX have no Protocol 1 Bulk Read), so nothing is
    /// sent.
    #[error("actuator {} does not support the {instruction} instruction", id.get())]
    UnsupportedInstruction {
        /// The actuator the call was for.
        id: Id,
        /// The instruction's e-Manual name, e.g. `"Bulk Read"`.
        instruction: &'static str,
    },
}

impl ValidationError {
    /// A [`NotFinite`](Self::NotFinite) with no context yet.
    pub(crate) const NOT_FINITE: Self = Self::NotFinite {
        request: Request::NONE,
    };

    /// The refused request, for the variants that carry one.
    pub const fn request(&self) -> Option<&Request> {
        match self {
            Self::AddressOutOfRange { request, .. }
            | Self::PacketTooLarge { request, .. }
            | Self::DataLength { request, .. }
            | Self::NotFinite { request }
            | Self::DoesNotFit { request, .. }
            | Self::BufferLength { request, .. }
            | Self::InvalidSlopeStep { request, .. } => Some(request),
            _ => None,
        }
    }

    const fn request_mut(&mut self) -> Option<&mut Request> {
        match self {
            Self::AddressOutOfRange { request, .. }
            | Self::PacketTooLarge { request, .. }
            | Self::DataLength { request, .. }
            | Self::NotFinite { request }
            | Self::DoesNotFit { request, .. }
            | Self::BufferLength { request, .. }
            | Self::InvalidSlopeStep { request, .. } => Some(request),
            _ => None,
        }
    }

    /// Fills the unknown fields of the refused request from `outer`.
    pub(crate) fn within(mut self, outer: Request) -> Self {
        if let Some(request) = self.request_mut() {
            request.fill(outer);
        }
        self
    }
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

    /// More actuators answered, or were declared, than the bus can hold.
    #[error(
        "{} actuators {} ({}), more than the bus capacity of {capacity}",
        ids.len(),
        if *declared { "were declared" } else { "answered" },
        IdList(*ids)
    )]
    TooManyActuators {
        /// The number of actuators the bus can hold.
        capacity: usize,
        /// Every ID that answered or was declared.
        ids: IdSet,
        /// Whether the IDs come from a declared list rather than a scan.
        declared: bool,
    },

    /// A declared actuator sent no byte to its Ping within the timeout.
    #[error(
        "declared actuator {} sent nothing within {} of its Ping{}",
        id.get(),
        Ms(*timeout_us),
        SilenceHint(*timeout_us)
    )]
    Missing {
        /// The declared actuator.
        id: Id,
        /// How long each attempt waited, in microseconds.
        timeout_us: u32,
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
    /// none of its model's value lists, or its MX-64 Protocol 1 Torque Ctrl
    /// Mode Enable value is outside 0 and 1; both come from its control
    /// table, and no mode is guessed (decided 2026-10-28).
    #[error(
        "actuator {} reports unknown {register} value {value}",
        id.get()
    )]
    UnknownOperatingMode {
        /// The actuator that reported the value.
        id: Id,
        /// The mode register value it reported.
        value: u8,
        /// The mode register's name, as the control table writes it.
        register: &'static str,
    },
}

/// "1, 2, 5".
struct IdList(IdSet);

impl fmt::Display for IdList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ids = self.0.into_iter();
        if let Some(first) = ids.next() {
            write!(f, "ids {}", first.get())?;
            ids.try_for_each(|id| write!(f, ", {}", id.get()))?;
        }
        Ok(())
    }
}

/// The serial line failed, or a reply did not arrive in time.
///
/// Each variant says what was waited for or written ([`Request`]) and, for
/// a wait, how long and how many bytes arrived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum TransportError {
    /// Not one byte arrived before the deadline, and none in one more
    /// timeout after it.
    #[error(
        "no reply within {} to {request}{}",
        Ms(*timeout_us),
        SilenceHint(*timeout_us)
    )]
    NoReply {
        /// What was waited for.
        request: Request,
        /// How long the wait was, in microseconds.
        timeout_us: u32,
    },

    /// Nothing arrived before the deadline, but `bytes` bytes arrived in
    /// the timeout after it: the reply was late, not missing. A USB
    /// adapter's latency timer above the timeout does this.
    #[error(
        "the reply to {request} came late: nothing within the {} timeout, then {bytes} bytes; \
         set the USB adapter's latency timer to 1 ms (FTDI default: 16 ms) or raise the bus timeout",
        Ms(*timeout_us)
    )]
    LateReply {
        /// What was waited for.
        request: Request,
        /// How long the wait was, in microseconds.
        timeout_us: u32,
        /// How many bytes arrived after the deadline.
        bytes: usize,
    },

    /// Part of a status packet arrived, then the line went quiet until the
    /// deadline.
    #[error(
        "only {bytes} bytes of the reply to {request} arrived within {}; \
         a USB adapter latency timer above the timeout or a noisy line cuts replies short",
        Ms(*timeout_us)
    )]
    PartialReply {
        /// What was waited for.
        request: Request,
        /// How long the wait was, in microseconds.
        timeout_us: u32,
        /// How many bytes arrived before the deadline.
        bytes: usize,
    },

    /// In the replies to a Sync Read or Bulk Read, the actuator after this
    /// one in the list answered first: this one sent nothing in its turn.
    #[error(
        "actuator {} sent nothing in its turn of {request}; actuator {}, later in the list, answered in its place",
        request.id.map_or(0, Id::get),
        answered.get()
    )]
    Skipped {
        /// The skipped target's request.
        request: Request,
        /// The later target whose reply came instead.
        answered: Id,
    },

    /// The transport's own timeout expired (an OS serial port's read or
    /// write timeout), not the bus's.
    #[error("the serial port's own timeout expired while {direction}, in {request}")]
    PortTimeout {
        /// What was being sent or waited for.
        request: Request,
        /// The transport call that timed out.
        direction: Direction,
    },

    /// A read returned no byte after the port said input was waiting: the
    /// port is gone (a USB adapter unplugged).
    #[error("the serial port closed (end of stream) while reading, in {request}")]
    EndOfStream {
        /// What was waited for.
        request: Request,
    },

    /// The transport returned an error other than a timeout.
    /// `embedded-io` gives only the error's kind, not the OS code.
    #[error("the serial port failed while {direction} ({kind:?}), in {request}")]
    Io {
        /// What was being sent or waited for.
        request: Request,
        /// The transport call that failed.
        direction: Direction,
        /// The `embedded-io` kind of the error.
        kind: embedded_io::ErrorKind,
    },
}

impl TransportError {
    /// What was being sent or waited for.
    pub const fn request(&self) -> &Request {
        match self {
            Self::NoReply { request, .. }
            | Self::LateReply { request, .. }
            | Self::PartialReply { request, .. }
            | Self::Skipped { request, .. }
            | Self::PortTimeout { request, .. }
            | Self::EndOfStream { request }
            | Self::Io { request, .. } => request,
        }
    }

    /// Whether the wait for a reply ran out: no reply, a late one, a
    /// partial one, a skipped turn, or the port's own timeout. Such an exchange may be
    /// tried again; a broken line may not.
    pub const fn is_timeout(&self) -> bool {
        matches!(
            self,
            Self::NoReply { .. }
                | Self::LateReply { .. }
                | Self::PartialReply { .. }
                | Self::Skipped { .. }
                | Self::PortTimeout { .. }
        )
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
    #[error(
        "status packet with id byte {packet_id} failed its checksum \
         (computed {computed:#04X}, received {received:#04X}){}",
        ReplyTo(request)
    )]
    Checksum {
        /// What the reply answered.
        request: Request,
        /// The raw ID byte of the packet, possibly corrupted itself.
        packet_id: u8,
        /// The checksum of the bytes received.
        computed: u8,
        /// The checksum byte the packet carried.
        received: u8,
    },

    /// A Protocol 2 status packet failed its CRC.
    #[error(
        "status packet with id byte {packet_id} failed its CRC \
         (computed {computed:#06X}, received {received:#06X}){}",
        ReplyTo(request)
    )]
    Crc {
        /// What the reply answered.
        request: Request,
        /// The raw ID byte of the packet, possibly corrupted itself.
        packet_id: u8,
        /// The CRC of the bytes received.
        computed: u16,
        /// The CRC the packet carried.
        received: u16,
    },

    /// A valid status packet came from another actuator than the one asked.
    #[error("the reply to {request} came from id {got}, not actuator {}", expected.get())]
    WrongId {
        /// What was asked.
        request: Request,
        /// The actuator the instruction was sent to.
        expected: Id,
        /// The raw ID byte of the reply.
        got: u8,
    },

    /// A valid status packet carried a different amount of data than asked.
    #[error("the reply to {request} carried {got} data bytes, expected {expected}")]
    WrongLength {
        /// What was asked.
        request: Request,
        /// The number of data bytes asked for.
        expected: usize,
        /// The number of data bytes the reply carried.
        got: usize,
    },

    /// A status packet header whose length field is below the smallest
    /// valid packet (Protocol 1: 2, Protocol 2: 3). It was skipped, and no
    /// valid packet followed before the deadline.
    #[error(
        "{protocol} status packet header with length field {length}, below the minimum {min}{}",
        ReplyTo(request)
    )]
    LengthTooShort {
        /// What the reply answered.
        request: Request,
        /// The protocol of the packet.
        protocol: ProtocolVersion,
        /// The length field it carried.
        length: u16,
        /// The smallest valid length field.
        min: u16,
    },

    /// A Protocol 2 status packet header whose length field is longer than
    /// the expected reply can be: a corrupted length that would otherwise
    /// swallow the packets after it. It was skipped, and no valid packet
    /// followed before the deadline.
    #[error(
        "Protocol 2 status packet header with length field {length}, above the {max} the reply can have{}",
        ReplyTo(request)
    )]
    LengthTooLong {
        /// What the reply answered.
        request: Request,
        /// The length field it carried.
        length: u16,
        /// The longest length field the expected reply can have.
        max: u16,
    },

    /// A reply to a broadcast Ping carries an ID no actuator can have.
    #[error("a reply to {request} came from id {got}, outside 0..=252")]
    InvalidReplyId {
        /// What was asked.
        request: Request,
        /// The raw ID byte of the reply.
        got: u8,
    },
}

impl ProtocolError {
    /// What the bad reply answered.
    pub const fn request(&self) -> &Request {
        match self {
            Self::Checksum { request, .. }
            | Self::Crc { request, .. }
            | Self::WrongId { request, .. }
            | Self::WrongLength { request, .. }
            | Self::LengthTooShort { request, .. }
            | Self::LengthTooLong { request, .. }
            | Self::InvalidReplyId { request, .. } => request,
        }
    }
}

/// A voltage in tenths of a volt, the unit of every Dynamixel voltage
/// register: `Decivolts(97)` is 9.7 V.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Decivolts(pub u16);

impl fmt::Display for Decivolts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{} V", self.0 / 10, self.0 % 10)
    }
}

/// A temperature in whole degrees Celsius, the unit of every Dynamixel
/// temperature register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Celsius(pub i16);

impl fmt::Display for Celsius {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} °C", self.0)
    }
}

/// "; it also reports <the other faults>" when more than `primary` is set.
struct Also(HardwareErrors, u8);

impl fmt::Display for Also {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let others = self.0.without(self.1);
        if others.is_empty() {
            Ok(())
        } else {
            write!(f, "; it also reports {others}")
        }
    }
}

/// A hardware fault an actuator reported, measured against its limits.
///
/// A reply's error byte says only that a fault exists (Protocol 1: Input
/// Voltage, Overheating, Overload; Protocol 2: the alert bit). When a reply
/// reports one, the bus reads Hardware Error Status (Protocol 2) and the
/// present value and EEPROM limits of the fault's quantity, and returns
/// this instead of the bare [`StatusError`]. With several faults at once,
/// the variant is the first of input voltage, temperature, overload,
/// electrical shock, encoder; `faults` holds them all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum FaultError {
    /// The input voltage is outside, or was outside, the voltage limits.
    #[error(transparent)]
    Voltage(#[from] VoltageError),

    /// The temperature is above, or was above, the temperature limit.
    #[error(transparent)]
    Temperature(#[from] TemperatureError),

    /// The load stayed above the maximum output.
    #[error(
        "actuator {} reports an overload: the load stays above what it can hold{}{}",
        id.get(),
        Also(*faults, HardwareErrors::OVERLOAD),
        ReplyTo(request)
    )]
    Overload {
        /// The actuator.
        id: Id,
        /// Every fault it reports.
        faults: HardwareErrors,
        /// The request whose reply reported it.
        request: Request,
    },

    /// An electrical shock on the circuit, or too little power for the
    /// motor (Protocol 2 Hardware Error Status bit 4).
    #[error(
        "actuator {} reports an electrical shock, or too little power for its motor{}{}",
        id.get(),
        Also(*faults, HardwareErrors::ELECTRICAL_SHOCK),
        ReplyTo(request)
    )]
    ElectricalShock {
        /// The actuator.
        id: Id,
        /// Every fault it reports.
        faults: HardwareErrors,
        /// The request whose reply reported it.
        request: Request,
    },

    /// The motor encoder malfunctions (Protocol 2 Hardware Error Status
    /// bit 3).
    #[error(
        "actuator {} reports a motor encoder malfunction{}{}",
        id.get(),
        Also(*faults, HardwareErrors::ENCODER),
        ReplyTo(request)
    )]
    Encoder {
        /// The actuator.
        id: Id,
        /// Every fault it reports.
        faults: HardwareErrors,
        /// The request whose reply reported it.
        request: Request,
    },

    /// The fault's quantity cannot be measured: the model's table has no
    /// such register (Y series keeps motor and inverter temperatures
    /// apart).
    #[error(
        "actuator {} reports {faults}, but its model has no {register} register to measure it{}",
        id.get(),
        ReplyTo(request)
    )]
    Unmeasurable {
        /// The actuator.
        id: Id,
        /// Every fault it reports.
        faults: HardwareErrors,
        /// The register the measurement needs.
        register: &'static str,
        /// The request whose reply reported it.
        request: Request,
    },

    /// A Protocol 2 reply raised the alert bit, but Hardware Error Status
    /// reads no fault (it cleared, or its bits are unused ones).
    #[error(
        "actuator {} raised a hardware alert, but its Hardware Error Status reads {:#04X}, no known fault{}",
        id.get(),
        faults.raw(),
        ReplyTo(request)
    )]
    Unidentified {
        /// The actuator.
        id: Id,
        /// What Hardware Error Status read.
        faults: HardwareErrors,
        /// The request whose reply raised the alert.
        request: Request,
    },
}

impl FaultError {
    /// The actuator that reports the fault.
    pub const fn id(&self) -> Id {
        match self {
            Self::Voltage(err) => err.id(),
            Self::Temperature(err) => err.id(),
            Self::Overload { id, .. }
            | Self::ElectricalShock { id, .. }
            | Self::Encoder { id, .. }
            | Self::Unmeasurable { id, .. }
            | Self::Unidentified { id, .. } => *id,
        }
    }

    /// Every fault the actuator reports.
    pub const fn faults(&self) -> HardwareErrors {
        match self {
            Self::Voltage(err) => err.faults(),
            Self::Temperature(err) => err.faults(),
            Self::Overload { faults, .. }
            | Self::ElectricalShock { faults, .. }
            | Self::Encoder { faults, .. }
            | Self::Unmeasurable { faults, .. }
            | Self::Unidentified { faults, .. } => *faults,
        }
    }

    /// The request whose reply reported the fault.
    pub const fn request(&self) -> &Request {
        match self {
            Self::Voltage(err) => err.request(),
            Self::Temperature(err) => err.request(),
            Self::Overload { request, .. }
            | Self::ElectricalShock { request, .. }
            | Self::Encoder { request, .. }
            | Self::Unmeasurable { request, .. }
            | Self::Unidentified { request, .. } => request,
        }
    }
}

/// An input voltage fault, measured after the reply that reported it: the
/// present voltage against Min and Max Voltage Limit from the actuator's
/// EEPROM.
///
/// ```
/// use dxl_rs_v2::{Decivolts, Error, FaultError, HardwareErrors, Id, Request, VoltageError};
///
/// let err = Error::from(FaultError::from(VoltageError::BelowMin {
///     id: Id::new(5)?,
///     measured: Decivolts(97),
///     min: Decivolts(105),
///     max: Decivolts(130),
///     faults: HardwareErrors::from_raw(1),
///     request: Request::NONE,
/// }));
/// assert_eq!(
///     err.to_string(),
///     "actuator 5 measures 9.7 V input, below its Min Voltage Limit 10.5 V \
///      (limits 10.5 V to 13.0 V); check the supply and the voltage drop along the wiring"
/// );
/// # Ok::<(), Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum VoltageError {
    /// The present input voltage is below Min Voltage Limit.
    #[error(
        "actuator {} measures {measured} input, below its Min Voltage Limit {min} \
         (limits {min} to {max}); check the supply and the voltage drop along the wiring{}{}",
        id.get(),
        Also(*faults, HardwareErrors::INPUT_VOLTAGE),
        ReplyTo(request)
    )]
    BelowMin {
        /// The actuator.
        id: Id,
        /// Present (Input) Voltage, read after the fault.
        measured: Decivolts,
        /// Min Voltage Limit.
        min: Decivolts,
        /// Max Voltage Limit.
        max: Decivolts,
        /// Every fault it reports.
        faults: HardwareErrors,
        /// The request whose reply reported it.
        request: Request,
    },

    /// The present input voltage is above Max Voltage Limit.
    #[error(
        "actuator {} measures {measured} input, above its Max Voltage Limit {max} \
         (limits {min} to {max}); lower the supply voltage{}{}",
        id.get(),
        Also(*faults, HardwareErrors::INPUT_VOLTAGE),
        ReplyTo(request)
    )]
    AboveMax {
        /// The actuator.
        id: Id,
        /// Present (Input) Voltage, read after the fault.
        measured: Decivolts,
        /// Min Voltage Limit.
        min: Decivolts,
        /// Max Voltage Limit.
        max: Decivolts,
        /// Every fault it reports.
        faults: HardwareErrors,
        /// The request whose reply reported it.
        request: Request,
    },

    /// The actuator reported an input voltage error, but the voltage read
    /// after it is inside the limits: it dipped or spiked briefly (a load
    /// step on a weak supply, long thin wiring).
    #[error(
        "actuator {} reported an input voltage error, but measures {measured}, inside its limits \
         {min} to {max}: the voltage dipped or spiked briefly{}{}",
        id.get(),
        Also(*faults, HardwareErrors::INPUT_VOLTAGE),
        ReplyTo(request)
    )]
    InsideLimits {
        /// The actuator.
        id: Id,
        /// Present (Input) Voltage, read after the fault.
        measured: Decivolts,
        /// Min Voltage Limit.
        min: Decivolts,
        /// Max Voltage Limit.
        max: Decivolts,
        /// Every fault it reports.
        faults: HardwareErrors,
        /// The request whose reply reported it.
        request: Request,
    },
}

impl VoltageError {
    /// Classifies `measured` against `min..=max`.
    pub(crate) const fn classify(
        id: Id,
        measured: Decivolts,
        min: Decivolts,
        max: Decivolts,
        faults: HardwareErrors,
        request: Request,
    ) -> Self {
        if measured.0 < min.0 {
            Self::BelowMin {
                id,
                measured,
                min,
                max,
                faults,
                request,
            }
        } else if measured.0 > max.0 {
            Self::AboveMax {
                id,
                measured,
                min,
                max,
                faults,
                request,
            }
        } else {
            Self::InsideLimits {
                id,
                measured,
                min,
                max,
                faults,
                request,
            }
        }
    }

    /// The actuator.
    pub const fn id(&self) -> Id {
        match self {
            Self::BelowMin { id, .. }
            | Self::AboveMax { id, .. }
            | Self::InsideLimits { id, .. } => *id,
        }
    }

    /// Every fault the actuator reports.
    pub const fn faults(&self) -> HardwareErrors {
        match self {
            Self::BelowMin { faults, .. }
            | Self::AboveMax { faults, .. }
            | Self::InsideLimits { faults, .. } => *faults,
        }
    }

    /// The request whose reply reported the fault.
    pub const fn request(&self) -> &Request {
        match self {
            Self::BelowMin { request, .. }
            | Self::AboveMax { request, .. }
            | Self::InsideLimits { request, .. } => request,
        }
    }
}

/// An overheating fault, measured after the reply that reported it: the
/// present temperature against Temperature Limit from the actuator's
/// EEPROM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum TemperatureError {
    /// The present temperature is above Temperature Limit.
    #[error(
        "actuator {} measures {measured}, above its Temperature Limit {limit}; \
         let it cool and reduce its load{}{}",
        id.get(),
        Also(*faults, HardwareErrors::OVERHEATING),
        ReplyTo(request)
    )]
    AboveLimit {
        /// The actuator.
        id: Id,
        /// Present Temperature, read after the fault.
        measured: Celsius,
        /// Temperature Limit.
        limit: Celsius,
        /// Every fault it reports.
        faults: HardwareErrors,
        /// The request whose reply reported it.
        request: Request,
    },

    /// The actuator reported overheating, but the temperature read after it
    /// is at or below the limit: it has cooled since, and the fault stays
    /// latched until cleared.
    #[error(
        "actuator {} reported overheating, but measures {measured}, at or below its \
         Temperature Limit {limit}: it has cooled since{}{}",
        id.get(),
        Also(*faults, HardwareErrors::OVERHEATING),
        ReplyTo(request)
    )]
    WithinLimit {
        /// The actuator.
        id: Id,
        /// Present Temperature, read after the fault.
        measured: Celsius,
        /// Temperature Limit.
        limit: Celsius,
        /// Every fault it reports.
        faults: HardwareErrors,
        /// The request whose reply reported it.
        request: Request,
    },
}

impl TemperatureError {
    /// Classifies `measured` against `limit`.
    pub(crate) const fn classify(
        id: Id,
        measured: Celsius,
        limit: Celsius,
        faults: HardwareErrors,
        request: Request,
    ) -> Self {
        if measured.0 > limit.0 {
            Self::AboveLimit {
                id,
                measured,
                limit,
                faults,
                request,
            }
        } else {
            Self::WithinLimit {
                id,
                measured,
                limit,
                faults,
                request,
            }
        }
    }

    /// The actuator.
    pub const fn id(&self) -> Id {
        match self {
            Self::AboveLimit { id, .. } | Self::WithinLimit { id, .. } => *id,
        }
    }

    /// Every fault the actuator reports.
    pub const fn faults(&self) -> HardwareErrors {
        match self {
            Self::AboveLimit { faults, .. } | Self::WithinLimit { faults, .. } => *faults,
        }
    }

    /// The request whose reply reported the fault.
    pub const fn request(&self) -> &Request {
        match self {
            Self::AboveLimit { request, .. } | Self::WithinLimit { request, .. } => request,
        }
    }
}

/// An actuator's status packet reported an error: a refused instruction,
/// or (before the bus measures it into a [`FaultError`]) a hardware fault.
///
/// `request` names the instruction, register and address the reply
/// answered, so a P2 `DataRange` or P1 `Range` says which write was
/// refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("actuator {} reported {report}{}", id.get(), ReplyTo(request))]
#[non_exhaustive]
pub struct StatusError {
    /// The actuator that reported it.
    pub id: Id,
    /// What its error byte says, decoded per protocol.
    pub report: StatusReport,
    /// The request the reply answered.
    pub request: Request,
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
            StatusReport::P1(flags) => !flags.hardware().is_empty(),
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

/// What to check after the actuator saw a corrupted instruction packet.
const CORRUPTED: &str = "the instruction packet it received was corrupted on the line \
     (check wiring, connectors, supply noise and the baud rate)";

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
            Self::ResultFail => f.write_str("result fail (it could not process the instruction)"),
            Self::Instruction => f.write_str(
                "an instruction error (undefined instruction, or Action without Reg Write)",
            ),
            Self::Crc => write!(f, "a CRC error: {CORRUPTED}"),
            Self::DataRange => {
                f.write_str("a data range error (the value is outside the register's range)")
            }
            Self::DataLength => {
                f.write_str("a data length error (the data is shorter than the register)")
            }
            Self::DataLimit => {
                f.write_str("a data limit error (the value is outside its configured limit)")
            }
            Self::Access => f.write_str(
                "an access error (read-only, write-only or undefined address, \
                 or an EEPROM write while torque is on)",
            ),
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
}

impl fmt::Display for P1Flag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputVoltage => {
                f.write_str("an input voltage error (supply outside its voltage limits)")
            }
            Self::AngleLimit => {
                f.write_str("an angle limit error (goal position outside its CW/CCW angle limits)")
            }
            Self::Overheating => {
                f.write_str("overheating (temperature above its Temperature Limit)")
            }
            Self::Range => f.write_str("a range error (a value outside the register's range)"),
            Self::Checksum => write!(f, "a checksum error: {CORRUPTED}"),
            Self::Overload => f.write_str("an overload (load above its maximum torque)"),
            Self::Instruction => f.write_str(
                "an instruction error (undefined instruction, or Action without Reg Write)",
            ),
        }
    }
}

/// A Protocol 1 error byte: any combination of [`P1Flag`]s. `Debug` names
/// the set flags.
#[derive(Clone, Copy, PartialEq, Eq)]
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

    /// The hardware fault flags (Input Voltage, Overheating, Overload), as
    /// the matching Hardware Error Status bits, which sit at the same
    /// positions.
    pub const fn hardware(self) -> HardwareErrors {
        HardwareErrors::from_raw(
            self.0
                & (HardwareErrors::INPUT_VOLTAGE
                    | HardwareErrors::OVERHEATING
                    | HardwareErrors::OVERLOAD),
        )
    }
}

impl fmt::Debug for P1Flags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = f.debug_list();
        list.entries(self.iter());
        let unknown = self.0 & !P1Flag::ALL.iter().fold(0, |bits, flag| bits | flag.bit());
        if unknown != 0 {
            list.entry(&format_args!("unknown bits {unknown:#04X}"));
        }
        list.finish()
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
    use std::format;
    use std::string::{String, ToString};

    fn id(raw: u8) -> Id {
        Id::new(raw).unwrap()
    }

    /// The setup read of Model Number on actuator 1.
    fn model_number_read() -> Request {
        Request::NONE
            .instruction(Instruction::Read)
            .id(id(1))
            .register("Model Number")
            .addr(0)
            .len(2)
            .step(Step::ModelNumber)
    }

    fn text(err: impl Into<Error>) -> String {
        err.into().to_string()
    }

    #[test]
    fn request_names_what_it_knows() {
        assert_eq!(Request::NONE.to_string(), "the request");
        assert_eq!(
            model_number_read().to_string(),
            "Read of Model Number (address 0, 2 bytes) from actuator 1, \
             during setup, reading the model number"
        );
        let write = Request::NONE
            .instruction(Instruction::Write)
            .id(id(3))
            .addr(30)
            .len(1);
        assert_eq!(
            write.to_string(),
            "Write (address 30, 1 byte) to actuator 3"
        );
        let sync = Request::NONE
            .instruction(Instruction::SyncWrite)
            .register("Goal Position")
            .addr(116)
            .len(4);
        assert_eq!(
            sync.to_string(),
            "Sync Write of Goal Position (address 116, 4 bytes) (broadcast)"
        );
        assert_eq!(Request::NONE.register("LED").to_string(), "LED");
        assert_eq!(Request::NONE.id(id(4)).to_string(), "on actuator 4");
        assert_eq!(Request::NONE.addr(7).to_string(), "(address 7)");
        assert_eq!(Request::NONE.len(3).to_string(), "(3 bytes)");
        assert_eq!(
            Request::NONE.step(Step::Scan).to_string(),
            "during setup, scanning for actuators"
        );
        assert_eq!(
            Request::NONE.instruction(Instruction::Ping).to_string(),
            "Ping"
        );
    }

    #[test]
    fn request_fill_keeps_known_fields() {
        let mut inner = Request::NONE.instruction(Instruction::Read).id(id(1));
        inner.fill(
            Request::NONE
                .instruction(Instruction::Write)
                .register("Goal Position")
                .step(Step::PositionLimits),
        );
        assert_eq!(inner.instruction, Some(Instruction::Read));
        assert_eq!(inner.register, Some("Goal Position"));
        assert_eq!(inner.step, Some(Step::PositionLimits));
        assert!(Request::NONE.is_empty());
        assert!(!inner.is_empty());
        assert_eq!(Request::len_of(70_000), u16::MAX);
    }

    #[test]
    fn every_instruction_and_step_has_a_name() {
        let names: std::vec::Vec<_> = [
            Instruction::Ping,
            Instruction::Read,
            Instruction::Write,
            Instruction::Reboot,
            Instruction::SyncRead,
            Instruction::SyncWrite,
            Instruction::BulkRead,
            Instruction::BulkWrite,
        ]
        .iter()
        .map(ToString::to_string)
        .collect();
        assert_eq!(
            names,
            [
                "Ping",
                "Read",
                "Write",
                "Reboot",
                "Sync Read",
                "Sync Write",
                "Bulk Read",
                "Bulk Write"
            ]
        );
        for step in [
            Step::Scan,
            Step::Ping,
            Step::ModelNumber,
            Step::Firmware,
            Step::PositionLimits,
            Step::OperatingMode,
            Step::GoalBound,
            Step::FaultDiagnosis,
        ] {
            assert!(!step.to_string().is_empty());
        }
        for (direction, name) in [
            (Direction::Write, "writing"),
            (Direction::Flush, "flushing"),
            (Direction::ReadReady, "checking for input"),
            (Direction::Read, "reading"),
        ] {
            assert_eq!(direction.to_string(), name);
        }
    }

    #[test]
    fn milliseconds_keep_their_fraction() {
        assert_eq!(Ms(10_000).to_string(), "10 ms");
        assert_eq!(Ms(1_500).to_string(), "1.5 ms");
        assert_eq!(Ms(250).to_string(), "0.25 ms");
        assert_eq!(Ms(16_001).to_string(), "16.001 ms");
    }

    #[test]
    fn transport_messages_name_request_timeout_and_bytes() {
        let request = model_number_read();
        let no_reply = text(TransportError::NoReply {
            request,
            timeout_us: 10_000,
        });
        assert_eq!(
            no_reply,
            "no reply within 10 ms to Read of Model Number (address 0, 2 bytes) from actuator 1, \
             during setup, reading the model number; check power, wiring, the baud rate and the ID, \
             and that the USB adapter's latency timer is below 10 ms (FTDI default: 16 ms)"
        );
        assert!(
            !text(TransportError::NoReply {
                request,
                timeout_us: 50_000,
            })
            .contains("latency")
        );
        let late = text(TransportError::LateReply {
            request,
            timeout_us: 10_000,
            bytes: 8,
        });
        assert!(late.starts_with("the reply to Read of Model Number"));
        assert!(late.contains("nothing within the 10 ms timeout, then 8 bytes"));
        assert!(late.contains("latency timer to 1 ms"));
        let partial = text(TransportError::PartialReply {
            request,
            timeout_us: 10_000,
            bytes: 3,
        });
        assert!(partial.starts_with("only 3 bytes of the reply to Read of Model Number"));
        assert!(partial.contains("within 10 ms"));
        let skipped = text(TransportError::Skipped {
            request: Request::NONE.instruction(Instruction::SyncRead).id(id(3)),
            answered: id(4),
        });
        assert_eq!(
            skipped,
            "actuator 3 sent nothing in its turn of Sync Read from actuator 3; \
             actuator 4, later in the list, answered in its place"
        );
        assert_eq!(
            text(TransportError::PortTimeout {
                request,
                direction: Direction::Write,
            }),
            format!("the serial port's own timeout expired while writing, in {request}")
        );
        assert_eq!(
            text(TransportError::EndOfStream { request }),
            format!("the serial port closed (end of stream) while reading, in {request}")
        );
        assert_eq!(
            text(TransportError::Io {
                request,
                direction: Direction::Flush,
                kind: embedded_io::ErrorKind::BrokenPipe,
            }),
            format!("the serial port failed while flushing (BrokenPipe), in {request}")
        );
    }

    #[test]
    fn timeouts_are_retryable_and_broken_lines_are_not() {
        let request = Request::NONE;
        for err in [
            TransportError::NoReply {
                request,
                timeout_us: 1,
            },
            TransportError::LateReply {
                request,
                timeout_us: 1,
                bytes: 1,
            },
            TransportError::PartialReply {
                request,
                timeout_us: 1,
                bytes: 1,
            },
            TransportError::Skipped {
                request,
                answered: id(2),
            },
            TransportError::PortTimeout {
                request,
                direction: Direction::Read,
            },
        ] {
            assert!(err.is_timeout(), "{err:#?}");
            assert_eq!(*err.request(), request);
        }
        assert!(!TransportError::EndOfStream { request }.is_timeout());
        assert!(
            !TransportError::Io {
                request,
                direction: Direction::Read,
                kind: embedded_io::ErrorKind::Other,
            }
            .is_timeout()
        );
    }

    #[test]
    fn protocol_messages_name_every_field() {
        let request = model_number_read();
        let cases = [
            (
                ProtocolError::Checksum {
                    request,
                    packet_id: 1,
                    computed: 0xAB,
                    received: 0xCD,
                },
                format!(
                    "status packet with id byte 1 failed its checksum (computed 0xAB, received 0xCD) \
                     in reply to {request}"
                ),
            ),
            (
                ProtocolError::Crc {
                    request,
                    packet_id: 2,
                    computed: 0x1234,
                    received: 0xBEEF,
                },
                format!(
                    "status packet with id byte 2 failed its CRC (computed 0x1234, received 0xBEEF) \
                     in reply to {request}"
                ),
            ),
            (
                ProtocolError::WrongId {
                    request,
                    expected: id(1),
                    got: 7,
                },
                format!("the reply to {request} came from id 7, not actuator 1"),
            ),
            (
                ProtocolError::WrongLength {
                    request,
                    expected: 2,
                    got: 0,
                },
                format!("the reply to {request} carried 0 data bytes, expected 2"),
            ),
            (
                ProtocolError::LengthTooShort {
                    request,
                    protocol: ProtocolVersion::P1,
                    length: 1,
                    min: 2,
                },
                format!(
                    "Protocol 1 status packet header with length field 1, below the minimum 2 \
                     in reply to {request}"
                ),
            ),
            (
                ProtocolError::LengthTooLong {
                    request,
                    length: 400,
                    max: 7,
                },
                format!(
                    "Protocol 2 status packet header with length field 400, above the 7 the reply \
                     can have in reply to {request}"
                ),
            ),
            (
                ProtocolError::InvalidReplyId { request, got: 253 },
                format!("a reply to {request} came from id 253, outside 0..=252"),
            ),
        ];
        for (err, msg) in cases {
            assert_eq!(*err.request(), request);
            assert_eq!(text(err), msg);
        }
    }

    #[test]
    fn validation_messages_name_the_request() {
        let request = Request::NONE
            .instruction(Instruction::Write)
            .id(id(3))
            .register("Goal Position")
            .addr(30)
            .len(2);
        let cases = [
            (
                ValidationError::AddressOutOfRange { addr: 300, request },
                format!(
                    "address 300 does not fit Protocol 1's one-byte address field (in {request})"
                ),
            ),
            (
                ValidationError::PacketTooLarge {
                    len: 300,
                    max: 255,
                    request,
                },
                format!("packet of 300 bytes exceeds the protocol limit of 255 (in {request})"),
            ),
            (
                ValidationError::DataLength {
                    expected: 4,
                    got: 2,
                    request,
                },
                format!("data is 2 bytes, expected 4 (in {request})"),
            ),
            (
                ValidationError::NotFinite { request },
                format!("value is not a finite number (in {request})"),
            ),
            (
                ValidationError::DoesNotFit {
                    raw: 70000,
                    size: 2,
                    request,
                },
                format!("value 70000 does not fit a 2-byte register (in {request})"),
            ),
            (
                ValidationError::DoesNotFit {
                    raw: 3_000_000_000,
                    size: 0,
                    request: Request::NONE,
                },
                "value 3000000000 does not fit in 32 bits".into(),
            ),
            (
                ValidationError::BufferLength {
                    targets: 3,
                    got: 2,
                    request,
                },
                format!("buffer has 2 slots for 3 targets (in {request})"),
            ),
            (
                ValidationError::InvalidSlopeStep { step: 9, request },
                format!("compliance slope step 9 is outside 1..=7 (in {request})"),
            ),
            (
                ValidationError::NOT_FINITE,
                "value is not a finite number".into(),
            ),
        ];
        for (err, msg) in cases {
            assert_eq!(text(err), msg);
            assert_eq!(Error::from(err), Error::Validation(err));
        }
        assert_eq!(
            ValidationError::NOT_FINITE.within(request),
            ValidationError::NotFinite { request }
        );
        assert_eq!(
            ValidationError::DuplicateId { id: id(1) }.within(request),
            ValidationError::DuplicateId { id: id(1) }
        );
        assert_eq!(ValidationError::DuplicateId { id: id(1) }.request(), None);
    }

    #[test]
    fn other_validation_messages() {
        let cases = [
            (
                ValidationError::InvalidId(253),
                "invalid actuator id 253: must be in 0..=252",
            ),
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
            (
                ValidationError::DuplicateId { id: id(5) },
                "actuator 5 is listed more than once",
            ),
            (
                ValidationError::NotPositionMode { id: id(4) },
                "actuator 4 is not in a position mode; it ignores goal positions",
            ),
            (
                ValidationError::NotVelocityMode {
                    id: id(2),
                    mode: LimitMode::Bounded,
                },
                "actuator 2 is in Bounded mode; only velocity or wheel mode uses a goal velocity",
            ),
            (
                ValidationError::NotCurrentMode {
                    id: id(2),
                    mode: LimitMode::Velocity,
                },
                "actuator 2 is in Velocity mode; only current or current-based position mode uses a goal current",
            ),
            (
                ValidationError::Unsupported {
                    id: id(1),
                    register: "Present Current",
                },
                "actuator 1 has no Present Current register",
            ),
            (
                ValidationError::NoUnit {
                    id: id(1),
                    register: "Moving Speed",
                    mode: LimitMode::Wheel,
                },
                "Moving Speed of actuator 1 has no typed form in Wheel mode; use its raw form",
            ),
            (
                ValidationError::UnsupportedInstruction {
                    id: id(3),
                    instruction: "Bulk Read",
                },
                "actuator 3 does not support the Bulk Read instruction",
            ),
        ];
        for (err, msg) in cases {
            assert_eq!(text(err), msg);
        }
    }

    #[test]
    fn setup_messages_name_every_field() {
        let unsupported = |feature| SetupError::UnsupportedModel {
            id: id(3),
            model_number: 1020,
            protocol: ProtocolVersion::P1,
            feature,
        };
        let ids: IdSet = [1, 2, 5].into_iter().map(id).collect();
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
                SetupError::TooManyActuators {
                    capacity: 2,
                    ids,
                    declared: false,
                },
                "3 actuators answered (ids 1, 2, 5), more than the bus capacity of 2",
            ),
            (
                SetupError::TooManyActuators {
                    capacity: 2,
                    ids,
                    declared: true,
                },
                "3 actuators were declared (ids 1, 2, 5), more than the bus capacity of 2",
            ),
            (
                SetupError::Missing {
                    id: id(5),
                    timeout_us: 10_000,
                },
                "declared actuator 5 sent nothing within 10 ms of its Ping; check power, wiring, \
                 the baud rate and the ID, and that the USB adapter's latency timer is below 10 ms \
                 (FTDI default: 16 ms)",
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
                    register: "Operating Mode",
                },
                "actuator 5 reports unknown Operating Mode value 7",
            ),
        ];
        for (err, msg) in cases {
            assert_eq!(text(err), msg);
            assert_eq!(Error::from(err), Error::Setup(err));
            assert_eq!(Error::from(err).request(), None);
        }
    }

    /// E13: 12 × AX-12A on 12 V, actuator 5 at 9.7 V against 10.5..13.0 V.
    #[test]
    fn voltage_fault_names_measurement_limit_and_direction() {
        let request = Request::NONE
            .instruction(Instruction::Ping)
            .id(id(5))
            .step(Step::Scan);
        let faults = HardwareErrors::from_raw(HardwareErrors::INPUT_VOLTAGE);
        let classify = |measured| {
            VoltageError::classify(
                id(5),
                Decivolts(measured),
                Decivolts(105),
                Decivolts(130),
                faults,
                request,
            )
        };
        let below = classify(97);
        assert!(matches!(below, VoltageError::BelowMin { .. }));
        assert_eq!(
            text(FaultError::from(below)),
            "actuator 5 measures 9.7 V input, below its Min Voltage Limit 10.5 V (limits 10.5 V \
             to 13.0 V); check the supply and the voltage drop along the wiring in reply to Ping \
             from actuator 5, during setup, scanning for actuators"
        );
        let above = classify(140);
        assert!(matches!(above, VoltageError::AboveMax { .. }));
        assert!(
            text(FaultError::from(above)).starts_with(
                "actuator 5 measures 14.0 V input, above its Max Voltage Limit 13.0 V"
            )
        );
        let inside = classify(120);
        assert!(matches!(inside, VoltageError::InsideLimits { .. }));
        assert!(text(FaultError::from(inside)).contains("measures 12.0 V, inside its limits"));
        for err in [below, above, inside] {
            assert_eq!(err.id(), id(5));
            assert_eq!(err.faults(), faults);
            assert_eq!(*err.request(), request);
            let err = Error::from(FaultError::from(err));
            assert!(err.is_hardware_fault());
            assert_eq!(err.request(), Some(&request));
        }
        assert!(matches!(classify(105), VoltageError::InsideLimits { .. }));
        assert!(matches!(classify(130), VoltageError::InsideLimits { .. }));
    }

    #[test]
    fn temperature_fault_names_measurement_and_limit() {
        let faults =
            HardwareErrors::from_raw(HardwareErrors::OVERHEATING | HardwareErrors::OVERLOAD);
        let hot =
            TemperatureError::classify(id(2), Celsius(80), Celsius(72), faults, Request::NONE);
        assert!(matches!(hot, TemperatureError::AboveLimit { .. }));
        assert_eq!(
            text(FaultError::from(hot)),
            "actuator 2 measures 80 °C, above its Temperature Limit 72 °C; let it cool and reduce \
             its load; it also reports an overload"
        );
        let cooled =
            TemperatureError::classify(id(2), Celsius(72), Celsius(72), faults, Request::NONE);
        assert!(matches!(cooled, TemperatureError::WithinLimit { .. }));
        assert!(text(FaultError::from(cooled)).contains("measures 72 °C, at or below"));
        for err in [hot, cooled] {
            assert_eq!(err.id(), id(2));
            assert_eq!(err.faults(), faults);
            assert_eq!(*err.request(), Request::NONE);
        }
    }

    #[test]
    fn other_faults_name_the_actuator_and_every_fault() {
        let request = Request::NONE;
        let faults = HardwareErrors::from_raw(0x3D);
        let cases = [
            (
                FaultError::Overload {
                    id: id(1),
                    faults: HardwareErrors::from_raw(HardwareErrors::OVERLOAD),
                    request,
                },
                "actuator 1 reports an overload: the load stays above what it can hold",
            ),
            (
                FaultError::ElectricalShock {
                    id: id(1),
                    faults,
                    request,
                },
                "actuator 1 reports an electrical shock, or too little power for its motor; it \
                 also reports an input voltage error, overheating, a motor encoder malfunction, \
                 an overload",
            ),
            (
                FaultError::Encoder {
                    id: id(1),
                    faults: HardwareErrors::from_raw(HardwareErrors::ENCODER),
                    request,
                },
                "actuator 1 reports a motor encoder malfunction",
            ),
            (
                FaultError::Unmeasurable {
                    id: id(1),
                    faults: HardwareErrors::from_raw(HardwareErrors::OVERHEATING),
                    register: "Present Temperature",
                    request,
                },
                "actuator 1 reports overheating, but its model has no Present Temperature \
                 register to measure it",
            ),
            (
                FaultError::Unidentified {
                    id: id(1),
                    faults: HardwareErrors::from_raw(0x40),
                    request,
                },
                "actuator 1 raised a hardware alert, but its Hardware Error Status reads 0x40, \
                 no known fault",
            ),
        ];
        for (err, msg) in cases {
            assert_eq!(text(err), msg);
            assert_eq!(err.id(), id(1));
            assert_eq!(*err.request(), request);
            assert!(Error::from(err).is_hardware_fault());
        }
        assert_eq!(
            FaultError::Encoder {
                id: id(1),
                faults,
                request
            }
            .faults(),
            faults
        );
        assert_eq!(HardwareErrors::default().to_string(), "no fault");
    }

    #[test]
    fn status_messages_explain_the_flag_and_name_the_request() {
        let request = Request::NONE
            .instruction(Instruction::Write)
            .id(id(11))
            .register("Goal Position")
            .addr(30)
            .len(2);
        let checksum = StatusError {
            id: id(11),
            report: StatusReport::P1(P1Flags(0x10)),
            request,
        };
        assert_eq!(
            text(checksum),
            "actuator 11 reported a checksum error: the instruction packet it received was \
             corrupted on the line (check wiring, connectors, supply noise and the baud rate) in \
             reply to Write of Goal Position (address 30, 2 bytes) to actuator 11"
        );
        assert_eq!(Error::from(checksum).request(), Some(&request));
        let p1 = |byte| {
            StatusError {
                id: id(1),
                report: StatusReport::P1(P1Flags(byte)),
                request: Request::NONE,
            }
            .to_string()
        };
        assert_eq!(
            p1(0x24),
            "actuator 1 reported overheating (temperature above its Temperature Limit), \
             an overload (load above its maximum torque)"
        );
        assert_eq!(p1(0), "actuator 1 reported no error");
        assert_eq!(p1(0x80), "actuator 1 reported unknown error bits 0x80");
        assert!(p1(0xA0).ends_with(", unknown error bits 0x80"));
        let p2 = |alert, code| {
            StatusError {
                id: id(5),
                report: StatusReport::P2 { alert, code },
                request: Request::NONE,
            }
            .to_string()
        };
        assert_eq!(
            p2(true, Some(P2Code::DataLength)),
            "actuator 5 reported a hardware alert and a data length error (the data is shorter \
             than the register)"
        );
        assert_eq!(p2(true, None), "actuator 5 reported a hardware alert");
        assert_eq!(
            p2(false, Some(P2Code::Unknown(9))),
            "actuator 5 reported unknown error code 9"
        );
        assert_eq!(p2(false, None), "actuator 5 reported no error");
        for code in [
            P2Code::ResultFail,
            P2Code::Instruction,
            P2Code::Crc,
            P2Code::DataRange,
            P2Code::DataLimit,
            P2Code::Access,
        ] {
            assert!(code.to_string().contains('('), "{code:?} explains itself");
        }
        assert!(P2Code::Crc.to_string().contains("corrupted on the line"));
        for flag in P1Flag::ALL {
            assert!(flag.to_string().len() > 10, "{flag:?} explains itself");
        }
    }

    #[test]
    fn debug_of_error_is_its_sentence_and_alternate_is_its_fields() {
        let err = Error::from(TransportError::NoReply {
            request: model_number_read(),
            timeout_us: 10_000,
        });
        assert_eq!(format!("{err:?}"), err.to_string());
        let fields = format!("{err:#?}");
        assert!(fields.starts_with("Transport(\n    NoReply {"));
        assert!(fields.contains("timeout_us: 10000"));
        for err in [
            Error::from(ValidationError::NOT_FINITE),
            Error::from(ProtocolError::InvalidReplyId {
                request: Request::NONE,
                got: 255,
            }),
            Error::from(StatusError {
                id: id(1),
                report: StatusReport::P1(P1Flags(1)),
                request: Request::NONE,
            }),
            Error::from(FaultError::Encoder {
                id: id(1),
                faults: HardwareErrors::default(),
                request: Request::NONE,
            }),
            Error::from(SetupError::NotOnBus { id: id(1) }),
        ] {
            assert_eq!(format!("{err:?}"), err.to_string());
            assert!(!format!("{err:#?}").is_empty());
        }
        assert_eq!(
            format!("{:?}", P1Flags(0x81)),
            "[InputVoltage, unknown bits 0x80]"
        );
    }

    #[test]
    fn hardware_faults_per_protocol() {
        let p1 = |byte| StatusError {
            id: id(1),
            report: StatusReport::P1(P1Flags(byte)),
            request: Request::NONE,
        };
        assert!(p1(0x20).is_hardware_fault());
        assert!(p1(0x04).is_hardware_fault());
        assert!(p1(0x01).is_hardware_fault());
        assert!(!p1(0x08).is_hardware_fault());
        assert!(!p1(0x40).is_hardware_fault());
        assert_eq!(P1Flags(0x7F).hardware().raw(), 0x25);
        let p2 = |alert, code| StatusError {
            id: id(1),
            report: StatusReport::P2 { alert, code },
            request: Request::NONE,
        };
        assert!(p2(true, None).is_hardware_fault());
        assert!(!p2(false, Some(P2Code::Access)).is_hardware_fault());
        assert!(Error::from(p2(true, None)).is_hardware_fault());
        assert!(!Error::from(ValidationError::NOT_FINITE).is_hardware_fault());
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
        let set: std::vec::Vec<_> = P1Flags(0x24).iter().collect();
        assert_eq!(set, [P1Flag::Overheating, P1Flag::Overload]);
        assert_eq!(P1Flags(0).iter().count(), 0);
        assert_eq!(P1Flags(0x7F).iter().count(), 7);
    }

    #[test]
    fn errors_are_copy_eq_and_small() {
        const fn assert_traits<T: Copy + Eq + core::error::Error>() {}
        assert_traits::<Error>();
        assert_traits::<ValidationError>();
        assert_traits::<SetupError>();
        assert_traits::<TransportError>();
        assert_traits::<ProtocolError>();
        assert_traits::<FaultError>();
        assert!(
            core::mem::size_of::<Error>() <= 64,
            "Error is {} bytes",
            core::mem::size_of::<Error>()
        );
    }
}
