# Cross-module interface contract (core, tables, xtask, bus, ci)

**Status**: Contract, 2026-10-27, updated 2026-10-28 (PR #7 merged, architect decisions of
2026-10-28, plan deviations folded in). The plans of specs 003 (bus), 004 (core), 005 (tables),
006 (xtask), 007 (ci) follow it.

**Sources**: SRS (2026-10-27 architect answers, applied; the 2026-10-28 decisions are not in it
yet, section 9), constitution v1.3.0, ARCHITECTURE.md, the five specs and plans,
`specs/ideas/bus-proposal.md`, and `src/` + `xtask/` on `main` (protocol PR #7 merged
2026-10-28).

**Rule for planners**: every plan uses the names, signatures, file owners and order below. A
plan that needs something else raises it as a deviation in its Architecture section. It does
not silently redefine an item another module owns. Signatures are the contract; bodies, private
helpers, tests and doc text are not. "Crate-internal" means `pub(crate)`, or a `pub` item inside
a private or `pub(crate)` module (the sealed pattern PR #7 uses for `Link`, `Pong` and
`Exchange`).

---

## 1. File ownership

| File or path | Owner | Others may |
|---|---|---|
| `src/lib.rs` | core | Add only their own `mod x;` line and their own `pub use x::…` lines. List them in the PR. core-1 deletes `P2::broadcast_ping` (its only dead item, checked on `main` 2026-10-28) together with PR #7's `#[cfg_attr(not(test), expect(dead_code, …))]` on `mod protocol;` (architect 2026-10-28; a protocol touch, listed in the core-1 PR). |
| `src/error.rs` | core | PR #7 (merged) added `Transport`, `Protocol`, `Status`. No other module edits it. A missing variant is raised to core. |
| `src/id.rs`, `src/actuator.rs`, `src/register.rs`, `src/units.rs`, `src/units/**` | core | Nothing. |
| `src/protocol.rs`, `src/protocol/**` | protocol (PR #7, merged; not one of the five) | Nothing. Section 5 is merged. A further need goes to a protocol follow-up PR. |
| `src/transport.rs`, `src/transport/**` | transport (merged) | Nothing. |
| `src/models.rs`, `src/models/**` | xtask (generated only) | A tables PR commits the regenerated output when it changes a table. Nobody edits by hand. |
| `xtask/**` | xtask | Nothing. |
| `control_tables/**` | tables | Nothing. |
| `src/bus.rs`, `src/bus/**` | bus | Nothing. |
| `tests/bus_no_alloc.rs` | bus (bus plan, deviation 3: a counting allocator needs its own test binary) | Nothing. |
| `.github/workflows/**` (never `ov-*.yml`) | ci | Nothing. |
| `Cargo.toml` `[package]`, metadata, `[lints]` | ci | Nothing. |
| `Cargo.toml` `[features]`, `[dependencies]`, `[dev-dependencies]` | The module that first needs the line, in its own PR, flagged as a shared-file touch | ci never adds or removes features or dependencies (ci FR-015). |
| `docs/srs/SRS.md`, constitution, `.aiignore`, `ov-*` files | architect | Nothing (no-go). |
| `docs/ARCHITECTURE.md` | architect | Nothing. Stale lines are listed in section 9. |

Known `Cargo.toml` touches: `heapless` (bus-1), `partial-reads` and `raw-registers` features
(bus-5). Missing docs are caught by `#![warn(missing_docs)]` in `src/lib.rs` (PR #7) under CI's
`-D warnings`, so ci adds no `[lints]` table (ci plan). PR #7 (merged) added dev-dependency
`proptest`. Family features `ax, mx, x-series, y-series, pro` do not change.

---

## 2. core → xtask, bus

Core does no I/O. Everything here is `no_std`, allocation-free, `dyn`-free and panic-free.

### 2.1 Identity types

```rust
// src/id.rs (exists, unchanged)
pub struct Id(u8);                       // 0..=252

// src/actuator.rs (new)
/// A model number as the actuator reports it (register Model Number, address 0, 2 bytes).
/// Any u16 is valid; whether this build supports it is decided at Setup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModelNumber(pub u16);
impl ModelNumber {
    pub const fn get(self) -> u16;
}
impl From<u16> for ModelNumber;
impl core::fmt::Display for ModelNumber;
```

`ModelNumber` is core's, not bus's (resolves core Assumptions vs xtask Assumptions). It is the
input type of declared setup and of the generated lookup. Errors and `Actuator::model_number()`
keep a plain `u16` (no breaking change to the existing trait method).

`ProtocolVersion` comes from the protocol module (section 5). Core uses it in `SetupError`.

### 2.2 Register

```rust
// src/register.rs. Crate-internal: pub items inside a private module, never re-exported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Register {
    pub(crate) addr: u16,
    pub(crate) size: u8,        // 1, 2 or 4
    pub(crate) access: Access,
    pub(crate) area: Area,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access { R, RW }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Area { Eeprom, Ram }

impl Register {
    // No constructor after core-2. X1's generated `reg()` helper builds registers (§4.1).
    /// Little-endian bytes of `raw` into `out[..size]`. A value that does not fit the
    /// register is `ValidationError::DoesNotFit`, never a truncation.
    pub(crate) fn encode(self, raw: i32, out: &mut [u8; 4]) -> Result<&[u8], ValidationError>;
    /// Sizes 1 and 2 decode unsigned; size 4 decodes as two's-complement `i32`.
    pub(crate) fn decode(self, bytes: &[u8]) -> Result<i32, ValidationError>;
}
```

- Range, default and unit are not fields now. They come per model on verification (architect,
  2026-10-27).
- The decode sign rule covers the four common registers. Every table today has 1- and 2-byte
  positions unsigned and 4-byte positions signed. The tables module confirms this per model on
  verification. A register with another sign rule needs a contract change. Known case: MX on P1
  in multi-turn mode, where the 2-byte Goal Position ranges -28 672 to 28 672 on the e-Manual;
  until T2 checks it, a negative goal is refused with `DoesNotFit` (core research R-17).
- Registers are built only by X1's private generated helper
  `const fn reg(addr, size, access, area) -> Register`, a struct literal over the four
  `pub(crate)` fields. Core has no 4-argument `new` (core and xtask plans, deviation D1). Rust has
  no overloading, so a 4-argument `new` could not coexist with the core-1 shim.
- Transition: core-1 keeps a crate-internal 2-argument `Register::new(addr, size)` shim,
  `#[doc(hidden)]` with an unconditional `allow(dead_code)` (section 7). core-2 removes it and
  adds no constructor.
- A build with no family feature must stay free of warnings (today `Register::new` warns; core-1
  fixes it).

### 2.3 Typed position

```rust
// src/units.rs + src/units/position.rs
/// An angle in radians. Zero is the model's zero-radian tick (2048 on XM430, 512 on AX-12A).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Position(f32);
impl Position {
    pub const ZERO: Self;
    pub const fn from_radians(radians: f32) -> Self;
    pub const fn radians(self) -> f32;
}
impl core::ops::Add for Position;        // + AddAssign
impl core::ops::Sub for Position;        // + SubAssign

/// Per-model tick <-> radian mapping, from the table's `type_info`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositionScale { /* zero_raw, min_raw, max_raw: i32; min_rad, max_rad: f32 */ }
impl PositionScale {
    pub(crate) const fn new(zero_raw: i32, min_raw: i32, max_raw: i32, min_rad: f32, max_rad: f32) -> Self;
    pub const fn zero_raw(&self) -> i32;
    /// (max_rad - min_rad) / (max_raw - min_raw)
    pub fn radians_per_tick(&self) -> f32;
    pub fn to_position(&self, raw: i32) -> Position;
    /// Rounds to the nearest tick. NaN or infinite: `NotFinite`. Beyond i32: `DoesNotFit`.
    pub fn to_raw(&self, position: Position) -> Result<i32, ValidationError>;
}
```

Raw ticks are `i32` everywhere (API, cache, buffers). The scale uses only `zero_raw` and the
slope. T1 checks `type_info` against each page and fixes it (tables research R-8): AX angles
moved to the zero, XL-320 range, X/MX(2.0)/2XL slope 2π/4096. After T1, 4096-tick tables read
`PositionScale::new(2048, 0, 4095, -3.1415927, 3.1400587)` and AX / XL-320
`(512, 0, 1023, -2.6205530, 2.6154347)`.

### 2.4 Limits and the per-actuator cache

```rust
// src/units/position.rs. Limits and Cache are crate-internal (sealed pattern).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits { min: i32, max: i32 }
impl Limits {
    /// `min > max` gives `InvertedLimits`.
    pub(crate) const fn new(id: Id, min: i32, max: i32) -> Result<Self, ValidationError>;
    pub(crate) const fn min_raw(&self) -> i32;
    pub(crate) const fn max_raw(&self) -> i32;
}

// src/actuator.rs
/// Held inside every model type: the Id and the limits read from EEPROM at Init.
/// No torque state, no fault state, no fault report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cache { id: Id, limits: Option<Limits> }
impl Cache {
    pub(crate) const fn new(id: Id) -> Self;              // limits unknown
    pub(crate) const fn id(&self) -> Id;
    pub(crate) const fn limits(&self) -> Option<Limits>;
    pub(crate) fn record_limits(&mut self, min: i32, max: i32) -> Result<Limits, ValidationError>;
    /// The one limit check, typed and raw paths alike. Unknown limits: `LimitsUnknown`.
    /// Outside (bounds included): `OutOfLimits`. Returns `raw` on success.
    pub(crate) fn check_goal(&self, raw: i32) -> Result<i32, ValidationError>;
}
```

No constructor fills limits from a control table (FR-007 core). Section 2.4b adds
`mode: LimitMode` to `Cache` and makes `check_goal` follow it.

### 2.4b Limit mode (architect, 2026-10-28: limits follow the operating mode)

The firmware applies position limits only in position mode. The e-Manual (XM430-W350, Min/Max
Position Limit): "These values are not used in Extended Position Control Mode and Current-based
Position Control Mode". The goal check follows the firmware.

```rust
// src/actuator.rs, crate-internal
/// How a goal position is checked, decided at Setup from the operating mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub(crate) enum LimitMode {
    /// Position (joint) mode: goal must lie in the EEPROM limits read at Setup.
    #[default]
    Bounded = 0,
    /// Extended / current-based position, MX multi-turn: the EEPROM limits are not used by the
    /// firmware; only the register size check applies until the model's range is verified.
    Unbounded = 1,
    /// Velocity, current, PWM, AX/XL-320 wheel mode: the firmware ignores Goal Position.
    /// A goal write returns `ValidationError::NotPositionMode { id }` and sends nothing.
    NoPosition = 2,
}

pub struct Cache { id: Id, mode: LimitMode, limits: Option<Limits> } // mode sits in the padding
                                                                    // after `id`: still 16 bytes
impl Cache {
    pub(crate) fn record_mode(&mut self, mode: LimitMode);
    /// Bounded: as before (`LimitsUnknown`, `OutOfLimits`). Unbounded: `Ok(raw)`.
    /// NoPosition: `NotPositionMode { id }`.
    pub(crate) fn check_goal(&self, raw: i32) -> Result<i32, ValidationError>;
}

// src/actuator.rs, crate-internal (core-1). X1 emits one value per model in `ModelData::mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]       // ModelData derives these
pub(crate) enum ModeSource {
    /// "Operating Mode" (X, MX(2.0), PRO, PRO+, Y, RH-P12-RN) or XL-320 "Control Mode":
    /// 1 byte read at Setup, decoded with the model's own value lists.
    Register { register: Register, values: ModeValues },
    /// AX, MX on P1: no register; derived from the CW/CCW limits already read.
    /// CW = CCW = 0 → NoPosition (wheel). MX P1 CW = CCW = 4095 → Unbounded (multi-turn).
    AngleLimits,
}
impl ModeSource {
    pub(crate) const fn register(self) -> Option<Register>;               // None for AngleLimits
    pub(crate) const fn limit_mode(self, value: u8) -> Option<LimitMode>; // None: not listed
}

/// The meaning of each mode value for one model, generated from its table's
/// `operating_modes` field (architect 2026-10-28: the value lists live in the tables).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModeValues {
    pub(crate) bounded: &'static [u8],
    pub(crate) unbounded: &'static [u8],
    pub(crate) no_position: &'static [u8],
}
impl ModeValues {
    /// `None` when the value is in no list.
    pub(crate) const fn limit_mode(self, value: u8) -> Option<LimitMode>;
}
impl LimitMode {
    /// AX and MX on P1, from the limits Setup has just read.
    pub(crate) const fn from_angle_limits(limits: Limits) -> LimitMode;
}
```

- Decided 2026-10-28: each table carries its own value lists (§3.1 `operating_modes`), copied
  from that model's e-Manual page. Core holds no per-family list, and a new model or family is a
  data change only. Value 5 shows why: Unbounded on X and MX(2.0), Bounded on RH-P12-RN (`pro`).
  Today's lists, cited in core research R-17:

  | Tables | Bounded | Unbounded | NoPosition |
  |---|---|---|---|
  | X-series but XL-320, MX(2.0) | 3 | 4, 5 | 0, 1, 16 |
  | PRO (A), PRO+ | 3 | 4 | 0, 1, 16 |
  | Y | 3 | — | 0, 1 |
  | RH-P12-RN | 5 | — | 0 |
  | XL-320 (Control Mode) | 2 | — | 1 |
  | AX, MX P1 (no field: `AngleLimits`) | any other pair | CW = CCW = 4095 | CW = CCW = 0 |

- A register value in no list (`None`) fails Setup with
  `SetupError::UnknownOperatingMode { id, value }` (constitution III: no fallback). The bus
  builds the error; core never guesses a mode.
- X1 takes the register from the row, by e-Manual name: an `Operating Mode` or (XL-320) a
  `Control Mode` row gives `Register` with the table's `operating_modes`; the CW/CCW pair with no
  mode row gives `AngleLimits`. Each table has exactly one source; a mode row without
  `operating_modes`, `operating_modes` without a mode row, or overlapping lists fail codegen and
  the tables check (§4.3). The address comes from the row, never a per-family constant (11 in 54
  tables, 33 in the 12 Y tables). Today: 66 tables have `Operating Mode` + Min/Max, `xl320` has
  `Control Mode` + CW/CCW, the 5 AX tables have CW/CCW only; T2's 3 MX P1 tables add CW/CCW only.
- `position_limits()` / `position_limits_raw()` return the EEPROM limits in every mode
  (architect 2026-10-28); the rustdoc says they are applied only in `Bounded` mode.
- Setup cost: one 1-byte read per actuator for `ModeSource::Register` models, on
  either protocol (X and MX(2.0) on P1 use the same table as on P2); none for `AngleLimits` (AX,
  MX P1).
- Memory cost: zero. `mode` fits in the 3 padding bytes after `id` in `Cache`.
- The mode is read once at Setup. It is EEPROM and changes only with torque off. A
  `raw-registers` write to it leaves the cache stale until the next Setup; documented on
  `write_raw`. A typed `set_operating_mode` waits for OQ-2.

### 2.5 Torque state (one byte, held by the bus)

```rust
// src/actuator.rs
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
    pub const fn is_on(self) -> bool;          // On only
    pub const fn is_faulted(self) -> bool;
    pub(crate) const fn enable(self, id: Id) -> Result<Self, ValidationError>;
    pub(crate) const fn disable(self) -> Self;
    pub(crate) const fn fault(self) -> Self;
    pub(crate) const fn clear(self) -> Self;
    /// Goal-write guard: `On` passes, `Off` and `Faulted` give `ActuatorIsOff { id }`.
    pub(crate) const fn check_goal(self, id: Id) -> Result<(), ValidationError>;
}
```

Not `#[non_exhaustive]`: the three states are final, so users can match them in full.
`size_of::<TorqueState>() == 1` is tested.

