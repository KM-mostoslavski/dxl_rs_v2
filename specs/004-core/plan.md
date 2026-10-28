# Implementation Plan: Core (typed values, Actuator trait, errors)

**Branch**: `dev` (planning only; `004-core-1` and `004-core-2` are cut from `main` at implement
time) | **Date**: 2026-10-27 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/004-core/spec.md`, SRS module `docs/srs/modules/core.md`,
the cross-module contract `specs/ideas/module-interfaces.md` (sections 1, 2, 7), constitution 1.3.0.

## Summary

Core gives every other module its shared types. It does no I/O. It adds:

- **`Position`** (radians, `f32`) and **`PositionScale`** (zero tick + slope from the table's
  `type_info`), with rounding to the nearest tick and typed errors for NaN, infinity and overflow.
- **`Limits`** and the per-actuator **`Cache`** (`Id` + limits read from EEPROM, nothing else),
  with the one limit check used by the typed and the raw path.
- **`TorqueState`**: one byte, `Off | On | Faulted`, with its transition rules. The bus stores it.
- **`ModelNumber`**, **`ModelData`**, **`LimitRegisters`**, **`Protocols`** and the sealed
  **`Describe`** trait: what the bus needs from a model, all crate-internal.
- **`Register`** with `access` and `area`, plus little-endian `encode`/`decode` that never truncate.
- Errors: the new `ValidationError` variants and a new **`Setup`** category. `UnsupportedModel`
  moves into it.

Core ships as **two PRs**, as in module-interfaces §7:

| PR | Content | Needs |
|---|---|---|
| **core-1** | Every type above, `Describe` standalone, `Error::Setup`. Shims keep today's generated code building with no regeneration. Fixes the no-family `dead_code` error. | Protocol PR #7 with `ProtocolVersion` (§5) |
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
1 byte (measured, research R-10).

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

**Scale/Scope**: about 10 types and 15 error variants. core-1 ≈ 900 lines with tests,
core-2 ≈ 250 lines.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Check | Status |
|---|---|---|
| I. no_std and Heap-Free | Only `core` + `thiserror`. No `alloc`, no `Vec`, no `String`. Errors are `Copy`. Checked by the no_std clippy line. | PASS |
| II. Absent Over Failing | Core adds no protocol-specific method. `UnsupportedModel` still fails Setup, now under `SetupError`, keeping `model_number` and `feature` and adding `id` and `protocol`. | PASS |
| III. Nothing Hidden | `TorqueState` starts `Off`. `Faulted` is off. No constructor fills limits from a table (FR-007). The limit check refuses when limits are unknown. | PASS, with one transitional exception (Complexity Tracking row 2) |
| IV. e-Manual Is the Only Source | Core holds no table facts. Scale and registers come from the tables module. Test values are checked against the e-Manual pages cited in research R-7 and R-9. | PASS |
| V. Committed, Data-Driven Codegen | Core never edits `src/models*`. Each core PR leaves `cargo xtask codegen` output unchanged. | PASS |
| VI. Typed, Static, Panic-Free API | `Position` newtype, raw `i32` path kept. Static dispatch only. Every conversion returns `Result`. Error categories become five (adds `Setup`). | PASS, with a wording flag: VI says "transport, protocol, status and validation"; `Setup` was added by the architect on 2026-10-27 (SRS FR-6.4 patch). Constitution 1.3.0 (2026-10-28) lists it. |
| VII. DX First | `goal + Position::from_radians(0.1)` in one expression. Every public item documented (`missing_docs` from PR #7). No feature changes core behaviour. Breaking changes (`UnsupportedModel` moves) are called out in the PR. | PASS |
| Workflow: one module per PR | Core ships as core-1 and core-2. | **DEVIATION**: Complexity Tracking row 1. Needs the architect's approval (module-interfaces §9). |
| Workflow: gates (fmt, clippy, test, no_std, codegen diff, ≥ 70 % coverage) | Each PR runs all of them. core-1 also makes `--no-default-features` pass under `-D warnings` (today it fails, reproduced). | PASS |

**Post-design re-check (after Phase 1)**: unchanged. No new violation. The two deviations from
module-interfaces.md below keep every gate green across the core-1 → X1 → core-2 sequence.

## Deviations from module-interfaces.md

| # | Contract text | Problem | This plan |
|---|---|---|---|
| D1 | §2.2: core-1 keeps `Register::new(addr, size)` as a shim; §4.1: X1 emits `Register::new(64, 1, Access::RW, Area::Ram)`; core-2 removes the shim. | Rust has no overloading. The 2-argument shim and the 4-argument `new` cannot both exist, so X1's output would not compile against core-1. | core-1 keeps `new(addr, size)` as the shim, with an unconditional `allow(dead_code)`. **X1 builds registers without calling `Register::new`**, through its own generated `reg()` helper (xtask plan D1). core-2 removes the shim and adds **no** 4-argument `new`: nothing would call it. module-interfaces §2.2, §4.1 and §7 now say the same. Research R-2, R-4. |
| D2 | §7: core-1 "builds against today's generated code". Not stated: who owns tests of the generated lookup. | `src/actuator.rs` (core) holds tests that call `ActuatorKind::new(u16, u8, Id)` and match `Error::UnsupportedModel`. X1 changes that signature and cannot edit core files, so X1's PR would fail `cargo test`. | core-1 rewrites those tests to build models with `Model::new(id)` (kept by §4.1) and drops the lookup tests. Lookup tests move to X1 (generated test module or xtask). The `UnsupportedModel` message test moves to `src/error.rs` on `SetupError`. Research R-3. |

Clarifications of the contract (no change in meaning):

- `mod sealed` in §2.6 is `pub(crate) mod sealed`, so `crate::models` can name
  `actuator::sealed::Describe`. `Describe` stays unnameable outside the crate.
- `Actuator::position_limits_raw` and `position_limits` are **provided** methods (bodies use
  `Describe`). Otherwise X1's `impl Actuator for … {}` would not compile after core-2.
- The import paths X1 and bus use are listed in [contracts/core-api.md](contracts/core-api.md#paths-for-xtask-and-bus).

Open risk for the architect (not a deviation): a user's generic code bounded on `A: Actuator`
can call the supertrait method `cache_mut()` and overwrite one actuator's cache with another's
(confirmed in a prototype). The fix is a crate-private token argument on `cache_mut`, which
changes X1's emitted impl. Research R-12.

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
│                        #   PositionScale, SetupError; + pub use protocol::ProtocolVersion if
│                        #   PR #7 has not added it. core-2: no change expected.
├── id.rs                # unchanged
├── error.rs             # core-1: ValidationError + 9 variants, SetupError (5 variants),
│                        #   Error::Setup, Error::UnsupportedModel kept #[doc(hidden)].
│                        #   core-2: Error::UnsupportedModel removed.
├── actuator.rs          # core-1: ModelNumber, TorqueState, Cache, ModelData, LimitRegisters,
│                        #   Protocols, pub(crate) mod sealed { Describe }; Actuator unchanged;
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
follow module-interfaces §2 exactly. `Limits` sits in `units/position.rs`; `Cache` and
`TorqueState` in `actuator.rs`.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| Core ships as two PRs (core-1, core-2), against "one module per PR". | The trait cutover (`Actuator: Describe`) needs generated code that implements `Describe`. That code comes from X1, which itself needs core-1's types. | One core PR after X1: X1 cannot build without the types. One core PR with a regeneration: core may not edit xtask, and today's xtask cannot emit `Describe`. |
| Transitional shims in core-1: `#[doc(hidden)] Error::UnsupportedModel`, 2-arg `#[doc(hidden)] Register::new` filling `access`/`area` with placeholders (`R`, `Eeprom`) under an unconditional `#[allow(dead_code, reason = "unused once X1 regenerates with reg(); removed in core-2")]` (unused in every build after X1), `allow(dead_code)` on not-yet-used items. | Today's generated code must keep compiling with no regeneration (§7). The placeholders are never read: nothing reads `access`/`area` before X1 regenerates every register, and bus lands after core-2. | Real attributes need the tables (T1) and X1. Leaving the fields out until core-2 breaks X1 (it must emit them). |
| A fifth error category (`Setup`) beside the four of constitution VI. | Architect decision 2026-10-27 (spec Clarifications, SRS FR-6.4 patch). | Folding setup failures into `Validation` mixes "rejected before sending" with Init outcomes; the architect chose option B. |
