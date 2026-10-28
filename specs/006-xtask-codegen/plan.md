# Implementation Plan: xtask codegen (generated models, dispatch and family features)

**Branch**: `dev` (PR branch `006-xtask-codegen`, step X1 of the contract) | **Date**: 2026-10-27 |
**Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/006-xtask-codegen/spec.md`, the cross-module contract
`specs/ideas/module-interfaces.md` (cited as "contract §n"), constitution v1.3.0, and the
2026-10-27 architect decisions (SRS patch pending).

## Summary

Extend `cargo xtask codegen` so that every generated model type describes itself to the bus,
with no hand-written model code. Today it only gives `id`, `model_number` and `name`.

- **Input**: read the new table and map schema of contract §3. Tables gain `source`, `protocols`,
  and `access` + `area` on every row. The map gains per-protocol `p1`/`p2` lists, `default`,
  `also` and `not_mapped`.
- **Checks**: run every check of contract §4.3 before writing anything, and report all problems
  in one run, each naming the file.
- **Model types**: each holds a core `Cache` (Id + limits) and a crate-internal `DATA: ModelData`
  with name, number, protocols, the common-role registers (by e-Manual name), limit form,
  hardware-fault register or `None`, and the position scale. It implements `Describe` and
  `Actuator`. Every register constant carries access and area.
- **`ActuatorKind`**: a per-protocol lookup `new(ProtocolVersion, ModelNumber, fw, Id)` returning
  `SetupError`, plus `firmware_register`, `MODEL_NUMBER` and `Describe` by `match`.
- **Output**: formatted in memory and written only when changed. Generated crate tests check the
  lookup for every model number.

Torque and fault state are not generated. The bus holds them as one byte per slot (contract
§2.5).

## Technical Context

**Language/Version**: Rust stable, latest release, edition 2024. No MSRV (architect,
2026-10-27); dev uses rustc 1.97.1, rustfmt 1.9.0.

**Primary Dependencies**:
- xtask: `ron` 0.12.2 (with the `IMPLICIT_SOME` extension for the map) and `serde` 1 (derive).
  Both exist already; no new dependency or dev-dependency.
- `rustfmt` from the rustup toolchain, run as a subprocess on stdin.
- Generated code: no dependency, only core and protocol types (contract §2, §5).

**Storage**: files. Reads `control_tables/*.ron`; writes `src/models.rs` and `src/models/*.rs`
only.

**Testing**:
- `cargo test -p xtask`: unit tests, one fixture per check, end-to-end `run` on a temp root, and a
  real-tables test with an independent oracle.
- `cargo test --lib models::tests`: generated crate tests (lookup at every boundary, family off,
  cache round trip).
- Clippy with `-D warnings` for 7 family combinations, std and no_std.
- The CI codegen diff check, and `cargo llvm-cov` at ≥ 70 % (xtask counted, `src/models*`
  excluded).

**Target Platform**: xtask runs on the maintainer's host and on ubuntu CI. Generated code builds
for every crate target, including `thumbv7em-none-eabihf` with no std and no alloc.

**Project Type**: workspace dev tool (binary `xtask`) that emits committed library source.

**Performance Goals**: a full run takes a few seconds; rustfmt over 70 files takes 0.7 s today.
In generated code, the lookup is one `match` and model data is a `&'static` constant: no
allocation, no runtime search (NFR-4).

**Constraints**:
- Validate before writing (FR-003).
- Deterministic, formatted output (FR-002).
- Write nothing outside `src/models*` (FR-018).
- Generated code: no `dyn`, no allocation, no panic path, warning-free in every family
  combination (FR-014).
- Register constants and `DATA` crate-internal; model types public (FR-015).
- Every public item documented (FR-016).

**Scale/Scope**:
- 72 table files: 69 mapped, 63 model numbers, about 100 rows each, about 8 000 generated lines
  today.
- T2 adds MX(1.0) P1 tables, AX-18F and XW430-T200.
- Two lookup protocols. Five family features.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Check | Status |
|---|---|---|
| I. no_std and Heap-Free | Generated code holds only `const` data and a `Cache` by value. `&'static ModelData` comes from constant promotion (R-7). No `alloc` path. The no_std clippy line in every family combination proves it (SC-003). | PASS |
| II. Absent Over Failing | A family that is off removes its types and variants (compile error to name them). Its model numbers fail Setup with `UnsupportedModel { feature: Some(family) }`. A model on the wrong protocol fails at Init, not in the first Cycle (lookup is per protocol, R-8). | PASS |
| III. Nothing Hidden | Roles are matched by exact e-Manual name and never guessed. A missing hardware-fault register is `None`, not a default. No table default is generated as a limit; limits stay `None` until the bus reads EEPROM. | PASS |
| IV. e-Manual Is the Only Source | Codegen adds no facts. It carries the table's `source` into docs (Unverified/Verified line, R-15) and enforces `default: P2` for dual-protocol models. Y-series `Error Code` is not mapped until verified. | PASS |
| V. Committed, Data-Driven Codegen | `cargo xtask` emits committed, formatted files. No `build.rs`, no proc macro. Byte-identical reruns, CI diff check, validate-before-write, nothing touched on failure (R-2). A new model needs only data (SC-005). | PASS |
| VI. Typed, Static, Panic-Free API | Dispatch by `match`, no `dyn`. `ActuatorKind` is `#[non_exhaustive]`. Typed `SetupError`. No `unwrap`/`expect`/indexing in generated code. | PASS |
| VII. DX First | Every public generated item documented: name, number, firmware range per protocol, source table, verification state. Model types are public `match` targets; register constants stay internal (users get get/set methods on `ActuatorRef`). | PASS |
| Tech: `foo.rs` + `foo/`, edition 2024 | xtask split into `codegen.rs` + `codegen/` (R-13). Generated `models.rs` + `models/`. | PASS |
| Workflow: one module per PR | X1 touches `xtask/**` and generated `src/models*` only. No `lib.rs`, `Cargo.toml` or core file edits (if C2 holds). | PASS |
| Workflow: gates (fmt, clippy, test, no_std, codegen diff, ≥ 70 % coverage) | All in quickstart §1. xtask coverage target ≥ 70 % with `main.rs` the only untested file. | PASS |

Post-design re-check (after Phase 1): unchanged, all PASS. Two items are recorded below rather
than as gate failures:

- The module-level `allow` in generated code (Complexity Tracking).
- The `reg()` helper (deviation D1).

The xtask binary itself may `expect` in `workspace_root`. Principle VI covers the crate's API, not
the dev tool.

## Spec items superseded by module-interfaces.md

The contract resolves these spec conflicts (contract §8). The plan follows the contract; these are
not deviations from it.

| Spec | Contract | Plan |
|---|---|---|
| FR-009, US1 AS5: cache holds torque and fault state | conflict 1 | `Cache` = Id + limits. Torque state is the bus's `[TorqueState; N]`. US1 AS5 is tested on limits only. |
| FR-006/FR-007: `new(model_number, firmware)`, `Error::UnsupportedModel` | conflicts 4, 6 | `new(ProtocolVersion, ModelNumber, u8, Id) -> Result<_, SetupError>`, where `UnsupportedModel` gains `id` and `protocol` |
| Assumptions: access and range not generated | conflict 2 | access and area generated on every register; range/default/unit later |
| Edge case / Assumptions: MX on P1 is roadmap | conflict 3 | per-protocol lookup now; T2 adds the tables |
| FR-003 error classes | §4.3 | the 16 checks in [data-model.md §3](data-model.md#3-checks-all-before-any-write) |
| FR-008 "fault register, Firmware Version, Model Number exposed" | conflict 17 | crate-internal only (`ModelData`, `firmware_register`, `MODEL_NUMBER`) |

## Deviations from module-interfaces.md

**D1. Register constants use a private `reg()` helper, not `Register::new(addr, size, access, area)`.**
Contract §2.2/§7 keep a 2-argument `Register::new(addr, size)` shim in core-1 until core-2. But
§4.1/§7 have X1, which lands in between, emit a 4-argument `Register::new`. Rust has no
overloading, so that does not compile. X1 instead generates
`const fn reg(addr, size, access, area) -> Register { Register { addr, size, access, area } }`,
private to `models`. It relies only on the `pub(crate)` fields of §2.2, so it works with any
core-1 naming and needs no regeneration at core-2 (research R-5). The core plan
(`specs/004-core/plan.md` D1) raises the same conflict and asks X1 for exactly this. If core-1 provides a callable
4-argument constructor under another name, X1 may call it instead; the generated data does not
change. module-interfaces §2.2, §4.1 and §7 now use `reg()`.

**Clarifications the contract leaves open** (consistent with it; raised so core-1 plans for them):

- **C1.** Generated code implements `Describe` through the path `crate::actuator::Describe`. A
  trait in a private `mod sealed` inside `actuator.rs` is not nameable from `crate::models`, so
  core-1 needs a `pub(crate) use sealed::Describe;` (or equivalent). The core plan provides
  this.
- **C2.** Core-1 must drop or move the `ActuatorKind::new(u16, u8, Id)` tests in
  `src/actuator.rs`. X1 changes that signature and may not edit core files; the generated
  `src/models/tests.rs` replaces them. Fallback, if core-1 has kept them: the X1 PR changes only
  that `#[cfg(test)]` block, flagged as a cross-module touch. The core plan (D2) drops them in
  core-1.
- **C3.** With a family off, `firmware_register` returns `None` for that family's numbers. The
  lookup then still gives `UnsupportedModel { feature: Some(..) }`, without a wasted read
  (R-9). Contract §4.2 does not say.
- **C4.** `MODEL_NUMBER` takes its access and area from the tables, and codegen checks that all
  tables agree on them. The contract shows them as `R`/`EEPROM`.

**Note to ci (spec 007), not a deviation. Resolved.** The family-off half of SC-004 compiles only
with that family off. ci's `feature-test` job runs `cargo test --lib` with `--features
std,<family>` for each family, plus a `std` leg (specs/007-ci contracts). Those legs run with
`RUSTFLAGS="-D warnings"`, so generated tests must be warning-free in each leg (quickstart §3).

## Project Structure

### Documentation (this feature)

```text
specs/006-xtask-codegen/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── codegen-cli.md       # command, streams, exit status, write rules
│   └── generated-api.md     # signatures and cfg rules of src/models*
├── checklists/
└── tasks.md                 # /speckit-tasks
```

### Source Code (repository root)

```text
xtask/
├── Cargo.toml               # unchanged (ron 0.12.2, serde 1)
└── src/
    ├── main.rs              # args, print report/diagnostics, exit code
    ├── codegen.rs           # run(root) and generate(dir); Report, Diagnostic; module/file constants
    └── codegen/
        ├── schema.rs        # serde input types (contract §3), brace rewrite, ron options
        ├── names.rs         # type_ident, module_ident, const_ident, keyword/reserved checks
        ├── resolve.rs       # GenModel, Roles, LookupList; the 16 checks; warnings
        ├── render.rs        # src/models.rs and src/models/<module>.rs text
        ├── render_tests.rs  # src/models/tests.rs text
        ├── output.rs        # rustfmt via stdin; sync (write changed, delete stale)
        └── fixtures.rs      # #[cfg(test)] fixture builders, temp dir

src/                         # generated only, regenerated and committed in the X1 PR
├── models.rs                # reg(), MODEL_NUMBER, ActuatorKind, new, firmware_register, Describe, Actuator
└── models/
    ├── <module>.rs          # 69 today, one per mapped table
    └── tests.rs             # generated #[cfg(test)] lookup and data tests
```

**Structure Decision**: Rust 2018+ layout (`codegen.rs` + `codegen/`, no `mod.rs`). Every path is
inside the SRS module paths `xtask/**`, `src/models.rs` and `src/models/**`. `src/lib.rs` already
has `pub mod models;` and `pub use models::ActuatorKind;`, and X1 leaves it unchanged. The stale
`xtask/Cargo.lock` (the workspace root lock is the real one) is left alone.

### Delivery and PR split

The feature fits one PR: **X1**, as in contract §7.

| Step | Content | Needs |
|---|---|---|
| **X1** (this plan) | All of the above, plus the regenerated `src/models*` from T1's tables | PR #7 with `ProtocolVersion` (contract §5), core-1 (incl. C1, C2), T1 |
| X2 (optional, after core-2) | Drop the `id`/`model_number`/`name` overrides from generated `impl Actuator` blocks, now that core-2 provides them; regenerate | core-2 |

X1 is one module, and the diff is large but mechanical: the xtask code plus regenerated output.
T2 (tables) later commits regenerated output for its new tables with no xtask change.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| `#![allow(dead_code, unused_variables, clippy::approx_constant, reason = …)]` at the top of generated `src/models.rs` | Most register constants are unused by design. `firmware` is unused when no family is on. `approx_constant` is deny-by-default and fires on `type_info` angles such as `3.1415927`, which are table data (R-6, R-10). | `#[expect]` warns in combinations where the lint does not fire. Per-item allows make every diff noisy. Emitting `PI` special-cases one value and hides the table text. |
| Generated `reg()` helper instead of core's constructor (D1) | The contract's core-1 shim and X1's 4-argument `Register::new` cannot coexist. | A struct literal per constant makes rustfmt split each over six lines. Renaming core's constructor is core's decision. |
| Crate tests generated into `src/models/tests.rs` | FR-017 and SC-004 need lookup tests for every model number, and no hand-written file under `src/` other than the `lib.rs` line is allowed. | Integration tests under `tests/` cannot see crate-internal `new`, `DATA` or `firmware_register`. Hand-listing 63 numbers in core's file duplicates the map. |
