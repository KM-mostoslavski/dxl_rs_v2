Stacked on #8 (core-1) and #9 (tables T1); its diff shrinks once they merge.

The branch starts from `integration` (`main` + #8 + #9, at f1983b2). The only commit of its own is `ci: build no_std and every feature set, publish coverage, check docs and error codes`, which changes `.github/workflows/ci.yml` and nothing else.

## What
Hardens `.github/workflows/ci.yml`. CI now builds the library with no_std (not just lints it), builds and tests every feature set, puts the coverage figure on the PR, checks that docs are complete with no warnings and that compile-fail examples fail with the right error codes, runs on Windows and macOS, and runs the control-table schema check.

## Why
SRS module `ci` (NFR-1, NFR-6, FR-7.3); spec `specs/007-ci/spec.md` FR-001 … FR-016. The old workflow only linted one no_std feature set, never built a single feature on its own, never ran rustdoc, and ran on Linux only.

## How
- **`feature sets: list`** reads `cargo metadata --no-deps` and uses an inline `jq` program to turn `[features]` into one build matrix (host: no features, each feature alone, defaults, all features; thumbv7em: each feature that doesn't enable `std`), one test leg per heap-free feature, and the `heap_free` / `no_std` lists. Adding a feature to `Cargo.toml` adds its legs with no workflow edit. If the input is empty, or the package or the lists are missing, the job fails (`jq -e` plus a guard), so it can never pass with an empty list.
- **`no_std build (thumbv7em-none-eabihf)`**: clippy plus four `cargo build --lib`: no features, `alloc` alone, every heap-free feature, every no_std feature.
- **`feature set (…)` / `feature tests (host): …`** matrix legs, plus one aggregate check, **`feature sets`** (`if: always()`, red unless every leg succeeded), which can be marked required without listing each leg.
- **`fmt, clippy, test (Linux)`** (renamed `check`) adds `clippy --all-features` and `test --workspace --all-features`.
- **`coverage (>= 70 % lines)`** runs in three steps: tests with `--no-report`, then the run summary table and a `::notice`, then the gate. The figure shows even when the gate fails.
- **`docs: complete, no warnings`** runs `cargo doc --no-deps` with `RUSTDOCFLAGS=-D warnings`, with all features and with none.
- **`doc examples: compile-fail error codes`** runs on pinned `1.97.1` with `RUSTC_BOOTSTRAP=1`, set only on its three doc-test steps: all features, defaults, std off.
- **`std build and tests (Windows|macOS)`**.
- **`control tables: schema check`**: `check_tables.py` plus its unit tests.
- Workflow level: `permissions: contents: read`, `persist-credentials: false` on every checkout, `timeout-minutes: 20` on every job, stale PR runs cancelled. No `paths` filter, no secrets, no `pull_request_target`. Matrix and `needs` values reach scripts only through `env:`.

## Architecture
- One missing-docs switch: PR #7's `#![warn(missing_docs)]` in `src/lib.rs` plus the workflow-wide `-D warnings`. No `[lints]` table and no `Cargo.toml` change (FR-015). Deviation from module-interfaces.md § 1 ("Known `Cargo.toml` touches"), as recorded in plan.md.
- The FR-002 feature set is computed from `Cargo.toml` and not hard-coded. With RX-28 (#9) it is `heap_free = ax,mock,mx,pro,rx,x-series,y-series` and `no_std = alloc,` + that list, so `rx` is in the no_std set (module-interfaces.md § 9) with no edit here.
- **Deviation:** the two split `cargo llvm-cov report` calls carry `--workspace`. Without it, `report` drops the `xtask` sources and the figure goes from 97.11 % to 98.39 %. With it, the split figure equals the old one-shot command exactly (FR-001: no gate weakened).
- `control tables: schema check` has no `if [ -f check_tables.py ]` guard, because T1 (#9) is already in this branch's base (T036 rule). It is green on the current `integration` (f1983b2): 73 tables, 0 errors, 107 unit tests OK.
- FR-003 (firmware link test) and US6 (MSRV) are deferred.
- Precondition not met: `.aiignore` and `.github/workflows/ov-aiignore.yml` are not on `main` yet (human-only paths). This PR neither touches nor duplicates them.

## Local results (RUSTFLAGS="-D warnings")
- Feature-set program: 12 host legs, 8 thumbv7em legs, 8 test legs (11/7/7 in the contract, plus `rx`). With a scratch `serial-extra = ["std"]` and `partial-reads = []` added: 14/9/9. `partial-reads` gets both build legs and a test leg; `serial-extra` gets no target leg. Empty input exits non-zero.
- Every build leg, every test leg, all-features clippy and tests, the no_std clippy and its four builds, both `cargo doc` runs, the three compile-fail passes, the coverage steps (97.11 %, 4065 / 4186 lines, same as before) and codegen all pass.
- Seeded faults, run locally on throwaway branches:

| Fault | Old workflow | Red with this PR (locally) |
|---|---|---|
| F1 `alloc` without cfg | red (no_std clippy) | no_std builds without `alloc`; host `no features` and every heap-free leg; thumbv7em heap-free legs; `cargo doc --no-default-features`. `alloc alone` green |
| F1b `alloc` with every family off | **green** | `build: no features`, host `no features`, `cargo doc --no-default-features` |
| F2 `std` without cfg | red | every no_std step, every leg without `std`, `cargo doc --no-default-features`, std-off compile-fail pass |
| F3 `ax` item uses `Mx28` | **green** | host/thumbv7em `ax alone`, `feature tests (host): ax` |
| F4 new feature `partial-reads` | n/a | nothing red; 3 new legs present |
| F5 `Id::new` undocumented | red | every check that compiles the crate |
| F5b undocumented item only with `alloc` and without `std` | **green** | host/thumbv7em `alloc alone`, `build: alloc alone`, `build: no_std, all`, std-off compile-fail pass |
| F6 broken intra-doc link | **green** | `docs` only |
| F7 `E0277` → `E0308` | **green** | compile-fail passes only |
| F8 type error in `#[cfg(windows)]` | **green** | `cargo check --target x86_64-pc-windows-gnu` (E0308) |
| F10 test only with `mock` | **green** | `test --workspace --all-features`, `feature tests (host): mock` |
| F11 test only with `ax` off | **green** | every test leg with `ax` off (std, mock, mx, pro, rx, x-series, y-series) |

## On GitHub
- **`std build and tests (macOS)` is red**, and this is a real finding, not a CI bug: two pty tests in `src/transport/serial.rs` (`pty_tests::open_configures_8n1_without_flow_control` and `open_busy_port_is_no_device`) fail on macOS with `Not a typewriter` (ENOTTY) when they reopen the slave path after dropping the slave. All other 181 tests pass. The fix belongs to the transport module (plan.md: the ci PR does not skip the test or make the job non-blocking). Every other check is green (38 of 39).
- Seeded faults (T040) ran as draft PRs #11–#23. Each turned red exactly the checks listed in the local table above, with the log naming the item (e.g. F1: `error[E0433] … --> src/lib.rs:47:18`). There were no extra reds apart from macOS, which is red everywhere for the reason above. F4 added the three `partial-reads` legs, all green. F5 turned red every compiling check, Windows included. F8 turned red only Windows. F9 turned red only coverage, and the summary still showed `67.87 % (threshold 70 %)`.
- Probe #24 (a change only under `specs/`) ran every check (FR-013). Its first run was cancelled by a second push 30 s later (FR-012). "Set up job" shows `Contents: read` (FR-011). The compile-fail job installs `rustc 1.97.1` (US4 AS4). The coverage table is on the run summary of `coverage (>= 70 % lines)` ("Details" → "Summary", SC-002), plus a `::notice` (97.11 %).
- Not done: the fork PR of T041 (creating a fork under the Ovsiankina account was not permitted in this session). Contents: read was checked on in-repo PRs only.
- Wall clock (SC-003): about 1 min 30 s for the whole run, cold and warm alike (1:26 first, 1:32 on re-run), well under 15 min.

## Required checks (for the architect, not set by this PR)
`fmt, clippy, test (Linux)`, `no_std build (thumbv7em-none-eabihf)`, `coverage (>= 70 % lines)`, `generated models up to date`, `feature sets`, `docs: complete, no warnings`, `doc examples: compile-fail error codes`, `control tables: schema check`, `std build and tests (Windows)`, `std build and tests (macOS)`. Do not mark the individual `feature set (…)` / `feature tests (…)` legs: their names follow `Cargo.toml`.

## Follow-ups for the architect
- T043: mark the checks above as required in `.github/ov-ruleset-main.json` (not in the repo yet).
- T044: add the new gates to the constitution's quality-gate list, the PR template checklist and the Commands section of `CLAUDE.md` (its no_std command should list `rx`).
- T045: track SC-003 (≤ 15 min warm) over the first 5 runs and SC-006 over the first 20 PRs; decide whether to pin actions by SHA.
- Drop `[lints.rust] missing_docs` from module-interfaces.md § 1.
- Commit `.aiignore` and `ov-aiignore.yml` to `main`.

## Checklist
- [x] `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass
- [x] `cargo test` passes; line coverage ≥ 70 % (`cargo llvm-cov`): 97.11 %
- [x] `no_std` build passes (now built for every no_std feature set)
- [x] `cargo xtask codegen` leaves `src/models.rs` and `src/models/` unchanged
- [x] Public methods documented (no Rust change)
- [ ] Docs updated if behaviour or setup changed: the PR template, `CLAUDE.md` and the constitution are architect-owned (T044)
