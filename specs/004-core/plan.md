# Implementation Plan: Core (typed values, Actuator trait, errors)

**Branch**: `dev` (planning only; `004-core-1` and `004-core-2` are cut from `main` at implement
time) | **Date**: 2026-10-27, updated 2026-10-28 (architect decisions) | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/004-core/spec.md`, SRS module `docs/srs/modules/core.md`,
the cross-module contract `specs/ideas/module-interfaces.md` (sections 1, 2, 7), constitution 1.3.0.

## Summary

Core gives every other module its shared types. It does no I/O. It adds:

- **`Position`** (radians, `f32`) and **`PositionScale`** (zero tick + slope from the table's
  `type_info`), with rounding to the nearest tick and typed errors for NaN, infinity and overflow.
- **`Limits`** and the per-actuator **`Cache`** (`Id`, limit mode and the limits read from EEPROM,
  nothing else), with the one limit check used by the typed and the raw path.
- **`LimitMode`** (`Bounded | Unbounded | NoPosition`, one byte in the `Cache` padding) and
  **`ModeSource`** / **`ModeValues`**: limits follow the operating mode (architect, 2026-10-28).
  Core maps each family's Operating Mode values, cited from the e-Manual (research R-17).
- **`TorqueState`**: one byte, `Off | On | Faulted`, with its transition rules. The bus stores it.
- **`ModelNumber`**, **`ModelData`**, **`LimitRegisters`**, **`Protocols`** and the sealed
  **`Describe`** trait: what the bus needs from a model, all crate-internal.
- **`Register`** with `access` and `area`, plus little-endian `encode`/`decode` that never truncate.
- Errors: the new `ValidationError` variants (with `NotPositionMode`) and a new **`Setup`**
  category (with `UnknownOperatingMode`). `UnsupportedModel` moves into it and gains `id` and
  `protocol` (conflict 19, accepted 2026-10-28).

Core ships as **two PRs**, as in module-interfaces §7:

| PR | Content | Needs |
|---|---|---|
| **core-1** | Every type above (the limit mode included: it is additive), `Describe` standalone, `Error::Setup`. Shims keep today's generated code building with no regeneration. Fixes the no-family `dead_code` error. | Protocol PR #7 with `ProtocolVersion` (§5): **merged** into `main` (f338e2a), root re-export included |
| **core-2** | `Actuator: Describe` with provided methods; removes the shims; tests over every compiled-in model. | xtask X1 |

Technical approach (research.md): manual rounding (core has no `f32::round`), `i64`
intermediates so no conversion can overflow, a slope precomputed in a `const fn`, and
`allow(dead_code)` (never `expect`) on items that later PRs start using.

## Technical Context

**Language/Version**: Rust latest stable (dev uses rustc 1.97.1), edition 2024. No MSRV
(architect, 2026-10-27).

**Primary Dependencies**: `thiserror` 2 (existing, `default-features = false`). No new
dependency. `libm` is not added (research R-5). Dev-dependency `proptest` comes with PR #7.

**Storage**: N/A. Per actuator: `Cache` = 16 bytes, `ActuatorKind` = 20 bytes, `TorqueState` =
1 byte (measured, research R-10). The 1-byte `LimitMode` fits the padding after `id`, so `Cache`
stays 16 bytes (research R-17).

**Testing**: `cargo test`, unit tests next to each file (`#[cfg(test)] mod tests`). Exhaustive tick
loops for the round trip, `proptest` for no-panic on any `f32`/`i32`, message tests per error.
core-2 adds one generic test over every variant the generated lookup can return.

**Target Platform**: host (Linux, macOS, Windows) and `thumbv7em-none-eabihf` (no heap, single
precision FPU).

**Project Type**: library crate (workspace with `xtask/`).

**Performance Goals**: `to_raw` and `to_position` are O(1): one `f32` multiply, one rounding, no
division per call. The register lookup is a `match` returning `&'static ModelData` (NFR-4).

**Constraints**: no panic on any input, no allocation, no `dyn` (FR-018). Only core paths are
edited (`src/lib.rs`, `src/id.rs`, `src/error.rs`, `src/actuator.rs`, `src/register.rs`,
`src/units.rs`, `src/units/**`). No edit to `src/models*`, `xtask/`, `control_tables/`,
`Cargo.toml`.

