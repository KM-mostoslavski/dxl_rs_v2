Stacked on #32 (bus-3), which is stacked on #8 (core-1), #9 (tables T1), #10 (ci), #25 (xtask X1), #26 (transport macOS test fix), #27 (core-2), #28 (tables T2), #29 (xtask X2), #30 (bus-1) and #31 (bus-2) through the `integration` branch; its diff shrinks to the bus-4 files once they merge.

## What

bus-4 of the bus module (spec 003, tasks T057–T066, user story US6, plus SC-001): read many actuators into the caller's own buffer.

- Easy tier on every protocol: `Bus::read_positions(ids, &mut [Position])` and `Bus::read_positions_raw(ids, &mut [i32])`. Protocol 2 sends one Sync Read per register layout, Protocol 1 one Read per target.
- Explicit tier, Protocol 2 only: `impl Bus<P2, …> { sync_read(field, ids, &mut out) }`, absent on `Bus<P1, …>` (`compile_fail,E0599` doc example).
- `tests/bus_no_alloc.rs`: zero allocations across every Cycle call on `Bus<P2, …>` and `Bus<P1, …>` (SC-005).
- The SC-001 five-line desktop example on `Bus` (`no_run`, `std`): `SerialBus::<P2>::open`, `autoscan`, `enable_torque([ID])`, `set_goal_position`, `present_position`.

## Why

SRS step 6 (FR-031, FR-032, FR-035…FR-037, FR-043, SC-001, SC-004, SC-005, SC-006): a control loop reads a whole chain per cycle with no allocation, and either gets every value or a clear error with its buffer untouched. With bus-3 this closes the cycle: e.g. an RX-28 chain on Protocol 1 runs `write_goal_positions(bus.ids(), &goals)` then `read_positions(bus.ids(), &mut present)` every cycle.

## How

- `group.rs` read staging (research R-8 step 5): `check_out` (`out.len() >= targets`, else `BufferLength`; extra slots untouched), `stage_reads` (each target's register layout), `read_stages`: one `read_many` per layout with a 4-byte stack scratch; the stages are viewed as `&[Cell<Stage>]` so the ID iterator handed to the protocol and the reply callback can share them; a cursor over the group's stages maps each callback (target order, the e-Manual Sync Read reply order, research R-11) to its stage; the decoded tick goes into the stage; every target's result passes `observe`; a `read_many` `Err` (invalid request or failing transport) aborts at once.
- `FirstError` now keeps the target index, so "the first hardware fault, else the first error" is in target order even when layouts are read in another order (e.g. XM430, XL-320, XM430: the XL-320 group is read second but is target 2).
- `multi.rs`: on success the stages are converted (`Position` through the model's scale, or the raw tick) and copied into `out` (transactional); on error `out` is unchanged (FR-035, FR-036).
- `wire.rs`: `Wire::read_many` (`config.timeout`, never retried, research R-5) and the P2-only `Wire::sync_read` over `P2::sync_read`. A small crate-internal `ReadMany` trait (`AnyProtocol` → `read_many`, `SyncRead` → `sync_read`, P2 only) lets the Easy tier and `sync_read` share one engine.
- Tests (15 new unit tests, byte-exact on `MockTransport`): P2 three targets given as 3, 1, 2 → one Sync Read, `out` in target order, extra slot untouched (US6-1, SC-004); P1 three Read packets (US6-2); a timeout leaves `out` unchanged (US6-3); slot 1 silent + slot 3 alert → slot 3's `Status`, slot 3 `Faulted` on that call, `out` unchanged (US6-4, SC-003); fault order across layouts; short `out`, unknown and repeated targets, empty targets → zero bytes (US6-5); typed and raw forms; XM430 + XL-320 → two Sync Reads; a failing transport aborts; `sync_read(PresentPosition | Raw(PresentPosition))` on P2; P1 silent target then overload. `tests/bus_no_alloc.rs` adds three tests (a counter self-check, P2 and P1 cycles through `declare` on the public API). The RX-28 doc example now also reads back.

## Architecture

Follows `specs/003-bus` (contracts/bus-api.md "Easy tier", "Explicit tier") and module-interfaces §6.

- Plan deviation 3: the zero-allocation proof is an integration test, `tests/bus_no_alloc.rs` (new file; shared-file touch). It needs `mock`; ci runs it in `cargo test --workspace --all-features` (the `check` job), and the P2/P1 cases also need `x-series` / `ax`.
- Retries are not applied to multi-reads (research R-5); documented on `read_positions`.
- Shared-file touches: `tests/bus_no_alloc.rs` (new). `src/lib.rs` is unchanged: bus-3 already re-exports `field`.

Deviations from the task text, for the architect:

- The crate-internal `ReadMany` trait in `wire.rs` (not in the plan) selects `read_many` or the P2-only `sync_read` for the shared engine.
- `FirstError::note` takes the target index (bus-3 had arrival order, the same thing for its unicast torque writes).
- The SC-001 example spells the `ID` constant as a `const` `match` on `Id::new(1)` (`Result::ok` is not `const`), and handles `bus.actuator(ID)` being `None` with `ok_or("ID 1 not found")?`, so it returns `Box<dyn std::error::Error>`; still five statements after the constant, each ending in `?`.

## Checklist
- [x] `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass (also `--features mock` and `--all-features`)
- [x] `cargo test` passes, including `cargo test --features mock` (runs `tests/bus_no_alloc.rs`); line coverage ≥ 70 % (`cargo llvm-cov --workspace --features mock`, models excluded: 98.6 % lines; bus files 93.3–100 %)
- [x] `no_std` build passes (`--no-default-features --features ax,mx,rx,x-series,y-series,pro,mock --target thumbv7em-none-eabihf`, clippy `-D warnings`), so every read path builds without `std` or a heap; every ci feature-set leg (host and thumbv7em, each family alone, no features)
- [x] `cargo xtask codegen` leaves `src/models.rs` and `src/models/` unchanged
- [x] Public methods documented (`cargo doc --no-deps` with `-D warnings`, all features and no features)
- [x] Docs updated if behaviour or setup changed
- [x] Family-off leg `cargo test --no-default-features --features std,mock,ax,mx,rx,x-series,y-series`; per-family lib tests (`std` + each family, `std,mock`, `std` alone)
- [x] Doc tests with `RUSTC_BOOTSTRAP=1` on 1.97.1 (so the `E0599` code is checked): all features, defaults, `std` off
- [ ] Manual hardware check (quickstart §4): not run
