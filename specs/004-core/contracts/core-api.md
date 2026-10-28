# Contract: Core API

**Feature**: [../spec.md](../spec.md) | **Model**: [../data-model.md](../data-model.md) |
**Cross-module source**: `specs/ideas/module-interfaces.md` §2

Signatures are the contract. Bodies, private fields, private helpers and tests are not. Items
match module-interfaces §2; differences are marked **D1**/**D2**/**D3** (see plan.md). The limit
mode (§2.4b) and its two errors were decided by the architect on 2026-10-28.

## Public (re-exported at the crate root)

```rust
// src/actuator.rs
/// A model number as the actuator reports it (Model Number register, address 0, 2 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModelNumber(pub u16);
impl ModelNumber {
    pub const fn get(self) -> u16;
}
impl From<u16> for ModelNumber;
impl core::fmt::Display for ModelNumber;         // "1020"

/// Torque and fault state of one actuator. Faulted always means torque off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum TorqueState {
    #[default]
    Off = 0,
    On = 1,
    Faulted = 2,
}
impl TorqueState {
    pub const fn is_on(self) -> bool;
    pub const fn is_faulted(self) -> bool;
}

// core-1: unchanged from dev.
pub trait Actuator {
    fn id(&self) -> Id;
    fn model_number(&self) -> u16;
    fn name(&self) -> &'static str;
}

// core-2: sealed, describes a model, no I/O. Every method is provided.
pub trait Actuator: sealed::Describe {
    fn id(&self) -> Id;
    fn model_number(&self) -> u16;
    fn name(&self) -> &'static str;
    fn position_scale(&self) -> PositionScale;
    /// `None` until Setup read the limits from EEPROM.
    fn position_limits_raw(&self) -> Option<core::ops::RangeInclusive<i32>>;
    fn position_limits(&self) -> Option<core::ops::RangeInclusive<Position>>;
}

// src/units/position.rs
/// An angle in radians. Zero is the model's zero-radian tick (2048 on XM430, 512 on AX-12A).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Position(/* f32 */);
impl Position {
    pub const ZERO: Self;
    pub const fn from_radians(radians: f32) -> Self;
    pub const fn radians(self) -> f32;
}
impl core::ops::Add for Position { type Output = Position; }
impl core::ops::Sub for Position { type Output = Position; }
impl core::ops::AddAssign for Position;
impl core::ops::SubAssign for Position;

/// Per-model tick <-> radian mapping, from the control table's `type_info`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositionScale { /* private */ }
impl PositionScale {
    pub const fn zero_raw(&self) -> i32;
    pub fn radians_per_tick(&self) -> f32;
    pub fn to_position(&self, raw: i32) -> Position;
    /// Nearest tick. NaN or infinite: `NotFinite`. Beyond `i32`: `DoesNotFit { size: 0, .. }`.
    pub fn to_raw(&self, position: Position) -> Result<i32, ValidationError>;
}
```

Root re-exports added by core: `ModelNumber`, `TorqueState`, `Position`, `PositionScale`,
`SetupError` (plus the existing `Actuator`, `Error`, `ValidationError`, `Id`). `ProtocolVersion`,
which `SetupError::UnsupportedModel` puts in core's public API, is already re-exported by PR #7
(merged into `main`: `pub use protocol::{Clock, P1, P2, Protocol, ProtocolVersion};`). core-1 only
checks that line and adds none. The rest of the module-interfaces §2.8 list comes from other
modules.

Doc requirements: `Position` states the unit, the zero reference, and the `f32` limit (exact
ticks up to an offset of 2²⁴, research R-6). `TorqueState` documents the transition table.
`Actuator` has an example of a generic function over `A: Actuator`, a `compile_fail,E0603`
example showing `Describe` cannot be named (never re-exported, §2.8), and from core-2 a
`compile_fail,E0277` example showing a user type cannot implement `Actuator` (the seal). Each
`ModeValues` variant, the XL-320 `ControlMode` arm and `LimitMode::from_angle_limits` cite their
e-Manual page (research R-17).

## Crate-internal (for xtask and bus)

