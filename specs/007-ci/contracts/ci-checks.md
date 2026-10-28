# Contract: CI checks

**Feature**: [../spec.md](../spec.md) | **Model**: [../data-model.md](../data-model.md) |
**Research**: [../research.md](../research.md)

This is the surface the ci PR delivers: one workflow file and its checks. `Cargo.toml` is not
changed. Check names, commands, triggers and permissions are the contract. Step order inside
a job, cache keys and YAML layout are not, except where a row says so.

## Workflow `.github/workflows/ci.yml`

| Setting | Value |
|---|---|
| `name` | `CI` |
| `on` | `pull_request` (all branches, no `paths` filter); `push` to `main` |
| `permissions` | `contents: read` (top level; no job widens it) |
| `concurrency` | group `ci-<PR number or ref>`; `cancel-in-progress` only for `pull_request` |
| `env` | `CARGO_TERM_COLOR: always`, `RUSTFLAGS: -D warnings`, `COMPILE_FAIL_TOOLCHAIN: "1.97.1"` |
| Runners | GitHub-hosted only (`ubuntu-latest`, `windows-latest`, `macos-latest`) |
| Checkout | `actions/checkout@v4` with `persist-credentials: false` |
| Job time-out | `timeout-minutes: 20` on every job |

`ov-aiignore.yml` is not touched and not duplicated. The human commits it, with `.aiignore`, to
`main` before the ci PR (precondition, spec Assumptions).

## Checks

Shown on the PR as `CI / <check name>`. "Kept" rows must keep every listed command (FR-001).

| Job id | Check name | Runs on | Commands (all must pass) | Proves |
|---|---|---|---|---|
| `check` (kept, renamed) | `fmt, clippy, test (Linux)` | ubuntu, stable | `cargo fmt --check`; `cargo clippy --workspace --all-targets`; same `--features mock`; `cargo test --workspace`; `cargo test --features mock --doc`; **new:** `cargo clippy --workspace --all-targets --all-features`; **new:** `cargo test --workspace --all-features` | FR-001, FR-005, FR-009 (Linux) |
| `no-std` (kept, extended) | `no_std build (thumbv7em-none-eabihf)` | ubuntu, stable + target | `cargo clippy --no-default-features --features $HEAP_FREE --target thumbv7em-none-eabihf`; `cargo build --lib --no-default-features --target thumbv7em-none-eabihf` with: no features; `--features alloc`; `--features $HEAP_FREE`; `--features $NO_STD` | FR-001, FR-002 |
| `coverage` (kept, extended) | `coverage (>= 70 % lines)` | ubuntu, stable | `cargo llvm-cov --workspace --no-report`; `cargo llvm-cov report --json --summary-only --ignore-filename-regex 'src/models(\.rs\|/)'` → run summary + notice; `cargo llvm-cov report --ignore-filename-regex 'src/models(\.rs\|/)' --fail-under-lines 70` | FR-001, FR-006 |
| `codegen` (kept, unchanged) | `generated models up to date` | ubuntu, stable | `cargo xtask codegen`; `git add --intent-to-add src/models.rs src/models`; `git diff --exit-code -- src/models.rs src/models` | FR-001 (FR-7.3) |
| `feature-sets` (new) | `feature sets: list` | ubuntu, stable | `cargo metadata --no-deps --format-version 1` piped to the [feature-set rule](feature-sets.md); outputs `matrix`, `tests`, `heap_free`, `no_std` | FR-004 |
| `feature-set` (new, matrix) | `feature set (<where>): <name>` | ubuntu, stable (+ target for `thumbv7em`) | `cargo build --lib $ARGS` | FR-004, FR-007 (missing docs per set) |
| `feature-test` (new, matrix) | `feature tests (host): <name>` | ubuntu, stable | `cargo test --lib $ARGS`; one leg per entry of `tests`: `--no-default-features --features std,<f>` for every `f` in `heap_free`, plus `--no-default-features --features std` | FR-005 (crate tests that compile only with a family off, e.g. generated `src/models/tests.rs`) |
| `features` (new) | `feature sets` | ubuntu | `if: always()`; fails unless `feature-sets`, every `feature-set` leg and every `feature-test` leg succeeded | FR-004, FR-005 (one stable name to mark required) |
| `docs` (new) | `docs: complete, no warnings` | ubuntu, stable | with `RUSTDOCFLAGS: -D warnings`: `cargo doc --no-deps --all-features`; `cargo doc --no-deps --no-default-features` | FR-007 |
| `compile-fail` (new) | `doc examples: compile-fail error codes` | ubuntu, `COMPILE_FAIL_TOOLCHAIN` | with `RUSTC_BOOTSTRAP=1` on these steps only, three passes: `cargo test --doc --all-features`; `cargo test --doc` (defaults); `cargo test --doc --no-default-features --features $NO_STD` | FR-008 (also the examples that compile only with a feature or `std` off) |
| `control-tables` (new) | `control tables: schema check` | ubuntu, preinstalled `python3` | `python3 control_tables/check_tables.py`; `python3 -m unittest discover -s control_tables -p 'test_*.py'`; both guarded by `if [ -f control_tables/check_tables.py ]` until tables T1 merges | FR-016 (tables spec 005 FR-014) |
| `os` (new, matrix) | `std build and tests (Windows)`, `std build and tests (macOS)` | `windows-latest`, `macos-latest`, stable | `cargo build -p dxl_rs_v2`; `cargo test -p dxl_rs_v2` | FR-009 |

