Stacked on #8 #9 #10 #25 #26 #27 #28 #29 #30 #31 #32 #33 #34 #35 (branched from `integration`); its diff shrinks once they merge.

## What

bus-6, the last of the six bus PRs (spec 003, tasks T078–T086, user story US9, plus T072 taken over from bus-5): `DynBus<P, T, C = StdClock>`, a bus with no capacity limit for desktops (`std` only). Storage grows at Setup only; no Cycle call allocates.

- Same public methods as `Bus`, same signatures and behaviour, without `N` (contracts/bus-api.md `DynBus`): `new`, `into_parts`, `config`, `declare`, `autoscan`, `actuators`, `ids`, `actuator` (the same `ActuatorRef`), `torque_state`, `torque_states`, `enable_torque`, `disable_torque`, `write_goal_positions[_raw]`, `read_positions[_raw]`, `sync_write`, `sync_read` on `DynBus<P2, …>` only, and with `partial-reads` `read_positions_partial` / `read_positions_raw_partial`.
- Re-exported as `dxl_rs_v2::DynBus` with `std`; absent without it (`compile_fail,E0433` doc example on `Bus`, built only with `std` off).

## Why

SRS step 10 (FR-040, FR-041, FR-043; FR-1.3, spec conflict 10): a desktop user with a long chain (more than 32 actuators, or a count not known at compile time) gets the same API without picking `N`, and keeps the zero-allocation Cycle.

## How