```rust
// src/units/position.rs
impl PositionScale {
    pub(crate) const fn new(zero_raw: i32, min_raw: i32, max_raw: i32, min_rad: f32, max_rad: f32) -> Self;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits { /* min, max: i32 */ }
impl Limits {
    pub(crate) const fn new(id: Id, min: i32, max: i32) -> Result<Self, ValidationError>;
    pub(crate) const fn min_raw(&self) -> i32;
    pub(crate) const fn max_raw(&self) -> i32;
}

// src/actuator.rs
/// How a goal position is checked, decided at Setup from the operating mode (§2.4b).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub(crate) enum LimitMode {
    #[default]
    Bounded = 0,     // position mode: EEPROM limits apply
    Unbounded = 1,   // extended / current-based position, MX multi-turn: register size only
    NoPosition = 2,  // velocity, current, PWM, wheel: every goal is NotPositionMode
}
impl LimitMode {
    /// AX, MX on P1: CW = CCW = 0 → NoPosition; CW = CCW = 4095 → Unbounded; else Bounded.
    pub(crate) const fn from_angle_limits(limits: Limits) -> Self;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cache { /* id: Id, mode: LimitMode, limits: Option<Limits> */ } // 16 bytes
impl Cache {
    pub(crate) const fn new(id: Id) -> Self;                // mode Bounded, limits None
    pub(crate) const fn id(&self) -> Id;
    pub(crate) const fn limits(&self) -> Option<Limits>;
    pub(crate) fn record_limits(&mut self, min: i32, max: i32) -> Result<Limits, ValidationError>;
    pub(crate) fn record_mode(&mut self, mode: LimitMode);
    /// Bounded: `LimitsUnknown`, `OutOfLimits`, else `Ok(raw)`. Unbounded: `Ok(raw)`.
    /// NoPosition: `NotPositionMode { id }`.
    pub(crate) fn check_goal(&self, raw: i32) -> Result<i32, ValidationError>;
}

/// Where Setup gets the limit mode. One per model, in `ModelData::mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModeSource {
    /// "Operating Mode" (X, MX(2.0), PRO, PRO+, Y, RH-P12-RN): 1 byte read at Setup. (D3)
    OperatingMode { register: Register, values: ModeValues },
    /// XL-320 "Control Mode": 1 = wheel → NoPosition, 2 = joint → Bounded.
    ControlMode(Register),
    /// AX, MX on P1: no register; `LimitMode::from_angle_limits` on the limits read at Setup.
    AngleLimits,
}
impl ModeSource {
    /// The 1-byte register Setup reads; `None` for `AngleLimits`.
    pub(crate) const fn register(self) -> Option<Register>;
    /// `None`: not listed for this family (the bus returns `UnknownOperatingMode`), or `AngleLimits`.
    pub(crate) const fn limit_mode(self, value: u8) -> Option<LimitMode>;
}

/// Which e-Manual Operating Mode list applies (D3). Values: research R-17.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModeValues {
    X,        // X-series, MX(2.0): 0, 1, 16 NoPosition; 3 Bounded; 4, 5 Unbounded
    Pro,      // PRO (A), PRO+: 0, 1, 16 NoPosition; 3 Bounded; 4 Unbounded
    Y,        // Y: 0, 1 NoPosition; 3 Bounded
    RhP12Rn,  // RH-P12-RN: 0 NoPosition; 5 Bounded
}
impl ModeValues {
    pub(crate) const fn limit_mode(self, value: u8) -> Option<LimitMode>;
}

