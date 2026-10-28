# Research: Core (typed values, Actuator trait, errors)

**Feature**: [spec.md](spec.md) | **Date**: 2026-10-27, updated 2026-10-28 (R-17, R-18, R-19)

Sources: `src/` on `dev`; the protocol module of PR #7 (merged into `main` as f338e2a:
`src/error.rs`, `src/lib.rs`, `src/protocol.rs`, `src/protocol/p2.rs`, `src/protocol/no_alloc.rs`);
`specs/ideas/module-interfaces.md` (with commit 96b317a, 2026-10-28); the SRS patch
`/tmp/dxl-srs-bus.patch`; all 72 `control_tables/*.ron` (scanned by script); the e-Manual pages
for XM430-W350 and AX-12A; the e-Manual pages cited in R-17 (read 2026-10-28); and a scratch
prototype crate under `/tmp/claude-1000/core-proto` (host and `thumbv7em-none-eabihf`, rustc
1.97.1).

## R-1 PR split and order

**Decision**: two PRs, as module-interfaces §7 sets out.

- **core-1** (after protocol PR #7 with `ProtocolVersion`, merged): every type of §2 including the
  limit mode of §2.4b (R-17), `Describe` standalone, `SetupError` and `Error::Setup`, and the shims
  of R-2 and R-14. `Actuator` keeps its three required methods. Builds against today's generated
  code. It also deletes `P2::broadcast_ping` and PR #7's `expect(dead_code)` on `mod protocol;`
  (R-19, a protocol touch).
- **core-2** (after xtask X1): `Actuator: sealed::Describe` with provided methods, shims removed,
  generic tests over every model.

**Rationale**: the trait cutover needs every generated type to implement `Describe`. Only X1 can
emit that, and X1 needs core-1's types first. Each step keeps `cargo build`, `cargo test` and
`cargo xtask codegen` (no diff) green.

**Alternatives considered**:
- One core PR after X1. Rejected: X1 cannot compile without `Cache`, `ModelData`, `Describe`.
- One core PR that also regenerates models. Rejected: today's xtask cannot emit `Describe`, and
  core may not edit `xtask/`.

## R-2 `Register` constructor across core-1 → X1 → core-2 (deviation D1)

**Decision**:
- core-1: `Register` gains `access: Access` and `area: Area` (`pub(crate)` fields, §2.2). The old
  `pub(crate) const fn new(addr, size)` stays, `#[doc(hidden)]`, and fills `Access::R`,
  `Area::Eeprom` as placeholders.
- X1: builds each `Register` without `Register::new`, through its own generated helper
  `const fn reg(addr, size, access, area) -> Register` (xtask plan D1).
- core-2: the shim is removed. Core adds **no** 4-argument `new`: registers are built only by
  X1's `reg()`, the bus never builds one, and raw-registers takes an address. A 4-argument `new`
  would be dead in every build.

**Rationale**: Rust has no overloading. A 2-argument and a 4-argument `new` cannot coexist, and
X1 cannot edit `src/register.rs`. `reg()` builds a struct literal, which compiles in both the
core-1 and core-2 states. The placeholders are never read: no code reads `access` or `area`
before X1 regenerates every register, and the bus lands after core-2.

module-interfaces §2.2, §4.1 and §7 now drop `Register::new` (§4.1 used to show X1 emitting
it).

**Alternatives considered**:
- 4-argument `new` in core-1. Rejected: breaks today's generated code (no regeneration allowed).
- A second constructor name (`Register::with`) for X1. Rejected: leaves a permanent second name,
  or forces a rename that needs an xtask edit in a core PR.
- No new fields until core-2. Rejected: X1 must emit them and check role attributes (§4.3).

## R-3 Tests that pin the generated lookup (deviation D2)

**Decision**: core-1 rewrites the tests in `src/actuator.rs`:
- Models are built as `ActuatorKind::Xm430W350(Xm430W350::new(id))`. `new(id: Id)` is kept by
  §4.1.
- `builds_model_from_number`, `picks_table_by_firmware` and `unknown_model_is_unsupported` are
  dropped from core. X1 owns lookup tests.
- `unsupported_model_message_names_the_feature` moves to `src/error.rs` and tests
  `SetupError::UnsupportedModel`.

**Rationale**: X1 changes `ActuatorKind::new` to `(ProtocolVersion, ModelNumber, u8, Id) ->
Result<Self, SetupError>`. A core test calling the old signature would break X1's PR, and X1 may
not edit core files.

**Alternatives considered**: keep the tests and let X1 break them. Rejected: X1 could not merge.

## R-4 Dead code across the transition

**Decision**: items that only later PRs use carry `#[allow(dead_code, reason = "used by <PR>")]`
in core-1. The full core-1 list, with the reason each one is dead until then:

| Item | Dead in core-1 because | First user |
|---|---|---|
| `Access` (variant `RW`), `Area` (variant `Ram`) | the 2-arg shim fills `R` and `Eeprom`, so nothing builds `RW` or `Ram` | X1 (`reg()` calls) |
| `sealed::Describe` | no impl and no caller | X1 (generated impls) |
| `ModelData`, `LimitRegisters`, `Protocols`, `ModeSource`, `ModeValues` | only generated `DATA` consts build them | X1 |
| `Cache::new`, `Cache::id`, `Cache::limits`, `PositionScale::new` | called only by generated code and core-2's provided methods | X1, core-2 |
| `Cache::record_limits`, `Cache::record_mode`, `Cache::check_goal`, the `mode` field, `LimitMode` (variants `Unbounded`, `NoPosition`), `LimitMode::from_angle_limits`, `ModeSource::register`, `ModeSource::limit_mode`, `ModeValues::limit_mode` | called only at Setup and on the goal path | bus-1 |
| `Register::encode/decode`, `TorqueState::enable/disable/fault/clear/check_goal`, `Limits::min_raw/max_raw`, `LimitRegisters::min/max`, `Protocols::supports` | I/O paths | bus-1 |

- The 2-arg shim gets an **unconditional**
  `#[allow(dead_code, reason = "unused once X1 regenerates with reg(); removed in core-2")]`, with
  `#[doc(hidden)]`. A no-family `cfg_attr` is not enough: once X1 regenerates, nothing calls the
  shim in any build, and X1 may not edit `src/register.rs`.
- The `cfg_attr(not(test), expect(dead_code))` on `Register` today is replaced by an
  unconditional `#[allow(dead_code, reason = "fields read by bus-1")]` on the struct. No live code
  reads the fields until bus-1 (`encode`/`decode` are themselves unused, and tests never read
  `access`/`area`), and an `expect` would fail bus-1 once they are read.
- core-2 narrows the allow to
  `#[cfg_attr(not(any(feature = "ax", feature = "mx", feature = "x-series", feature = "y-series",
  feature = "pro")), allow(dead_code, reason = "only generated models construct it"))]` **only** on
  items that every family's generated code uses (e.g. `ModelData`, `Protocols`, `Cache::new`,
  `PositionScale::new`, `Describe`, `Access`, `Area`).