| From \ event | `enable` | `disable` | `fault` | `clear` |
|---|---|---|---|---|
| `Off` | `On` | `Off` | `Faulted` | `Off` |
| `On` | `On` | `Off` | `Faulted` | `On` |
| `Faulted` | Err `ActuatorFaulted` | `Faulted` | `Faulted` | `Off` |

The bus applies these rules (core only defines them):

- `enable` is checked before any byte is sent.
- After a torque write: `Ok` gives the new state. A hardware fault gives `fault()`. Any other
  error applies `state.disable()`, never `On` (bus FR-027): `On` and `Off` give `Off`, and a
  `Faulted` slot stays `Faulted`.
- Any status reply whose `StatusError::is_hardware_fault()` is true gives `fault()`. This
  applies on every path, `raw-registers` included: raw replies run `observe` too (architect,
  2026-10-28; no exception to constitution III). Raw calls still skip the torque guard, the
  limit check and the cache.
- `clear()` runs only after the actuator reports no fault (bus FR-028).
- Setup always starts every slot at `Off` and sends no torque write.

### 2.6 Model description: `ModelData`, `Describe`, `Actuator`

```rust
// src/actuator.rs. ModelData, LimitRegisters, Protocols: crate-internal (sealed pattern).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelData {
    pub(crate) name: &'static str,
    pub(crate) model_number: u16,
    pub(crate) protocols: Protocols,             // from the table's `protocols` field
    pub(crate) torque_enable: Register,
    pub(crate) goal_position: Register,
    pub(crate) present_position: Register,
    pub(crate) limits: LimitRegisters,
    pub(crate) hardware_error: Option<Register>, // "Hardware Error Status", else None
    pub(crate) mode: ModeSource,                 // section 2.4b
    pub(crate) scale: PositionScale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitRegisters {
    MinMax { min: Register, max: Register },     // "Min/Max Position Limit"
    CwCcw { cw: Register, ccw: Register },       // "CW/CCW Angle Limit" (AX, MX P1, XL-320)
}
impl LimitRegisters {
    pub(crate) const fn min(&self) -> Register;  // CW maps to min
    pub(crate) const fn max(&self) -> Register;  // CCW maps to max
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Protocols { pub(crate) p1: bool, pub(crate) p2: bool }
impl Protocols {
    pub(crate) const fn supports(self, protocol: ProtocolVersion) -> bool;
}

pub(crate) mod sealed {
    /// What the bus needs from a model. No I/O.
    pub trait Describe {
        fn data(&self) -> &'static ModelData;
        fn cache(&self) -> &Cache;
        fn cache_mut(&mut self) -> &mut Cache;
    }
}
pub(crate) use sealed::Describe;   // X1 and bus name it `crate::actuator::Describe`

/// What every model shares. Sealed: only generated types, `ActuatorKind` and `ActuatorRef`
/// implement it. Describes a model; performs no I/O (I/O is on `ActuatorRef`).
/// core-2: every method is provided (core research R-13).
pub trait Actuator: sealed::Describe {
    fn id(&self) -> Id { self.cache().id() }
    fn model_number(&self) -> u16 { self.data().model_number }
    fn name(&self) -> &'static str { self.data().name }
    fn position_scale(&self) -> PositionScale { self.data().scale }
    /// `None` until Setup read the limits from EEPROM.
    fn position_limits_raw(&self) -> Option<core::ops::RangeInclusive<i32>> {
        self.cache().limits().map(|l| l.min_raw()..=l.max_raw())
    }
    fn position_limits(&self) -> Option<core::ops::RangeInclusive<Position>> {
        let scale = self.position_scale();
        self.position_limits_raw()
            .map(|r| scale.to_position(*r.start())..=scale.to_position(*r.end()))
    }
}
```

