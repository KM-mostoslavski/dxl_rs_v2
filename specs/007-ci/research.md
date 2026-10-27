# Research: CI (hardened quality gates)

**Feature**: [spec.md](spec.md) | **Date**: 2026-10-27

Sources: `.github/workflows/ci.yml` and `Cargo.toml` on `dev`; `src/lib.rs`, `src/error.rs` and
`Cargo.toml` on `001-protocol-packets` (PR #7); `specs/ideas/module-interfaces.md`; GitHub
Actions docs; `cargo-llvm-cov` 0.9.1 and rustdoc behaviour checked locally.

Prototype: scratch copies of `dev` and `001-protocol-packets` (made with `git archive`) under
`/tmp/claude-1000/ci-proto/`, built with stable 1.97.1, a pinned 1.97.1 and nightly 1.99.0
(2026-08-03). Nothing was built in the repository.

## R-1 How the feature sets are listed

**Decision**: A first job, `feature-sets`, reads the features from
`cargo metadata --no-deps --format-version 1` and turns them into a JSON list with a short `jq`
program (contract: [contracts/feature-sets.md](contracts/feature-sets.md)). A second job runs one
matrix leg per entry. A third job, `feature sets`, waits for all legs and fails if any leg failed.

Host list: no features, each declared feature alone, the defaults, all features. Target list
(`thumbv7em-none-eabihf`): each feature alone that does not need `std`.

**Rationale**:
- `Cargo.toml` is the only list. A new feature (e.g. `partial-reads`) joins with no workflow
  edit (FR-004, SC-005).
- Each leg is its own check, named after its feature set, e.g.
  `feature set (thumbv7em): ax alone`. A red leg names the set on the PR page (US2 AS2,
  FR-014).
- The `feature sets` job gives one stable name to mark required. Its name does not change when a
  feature is added.
- Cost is linear: one leg per feature, never every combination.

Prototype (scratch copy of PR #7): the program gives 11 host sets (8 features + 3) and 7 target
sets. With a fake `serial-extra = ["std"]` and `partial-reads = []` added, it gives 13 host sets,
drops `serial-extra` from the target list and adds `partial-reads` to it.

**Alternatives considered**:
- `cargo hack --each-feature` (taiki-e). Its docs say it covers the same sets (none, each alone,
  all). Rejected: the target run needs a hand-kept `--exclude-features std,default` list, so a
  new feature that enables `std` breaks the target run until someone edits the workflow
  (fails FR-004). It also reports every set inside one check. Not run here: installing it was
  not allowed in this session, so its behaviour is taken from its docs only.
- A hand-written matrix (today's style). Rejected: it goes stale when a feature is added.
- One job that loops over the sets. Rejected: the check name does not say which set broke; the
  reader must open the log.

## R-2 Which features need `std`

**Decision**: A feature needs `std` when `std` is in its transitive closure over local feature
names. Entries with `:` or `/` (`dep:serialport`, `thiserror/std`) are not local features and
are not followed. `needs alloc` uses the same rule with `alloc`.

**Rationale**: Today `std` and `default` need `std`; `alloc`, the five families and `mock` do
not. The rule finds that from `Cargo.toml` alone, so FR-004's "features that need `std` are built
on the host; all others also on the microcontroller target" holds for future features too.

**Alternatives considered**: a naming rule (e.g. `std-*`). Rejected: features are named by the
module that adds them, not by CI.

## R-3 no_std build sets and the heap-free proof

**Decision**: The `no_std build (thumbv7em-none-eabihf)` job keeps today's clippy run and adds
four `cargo build --lib` runs on `thumbv7em-none-eabihf` with `--no-default-features`:

| Set | Features |
|---|---|
| no features | none |
| alloc alone | `alloc` |
| heap-free, all | every feature that needs neither `std` nor `alloc` (today: the five families + `mock`) |
| no_std, all | every feature that does not need `std` (today: the above + `alloc`) |

The clippy run uses the "heap-free, all" set, which today is exactly
`ax,mx,x-series,y-series,pro,mock` (same strength as now, FR-001) and grows by itself.

**Rationale**:
- `src/lib.rs` declares `extern crate alloc` only under the `alloc` feature. Without it, any use
  of `alloc::` does not compile. Prototype: a seeded `pub fn leak() -> alloc::vec::Vec<u8>` fails
  the heap-free build with `E0433: cannot find module or crate 'alloc'`, naming the line (US1
  AS2).
- `std` is not linked on the target at all, so `std::` use fails the same way.
- FR-002 asks for "every model family plus `mock`". The computed set is that set today and a
  superset later. That is stronger, never weaker.

**Limit (recorded, not fixed)**: a library build does not prove that a dependency never
allocates inside its own code. Only linking a firmware binary with no global allocator proves
that. That is the FR-003 TODO. Today's dependencies (`thiserror`, `embedded-io`, both without
default features) are `no_std` without `alloc`.

**Alternatives considered**: `cargo check` instead of `cargo build`. Rejected: US1 AS1 says
"built, not only linted"; `build` also runs codegen and catches monomorphisation errors.

## R-4 What each feature-set leg runs

**Decision**: `cargo build --lib` with the leg's flags (host legs on `ubuntu-latest`, target legs
with `--target thumbv7em-none-eabihf`), under the workflow's `RUSTFLAGS: -D warnings`.

A second matrix, `feature-test`, runs `cargo test --lib` on the host for a short list of test
sets: `--no-default-features --features std`, and `--no-default-features --features std,<f>` for
every feature `f` that needs neither `std` nor `alloc` (today `ax`, `mx`, `x-series`,
`y-series`, `pro`, `mock`). The list is the `tests` array of the same `jq` output. Each leg is a
check named `feature tests (host): <f>` (or `: std`), and the `feature sets` aggregate waits for
it too.

**Rationale**:
- FR-004 asks for a build. With `-D warnings`, a dead item, an unused import or a missing doc
  that shows up under one feature set only fails that leg.
- Some crate tests compile only with a family off. The xtask plan (R-11, quickstart §3) puts the
  "family off" half of SC-004 in generated `src/models/tests.rs` under `cfg(not(feature = …))`.
  The defaults and all-features runs compile every family in, so they never run those tests. One
  family at a time, plus no family, runs each such test at least once.
- `std` stays on in every test leg: the test harness needs it anyway, and it keeps the list
  short (one leg per heap-free feature, not every combination).
- Prototype (scratch copy of PR #7, `-D warnings`): all 7 test sets pass (114 to 116 tests each).

**Alternatives considered**:
- `cargo clippy --all-targets` or `cargo test` on every build leg. Rejected: test code is not
  written to compile under every set (e.g. `alloc` alone with no `std`, or the target legs), so
  this would add red legs that do not match a user-visible promise.
- Tests only with defaults and all features (the first plan). Rejected: no run executes the
  family-off tests, so a broken lookup for a disabled family goes unnoticed.

## R-5 Missing documentation under every feature set

**Decision**: Add `[lints.rust] missing_docs = "warn"` to `Cargo.toml`. With `-D warnings` in CI
it fails every leg where a public item lacks docs. Locally it warns.

**Rationale**:
- PR #7 adds `#![warn(missing_docs)]` to `src/lib.rs`. That works, but the gate then depends on
  a line in a file `ci` does not own. The `[lints]` table is `ci`'s
  (module-interfaces.md § 1), so the promise is held where `ci` can keep it.
- The root `[lints]` table applies only to the root package, not to `xtask`. Prototype: on a
  scratch copy of `dev` with the table added, `cargo build --workspace` with `-D warnings` fails
  only on the two undocumented `UnsupportedModel` fields, and nothing in `xtask`.
- The feature-set legs build each set, so "under any checked feature set" (FR-007) holds.

**Interpretation of FR-015**: FR-015 allows "package metadata only" in `Cargo.toml`. The shared
contract (module-interfaces.md § 1) gives `ci` the `[package]`, metadata and `[lints]` tables.
A lint level is neither a feature nor a dependency, so the plan follows the contract.

**Day-one blocker**: `dev` fails this gate on the two `UnsupportedModel` fields. PR #7 documents
them. core-1 moves `UnsupportedModel` into `SetupError`; core keeps every new field documented.

**Alternatives considered**: `RUSTFLAGS="-D warnings -D missing_docs"`. Rejected: it also hits
the `xtask` binary and every path dependency, and it is invisible to local builds.

## R-6 Doc warnings

**Decision**: A `docs` job runs `cargo doc --no-deps` twice with `RUSTDOCFLAGS: -D warnings`:
with `--all-features`, and with `--no-default-features`.

**Rationale**: rustdoc lints (`broken_intra_doc_links`, `private_intra_doc_links`, bad code
blocks) warn by default; `-D warnings` makes them fail (FR-007). The two runs catch links that
break only when `std` items are compiled out. Prototype: both runs are clean on the PR #7 copy.

**Alternatives considered**: one doc run per feature set. Rejected: about 11 extra doc builds for
a rare case; missing docs per set are already caught by R-5.

## R-7 Compile-fail examples that fail for the right reason

**Decision**: A `compile-fail error codes` job runs three doc-test passes on a pinned stable
toolchain, `1.97.1`, with `RUSTC_BOOTSTRAP=1` set on those steps only:

1. `cargo test --doc --all-features`;
2. `cargo test --doc` (defaults);
3. `cargo test --doc --no-default-features --features $NO_STD` (the `no_std` output of
   `feature-sets`, so the job `needs: feature-sets`).

The version lives in one workflow-level variable, `COMPILE_FAIL_TOOLCHAIN`.

Every doc example that uses a std-only item is gated with
`#[cfg_attr(feature = "std", doc = …)]`, so pass 3 compiles. A ` ```compile_fail ` example that
must fail only with a feature or `std` off is gated with `#[cfg_attr(not(feature = …), doc = …)]`.

**Rationale**:
- Verified locally: on stable, a ` ```compile_fail,E0277 ` example that fails with `E0308`
  passes. On nightly it fails with "Some expected error codes were not found: [\"E0277\"]".
  Stable 1.97.1 with `RUSTC_BOOTSTRAP=1` gives the same failure. A correct code passes on both;
  an example with no code is not affected.
- A pinned toolchain changes only by a commit that edits `COMPILE_FAIL_TOOLCHAIN` (FR-008, US4
  AS4).
- 1.97.1 is the maintainer's toolchain (FR-010), so a contributor reproduces the check with the
  compiler they already have: `RUSTC_BOOTSTRAP=1 cargo +1.97.1 test --doc --all-features`.
- A stable release is never withdrawn, so the job cannot break because the toolchain vanished
  (spec edge case).
- The bus plan (003 R-14) gates some examples so they exist only with a feature off:
  `read_positions_partial`, `read_raw`, `write_raw` without their features (`E0599`), `DynBus`
  without `std` (`E0412`/`E0433`), a no_std `Bus::new` without a clock (`E0277`). Under
  `--all-features` they are compiled out, so pass 1 alone never checks their codes. Pass 2 runs
  the opt-in-feature-off cases; pass 3 runs the std-off cases. Without pass 3, no CI step would
  run doc tests with `std` off at all.
- Prototype: `dev` (1 compile-fail example, `E0277`) and PR #7 both pass all three passes.
- `RUSTC_BOOTSTRAP=1` only unlocks unstable behaviour; the crate uses no `#![feature]`, and the
  stable jobs still build everything without it. Its effect is limited to this one job.

**Alternatives considered**:
- A dated nightly (the spec's default). Works the same (verified with nightly 2026-08-03). Not
  chosen: a second compiler family to install and to keep in step; nightly may emit a different
  code than stable for the same snippet, and users build on stable.
- `trybuild` with `.stderr` files. Rejected by module-interfaces.md conflict 13 (no extra
  dependency; compile-fail cases stay rustdoc examples).
- Stable alone. Rejected: ignores the code, fails FR-008.

## R-8 Coverage figure on every PR

**Decision**: The coverage job runs in three steps:

1. `cargo llvm-cov --workspace --no-report` runs the tests and keeps the profile data.
2. `cargo llvm-cov report --json --summary-only --ignore-filename-regex 'src/models(\.rs|/)'`
   gives the figure (`.data[0].totals.lines.percent`). The step writes a short table to
   `$GITHUB_STEP_SUMMARY` (figure, threshold 70 %, pass or fail) and emits a `::notice` with the
   same line.
3. `cargo llvm-cov report --ignore-filename-regex 'src/models(\.rs|/)' --fail-under-lines 70` is
   the gate.

**Rationale**:
- The summary is written before the gate, so a failing PR still shows its figure (US3 AS2).
- `$GITHUB_STEP_SUMMARY` and workflow commands need no token and no write permission, so fork
  PRs show the figure too (US3 AS3, SC-004).
- Reach: PR page → the check's "Details" → "Summary" of the run: 2 clicks (SC-002).
- Same command, flags and regex as today, so the gate is not weakened (FR-001).
- Verified on `cargo-llvm-cov` 0.9.1: `report --json --summary-only` gives
  `{"count":…,"covered":…,"percent":…}` under `.data[0].totals.lines`, and
  `report --fail-under-lines` exits 1 when below.

**Alternatives considered**:
- A PR comment bot. Rejected: needs `pull-requests: write`, which fork PRs do not get (FR-011).
- An external coverage service. Rejected: needs a secret.

## R-9 Desktop operating systems

**Decision**: A `std build and tests` job with a matrix over `windows-latest` and
`macos-latest` runs `cargo build -p dxl_rs_v2` and `cargo test -p dxl_rs_v2` with default
features. Linux is the existing `check` job, renamed `fmt, clippy, test (Linux)` so every OS shows
in a check name.

**Rationale**:
- FR-009 is about the crate's `std` build (SRS 4.3). `xtask` is a codegen-time tool; its output
  is checked by the `codegen` job on Linux.
- Windows runners check out with `core.autocrlf=true`. `xtask` tests that compare text would fail
  on line endings, which is not a crate defect.
- Linux-only tests (pseudo-terminal pair) are already `#[cfg(all(test, unix))]` in
  `src/transport/serial.rs`, so Windows skips them by their own gate (spec edge case). macOS runs
  them.
- Neither OS needs a system package: `serialport` uses IOKit on macOS and the Win32 API on
  Windows. On Linux, `libudev-dev` stays a named setup step, so a missing package fails that
  step, not the linker (spec edge case).

**Risk (not verifiable here)**: CI has never built on Windows or macOS. The pty tests use 57 600
baud (a standard rate) and exclusive open; both should work on macOS, but this is unverified. If
a test fails on one OS, the fix lands in the transport module first, as the spec's edge cases
say. The workflow never skips a test.

**Alternatives considered**: a three-OS matrix that also repeats Linux. Rejected: runs the Linux
tests twice.

## R-10 Tests with every feature

**Decision**: The `check` job gains `cargo test --workspace --all-features` (FR-005) and
`cargo clippy --workspace --all-targets --all-features`, under the workflow's
`RUSTFLAGS: -D warnings`.

**Rationale**:
- Today's runs cover the defaults and `mock` doc examples only. Prototype on PR #7: passes (117
  unit tests, doc tests and `xtask` tests).
- Code and tests behind opt-in features (e.g. bus-5's `src/bus/partial.rs` and `src/bus/raw.rs`)
  are otherwise never linted with `--all-targets`: build legs run `cargo build --lib`, and the
  no_std clippy run skips tests. Prototype on PR #7: clean.

## R-11 Fork safety and permissions

**Decision**:
- `permissions: contents: read` at the top of `ci.yml`.
- Triggers stay `pull_request` and `push` to `main`. Never `pull_request_target`.
- Every `actions/checkout` uses `persist-credentials: false`.
- Values from the matrix reach `run:` scripts only through `env:`, never as `${{ … }}` inside
  the script text.
- GitHub-hosted runners only.

**Rationale**: FR-011 and SC-004. Feature names are limited by Cargo to letters, digits, `_`,
`-`, `+` and `.`, but they come from a PR-controlled file, so passing them through `env:` is the
standard guard against script injection. `pull_request` from a fork already gets a read-only
token and no secrets.

**Alternatives considered**: pinning actions by commit SHA. Not now: the existing jobs use tags
(`@v4`, `@v2`, `@stable`) and the token is read-only. Recorded as an option for the architect.

## R-12 Cancelling stale runs

**Decision**:

```yaml
concurrency:
  group: ci-${{ github.event.pull_request.number || github.ref }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}
```

**Rationale**: a newer push to a PR cancels the older run (FR-012). Pushes to `main` are never
cancelled, so each merge keeps its own result and saves its cache.

## R-13 Check names

**Decision**: The names in [contracts/ci-checks.md](contracts/ci-checks.md). Each says what it
proves. Matrix names carry the set and the target or OS. Nothing in a name changes with a
toolchain bump.

**Rationale**: FR-014. The ruleset (`.github/ov-ruleset-main.json`) is the architect's; no
required checks are configured today, so renames are free now and should stay rare after.

## R-14 Run time and caching

**Decision**:
- `Swatinem/rust-cache@v2` everywhere, as today.
- Feature-set legs share one cache per target (`shared-key: feature-set-host` /
  `feature-set-thumbv7em`) and save only on `main` (`save-if`).
- The OS legs use one key per OS.
- Every job gets `timeout-minutes: 20`.

**Rationale**: 36 jobs per run today (4 existing, 1 list, 18 build legs, 7 test legs,
1 aggregate, docs, compile-fail, control tables, 2 OS legs). Test legs share one cache
(`shared-key: feature-test-host`). The legs are small: one crate plus `thiserror` and `embedded-io`
(plus `serialport` on `std` legs). Shared, main-only caches keep the repository's 10 GB cache
budget in check. The slowest path is expected to be the Windows leg (serialport + tests), well
under 15 minutes with a warm cache (SC-003). The time-out turns a hung pty test into a red check
instead of a 6-hour wait.

**To measure after merge**: wall-clock time of the first 5 PR runs (SC-003), and flaky failures
over the first 20 PRs (SC-006).

## R-15 Toolchains

**Decision**: latest stable (`dtolnay/rust-toolchain@stable`) for every job except
compile-fail, which uses `COMPILE_FAIL_TOOLCHAIN` (R-7). No `rust-version` in `Cargo.toml`, no
`rust-toolchain.toml` (FR-010, User Story 6 deferred).

**Rationale**: a `rust-toolchain.toml` would pin every local build too, against FR-010.

## R-16 State of each gate on day one

Checked on scratch copies with `-D warnings`:

| Gate | `dev` today | After PR #7 | After core-1 |
|---|---|---|---|
| no_std: no features, alloc alone | red (`Register::new` never used) | red | green (core-1 fixes the warning) |
| no_std: heap-free all, no_std all | green | green | green |
| feature sets: families alone, defaults, all | green | green | green |
| feature sets: none, `alloc`, `mock`, `std` alone (host); `alloc`, `mock` alone (target) | red (same warning) | red | green |
| missing docs (`[lints]`) | red (2 `UnsupportedModel` fields) | green | green, if core keeps `SetupError` documented |
| docs `-D warnings` | not run | green | expected green |
| compile-fail codes (all three passes) | green | green | expected green |
| tests, all features; clippy `--all-targets --all-features` | not run | green | expected green |
| feature tests (host): `std` and each heap-free feature | green (`std`, `std,ax` checked) | green (all 7) | expected green |
| control tables: schema check | green, no-op (guard: no script yet) | green, no-op | real check after tables T1 |
| Windows, macOS | never run | unknown (R-9) | unknown |

The spec's Assumptions say the no-feature build is green on day one. That probe ran without
`-D warnings`. With it, the build fails, as module-interfaces.md conflict 14 records. So the
`ci` PR lands after PR #7 and core-1.

## R-17 Table check

**Decision**: A `control tables: schema check` job on `ubuntu-latest` runs, with the
preinstalled `python3`, `python3 control_tables/check_tables.py` and
`python3 -m unittest discover -s control_tables -p 'test_*.py'`. Until tables T1 merges, both
are guarded by `if [ -f control_tables/check_tables.py ]`.

**Rationale**: The tables plan (005 R-12, quickstart §1) makes both commands a gate for every
tables PR and hands the wiring to `ci`. Without a CI step, its FR-014 check never runs on PRs.
The scripts use the Python standard library only, so no setup step is needed. The guard keeps the
job green (not skipped by a path filter, FR-013) while the scripts do not exist yet; Python 3.12+
`unittest discover` exits non-zero when it finds no test, so the guard covers both commands.

## R-18 Out of scope

Recorded only, as in the spec's Assumptions: dependency audit and licence check, a manual
hardware job on a self-hosted runner, semver checks, a scheduled run, Miri, update bots, linking
a firmware binary (FR-003 TODO), MSRV (User Story 6). Adding the new gates to the constitution's
list, the PR template and `CLAUDE.md`, and marking checks required in the ruleset, are the
architect's follow-ups.