- Enums whose variants are each built by **some** families only keep an unconditional
  `#[allow(dead_code, reason = "each variant is built only by the generated models of its families")]`
  through core-2: `LimitRegisters` (AX alone builds no `MinMax`; MX, PRO and Y alone build no
  `CwCcw` until T2) and `ModeSource` (AX alone builds no `Register`; X, PRO and Y never build
  `AngleLimits`, MX only from T2). `ModeValues` keeps one too: it is a struct, but only tables
  with an `operating_modes` field build it, and no AX table has one, so an `ax`-only build never
  does. The ci feature-set job builds each family alone, and a no-family `cfg_attr` would leave
  those legs red.
- Items whose first user is the bus keep an unconditional
  `#[allow(dead_code, reason = "used by bus-1")]` through core-2: `Register::encode/decode`,
  `Cache::record_limits/record_mode/check_goal`, the `Cache::mode` field, `LimitMode`,
  `LimitMode::from_angle_limits`, `ModeSource::register/limit_mode`, `ModeValues::limit_mode`,
  `TorqueState::enable/disable/fault/clear/check_goal`, `Limits::min_raw/max_raw`,
  `LimitRegisters::min/max`, `Protocols::supports`. bus-1 lands after core-2, so narrowing them
  would make core-2 red.

**Rationale**:
- Reproduced on `dev`: `RUSTFLAGS="-D warnings" cargo build --lib --no-default-features` fails
  with "associated function `new` is never used" (`src/register.rs:19`).
- After X1, `cargo clippy --all-targets -- -D warnings` checks the non-test lib with every family
  on. A shim allowed only without families would fail there with the same message, in X1's PR.
- The prototype shows `pub` items inside a private module still get `dead_code` warnings.
- `expect` is wrong here: when X1 or bus starts using an item, the `expect` becomes unfulfilled,
  warns, and fails their build, and they cannot edit core files to remove it.

**Alternatives considered**: `expect(dead_code)` as `register.rs` uses today. Rejected for the
reason above. Exporting the items publicly to silence the lint. Rejected: §2.8 forbids it.

## R-5 Rounding without `std`

**Decision**: round half away from zero by hand: `if x >= 0.0 { x + 0.5 } else { x - 0.5 }`, then
`as i64` (truncates, saturates, maps NaN to 0, never panics). Finite-ness is checked first with
`f32::is_finite` (in `core`).

**Rationale**: `f32::round` is not in `core` on stable: the prototype fails on
`thumbv7em-none-eabihf` with E0599 ("no method named `round` found for type `f32`"). `abs`,
`copysign` and `is_finite` are in `core`. Ties at exactly half a tick are not reachable from a
tick (R-6), so the tie rule only matters for user angles.