- `sealed` is `pub(crate)`, not private, so `crate::models` can name `Describe` to implement it
  (core research R-12, xtask C1). Outside the crate it cannot be named, which seals `Actuator`
  (a `compile_fail,E0603` doc example shows it).
- Every method is provided, so X1's emitted `impl Actuator for X { fn id… fn model_number…
  fn name… }` stays valid at core-2, and `impl Actuator for X {}` compiles too.
- The four OQ-2 registers, the fault register and the scale are reachable in-crate only, via
  `Describe`. That keeps register locations crate-internal (xtask clarification) while the bus
  stays generic (core FR-010, FR-011). The public trait exposes no registers (core FR-012).
- One `match` per `Describe` method on `ActuatorKind`, returning `&'static ModelData`. This is
  static dispatch with no runtime address lookup (NFR-4).
- Transition: in core-1, `Describe` exists alone and `Actuator` keeps its three required
  methods. core-2 adds the supertrait and makes all six methods provided (section 7).

### 2.7 Errors after the change

```rust
// src/error.rs. All Debug, Clone, Copy, PartialEq, Eq, thiserror::Error, no allocation.
#[non_exhaustive]
pub enum Error {
    Transport(#[from] TransportError),    // PR #7: Io(ErrorKind), Timeout { id: Option<Id> }
    Protocol(#[from] ProtocolError),      // PR #7: Checksum, Crc, WrongId, WrongLength, Malformed
    Status(#[from] StatusError),          // PR #7: { id, report: StatusReport }, is_hardware_fault()
    Validation(#[from] ValidationError),
    Setup(#[from] SetupError),            // new
}

#[non_exhaustive]
pub enum ValidationError {
    InvalidId(u8),                                        // exists
    AddressOutOfRange { addr: u16 },                      // PR #7
    PacketTooLarge { len: usize, max: usize },            // PR #7
    DataLength { expected: usize, got: usize },           // PR #7 (instruction data size)
    ActuatorIsOff { id: Id },                             // goal write while Off or Faulted
    ActuatorFaulted { id: Id },                           // enable_torque while Faulted
    OutOfLimits { id: Id, raw: i32, min: i32, max: i32 },
    LimitsUnknown { id: Id },
    InvertedLimits { id: Id, min: i32, max: i32 },
    NotFinite,                                            // NaN or infinite angle
    DoesNotFit { raw: i64, size: u8 },                    // size 0 = does not fit i32
    BufferLength { targets: usize, got: usize },          // caller buffer or value slice
    DuplicateId { id: Id },                               // multi-actuator targets, declared list
    /// Goal position is ignored in the actuator's operating mode (§2.4b, `LimitMode::NoPosition`).
    NotPositionMode { id: Id },
}

#[non_exhaustive]
pub enum SetupError {
    /// No compiled-in model for this number on this protocol. `feature` names the family
    /// that would add it, when the model is known but its family is off.
    UnsupportedModel { id: Id, model_number: u16, protocol: ProtocolVersion, feature: Option<&'static str> },
    TooManyActuators { capacity: usize },
    Missing { id: Id },                                   // declared, did not answer
    WrongModel { id: Id, declared: u16, found: u16 },
    NotOnBus { id: Id },                                  // target Id the bus does not hold
    /// Operating Mode / Control Mode value in none of the model's `operating_modes` lists (§2.4b). No fallback.
    UnknownOperatingMode { id: Id, value: u8 },
}
```

- The top-level `Error::UnsupportedModel` moves into `SetupError` (breaking, accepted in alpha).
  It gains `id` and `protocol` so the message names the actuator and the protocol.
