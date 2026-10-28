---

description: "Task list for the core module (typed values, Actuator trait, errors), shipped as core-1 and core-2"
---

# Tasks: Core (typed values, Actuator trait, errors)

**Input**: Design documents from `specs/004-core/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/core-api.md, quickstart.md,
`specs/ideas/module-interfaces.md` (§2 core items, §7 delivery order), constitution 1.3.0

**Tests**: Required. Constitution: ≥ 70 % line coverage per PR, every acceptance scenario tested;
API that must be absent is proven with rustdoc `compile_fail,EXXXX` examples (module-interfaces
conflict 13, no `trybuild`). Test tasks come first in each story and must fail (or not compile)
before the implementation task after them.

**Organization**: Grouped by user story. Core ships as **two PRs** (plan.md Summary, research
R-1): **core-1** (Phases 1–7: every type, standalone `Describe`, `Error::Setup`, shims) and
**core-2** (Phases 8–10: `Actuator: Describe`, shims removed, tests over every compiled-in model).
core-2 cannot start before xtask X1 merges, so the parts of US2 and US3 that need generated
`Describe` impls sit in Phase 9 (core-2), after all core-1 work. Task IDs follow execution order.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1 = typed goal + limit check, US2 = model-generic `Actuator` + `TorqueState`,
  US3 = errors by category, US4 = raw values
- **[human]**: only a human (the architect) does this task; agents never do it

## Rules for every task

- Agents may edit only core paths: `src/lib.rs`, `src/error.rs`, `src/actuator.rs`,
  `src/register.rs`, `src/units.rs`, `src/units/**` (`src/id.rs` stays unchanged). Never edit
  `src/models.rs`, `src/models/**` (generated: only `cargo xtask codegen` writes them, and core
  never regenerates), `xtask/**`, `control_tables/**`, `src/protocol*`, `src/transport*`,
  `Cargo.toml` (plan Technical Context: Constraints).
- `.aiignore` paths are human-only: `.aiignore`, `.claude/settings.json`,
  `.github/workflows/ov-*.yml`, `.github/ov-ruleset-main.json`, `.specify/extensions/ov/**`,
  `docs/srs/SRS.md`. Any change needed there is a `[human]` task. The constitution and
  `docs/ARCHITECTURE.md` are architect-owned (module-interfaces §1): also `[human]`.
- Rust 2024, layout `foo.rs` + `foo/` (`src/units.rs` + `src/units/position.rs`), never `mod.rs`.
- Document every public item and every field (`#![warn(missing_docs)]` comes with PR #7). No
  comments inside fn bodies unless they resolve a real ambiguity.
- No panic on any input (no `unwrap`, `expect(`, `panic!`, unchecked indexing, overflowing
  arithmetic in non-test code), no allocation, no `dyn` (FR-018). Every conversion returns
  `Result`. Rounding without `f32::round` (not in `core`, research R-5).
- Crate-internal items that only later PRs use carry `#[allow(dead_code, reason = "…")]`, never
  `expect(dead_code)` (research R-4).
- Unit tests live next to the code (`#[cfg(test)] mod tests`). `proptest` is already a
  dev-dependency (PR #7). Test messages with `extern crate std; use std::string::ToString;` as the
  existing tests do.
- Commits and PRs carry no AI co-authorship or attribution lines (constitution, Workflow). PRs are
  authored by Ovsiankina.

---

## Phase 1: Setup (PR core-1)

**Purpose**: Branch from a `main` that carries `ProtocolVersion`, record the baseline, add the
`units` skeleton

- [ ] T001 Check prerequisites and create the branch: `git fetch origin`; confirm protocol PR #7 is merged into `origin/main` with `git show origin/main:src/protocol.rs | grep -n "pub enum ProtocolVersion"`, `git show origin/main:src/lib.rs | grep -n "ProtocolVersion"` (root re-export `pub use protocol::{Clock, P1, P2, Protocol, ProtocolVersion};`), `git show origin/main:Cargo.toml | grep -n proptest` and `git show origin/main:src/lib.rs | grep -n missing_docs`. If any is missing, stop: core-1 is blocked (research R-15, core never defines a stand-in). Check for unmerged work and a dirty tree, then create branch `004-core-1` from `origin/main`
- [ ] T002 Run the quickstart.md §1 gates on the fresh branch and record the result in the PR notes: every gate passes except `RUSTFLAGS="-D warnings" cargo build --lib --no-default-features`, which fails with "associated function `new` is never used" in src/register.rs (expected, research R-4, fixed by T007). Change nothing in this task
- [ ] T003 Create src/units.rs with a module doc (typed SI values; `Position` is radians in `f32`; every typed value keeps a raw form, FR-004) and `mod position;`, and create src/units/position.rs with a module doc only. Add `mod units;` to src/lib.rs (alphabetical, after `mod transport;`). `cargo build` passes

---

## Phase 2: Foundational (PR core-1)

**Purpose**: The validation variants every story returns, and the `Register` shape that US2
(`ModelData`) and US4 (`encode`/`decode`) build on

**⚠️ CRITICAL**: No user-story work starts until this phase is complete

- [ ] T004 [P] Add test `core_validation_messages` to `mod tests` in src/error.rs: a table of `(ValidationError, &str)` checked through `Error::from(err).to_string()`, with exactly the contract texts (contracts/core-api.md § Error messages): `ActuatorIsOff { id: 4 }` → "actuator 4 has torque off; enable torque before setting a goal"; `ActuatorFaulted { id: 4 }` → "actuator 4 is faulted; clear the fault before enabling torque"; `OutOfLimits { id: 3, raw: 3001, min: 1000, max: 3000 }` → "goal 3001 for actuator 3 is outside its limits 1000..=3000"; `LimitsUnknown { id: 3 }` → "limits of actuator 3 are not known; they are read at setup"; `InvertedLimits { id: 3, min: 3000, max: 1000 }` → "actuator 3 reports a minimum limit 3000 above its maximum 1000"; `NotFinite` → "angle is not a finite number"; `DoesNotFit { raw: 70000, size: 2 }` → "value 70000 does not fit a 2-byte register"; `DoesNotFit { raw: 3000000000, size: 0 }` → "value 3000000000 does not fit in 32 bits"; `BufferLength { targets: 3, got: 2 }` → "buffer has 2 slots for 3 targets"; `DuplicateId { id: 5 }` → "actuator 5 is listed more than once". Add test `validation_converts_under_its_category`: for each, `Error::from(v) == Error::Validation(v)`. Covers US3 AS1, AS2 for the validation half. Must not compile yet
- [ ] T005 [P] Rewrite `mod tests` in src/register.rs: `shim_keeps_address_and_size` asserts `Register::new(116, 4)` gives `addr == 116`, `size == 4`, `access == Access::R`, `area == Area::Eeprom` (the core-1 placeholders, data-model.md § Register); `access_and_area_are_comparable` checks `Access::R != Access::RW` and `Area::Eeprom != Area::Ram`. Must not compile yet
- [ ] T006 Add the nine variants to `ValidationError` in src/error.rs after the four existing ones (`InvalidId(u8)`, `AddressOutOfRange { addr: u16 }`, `PacketTooLarge { len: usize, max: usize }`, `DataLength { expected: usize, got: usize }` stay unchanged): `ActuatorIsOff { id: Id }`, `ActuatorFaulted { id: Id }`, `OutOfLimits { id: Id, raw: i32, min: i32, max: i32 }`, `LimitsUnknown { id: Id }`, `InvertedLimits { id: Id, min: i32, max: i32 }`, `NotFinite`, `DoesNotFit { raw: i64, size: u8 }` (doc: "`size` 0 = does not fit `i32`"), `BufferLength { targets: usize, got: usize }`, `DuplicateId { id: Id }` (contracts/core-api.md § Errors). Messages exactly as in T004; `{id}` is `id.get()` (no `Display for Id`, research R-14). Format the `DoesNotFit` size part with a private `Display` helper struct next to `FeatureHint` (size 0 → "in 32 bits", else "a {size}-byte register"). Doc every variant and field. Enum stays `Debug, Clone, Copy, PartialEq, Eq, thiserror::Error`, `#[non_exhaustive]`. T004 passes
- [ ] T007 In src/register.rs (contracts/core-api.md § Crate-internal, data-model.md § Register): add `pub enum Access { R, RW }` and `pub enum Area { Eeprom, Ram }` (`Debug, Clone, Copy, PartialEq, Eq`, documented, each with `#[allow(dead_code, reason = "RW and Ram are built by X1's generated reg() calls")]`); make `Register` a `pub struct` (the module stays private, never re-exported) with `pub(crate)` fields `addr: u16`, `size: u8` ("1, 2 or 4 for every register the core code encodes"), `access: Access`, `area: Area`; replace today's `#[cfg_attr(not(test), expect(dead_code, …))]` with `#[allow(dead_code, reason = "fields read by bus-1")]` on the struct; keep `pub(crate) const fn new(addr: u16, size: u8) -> Self` as the shim filling `Access::R`, `Area::Eeprom`, marked `#[doc(hidden)]` and `#[allow(dead_code, reason = "unused once X1 regenerates with reg(); removed in core-2")]` (deviation D1). Do not add a 4-argument constructor. T005 passes
- [ ] T008 Checkpoint: `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings` and `RUSTFLAGS="-D warnings" cargo build --lib --no-default-features` all pass (the no-family line now passes). `cargo xtask codegen && git diff --exit-code -- src/models.rs src/models` shows no diff. Fix fallout only in files from T003–T007

**Checkpoint**: Foundation ready. US1–US4 (core-1 parts) can start.

---

## Phase 3: User Story 1 - Give a goal in radians and have it checked against the real limits (Priority: P1, PR core-1) 🎯 MVP

**Goal**: `Position` (radians, `f32`), `PositionScale` (zero tick + slope from `type_info`),
`Limits` and the one limit check on `Cache` (FR-001…FR-007).

**Independent Test**: `cargo test units:: && cargo test actuator::tests::cache` — for the XM430
scale (2048, 0..=4095, ±π) and the AX scale (512, 0..=1023, 0..=5.2359877), angles convert to
ticks and back; goals are accepted and rejected against recorded limits; no bus involved.

### Tests for User Story 1 ⚠️ (write first, must fail)

- [ ] T009 [P] [US1] Conversion tests in `mod tests` of src/units/position.rs. Helper `xm() = PositionScale::new(2048, 0, 4095, -3.14159265, 3.14159265)`, `ax() = PositionScale::new(512, 0, 1023, 0.0, 5.2359877)`. Tests: `zero_radians_is_the_zero_tick` (US1 AS1: `xm().to_raw(Position::ZERO) == Ok(2048)`, `ax()` gives 512); `every_tick_of_every_table_scale_round_trips` (US1 AS2, SC-001 per scale): for each of the eight `type_info` scales of research R-6 — (2048, 0..=4095, ±3.14159265), (512, 0..=1023, 0.0..=5.2359877), (0, ±303750, ±3.14159265) H42/PH42, (0, ±501923, ±3.14159265) H54/PH54, (0, ±263187, ±3.14159265) M42/PM42, (0, ±251417, ±3.14159265) M54/PM54, (0, ±740, ±1.099) RH-P12-RN, (0, ±262144, ±3.14159265) Y-series — every tick `t` in `min_raw..=max_raw` gives `to_raw(to_position(t)) == Ok(t)`; `multi_turn_ticks_round_trip` (edge "beyond one turn": XM scale, ticks −2²⁰..=2²⁰ step 7); `rounds_half_away_from_zero` (edge "between two ticks": an angle 0.4 tick above zero gives 2048, 0.6 gives 2049, −0.5 tick gives 2047); `non_finite_angles_are_refused` (US1 AS5: `f32::NAN`, `f32::INFINITY`, `f32::NEG_INFINITY` → `Err(ValidationError::NotFinite)`); `angles_beyond_i32_do_not_fit` (`from_radians(1e30)` → `DoesNotFit { size: 0, .. }`); `to_position_takes_any_i32` (`i32::MIN` and `i32::MAX` give finite radians, research R-8); `ax_angle_below_zero_gives_negative_tick` (edge: on `ax()`, −π/2 rad gives `Ok(t)` with `t < 0`); `position_arithmetic` (SC-005: `Position::from_radians(0.5) + Position::from_radians(0.1)` has `radians()` ≈ 0.6; `-`, `+=`, `-=`, `Position::default() == Position::ZERO`, `PartialOrd`); `proptest` cases `to_raw_never_panics(any::<f32>())` and `to_position_never_panics(any::<i32>())` (US1 AS5, SC-002 "zero panics")
- [ ] T010 [US1] `Limits` tests in `mod tests` of src/units/position.rs (same file as T009): `inverted_limits_are_refused` (`Limits::new(id(3), 3000, 1000) == Err(ValidationError::InvertedLimits { id: id(3), min: 3000, max: 1000 })`, data-model: "`min > max` gives `InvertedLimits { id, min, max }`"); `equal_limits_are_accepted` ("`min == max` is accepted"); `limits_keep_raw_bounds` (`min_raw()`, `max_raw()`)
- [ ] T011 [P] [US1] `Cache` limit-check tests in a `mod cache` inside `mod tests` of src/actuator.rs: `goal_outside_limits_names_id_tick_and_limits` (US1 AS3, SC-002: `Cache::new(id(3))`, `record_limits(1000, 3000)`, then `check_goal(3001)` and `check_goal(999)` give `OutOfLimits { id: id(3), raw, min: 1000, max: 3000 }`); `goals_on_the_bounds_pass` (US1 AS4: 1000 and 3000 give `Ok(raw)`); `unknown_limits_refuse_every_goal` (US1 AS6, FR-007: `Cache::new(id(3)).check_goal(2048) == Err(LimitsUnknown { id: id(3) })`); `inverted_record_leaves_limits_unknown` (FR-005 edge: `record_limits(3000, 1000)` gives `InvertedLimits` and `limits()` stays `None`); `typed_goal_is_checked_on_the_rounded_tick` (edge: an XM430 angle converted with `to_raw` that rounds to 3001 is refused by `check_goal`)

### Implementation for User Story 1

- [ ] T012 [US1] Implement `Position` in src/units/position.rs (contracts/core-api.md; data-model.md § Position: "`f32` (private) | Radians. Zero is the model's zero-radian tick. Any `f32` is storable; NaN and infinity are refused when converted to ticks."): `#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)] pub struct Position(f32)`, `pub const ZERO`, `pub const fn from_radians(f32) -> Self`, `pub const fn radians(self) -> f32`, `Add`, `Sub`, `AddAssign`, `SubAssign`. Doc states the unit, the zero reference (2048 on XM430, 512 on AX-12A) and the `f32` limit ("exact ticks up to an offset of 2²⁴", research R-6; the raw path covers more), with a doctest `let moved = goal + Position::from_radians(0.1);` (SC-005)
- [ ] T013 [US1] Implement `PositionScale` in src/units/position.rs (data-model.md § PositionScale; research R-5, R-6, R-7, R-8): private fields `zero_raw`, `min_raw`, `max_raw: i32`, `min_rad`, `max_rad: f32`, `radians_per_tick` and `ticks_per_radian: f32`, the last two computed in `pub(crate) const fn new(zero_raw, min_raw, max_raw, min_rad, max_rad)` as `(max_rad - min_rad) / (max_raw - min_raw)` with the tick span taken in `i64` then `f32`, and its inverse; `#[allow(dead_code, reason = "called by generated ModelData from X1")]` on `new`. Public: `pub const fn zero_raw(&self) -> i32`, `pub fn radians_per_tick(&self) -> f32`, `pub fn to_position(&self, raw: i32) -> Position` = "`(raw − zero_raw)` in `i64`, as `f32`, × `radians_per_tick`", `pub fn to_raw(&self, p: Position) -> Result<i32, ValidationError>` = "`p × ticks_per_radian`, round half away from zero, + `zero_raw` in `i64`", errors "`NotFinite` (NaN, ±∞); `DoesNotFit { raw, size: 0 }` outside `i32`"; rounding is `if x >= 0.0 { x + 0.5 } else { x - 0.5 }` then `as i64`, after `is_finite`; `i32::try_from` for the result. "No range clamp: ticks outside `min_raw..=max_raw` convert linearly (multi-turn)." Add a `compile_fail,E0624` doctest showing `PositionScale::new(..)` cannot be called from outside the crate (no scale from user-supplied defaults). T009 passes
- [ ] T014 [US1] Implement `Limits` in src/units/position.rs (data-model.md § Limits: fields `min`, `max: i32`; "Validation: `min > max` gives `InvertedLimits { id, min, max }`. `min == max` is accepted. No constructor from table defaults (FR-007)."): `pub struct Limits` (`Debug, Clone, Copy, PartialEq, Eq`), `pub(crate) const fn new(id: Id, min: i32, max: i32) -> Result<Self, ValidationError>`, `pub(crate) const fn min_raw(&self)`, `max_raw(&self)`, each with `#[allow(dead_code, reason = "used by bus-1")]` where unused. In src/units.rs add `pub use position::{Position, PositionScale};` and `pub(crate) use position::Limits;`; in src/lib.rs add `pub use units::{Position, PositionScale};`. T010 passes
- [ ] T015 [US1] Implement `Cache` in src/actuator.rs (data-model.md § Cache: "`id` | `Id` | Set at construction. Never changes." / "`limits` | `Option<Limits>` | `None` until Setup records the EEPROM values." / "No torque state, no fault state, no fault report (SC-008). 16 bytes."): `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub struct Cache { id: Id, limits: Option<Limits> }` (private fields, never re-exported), `pub(crate) const fn new(id)`, `const fn id`, `const fn limits`, `fn record_limits(&mut self, min, max) -> Result<Limits, ValidationError>` ("`Ok(Limits)` and stored, or `InvertedLimits` and unchanged"), `fn check_goal(&self, raw: i32) -> Result<i32, ValidationError>` ("`Ok(raw)` inside `min..=max`; `LimitsUnknown { id }`; `OutOfLimits { id, raw, min, max }`", bounds inclusive). Doc: the one limit check for typed and raw paths; no constructor fills limits from a control table. `#[allow(dead_code, reason = "used by X1 / bus-1")]` on unused items. T011 passes
- [ ] T016 [US1] Checkpoint: `cargo test units:: && cargo test actuator::tests::cache` pass; `cargo clippy --all-targets -- -D warnings` and the no-family build stay clean

**Checkpoint**: US1 works on its own: typed goal → tick → limit check, no bus.

---

## Phase 4: User Story 2 - Write code once that works on any actuator model (Priority: P1, PR core-1 part; part 2 in Phase 9, PR core-2)

**Goal (core-1 part)**: `ModelNumber`, `TorqueState` with its rules, and the crate-internal model
description (`ModelData`, `LimitRegisters`, `Protocols`, sealed `Describe`) beside the unchanged
3-method `Actuator`; today's generated code keeps compiling (FR-008…FR-012, D2).

**Independent Test**: `cargo test actuator::` — the state value goes through every transition of
the table; a hand-built XM430 and AX-12A `ModelData` give the right role registers under the same
names; `size_of::<TorqueState>() == 1`.

### Tests for User Story 2 ⚠️ (write first, must fail)

- [ ] T017 [US2] Apply deviation D2 to `mod tests` in src/actuator.rs (research R-3): delete `builds_model_from_number`, `picks_table_by_firmware` and `unknown_model_is_unsupported` (X1 owns lookup tests); move `unsupported_model_message_names_the_feature` out (it is rewritten on `SetupError` in T026); rewrite `concrete_types_implement_actuator` to build `crate::models::Ax12a::new(id(9))` (`#[cfg(feature = "ax")]`) and `ActuatorKind::Xm430W350(crate::models::Xm430W350::new(id(3)))` (`#[cfg(feature = "x-series")]`) and check `id`, `model_number`, `name` (quickstart §2 "FR-011 shape"). No remaining test calls `ActuatorKind::new`
- [ ] T018 [US2] `TorqueState` and `ModelNumber` tests in `mod tests` of src/actuator.rs: `new_state_is_off` (US2 AS3: `TorqueState::default() == TorqueState::Off`); `state_is_one_byte` (SC-008: `size_of::<TorqueState>() == 1`, `size_of::<Option<TorqueState>>() == 1`); `transition_table` checking every cell of data-model.md § TorqueState — `enable(id)`: Off→On, On→On, Faulted→`Err(ActuatorFaulted { id })`; `disable()`: Off→Off, On→Off, Faulted→Faulted; `fault()`: every state→Faulted; `clear()`: Off→Off, On→On, Faulted→Off; `fault_is_one_step_and_counts_as_off` (US2 AS4: `On.fault()` is `Faulted`, `!Faulted.is_on()`); `enable_while_faulted_is_refused` (US2 AS5: `Faulted.enable(id(4)) == Err(ActuatorFaulted { id: id(4) })`, value unchanged); `queries` (`is_on`: false/true/false; `is_faulted`: false/false/true; `check_goal(id)`: Off → `Err(ActuatorIsOff { id })`, On → `Ok(())`, Faulted → `Err(ActuatorIsOff { id })`); `model_number_converts_and_displays` (`ModelNumber::from(1020).get() == 1020`, `to_string() == "1020"`, `Ord`)
- [ ] T019 [US2] Model-description tests in `mod tests` of src/actuator.rs: `cache_holds_only_id_and_limits` (US2 AS2 core-1 half, SC-008: `let Cache { id: _, limits: _ } = Cache::new(id(3));` compiles only with exactly those two fields, `Cache::new(id(3)).limits() == None`, `size_of::<Cache>() == 16`); test fixtures `const XM430: ModelData` (name "XM430-W350", model_number 1020, protocols `{ p1: true, p2: true }`, torque_enable 64/1, goal_position 116/4, present_position 132/4, limits `MinMax { min: 52/4, max: 48/4 }`, hardware_error `Some(70/1)`, scale (2048, 0, 4095, ±3.14159265)) and `const AX12A: ModelData` (number 12, `{ p1: true, p2: false }`, torque 24/1, goal 30/2, present 36/2, limits `CwCcw { cw: 6/2, ccw: 8/2 }`, hardware_error `None`, scale (512, 0, 1023, 0.0, 5.2359877)) built with `Register::new` (research R-9 e-Manual values); `goal_register_per_model` (US2 AS1 core-1 analogue: a generic `fn goal<D: Describe>(d: &D) -> (u16, u8)` over a test-only `struct Fake { data: &'static ModelData, cache: Cache }` implementing `Describe` gives (116, 4) and (30, 2)); `limit_registers_share_names` (US2 AS6: `AX12A.limits.min().addr == 6`, `.max().addr == 8`; XM430 52 and 48); `protocols_support` (`Protocols { p1: true, p2: false }.supports(ProtocolVersion::P1)` true, `P2` false)
- [ ] T020 [US2] Doc examples on `Actuator` in src/actuator.rs: a runnable example `fn describe<A: Actuator>(a: &A) -> (u8, &'static str) { (a.id().get(), a.name()) }` (quickstart §4 step 1); a `compile_fail,E0603` example `use dxl_rs_v2::actuator::Describe;` showing `Describe` cannot be named outside the crate (contracts/core-api.md Doc requirements)

### Implementation for User Story 2

- [ ] T021 [US2] Implement `ModelNumber` in src/actuator.rs (data-model.md: "`0` | `pub u16` | Any value. Whether this build supports it is decided at Setup."): `#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)] pub struct ModelNumber(pub u16)`, `pub const fn get(self) -> u16`, `From<u16>`, `Display` (the plain number). Doc: "A model number as the actuator reports it (Model Number register, address 0, 2 bytes)." `Actuator::model_number()` keeps returning `u16`
- [ ] T022 [US2] Implement `TorqueState` in src/actuator.rs (data-model.md: "`#[repr(u8)]`: `Off = 0` (default), `On = 1`, `Faulted = 2`. Exactly 1 byte. Faulted counts as off. Stored by the bus, one per slot, beside the actuator list."): `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)] #[repr(u8)] pub enum TorqueState { #[default] Off = 0, On = 1, Faulted = 2 }`, not `#[non_exhaustive]`; `pub const fn is_on`, `pub const fn is_faulted`; `pub(crate) const fn enable(self, id: Id) -> Result<Self, ValidationError>`, `disable(self) -> Self`, `fault(self) -> Self`, `clear(self) -> Self`, `check_goal(self, id: Id) -> Result<(), ValidationError>` per the transition table, each `#[allow(dead_code, reason = "used by bus-1")]`. Doc: the transition table, a runnable exhaustive `match` with no `_` arm, and a `compile_fail,E0624` example calling `TorqueState::Off.enable(id)` from outside. T018 passes
- [ ] T023 [US2] Implement in src/actuator.rs (contracts/core-api.md § Crate-internal; data-model.md § ModelData): `pub enum LimitRegisters { MinMax { min: Register, max: Register }, CwCcw { cw: Register, ccw: Register } }` with `pub(crate) const fn min(&self)` ("`min()` = Min or CW") and `max(&self)` ("`max()` = Max or CCW"); `pub struct Protocols { pub(crate) p1: bool, pub(crate) p2: bool }` with `pub(crate) const fn supports(self, protocol: crate::protocol::ProtocolVersion) -> bool`; `pub struct ModelData` with `pub(crate)` fields `name: &'static str`, `model_number: u16`, `protocols: Protocols`, `torque_enable`, `goal_position`, `present_position: Register`, `limits: LimitRegisters`, `hardware_error: Option<Register>` ("Hardware Error Status, or `None` (AX, Y-series tables have none)"), `scale: PositionScale`. Derives per the contract. None re-exported. `#[allow(dead_code, reason = "built by X1's generated DATA consts; read by bus-1")]` on each. T019 passes
- [ ] T024 [US2] Add the sealed trait in src/actuator.rs: `pub(crate) mod sealed { pub trait Describe { fn data(&self) -> &'static ModelData; fn cache(&self) -> &Cache; fn cache_mut(&mut self) -> &mut Cache; } }` and `pub(crate) use sealed::Describe;` (plan clarification; xtask plan C1), documented "What the bus needs from a model. No I/O.", with `#[allow(dead_code, reason = "implemented by X1's generated code")]`. `Actuator` stays exactly as today (three required methods, no supertrait). T020 passes
- [ ] T025 [US2] In src/lib.rs change `pub use actuator::Actuator;` to `pub use actuator::{Actuator, ModelNumber, TorqueState};`. Checkpoint: `cargo test actuator::`, `cargo clippy --all-targets -- -D warnings`, the no-family build and the no_std clippy line pass

**Checkpoint**: US2 core-1 part done. `Describe` exists alone; generated code is untouched.

---

## Phase 5: User Story 3 - Handle every failure by its category (Priority: P2, PR core-1 part; part 2 in Phase 9, PR core-2)

**Goal (core-1 part)**: the `Setup` category with its five variants; `UnsupportedModel` moves into
it while a `#[doc(hidden)]` top-level copy keeps today's generated code building (FR-014…FR-017).

**Independent Test**: `cargo test error::` plus `cargo test --doc` — every error is built, matched
by category, compared, copied and its message checked.

### Tests for User Story 3 ⚠️ (write first, must fail)

- [ ] T026 [US3] Setup tests in `mod tests` of src/error.rs: `setup_messages` checks through `Error::from(err).to_string()` exactly: `UnsupportedModel { id: 3, model_number: 1020, protocol: ProtocolVersion::P1, feature: Some("x-series") }` → "actuator 3 reports model number 1020, not supported on Protocol 1 (enable the `x-series` feature)"; same with `feature: None` → "actuator 3 reports model number 1020, not supported on Protocol 1"; `TooManyActuators { capacity: 32 }` → "more actuators answered than the bus capacity of 32"; `Missing { id: 5 }` → "declared actuator 5 did not answer" (US3 AS5); `WrongModel { id: 5, declared: 1020, found: 12 }` → "actuator 5 was declared as model number 1020 but reports 12"; `NotOnBus { id: 7 }` → "actuator 7 is not on this bus"; `setup_is_its_own_category` (US3 AS5: `matches!(Error::from(SetupError::Missing { id: id(5) }), Error::Setup(SetupError::Missing { id }) if id == id(5))`); `unsupported_model_is_under_setup` (US3 AS6: matched as `Error::Setup(SetupError::UnsupportedModel { .. })`; this replaces the D2-moved `unsupported_model_message_names_the_feature`)
- [ ] T027 [US3] Category tests in src/error.rs: `errors_are_copy_eq_and_alloc_free` (FR-017, US3 AS4: a `const fn assert_traits<T: Copy + Eq + core::error::Error>()` called for `Error`, `ValidationError`, `SetupError`; copy an error and compare); doc examples on `Error`: a runnable match "category first, detail second" with a `_` arm (US3 AS3, quickstart §4 step 3), and `compile_fail,E0004` examples on `Error`, `ValidationError` and `SetupError` showing that a match without `_` does not compile outside the crate (`#[non_exhaustive]`, US3 AS3 "a new variant added later does not break their match")

### Implementation for User Story 3

- [ ] T028 [US3] In src/error.rs (contracts/core-api.md § Errors; research R-14): add `#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)] #[non_exhaustive] pub enum SetupError { UnsupportedModel { id: Id, model_number: u16, protocol: ProtocolVersion, feature: Option<&'static str> }, TooManyActuators { capacity: usize }, Missing { id: Id }, WrongModel { id: Id, declared: u16, found: u16 }, NotOnBus { id: Id } }` with the T026 messages (reuse `FeatureHint` for the feature suffix; `ProtocolVersion` from `crate::protocol`, its `Display` gives "Protocol 1"/"Protocol 2"); add `Error::Setup(#[from] SetupError)` with `#[error(transparent)]`; mark the existing top-level `Error::UnsupportedModel` `#[doc(hidden)]` and document it as "kept for today's generated code; use `SetupError::UnsupportedModel`; removed in core-2". Update the `Error` doc to list the five categories (`Transport`, `Protocol`, `Status`, `Validation`, `Setup`). Doc every variant and field. T026, T027 pass
- [ ] T029 [US3] In src/lib.rs add `SetupError` to the existing `pub use error::{…}` list. Do not add another `ProtocolVersion` re-export: PR #7 already exports it (verified in T001)

**Checkpoint**: US3 core-1 part done. Generated code still uses the hidden variant.

---

## Phase 6: User Story 4 - Drop to raw register values when needed (Priority: P3, PR core-1)

**Goal**: `Register::encode`/`decode` that never truncate, and proof that raw and typed goals hit
the same check (FR-004, FR-013).

**Independent Test**: `cargo test register::` — raw ticks pass the limit check and the byte
encoding unchanged; out-of-limit and oversized values are refused.

### Tests for User Story 4 ⚠️ (write first, must fail)

- [ ] T030 [P] [US4] Encode/decode tests in `mod tests` of src/register.rs: `encodes_2500_in_four_bytes` (US4 AS1: `Register::new(116, 4).encode(2500, &mut out) == Ok(&[0xC4, 0x09, 0x00, 0x00][..])`); `refuses_values_that_do_not_fit` (US4 AS3: 70 000 in size 2 → `DoesNotFit { raw: 70000, size: 2 }`; −1 and 256 in size 1; −1 in size 2 (AX edge: a negative tick is refused, not wrapped); any value in size 3 → `DoesNotFit`); `encode_writes_size_bytes` (returned slice length equals `size` for 1, 2, 4; size 4 accepts `i32::MIN` and `i32::MAX`); `decode_sign_rules` (`FF FF` size 2 → 65535; `FF FF FF FF` size 4 → −1; `80` size 1 → 128); `decode_checks_length` (`[0; 2]` on size 4 → `DataLength { expected: 4, got: 2 }`); `decode_refuses_other_sizes` (size 3 → `DoesNotFit`); `proptest` `round_trip_when_it_fits(raw in any::<i32>(), size in prop::sample::select(vec![1u8, 2, 4]))`: if `encode` accepts, `decode(encode(x)) == x` (data-model.md invariant), and neither call panics
- [ ] T031 [P] [US4] Test `raw_and_typed_goals_hit_the_same_check` in `mod tests` of src/actuator.rs (US4 AS2): with limits 1000..=3000 on `id(3)`, `check_goal(3001)` and `check_goal(xm_scale.to_raw(xm_scale.to_position(3001))?)` return the same `OutOfLimits` error; `check_goal(2500)` returns `Ok(2500)`

### Implementation for User Story 4

- [ ] T032 [US4] Implement in src/register.rs (data-model.md § Register; research R-9): `pub(crate) fn encode(self, raw: i32, out: &mut [u8; 4]) -> Result<&[u8], ValidationError>` — "size 1: `0..=255`; size 2: `0..=65535`; size 4: any `i32`; little-endian into `out[..size]`", error "`DoesNotFit { raw, size }` (also for any other size)", bytes are `raw.to_le_bytes()` truncated only after the range check; `pub(crate) fn decode(self, bytes: &[u8]) -> Result<i32, ValidationError>` — "`bytes.len() == size`; sizes 1, 2 unsigned; size 4 two's-complement", errors "`DataLength { expected, got }`; `DoesNotFit` for another size". No indexing that can panic (use `get`/`try_into`). Each `#[allow(dead_code, reason = "used by bus-1")]`. Doc both. T030, T031 pass

**Checkpoint**: All four stories' core-1 parts work; core-1 is feature-complete.

---

## Phase 7: Polish & PR core-1

**Purpose**: Cross-cutting checks, the X1 simulation, and opening core-1

- [ ] T033 [P] Docs pass over src/error.rs, src/actuator.rs, src/register.rs, src/units.rs, src/units/position.rs: every public item and field documented (FR-019); `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` passes; `cargo test --doc` runs every `compile_fail` example with its pinned code
- [ ] T034 [P] Static audit of the core files (FR-018, SC-006): `rg -n "dyn |Box<|alloc::|String|Vec<" src/error.rs src/actuator.rs src/register.rs src/units.rs src/units/` finds nothing; `rg -n "unwrap\(|expect\(|panic!|unreachable!" ` over the same files finds hits only inside `#[cfg(test)]` modules or doctests; `rg -n "expect\(dead_code" src/` finds nothing in core files; no `mod.rs` exists; no comments inside fn bodies
- [ ] T035 Run all seven quickstart.md §1 gates: `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`; `cargo test --workspace`; `cargo clippy --no-default-features --features ax,mx,x-series,y-series,pro --target thumbv7em-none-eabihf -- -D warnings`; `RUSTFLAGS="-D warnings" cargo build --lib --no-default-features`; `cargo xtask codegen && git diff --exit-code -- src/models.rs src/models`; `cargo llvm-cov --workspace --ignore-filename-regex 'src/models(\.rs|/)' --fail-under-lines 70`. All pass
- [ ] T036 Simulate X1 (quickstart.md §1 "core-1 only"; research R-4): `git worktree add` a scratch copy under the session scratchpad; in it, add to src/models.rs `const fn reg(addr: u16, size: u8) -> crate::register::Register { crate::register::Register { addr, size, access: crate::register::Access::R, area: crate::register::Area::Eeprom } }` and replace every `Register::new(` in src/models/*.rs with `super::reg(`; run `cargo clippy --all-targets -- -D warnings` and `RUSTFLAGS="-D warnings" cargo build --lib --no-default-features`; both must stay clean (the shim's unconditional `allow` holds). Remove the worktree; nothing from it is committed
- [ ] T037 [human] Architect approvals needed before core-1 merges (plan Complexity Tracking, module-interfaces §9): (a) core in two PRs against "one module per PR"; (b) conflict 19: `SetupError::UnsupportedModel` gains `id` and `protocol` beyond spec FR-016's `{ model_number, feature }`; (c) open risk research R-12 (`cache_mut()` callable through `A: Actuator` once core-2 lands; a token argument would change X1's emitted impl)
- [ ] T038 Commit (no AI attribution), push `004-core-1` and open the PR into `main` with the body from .github/pull_request_template.md: What/Why (SRS module `core`, FR-001…FR-020 core-1 part); Architecture section listing deviations D1 and D2, the shims (`#[doc(hidden)] Error::UnsupportedModel`, 2-arg `#[doc(hidden)] Register::new`, `allow(dead_code)` items), the fifth error category, T037's open points, and that the no-family `-D warnings` build is now green (unblocks ci's feature-set job); checklist ticked from T035; follow-ups for X1 (`reg()` helper, `crate::actuator::Describe` path, lookup tests now X1's)

**Checkpoint (PR core-1)**: core-1 merged into `main`. X1 can start (with tables T1).

---

## Phase 8: Setup (PR core-2)

**Purpose**: Start core-2 only on a `main` that carries X1's generated `Describe` impls

- [ ] T039 Check prerequisites and create the branch: `git fetch origin`; on `origin/main` confirm core-1 is merged and xtask X1 is merged: `git grep -n "Describe for" origin/main -- src/models.rs src/models/xm430_w350.rs` finds impls; `git grep -n "Register::new(" origin/main -- src/models.rs src/models` finds nothing (registers built by `reg()`); `git grep -n "Error::UnsupportedModel" origin/main -- src/models.rs` finds nothing; `ActuatorKind::new` has the signature `(ProtocolVersion, ModelNumber, u8, Id) -> Result<Self, SetupError>` and `firmware_register(ProtocolVersion, ModelNumber) -> Option<Register>` exists. If not, stop: core-2 is blocked. Create `004-core-2` from `origin/main`; run the quickstart §1 gates as a baseline (all pass)

---

## Phase 9: User Stories 2 and 3, part 2 - trait cutover (Priority: P1/P2, PR core-2)

**Goal**: `Actuator: sealed::Describe` with six provided methods, one generic test over every
compiled-in model (SC-001 per model, SC-003), the top-level `Error::UnsupportedModel` and the
`Register::new` shim removed (FR-010, FR-011, FR-016).

**Independent Test**: `cargo test actuator::` — one generic check passes for every variant
`ActuatorKind::new` can return and for the enum holding each; `cargo test --doc` proves the trait
is sealed and the old variant is gone.

### Tests for User Stories 2 and 3, part 2 ⚠️ (write first, must fail)

- [ ] T040 [P] [US2] Test support in src/units/position.rs: `#[cfg(test)] pub(crate) const fn raw_range(&self) -> core::ops::RangeInclusive<i32>` on `PositionScale` returning `min_raw..=max_raw` (tests only; not part of the contract)
- [ ] T041 [US2] Generic tests in `mod tests` of src/actuator.rs (research R-16; quickstart §3): helper `every_variant(id) -> std::vec::Vec<ActuatorKind>` calling `ActuatorKind::new(protocol, ModelNumber(n), fw, id)` for every `n` in `0..=u16::MAX`, both `ProtocolVersion::P1` and `P2`, with `fw` = 0 or every value `0..=255` when `ActuatorKind::firmware_register(protocol, ModelNumber(n))` is `Some`, deduped by `core::mem::discriminant`. Tests: `generic_check_passes_for_every_model` (SC-003) running `fn check<A: Actuator>(a: &A, id: Id)` that asserts `a.id() == id`, `a.model_number() == a.data().model_number`, `a.name() == a.data().name`, `a.position_scale() == a.data().scale`, `a.position_limits_raw().is_none()` (US2 AS2: new model, limits unknown); `every_tick_of_every_model_round_trips` (SC-001 per model: dedupe by scale, every tick in `raw_range()` survives `to_position` → `to_raw`); `goal_register_per_model` (US2 AS1: XM430-W350 on P2 `data().goal_position` is addr 116 size 4; AX-12A (`ModelNumber(12)` on P1) addr 30 size 2); `limit_registers_share_names` (US2 AS6: AX-12A `data().limits.min()`/`max()` are 6/8; XM430-W350 52/48); `limits_show_after_recording` (US2 AS2: `cache_mut().record_limits(1000, 3000)` then `position_limits_raw() == Some(1000..=3000)` and `position_limits() == Some(scale.to_position(1000)..=scale.to_position(3000))`); `enum_answers_like_the_inner_model` (US2 AS7: for `ActuatorKind::Xm430W350`, `Ax12a`, `Mx64`, `H54200S500Ra`, `Ym070210M001`, each under its family `cfg`, every `Actuator` method of the enum equals the inner model's). Use the generated type names as X1 emits them
- [ ] T042 [US2] Doc examples on `Actuator` in src/actuator.rs: extend the T020 runnable example with `a.position_scale()` and `a.position_limits()`; add a `compile_fail,E0277` example `struct Mine; impl dxl_rs_v2::Actuator for Mine {}` (sealed: `Mine: Describe` is not satisfied); keep the `compile_fail,E0603` example
- [ ] T043 [P] [US3] In src/error.rs: add a `compile_fail` doc example on `SetupError` that matches `dxl_rs_v2::Error::UnsupportedModel { .. }` (FR-016: the top-level variant no longer exists), pinned to the error code rustc reports (expected `E0599`; check by running `cargo test --doc` after T045); confirm no test in src/ still names `Error::UnsupportedModel`

### Implementation for User Stories 2 and 3, part 2

- [ ] T044 [US2] Switch the trait in src/actuator.rs (contracts/core-api.md "core-2"; research R-13): `pub trait Actuator: sealed::Describe` with every method provided — `id` = `self.cache().id()`, `model_number` = `self.data().model_number`, `name` = `self.data().name`, `position_scale` = `self.data().scale`, `position_limits_raw` = `Option<RangeInclusive<i32>>` from `cache().limits()` (`None` until Setup read the limits from EEPROM), `position_limits` = the same mapped through `position_scale().to_position`. Doc: sealed, describes a model, performs no I/O (I/O is on `ActuatorRef`), registers stay crate-internal (FR-012: no registers beyond OQ-2 on the public trait). X1's emitted `id`/`model_number`/`name` overrides stay valid. T041, T042 pass
- [ ] T045 [P] [US3] Remove the `#[doc(hidden)]` top-level `Error::UnsupportedModel` from src/error.rs (keep `FeatureHint`, now used only by `SetupError`). `Error` has exactly `Transport`, `Protocol`, `Status`, `Validation`, `Setup`. T043 passes
- [ ] T046 [P] [US2] Remove the 2-argument `Register::new` shim from src/register.rs and add no constructor (deviation D1: registers are built only by X1's `reg()`). Rewrite the register tests and the src/actuator.rs test fixtures (`XM430`, `AX12A` from T019) to build registers with struct literals `Register { addr, size, access, area }`, using the e-Manual access/area of each role (Torque Enable and Goal Position `RW`/`Ram`; Present Position `R`/`Ram`; limit registers `RW`/`Eeprom`)
- [ ] T047 [US2] Narrow the dead-code allows (research R-4) in src/actuator.rs, src/units/position.rs and src/register.rs: items used by generated code (`ModelData`, `LimitRegisters`, `Protocols` construction, `Cache::new`, `PositionScale::new`, `Describe`, `Access`, `Area`) get `#[cfg_attr(not(any(feature = "ax", feature = "mx", feature = "x-series", feature = "y-series", feature = "pro")), allow(dead_code, reason = "only generated models construct it"))]`; items first used by the bus keep an unconditional `#[allow(dead_code, reason = "used by bus-1")]`: `Register::encode/decode`, `Cache::record_limits/check_goal`, `TorqueState::enable/disable/fault/clear/check_goal`, `Limits::min_raw/max_raw`, `LimitRegisters::min/max`, `Protocols::supports`, the `Register` struct allow. `cargo clippy --all-targets -- -D warnings` and `RUSTFLAGS="-D warnings" cargo build --lib --no-default-features` both pass

**Checkpoint**: US2 and US3 complete; the generic API works over every compiled-in model.

---

## Phase 10: Polish & PR core-2

**Purpose**: Final checks, user-crate check, opening core-2, architect follow-ups

- [ ] T048 [P] Repeat the docs pass (T033) and the static audit (T034) on the core-2 diff; `rg -n "Register::new|UnsupportedModel \{ model_number" src/actuator.rs src/register.rs src/error.rs src/units.rs src/units/` finds nothing outside the `compile_fail` example
- [ ] T049 Run all seven quickstart.md §1 gates (as in T035). All pass, coverage ≥ 70 % with generated models excluded
- [ ] T050 User-crate check (quickstart.md §4) in a scratch crate under the session scratchpad depending on this repo by path: (1) `fn describe<A: dxl_rs_v2::Actuator>(a: &A) -> (u8, &'static str)`; (2) `Position::from_radians(0.5) + Position::from_radians(0.1)` and `radians()`; (3) `match e { dxl_rs_v2::Error::Setup(dxl_rs_v2::SetupError::Missing { id }) => …, _ => … }`. Compiles with default features with no crate-internal import, and the `_` arm is required; repeat with `default-features = false, features = ["x-series"]` as a `#![no_std]` lib. Delete the scratch crate
- [ ] T051 Commit (no AI attribution), push `004-core-2` and open the PR into `main` from .github/pull_request_template.md. Architecture section calls out the breaking changes (constitution VII): `Error::UnsupportedModel` removed (use `Error::Setup(SetupError::UnsupportedModel { .. })`); `Actuator` is now sealed (users can no longer implement it). Notes: shims removed, deviation D1/D2 complete, R-12 status from T037, follow-up xtask X2 may drop the generated `id`/`model_number`/`name` overrides; bus-1 is unblocked
- [ ] T052 [human] Architect follow-ups, not blocking either PR: amend constitution Principle II (`UnsupportedModel { model_number, feature }` → the `SetupError::UnsupportedModel { id, model_number, protocol, feature }` shape) through `/speckit-constitution` if conflict 19 is confirmed; update docs/ARCHITECTURE.md to the 2026-10-27 decisions (cache = `Id` + limits, one-byte `TorqueState` in the bus, five error categories; module-interfaces conflict 20)

---

## PR map

| PR | Tasks | Branch (off `main`) | Must be merged first | Unblocks |
|---|---|---|---|---|
| **core-1** | T001–T038 (37 agent tasks + T037 [human]) | `004-core-1` | Protocol PR #7 with `ProtocolVersion` and its root re-export (module-interfaces §5, §7 step P). Tables T1 has no dependency on core-1 and may land in parallel | xtask X1 (with T1); ci (feature-set job needs the green no-family build) |
| **core-2** | T039–T052 (13 agent tasks + T052 [human]) | `004-core-2` | core-1 and xtask X1 (which itself needs core-1 and T1) | bus-1 … bus-6 (with X1 and P); optional xtask X2 |

Gate commands to run before opening **each** PR (quickstart.md §1):

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
cargo clippy --no-default-features --features ax,mx,x-series,y-series,pro --target thumbv7em-none-eabihf -- -D warnings
RUSTFLAGS="-D warnings" cargo build --lib --no-default-features      # no family enabled
cargo xtask codegen && git diff --exit-code -- src/models.rs src/models
cargo llvm-cov --workspace --ignore-filename-regex 'src/models(\.rs|/)' --fail-under-lines 70
```

core-1 additionally runs the X1 simulation (T036). core-2 additionally runs the user-crate check
(T050).

---

## Dependencies & Execution Order

### Phase dependencies

- **Phase 1 Setup (core-1)**: needs PR #7 on `main`. Blocks everything.
- **Phase 2 Foundational (core-1)**: after Phase 1. Blocks every story.
- **Phases 3–6 (US1–US4, core-1)**: after Phase 2. US1 adds `Cache` to src/actuator.rs, which
  US2 (T019 fixture) and US4 (T031) use, so in practice US1 → US2; US3 (src/error.rs) and the
  register half of US4 (src/register.rs) can run beside US1/US2.
- **Phase 7 Polish core-1**: after Phases 3–6. T037 [human] must be resolved before merge.
- **Phase 8 Setup (core-2)**: after core-1 **and X1** are merged into `main`.
- **Phase 9 (US2/US3 part 2, core-2)**: after Phase 8.
- **Phase 10 Polish core-2**: after Phase 9. T052 [human] is independent.

### Within each story

- Tests first; they fail or do not compile before their implementation task.
- US1: T009 → T010 (same file) ‖ T011 → T012 → T013 → T014 → T015 → T016.
- US2 (core-1): T017 → T018 → T019 → T020 → T021 → T022 → T023 → T024 → T025 (all src/actuator.rs,
  except T025 in src/lib.rs).
- US3 (core-1): T026 → T027 → T028 → T029.
- US4: T030 ‖ T031 → T032.
- core-2: T040 ‖ T043, then T041 → T042 → T044; T045 ‖ T046 after the tests; T047 last.

### Shared files (never edit in parallel)

src/actuator.rs (T011, T015, T017–T024, T031, T041, T042, T044, T046 fixtures, T047), src/error.rs
(T004, T006, T026–T028, T043, T045), src/lib.rs (T003, T014, T025, T029), src/units/position.rs
(T009, T010, T012–T014, T040, T047), src/register.rs (T005, T007, T030, T032, T046, T047).

## Parallel Examples

### Foundational

```text
Task: "T004 core validation message tests in src/error.rs"
Task: "T005 Register shape tests in src/register.rs"
# then
Task: "T006 ValidationError variants in src/error.rs"
Task: "T007 Access, Area, Register fields and shim in src/register.rs"
```

### User Story 1

```text
Task: "T009 + T010 Position, PositionScale and Limits tests in src/units/position.rs"
Task: "T011 Cache limit-check tests in src/actuator.rs"
```

### User Story 2 (core-1)

All tasks touch src/actuator.rs and run in order. US2 can overlap with US3 (src/error.rs) and with
T030 (src/register.rs):

```text
Task: "T018–T024 TorqueState, ModelNumber, model description in src/actuator.rs"
Task: "T026–T028 SetupError in src/error.rs"
Task: "T030 encode/decode tests in src/register.rs"
```

### User Story 3 (core-1)

```text
Task: "T026 + T027 setup and category tests in src/error.rs"   # same file, in order
Task: "T030 encode/decode tests in src/register.rs"            # other story, other file
```

### User Story 4

```text
Task: "T030 encode/decode tests in src/register.rs"
Task: "T031 raw vs typed check test in src/actuator.rs"
```

### core-2 (US2/US3 part 2)

```text
Task: "T040 raw_range test helper in src/units/position.rs"
Task: "T043 UnsupportedModel-absent doc example in src/error.rs"
# after T044
Task: "T045 remove Error::UnsupportedModel in src/error.rs"
Task: "T046 remove Register::new shim in src/register.rs"
```

---

## Implementation Strategy

### MVP first (= PR core-1)

1. Phases 1–2: branch, baseline, validation variants, `Register` shape. The no-family build turns
   green at T008.
2. Phase 3 (US1): typed position and the limit check. Stop and validate: `cargo test units::`.
3. Phases 4–6 (US2, US3, US4 core-1 parts).
4. Phase 7: gates, X1 simulation, PR. core-1 is the MVP: every type other modules need exists,
   today's generated code is untouched, and X1 and ci can start.

### Incremental delivery

1. core-1 merged → X1 (needs T1) regenerates models with `Describe`, `DATA`, `cache`, `reg()`.
2. core-2 (Phases 8–10): trait cutover, shims removed, tests over every model → bus-1 starts.
3. Each phase ends at a checkpoint that passes the gates; commit per phase, no AI attribution.

## Notes

- [P] = different files and no unfinished dependency.
- `[human]` tasks (T037, T052) are never done by an agent.
- Core never regenerates `src/models*`: the codegen gate must show no diff in both PRs.