- `src/bus/dyn_bus.rs`: `DynBus` owns three `alloc::Vec`s (actuators, torque states, staging). `DynSlots<'_>` implements the crate-internal `Slots` trait over the first two (`push` grows both, `clear` empties both), so `declare` / `autoscan` run the same `setup.rs` code as `Bus`. After a successful Setup the staging Vec is resized to the actuator count; it is cleared first, so a failed Setup leaves all three Vecs empty.
- Setup runs with capacity `Id::MAX + 1` (253): every ID there is. `TooManyActuators` cannot happen on a `DynBus` (a 254th entry is always a `DuplicateId` first).
- The multi-actuator methods (and, with `partial-reads`, the partial reads over bus-5's `partial::read_partial`) are thin wrappers over bus-3/bus-4's engine (`multi::{torque_on, torque_off, write_field, read_field}`, now `pub(super)`), passing the Setup-sized staging Vec where `Bus` passes its stack `[Stage; N]` (research R-17).
- Tests (9 unit tests, byte-exact on `MockTransport`): US9-1 `dyn_bus_holds_40` (P2 autoscan, 40 pongs in one reply chunk, then 80 limit reads and 40 mode reads on `MockTransport::<'_, 4096, 4096, 128>` → 40 held, ascending, all `Off`); 40 actuators enabled, written and sync-read in one call each; parity with `Bus` through an `on_both!` macro that runs the same calls on a `Bus` and a `DynBus` on identically scripted lines and compares every result and every byte sent: P2 (XM430 ×2 + XL-320) `declare`, `ids`, `actuator` (typed as `ActuatorRef<'_, P2, _, _>`), `torque_state(s)`, `enable_torque`, goals while off, `write_goal_positions(_raw)`, `sync_write`, refusals (duplicate, short buffer, out of limits), `read_positions(_raw)`, `sync_read` (typed and raw), a fault in a sync read, enable while faulted, `disable_torque`; P2 `autoscan`; P1 (AX-12A ×2) declare, torque, writes, reads; with `partial-reads`, `read_positions_partial` / `read_positions_raw_partial` (a silent target, a hardware fault, untouched extra slot, duplicate and short-buffer refusals); failed `declare` and failed `autoscan` leave the three Vecs empty.
- `tests/bus_no_alloc.rs`: `DynBus` P2 and P1 cycles (the same calls as the `Bus` cases, plus `disable_torque` on P2) allocate nothing after `declare`; the P2 case also asserts that `declare` does allocate, so the counter sees the `DynBus` storage. With `partial-reads`, the `DynBus` partial reads allocate nothing either (the same case as bus-5's `Bus` one).
- Merged `origin/integration` after bus-5 (#35) landed there; `tests/bus_no_alloc.rs` (both PRs appended tests) was the only conflict.

## Architecture

Follows `specs/003-bus` (research R-17, data-model `DynBus`, contracts/bus-api.md) and module-interfaces §9. `Bus`'s public API is unchanged (FR-040, FR-041): the only `Bus` edit is a second `compile_fail` example in its `std`-off docs.

Shared-file touches: `src/lib.rs` (`#[cfg(feature = "std")] pub use bus::{DynBus, SerialBus};`, replacing the `SerialBus`-only line), `tests/bus_no_alloc.rs` (three new tests appended, one of them `partial-reads`).

Taken over from bus-5: T072 (the `DynBus` partial wrappers and their no-alloc case), which bus-5 skipped because `DynBus` did not exist yet. T084 is the same work on this side, so both are done here. No ci change (see T089 below).

Deviations from the task text, for the architect:

- **No `Slots::stages` (T081).** The multi-op engine already takes staging from the caller since bus-3 (`stages: &mut [Stage]`), so `Bus` passes its stack array and `DynBus` its Vec without a trait method. A `stages(&mut self)` on `Slots` would also borrow the slots the engine needs at the same time; `DynBus` instead splits its own fields into wire, slots and staging. So `DynSlots` borrows two Vecs, not three (data-model "Slots"), and `DynBus` sizes `stages` after Setup instead of `push` growing it. `FixedSlots` and the engine signatures are unchanged, which keeps bus-5's merge trivial.
- **`E0433`, not `E0412` (T079).** On the pinned 1.97.1, naming `dxl_rs_v2::DynBus` in type position gives `E0425` (rustc folded `E0412` into it). The example calls `dxl_rs_v2::DynBus::<P2, T, _>::new(…)`, which gives `E0433` ("could not find `DynBus`"), one of the two codes the task allows; the same code compiles with `std` on.
- The struct's `no_run` usage example is gated `#[cfg_attr(feature = "std", doc = …)]` as the task says, although the whole module is `std`-only already.
- The engine's four entry functions in `multi.rs` became `pub(super)` (crate-internal, the only `multi.rs` change).
- The 40-actuator test scripts its Setup reads with a local fixture: `testing.rs`'s fixtures take the default `MockTransport<'_>`, and generalising them over the capacities would touch every bus test file.

T089 (ci coverage of the bus legs), checked against `.github/workflows/ci.yml`, no gap, no edit:

- `tests/bus_no_alloc.rs` runs in the `check` job (`cargo test --workspace --all-features`, which includes `mock`).
- The family-off test (`#[cfg(not(feature = "pro"))]` in `setup.rs`) runs in every `feature-test` leg without `pro` (`std,ax`, …, `std,mock`, `std` alone), since lib tests get the mock from `cfg(test)`.
- The `partial-reads` / `raw-registers` legs are generated from `Cargo.toml` by the `feature-sets` job (host and thumbv7em build legs, `std,<feature>` test legs); with bus-5 merged in they run on this PR without a workflow change. `--all-features` (check, docs, compile-fail jobs) covers them together.
- The std-off doctest pass is the `compile-fail` job's "doc tests: std off" step (`--no-default-features --features "$NO_STD"`, every non-std feature).
- Note for the ci owner: the `coverage` job runs `cargo llvm-cov --workspace` without `--features mock`, so `tests/bus_no_alloc.rs` is not counted there (the lib tests are). It does not change the gate result.

## Checklist
- [x] G-full (bus-5 is merged in): `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass (also `--features mock`, `--features mock,partial-reads,raw-registers` and `--all-features`), all with `RUSTFLAGS="-D warnings"` like ci
- [x] `cargo test` passes, including `cargo test --features mock` and `--features mock,partial-reads,raw-registers` (runs `tests/bus_no_alloc.rs`, 7 tests); line coverage ≥ 70 % (`cargo llvm-cov --workspace --features mock,partial-reads,raw-registers`, models excluded: 98.7 % lines; `dyn_bus.rs` 99.4 %, bus files 93.3–100 %)
- [x] `no_std` build passes (`--no-default-features --features ax,mx,rx,x-series,y-series,pro,mock --target thumbv7em-none-eabihf`, also with `partial-reads,raw-registers`, clippy `-D warnings`), so `DynBus` stays out of no_std; every ci feature-set leg (host and thumbv7em, each feature alone including `partial-reads` and `raw-registers`, `std` alone, `alloc` alone, no features)
- [x] `cargo xtask codegen` leaves `src/models.rs` and `src/models/` unchanged
- [x] Public methods documented (`cargo doc --no-deps` with `-D warnings`: all features, defaults, no features)
- [x] Docs updated if behaviour or setup changed
- [x] Family-off leg `cargo test --no-default-features --features std,mock,ax,mx,rx,x-series,y-series`; per-feature lib tests with `-D warnings` (`std` + each family, `std,mock`, `std,partial-reads`, `std,raw-registers`, `std` alone)
- [x] Doc tests with `RUSTC_BOOTSTRAP=1` on 1.97.1 (so `E0433` is checked): all features, defaults, `std` off (`alloc,ax,mx,rx,x-series,y-series,pro,mock`, and with `partial-reads,raw-registers`)
- [ ] Manual hardware check (quickstart §4): not run