- `bus.actuator(id)` returns `None` for an unknown Id, not `NotOnBus`. `NotOnBus` is for
  multi-actuator targets and for explicit calls that must name an Id.
- Transition: core-1 adds `Setup` and keeps the old top-level variant, `#[doc(hidden)]`, for the
  generated code. core-2 removes it.
- `NotPositionMode` and `UnknownOperatingMode` come with §2.4b in core-1. Like every other
  validation refusal, `NotPositionMode` sends zero bytes.

### 2.8 Public re-exports at the end of the chain

`dxl_rs_v2::{Actuator, ActuatorKind, ActuatorRef, Bus, Config, IdSet, field, SerialBus (std),
DynBus (std), Clock, DefaultClock, NoClock, StdClock (std), Error, TransportError, ProtocolError,
StatusError, StatusReport, P1Flag, P1Flags, P2Code, ValidationError, SetupError, Id, ModelNumber,
Position, PositionScale, TorqueState, Protocol, ProtocolVersion, P1, P2, Transport,
SerialTransport (std), MockTransport, MockTime, Mismatch (mock), embedded_io, serialport (std),
models}`.

`IdSet`, `field` and `DynBus` are bus additions (bus plan, deviation 4). `ProtocolVersion` is
already re-exported from the root by PR #7 (`pub use protocol::{Clock, P1, P2, Protocol,
ProtocolVersion};`), so core-1 does not add it.

`Register`, `Access`, `Area`, `Limits`, `Cache`, `LimitMode`, `ModelData`, `ModeSource`,
`ModeValues`, `LimitRegisters`, `Protocols` and `Describe` are never re-exported.

---

## 3. tables → xtask: RON schema

### 3.1 Per table (`control_tables/<file>.ron`)

`operating_modes` (added in T1, architect 2026-10-28): present exactly when the table has an
`Operating Mode` or `Control Mode` row, absent otherwise. Values from that model's e-Manual page;
the lists are disjoint.

```ron
operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),
```

```ron
(
    source: Verified(url: "https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/", verified: "2026-10-28"),
    // or: source: Unverified, and then no `instructions`
    protocols: [P1],                 // [P1], [P2] or [P1, P2]: protocols the e-Manual serves this layout on
    instructions: [                  // verified tables only; codegen ignores it for now
        (protocol: P1, source: "https://emanual.robotis.com/docs/en/dxl/protocol1/",
         list: [Ping, Read, Write, RegWrite, Action, FactoryReset, SyncWrite]),   // AX: no Reboot, no Bulk Read
    ],
    type_info: { /* shape unchanged: position scale; values checked in T1 (§2.3) */ },
    unit_info: [ /* unchanged */ ],
    control_table: [
        { address: 24, size: 1, data_name: "Torque Enable", access: RW, area: RAM },
        // every row: address, size, data_name, access (R | RW), area (EEPROM | RAM)
    ],
)
```

- `source` and `protocols` are required on every table. `access` and `area` are required on
  every row. All come from the e-Manual.
- `source` follows the SRS "Meanwhile" shape `(url, verified)`, tagged so "unverified" is
  explicit. One flag per table.
- `instructions` is free to change shape until xtask reads it. Tables uses
  `(protocol, source, list, from_fw?)`: `source` is the protocol page, since model pages list no
  instructions, and `from_fw` carries firmware conditions such as P1 Reboot on MX-28/64/106
  from V40 (tables research R-5, contracts/ron-schema.md). Verified tables have at least one
  entry per protocol in `protocols`; unverified tables have none.
- A table keeps brace style. xtask's brace-to-paren rewrite stays valid: no real maps appear in
  a table.
- If the e-Manual shows an access other than R or RW (e.g. W), the tables plan raises it before
  writing it. The enum grows only by a contract change.

### 3.2 Model map (`control_tables/dynamixel.ron`)

```ron
(
    models: {
        1020: (family: "x-series", name: "XM430-W350",
               p2: [(from_fw: 0, file: "xm430_w350")],
               p1: [(from_fw: 0, file: "xm430_w350")],
               default: P2),
        12:   (family: "ax", name: "AX-12A", also: ["AX-12+"],
               p1: [(from_fw: 0, file: "ax_12a")]),
        310:  (family: "mx", name: "MX-64", p1: [(from_fw: 0, file: "mx_64_p1")]),
        1190: (family: "x-series", name: "XL330-M077",
               p2: [(from_fw: 0, file: "xl330_m077_fw52"), (from_fw: 53, file: "xl330_m077")], ...),
    },
    not_mapped: [
        (file: "ax_12_plus", reason: SameNumberAs(12)),
        (file: "ax_18f", reason: SameNumberAs(18)),
        (file: "xw430_t200", reason: SameNumberAs(1270)),
    ],
)
```

Shown in the T2 shape (no legacy `tables`, `also` present). T1 writes the three `not_mapped`
records; T2 adds `also` on 12, 18 and 1270.

- Per model number: `p1` and/or `p2`, each a firmware-ordered list (from_fw starts at 0 and
  strictly increases). At least one list is present.
- `default: P2` is required when both lists exist, and must be `P2` (SRS decision). It is
  omitted when only one list exists.
- The same file may appear under `p1` and `p2` of one model number. A file must not appear under
  two model numbers.
- `also`: other product names sold under the same number (FR-012 tables). Docs only.
- `not_mapped` reasons: `NoModelNumber` or `SameNumberAs(u16)`. "Unverified" is never a reason.
  `SameNumberAs(n)` names a key of `models`.
- XW430-T200 and XW430-T333 both give Model Number 1270 on the e-Manual (pages re-checked
  2026-10-28). AX-18F is on the AX-18A page, number 18. AX-12+ is on the AX-12A page, number 12.
- Transition: T1 adds `p1`/`p2`/`default`/`not_mapped` and keeps the legacy `tables:` field (equal
  to the default protocol's list) so today's xtask still reads the map. X1 reads only the new
  fields and ignores `tables`. T2 deletes `tables`.

---

## 4. xtask → bus: generated items

### 4.1 Per model type (`src/models/<module>.rs`, one per table file)

```rust
/// <name> (model number <n>[, also <also...>]), firmware <range>, Protocol 1/2.
/// Source table: `control_tables/<file>.ron`.
/// Unverified: not yet checked against the e-Manual.
///   (or) Verified against <url> on <date>.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Xm430W350 { cache: Cache }