impl TorqueState {
    pub(crate) const fn enable(self, id: Id) -> Result<Self, ValidationError>;
    pub(crate) const fn disable(self) -> Self;
    pub(crate) const fn fault(self) -> Self;
    pub(crate) const fn clear(self) -> Self;
    pub(crate) const fn check_goal(self, id: Id) -> Result<(), ValidationError>;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelData {
    pub(crate) name: &'static str,
    pub(crate) model_number: u16,
    pub(crate) protocols: Protocols,
    pub(crate) torque_enable: Register,
    pub(crate) goal_position: Register,
    pub(crate) present_position: Register,
    pub(crate) limits: LimitRegisters,
    pub(crate) hardware_error: Option<Register>,
    pub(crate) mode: ModeSource,
    pub(crate) scale: PositionScale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitRegisters {
    MinMax { min: Register, max: Register },
    CwCcw { cw: Register, ccw: Register },
}
impl LimitRegisters {
    pub(crate) const fn min(&self) -> Register;   // Min, or CW
    pub(crate) const fn max(&self) -> Register;   // Max, or CCW
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Protocols { pub(crate) p1: bool, pub(crate) p2: bool }
impl Protocols {
    pub(crate) const fn supports(self, protocol: ProtocolVersion) -> bool;
}

pub(crate) mod sealed {
    pub trait Describe {
        fn data(&self) -> &'static ModelData;
        fn cache(&self) -> &Cache;
        fn cache_mut(&mut self) -> &mut Cache;
    }
}
pub(crate) use sealed::Describe;

// src/register.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Register {
    pub(crate) addr: u16,
    pub(crate) size: u8,
    pub(crate) access: Access,
    pub(crate) area: Area,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access { R, RW }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Area { Eeprom, Ram }
// core-1 and core-2: #[allow(dead_code, reason = "fields read by bus-1")] on the struct
// (replaces today's cfg_attr(not(test), expect(dead_code))).
impl Register {
    // core-1 only (shim, D1): access = R, area = Eeprom placeholders. #[doc(hidden)],
    // #[allow(dead_code, reason = "unused once X1 regenerates with reg(); removed in core-2")].
    pub(crate) const fn new(addr: u16, size: u8) -> Self;
    // core-2: shim removed, no constructor added. X1's reg() builds registers (D1).

    pub(crate) fn encode(self, raw: i32, out: &mut [u8; 4]) -> Result<&[u8], ValidationError>;
    pub(crate) fn decode(self, bytes: &[u8]) -> Result<i32, ValidationError>;
}
```

### Paths for xtask and bus

| Item | Path |
|---|---|
| `Describe` | `crate::actuator::Describe` (or `crate::actuator::sealed::Describe`) |
| `Cache`, `ModelData`, `LimitRegisters`, `Protocols`, `ModelNumber`, `TorqueState`, `LimitMode`, `ModeSource`, `ModeValues` | `crate::actuator::…` |
| `Register`, `Access`, `Area` | `crate::register::…` |
| `Position`, `PositionScale` | `crate::units::…` (also `crate::…`) |
| `Limits` | `crate::units::Limits` |
| `SetupError`, `ValidationError`, `Error` | `crate::error::…` (also `crate::…` except internal use) |
| `ProtocolVersion` | `crate::protocol::ProtocolVersion` (protocol module, §5) |

**For X1 (D1)**: build each `Register` through the generated `reg()` helper (a struct literal),
not with `Register::new`. It compiles against core-1 and core-2. core-2 has no `Register::new`.

**For X1 (D2)**: core-1 removes the core tests of `ActuatorKind::new`. X1 owns tests of the
lookup (firmware split, unknown number, family off).

**For X1 (D3)**: every `DATA` sets `mode`. Proposed rule (xtask's plan decides how it gets
there): a table with an "Operating Mode" row gives
`ModeSource::OperatingMode { register: reg(…), values }`, with `values` = `X` for the
`x-series` and `mx` families, `Y` for `y-series`, `RhP12Rn` for RH-P12-RN and `Pro` for every
other `pro` table. XL-320's "Control Mode" row gives `ControlMode(reg(…))`. A table with
CW/CCW Angle Limit and neither row (AX, MX on P1) gives `AngleLimits`.

**For bus (D3)**: Setup, after the limits are recorded: if `data().mode.register()` is
`Some(r)`, read `r` (1 byte) and map it with `data().mode.limit_mode(value)`, where `None` fails
Setup with `SetupError::UnknownOperatingMode { id, value }`; else use
`LimitMode::from_angle_limits(limits)`. Then `cache_mut().record_mode(mode)`. The goal path
calls `cache().check_goal(raw)` as before; it applies the mode.

## Errors

```rust
// src/error.rs. All: Debug, Clone, Copy, PartialEq, Eq, thiserror::Error. No allocation.
#[non_exhaustive]
pub enum Error {
    Transport(#[from] TransportError),       // PR #7
    Protocol(#[from] ProtocolError),         // PR #7
    Status(#[from] StatusError),             // PR #7
    Validation(#[from] ValidationError),
    Setup(#[from] SetupError),               // core-1
    #[doc(hidden)]
    UnsupportedModel { model_number: u16, feature: Option<&'static str> }, // core-1 keeps, core-2 removes
}

#[non_exhaustive]
pub enum ValidationError {
    InvalidId(u8),
    AddressOutOfRange { addr: u16 },
    PacketTooLarge { len: usize, max: usize },
    DataLength { expected: usize, got: usize },
    ActuatorIsOff { id: Id },
    ActuatorFaulted { id: Id },
    OutOfLimits { id: Id, raw: i32, min: i32, max: i32 },
    LimitsUnknown { id: Id },
    InvertedLimits { id: Id, min: i32, max: i32 },
    NotFinite,
    DoesNotFit { raw: i64, size: u8 },       // size 0 = does not fit i32
    BufferLength { targets: usize, got: usize },
    DuplicateId { id: Id },
    NotPositionMode { id: Id },              // 2026-10-28
}

#[non_exhaustive]
pub enum SetupError {
    UnsupportedModel { id: Id, model_number: u16, protocol: ProtocolVersion, feature: Option<&'static str> },
    TooManyActuators { capacity: usize },
    Missing { id: Id },
    WrongModel { id: Id, declared: u16, found: u16 },
    NotOnBus { id: Id },
    UnknownOperatingMode { id: Id, value: u8 }, // 2026-10-28
}
```

Every field is documented (`missing_docs`). `Error::Setup` uses `#[error(transparent)]` like the
other categories. `UnsupportedModel` carries `id` and `protocol` beyond SRS FR-2.4's
`{ model_number, feature }` (conflict 19, accepted by the architect on 2026-10-28).

### Error messages

Fixed texts, checked by tests (SC-004). `{id}` is `id.get()`. `ProtocolVersion` displays as
`Protocol 1` / `Protocol 2` (§5).

| Variant | Message |
|---|---|
| `ActuatorIsOff { id: 4 }` | `actuator 4 has torque off; enable torque before setting a goal` |
| `ActuatorFaulted { id: 4 }` | `actuator 4 is faulted; clear the fault before enabling torque` |
| `OutOfLimits { id: 3, raw: 3001, min: 1000, max: 3000 }` | `goal 3001 for actuator 3 is outside its limits 1000..=3000` |
| `LimitsUnknown { id: 3 }` | `limits of actuator 3 are not known; they are read at setup` |
| `InvertedLimits { id: 3, min: 3000, max: 1000 }` | `actuator 3 reports a minimum limit 3000 above its maximum 1000` |
| `NotFinite` | `angle is not a finite number` |
| `DoesNotFit { raw: 70000, size: 2 }` | `value 70000 does not fit a 2-byte register` |
| `DoesNotFit { raw: 3000000000, size: 0 }` | `value 3000000000 does not fit in 32 bits` |
| `BufferLength { targets: 3, got: 2 }` | `buffer has 2 slots for 3 targets` |
| `DuplicateId { id: 5 }` | `actuator 5 is listed more than once` |
| `NotPositionMode { id: 4 }` | `actuator 4 is not in a position mode; it ignores goal positions` |
| `SetupError::UnsupportedModel { id: 3, model_number: 1020, protocol: P1, feature: Some("x-series") }` | `actuator 3 reports model number 1020, not supported on Protocol 1 (enable the `x-series` feature)` |
| same, `feature: None` | `actuator 3 reports model number 1020, not supported on Protocol 1` |
| `TooManyActuators { capacity: 32 }` | `more actuators answered than the bus capacity of 32` |
| `Missing { id: 5 }` | `declared actuator 5 did not answer` |
| `WrongModel { id: 5, declared: 1020, found: 12 }` | `actuator 5 was declared as model number 1020 but reports 12` |
| `NotOnBus { id: 7 }` | `actuator 7 is not on this bus` |
| `UnknownOperatingMode { id: 5, value: 7 }` | `actuator 5 reports unknown operating mode 7` |

Existing messages (`InvalidId`, PR #7 variants) do not change.