`$HEAP_FREE`, `$NO_STD` and `$ARGS` come from the `feature-sets` job outputs and reach scripts
through `env:` only. `no-std`, `feature-set`, `feature-test` and `compile-fail` depend on
`feature-sets` (`needs:`). Every other job starts at once.

The std-off doc pass of `compile-fail` builds every doc example without `std`. So every doc
example that uses a std-only item (`SerialTransport`, `SerialBus`, `DynBus`, `StdClock`, …) is
gated with `#[cfg_attr(feature = "std", doc = …)]`. A ` ```compile_fail ` example that must fail
only with a feature or `std` off is gated with `#[cfg_attr(not(feature = "…"), doc = …)]`. The
three passes together run every such example once.

### System packages

A named step `Install libudev (serialport)` (`sudo apt-get update && sudo apt-get install -y
libudev-dev`) runs before any Linux build that enables `std`: `check`, `coverage`, `docs`,
`compile-fail`, `feature-test` legs, and `feature-set` legs with `needs_std: true`. Windows and macOS need none.

### Run summary of `coverage`

Written to `$GITHUB_STEP_SUMMARY` before the gate step, pass or fail:

```markdown
### Line coverage

| Lines covered | Threshold | Result |
|---|---|---|
| 81.42 % (1234 / 1516) | 70 % | pass |

Generated models (`src/models.rs`, `src/models/`) excluded.
```

Plus one workflow command: `::notice title=Line coverage::81.42 % (threshold 70 %)`.

### Required-check guidance (for the architect, not set by this PR)

Mark as required: `fmt, clippy, test (Linux)`, `no_std build (thumbv7em-none-eabihf)`,
`coverage (>= 70 % lines)`, `generated models up to date`, `feature sets`,
`docs: complete, no warnings`, `doc examples: compile-fail error codes`,
`control tables: schema check`, `std build and tests (Windows)`, `std build and tests (macOS)`.
`.github/ov-ruleset-main.json` is not in the repository yet; the architect creates it.
Not the individual `feature set (…)` or `feature tests (…)` legs: their names follow
`Cargo.toml`.

## `Cargo.toml`

Not changed (FR-015). The missing-docs lint is `#![warn(missing_docs)]` in `src/lib.rs` (core's
line, on `main` since PR #7); `RUSTFLAGS: -D warnings` makes it fail every check that compiles
the crate (R-5). No `[lints]` table, no `rust-version` (FR-010), and no feature, dependency or
`[package]` field is added, removed or renamed.

## Reproduce a check locally

| Check | Command (Linux, from the repo root) |
|---|---|
| `feature set (host): <f> alone` | `RUSTFLAGS="-D warnings" cargo build --lib --no-default-features --features <f>` |
| `feature set (thumbv7em): <f> alone` | same, plus `--target thumbv7em-none-eabihf` |
| `feature tests (host): <f>` | `RUSTFLAGS="-D warnings" cargo test --lib --no-default-features --features std,<f>` (`std` leg: `--features std`) |
| `no_std build` | `RUSTFLAGS="-D warnings" cargo build --lib --no-default-features --features ax,mx,x-series,y-series,pro,mock --target thumbv7em-none-eabihf` (and the other three sets) |
| `docs` | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` |
| `compile-fail` | `RUSTC_BOOTSTRAP=1 cargo +1.97.1 test --doc --all-features`, then the same with no feature flag, then with `--no-default-features --features alloc,ax,mock,mx,pro,x-series,y-series` (today's `no_std` list) |
| `control tables` | `python3 control_tables/check_tables.py && python3 -m unittest discover -s control_tables -p 'test_*.py'` |
| `fmt, clippy, test (Linux)` | as listed in the row above; the new clippy line: `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets --all-features` |
| `coverage` | `cargo llvm-cov --workspace --ignore-filename-regex 'src/models(\.rs\|/)' --fail-under-lines 70` |
