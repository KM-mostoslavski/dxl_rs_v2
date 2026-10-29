Stacked on #8 (core-1), #9 (tables T1), #10 (ci), #25 (xtask X1), #26 (transport macOS test fix), #27 (core-2), #28 (tables T2), #29 (xtask X2), #30 (bus-1) and #31 (bus-2), through the `integration` branch; its diff shrinks to the bus-3 files once they merge.

## What

bus-3 of the bus module (spec 003, tasks T045–T056, user story US5): command many actuators in one call.

- Easy tier on every protocol: `Bus::enable_torque(ids)`, `Bus::disable_torque(ids)`, `Bus::write_goal_positions(ids, &[Position])`, `Bus::write_goal_positions_raw(ids, &[i32])`.
- Explicit tier: `Bus::sync_write(field, ids, values)` with the public `field` module (`GoalPosition`, `PresentPosition`, `Raw<F>`, sealed `ReadField` / `WriteField`).
- `IdSet` (a `Copy` 32-byte bitset, ascending) and `Bus::ids()`, so `bus.enable_torque(bus.ids())` compiles.

## Why

SRS step 6 (FR-030…FR-034, SC-002, SC-004): a control loop moves a chain with one Sync Write per register layout, and a refusal sends nothing to anyone. With bus-1/bus-2 this drives e.g. an RX-28 chain on Protocol 1: `enable_torque(bus.ids())`, then `write_goal_positions(bus.ids(), &goals)` each cycle.

## How

- `group.rs`, one N-free engine over slices (research R-8): `resolve` (targets in caller order; `NotOnBus`, `DuplicateId` via an `IdSet`), `check_values` (`BufferLength`), `validate_writes` (torque guard → `scale.to_raw` for typed values → the target's mode-aware `Cache::check_goal` → `Register::encode` into an 8-byte `Stage`), `groups` (distinct `(addr, size)` in first-appearance order) and `entries` (a `Clone` iterator of `(Id, &[u8])` per group). `Bus` stages on a stack `[Stage; N]`.
- `multi.rs`: goal writes resolve, count, validate, then call `Wire::sync_write` once per group; no `observe` (no reply). Torque: the whole list is checked first (unknown, repeat, and any `Faulted` target for enable), then one acknowledged unicast write per target in target order through `Wire::write` (retried like any unicast), each result through `after_torque_write`. `enable_torque` stops at the first error; `disable_torque` tries every target and returns the first hardware fault, else the first error (`FirstError`, reused by bus-4's reads).
- `Wire::sync_write` uses `config.timeout` (drain only) and is never retried (research R-5).
- Tests (43 new unit tests and 3 doc examples, byte-exact on `MockTransport`): XM430 (116/4) + XL-320 (30/2) → exactly two Sync Writes (SC-004); one target `Off`/`Faulted` → `ActuatorIsOff` naming it, zero bytes (SC-002); velocity mode → `NotPositionMode`, extended mode outside the EEPROM limits passes (US5-6); short slice, unknown, repeated, NaN → zero bytes; empty targets → `Ok`, zero bytes; `sync_write(GoalPosition | Raw(GoalPosition))` on P1 (AX-12A) and P2; torque order, refusal while faulted, stop on first error, disable-all with a fault first; an RX-28 chain on P1; proptest `no_goal_byte_for_a_slot_not_on` (1–8 actuators, random states and goals); `IdSet` and `Stage` sizes; sync-write builders checked against the e-Manual vectors.

## Architecture

Follows `specs/003-bus` (contracts/bus-api.md "Easy tier", "Explicit tier", "Field markers", "IdSet") and module-interfaces §6.

- Plan deviation 4: new public items `IdSet` and `field`.
- Torque Easy tier as acknowledged unicast writes, not a Sync Write (spec Assumptions, research R-7).
- Goal validation follows each target's limit mode (architect 2026-10-28).
- Shared-file touch: `src/lib.rs` (`pub use bus::{…, IdSet, IdSetIter, …, field}` only).

Deviations from the task text, for the architect:

- `Slots::split_mut` is now on the trait (bus-1 had only `FixedSlots::into_split`, see pr-bus-1); `into_split` stays for `Bus::actuator`, which needs the bus lifetime.
- `IdSetIter` is re-exported from the crate root besides `IdSet`: it is `IdSet`'s public `IntoIter` type, and the contract's re-export list omits it.
- `ReadField::Value` / `WriteField::Value` are bounded `Copy + sealed::Value` (contract: `Copy`). The sealed trait carries the value-to-tick conversion; users cannot name or implement it, so the public surface is the same.
- `observe::is_hardware_fault` now takes `&Error` (shared with `FirstError`).
- Test fixtures gained `hold` (puts a model with given limits, mode and torque state straight into the slots) and `ack`, and Sync Write builders `p1_sync_write` / `p2_sync_write`.
- Multi-write doc example (T054) is on `write_goal_positions` with two RX-28 on P1, gated on `mock` and `rx`.

## Checklist
- [x] `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass (also `--features mock` and `--all-features`)
- [x] `cargo test` passes; line coverage ≥ 70 % (`cargo llvm-cov --workspace --features mock`, models excluded: 98.5 % lines; bus files 80.0–100 %, `field.rs` 80 % because `PresentPosition` is first read in bus-4)
- [x] `no_std` build passes (`--no-default-features --features ax,mx,rx,x-series,y-series,pro,mock --target thumbv7em-none-eabihf`, clippy `-D warnings`), and every ci feature-set leg (host and thumbv7em, each family alone, no features)
- [x] `cargo xtask codegen` leaves `src/models.rs` and `src/models/` unchanged
- [x] Public methods documented (`cargo doc --no-deps` with `-D warnings`, all features and no features)
- [x] Docs updated if behaviour or setup changed (module docs: multi-writes validate first and are not atomic on the wire)
- [x] Family-off leg `cargo test --no-default-features --features std,mock,ax,mx,rx,x-series,y-series`; per-family lib tests (`std` + each family, `std,mock`, `std` alone)
- [x] Doc tests with `RUSTC_BOOTSTRAP=1` on 1.97.1: all features, defaults, `std` off
- [ ] Manual hardware check (quickstart §4): not run