**Alternatives considered**: the `libm` crate. Rejected: a new dependency for one function, and
`Cargo.toml` is a shared file. `f64` math. Rejected: the Cortex-M4F FPU is single precision, so
`f64` is software-emulated on the binding target.

## R-6 Precision of `f32` radians

**Decision**: `Position` is `f32` radians (OQ-3 default). `PositionScale` precomputes
`radians_per_tick` and `ticks_per_radian` in its `const fn new`. `to_position(raw) =
(raw as i64 - zero_raw as i64) as f32 * radians_per_tick`. `to_raw(p) = round(p * ticks_per_radian)
as i64 + zero_raw`.

**Rationale**: the prototype ran tick → radians → tick for every tick of every distinct scale in
the 72 tables, plus multi-turn ticks up to ±2²⁰ (step 7). Zero mismatches, with both multiply and
divide forms:

| Scale (zero, raw range, rad range) | Tables | Mismatches |
|---|---|---|
| 2048, 0..=4095, ±π | X-series, MX(2.0), 2XL/2XC | 0 |
| 512, 0..=1023, 0..=5.236 | AX | 0 |
| 0, ±303 750, ±π | H42 / PH42 | 0 |
| 0, ±501 923, ±π | H54 / PH54 | 0 |
| 0, ±263 187, ±π | M42 / PM42 | 0 |
| 0, ±251 417, ±π | M54 / PM54 | 0 |
| 0, ±740, ±1.099 | RH-P12-RN | 0 |
| 0, ±262 144, ±π | Y-series | 0 |

These are today's tables. T1 corrects the first two rows (−π..3.1400587 and −2.6205530..2.6154347,
see R-7); the round-trip test runs over the scales as the tables give them.

Beyond an offset of 2²⁴ ticks, `f32` cannot hold every tick. That is more than 4000 turns on an
X-series and about 33 turns on an H54. The raw path covers it, and the docs say so.

**Alternatives considered**: `f64`. Rejected (R-5). Fixed-point radians. Rejected: SRS OQ-3 default
is `f32`, and users expect float angles.

## R-7 Scale from `type_info`

**Decision**: `PositionScale::new(zero_raw, min_raw, max_raw, min_rad, max_rad)` takes the five
`type_info` values as they are. Only `zero_raw` and the slope `(max_rad - min_rad) / (max_raw -
min_raw)` are used for conversion. Division uses `i64` then `f32`, so `new` cannot overflow.

**Rationale**:
- AX tables say `min_radian: 0.0` with zero at tick 512. Using zero + slope makes 512 → 0 rad
  regardless, as §2.3 says. The e-Manual confirms the AX-12A center is 512, range 0–1023 over
  0–300° ([AX-12A](https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/)).
