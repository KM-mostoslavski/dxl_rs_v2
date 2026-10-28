# Quickstart: validating the CI module

**Feature**: [spec.md](spec.md) | **Checks**: [contracts/ci-checks.md](contracts/ci-checks.md) |
**Feature sets**: [contracts/feature-sets.md](contracts/feature-sets.md)

## Prerequisites

- core-1 merged into `main` (PR #7 is merged; see [research.md § R-16](research.md#r-16-state-of-each-gate-on-day-one)).
  Before that, every build without a model family is red for a reason outside this module.
- `.aiignore` and `.github/workflows/ov-aiignore.yml` committed to `main` by the human.
- Rust stable (1.97.1 on the maintainer's machine), the `thumbv7em-none-eabihf` target,
  `cargo-llvm-cov`, `jq`, and `libudev-dev` on Linux.
- The `1.97.1` toolchain for the compile-fail check: `rustup toolchain install 1.97.1`.
- A GitHub fork of the repository, for scenario 4.

## 1. Run the gates locally

From the repo root, on the ci branch:

```sh
export RUSTFLAGS="-D warnings"

# feature sets, as the workflow derives them (jq program: the `feature-sets` step in ci.yml)
cargo metadata --no-deps --format-version 1 | jq -c --arg pkg dxl_rs_v2 '<program>'
cargo build --lib --no-default-features                       # host: no features
cargo build --lib --no-default-features --features ax         # host: ax alone (repeat per feature)
cargo build --lib                                             # host: defaults
cargo build --lib --all-features                              # host: all features
cargo build --lib --no-default-features --features ax --target thumbv7em-none-eabihf

# feature tests (host): std, then std plus each heap-free feature
cargo test --lib --no-default-features --features std
cargo test --lib --no-default-features --features std,ax    # repeat for mx, x-series, y-series, pro, mock

# no_std build
cargo clippy --no-default-features --features ax,mx,x-series,y-series,pro,mock --target thumbv7em-none-eabihf
cargo build --lib --no-default-features --target thumbv7em-none-eabihf
cargo build --lib --no-default-features --features alloc --target thumbv7em-none-eabihf
cargo build --lib --no-default-features --features ax,mx,x-series,y-series,pro,mock --target thumbv7em-none-eabihf
cargo build --lib --no-default-features --features alloc,ax,mx,x-series,y-series,pro,mock --target thumbv7em-none-eabihf

# clippy and tests with every feature, docs, compile-fail codes (three passes), coverage
cargo clippy --workspace --all-targets --all-features
cargo test --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --no-default-features
RUSTC_BOOTSTRAP=1 cargo +1.97.1 test --doc --all-features
RUSTC_BOOTSTRAP=1 cargo +1.97.1 test --doc
RUSTC_BOOTSTRAP=1 cargo +1.97.1 test --doc --no-default-features --features alloc,ax,mock,mx,pro,x-series,y-series
cargo llvm-cov --workspace --no-report
cargo llvm-cov report --json --summary-only --ignore-filename-regex 'src/models(\.rs|/)' | jq '.data[0].totals.lines'
cargo llvm-cov report --ignore-filename-regex 'src/models(\.rs|/)' --fail-under-lines 70

# table check (after tables T1; before it, the CI step is a guarded no-op)
python3 control_tables/check_tables.py
python3 -m unittest discover -s control_tables -p 'test_*.py'
```

Expected: every command passes. The `jq` line prints 11 host sets, 7 target sets and 7 test sets
for today's `Cargo.toml`.

The third compile-fail pass builds every doc example without `std`. A doc example that uses a
std-only item must be gated with `#[cfg_attr(feature = "std", doc = …)]`, or this pass fails.

## 2. Seeded faults (SC-001)

Each row: push one throwaway branch with only that change, open a draft PR, and check the result.
Close the PR and delete the branch afterwards. Expected for all: only the named check(s) turn red,
and the log names the item. `feature sets` (the aggregate) is red whenever a `feature set (…)` or
`feature tests (…)` leg is red. `src/lib.rs` is `#![no_std]` and declares `alloc` and `std` only
under their features, so a fault without a `cfg` fails on the host as well as on the target.
`RUSTFLAGS: -D warnings` is workflow-wide, so a warning fails every check that compiles the
crate. Red sets F1, F1b, F2, F3, F5, F5b, F10 and F11 were checked on a scratch copy of `main`
with core-1's fix simulated (research R-16).

| # | Story | Seeded change | Expected red check |
|---|---|---|---|
| F1 | US1 AS2 | add `/// Seeded fault.` and `pub fn leak() -> alloc::vec::Vec<u8> { alloc::vec::Vec::new() }` to `src/lib.rs` | `no_std build (thumbv7em-none-eabihf)` (`E0433`, names the line); every `feature set` leg without `alloc`: host `no features` and each heap-free feature alone, thumbv7em each heap-free feature alone; `docs: complete, no warnings` (the `--no-default-features` run); `feature sets`. Green: legs with `alloc` (`alloc`, `std` alone, defaults, all features), the test legs. Today's no_std clippy already catches F1, so F1 is the regression guard; F1b shows the gap |
| F1b | US1 AS1 | the F1 function, gated `#[cfg(not(any(feature = "ax", feature = "mx", feature = "x-series", feature = "y-series", feature = "pro", feature = "mock")))]` | `no_std build (…)` (step `build: no features`), `feature set (host): no features`, `docs: complete, no warnings` (`--no-default-features` run), `feature sets`. Today's workflow stays green: it never builds without a family |
| F2 | US1 AS2 | add `/// Seeded fault.` and `pub fn shout() { std::println!("x"); }` to `src/lib.rs` (no `cfg`) | `no_std build (…)` (every step); every `feature set` leg without `std`: host `no features`, `alloc alone` and each heap-free feature alone, every thumbv7em leg; `docs: complete, no warnings` (`--no-default-features` run); `doc examples: compile-fail error codes` (std-off pass); `feature sets` |
| F3 | US2 AS1-2 | in one `ax`-gated item, use an item gated on `mx`: a documented `#[cfg(feature = "ax")] pub const SEEDED: usize = core::mem::size_of::<crate::models::Mx28>();` | `feature set (host): ax alone`, `feature set (thumbv7em): ax alone`, `feature tests (host): ax`, `feature sets` |
| F4 | US2 AS3 | add `partial-reads = []` to `[features]` in `Cargo.toml` (no code) | nothing red. Three new legs: `feature set (host): partial-reads alone`, `feature set (thumbv7em): partial-reads alone`, `feature tests (host): partial-reads`. `partial-reads` joins `heap_free` and `no_std`, so the no_std job and the std-off compile-fail pass build it too. Counts: 12 host, 8 target, 8 test legs |
| F5 | US4 AS1 | remove the doc comment of one public item every set compiles (e.g. `Id::new` in `src/id.rs`) | every check that compiles the crate: `fmt, clippy, test (Linux)`, `no_std build (…)`, `coverage (>= 70 % lines)`, every `feature set (…)` and `feature tests (…)` leg, `feature sets`, `docs: complete, no warnings`, `doc examples: compile-fail error codes`, both `std build and tests (…)` legs. Green: `generated models up to date`, `control tables: schema check`, `feature sets: list`. Today's `check` already catches F5 (PR #7's `#![warn(missing_docs)]`), so F5 is the regression guard; F5b shows the gap |
| F5b | US4 AS1 | add an undocumented `#[cfg(all(feature = "alloc", not(feature = "std")))] pub fn alloc_only() {}` to `src/lib.rs` | `feature set (host): alloc alone`, `feature set (thumbv7em): alloc alone`, `no_std build (…)` (steps `build: alloc alone`, `build: no_std, all`), `doc examples: compile-fail error codes` (std-off pass), `feature sets`. Today's workflow stays green: it never builds `alloc` without `std` |
| F6 | US4 AS2 | break one intra-doc link (`` [`NoSuchItem`] ``) | `docs: complete, no warnings` |
| F7 | US4 AS3 | change ` ```compile_fail,E0277 ` in `src/transport.rs` to `E0308` | `doc examples: compile-fail error codes` only (stable doc tests stay green) |
| F8 | US5 AS2 | add a type error inside `#[cfg(windows)]` code in `src/transport/serial.rs` | `std build and tests (Windows)` only |
| F9 | US3 AS2 | add a large, documented public function that no test calls, enough to drop line coverage below 70 % | `coverage (>= 70 % lines)` red, and the run summary still shows the figure |
| F10 | US2 AS4 | add `#[cfg(all(test, feature = "mock"))] #[test] fn seeded_all_features() { panic!("seeded"); }` to `src/transport/mock.rs` | `fmt, clippy, test (Linux)` (`cargo test --workspace --all-features`), `feature tests (host): mock`, `feature sets` |
| F11 | US2 AS4 (R-4) | add `#[cfg(all(test, not(feature = "ax")))] mod seeded { #[test] fn family_off() { panic!("seeded"); } }` to `src/lib.rs` | every `feature tests (host)` leg with `ax` off (`std`, `mock`, `mx`, `pro`, `x-series`, `y-series`), `feature sets` |

Every row touches `src/**` (F4: `Cargo.toml`) on a throwaway branch only. They are never
merged.

## 3. Coverage on the PR page (US3, SC-002)

1. Open any PR after the ci PR merges.
2. Click "Details" on `CI / coverage (>= 70 % lines)`.
3. Click "Summary" of the run.

Expected: the "Line coverage" table with the figure, the 70 % threshold and pass/fail. Two
clicks, no log opened.

## 4. Fork PR (FR-011, SC-004)

From a fork, open a PR with a one-line docs change.

Expected: the same list of checks as a branch PR, all run (none "pending" or skipped), the
coverage summary is present, and no job asks for approval of secrets. The workflow's token shows
`Contents: read` in the "Set up job" log.

## 5. Cancel and no path filter (FR-012, FR-013)

1. Push two commits to a PR within a minute.
   Expected: the first run shows "cancelled", the second runs to the end.
2. Open a PR that changes only `specs/` or `docs/`.
   Expected: every check runs. None stays "expected" or "pending".

## 6. Pinned toolchain (US4 AS4)

Look at the `compile-fail` job's "Set up job" / toolchain step.

Expected: it installs `1.97.1`, whatever the latest stable is. Changing the version needs a
commit that edits `COMPILE_FAIL_TOOLCHAIN` in `ci.yml`.

## 7. Time budget (SC-003, SC-006)

After merge, note the wall-clock time of the first 5 PR runs with a warm cache (expected ≤ 15
minutes), and count blocking failures unrelated to the PR over the first 20 PRs (expected ≤ 1).
