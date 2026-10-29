Stacked on #8 (core-1), #9 (tables T1), #10 (ci), #25 (xtask X1), #26 (transport macOS test fix) and #27 (core-2), through the `integration` branch; its diff shrinks to the bus files once they merge.

## What

bus-1, the first of six bus PRs (spec 003, tasks T002–T036; user stories US1, US3, US4). A `Bus<P, T, C = DefaultClock, const N: usize = 32>` that is set up from a declared list, lends a short-lived `ActuatorRef` per actuator, keeps one torque-state byte per actuator, and sees hardware faults on the call that receives them.

- `Config` (timeout, scan window, retries; `const` builder, `Default`), `NoClock`, `DefaultClock` (`StdClock` with `std`, `NoClock` without).
- `Bus::new` (no I/O, `N <= 253` checked at compile time), `into_parts`, `config`, `declare`, `actuators`, `actuator`, `torque_state`, `torque_states`.
- `declare` (FR-008, FR-011…FR-015): checks the list before any byte (`TooManyActuators`, `DuplicateId`, `UnsupportedModel`), then per actuator in `Id` order: ping (`Missing` on timeout), model number (P2 pong, P1 Model Number read; `WrongModel`), firmware where the table depends on it, the two EEPROM limit registers, and the limit mode (one 1-byte Operating Mode / Control Mode read, or `LimitMode::from_angle_limits` for AX, RX and MX on P1). Torque stays `Off`, nothing is written. Any error leaves the list empty.
- `ActuatorRef`: `kind`, `torque_state`, `enable_torque`, `disable_torque`, `set_goal_position[_raw]`, `goal_position[_raw]`, `present_position[_raw]`, `clear_fault`, and `Actuator` (`id`, `name`, `model_number`, `position_scale`, `position_limits[_raw]`).
- Goals: torque guard, then `scale.to_raw` (typed), then `Cache::check_goal` (limits in position mode, nothing in extended modes, `NotPositionMode` otherwise), then `Register::encode`, then one write. Every refusal sends zero bytes.
- Faults (FR-024…FR-029): every reply passes one `observe` rule; a failed torque write applies `disable()` (never `On`, a fault is never dropped); `clear_fault` reads Hardware Error Status, or pings where the model has none.
- Retries (FR-004): only in `src/bus/wire.rs`, only for unicast exchanges, only after a timeout or a corrupted reply.

## Why

SRS steps 4 and 5: the bus is the layer users call. This PR is the MVP of the module: a declared bus that moves one actuator safely (FR-001…FR-029 as far as US1, US3 and US4 reach). Fault handling ships here, not later, so `main` never carries a bus that ignores hardware faults (plan deviation 2).

## How

- `src/bus/wire.rs` is the only file that builds a `protocol::Link` and calls `Exchange`; it owns the retry loop, so `observe` sees only the final result and a fault is marked once.
- `src/bus/slots.rs`: the crate-internal `Slots` trait (`kinds`, `clear`, `push`) over `FixedSlots`, which borrows `Bus`'s `heapless::Vec<ActuatorKind, N>` and `[TorqueState; N]`; lookups are a binary search on `Id`.
- `src/bus/setup.rs`: `declare` and `init_one` are generic over `Slots`, so `DynBus` (bus-6) reuses them.
- `src/bus/observe.rs`: `observe`, `after_torque_write`, `check_clear`.
- `src/bus/testing.rs` (`#[cfg(test)]`): P1/P2 status and instruction builders, checked against the e-Manual examples, and fixtures that script a declared XM430-W350, XL-320, AX-12A or RX-28.
- Tests: 87 bus unit tests on `MockTransport`, byte-exact against the builders. Compile-fail doc examples: two live handles (`E0499`), a handle outliving its bus (`E0597`), and, with `std` off, `Bus::new` without a clock (`E0277`). A runnable module example drives one XM430-W350 through declare, enable, goal and present on the mock.

## Architecture

Follows `specs/003-bus` (plan, data-model, contracts/bus-api.md) and module-interfaces §6, with the plan's recorded deviations:

- Deviation 1: no `Clock` bound on the `Bus` struct; every `impl` bounds `C: Clock`. A no_std bus that names no clock cannot call `new` (`E0277`, doc example).
- Deviation 2: fault observation, retries and `clear_fault` are in bus-1.
- Deviation 5: a failed torque write applies `state.disable()`, so a `Faulted` slot stays `Faulted` until `clear_fault` passes. `disable_torque` also passes `disable()` as the wanted state on success, which keeps a fault (core's table: `Faulted` + disable = `Faulted`).
- Limits follow the operating mode (architect, 2026-10-28). The SRS FR-3.7 / FR-2.6 patch is pending (T087, human).
- The six-PR split was approved on 2026-10-28 (T001).

Shared-file touches: `Cargo.toml` (`heapless = { version = "0.9", default-features = false }`), `Cargo.lock`, and `src/lib.rs` (`mod bus;` and `pub use bus::{ActuatorRef, Bus, Config, DefaultClock, NoClock};` only).

Deviations from the task text, all crate-internal:

- `Slots::split_mut` is not on the trait yet. A trait method borrows the local `FixedSlots`, so `Bus::actuator` could not return the handle; `FixedSlots::into_split(self)` gives the two slices for the bus's lifetime instead. The trait method comes with the first multi-actuator op (bus-3).
- `setup::declare` takes the capacity `N` as a const parameter for its stack list.
- `Wire` also has `read_value` / `write_value` (decode or encode around `read` / `write`).
- Test fixtures take the first flush number and return the next one, so several actuators can be scripted for one `declare`. Test accessors on the mocked bus are free functions: an inherent `impl` on `Bus<_, MockTransport, _>` makes `src/transport.rs`'s `expect(dead_code)` on `mod mock` unfulfilled in test builds without `mock`. That expectation is fragile; raised to the transport owner, not changed here.
- `src/bus.rs`'s module docs are on a private module, so they do not render. The fault and broadcast rules are repeated on `Bus`.

## Checklist
- [x] `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass (also `--features mock` and `--all-features`)
- [x] `cargo test` passes; line coverage ≥ 70 % (`cargo llvm-cov --workspace --features mock`: 98.4 % total; bus files 92.9–100 %)
- [x] `no_std` build passes (`--no-default-features --features ax,mx,rx,x-series,y-series,pro,mock --target thumbv7em-none-eabihf`, clippy `-D warnings`), and every ci feature-set leg (`cargo build --lib`, each feature alone, host and thumbv7em)
- [x] `cargo xtask codegen` leaves `src/models.rs` and `src/models/` unchanged
- [x] Public methods documented (`cargo doc --no-deps` with `-D warnings`, all features and no features)
- [x] Docs updated if behaviour or setup changed
- [x] Per-feature lib tests (`--no-default-features --features std,<family>` for every family, `std` alone, `std,mock`)
- [x] Doc tests, with `RUSTC_BOOTSTRAP=1` so the compile-fail error codes are checked: all features, defaults, `std` off (`alloc,ax,mx,rx,x-series,y-series,pro,mock`)
- [ ] Manual hardware check (quickstart §4): not run
