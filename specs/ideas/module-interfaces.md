# Cross-module interface contract (core, tables, xtask, bus, ci)

**Status**: Draft contract, 2026-10-27. Input for the five parallel `/speckit-plan` runs of
specs 003 (bus), 004 (core), 005 (tables), 006 (xtask), 007 (ci).

**Sources**: SRS (with the 2026-10-27 architect answers, patch `/tmp/dxl-srs-bus.patch`),
constitution v1.2.0, ARCHITECTURE.md, the five specs and their Clarifications,
`specs/ideas/bus-proposal.md`, the protocol module on branch `001-protocol-packets` (PR #7), and
`src/` + `xtask/` on `dev`.

**Rule for planners**: every plan uses the names, signatures, file owners and order below. A
plan that needs something else raises it as a deviation in its Architecture section. It does
not silently redefine an item another module owns. Signatures are the contract; bodies, private
helpers, tests and doc text are not. "Crate-internal" means `pub(crate)`, or a `pub` item inside
a private module (the sealed pattern PR #7 already uses for `Link`, `Pong` and `Exchange`).

---

## 1. File ownership

| File or path | Owner | Others may |
|---|---|---|
| `src/lib.rs` | core | Add only their own `mod x;` line and their own `pub use x::…` lines. List them in the PR. One exception: the last of bus-2/3/4 to merge (the one that leaves no protocol item unused) deletes PR #7's `#[cfg_attr(not(test), expect(dead_code, …))]` on `mod protocol;`, listed in that PR (bus plan, deviation 6). |
| `src/error.rs` | core | Protocol PR #7 already adds `Transport`, `Protocol`, `Status` (accepted). No other module edits it. A missing variant is raised to core. |
| `src/id.rs`, `src/actuator.rs`, `src/register.rs`, `src/units.rs`, `src/units/**` | core | Nothing. |
| `src/protocol.rs`, `src/protocol/**` | protocol (PR #7, not one of the five) | Nothing. Needed changes are listed in section 5. |
| `src/transport.rs`, `src/transport/**` | transport (merged) | Nothing. |
| `src/models.rs`, `src/models/**` | xtask (generated only) | A tables PR commits the regenerated output when it changes a table. Nobody edits by hand. |
| `xtask/**` | xtask | Nothing. |
| `control_tables/**` | tables | Nothing. |
| `src/bus.rs`, `src/bus/**` | bus | Nothing. |
| `.github/workflows/**` (never `ov-*.yml`) | ci | Nothing. |
| `Cargo.toml` `[package]`, metadata, `[lints]` | ci | Nothing. |
| `Cargo.toml` `[features]`, `[dependencies]`, `[dev-dependencies]` | The module that first needs the line, in its own PR, flagged as a shared-file touch | ci never adds or removes features or dependencies (ci FR-015). |
| `docs/srs/SRS.md`, constitution, `.aiignore`, `ov-*` files | architect | Nothing (no-go). |
| `docs/ARCHITECTURE.md` | architect | Nothing. Stale lines are listed in section 9. |

Known `Cargo.toml` touches: `heapless` (bus-1), `partial-reads` and `raw-registers` features
(bus-5). Protocol PR #7 already adds dev-dependency `proptest`. Family features
`ax, mx, x-series, y-series, pro` do not change.

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
  verification. A register with another sign rule needs a contract change.
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
slope, so the AX `type_info` quirk (`min_radian: 0.0` with zero at 512) does not shift angles. The
tables module still checks it against the e-Manual when AX-12 is verified.

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

No constructor fills limits from a control table (FR-007 core).

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
  applies on every path except `raw-registers`, which returns the error and leaves the state
  alone (bus spec, proposal Q5).
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

mod sealed {
    /// What the bus needs from a model. No I/O.
    pub trait Describe {
        fn data(&self) -> &'static ModelData;
        fn cache(&self) -> &Cache;
        fn cache_mut(&mut self) -> &mut Cache;
    }
}

/// What every model shares. Sealed: only generated types (and `ActuatorKind`) implement it.
/// Describes a model; performs no I/O (I/O is on `ActuatorRef`).
pub trait Actuator: sealed::Describe {
    fn id(&self) -> Id { self.cache().id() }
    fn model_number(&self) -> u16 { self.data().model_number }
    fn name(&self) -> &'static str { self.data().name }
    fn position_scale(&self) -> PositionScale { self.data().scale }
    /// `None` until Setup read the limits from EEPROM.
    fn position_limits_raw(&self) -> Option<core::ops::RangeInclusive<i32>>;
    fn position_limits(&self) -> Option<core::ops::RangeInclusive<Position>>;
}
```

- The four OQ-2 registers, the fault register and the scale are reachable in-crate only, via
  `Describe`. That keeps register locations crate-internal (xtask clarification) while the bus
  stays generic (core FR-010, FR-011). The public trait exposes no registers (core FR-012).
- One `match` per `Describe` method on `ActuatorKind`, returning `&'static ModelData`. This is
  static dispatch with no runtime address lookup (NFR-4).
- Transition: in core-1, `Describe` exists alone and `Actuator` keeps its three required
  methods. core-2 adds the supertrait and the provided methods (section 7).

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
}
```

- The top-level `Error::UnsupportedModel` moves into `SetupError` (breaking, accepted in alpha).
  It gains `id` and `protocol` so the message names the actuator and the protocol.
- `bus.actuator(id)` returns `None` for an unknown Id, not `NotOnBus`. `NotOnBus` is for
  multi-actuator targets and for explicit calls that must name an Id.
- Transition: core-1 adds `Setup` and keeps the old top-level variant, `#[doc(hidden)]`, for the
  generated code. core-2 removes it.

### 2.8 Public re-exports at the end of the chain

`dxl_rs_v2::{Actuator, ActuatorKind, ActuatorRef, Bus, Config, SerialBus (std), Clock,
DefaultClock, NoClock, StdClock (std), Error, TransportError, ProtocolError, StatusError,
StatusReport, P1Flag, P1Flags, P2Code, ValidationError, SetupError, Id, ModelNumber, Position,
PositionScale, TorqueState, Protocol, ProtocolVersion, P1, P2, Transport, SerialTransport (std),
MockTransport, MockTime, Mismatch (mock), embedded_io, serialport (std), models}`.

`ProtocolVersion` is re-exported from the root (`pub use protocol::ProtocolVersion;`). core-1
adds that line if PR #7 does not.

`Register`, `Access`, `Area`, `Limits`, `Cache`, `ModelData`, `LimitRegisters`, `Protocols` and
`Describe` are never re-exported.

---

## 3. tables → xtask: RON schema

### 3.1 Per table (`control_tables/<file>.ron`)

```ron
(
    source: Unverified,
    // or: source: Verified(url: "https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/", verified: "2026-10-28"),
    protocols: [P1],                 // [P1], [P2] or [P1, P2]: protocols the e-Manual serves this layout on
    instructions: [                  // optional; verified tables only; codegen ignores it for now
        (protocol: P1, list: [Ping, Read, Write, RegWrite, Action, FactoryReset, Reboot, SyncWrite]),
    ],
    type_info: { /* unchanged: position scale */ },
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
- `instructions` is free to change shape until xtask reads it.
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
        (file: "xw430_t200", reason: NoModelNumber),
        (file: "ax_12_plus", reason: SameNumberAs(12)),
    ],
)
```

- Per model number: `p1` and/or `p2`, each a firmware-ordered list (from_fw starts at 0 and
  strictly increases). At least one list is present.
- `default: P2` is required when both lists exist, and must be `P2` (SRS decision). It is
  omitted when only one list exists.
- The same file may appear under `p1` and `p2` of one model number. A file must not appear under
  two model numbers.
- `also`: other product names sold under the same number (FR-012 tables). Docs only.
- `not_mapped` reasons: `NoModelNumber` or `SameNumberAs(u16)`. "Unverified" is never a reason.
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
    pub(crate) const DATA: ModelData = ModelData { /* name, number, protocols, roles, scale */ };
    pub(crate) const TORQUE_ENABLE: Register = reg(64, 1, Access::RW, Area::Ram);
    // ... one crate-internal const per row, as today
}
impl sealed::Describe for Xm430W350 {
    fn data(&self) -> &'static ModelData { &Self::DATA }
    fn cache(&self) -> &Cache { &self.cache }
    fn cache_mut(&mut self) -> &mut Cache { &mut self.cache }
}
impl Actuator for Xm430W350 {}   // X1 still emits id/model_number/name until core-2 lands
```

Public: the model type (as a documented `match` target). Crate-internal: `new`, `DATA`, every
register const, the `cache` field. `reg()` is private to `models` (model files call
`super::reg`).

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
pub(crate) const MODEL_NUMBER: Register = reg(0, 2, Access::R, Area::Eeprom);

impl ActuatorKind {
    /// The variant for this model number on this protocol, picking the table whose `from_fw`
    /// is the highest not above `firmware`.
    pub(crate) fn new(protocol: ProtocolVersion, model_number: ModelNumber, firmware: u8, id: Id)
        -> Result<Self, SetupError>;
    /// `Some` when the table depends on firmware on this protocol: the Firmware Version register
    /// to read first (the same location in every variant; codegen checks it).
    pub(crate) fn firmware_register(protocol: ProtocolVersion, model_number: ModelNumber)
        -> Option<Register>;
}
impl sealed::Describe for ActuatorKind { /* match over variants */ }
impl Actuator for ActuatorKind {}
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

- Unknown family. Missing `source` or `protocols` on a mapped table (fails and names the table).
  Missing `access`/`area` on any row.
- A map list (`p1`/`p2`) naming a table whose `protocols` lacks that protocol. `default` missing
  or not `P2` when both lists exist. A file under two model numbers. A file both mapped and in
  `not_mapped`. A `not_mapped` entry naming no file.
- Missing role: Torque Enable, Goal Position, Present Position, and one complete limit pair
  (Min/Max Position Limit, or CW/CCW Angle Limit). Roles match e-Manual names only; codegen never
  guesses an address.
- Role attributes: Torque Enable and Goal Position RW and RAM; Present Position R and RAM; limit
  registers RW and EEPROM.
- Model Number not at (0, 2). Firmware Version at different locations across the variants of one
  (model number, protocol).
- A table listed under `p1` with a role register above address 255.
- Scale: `max_raw > min_raw`, `max_rad > min_rad`, `zero_raw` in `min_raw..=max_raw`.

Unmapped files without a `not_mapped` record still only warn (FR-7.4). The tables check reports
them as errors.

---

## 5. Protocol module changes the bus needs (PR #7 or a protocol follow-up)

Not one of the five modules. The preferred route is to fold these into PR #7 before merge.
`ProtocolVersion` is needed before core-1. The hooks are needed before bus autoscan and the Easy
tier reads.

```rust
// src/protocol.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProtocolVersion { P1, P2 }           // Display: "Protocol 1" / "Protocol 2"

pub trait Protocol: sealed::Sealed + Exchange {
    const BROADCAST: u8 = 0xFE;
    const VERSION: ProtocolVersion;
}

pub trait Exchange: sealed::Sealed {
    // existing: ping, read, write, sync_write, report
    /// Finds every actuator at this baud. P2: one broadcast ping. P1: ping 0..=252 in turn, each
    /// bounded by `link.timeout` (the bus passes its scan wait). A reply with an error byte is a
    /// `Pong` with `fault` set. A corrupted reply is passed on as `Err`, not skipped.
    /// Returns `Err` only when the transport fails.
    fn scan<T: Transport, C: Clock, F>(link: &mut Link<'_, T, C>, on_reply: F) -> Result<(), Error>
    where F: FnMut(Result<Pong, Error>);

    /// Reads `size` bytes at `addr` from each Id. P2: one Sync Read (today's `P2::sync_read`).
    /// P1: one Read per Id. `on_reply` is called once per Id, in order.
    fn read_many<T: Transport, C: Clock, I, F>(
        link: &mut Link<'_, T, C>, addr: u16, size: u16, ids: I, scratch: &mut [u8], on_reply: F,
    ) -> Result<(), Error>
    where I: Iterator<Item = Id> + Clone, F: FnMut(Id, Result<&[u8], Error>);
}
```

`P2::sync_read` stays as the explicit-tier entry (`impl Bus<P2, …>` only).

---

## 6. bus: shape

```rust
// src/bus.rs
#[cfg(feature = "std")]      pub type DefaultClock = StdClock;
#[cfg(not(feature = "std"))] pub type DefaultClock = NoClock;
/// Always present. Implements no `Clock`, so a no_std bus must name its clock.
pub enum NoClock {}

pub struct Bus<P: Protocol, T: Transport, const N: usize = 32, C: Clock = DefaultClock> {
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

impl<P: Protocol, T: Transport, const N: usize, C: Clock> Bus<P, T, N, C> {
    pub fn new(transport: T, clock: C, config: Config) -> Self;            // no I/O
    pub fn into_parts(self) -> (T, C);
    pub fn autoscan(&mut self) -> Result<&[ActuatorKind], Error>;          // no_std too
    pub fn declare<I: IntoIterator<Item = (Id, ModelNumber)>>(&mut self, list: I)
        -> Result<&[ActuatorKind], Error>;
    pub fn actuators(&self) -> &[ActuatorKind];
    pub fn actuator(&mut self, id: Id) -> Option<ActuatorRef<'_, P, T, C>>;
    // Easy and Explicit tiers: names and buffers per the bus plan (FR-4)
}
impl<T: Transport, const N: usize, C: Clock> Bus<P2, T, N, C> { /* sync_read */ }

#[cfg(feature = "std")]
pub type SerialBus<P, const N: usize = 32> = Bus<P, SerialTransport, N, StdClock>;
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
    pub fn present_position(&mut self) -> Result<Position, Error>;
    pub fn present_position_raw(&mut self) -> Result<i32, Error>;
    pub fn clear_fault(&mut self) -> Result<(), Error>;
}
impl<P: Protocol, T: Transport, C: Clock> Actuator for ActuatorRef<'_, P, T, C> {} // via Describe delegation
```

How the bus uses lower layers:

- Only `src/bus/wire.rs` builds a `protocol::Link { transport, clock, timeout }` and calls
  `Exchange`. It holds the retry loop: unicast only, on timeout or protocol corruption only, never
  on `Status`, never on broadcast.
- Setup per actuator:
  1. Ping (P2) or scan reply.
  2. On P1, read `models::MODEL_NUMBER`.
  3. If `ActuatorKind::firmware_register(P::VERSION, n)` is `Some`, read it (on P2 the `Pong`
     already carries the firmware).
  4. Call `ActuatorKind::new(P::VERSION, n, fw, id)`.
  5. Read the limits via `data().limits.min()/max()` and call `cache_mut().record_limits(..)`.
  6. Set the slot to `TorqueState::Off`.

  Any error leaves the list empty. A fault reported at Setup fails Setup.
- Goal path: `state.check_goal(id)?`, then `scale.to_raw`, then `cache.check_goal(raw)?`, then
  `Register::encode`, then send. Every refusal sends zero bytes.
- The `alloc` variant (`DynBus`, FR-1.3) is bus-internal design. It must not change `Bus`.

---

## 7. Delivery order

```
protocol PR #7 (+ section 5)
   │
   ├── core-1 ─────────┐        tables T1  (no dependency; parallel with core-1)
   │                   ▼            │
   │               xtask X1 ◄───────┘
   │                   │
   │                core-2 ──────────► bus-1 … bus-6
   │                   │
   │                   └─(after X1)──► tables T2   (parallel with core-2 and bus)
   │
   └── ci  (after #7 and core-1; T1 optional, its table check is guarded until T1)
```

| Step | Module | Content | Needs |
|---|---|---|---|
| P | protocol | Section 5 in PR #7 | — |
| core-1 | core | Every type in section 2, standalone `Describe`, `SetupError` + `Error::Setup`. Shims: old `Error::UnsupportedModel` (`#[doc(hidden)]`), 2-argument `Register::new(addr, size)` (`#[doc(hidden)]`, unconditional `allow(dead_code)`; no 4-argument `new`), `Actuator` with three required methods. Fixes the no-family warning. Builds against today's generated code with no regeneration. | P |
| T1 | tables | `source: Unverified` on all 72 tables, `protocols`, `access` + `area` on every row, `p1`/`p2`/`default` beside legacy `tables`, `not_mapped`, table check script. AX-12A and MX-64(2.0) verification may land here. No regeneration (today's xtask ignores the new fields). | — |
| X1 | xtask | Reads section 3. Emits section 4: `Describe`, `DATA`, `cache` field, registers built by a private generated `reg()` helper (never `Register::new`), per-protocol lookup returning `SetupError`, `firmware_register`, `MODEL_NUMBER`, checks, verification docs. Still emits `id`/`model_number`/`name` in `impl Actuator`. | core-1, T1 |
| core-2 | core | `Actuator: Describe` with provided methods; removes the shims (no `Register::new` remains). Generated overrides stay valid, and a later regeneration may drop them. | X1 |
| T2 | tables | MX-28/64/106 P1 tables, AX-12+ (`also`), AX-18F, XW430-T200, verification of AX-12+ and MX-64 P1, removal of legacy `tables`, regenerated `src/models*` committed. | X1 |
| bus-1 … bus-6 | bus | As in specs/003-bus/plan.md. bus-1 adds `heapless`; bus-5 adds `partial-reads`/`raw-registers` to `Cargo.toml`. | core-2, X1, P |
| ci | ci | Gates from spec 007. The feature-set job needs core-1 (today `--no-default-features` fails under `-D warnings`). The missing-docs gate needs #7 (it documents the `UnsupportedModel` fields). The `control tables: schema check` job runs T1's script; it is guarded until T1 merges, and the first ci change after T1 removes the guard. | P, core-1, T1 (optional) |

Core, tables and bus each ship as more than one PR. That needs the architect's approval under
"one module per PR" (section 9).

---

## 8. Conflicts found, with resolutions

| # | Conflict | Resolution in this contract |
|---|---|---|
| 1 | xtask FR-009, US1 AS5: the cache holds torque state and fault state. Core FR-008/009, bus FR-024, SRS patch: one byte per slot in a bus array. | Cache = `Id` + limits. `TorqueState` lives in `Bus::states`. xtask's plan drops torque/fault from the cache. |
| 2 | xtask Assumptions: "register access not in tables, not generated". Core Assumptions: `Register` is address + size only (gap). Tables FR-007 + architect: access and area on every row now. | `Register` carries `access` and `area`. xtask requires and emits them, and checks role attributes. |
| 3 | xtask Assumptions/Edge: MX on P1 is roadmap. Tables Clarification + SRS step 7: MX P1 tables are written now. | In scope. X1 generates a per-protocol lookup. T2 adds the tables. |
| 4 | xtask FR-006: lookup by model number + firmware. Tables FR-008: per-protocol map. | `ActuatorKind::new(ProtocolVersion, ModelNumber, fw, Id)`. Needs `Protocol::VERSION` (section 5). |
| 5 | xtask FR-012: protocol set per model type. Tables FR-005/008: protocol per table and per map entry. | Both, kept consistent: the table's `protocols` must include every map list naming it (codegen check). `ModelData::protocols` comes from the table. |
| 6 | Bus FR-012: a model unreachable on the bus protocol fails Setup with its own error. Tables US4 AS4: the bus reports `UnsupportedModel`. Core FR-016 has no such variant. | One variant: `UnsupportedModel` gains `protocol` (and `id`), and covers unknown numbers, families that are off, and missing tables on this protocol. |
| 7 | SRS FR-6.4 and core FR-016: "`Id` not on the bus" is a Setup error. Bus US5 AS4 and FR-034: an unknown target Id is a validation error. | SRS wins (higher authority): `SetupError::NotOnBus`. Repeated Id: `ValidationError::DuplicateId`. Both still send nothing. `bus.actuator(id)` returns `None`. |
| 8 | Bus FR-026: enabling torque while faulted is refused with a typed error. Core FR-015 lists no such variant. | Add `ValidationError::ActuatorFaulted { id }`. |
| 9 | Core spec: `ModelNumber` is a bus item. xtask spec: core defines it. | Core (`src/actuator.rs`). |
| 10 | Core spec: core and xtask sequencing left open; the trait change breaks generated code. xtask FR-018 and the no-go rules forbid cross edits. | core-1 (additive + shims) → X1 → core-2 (cutover). No hand edits to generated code. |
| 11 | Tables FR-016 (keep today's codegen working) vs the new map format. | T1 keeps the legacy `tables:` beside `p1`/`p2`. X1 ignores it. T2 removes it. |
| 12 | SRS §9 gives `Cargo.toml` to ci, and ci FR-015 forbids feature and dependency edits. Bus needs `heapless`, `partial-reads`, `raw-registers`. PR #7 already adds `proptest`. | Split ownership (section 1). The module that needs a line adds it and flags it. |
| 13 | Bus proposal: compile-fail via trybuild `.stderr`. ci FR-008: rustdoc `compile_fail,EXXXX` checked on a pinned toolchain. | Compile-fail cases are rustdoc examples with error codes, so the ci check covers them. No `trybuild` dependency. |
| 14 | ci Assumptions: the no-feature build is green on day one. xtask Assumptions: a no-family build warns. Checked on `dev` with `-D warnings`: `cargo build --lib --no-default-features` fails (`Register::new` never used). | core-1 fixes it. The ci feature-set job lands after core-1. |
| 15 | Bus proposal: `C: Clock = fn() -> u64`. Architect: zero bytes, no indirect call, system clock under std. A cfg-switched default must not break additivity. | `DefaultClock` = `StdClock` (std) or `NoClock` (no_std, not a `Clock`). No_std users must name a clock, and enabling std only makes more code compile. |
| 16 | PR #7 `broadcast_ping` skips corrupted replies. Bus spec edge case: a corrupted reply fails Setup with a protocol error. | `scan` passes corrupted replies on as `Err`. The bus fails Setup (nothing hidden). A protocol change, section 5. |
| 17 | Core FR-012: no registers beyond OQ-2 in the trait. xtask FR-008/011: fault register, Firmware Version, Model Number exposed. | Those are crate-internal (`ModelData`, `firmware_register`, `MODEL_NUMBER`), not on the public `Actuator` trait. |
| 18 | Bus Assumptions: "position scale … not in the tables yet". All 72 tables have `type_info`. | The scale comes from `type_info` (zero + slope). The AX `min_radian` oddity is checked at AX-12 verification. Model Number (0, 2) and Firmware Version (2 or 6) are table rows. |
| 19 | Core FR-016 names `UnsupportedModel { model_number, feature }`. | Fields `id` and `protocol` are added (DX: name the actuator and the protocol). Flagged to the architect. |
| 20 | ARCHITECTURE.md: "Torque state = runtime field on the actuator", "Actuator cache: Id, torque state, limits", `Bus<P, T, N>`, autoscan std-only (SRS 2.1 text not yet patched in modules). | Superseded by the 2026-10-27 decisions. ARCHITECTURE.md and `docs/srs/modules/*.md` need the architect's update. Agents do not edit them. |

---

## 9. For the architect

- Approve the multi-PR split: core-1/core-2, T1/T2, bus-1 … bus-6. Approve that protocol changes
  (section 5) go into PR #7.
- Confirm conflict resolutions 6, 7, 15, 16 and 19. They change public error shapes or the
  protocol.
- Update ARCHITECTURE.md to the 2026-10-27 decisions (`docs/srs/modules/*.md` were regenerated
  from the patched SRS).
- Constitution IV already has the deferred-verification amendment (v1.2.0). Principle VI still
  lists four error categories and needs `Setup` added (core plan).
- Limits that may hold only in position mode (bus R3, conflict 8 in the bus spec) and AX
  wheel mode (CW = CCW = 0) stay open for the bus plan. They are checked against the e-Manual
  before the limit check ships.