impl Xm430W350 {
    pub(crate) const fn new(id: Id) -> Self;
    pub(crate) const DATA: ModelData = ModelData {
        /* name, number, protocols, roles, hardware_error, */
        mode: ModeSource::Register { register: Self::OPERATING_MODE, values: ModeValues { bounded: &[3], unbounded: &[4, 5], no_position: &[0, 1, 16] } }, // §2.4b, from `operating_modes`
        /* scale */
    };
    pub(crate) const TORQUE_ENABLE: Register = super::reg(64, 1, Access::RW, Area::Ram);
    // ... one crate-internal const per row, as today
}
impl Describe for Xm430W350 {        // crate::actuator::Describe
    fn data(&self) -> &'static ModelData { &Self::DATA }
    fn cache(&self) -> &Cache { &self.cache }
    fn cache_mut(&mut self) -> &mut Cache { &mut self.cache }
}
impl Actuator for Xm430W350 {
    // X1 emits id, model_number, name (core-1 requires them). After core-2 they are valid
    // overrides; the optional X2 regeneration drops them.
}
```

Public: the model type (as a documented `match` target). Crate-internal: `new`, `DATA`, every
register const, the `cache` field. `reg()` is private to `models` (model files call
`super::reg`). Exact shape: specs/006-xtask-codegen/contracts/generated-api.md.

### 4.2 `src/models.rs`

```rust
/// Builds a register from the four `pub(crate)` fields (§2.2). Private to `models`.
const fn reg(addr: u16, size: u8, access: Access, area: Area) -> Register {
    Register { addr, size, access, area }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ActuatorKind {
    #[cfg(feature = "x-series")]
    Xm430W350(Xm430W350),
    // one variant per table file, family-gated
}

/// Model Number location, the same in every table (codegen checks it). Read on P1 Setup.
/// Access and area come from the tables; codegen checks they agree (xtask C4).
pub(crate) const MODEL_NUMBER: Register = reg(0, 2, Access::R, Area::Eeprom);

impl ActuatorKind {
    /// The variant for this model number on this protocol, picking the table whose `from_fw`
    /// is the highest not above `firmware`.
    pub(crate) fn new(protocol: ProtocolVersion, model_number: ModelNumber, firmware: u8, id: Id)
        -> Result<Self, SetupError>;
    /// `Some` when the table depends on firmware on this protocol: the Firmware Version register
    /// to read first (the same location in every variant; codegen checks it). `None` otherwise,
    /// including when the model's family is off (xtask C3).
    pub(crate) fn firmware_register(protocol: ProtocolVersion, model_number: ModelNumber)
        -> Option<Register>;
}
impl Describe for ActuatorKind { /* match over variants */ }
impl Actuator for ActuatorKind { /* id, model_number, name emitted, as in §4.1 */ }

#[cfg(test)]
mod tests;   // generated src/models/tests.rs: lookup at every boundary, family off, cache
```

Lookup results:

| Model number on this protocol | Family on | Family off |
|---|---|---|
| Has tables | `Ok(variant)` | `UnsupportedModel { feature: Some(family) }` |
| Known, but no tables on this protocol | `UnsupportedModel { feature: None }` | same |
| Unknown | `UnsupportedModel { feature: None }` | same |

With no family enabled, `ActuatorKind` has no variants, every lookup fails, and the build has no
warnings.

### 4.3 Codegen checks (all before writing any file)

On top of today's checks (missing file, `from_fw` order, duplicate identifiers, unmapped warning):

- Unknown family. Missing `source` or `protocols` on a mapped table (fails and names the table),
  or empty `protocols`. Missing `access`/`area` on any row.
- No `p1` and no `p2`, or an empty list. A map list (`p1`/`p2`) naming a table whose `protocols`
  lacks that protocol. `default` missing or not `P2` when both lists exist, or present with one
  list. A file under two model numbers. A file both mapped and in `not_mapped`. A `not_mapped`
  entry naming no file.
- Missing role: Torque Enable, Goal Position, Present Position, and one complete limit pair
  (Min/Max Position Limit, or CW/CCW Angle Limit; half a pair or both pairs fail). Roles match
  e-Manual names only; codegen never guesses an address.
- Mode source (§2.4b), exactly one per table (xtask C5, check 17; tables plan, deviation 4): no
  source, an `Operating Mode` row beside the CW/CCW pair, or both mode rows fail. A `Control
  Mode` row outside `xl320` fails the tables check. The mode register counts as a role for the P1
  address check.
- Role attributes: Torque Enable and Goal Position RW and RAM; Present Position R and RAM; limit
  registers RW and EEPROM; mode register 1 byte, RW, EEPROM.
- Model Number not at (0, 2), or its access/area differ between tables. Firmware Version at
  different locations across the variants of one (model number, protocol).
- A table listed under `p1` with a role register above address 255.
- Scale: `max_raw > min_raw`, `max_rad > min_rad`, `zero_raw` in `min_raw..=max_raw`.
- Duplicate type identifier or register constant. A module identifier `tests` or a Rust keyword.

The numbered list is xtask data-model §3: 16 checks, plus check 17 (mode source) from the
xtask plan, C5. Unmapped files without a `not_mapped` record still only warn (FR-7.4). The tables check reports
them as errors, and repeats every data-only check above (tables plan, deviation 4).

---

## 5. Protocol items the bus uses (done: merged in PR #7, 2026-10-28)

Not one of the five modules. Everything below is on `main` (commits `e5f460c`, `b2c9a9e`). The
signatures are the merged code. A further need goes to a protocol follow-up PR.

```rust
// src/protocol.rs. Root re-export: pub use protocol::{Clock, P1, P2, Protocol, ProtocolVersion};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProtocolVersion { P1, P2 }           // Display: "Protocol 1" / "Protocol 2"

pub trait Protocol: sealed::Sealed + Exchange {
    const BROADCAST: u8 = 0xFE;
    const VERSION: ProtocolVersion;
}

// `Link`, `Pong` and `Exchange` are `pub` in the private `protocol` module: crate-internal.
pub struct Link<'a, T, C> {
    pub(crate) transport: &'a mut T,
    pub(crate) clock: &'a mut C,
    pub(crate) timeout: Duration,             // wait for each expected reply
}

pub struct Pong {
    pub(crate) id: Id,
    pub(crate) model_number: Option<u16>,     // P2 only
    pub(crate) firmware: Option<u8>,          // P2 only
    pub(crate) fault: Option<StatusReport>,   // scan / broadcast ping only; unicast ping gives Err(Status)
}

