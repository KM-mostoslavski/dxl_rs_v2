# Contract: Core API

**Feature**: [../spec.md](../spec.md) | **Model**: [../data-model.md](../data-model.md) |
**Cross-module source**: `specs/ideas/module-interfaces.md` §2

Signatures are the contract. Bodies, private fields, private helpers and tests are not. Items
match module-interfaces §2; differences are marked **D1**/**D2** (see plan.md).

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
`SetupError`, and `ProtocolVersion` (`pub use protocol::ProtocolVersion;`) if PR #7 has not added
it, since core-1 is the first PR to put it in a public API (`SetupError::UnsupportedModel`)
(plus the existing `Actuator`, `Error`, `ValidationError`, `Id`). The rest of the
module-interfaces §2.8 list comes from other modules.

Doc requirements: `Position` states the unit, the zero reference, and the `f32` limit (exact
ticks up to an offset of 2²⁴, research R-6). `TorqueState` documents the transition table.
`Actuator` has an example of a generic function over `A: Actuator` and a `compile_fail,E0603`
example showing `Describe` cannot be named.

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cache { /* id: Id, limits: Option<Limits> */ }
impl Cache {
    pub(crate) const fn new(id: Id) -> Self;
    pub(crate) const fn id(&self) -> Id;
    pub(crate) const fn limits(&self) -> Option<Limits>;
    pub(crate) fn record_limits(&mut self, min: i32, max: i32) -> Result<Limits, ValidationError>;
    pub(crate) fn check_goal(&self, raw: i32) -> Result<i32, ValidationError>;
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
| `Cache`, `ModelData`, `LimitRegisters`, `Protocols`, `ModelNumber`, `TorqueState` | `crate::actuator::…` |
| `Register`, `Access`, `Area` | `crate::register::…` |
| `Position`, `PositionScale` | `crate::units::…` (also `crate::…`) |
| `Limits` | `crate::units::Limits` |
| `SetupError`, `ValidationError`, `Error` | `crate::error::…` (also `crate::…` except internal use) |
| `ProtocolVersion` | `crate::protocol::ProtocolVersion` (protocol module, §5) |

**For X1 (D1)**: build each `Register` through the generated `reg()` helper (a struct literal),
not with `Register::new`. It compiles against core-1 and core-2. core-2 has no `Register::new`.

**For X1 (D2)**: core-1 removes the core tests of `ActuatorKind::new`. X1 owns tests of the
lookup (firmware split, unknown number, family off).

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
}

#[non_exhaustive]
pub enum SetupError {
    UnsupportedModel { id: Id, model_number: u16, protocol: ProtocolVersion, feature: Option<&'static str> },
    TooManyActuators { capacity: usize },
    Missing { id: Id },
    WrongModel { id: Id, declared: u16, found: u16 },
    NotOnBus { id: Id },
}
```

Every field is documented (`missing_docs`). `Error::Setup` uses `#[error(transparent)]` like the
other categories.

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
| `SetupError::UnsupportedModel { id: 3, model_number: 1020, protocol: P1, feature: Some("x-series") }` | `actuator 3 reports model number 1020, not supported on Protocol 1 (enable the `x-series` feature)` |
| same, `feature: None` | `actuator 3 reports model number 1020, not supported on Protocol 1` |
| `TooManyActuators { capacity: 32 }` | `more actuators answered than the bus capacity of 32` |
| `Missing { id: 5 }` | `declared actuator 5 did not answer` |
| `WrongModel { id: 5, declared: 1020, found: 12 }` | `actuator 5 was declared as model number 1020 but reports 12` |
| `NotOnBus { id: 7 }` | `actuator 7 is not on this bus` |

Existing messages (`InvalidId`, PR #7 variants) do not change.
