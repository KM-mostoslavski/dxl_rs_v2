# Implementation Plan: CI (hardened quality gates)

**Branch**: `dev` (spec directory `007-ci`; the PR branch is cut from `main` at implement time) |
**Date**: 2026-10-27 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/007-ci/spec.md`, SRS module `docs/srs/modules/ci.md`,
shared contract `specs/ideas/module-interfaces.md`, constitution v1.3.0
(`.specify/memory/constitution.md`). The contract and the constitution are on `dev` only; they
reach `main` with the dev merge, not with the ci PR, so these docs cite them by repo path.

**Updated**: 2026-10-28 (PR #7 merged; one missing-docs mechanism, no `Cargo.toml` change; table
check gets FR-016; `ov-aiignore.yml` is a human precondition).

## Summary

Close the gaps between what the SRS and constitution promise and what `ci.yml` proves. All
changes are in `.github/workflows/ci.yml`. `Cargo.toml` is not changed.

- **no_std, heap-free (P1)**: the no_std job builds the library for `thumbv7em-none-eabihf`
  with no features, `alloc` alone, every heap-free feature, and every no_std feature. It keeps
  today's clippy run.
- **Feature sets (P1)**: a job reads the features from `cargo metadata` and makes a matrix: on
  the host, no features, each feature alone, the defaults, all features; on the target, each
  feature alone that does not need `std`. Each leg is a named check. On the host, crate tests
  also run with `std` alone and with `std` plus each heap-free feature, so tests that compile
  only with a family off run. One aggregate check, `feature sets`, is the stable name to mark
  required. A new feature joins with no workflow edit.
- **Tests and clippy with all features (P1)**: two more steps in the Linux job.
- **Coverage figure (P2)**: written to the run summary before the 70 % gate, pass or fail. No
  write permission.
- **Docs (P2)**: `#![warn(missing_docs)]` in `src/lib.rs` (core's line, on `main` since PR #7)
  fails every leg under `-D warnings`; ci adds no second lint (R-5). A docs job fails on any
  rustdoc warning. A compile-fail job checks error codes on a pinned toolchain (stable 1.97.1
  with `RUSTC_BOOTSTRAP=1`), with all features, defaults, and `std` off, so feature-off
  examples run too.
- **Table check (FR-016)**: runs the tables module's `check_tables.py` and its unit tests.
- **Desktop OSes (P2)**: build and crate tests on Windows and macOS; Linux is the existing job.
- **Hygiene**: read-only permissions, cancel stale PR runs, no path filters, descriptive check
  names, `persist-credentials: false`, a 20-minute time-out per job.

## Technical Context

**Language/Version**: GitHub Actions workflow YAML; `bash` and `jq` in steps. Rust edition 2024,
latest stable on runners; one pinned toolchain, 1.97.1, for the compile-fail check. No MSRV
(FR-010).

**Primary Dependencies**: `actions/checkout@v4`, `dtolnay/rust-toolchain` (`@stable`, and
`@master` with `toolchain: 1.97.1`), `Swatinem/rust-cache@v2`,
`taiki-e/install-action@cargo-llvm-cov` (all already in use), `jq` (preinstalled on
`ubuntu-latest`). No new crate dependency, no `cargo-hack` (research R-1).

**Storage**: Actions cache only (`rust-cache`; matrix legs save on `main` only).