pub trait Exchange: sealed::Sealed {
    // also: ping, read, write, sync_write, report
    /// Finds every actuator at this baud and calls `on_reply` for each answer.
    /// P2: one broadcast Ping, replies collected for one `link.timeout`.
    /// P1: pings 0..=252 in turn, each bounded by `link.timeout`; a silent Id is skipped. A reply
    /// with its error byte set becomes `Ok(Pong { fault: Some(report), .. })`.
    /// A corrupted reply (checksum or CRC, malformed, wrong Id or length) is passed on as `Err`.
    /// Returns `Err` only on `Transport(Io)`.
    fn scan<T: Transport, C: Clock, F>(link: &mut Link<'_, T, C>, on_reply: F) -> Result<(), Error>
    where F: FnMut(Result<Pong, Error>);

    /// Reads `size` bytes at `addr` from every Id in `ids`. P2: one Sync Read (`P2::sync_read`).
    /// P1: one Read per Id, in order, each bounded by `link.timeout`. `on_reply` is called exactly
    /// once per Id, in order, with its data or its own error (timeout, status, corrupted reply).
    /// Checked first, before any byte, returning `Err(Validation)`: P1 `addr` ≤ 255
    /// (`AddressOutOfRange`), `size` ≤ 253 on P1 / 65531 on P2 (`PacketTooLarge`), `scratch` ≥
    /// `size` (`DataLength`). After that, only `Transport(Io)` returns `Err` from the function.
    fn read_many<T: Transport, C: Clock, I, F>(
        link: &mut Link<'_, T, C>, addr: u16, size: u16, ids: I, scratch: &mut [u8], on_reply: F,
    ) -> Result<(), Error>
    where I: Iterator<Item = Id> + Clone, F: FnMut(Id, Result<&[u8], Error>);
}

// src/protocol/clock.rs
#[diagnostic::on_unimplemented(message = "`{Self}` is not a dxl_rs_v2 clock", note = "…")]
pub trait Clock { fn now_us(&mut self) -> u64; }   // any FnMut() -> u64; StdClock under std
```

- `P2::sync_read` (`pub(crate)`, inherent) stays the explicit-tier entry (`impl Bus<P2, …>` only).
- `P2::broadcast_ping` (`pub(crate)`) is `scan` with corrupted replies dropped. Setup uses `scan`
  (conflict 16), so nothing calls it; core-1 deletes it and the `expect(dead_code)` on
  `mod protocol;` (section 1, architect 2026-10-28).
- On a P2 scan, a reply with its error byte set but not 3 data bytes is `Err(Status)`, and one
  from an invalid Id is `Err(Protocol(Malformed))`.
- `on_unimplemented` makes a no_std `Bus::new` without a named clock report "not a dxl_rs_v2
  clock" instead of a bare E0277 (conflict 15).

---

## 6. bus: shape

```rust
// src/bus.rs
#[cfg(feature = "std")]      pub type DefaultClock = StdClock;
#[cfg(not(feature = "std"))] pub type DefaultClock = NoClock;
/// Always present. Implements no `Clock`, so a no_std bus must name its clock.
pub enum NoClock {}

/// No `C: Clock` bound on the struct: rustc checks a default against the bound, and `NoClock`
/// is not a `Clock`, so every no_std build would fail (bus research R-2). `C: Clock` is on
/// every impl.
pub struct Bus<P: Protocol, T: Transport, C = DefaultClock, const N: usize = 32> {
    transport: T,
    clock: C,
    config: Config,
    actuators: heapless::Vec<ActuatorKind, N>,   // sorted by Id
    states: [TorqueState; N],                    // states[i] belongs to actuators[i]; N bytes
    _protocol: core::marker::PhantomData<P>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Config { pub timeout: Duration, pub scan_timeout: Duration, pub retries: u8 } // 10 ms, 10 ms, 0
impl Config {
    pub const fn new() -> Self;
    pub const fn timeout(self, t: Duration) -> Self;
    pub const fn scan_timeout(self, t: Duration) -> Self;
    pub const fn retries(self, n: u8) -> Self;
}
impl Default for Config;   // = new()

impl<P: Protocol, T: Transport, C: Clock, const N: usize> Bus<P, T, C, N> {
    pub fn new(transport: T, clock: C, config: Config) -> Self;            // no I/O; N > 253 fails to compile
    pub fn into_parts(self) -> (T, C);
    pub fn config(&self) -> Config;
    pub fn autoscan(&mut self) -> Result<&[ActuatorKind], Error>;          // no_std too
    pub fn declare<I: IntoIterator<Item = (Id, ModelNumber)>>(&mut self, list: I)
        -> Result<&[ActuatorKind], Error>;
    pub fn actuators(&self) -> &[ActuatorKind];
    pub fn ids(&self) -> IdSet;
    pub fn actuator(&mut self, id: Id) -> Option<ActuatorRef<'_, P, T, C>>;
    pub fn torque_state(&self, id: Id) -> Option<TorqueState>;
    pub fn torque_states(&self) -> &[TorqueState];
    // Easy and Explicit tiers: names and buffers per the bus plan (FR-4, contracts/bus-api.md)
}
impl<T: Transport, C: Clock, const N: usize> Bus<P2, T, C, N> { /* sync_read */ }

#[cfg(feature = "std")]
pub type SerialBus<P, const N: usize = 32> = Bus<P, SerialTransport, StdClock, N>;
#[cfg(feature = "std")]
impl<P: Protocol, const N: usize> SerialBus<P, N> {
    pub fn open(path: &str, baud: u32) -> Result<Self, serialport::Error>;
}

// src/bus/actuator_ref.rs. No N in the type.
pub struct ActuatorRef<'b, P: Protocol, T: Transport, C: Clock> {
    kind: &'b mut ActuatorKind,
    state: &'b mut TorqueState,
    wire: Wire<'b, P, T, C>,        // crate-internal: &mut T, &mut C, &Config
}
impl<P: Protocol, T: Transport, C: Clock> ActuatorRef<'_, P, T, C> {
    pub fn kind(&self) -> &ActuatorKind;                       // Tier 2: match
    pub fn torque_state(&self) -> TorqueState;
    pub fn enable_torque(&mut self) -> Result<(), Error>;
    pub fn disable_torque(&mut self) -> Result<(), Error>;
    pub fn set_goal_position(&mut self, goal: Position) -> Result<(), Error>;
    pub fn set_goal_position_raw(&mut self, raw: i32) -> Result<(), Error>;
    pub fn goal_position(&mut self) -> Result<Position, Error>;      // explicit read-back (FR-3.8)
    pub fn goal_position_raw(&mut self) -> Result<i32, Error>;
    pub fn present_position(&mut self) -> Result<Position, Error>;
    pub fn present_position_raw(&mut self) -> Result<i32, Error>;
    pub fn clear_fault(&mut self) -> Result<(), Error>;
}
impl<P: Protocol, T: Transport, C: Clock> Actuator for ActuatorRef<'_, P, T, C> {} // via Describe delegation
```

How the bus uses lower layers:

- Only `src/bus/wire.rs` builds a `protocol::Link { transport, clock, timeout }` and calls
  `Exchange`. Unicast calls use `config.timeout`, `scan` uses `config.scan_timeout`. It holds the
  retry loop: unicast only, on timeout or protocol corruption only, never on `Status`, never on
  `sync_write`, `scan` or `read_many` (bus research R-5).
- Setup (bus research R-6). `declare` checks the whole list before any byte: overflow
  (`TooManyActuators`), a repeated Id (`DuplicateId`), and `ActuatorKind::new(P::VERSION, n, 0,
  id)` for each entry (`UnsupportedModel`). Then per actuator:
  1. Ping (`declare`; a timeout gives `Missing`) or scan reply (`autoscan`; an `Err` reply, or a
     `Pong` with `fault` set, fails Setup).
  2. Model number: from the `Pong` on P2; on P1, read `models::MODEL_NUMBER`. A declared model
     that differs gives `WrongModel`.
  3. If `ActuatorKind::firmware_register(P::VERSION, n)` is `Some`, read it (on P2 the `Pong`
     already carries the firmware).
  4. Call `ActuatorKind::new(P::VERSION, n, fw, id)`.
  5. Read the limits via `data().limits.min()/max()` and call `cache_mut().record_limits(..)`.
  6. Mode (§2.4b): if `data().mode.register()` is `Some`, read that 1 byte and map it with
     `mode.limit_mode(value)`; `None` gives `UnknownOperatingMode { id, value }`. For
     `AngleLimits`, `LimitMode::from_angle_limits` on the limits of step 5, no read. Then
     `cache_mut().record_mode(..)`.
  7. Set the slot to `TorqueState::Off`.

  Any error leaves the list empty. A fault reported at Setup fails Setup.
- Goal path: `state.check_goal(id)?`, then `scale.to_raw`, then `cache.check_goal(raw)?`
  (mode-aware: `NotPositionMode`, `LimitsUnknown`, `OutOfLimits`), then `Register::encode`, then
  send. Every refusal sends zero bytes.
- Every status reply runs `observe`, `raw-registers` included (§2.5).
- `partial-reads` (bus-5): inside a P2 sync group each actuator keeps its own result; only a
  failed instruction (`read_many` returns `Err`) fails every slot of the group (architect,
  2026-10-28).
- `DynBus` (`std`, FR-1.3) is bus-internal design. It must not change `Bus`.

---

## 7. Delivery order

```
protocol PR #7 (with section 5) ── merged 2026-10-28
   │
   ├── core-1 ─────────┐        tables T1  (no dependency; parallel with core-1)
   │                   ▼            │
   │               xtask X1 ◄───────┘
   │                   │
   │                core-2 ──────────► bus-1 … bus-6
   │                   │
   │                   ├─(optional)──► xtask X2
   │                   │
   │                   └─(after X1)──► tables T2   (parallel with core-2 and bus)
   │
   └── ci  (after #7 and core-1; T1 optional, its table check is guarded until T1)
```

| Step | Module | Content | Needs |
|---|---|---|---|
| P | protocol | Section 5. **Merged in PR #7, 2026-10-28.** | — |
| core-1 | core | Every type in section 2, including `LimitMode`, `ModeSource`, `ModeValues`, the value mappings, `Cache::record_mode` and the mode-aware `check_goal` (§2.4b), `NotPositionMode` and `UnknownOperatingMode` (§2.7). Standalone `Describe` (`pub(crate) mod sealed`), `SetupError` + `Error::Setup`. Shims: old `Error::UnsupportedModel` (`#[doc(hidden)]`), 2-argument `Register::new(addr, size)` (`#[doc(hidden)]`, unconditional `allow(dead_code)`; no 4-argument `new`), `Actuator` with three required methods. Drops its `ActuatorKind::new` tests (core D2). Fixes the no-family warning. Builds against today's generated code with no regeneration. | P (merged) |
| T1 | tables | Tools and tests (`check_tables.py`, `emanual.py`). `source` and `protocols` on all 72 tables (`Unverified`; AX-12A `Verified` with `instructions`). `access` + `area` on every row. `type_info` checked and fixed (§2.3). `p1`/`p2`/`default` beside legacy `tables`. `not_mapped` for `ax_12_plus`, `ax_18f`, `xw430_t200`, all `SameNumberAs`. Findings list. No generated diff (today's xtask ignores the new fields). | — |
| X1 | xtask | Reads section 3. Emits section 4: `Describe`, `DATA` with `mode` (§2.4b), `cache` field, registers built by a private generated `reg()` helper (never `Register::new`), per-protocol lookup returning `SetupError`, `firmware_register`, `MODEL_NUMBER`, the checks of §4.3 (mode source included), verification docs, generated `src/models/tests.rs`. Still emits `id`/`model_number`/`name` in `impl Actuator`. Commits the output regenerated from T1's tables. | core-1, T1 |
| core-2 | core | `Actuator: Describe` with all six methods provided; removes the shims (no `Register::new` remains). Generic tests over every compiled-in model. Generated overrides stay valid. | X1 |
| X2 | xtask | Optional. Drops the generated `id`/`model_number`/`name` overrides and regenerates. | core-2 |
| T2 | tables | `mx_28_p1`, `mx_64_p1`, `mx_106_p1` + map entries 29, 310, 320 (`p1` only). `also` on 12, 18, 1270. Verification of MX-64(2.0), AX-12+ and MX-64 P1. MX-64(2.0) waits for T2 because removing its alias rows changes generated constants (tables research R-9, R-14). Alias rows removed from 66 tables, other T1 findings applied, `RH-P12-RN(A)` name fix (35074), legacy `tables` removed, regenerated `src/models*` committed. | X1 |
| bus-1 … bus-6 | bus | As in specs/003-bus/plan.md (PR split). bus-1 adds `heapless` and reads the mode at Setup (§6 step 6). bus-2 uses `scan`, bus-4 `read_many`. bus-4 needs bus-2 and bus-3 (the SC-001 example) and creates `tests/bus_no_alloc.rs`; bus-5 and bus-6 extend it. bus-5 adds `partial-reads`/`raw-registers` to `Cargo.toml`; raw replies run `observe`. Bus edits only its own `lib.rs` lines (section 1). bus-2/bus-3 and bus-5/bus-6 are independent. | core-2, X1, P (merged) |
| ci | ci | Gates from spec 007, one PR. The feature-set job needs core-1 (today `--no-default-features` fails under `-D warnings`). The missing-docs gate needed #7 (it documents the `UnsupportedModel` fields; merged). The `control tables: schema check` job runs T1's script; it is guarded until T1 merges, and the first ci change after T1 removes the guard. A ci-1/ci-2 split, only if core-1 slips, needs its own approval (ci plan). | P (merged), core-1, T1 (optional) |

Core, tables and bus each ship as more than one PR. The architect approved this split under
"one module per PR" on 2026-10-28.

Each plan and its tasks carry the 2026-10-28 items of its PRs. A plan or task list dated
2026-10-27 only does not yet: core-1 `LimitMode`, `ModeSource`, `ModeValues`, `record_mode`,
mode-aware `check_goal`, the two new error variants and their messages; X1 `ModelData::mode` and
check 17; T1 the mode-source rule in the table check; bus-1 the mode read at Setup and
`NotPositionMode` in place of "limits apply in every operating mode"; bus-5 `observe` on raw
replies and per-actuator partial results.

---

## 8. Conflicts found, with resolutions

| # | Conflict | Resolution in this contract |
|---|---|---|
| 1 | xtask FR-009, US1 AS5: the cache holds torque state and fault state. Core FR-008/009, bus FR-024, SRS patch: one byte per slot in a bus array. | Cache = `Id` + limit mode (§2.4b) + limits. `TorqueState` lives in `Bus::states`. xtask's plan drops torque/fault from the cache. |
| 2 | xtask Assumptions: "register access not in tables, not generated". Core Assumptions: `Register` is address + size only (gap). Tables FR-007 + architect: access and area on every row now. | `Register` carries `access` and `area`. xtask requires and emits them, and checks role attributes. |
| 3 | xtask Assumptions/Edge: MX on P1 is roadmap. Tables Clarification + SRS step 7: MX P1 tables are written now. | In scope. X1 generates a per-protocol lookup. T2 adds the tables. |
| 4 | xtask FR-006: lookup by model number + firmware. Tables FR-008: per-protocol map. | `ActuatorKind::new(ProtocolVersion, ModelNumber, fw, Id)`. Needs `Protocol::VERSION` (section 5, merged in PR #7). |
| 5 | xtask FR-012: protocol set per model type. Tables FR-005/008: protocol per table and per map entry. | Both, kept consistent: the table's `protocols` must include every map list naming it (codegen check). `ModelData::protocols` comes from the table. |
| 6 | Bus FR-012: a model unreachable on the bus protocol fails Setup with its own error. Tables US4 AS4: the bus reports `UnsupportedModel`. Core FR-016 has no such variant. | One variant: `UnsupportedModel` gains `protocol` (and `id`), and covers unknown numbers, families that are off, and missing tables on this protocol. Accepted by the architect 2026-10-28. |
| 7 | SRS FR-6.4 and core FR-016: "`Id` not on the bus" is a Setup error. Bus US5 AS4 and FR-034: an unknown target Id is a validation error. | SRS wins (higher authority): `SetupError::NotOnBus`. Repeated Id: `ValidationError::DuplicateId`. Both still send nothing. `bus.actuator(id)` returns `None`. Accepted by the architect 2026-10-28. |
| 8 | Bus FR-026: enabling torque while faulted is refused with a typed error. Core FR-015 lists no such variant. | Add `ValidationError::ActuatorFaulted { id }`. |
| 9 | Core spec: `ModelNumber` is a bus item. xtask spec: core defines it. | Core (`src/actuator.rs`). |
| 10 | Core spec: core and xtask sequencing left open; the trait change breaks generated code. xtask FR-018 and the no-go rules forbid cross edits. | core-1 (additive + shims) → X1 → core-2 (cutover). No hand edits to generated code. |
| 11 | Tables FR-016 (keep today's codegen working) vs the new map format. | T1 keeps the legacy `tables:` beside `p1`/`p2`. X1 ignores it. T2 removes it. |
| 12 | SRS §9 gives `Cargo.toml` to ci, and ci FR-015 forbids feature and dependency edits. Bus needs `heapless`, `partial-reads`, `raw-registers`. PR #7 added `proptest`. | Split ownership (section 1). The module that needs a line adds it and flags it. |
| 13 | Bus proposal: compile-fail via trybuild `.stderr`. ci FR-008: rustdoc `compile_fail,EXXXX` checked on a pinned toolchain. | Compile-fail cases are rustdoc examples with error codes, so the ci check covers them. No `trybuild` dependency. |
| 14 | ci Assumptions: the no-feature build is green on day one. xtask Assumptions: a no-family build warns. Checked on `dev` and PR #7 with `-D warnings` (ci research R-16): `cargo build --lib --no-default-features` fails (`Register::new` never used). | core-1 fixes it. The ci feature-set job lands after core-1. |
| 15 | Bus proposal: `C: Clock = fn() -> u64`. Architect: zero bytes, no indirect call, system clock under std. A cfg-switched default must not break additivity. | `DefaultClock` = `StdClock` (std) or `NoClock` (no_std, not a `Clock`). No_std users must name a clock, and enabling std only makes more code compile. The clock comes before `N`. Accepted by the architect 2026-10-28. |
| 16 | PR #7 `broadcast_ping` skips corrupted replies. Bus spec edge case: a corrupted reply fails Setup with a protocol error. | `scan` passes corrupted replies on as `Err`. The bus fails Setup (nothing hidden). A protocol change, section 5, merged in PR #7. Accepted by the architect 2026-10-28. |
| 17 | Core FR-012: no registers beyond OQ-2 in the trait. xtask FR-008/011: fault register, Firmware Version, Model Number exposed. | Those are crate-internal (`ModelData`, `firmware_register`, `MODEL_NUMBER`), not on the public `Actuator` trait. |
| 18 | Bus Assumptions: "position scale … not in the tables yet". All 72 tables have `type_info`. | The scale comes from `type_info` (zero + slope). T1 fixes the AX angles, the XL-320 range and the 2π/4096 slope against each page (§2.3). Model Number (0, 2) and Firmware Version (2 or 6) are table rows. |
| 19 | Core FR-016 names `UnsupportedModel { model_number, feature }`. | Fields `id` and `protocol` are added (DX: name the actuator and the protocol). Accepted by the architect 2026-10-28. |
| 20 | ARCHITECTURE.md: "Torque state = runtime field on the actuator", "Actuator cache: Id, torque state, limits", `Bus<P, T, N>`, autoscan std-only. `docs/srs/modules/bus.md` still has `const N` before `C`. | Superseded by the 2026-10-27 and 2026-10-28 decisions. ARCHITECTURE.md and `docs/srs/modules/*.md` need the architect's update (section 9). Agents do not edit them. |
| 21 | SRS FR-3.7 (Decided): limits are unconditional. e-Manual: X-series Min/Max Position Limit "not used in Extended Position Control Mode and Current-based Position Control Mode"; AX CW = CCW = 0 is wheel mode (bus spec conflict 8, bus research R-9). The bus plan kept the unconditional check. | Resolved by §2.4b (architect 2026-10-28): limits follow the operating mode, read once at Setup. |
| 22 | Bus plan: `raw-registers` replies return a hardware fault but leave the state alone, an exception to constitution III. | Architect 2026-10-28: raw replies run `observe`; a fault sets `Faulted`. No exception (§2.5). |
| 23 | SRS FR-4.7: "On P2 a failed sync group fails all slots in that group". Bus plan SRS deviation 1: per-Id results inside the group. | Architect 2026-10-28: each actuator keeps its own result; only a failed instruction fails the group (§6). |
| 24 | §6 had `C: Clock = DefaultClock` on the `Bus` struct. rustc checks the default against the bound, and `NoClock` is not a `Clock` (bus plan deviation 1, research R-2). | No bound on the struct; `C: Clock` on every impl (§6). The "name your clock" error stays, with PR #7's `on_unimplemented` text. |

---

## 9. For the architect

Open:

- Update ARCHITECTURE.md to the 2026-10-27 and 2026-10-28 decisions (conflict 20).
- Patch the SRS (architect-only) to the 2026-10-28 decisions: FR-3.7 still says limits are
  unconditional (now §2.4b); FR-4.7 still says a failed P2 sync group fails all slots (now
  per-actuator results, §6); FR-8.1 could say raw replies still mark faults (§2.5); FR-2.4 and
  constitution Principle II still write `UnsupportedModel { model_number, feature }` (conflict
  19); the roadmap line "AX-12+, AX-18F, XW430-T200 … Adding the number is enough" is stale (the
  numbers are shared, §3.2). Then regenerate `docs/srs/modules/*.md`; `bus.md` also predates the
  clock-before-`N` change.
- RX-28 (architect 2026-10-28; tables spec Clarifications): T1 adds `rx_28.ron` (verified,
  Protocol 1, mode source CW/CCW Angle Limit, no `operating_modes`), maps 28 with family `rx`
  and adds the default Cargo feature `rx`. To follow: the constitution's feature list and the
  SRS gain `rx`; X1 accepts family `rx` and its model counts include RX-28; ci adds `rx` to the
  no_std feature set; `CLAUDE.md`'s no_std command lists `rx`.
- AX-18F is recorded as `SameNumberAs(18)` (tables plan, deviation 2): the AX-18A page covers
  "AX-18F, AX-18A", number 18, same layout. Data only; confirm.
- `cache_mut` misuse (core research R-12): generic user code bounded on `A: Actuator` can call
  `cache_mut()` and copy one actuator's `Cache` (Id, mode, limits) over another's. Provisional
  (2026-10-29, agent, to unblock X1): no token; `cache_mut` stays as specified and its doc says it
  is for the bus. A token can still be added before 1.0 (breaking changes allowed in alpha).
- Provisional 2026-10-29 (tables spec, Session 2026-10-29): `Area` gains `Hybrid` (Y series),
  added to core-1; X1 maps `area: Hybrid` to it. PRO External Port Data `R/RW` is recorded `RW`.
- T1 follow-ups (2026-10-29, PR #9): the elided-block rule was also applied to PRO/PRO+/RH
  Indirect Address/Data Read (296, 698) and Y (384, 698, Error Code History 3–14 at 156–167);
  Y `type_info` corrected on 8 tables from the pages' Resolution rows (A051/R051 ±13,369,344,
  A099/R099 ±25,952,256), while every Y page also says "Approx. 0.006866 deg" per pulse, which
  matches none; `rh_p12_rn` is a gripper (0 open, 740 closed) with no radian scale on its page,
  `type_info` left as is: decide what scale a gripper carries; M42-10-S260-R(A) gives 526,375
  pulse/rev against a ±263,187 range.
- core-2 (PR #27, 2026-10-29): Y scales from T1 (±13,369,344 and ±25,952,256 ticks for ±π) are
  finer than an `f32` angle near ±π holds, so ticks round-trip within 1 and 3 ticks there (every
  other scale is exact); the every-tick test allows that on those two scales only and `Position`'s
  docs say so. Decide together with the Y `type_info` question (an `f64` angle, or keep). Also: the
  generated `src/models/tests.rs` trips `clippy::single_element_loop` on single-family test
  builds (xtask's, not run by ci); X2 (drop the generated `id`/`model_number`/`name` overrides) is
  now possible.
- T2 (PR #28, 2026-10-29), questions in its PR and `specs/005-tables/findings/t2-*.md`: `mx_64`
  diff still lists 22 page-only indirect rows (T060 over T063); Clear listed for MX(2.0) P2;
  MX-64/MX-106 P1 "Torque Ctrl Mode Enable" (70) makes the actuator ignore Goal Position, which
  the AngleLimits mode source cannot see; rows with a firmware condition (X Startup Configuration,
  Backup Ready from V45; MX P1 Realtime Tick from v40) have no format; MX P1 page contradicts
  itself on multi-turn (note says CW = CCW = 0, table says 4095; table followed); table-tools §1
  check 14 should name Hardware Error Status; MX-106 P1 Drive Mode reverse bit is not modelled.
- X1 deltas from T1 (2026-10-29): family `rx` is valid (Cargo feature `rx` exists); `area` has a
  third value `Hybrid` (serde name `Hybrid`) mapped to core's `Area::Hybrid`.
- Integration (user, 2026-10-29): agents no longer wait for merges. Finished branches are merged
  into branch `integration` (worktree `.worktrees/int`; every worktree lives under `.worktrees/`); each next branch starts from it and its
  PR into `main` says which PRs it is stacked on. "Merged into `main`" in the task lists reads
  "merged into `integration`".