**Scale/Scope**: about 18 types and 16 new error variants. core-1 ≈ 1100 lines with tests,
core-2 ≈ 280 lines.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Check | Status |
|---|---|---|
| I. no_std and Heap-Free | Only `core` + `thiserror`. No `alloc`, no `Vec`, no `String`. Errors are `Copy`. Checked by the no_std clippy line. | PASS |
| II. Absent Over Failing | Core adds no protocol-specific method. `UnsupportedModel` still fails Setup, now under `SetupError`, keeping `model_number` and `feature` and adding `id` and `protocol` (conflict 19, accepted 2026-10-28). | PASS. The text of Principle II still writes `UnsupportedModel { model_number, feature }`: a wording amendment for the architect (T056). |
| III. Nothing Hidden | `TorqueState` starts `Off`. `Faulted` is off. No constructor fills limits from a table (FR-007). The limit check refuses when limits are unknown. An operating mode value not listed for the family has no mapping, and the bus fails Setup with `UnknownOperatingMode` (no fallback mode). In `Unbounded` mode the goal check is the register size only, as the firmware does; the docs say so. A fault on a `raw-registers` reply also sets `Faulted` (2026-10-28): core has no exception. | PASS, with one transitional exception (Complexity Tracking row 2) |
| IV. e-Manual Is the Only Source | Core holds no register addresses. Scale and registers come from the tables module. The one capability core encodes, the Operating Mode value lists per family, is cited value by value from the e-Manual pages in research R-17. Test values are checked against the pages cited in research R-7, R-9 and R-17. | PASS |
| V. Committed, Data-Driven Codegen | Core never edits `src/models*`. Each core PR leaves `cargo xtask codegen` output unchanged. | PASS |
| VI. Typed, Static, Panic-Free API | `Position` newtype, raw `i32` path kept. Static dispatch only (the mode mapping is a `match` in a `const fn`). Every conversion returns `Result`. Five error categories, with `Setup`, as constitution 1.3.0 lists them. | PASS |
| VII. DX First | `goal + Position::from_radians(0.1)` in one expression. Every public item documented (`missing_docs` from PR #7). No feature changes core behaviour. Breaking changes (`UnsupportedModel` moves) are called out in the PR. | PASS |
| Workflow: one module per PR | Core ships as core-1 and core-2. | **DEVIATION**: Complexity Tracking row 1. Needs the architect's approval (module-interfaces §9). |
| Workflow: gates (fmt, clippy, test, no_std, codegen diff, ≥ 70 % coverage) | Each PR runs all of them. core-1 also makes `--no-default-features` pass under `-D warnings` (today it fails, reproduced). | PASS |

**Post-design re-check (after Phase 1, and after the 2026-10-28 decisions)**: unchanged. No new
violation. The three deviations from module-interfaces.md below keep every gate green across the
core-1 → X1 → core-2 sequence.

## Deviations from module-interfaces.md

| # | Contract text | Problem | This plan |
|---|---|---|---|
| D1 | §2.2: core-1 keeps `Register::new(addr, size)` as a shim; §4.1: X1 emits `Register::new(64, 1, Access::RW, Area::Ram)`; core-2 removes the shim. | Rust has no overloading. The 2-argument shim and the 4-argument `new` cannot both exist, so X1's output would not compile against core-1. | core-1 keeps `new(addr, size)` as the shim, with an unconditional `allow(dead_code)`. **X1 builds registers without calling `Register::new`**, through its own generated `reg()` helper (xtask plan D1). core-2 removes the shim and adds **no** 4-argument `new`: nothing would call it. module-interfaces §2.2, §4.1 and §7 now say the same. Research R-2, R-4. |
| D2 | §7: core-1 "builds against today's generated code". Not stated: who owns tests of the generated lookup. | `src/actuator.rs` (core) holds tests that call `ActuatorKind::new(u16, u8, Id)` and match `Error::UnsupportedModel`. X1 changes that signature and cannot edit core files, so X1's PR would fail `cargo test`. | core-1 rewrites those tests to build models with `Model::new(id)` (kept by §4.1) and drops the lookup tests. Lookup tests move to X1 (generated test module or xtask). The `UnsupportedModel` message test moves to `src/error.rs` on `SetupError`. Research R-3. |
| D3 | §2.4b: `ModeSource::OperatingMode(Register)`; "X: 3 → Bounded; 4, 5 → Unbounded; 0, 1, 16 → NoPosition; PRO/PRO+/Y checked by core". | The register alone does not say which value list applies, and the lists conflict: value 5 is Current-based Position on X and MX(2.0), where the firmware ignores the limits (`Unbounded`), and on RH-P12-RN (feature `pro`), where "The Goal Position(596) can't exceed these values" (`Bounded`). One mapping cannot serve both. | `OperatingMode { register: Register, values: ModeValues }` with `ModeValues { X, Pro, Y, RhP12Rn }` and `const fn ModeValues::limit_mode(value) -> Option<LimitMode>`. X1 emits the variant per table. module-interfaces §2.4b carries this shape since its 2026-10-28 update; how X1 picks `values` per table (family alone does not single out RH-P12-RN) is open for the xtask plan (T041). Research R-17. |

Clarifications of the contract (no change in meaning):

- The mode mapping returns `Option<LimitMode>`; `None` means "not listed", and the bus builds
  `SetupError::UnknownOperatingMode { id, value }` (it holds the `Id`). Two helpers the bus needs
  are named here: `ModeSource::register() -> Option<Register>` (what Setup reads) and
  `const fn LimitMode::from_angle_limits(Limits) -> LimitMode` (AX, MX P1). Research R-17.
- `Cache::check_goal` checks the mode first: a `NoPosition` actuator gives `NotPositionMode` even
  when its limits are unknown.

- `mod sealed` in §2.6 is `pub(crate) mod sealed`, so `crate::models` can name
  `actuator::sealed::Describe`. `Describe` stays unnameable outside the crate.
- `Actuator::position_limits_raw` and `position_limits` are **provided** methods (bodies use
  `Describe`). Otherwise X1's `impl Actuator for … {}` would not compile after core-2.
- The import paths X1 and bus use are listed in [contracts/core-api.md](contracts/core-api.md#paths-for-xtask-and-bus).

Open risks and questions for the architect (not deviations):

- A user's generic code bounded on `A: Actuator` can call the supertrait method `cache_mut()` and
  overwrite one actuator's cache with another's (confirmed in a prototype). The fix is a
  crate-private token argument on `cache_mut`, which changes X1's emitted impl. Research R-12.
- `position_limits_raw()` / `position_limits()` return the EEPROM limits whatever the mode. In
  `Unbounded` and `NoPosition` the goal check does not apply them. Returning `None` there would
  change the contract's meaning ("`None` until Setup read the limits"). Research R-17.
- MX on P1, multi-turn: Goal Position is 2 bytes with a range of −28 672 to 28 672, and §2.2
  encodes 2 bytes unsigned, so negative multi-turn goals give `DoesNotFit`. How the firmware
  reads values above 28 672 is not on the e-Manual page. For tables T2 (research R-17).

## Project Structure

### Documentation (this feature)

```text
specs/004-core/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── core-api.md
├── checklists/
│   └── requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
src/
├── lib.rs               # core-1: + mod units; + pub use of ModelNumber, TorqueState, Position,
│                        #   PositionScale, SetupError. ProtocolVersion is already re-exported
│                        #   by PR #7 (merged): only checked. core-2: no change expected.
├── id.rs                # unchanged
├── error.rs             # core-1: ValidationError + 10 variants, SetupError (6 variants),
│                        #   Error::Setup, Error::UnsupportedModel kept #[doc(hidden)].
│                        #   core-2: Error::UnsupportedModel removed.
├── actuator.rs          # core-1: ModelNumber, TorqueState, Cache (with LimitMode), LimitMode,
│                        #   ModeSource, ModeValues, ModelData, LimitRegisters, Protocols,
│                        #   pub(crate) mod sealed { Describe }; Actuator unchanged;
│                        #   tests rewritten (D2). core-2: Actuator: Describe, provided methods,
│                        #   generic tests over every compiled-in model.
├── register.rs          # core-1: Access, Area, access/area fields, encode/decode,
│                        #   2-arg new shim. core-2: shim removed, no new
│                        #   constructor (X1's reg() builds registers, D1).
├── units.rs             # core-1 (new): module docs, mod position; re-exports
└── units/
    └── position.rs      # core-1 (new): Position, PositionScale, Limits
```

Not touched: `src/models.rs`, `src/models/**`, `src/protocol*`, `src/transport*`, `xtask/**`,
`control_tables/**`, `Cargo.toml`.

**Structure Decision**: Rust 2018+ layout (`units.rs` + `units/`, no `mod.rs`). Files and items
follow module-interfaces §2, with D1–D3. `Limits` sits in `units/position.rs`; `Cache`,
`LimitMode`, `ModeSource`, `ModeValues` and `TorqueState` in `actuator.rs`.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| Core ships as two PRs (core-1, core-2), against "one module per PR". | The trait cutover (`Actuator: Describe`) needs generated code that implements `Describe`. That code comes from X1, which itself needs core-1's types. | One core PR after X1: X1 cannot build without the types. One core PR with a regeneration: core may not edit xtask, and today's xtask cannot emit `Describe`. |
| Transitional shims in core-1: `#[doc(hidden)] Error::UnsupportedModel`, 2-arg `#[doc(hidden)] Register::new` filling `access`/`area` with placeholders (`R`, `Eeprom`) under an unconditional `#[allow(dead_code, reason = "unused once X1 regenerates with reg(); removed in core-2")]` (unused in every build after X1), `allow(dead_code)` on not-yet-used items: `Access` (`RW` never built) and `Area` (`Ram` never built) until X1, `Describe` until X1, the D3 items until X1 or bus-1 (full list in research R-4). | Today's generated code must keep compiling with no regeneration (§7). The placeholders are never read: nothing reads `access`/`area` before X1 regenerates every register, and bus lands after core-2. | Real attributes need the tables (T1) and X1. Leaving the fields out until core-2 breaks X1 (it must emit them). |