**Testing**: Seeded faults on throwaway branches, one per promise
([quickstart.md § 2](quickstart.md#2-seeded-faults-sc-001)). Local reproduction of every gate
([quickstart.md § 1](quickstart.md#1-run-the-gates-locally)). The feature-set rule and every
command were prototyped on scratch copies of `dev` and PR #7, and re-run on a scratch copy of
`main` on 2026-10-28 with the seeded faults (research R-16).

**Target Platform**: GitHub-hosted `ubuntu-latest`, `windows-latest`, `macos-latest`; cross
target `thumbv7em-none-eabihf`.

**Project Type**: CI configuration for a library crate (workspace with `xtask/`).

**Performance Goals**: full PR run ≤ 15 min with a warm cache (SC-003). 36 jobs today; legs are
single-crate builds or single-crate lib tests.

**Constraints**: only `.github/workflows/**` (never `ov-*.yml`). `Cargo.toml` is in `ci`'s paths,
but this feature changes no line of it (FR-015). No secrets, read-only token, GitHub-hosted
runners. No path filters. No `continue-on-error` on a blocking check. No gate removed or weakened
(FR-001).

**Scale/Scope**: one workflow file (4 jobs kept, 8 added), no `Cargo.toml` change. 11 host,
7 target and 7 test feature sets today, growing by up to three per new feature (13, 9 and 9 once
bus-5 declares `partial-reads` and `raw-registers`).

No NEEDS CLARIFICATION remains. Every unknown is resolved in [research.md](research.md).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Check | Status |
|---|---|---|
| I. no_std and Heap-Free | CI now builds (not only lints) on `thumbv7em-none-eabihf` with default features off, and without `alloc`; any `alloc::` use outside the `alloc` feature fails (prototyped, R-3). Library only; firmware link is the FR-003 TODO. | PASS |
| II. Absent Over Failing | Compile-fail examples are checked by error code on a pinned toolchain (R-7). Every single family builds alone, so a compiled-out family cannot become a build error. | PASS |
| III. Nothing Hidden | No runtime code. The coverage figure is shown pass or fail; no check is skipped by path or OS in the workflow. | PASS |
| IV. e-Manual Is the Only Source | No Dynamixel facts in this module. | N/A |
| V. Committed, Data-Driven Codegen | The codegen diff job is kept unchanged. | PASS |
| VI. Typed, Static, Panic-Free API | No Rust code in this module. | N/A |
| VII. Developer Experience First | `missing_docs` enforced in every feature set; doc warnings fail; every check name says what it proves; each check is reproducible locally with one command (contract § "Reproduce a check locally"). | PASS |
| Technical Constraints | Edition 2024, workspace with `xtask`, features unchanged. | PASS |
| Workflow: quality gates | Every constitution gate keeps running at the same strength (FR-001). New gates are added on top. Adding them to the constitution's list is the architect's call. | PASS |
| Workflow: one module per PR | Touches only `ci`'s paths: `.github/workflows/ci.yml` (module-interfaces.md § 1). No `Cargo.toml` line. | PASS |
| Workflow: no AI attribution | Applies to the PR, not the plan. | PASS (at PR time) |
| Governance: SRS and ARCHITECTURE.md agree | NFR-1, NFR-6, FR-7.3 met. The 2026-10-27 SRS patch does not change CI, nor do the 2026-10-28 decisions (mode-aware limits, `raw-registers` replies observed, per-slot `partial-reads`): bus-5's tests run with `partial-reads,raw-registers` through the all-features run and their own test legs. | PASS |

Post-design re-check (after Phase 1): unchanged, all PASS. The design needs no exception.

One interpretation is recorded, not a violation:

- FR-002 names "every model family plus `mock`". The plan builds "every feature that needs
  neither `std` nor `alloc`", which is that set today and grows by itself (R-3). Stronger, never
  weaker.

## Project Structure

### Documentation (this feature)

```text
specs/007-ci/
├── plan.md              # this file
├── research.md          # Phase 0
├── data-model.md        # Phase 1
├── quickstart.md        # Phase 1
├── contracts/
│   ├── ci-checks.md     # check names, commands, triggers, permissions, Cargo.toml lines
│   └── feature-sets.md  # derivation rule and JSON shape of the feature-set list
├── checklists/
│   └── requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
.github/workflows/
├── ci.yml               # MODIFIED: permissions, concurrency, persist-credentials, time-outs;
│                        # check (renamed "(Linux)", + all-features tests); no-std (+ 4 builds);
│                        # coverage (+ run summary); codegen (unchanged);
│                        # NEW: feature-sets, feature-set (matrix), feature-test (matrix),
│                        # features (aggregate), docs, compile-fail, control-tables,
│                        # os (Windows, macOS)
└── ov-aiignore.yml      # NOT touched (no-go); the human commits it to main first
Cargo.toml               # NOT changed (FR-015; missing_docs stays in src/lib.rs, R-5)
```

Not touched: `src/**`, `xtask/**`, `control_tables/**`, `.cargo/config.toml`,
`.github/pull_request_template.md`, `.github/ov-ruleset-main.json`, `CLAUDE.md`, the
constitution, `docs/**`.

**Structure Decision**: one workflow file, `ci.yml`, so one `concurrency` group covers every
check and the PR shows them under one `CI /` prefix. The `jq` program lives inline in the
`feature-sets` step: a separate script file would sit outside `ci`'s paths
(`.github/scripts/`) or in the workflows folder, where GitHub expects only workflows.

## PR split

One PR. The change is one file, well inside one review.

Delivery order (module-interfaces.md § 7): the ci PR merges **after core-1**. PR #7 (step P) is
merged (2026-10-28).

- PR #7 documented the two `UnsupportedModel` fields and added `#![warn(missing_docs)]` to
  `src/lib.rs`, so the missing-docs gate is green on `main`.
- core-1 removes the `Register::new` "never used" warning that makes every no-family build red
  under `-D warnings` (conflict 14; confirmed on scratch copies of `dev`, PR #7 and `main`,
  R-16).

Precondition owned by the human: `.aiignore` and `.github/workflows/ov-aiignore.yml` are
committed to `main` before the ci PR opens (spec Assumptions). On 2026-10-28 both are untracked
on `dev`. The ci PR never creates or edits them.

The rest (coverage summary, docs, compile-fail, OSes, permissions) passes on `main` today.
The `control tables: schema check` step runs the tables module's scripts, which T1 adds
(spec 005). Land ci after T1, or guard the step with `if [ -f control_tables/check_tables.py ]`
until T1 merges (the default, R-17). Remove the guard in the first ci change after T1.

If core-1 slips, an early split is possible: **ci-1** with those parts, then **ci-2** with the
feature sets and the no_std builds. That split needs the architect's approval under "one module
per PR"; the default is one PR.

Before opening the PR, run [quickstart.md § 1](quickstart.md#1-run-the-gates-locally) on the
branch. If a Windows or macOS leg fails on its first run (R-9), the fix lands in the owning module
first. The ci PR does not skip the test or mark the job non-blocking.

## Deviations from module-interfaces.md

One.

- § 1 "Known `Cargo.toml` touches" lists `[lints.rust] missing_docs` (ci, its own table). This
  plan adds no `[lints]` table: PR #7 put `#![warn(missing_docs)]` in `src/lib.rs`, core's
  plan relies on that line, and one lint needs one switch (R-5). The line in § 1 should be
  dropped. `ci` keeps ownership of `[lints]` for later use.
- File ownership otherwise as § 1: only `.github/workflows/**` (not `ov-*.yml`). No `Cargo.toml`
  line, no feature or dependency line.
- Compile-fail cases stay rustdoc examples with error codes, checked on a pinned toolchain, as
  conflict 13 says. The pin is stable 1.97.1 with `RUSTC_BOOTSTRAP=1` rather than a nightly
  (R-7); the contract does not fix which pinned toolchain.
- Delivery after P (#7, merged) and core-1, as § 7 says.

## Follow-ups for the architect (outside this module)

- Before the ci PR: commit `.aiignore` and `.github/workflows/ov-aiignore.yml` to `main`.
- Mark the required checks listed in [contracts/ci-checks.md](contracts/ci-checks.md) in
  `.github/ov-ruleset-main.json` (not in the repository yet).
- Add the new gates to the constitution's gate list (`/speckit-constitution`), the PR template
  and `CLAUDE.md`.
- Drop `[lints.rust] missing_docs` from the "Known `Cargo.toml` touches" line of
  module-interfaces.md § 1 (see Deviations).
- Keep `#![warn(missing_docs)]` in `src/lib.rs` (core): the missing-docs gate relies on it (R-5).
- Optional: pin actions by commit SHA (R-11).
- Doc examples that use std-only items need `#[cfg_attr(feature = "std", doc = …)]` so the
  std-off compile-fail pass compiles (R-7). This binds every module that writes doc examples.

## Complexity Tracking

No constitution violation to justify.

| Choice | Why needed | Simpler alternative rejected because |
|---|---|---|
| Dynamic matrix from `cargo metadata` + `jq`, plus an aggregate check | A new feature must be built with no workflow edit (FR-004), and a red set must be named on the PR page (FR-014). | A hand-written list goes stale; `cargo hack` needs a hand-kept exclude list for the target; a loop hides which set failed (R-1). |
| `RUSTC_BOOTSTRAP=1` on the doc-test steps of a pinned stable | Stable rustdoc ignores error codes in `compile_fail` examples (verified). | A dated nightly works too, but adds a second compiler family; `trybuild` adds a dependency (conflict 13) (R-7). |