- The e-Manual gives XM430-W350 as 4096 pulses = 360°, center 2048
  ([XM430-W350](https://emanual.robotis.com/docs/en/dxl/x/xm430-w350/)). The table gives ±π over
  0..=4095, a slope of 2π/4095 instead of 2π/4096 (0.024 % off; half a tick at ±π). Core uses the
  table as it is. **The tables module fixes it in T1**: the X-series, MX(2.0) and 2XL/2XC tables
  get `min_radian: -3.1415927`, `max_radian: 3.1400587`, a slope of 2π/4096 (tables research
  R-8). Round trip is not affected. Core's ±π test vectors stay: they test the type, not the
  table.

**Alternatives considered**: computing the zero from `min_rad` (gives 0 for AX, wrong).
Hard-coding scales per family in core. Rejected: constitution V, scale is table data.

## R-8 No panic in conversions

**Decision**: all intermediate arithmetic is in `i64`. `to_raw` checks `is_finite` first
(`NotFinite`), then converts the rounded `i64` with `i32::try_from`; failure is
`DoesNotFit { raw, size: 0 }`. `to_position` never fails.

**Rationale**: `raw - zero_raw` in `i32` overflows (a debug-build panic) for `raw = i32::MIN`.
A 4-byte Present Position is signed with range ±2 147 483 647 on the XM430 e-Manual page, so any
`i32` can arrive. The prototype gives `to_position(i32::MIN)` = −3 295 006.5 rad with no panic, and
`to_raw(1e30)` an error.

**Alternatives considered**: `wrapping_sub`. Rejected: silently wrong angles.

## R-9 `Register` encode and decode

**Decision**:
- `encode(raw, &mut [u8; 4]) -> Result<&[u8], ValidationError>`: size 1 accepts `0..=255`, size 2
  `0..=65 535`, size 4 any `i32`. Other values give `DoesNotFit { raw, size }`. A size other than
  1, 2 or 4 also gives `DoesNotFit`. Bytes are `raw.to_le_bytes()[..size]`.
- `decode(&[u8]) -> Result<i32, ValidationError>`: the slice must be exactly `size` bytes
  (`DataLength { expected, got }`). Sizes 1 and 2 are unsigned, size 4 is two's-complement.

**Rationale**: matches §2.2. e-Manual facts for the four role registers: XM430 Goal Position 116,
4 bytes, RW, RAM; Present Position 132, 4 bytes, R, RAM, signed range. AX-12A Goal Position 30,
2 bytes, 0–1023; Present Position 36, 2 bytes, R; CW/CCW Angle Limit 6/8, 2 bytes, RW, EEPROM
(pages cited in R-7). A negative AX tick is refused, not wrapped (spec edge case). The prototype
confirms the elided lifetime: the returned slice borrows `out`.

**Alternatives considered**: a generic `encode<const S: usize>`. Rejected: register size is data,
known only at run time through `ModelData`. Signed 2-byte decode. Rejected: no role register needs
it; other registers need a contract change (§2.2).

## R-10 `TorqueState` and memory

**Decision**: `#[repr(u8)] enum TorqueState { Off = 0, On = 1, Faulted = 2 }`, `Default = Off`,
not `#[non_exhaustive]`. Transitions are `const fn`s per the table in data-model.md.

**Rationale**: measured in the prototype: `size_of::<TorqueState>() == 1`,
`size_of::<Option<TorqueState>>() == 1`, `Cache` (`Id` + `Option<Limits>`) = 16 bytes, an
`ActuatorKind`-shaped enum = 20 bytes. A torque `bool` plus a fault `bool` inside the cache
would not grow it today (padding), but it would put state in the generated structs, which the
architect rejected. One byte per slot in the bus array is the decided layout.

**Alternatives considered**: two flags (SRS 3.3, superseded). Typestate (rejected in the SRS).

## R-11 Limits and the limit check

**Decision**: `Limits { min, max }` built only by `Limits::new(id, min, max)`, which refuses
`min > max` with `InvertedLimits`. `Cache::check_goal(raw)` is the only check. In position mode
(`LimitMode::Bounded`, the default): unknown limits give `LimitsUnknown { id }`, outside
`min..=max` gives `OutOfLimits { id, raw, min, max }`. Bounds are inclusive. The other two modes
are in R-17 (decided 2026-10-28).

**Rationale**: FR-005 to FR-007. The typed path calls it after `to_raw`, so both paths share one
check (US4 AS2). The architect decided on 2026-10-28 that limits follow the operating mode, and
that the check lives in core (`Cache`), not in the bus (module-interfaces §2.4b).

**Alternatives considered**: storing limits as `Position`. Rejected: the check runs on the rounded
tick (spec edge case), and the raw path has no scale.

## R-12 Sealed `Describe` and its visibility

**Decision**: `pub(crate) mod sealed { pub trait Describe { … } }` inside `src/actuator.rs`, with
`pub(crate) use sealed::Describe;`. `ModelData`, `Cache`, `LimitRegisters`, `Protocols` are `pub`
items in the private `actuator` module, never re-exported.

**Rationale**: generated code in `crate::models` must name `Describe` to implement it, so the
module cannot be private to `actuator`. Outside the crate it stays unnameable, which seals
`Actuator`. PR #7 uses the same pattern for `Exchange`, `Link`, `Pong`.

**Risk (for the architect)**: the prototype confirms that user code with a bound `A: Actuator`
can call supertrait methods without naming the trait. So `fn f<A: Actuator>(a: &mut A, b: &A) {
*a.cache_mut() = *b.cache(); }` compiles and copies another actuator's `Id` and limits. Users
cannot build a `Cache`, only copy or swap existing ones, and only on purpose. Fix if wanted: add a
crate-private token argument, `fn cache_mut(&mut self, _: Token) -> &mut Cache`, where `Token`
has a private field. That changes X1's emitted impl, so it is not done here without the
architect's call.

**Alternatives considered**: `cache_mut` as an inherent `pub(crate)` method on `ActuatorKind`.
Rejected for now: the bus's `ActuatorRef` delegates through `Describe` (§6), and it changes X1.

## R-13 `Actuator` provided methods (core-2)

**Decision**: every `Actuator` method is provided: `id`, `model_number`, `name`,
`position_scale`, `position_limits_raw`, `position_limits`. Bodies use `Describe::data()` and
`cache()`.

**Rationale**: X1 emits `impl Actuator for X { fn id… fn model_number… fn name… }`. Those stay
valid overrides. A required `position_limits_raw` would break X1's output at core-2.

## R-14 Errors

**Decision**:
- `ValidationError` gains `ActuatorIsOff`, `ActuatorFaulted`, `OutOfLimits`, `LimitsUnknown`,
  `InvertedLimits`, `NotFinite`, `DoesNotFit`, `BufferLength`, `DuplicateId` (§2.7) and
  `NotPositionMode { id }` (§2.4b, 2026-10-28).
- New `SetupError`: `UnsupportedModel { id, model_number, protocol, feature }`,
  `TooManyActuators { capacity }`, `Missing { id }`, `WrongModel { id, declared, found }`,
  `NotOnBus { id }` and `UnknownOperatingMode { id, value: u8 }` (§2.4b, 2026-10-28).
  `Error::Setup(#[from] SetupError)`. The `id` and `protocol` fields of `UnsupportedModel`
  (conflict 19) were accepted by the architect on 2026-10-28.
- core-1 keeps `Error::UnsupportedModel` as `#[doc(hidden)]` for today's generated code; core-2
  removes it. Breaking change, called out in core-2's PR.
- Messages name the actuator with `id.get()` (no `Display for Id`; `id.rs` unchanged, §2.1).
  Texts are fixed in [contracts/core-api.md](contracts/core-api.md#error-messages).

**Rationale**: spec FR-014 to FR-017, FR-021 to FR-023 and module-interfaces conflicts 6, 7, 8, 19.
All variants are `Copy` and allocation-free (`&'static str` for `feature`).

**Alternatives considered**: `NotOnBus` as a validation error (bus spec FR-034). Rejected: SRS
FR-6.4 wins (conflict 7).

## R-15 Dependency on `ProtocolVersion`

**Decision**: core-1 imports `crate::protocol::ProtocolVersion` (module-interfaces §5). Core does
not define a stand-in. PR #7 is merged into `main` (f338e2a): `ProtocolVersion` is on `main`, and
`src/lib.rs` already re-exports it (`pub use protocol::{Clock, P1, P2, Protocol,
ProtocolVersion};`). core-1 only verifies both (T001) and adds no re-export of its own.

**Rationale**: `SetupError::UnsupportedModel` and `Protocols::supports` need it, and the protocol
module owns it. A core-side copy would be redefinition of another module's item.

**Alternatives considered**: `bool`/`u8` for the protocol in the error. Rejected: untyped, and the
message would need its own formatting.

## R-16 Testing strategy

**Decision**:
- core-1: unit tests per file. Exhaustive round trip over the eight scales of R-6, built with
  `PositionScale::new` from the table values. `proptest` (from PR #7) for `to_raw` on any `f32`
  and `encode`/`decode` on any `i32`: no panic, decode(encode(x)) == x when it fits.
- core-2: one generic `fn check<A: Actuator + Describe>(a: &A)` runs over every variant the
  generated lookup can return. Variants are found by calling `ActuatorKind::new` for every model
  number on both protocols, with every firmware value when `firmware_register` is `Some`, and
  dedup by `core::mem::discriminant`. Round-trip loops dedup by scale.
- US2 AS7 ("the enum gives the same answer as the model inside it") is covered in two parts
  (T048). (1) The generic check runs on every variant through `ActuatorKind`: each `Actuator`
  method of the enum equals the value read from `data()` and `cache()`, which the enum's `match`
  takes from the inner model. (2) For five representatives, one per family (`Xm430W350`,
  `Ax12a`, `Mx64`, `H54200S500Ra`, `Ym070210M001`, each under its family `cfg`), the test unwraps
  the inner model with a `let ActuatorKind::X(inner) = … else` and compares every method of the
  enum with the inner model's directly. Reaching the inner model needs a pattern that names its
  variant. A hand-written `match` over all 72 variants would copy the generated one and break
  with every new table, so the per-variant direct comparison is left to the representatives.
- The mode source is checked per model in the same test (T048): every `ModeSource::Register`
  register is 1 byte, RW, EEPROM, and its three lists are disjoint; `AngleLimits` comes only
  with `LimitRegisters::CwCcw`; seven representatives carry the address and the value lists of
  R-17, as the tables copy them.
- core-1 tests the mapping with hand-built `ModeValues` fixtures (the X, RH-P12-RN and XL-320
  lists of R-17), since core holds no list of its own (R-17).
- Compile-fail cases (e.g. `Describe` not nameable outside the crate) are rustdoc
  `compile_fail,E0603` examples (module-interfaces conflict 13), not `trybuild`.

**Rationale**: SC-001 and SC-003 ask for 100 % of compiled-in models without a hand-written list.
Lookup enumeration needs no list and follows the enabled features.

## R-17 Limit mode (architect, 2026-10-28: limits follow the operating mode)

**Decision**:
- `LimitMode { Bounded = 0, Unbounded = 1, NoPosition = 2 }`, `#[repr(u8)]`, `Default` =
  `Bounded`, crate-internal, in `src/actuator.rs` (module-interfaces §2.4b). `Cache` gains
  `mode: LimitMode` beside `id`. It sits in the padding after the 1-byte `id`, so `Cache` stays
  16 bytes; a test pins `size_of::<Cache>() == 16` and `size_of::<LimitMode>() == 1`.
- `Cache::record_mode(mode)` stores it; limits are untouched. `Cache::check_goal(raw)` checks the
  mode first. `Bounded`: R-11, unchanged. `Unbounded`: `Ok(raw)`; the register size check of
  `Register::encode` still runs on the goal path (module-interfaces §6), so a value that does
  not fit is still `DoesNotFit`. `NoPosition`: `NotPositionMode { id }`, whatever the limits.
- `ModelData::mode: ModeSource` says where Setup gets the mode: `Register { register, values:
  ModeValues }` (the Operating Mode register, or the XL-320 Control Mode register) or
  `AngleLimits` (AX, MX on P1). `ModeSource::register()` is the 1-byte register Setup reads
  (`None` for `AngleLimits`).
- **Decided by the architect on 2026-10-28: the value lists live in the tables.** `ModeValues` is
  a struct of three lists, `bounded`, `unbounded`, `no_position: &'static [u8]`. X1 generates one
  per model from its table's `operating_modes` field, which T1 copies from that model's e-Manual
  page (module-interfaces §2.4b, §3.1). Core holds no per-family list: a new model or family is a
  data change only. `ModeValues::limit_mode(value)` gives the mode of the list that holds
  `value`; `ModeSource::limit_mode(value)` is `values.limit_mode(value)` for `Register` and `None`
  for `AngleLimits`. Both are crate-internal `const fn`s returning `Option<LimitMode>`; the
  lookup walks the slices with slice patterns, no indexing. Codegen and the tables check refuse
  overlapping lists, a mode row without `operating_modes` and `operating_modes` without a mode
  row (§4.3). `LimitMode::from_angle_limits(limits)` gives the AX / MX P1 mode from the limits
  Setup has just read.
- `None` means "the value is in none of the model's lists". The bus turns it into
  `SetupError::UnknownOperatingMode { id, value }` (constitution III: no fallback). Core never
  guesses a mode.
- **Decided by the architect on 2026-10-28:** `position_limits_raw()` / `position_limits()` return
  the EEPROM limits in every mode, as read at Setup. Their rustdoc says the limits are applied to
  goals only in `Bounded` mode (module-interfaces §2.4b). `None` still means only "not read yet".

Values, from the e-Manual pages read on 2026-10-28. This is the reference the tables copy into
`operating_modes` (e.g. XM430-W350: `(bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16])`)
and the source of core's test fixtures (the X, RH-P12-RN and XL-320 rows):

| Tables | Source | Register | Value → `LimitMode` |
|---|---|---|---|
| X-series (feature `x-series`, every table but XL-320: X430, X540, 2XL/2XC, XL330/XC330, XW) and MX(2.0) (feature `mx`) | `Register` | Operating Mode (11) | 0 Current → `NoPosition`; 1 Velocity → `NoPosition`; 3 Position → `Bounded`; 4 Extended Position → `Unbounded`; 5 Current-based Position → `Unbounded`; 16 PWM → `NoPosition` |
| DYNAMIXEL-PRO (A) H42/H54/M42/M54 …-R(A) and DYNAMIXEL-P PH42/PH54/PM42/PM54 (feature `pro`) | `Register` | Operating Mode (11) | 0 Current → `NoPosition`; 1 Velocity → `NoPosition`; 3 Position → `Bounded`; 4 Extended Position → `Unbounded`; 16 PWM (Voltage) → `NoPosition` |
| DYNAMIXEL-Y YM070/YM080 (feature `y-series`) | `Register` | Operating Mode (33) | 0 Current → `NoPosition`; 1 Velocity → `NoPosition`; 3 Position → `Bounded` |
| RH-P12-RN gripper (feature `pro`) | `Register` | Operating Mode (11) | 0 Current → `NoPosition`; 5 Current based Position → `Bounded` |
| XL-320 (feature `x-series`) | `Register` | Control Mode (11) | 1 Wheel → `NoPosition`; 2 Joint → `Bounded` |
| AX-12A, AX-12W, AX-18A (feature `ax`); MX-28/64/106 on P1 (tables T2) | `AngleLimits` (no `operating_modes`) | none: CW/CCW Angle Limit (6, 8), already read | CW = CCW = 0 → `NoPosition` (wheel); CW = CCW = 4095 → `Unbounded` (MX multi-turn); otherwise `Bounded` (joint) |

Every other value is in no list, so `None`. Citations:
- X: [XM430-W350](https://emanual.robotis.com/docs/en/dxl/x/xm430-w350/), Operating Mode(11)
  lists 0, 1, 3, 4, 5, 16; Min/Max Position Limit: "These values are not used in Extended Position
  Control Mode and Current-based Position Control Mode." The same list and sentence are on
  [XL330-M288](https://emanual.robotis.com/docs/en/dxl/x/xl330-m288/) and
  [XW540-T260](https://emanual.robotis.com/docs/en/dxl/x/xw540-t260/).
  [XL430-W250](https://emanual.robotis.com/docs/en/dxl/x/xl430-w250/) and
  [2XL430-W250](https://emanual.robotis.com/docs/en/dxl/x/2xl430-w250/) list 1, 3, 4, 16 only:
  "The angle limits set through this control table item do not apply in Extended Position
  Control Mode."
- MX(2.0): [MX-64(2.0)](https://emanual.robotis.com/docs/en/dxl/mx/mx-64-2/), the same six values
  and the same sentence as XM430-W350.
- PRO: [H54-200-S500-R(A)](https://emanual.robotis.com/docs/en/dxl/pro/h54-200-s500-ra/) (model
  number 54,025), values 0, 1, 3, 4, 16; "Max Position Limit(48) and Min Position Limit(52) are
  NOT used in Extended Position Control Mode."
- PRO+: [PH54-200-S500-R](https://emanual.robotis.com/docs/en/dxl/p/ph54-200-s500-r/) and
  [PM54-060-S250-R](https://emanual.robotis.com/docs/en/dxl/p/pm54-060-s250-r/), values 0, 1, 3,
  4, 16; "The Max Position Limit(48) and Min Position Limit(52) are only used when the Operating
  Mode is the Position Control Mode (Joint Mode)".
- Y: [YM070-210-M001-RH](https://emanual.robotis.com/docs/en/dxl/y/ym070-210-m001-rh/) and
  [YM080-230-M001-RH](https://emanual.robotis.com/docs/en/dxl/y/ym080-230-m001-rh/), Operating
  Mode(33): "DYNAMIXEL-Y actuators support three control modes" (0, 1, 3); Position Control Mode:
  "Movement is restricted to within the range defined by Max and Min [Position Limit(84)]."
- RH-P12-RN: [RH-P12-RN](https://emanual.robotis.com/docs/en/platform/rh_p12_rn/), Operating
  Mode(11): 0 Current Control Mode, "1 ~ 4 Reserved", 5 (default) Current based Position Control
  Mode; Min/Max Position Limit(36, 40): "The Goal Position(596) can't exceed these values."
- XL-320: [XL-320](https://emanual.robotis.com/docs/en/dxl/x/xl320/), Control Mode(11): 1 Wheel
  Mode, 2 Joint Mode; "If it is set to Wheel Mode, Goal Position value is not used."
- AX: [AX-12A](https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/), CW/CCW Angle Limit(6, 8):
  "Wheel Mode | both are 0"; "If it is set to Wheel Mode, Goal Position value is not used."
- MX on P1: [MX-64 (Protocol 1.0)](https://emanual.robotis.com/docs/en/dxl/mx/mx-64/), CW/CCW
  Angle Limit(6, 8): "Multi-turn Mode | Condition that Both CW and CCW are 4095"; joint mode is
  "Conditions excluding Wheel Mode and Multi-turn Mode".

Notes:
- The rows group tables whose pages give the same list. The lists are per table: each table
  copies its own page, so a table may list fewer values than its row (XL430-W250 and
  2XL430-W250 list 1, 3, 4, 16 only). Either way, a value a model's firmware cannot hold never
  reaches the mapping.
- AX joint mode is every pair but (0, 0). The AX page writes "neither are 0", but its default
  pair CW 0 / CCW 1023 is joint mode, so the MX wording ("conditions excluding Wheel Mode and
  Multi-turn Mode") is the rule. AX limits are 0–1023, so (4095, 4095) never comes from an AX;
  one `from_angle_limits` serves both families.
- Every `pro` table in `control_tables/` is the (A) firmware (model numbers 51201, 53769, 54025,
  43289, 46097, 46353). The older non-A page
  ([H54-200-S500-R](https://emanual.robotis.com/docs/en/dxl/pro/h54-200-s500-r/)) lists 0, 1, 3,
  4 and "In Extended Position Control Mode, these limits will be ignored", a subset of the PRO
  row.
- MX on P1, multi-turn: Goal Position (30) is 2 bytes and the e-Manual gives "a range from
  -28,672 to 28,672". §2.2 encodes 2-byte registers unsigned, so a negative multi-turn goal is
  refused with `DoesNotFit` (safe side). The page does not say how the firmware reads values
  above 28 672. The tables module checks it when T2 adds the MX P1 tables; a signed 2-byte
  register needs a contract change (§2.2).

**Rationale**: the firmware applies the EEPROM limits only in position mode. Checking them in
extended or current-based position mode refuses goals the hardware accepts (bus research R-9).
Checking them in velocity, current, PWM or wheel mode passes goals the hardware ignores. Setup
cost: one 1-byte read per actuator on P2 and XL-320, none on AX and MX P1. Memory cost: zero.

**Deviation D3** (plan), decided: §2.4b first wrote `OperatingMode(Register)`. The register
alone cannot pick the value list: value 5 is `Unbounded` on X and MX(2.0) and `Bounded` on
RH-P12-RN, and both sit in default features. So the register carries its values. The architect
decided on 2026-10-28 that the values come from the tables: `ModeSource::Register { register,
values: ModeValues }`, with the lists X1 copies from `operating_modes` (module-interfaces §2.4b,
commit 96b317a). How X1 picks the values is no longer open: there is nothing to pick.

**Alternatives considered**:
- One mapping for every Operating Mode register (the first §2.4b shape). Rejected: value 5
  conflicts.
- A per-family `ModeValues` enum (`X`, `Pro`, `Y`, `RhP12Rn`) with the lists in core (this plan's
  first D3 shape). Replaced by the decision: X1 would have to pick a variant per table, family
  alone does not single out RH-P12-RN, and a new family would need a core change.
- A separate `ControlMode(Register)` variant with the XL-320 list in core. Replaced by the
  decision: XL-320's Control Mode values are a table's lists like any other (`bounded: [2]`,
  `no_position: [1]`).
- A family field on `ModelData` beside `mode`. Rejected: it could disagree with the source (a
  family on an `AngleLimits` model); the value list belongs to the register it decodes.
- A mapping returning `Result<LimitMode, SetupError>`. Rejected: it would need `SetupError`
  (US3) before `ModelData` (US2) in core-1, and the bus already builds Setup errors with the
  `Id`. `Option` keeps the mapping a plain table.
- A `modes: [LimitMode; N]` array in the bus (bus tasks, earlier alternative). Rejected by the
  decision: the mode lives in the `Cache` padding at zero cost.
- `position_limits*()` returning `None` outside `Bounded`. Rejected by the architect
  (2026-10-28): they return the EEPROM limits in every mode, and `None` means only that Setup
  has not read them yet.

## R-18 Other decisions of 2026-10-28

- **`raw-registers` replies run `observe`**: a hardware fault on a raw reply sets `Faulted`.
  Core is unchanged: `TorqueState::fault()` already gives `Faulted` from any state, and core has
  no exception to constitution III. The sentence "This applies on every path except
  `raw-registers`" in module-interfaces §2.5 is stale (outside core).
- **FR-4.7 partial reads**: one result per actuator inside a P2 sync group; only a failed
  instruction fails every slot of the group. No core change: per-slot results are bus types,
  and a caller buffer of the wrong length is still `BufferLength`.
- **Conflict 19 accepted** ("fine by me"): `SetupError::UnsupportedModel { id, model_number,
  protocol, feature }`. SRS FR-2.4 and constitution Principle II still write the old shape; both
  belong to the architect (T059).
- **Protocol PR #7 merged** into `main` (f338e2a) with `ProtocolVersion` and its root re-export
  (R-15). **Constitution 1.3.0** lists `setup` in Principle VI, so the fifth category needs no
  exception any more.

## R-19 core-1 deletes `P2::broadcast_ping` (architect, 2026-10-28)

**Decision**: core-1 deletes `P2::broadcast_ping` and PR #7's
`#[cfg_attr(not(test), expect(dead_code, reason = "exchanges are called by the bus module"))]` on
`mod protocol;` in `src/lib.rs`, as a protocol touch listed in the core-1 PR (module-interfaces
§1, §5). Tasks T003–T005, before any core file changes.
- `src/protocol/p2.rs`: the function with its doc, and its two tests.
  `broadcast_ping_reports_a_set_error_byte_as_a_fault` is covered by
  `scan_reports_a_set_error_byte_as_a_fault`. `broadcast_ping_collects_every_reply_in_the_window`
  is covered by `scan_sends_one_broadcast_ping_and_reports_every_reply` (replies, empty window)
  and `scan_passes_corrupted_replies_on_and_carries_on`, except its transport-failure check: no
  other P2 test runs `scan` on a failing transport, so that check moves to a new
  `scan_fails_on_the_transport`.
- `src/protocol/no_alloc.rs`: the `P2::broadcast_ping` call goes. The test scripts replies per
  flush, so the scan becomes the 6th flush and takes the two replies scripted for it; the 7th
  flush's reply goes, and the reply count drops from 6 to 5.
- `src/protocol.rs`: the `Pong::fault` doc says "scan reply" instead of "scan or broadcast ping
  reply". The `v2::BROADCAST_PING*` vectors stay (the scan tests use them), and so does "broadcast
  Ping" on `P2` and above `scan_reply`: it names the instruction `scan` sends.

**Rationale**: nothing calls `broadcast_ping`. Setup uses `Exchange::scan`, which passes
corrupted replies on (module-interfaces conflict 16); `broadcast_ping` is `scan` with them
dropped. The lint expectation exists only because of it.

Checked on a copy of `main` (f338e2a), 2026-10-28, rustc 1.97.1:
- Without the attribute alone, `cargo clippy --all-targets -- -D warnings` and the no_std clippy
  line each report exactly one error: "associated function `broadcast_ping` is never used"
  (`src/protocol/p2.rs`). No other protocol item is dead.
- With both deleted and the test changes above, both clippy lines, `cargo test --workspace`,
  `cargo fmt --check` and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` pass. The no-family
  build still fails on `Register::new` only, as before (R-4, fixed by T010).
- T003 repeats the first check on the core-1 branch and stops if any other item turns dead.

**Alternatives considered**: a protocol follow-up PR. Not chosen: the architect gave it to
core-1. Keeping both (the bus plan's earlier deviation 6): dead code kept alive by a lint
expectation that every later PR would have to leave alone.
