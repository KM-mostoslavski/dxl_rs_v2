---

description: "Task list for the ci module (hardened quality gates)"
---

# Tasks: CI (hardened quality gates)

**Input**: Design documents from `specs/007-ci/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/ci-checks.md,
contracts/feature-sets.md, quickstart.md; shared contract `specs/ideas/module-interfaces.md`
(§ 1 file ownership, § 7 delivery order; on `dev`, it reaches `main` with the dev merge).

**Tests**: Requested by the spec (SC-001: a seeded fault turns the matching check red). For CI a
"test" is a seeded fault on a throwaway branch, run locally with the job's exact command before
the job exists (to show the gap: today's workflow stays green) and again after (the new command
turns red and names the item). Each story's test tasks come before its job tasks and name the
acceptance scenarios they cover. The same faults run once more on GitHub as draft PRs in the
Polish phase (T040).

**Organization**: Tasks are grouped by user story so each story can be implemented and verified
on its own.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files or a separate throwaway worktree, no dependency
  on an unfinished task)
- **[Story]**: US1 = no_std heap-free build, US2 = every feature set builds, US3 = coverage figure
  on the PR, US4 = docs complete and compile-fail codes, US5 = Windows and macOS. US6 (MSRV) is
  deferred (FR-010) and has no tasks.
- **[human]**: the task touches a path in `.aiignore` or is the architect's call; an agent never
  does it.

## Rules for every task

- Only these files may change on the `007-ci` branch: `.github/workflows/ci.yml` and
  `specs/007-ci/**`. Never touch `Cargo.toml` (FR-015: the missing-docs lint stays PR #7's
  `#![warn(missing_docs)]` in `src/lib.rs`, R-5), `src/**`, `xtask/**`, `control_tables/**`,
  `.cargo/config.toml`, `.github/pull_request_template.md`, `CLAUDE.md`, `docs/**`, or any
  `.aiignore` path (`.aiignore`, `.claude/settings.json`, `.github/workflows/ov-*.yml`,
  `.github/ov-ruleset-main.json`, `.specify/extensions/ov/**`, `docs/srs/SRS.md`).
- No gate is removed or weakened: every command of today's `check`, `no-std`, `coverage` and
  `codegen` jobs keeps running with the same flags (FR-001).
- Every job: `runs-on` a GitHub-hosted image, `timeout-minutes: 20`, `actions/checkout@v4` with
  `persist-credentials: false`. No `paths`/`paths-ignore`, no `continue-on-error`, no
  `pull_request_target`, no self-hosted runner, no secret (FR-011, FR-013).
- Values from `matrix` or `needs.*.outputs` reach a `run:` script only through `env:`, never as
  `${{ … }}` inside the script text (R-11).
- Toolchain: latest stable (`dtolnay/rust-toolchain@stable`) everywhere except the compile-fail
  job, pinned to `COMPILE_FAIL_TOOLCHAIN: "1.97.1"`. No `rust-version`, no `rust-toolchain.toml`
  (FR-010, R-15).
- Seeded faults live only on throwaway branches `ci-fault/<id>` in their own worktree
  (`git worktree add ../dxl_rs_v2-fault-<id> -b ci-fault/<id> 007-ci`). They are never merged;
  T040 deletes them.
- Commits and PRs carry no co-authorship or AI attribution lines. PRs are opened from the
  `Ovsiankina` account, never `KM-mostoslavski`.

---

## Phase 1: Setup

**Purpose**: Prerequisites merged, branch cut, baseline recorded

- [X] T001 Check the delivery prerequisites of module-interfaces.md § 7 on `origin/main` after `git fetch origin`. PR #7 (protocol, step P) is merged (f338e2a) and needs no check. (a) core-1 is merged: in a clean worktree of `origin/main`, `RUSTFLAGS="-D warnings" cargo build --lib --no-default-features` and `RUSTFLAGS="-D warnings" cargo build --workspace` both pass (no `Register::new` "never used", no undocumented `SetupError` field), and `git show origin/main:src/lib.rs | grep -n 'warn(missing_docs)'` finds the attribute the missing-docs gate relies on (R-5); (b) human precondition: `git cat-file -e origin/main:.aiignore` and `git cat-file -e origin/main:.github/workflows/ov-aiignore.yml` both succeed; (c) tables T1 (optional): `git cat-file -e origin/main:control_tables/check_tables.py`. If (a) fails, stop and report to the architect (see PR map, "early split"). If (b) fails, stop and ask the human to commit both files (agents never write them). Record the result of (c) for T036
- [X] T002 Create the PR branch: run `git status` and `git branch --no-merged origin/main` first and report unmerged or dirty work; then `git worktree add ../dxl_rs_v2-007-ci -b 007-ci origin/main` and, inside it, `git checkout dev -- specs/007-ci` (brings spec.md, plan.md, research.md, data-model.md, contracts/, quickstart.md, checklists/ and this tasks.md, as `specs/002-transport/` travelled with PR #6). `specs/ideas/module-interfaces.md` and `.specify/memory/constitution.md` stay out of this PR; they reach `main` with the dev merge, and the docs cite them by repo path. All later tasks run in this worktree
- [X] T003 Record the baseline in the worktree: install `rustup target add thumbv7em-none-eabihf`, `rustup toolchain install 1.97.1`, `cargo-llvm-cov`, `jq`, `libudev-dev`; run every command of today's `.github/workflows/ci.yml` (`cargo fmt --check`; `cargo clippy --workspace --all-targets` with and without `--features mock`; `cargo test --workspace`; `cargo test --features mock --doc`; the no_std clippy with `ax,mx,x-series,y-series,pro,mock`; `cargo llvm-cov --workspace --ignore-filename-regex 'src/models(\.rs|/)' --fail-under-lines 70`; `cargo xtask codegen` + `git diff --exit-code -- src/models.rs src/models`) under `RUSTFLAGS="-D warnings"`; all must pass. Save the list of these commands and the coverage percentage in `$SCRATCH/baseline.txt` (outside the repo) for the FR-001 audit in T037

---

## Phase 2: Foundational (blocking prerequisites)

**Purpose**: Workflow hygiene shared by every job, and the `feature-sets` list job that US1, US2
and US4 read

**⚠️ CRITICAL**: US1, US2 and the compile-fail part of US4 need T008. US3, US5 and the docs part
of US4 need only T004–T005.

- [X] T004 In `.github/workflows/ci.yml`, top level (contract ci-checks.md § Workflow; FR-011, FR-012, FR-013; R-11, R-12): add `permissions:\n  contents: read`; add `concurrency:` with `group: ci-${{ github.event.pull_request.number || github.ref }}` and `cancel-in-progress: ${{ github.event_name == 'pull_request' }}`; add `COMPILE_FAIL_TOOLCHAIN: "1.97.1"` to the workflow `env` next to `CARGO_TERM_COLOR` and `RUSTFLAGS: -D warnings`. Leave `on:` exactly as is (`pull_request` with no filter, `push` to `main`)
- [X] T005 In `.github/workflows/ci.yml`, the four kept jobs `check`, `no-std`, `coverage`, `codegen`: add `timeout-minutes: 20` and change every `- uses: actions/checkout@v4` to carry `with:\n  persist-credentials: false`. Change no `run:` command and no job name yet (FR-001; R-11, R-14)
- [X] T006 Test for T008 (contract feature-sets.md § Rule, § Guarantees; US2 AS3; SC-005): write the workflow version of the `jq` program to `$SCRATCH/feature-sets.jq` (outside the repo). Start from the reference program in contracts/feature-sets.md and extend it to the § Output shape: one `matrix` array (host legs first: `no features`, `<f> alone` for every declared feature sorted by name, `defaults`, `all features`; then target legs `<f> alone` for every `f` with `not needs_std`), each `Leg` with `name`, `where` (`host` | `thumbv7em`), `args` (target legs end with `--target thumbv7em-none-eabihf`), `needs_std` (closure of the set contains `std`; `defaults` uses the closure of `default`, `all features` is true if any feature needs `std`, `no features` is false); plus `tests`, `heap_free`, `no_std`. Run `cargo metadata --no-deps --format-version 1 | jq -c --arg pkg dxl_rs_v2 -f $SCRATCH/feature-sets.jq` and check it equals contract § "Expected output" (11 host legs, 7 target legs, 7 test legs, `heap_free` = `ax,mock,mx,pro,x-series,y-series`, `no_std` = `alloc,ax,mock,mx,pro,x-series,y-series`, `needs_std` true only for `std alone`, `defaults`, `all features`). Then on a scratch copy of `Cargo.toml` (in `$SCRATCH`, never in the repo) with `serial-extra = ["std"]` and `partial-reads = []` added, check 13 host legs, 8 target legs (`partial-reads` in, `serial-extra` out), 8 test legs. Finally check `echo '' | jq -f $SCRATCH/feature-sets.jq` exits non-zero
- [X] T007 Write the output guard for T008 and test it: a `jq -e` expression over the program's output that fails unless the host part of `matrix` has at least 3 legs, `tests` has at least 1 entry and `no_std` is non-empty (contract feature-sets.md § Guarantees: "never passes with an empty list"). Check it accepts T006's real output and rejects `{"matrix":[],"tests":[],"heap_free":"","no_std":""}`
- [X] T008 In `.github/workflows/ci.yml` add job `feature-sets` with `name: "feature sets: list"` (contract ci-checks.md § Checks row `feature-sets`; FR-004; R-1, R-2): `runs-on: ubuntu-latest`, `timeout-minutes: 20`, checkout with `persist-credentials: false`, `dtolnay/rust-toolchain@stable`; one step `id: list` with `shell: bash` running `set -euo pipefail`, `cargo metadata --no-deps --format-version 1 | jq -c --arg pkg dxl_rs_v2 '<program from T006, inline>' > "$RUNNER_TEMP/sets.json"`, the T007 guard, then `echo "matrix=$(jq -c .matrix "$RUNNER_TEMP/sets.json")" >> "$GITHUB_OUTPUT"` and the same for `tests`, `heap_free` (`jq -r`), `no_std` (`jq -r`). Declare job `outputs:` `matrix`, `tests`, `heap_free`, `no_std` from `steps.list.outputs.*`. The program stays inline (plan Structure Decision: no script file outside `.github/workflows/ci.yml`)

**Checkpoint**: The workflow is fork-safe, cancels stale PR runs, and publishes the feature-set
lists. Stories can start.

---

## Phase 3: User Story 1 - Prove the no_std, heap-free promise as the SRS states it (Priority: P1) 🎯 MVP

**Goal**: The `no-std` job builds (not only lints) the library for `thumbv7em-none-eabihf` with
no features, `alloc` alone, every heap-free feature and every no_std feature (FR-002, R-3).

**Independent Test**: In a throwaway worktree, add a heap allocation to code reachable without
`std`; `no_std build (thumbv7em-none-eabihf)` turns red and names the line. Revert; it is green.

### Tests for User Story 1 ⚠️ (seeded faults, run before T012)

- [X] T009 [P] [US1] Seeded fault F1 (US1 AS2, AS3; quickstart § 2 F1) on `ci-fault/f1-alloc`: add to `src/lib.rs` `/// Seeded fault.\npub fn leak() -> alloc::vec::Vec<u8> { alloc::vec::Vec::new() }`. Run `RUSTFLAGS="-D warnings" cargo build --lib --no-default-features --features ax,mock,mx,pro,x-series,y-series --target thumbv7em-none-eabihf`: must fail with `E0433` naming the `src/lib.rs` line; the same without `--target` fails too (`src/lib.rs` is `#![no_std]`, so host sets without `alloc` fail), and so does `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --no-default-features`. Run the target build with `--features alloc,ax,mock,mx,pro,x-series,y-series`: must pass (AS3 contrast). Note that today's no_std clippy also catches F1; it is the regression guard, not the gap. Commit on the throwaway branch
- [X] T010 [P] [US1] Seeded fault F2 (US1 AS2; quickstart § 2 F2) on `ci-fault/f2-std`: add a documented `pub fn shout() { std::println!("x"); }` to `src/lib.rs` (no `cfg`). Run the T009 heap-free build: must fail naming `std` and the line; with `--features alloc` (host and target) it fails too, and so does the std-off compile-fail pass `RUSTC_BOOTSTRAP=1 cargo +1.97.1 test --doc --no-default-features --features alloc,ax,mock,mx,pro,x-series,y-series`. Commit on the throwaway branch
- [X] T011 [P] [US1] Seeded fault F1b (US1 AS1, the gap today's lint leaves) on `ci-fault/f1b-nofeature`: add to `src/lib.rs` a documented `#[cfg(not(any(feature = "ax", feature = "mx", feature = "x-series", feature = "y-series", feature = "pro", feature = "mock")))] pub fn leak() -> alloc::vec::Vec<u8> { alloc::vec::Vec::new() }`. Show today's no_std clippy (`--features ax,mx,x-series,y-series,pro,mock`) passes, and `RUSTFLAGS="-D warnings" cargo build --lib --no-default-features --target thumbv7em-none-eabihf` fails with `E0433`. Commit on the throwaway branch

### Implementation for User Story 1

- [X] T012 [US1] Extend job `no-std` in `.github/workflows/ci.yml` (contract ci-checks.md § Checks row `no-std`; FR-001, FR-002; R-3): set `name: no_std build (thumbv7em-none-eabihf)`, `needs: feature-sets`, job `env:` `HEAP_FREE: ${{ needs.feature-sets.outputs.heap_free }}` and `NO_STD: ${{ needs.feature-sets.outputs.no_std }}`. Replace the clippy step with a named step `clippy (heap-free, all)` running `cargo clippy --no-default-features --features "$HEAP_FREE" --target thumbv7em-none-eabihf` (today that set equals `ax,mx,x-series,y-series,pro,mock`, so FR-001 holds). Add four named steps, each `cargo build --lib --no-default-features --target thumbv7em-none-eabihf` plus: `build: no features` (nothing), `build: alloc alone` (`--features alloc`), `build: heap-free, all` (`--features "$HEAP_FREE"`), `build: no_std, all` (`--features "$NO_STD"`). No libudev step (no `std` here)
- [X] T013 [US1] Verify US1: in the `007-ci` worktree run the five commands of T012 with `RUSTFLAGS="-D warnings"` and today's `HEAP_FREE`/`NO_STD` values from T006: all pass. Rebase `ci-fault/f1-alloc`, `ci-fault/f2-std`, `ci-fault/f1b-nofeature` onto `007-ci` and run the same commands: F1 and F2 red on every set without `alloc`/`std`, F1b red on `build: no features` only, `build: alloc alone` green for F1 (AS3). Note the outputs for the PR description (T039)

**Checkpoint**: US1 is done: the no_std promise is built for every set FR-002 names.

---

## Phase 4: User Story 2 - Every feature set a user can pick builds (Priority: P1)

**Goal**: One named check per feature set (host: none, each alone, defaults, all; target: each
no-std feature alone), one test leg per heap-free feature, tests and clippy with all features, and
one stable aggregate check `feature sets` (FR-004, FR-005, FR-014; R-1, R-4, R-10).

**Independent Test**: In a throwaway worktree, make an `ax`-gated item use an `mx`-gated item;
`feature set (host): ax alone`, `feature set (thumbv7em): ax alone` and `feature sets` turn red.

### Tests for User Story 2 ⚠️ (seeded faults, run before T018)

- [X] T014 [P] [US2] Seeded fault F3 (US2 AS1, AS2; quickstart § 2 F3) on `ci-fault/f3-cross-family`: add to `src/lib.rs` a documented `#[cfg(feature = "ax")] pub const SEEDED: usize = core::mem::size_of::<crate::models::Mx28>();` (`Mx28` is `mx`-gated in `src/models.rs`). Show `cargo build --lib`, `cargo build --lib --all-features` and today's whole workflow pass (gap), and `RUSTFLAGS="-D warnings" cargo build --lib --no-default-features --features ax` fails, as do the same with `--target thumbv7em-none-eabihf` and `RUSTFLAGS="-D warnings" cargo test --lib --no-default-features --features std,ax`. Commit on the throwaway branch
- [X] T015 [P] [US2] Seeded change F4 (US2 AS3; SC-005; quickstart § 2 F4) on `ci-fault/f4-new-feature`: add `partial-reads = []` to `[features]` in `Cargo.toml` (throwaway only, no code). Run the T006 program: 12 host legs including `partial-reads alone`, 8 target legs including `partial-reads alone`, 8 test legs including `partial-reads`, and `heap_free`/`no_std` gain `partial-reads`; every new leg's command passes. No workflow edit. Commit on the throwaway branch
- [X] T016 [P] [US2] Seeded fault F10 (US2 AS4; FR-005) on `ci-fault/f10-all-features-test`: add to `src/transport/mock.rs` `#[cfg(all(test, feature = "mock"))] #[test] fn seeded_all_features() { panic!("seeded"); }`. Show `cargo test --workspace` passes (gap: today's unit tests never enable `mock`), and `RUSTFLAGS="-D warnings" cargo test --workspace --all-features` and `RUSTFLAGS="-D warnings" cargo test --lib --no-default-features --features std,mock` fail naming the test. Commit on the throwaway branch
- [X] T017 [P] [US2] Seeded fault F11 (US2 AS4 via R-4: tests that compile only with a family off) on `ci-fault/f11-family-off-test`: add to `src/lib.rs` `#[cfg(all(test, not(feature = "ax")))] mod seeded { #[test] fn family_off() { panic!("seeded"); } }`. Show `cargo test --workspace` and `cargo test --workspace --all-features` pass (never compiled), and `RUSTFLAGS="-D warnings" cargo test --lib --no-default-features --features std,mx` fails naming the test. Commit on the throwaway branch

### Implementation for User Story 2

- [X] T018 [US2] Add matrix job `feature-set` in `.github/workflows/ci.yml` (contract ci-checks.md § Checks row `feature-set`, § System packages; feature-sets.md § Output; R-4, R-14): `name: "feature set (${{ matrix.where }}): ${{ matrix.name }}"`, `needs: feature-sets`, `runs-on: ubuntu-latest`, `timeout-minutes: 20`, `strategy: { fail-fast: false, matrix: { include: ${{ fromJSON(needs.feature-sets.outputs.matrix) }} } }`; steps: checkout (`persist-credentials: false`); `dtolnay/rust-toolchain@stable` with `targets: thumbv7em-none-eabihf`; `Swatinem/rust-cache@v2` with `shared-key: feature-set-${{ matrix.where }}` and `save-if: ${{ github.ref == 'refs/heads/main' }}`; `Install libudev (serialport)` (`sudo apt-get update && sudo apt-get install -y libudev-dev`) with `if: matrix.needs_std`; a step `cargo build --lib` with `env: ARGS: ${{ matrix.args }}` and `run: cargo build --lib $ARGS` (unquoted on purpose so the flags split)
- [X] T019 [US2] Add matrix job `feature-test` in `.github/workflows/ci.yml` (contract ci-checks.md § Checks row `feature-test`; feature-sets.md § Rule 8; R-4): `name: "feature tests (host): ${{ matrix.name }}"`, `needs: feature-sets`, `runs-on: ubuntu-latest`, `timeout-minutes: 20`, `strategy: { fail-fast: false, matrix: { include: ${{ fromJSON(needs.feature-sets.outputs.tests) }} } }`; steps: checkout (`persist-credentials: false`), stable toolchain, `Swatinem/rust-cache@v2` with `shared-key: feature-test-host` and `save-if: ${{ github.ref == 'refs/heads/main' }}`, `Install libudev (serialport)` (always: every test leg enables `std`), then `cargo test --lib $ARGS` with `env: ARGS: ${{ matrix.args }}`
- [X] T020 [US2] Add aggregate job `features` in `.github/workflows/ci.yml` (contract ci-checks.md § Checks row `features`; data-model.md § Check states): `name: feature sets`, `if: always()`, `needs: [feature-sets, feature-set, feature-test]`, `runs-on: ubuntu-latest`, `timeout-minutes: 20`, no checkout; one step with `env: RESULTS: ${{ join(needs.*.result, ' ') }}` and a bash script that prints each result and exits 1 unless every word is `success` (so `failure`, `cancelled` and `skipped` all fail the aggregate)
- [X] T021 [US2] Extend job `check` in `.github/workflows/ci.yml` (contract ci-checks.md § Checks row `check`; FR-005; R-10): after the existing steps, which stay unchanged, add `cargo clippy --workspace --all-targets --all-features` and `cargo test --workspace --all-features`
- [X] T022 [US2] Verify US2: with a scratch shell loop (in `$SCRATCH`, not the repo) over the T006 output, run every `feature-set` leg (`cargo build --lib $ARGS`) and every `feature-test` leg (`cargo test --lib $ARGS`) plus the two T021 commands under `RUSTFLAGS="-D warnings"` in the `007-ci` worktree: all pass. Rebase T014–T017 onto `007-ci`: F3 red only on the two `ax alone` legs and the `ax` test leg, F4 green with the three new legs, F10 red on `cargo test --workspace --all-features` and the `mock` test leg, F11 red on every test leg that has `ax` off (`std`, `mx`, `x-series`, `y-series`, `pro`, `mock`). Rebase T009–T011 too: F1, F1b and F2 red on exactly the host legs quickstart § 2 names

**Checkpoint**: US1 and US2 done: both P1 promises are machine-checked.

---

## Phase 5: User Story 3 - See the coverage figure on every PR (Priority: P2)

**Goal**: The `coverage` job writes the line coverage figure and the 70 % threshold to the run
summary before the gate, pass or fail, with no write permission (FR-006, R-8).

**Independent Test**: On any PR after the change, "Details" on `CI / coverage (>= 70 % lines)`
then "Summary" shows the "Line coverage" table.

### Tests for User Story 3 ⚠️ (run before T024)

- [X] T023 [US3] Test the summary script (US3 AS1, AS2; contract ci-checks.md § Run summary of `coverage`): write the script of T024's summary step to `$SCRATCH/coverage-summary.sh` (bash, `set -euo pipefail`, reads a JSON file path, env `THRESHOLD=70`, appends to `$GITHUB_STEP_SUMMARY`, prints `::notice title=Line coverage::<percent> % (threshold 70 %)`). Run it (a) on the real output of `cargo llvm-cov --workspace --no-report && cargo llvm-cov report --json --summary-only --ignore-filename-regex 'src/models(\.rs|/)'` and (b) on a hand-made `{"data":[{"totals":{"lines":{"count":100,"covered":50,"percent":50.0}}}]}`, both with `GITHUB_STEP_SUMMARY=$SCRATCH/summary.md`. Expected: the exact table of the contract (percent with 2 decimals, `(covered / count)`, `70 %`, `pass` for (a) and `fail` for (b), the "Generated models … excluded" line), and exit 0 in both cases (the gate is the next step)

### Implementation for User Story 3

- [X] T024 [US3] Replace the single `cargo llvm-cov` step of job `coverage` in `.github/workflows/ci.yml` with three named steps (contract ci-checks.md § Checks row `coverage`; R-8): `Run tests with coverage` → `cargo llvm-cov --workspace --no-report`; `Publish line coverage to the run summary` → `cargo llvm-cov report --json --summary-only --ignore-filename-regex 'src/models(\.rs|/)' > "$RUNNER_TEMP/cov.json"` followed by the T023 script inline, with `env: THRESHOLD: 70`; `Gate: line coverage >= 70 %` → `cargo llvm-cov report --ignore-filename-regex 'src/models(\.rs|/)' --fail-under-lines 70`. Keep the job name `coverage (>= 70 % lines)`, the `llvm-tools-preview` component, `taiki-e/install-action@cargo-llvm-cov` and the libudev step. Add no permission
- [X] T025 [US3] Verify US3: run the three T024 commands locally; the percentage equals the baseline from T003 (same regex, same tests: FR-001 not weakened) and the gate exits 0; with a temporary `--fail-under-lines 101` the gate exits 1 while the summary step already wrote its table (AS2). Do not commit the 101

**Checkpoint**: US3 done: the coverage figure is on the PR page, pass or fail.

---

## Phase 6: User Story 4 - Docs are complete and their examples fail for the right reason (Priority: P2)

**Goal**: PR #7's `#![warn(missing_docs)]` fails every leg under `-D warnings`, the docs build
fails on any rustdoc warning, and compile-fail examples are checked by error code on a pinned
toolchain (FR-007, FR-008; R-5, R-6, R-7).

**Independent Test**: In throwaway worktrees, remove a doc comment from a public item, break an
intra-doc link, and change `compile_fail,E0277` to `E0308`; each turns its check red.

### Tests for User Story 4 ⚠️ (seeded faults, run before T030)

- [X] T026 [P] [US4] Seeded fault F5 (US4 AS1; quickstart § 2 F5) on `ci-fault/f5-missing-docs`: remove the doc comment of `Id::new` in `src/id.rs`. Show today's `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` already fails naming the item (PR #7's `#![warn(missing_docs)]`: F5 is the regression guard, not the gap), and that the new commands fail too: `RUSTFLAGS="-D warnings" cargo build --lib --no-default-features --features ax --target thumbv7em-none-eabihf`, `RUSTFLAGS="-D warnings" cargo test --lib --no-default-features --features std`, `RUSTFLAGS="-D warnings" cargo llvm-cov --workspace --no-report` and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`. Commit on the throwaway branch
- [X] T027 [P] [US4] Seeded fault F5b (US4 AS1 "with any feature set"; quickstart § 2 F5b) on `ci-fault/f5b-set-only-docs`: add to `src/lib.rs` an undocumented `#[cfg(all(feature = "alloc", not(feature = "std")))] pub fn alloc_only() {}`. Show every command of today's workflow passes (gap: no set with `alloc` and without `std` is built), and that `RUSTFLAGS="-D warnings" cargo build --lib --no-default-features --features alloc` (host, and with `--target thumbv7em-none-eabihf`) and `RUSTC_BOOTSTRAP=1 cargo +1.97.1 test --doc --no-default-features --features alloc,ax,mock,mx,pro,x-series,y-series` fail with "missing documentation". Commit on the throwaway branch
- [X] T028 [P] [US4] Seeded fault F6 (US4 AS2; quickstart § 2 F6) on `ci-fault/f6-doc-link`: add `` [`NoSuchItem`] `` to a doc comment in `src/transport.rs`. Show today's commands pass (gap: no doc build in CI) and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` fails with `rustdoc::broken_intra_doc_links`. Commit on the throwaway branch
- [X] T029 [P] [US4] Seeded fault F7 (US4 AS3; quickstart § 2 F7) on `ci-fault/f7-error-code`: in `src/transport.rs` change ` ```compile_fail,E0277 ` (the `Transport` example without `ReadReady`) to ` ```compile_fail,E0308 `. Show `cargo test --doc` on stable passes (gap) and `RUSTC_BOOTSTRAP=1 cargo +1.97.1 test --doc --all-features` fails with "Some expected error codes were not found". On the unmodified `007-ci` worktree, show all three passes of quickstart § 1 (`--all-features`, defaults, `--no-default-features --features alloc,ax,mock,mx,pro,x-series,y-series`) pass. Commit on the throwaway branch

### Implementation for User Story 4

- [X] T030 [US4] Add job `docs` in `.github/workflows/ci.yml` (contract ci-checks.md § Checks row `docs`; FR-007; R-6): `name: "docs: complete, no warnings"`, `runs-on: ubuntu-latest`, `timeout-minutes: 20`, job `env: RUSTDOCFLAGS: -D warnings`; steps: checkout (`persist-credentials: false`), stable toolchain, `Swatinem/rust-cache@v2`, `Install libudev (serialport)`, `cargo doc --no-deps --all-features`, `cargo doc --no-deps --no-default-features`
- [X] T031 [US4] Add job `compile-fail` in `.github/workflows/ci.yml` (contract ci-checks.md § Checks row `compile-fail`; data-model.md § Toolchain pin; FR-008; R-7): `name: "doc examples: compile-fail error codes"`, `needs: feature-sets`, `runs-on: ubuntu-latest`, `timeout-minutes: 20`, job `env: NO_STD: ${{ needs.feature-sets.outputs.no_std }}`; steps: checkout (`persist-credentials: false`); a step named `Install pinned toolchain (COMPILE_FAIL_TOOLCHAIN)` using `dtolnay/rust-toolchain@master` with `toolchain: ${{ env.COMPILE_FAIL_TOOLCHAIN }}`; `Swatinem/rust-cache@v2`; `Install libudev (serialport)`; three steps, each with step-level `env: RUSTC_BOOTSTRAP: 1` only: `doc tests: all features` → `cargo +"$COMPILE_FAIL_TOOLCHAIN" test --doc --all-features`; `doc tests: defaults` → `cargo +"$COMPILE_FAIL_TOOLCHAIN" test --doc`; `doc tests: std off` → `cargo +"$COMPILE_FAIL_TOOLCHAIN" test --doc --no-default-features --features "$NO_STD"`. `RUSTC_BOOTSTRAP` appears nowhere else in the file
- [X] T032 [US4] Verify US4: `grep -n RUSTC_BOOTSTRAP .github/workflows/ci.yml` shows exactly the three step envs; `grep -n COMPILE_FAIL_TOOLCHAIN` shows one definition (US4 AS4). In the `007-ci` worktree run both `cargo doc` commands and the three compile-fail passes: all pass. If the std-off pass fails on a doc example that uses a std-only item, stop: the fix (`#[cfg_attr(feature = "std", doc = …)]`) belongs to the module that owns that file, not to `ci`. Rebase T026–T029 onto `007-ci`: F5 red on every command T026 names, F5b red on the `alloc alone` builds, the no_std `alloc` and `no_std, all` builds and the std-off compile-fail pass only, F6 red on `docs` only, F7 red only on the compile-fail passes. With T009–T011 rebased: F1, F1b and F2 red on `cargo doc --no-deps --no-default-features`, F2 also on the std-off compile-fail pass

**Checkpoint**: US4 done: missing docs, doc warnings and wrong error codes all turn CI red.

---

## Phase 7: User Story 5 - The std build works on every desktop OS the SRS names (Priority: P2)

**Goal**: Default build and every crate test that needs no serial device run on Windows and
macOS; Linux is the renamed `check` job (FR-009; R-9).

**Independent Test**: In a throwaway worktree, add a type error in Windows-only serial code;
only `std build and tests (Windows)` turns red.

### Tests for User Story 5 ⚠️ (seeded fault, run before T034)

- [X] T033 [US5] Seeded fault F8 (US5 AS2; quickstart § 2 F8) on `ci-fault/f8-windows`: in `src/transport/serial.rs` add `#[cfg(windows)] const SEEDED: u8 = "x";` next to the `#[cfg(windows)] type NativePort` line. Show every Linux command of today's workflow passes (gap: Windows code never compiled). Best effort locally: `rustup target add x86_64-pc-windows-gnu && cargo check --lib --target x86_64-pc-windows-gnu` fails with `E0308`; if the Windows dependencies do not check from Linux, record that and rely on T040. Commit on the throwaway branch

### Implementation for User Story 5

- [X] T034 [US5] In `.github/workflows/ci.yml` set job `check`'s `name: fmt, clippy, test (Linux)` (job id `check` unchanged; contract ci-checks.md § Checks row `check`)
- [ ] T035 [US5] Add matrix job `os` in `.github/workflows/ci.yml` (contract ci-checks.md § Checks row `os`; FR-009; R-9): `name: "std build and tests (${{ matrix.label }})"`, `runs-on: ${{ matrix.os }}`, `timeout-minutes: 20`, `strategy: { fail-fast: false, matrix: { include: [ { os: windows-latest, label: Windows }, { os: macos-latest, label: macOS } ] } }`; steps: checkout (`persist-credentials: false`), stable toolchain, `Swatinem/rust-cache@v2`, `cargo build -p dxl_rs_v2`, `cargo test -p dxl_rs_v2`. No system-package step and no OS-based test skip in the workflow (pty tests skip on Windows by their own `#[cfg(all(test, unix))]`)

**Checkpoint**: All five active stories are done.

---

## Phase 8: Polish & cross-cutting

**Purpose**: The table check handed over by the tables module, audit, PR, seeded-fault runs on
GitHub, and the architect's follow-ups

- [ ] T036 Add job `control-tables` in `.github/workflows/ci.yml` (contract ci-checks.md § Checks row `control-tables`; spec FR-016; R-17; tables spec 005 FR-014): `name: "control tables: schema check"`, `runs-on: ubuntu-latest`, `timeout-minutes: 20`, checkout (`persist-credentials: false`), no Rust toolchain; one bash step: `if [ -f control_tables/check_tables.py ]; then python3 control_tables/check_tables.py && python3 -m unittest discover -s control_tables -p 'test_*.py'; else echo "::notice title=control tables::control_tables/check_tables.py not present yet (tables T1); nothing to check"; fi`. If T001 found T1 already merged, write the two commands without the guard
- [X] T037 Audit `.github/workflows/ci.yml` against the Rules and the contract: `grep -nE 'paths|continue-on-error|pull_request_target|self-hosted|secrets\.' ` finds nothing; the number of `persist-credentials: false` equals the number of `actions/checkout`; every job has `timeout-minutes: 20`; no `${{ matrix.` or `${{ needs.` inside any `run:` block; every check name matches contract ci-checks.md § Checks exactly; every command saved in `$SCRATCH/baseline.txt` (T003) is still present with the same flags (FR-001); `git diff --exit-code origin/main -- Cargo.toml` shows no change (FR-015). Run `actionlint .github/workflows/ci.yml` if `actionlint` is installed; otherwise parse with `python3 -c "import yaml,sys; yaml.safe_load(open(sys.argv[1]))" .github/workflows/ci.yml` if PyYAML is available, else rely on the first GitHub run
- [X] T038 Run quickstart.md § 1 in full in the `007-ci` worktree under `RUSTFLAGS="-D warnings"` (feature sets from the T006 program, feature tests, no_std builds, all-features clippy and tests, both docs runs, three compile-fail passes, coverage three steps, table check or its guard): every command passes
- [X] T039 Commit on `007-ci` (Conventional Commits, e.g. `ci: build no_std and every feature set, publish coverage, check docs and error codes`; plus `docs(ci): add spec, plan and tasks` for `specs/007-ci/`), no AI attribution. Fill `specs/007-ci/pr-description.md` from `.github/pull_request_template.md`: What/Why (SRS module `ci`: NFR-1, NFR-6, FR-7.3; spec FR-001…FR-016), How (dynamic matrix from `cargo metadata` + `jq`; pinned 1.97.1 with `RUSTC_BOOTSTRAP=1`; summary before gate), Architecture (one missing-docs switch, PR #7's `#![warn(missing_docs)]`, and no `Cargo.toml` change: the deviation from module-interfaces.md § 1 in plan.md; the computed FR-002 set from plan.md; the `control tables: schema check` job (FR-016, from tables spec 005 FR-014) is guarded until T1; FR-003 firmware link and US6 MSRV deferred), the local results of T013, T022, T025, T032, the required-check list of contract ci-checks.md § Required-check guidance, and the architect's follow-ups (T043–T045). Push `007-ci` and open the PR into `main` from the `Ovsiankina` account with that description
- [X] T040 Seeded-fault runs on GitHub (SC-001; quickstart § 2): rebase every `ci-fault/*` branch (F1, F1b, F2, F3, F4, F5, F5b, F6, F7, F8, F10, F11) onto the pushed `007-ci`, add F9 on `ci-fault/f9-coverage` (a large documented `pub fn` no test calls, enough to drop line coverage below 70 %), push each and open a draft PR into `main` titled `DO NOT MERGE: ci seeded fault <id>`. Check that exactly the checks in the "Expected red check" column of quickstart § 2 turn red, with the item named in the log (F4: nothing red, the three new `partial-reads` legs present; F9: the summary table present). Record any extra red check as a finding in the PR description. Then close every draft PR, delete the remote and local `ci-fault/*` branches and remove their worktrees
- [ ] T041 Validate quickstart.md § 3–6 on GitHub and record the result in the PR description: coverage table reached in 2 clicks (SC-002); a PR from a fork (`gh repo fork`) with a one-line docs change shows the same checks, all run, `Contents: read` in "Set up job" (FR-011, SC-004); two pushes within a minute cancel the first run (FR-012); a PR that changes only `specs/` runs every check (FR-013); the compile-fail job installs 1.97.1 (US4 AS4). Close the probe PRs afterwards
- [X] T042 Note the wall-clock time of the `007-ci` PR run with a warm cache (re-run once after the first run fills the cache) in the PR description (SC-003, ≤ 15 min). If it is over, report it; do not drop or weaken a gate to fit
- [ ] T043 [human] After the ci PR merges: mark the required checks of contract ci-checks.md § Required-check guidance in `.github/ov-ruleset-main.json` (`.aiignore` path; not in the repository yet, create it first, e.g. with `/speckit-ov-pr`): `fmt, clippy, test (Linux)`, `no_std build (thumbv7em-none-eabihf)`, `coverage (>= 70 % lines)`, `generated models up to date`, `feature sets`, `docs: complete, no warnings`, `doc examples: compile-fail error codes`, `control tables: schema check`, `std build and tests (Windows)`, `std build and tests (macOS)`. Not the individual `feature set (…)` / `feature tests (…)` legs
- [ ] T044 [human] Architect: add the new gates to the constitution's "Development Workflow and Quality Gates" list (`/speckit-constitution`), and update the checklist of `.github/pull_request_template.md` and the Commands section of `CLAUDE.md` (both outside `ci`'s paths, plan.md § Follow-ups)
- [ ] T045 [human] Architect, after merge: track SC-003 over the first 5 PR runs (≤ 15 min warm) and SC-006 over the first 20 PRs (≤ 1 unrelated blocking failure); decide on pinning actions by commit SHA (R-11, optional)

### Deferred (not tasks; do not implement)

- **DEFERRED** (not a task), architect, later feature (FR-003, US1 AS4): link a minimal firmware binary that
  calls the public API on `thumbv7em-none-eabihf` with no global allocator. It needs files
  outside `ci`'s paths. No task ID until the architect schedules it.
- US6 / MSRV: deferred by FR-010. No `rust-version`, no MSRV job.
- After tables T1 merges (if T036 shipped with the guard): the first ci change after T1 removes
  the `if [ -f control_tables/check_tables.py ]` guard (module-interfaces.md § 7).
- Optional items in spec Assumptions / R-18 (audit, licence, hardware job, semver, schedule,
  Miri, update bots) are not built here.

---

## PR map

One PR (plan.md § PR split).

| PR | Tasks | Branch | Must be merged first | Gate commands before opening |
|---|---|---|---|---|
| ci | T001–T042 (T043–T045 are [human], after merge) | `007-ci` off `main` | core-1 (module-interfaces.md § 7; PR #7 is merged); human precondition: `.aiignore` and `.github/workflows/ov-aiignore.yml` on `main`; tables T1 optional (the table check is guarded until T1) | quickstart.md § 1 in full under `RUSTFLAGS="-D warnings"`, including: `cargo fmt --check`; `cargo clippy --workspace --all-targets` (plain, `--features mock`, `--all-features`); `cargo test --workspace` and `--all-features`; `cargo test --features mock --doc`; every `feature-set` and `feature-test` leg; the no_std clippy and four `cargo build --lib … --target thumbv7em-none-eabihf`; `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` (`--all-features`, `--no-default-features`); `RUSTC_BOOTSTRAP=1 cargo +1.97.1 test --doc` (three passes); `cargo llvm-cov --workspace --ignore-filename-regex 'src/models(\.rs\|/)' --fail-under-lines 70`; `cargo xtask codegen` + `git diff --exit-code -- src/models.rs src/models` |

Early split, only with the architect's approval ("one module per PR") and only if core-1 slips
(plan.md § PR split):

| PR | Tasks | Branch | Must be merged first |
|---|---|---|---|
| ci-1 | T001–T008, T023–T025 (US3), T026, T028–T031 (F5, F6, F7, docs, compile-fail), T033–T035 (US5), T036–T042 | `007-ci-1` off `main` | nothing (PR #7 is merged; the human precondition of T001 still applies) |
| ci-2 | T009–T022 (US1, US2), T027 (F5b), T032, then re-run T037–T042 | `007-ci-2` off `main` | ci-1, core-1 |

---

## Dependencies & Execution Order

### Phase dependencies

- Setup (T001–T003) → Foundational (T004–T008) → stories → Polish (T036–T045).
- T001 blocks everything: without core-1 on `main`, the no-feature legs are red for a reason
  outside this module (R-16), and without `.aiignore` and `ov-aiignore.yml` on `main` the human
  precondition is not met. PR #7 is merged.
- Inside Foundational: T004 → T005 (same file); T006 → T007 → T008.

### User story dependencies

- US1 (T009–T013): needs T008 (`heap_free`, `no_std` outputs).
- US2 (T014–T022): needs T008 (`matrix`, `tests` outputs). Independent of US1.
- US3 (T023–T025): needs only T004–T005.
- US4 (T026–T032): T030 needs only T004–T005; T031 needs T008 (`no_std` output).
- US5 (T033–T035): needs only T004–T005.
- Polish: T036 after T005; T037–T042 after every story; T040 after T039 (pushed branch);
  T043–T045 after merge.

### Within each story

- Seeded-fault tasks first (they show the gap with today's commands), then the job, then the
  verify task, which reruns the faults against the new job.
- All story implementation tasks edit `.github/workflows/ci.yml`, so they run one after another
  even across stories.

### Parallel opportunities

- Seeded-fault tasks each use their own worktree: T009, T010, T011, T014, T015, T016, T017, T026,
  T027, T028, T029 can all run at once (after T002), and T033 with them.
- T006/T007 (scratch `jq` work) in parallel with T004/T005.
- Story job edits are sequential in one file, but US3, US5 and the docs half of US4 do not wait
  for T008.

---

## Parallel Example: User Story 1

```text
T009 F1 alloc  |  T010 F2 std  |  T011 F1b no-feature alloc      (three worktrees)
→ T012 extend no-std job → T013 verify (rebase faults, all red; alloc alone green)
```

## Parallel Example: User Story 2

```text
T014 F3 cross-family  |  T015 F4 new feature  |  T016 F10 all-features test  |  T017 F11 family-off test
→ T018 feature-set → T019 feature-test → T020 features aggregate → T021 check + all features → T022 verify
```

## Parallel Example: User Story 4

```text
T026 F5 missing docs  |  T027 F5b set-only docs  |  T028 F6 doc link  |  T029 F7 error code
→ T030 docs job → T031 compile-fail job → T032 verify
```

---

## Implementation Strategy

### MVP first (User Story 1)

1. Setup (T001–T003); stop if core-1 or the human precondition (`.aiignore`, `ov-aiignore.yml`
   on `main`) is missing.
2. Foundational (T004–T008).
3. US1 (T009–T013): the no_std, heap-free build is proved for every set FR-002 names.
4. Stop and validate with T013. This alone closes the NON-NEGOTIABLE gap (constitution I).

### Incremental delivery (one PR, in this order)

1. US1 → MVP.
2. US2 → every feature set and the `feature sets` aggregate.
3. US3 → coverage figure on the PR page.
4. US4 → missing docs, doc warnings, compile-fail error codes.
5. US5 → Windows and macOS.
6. Polish → table check, audit, PR, seeded faults on GitHub, follow-ups for the architect.
