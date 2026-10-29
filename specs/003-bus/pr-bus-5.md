Stacked on #8 #9 #10 #25 #26 #27 #28 #29 #30 #31 #32 #33 (branched from `integration`); its diff shrinks once they merge.

## What

bus-5 of the bus module (spec 003, tasks T067–T077, user stories US7 and US8): two opt-in features.

- `partial-reads`: `Bus::read_positions_partial(ids, &mut [Result<Position, Error>])` and `Bus::read_positions_raw_partial(ids, &mut [Result<i32, Error>])`, one result per target.
- `raw-registers`: `ActuatorRef::read_raw(addr, &mut [u8])` and `ActuatorRef::write_raw(addr, &[u8])`, any register by address.

Both are off by default. Builds without them are unchanged.

## Why

SRS step 9 (FR-038, FR-039, FR-041; FR-4.7 and FR-8.1/8.2 as decided by the architect on 2026-10-28):

- A control loop that must keep going when one actuator drops out reads the rest of the chain and sees which one failed, instead of losing the whole cycle.
- A user can reach a register the typed API does not cover (e.g. Return Delay Time, LED, a model-only register) without patching the crate, while a hardware fault in the reply is still caught.

## How

- `group.rs`: the strict read engine becomes `read_each`, shared by strict and partial reads (research R-8 steps 5 and 6). It takes a policy for a failed layout instruction: `Abort` (strict; unchanged behaviour) or `Share` (partial; `#[cfg(feature = "partial-reads")]`). Every target's result still passes `observe` before it reaches the caller's closure. `read_stages` is now a thin strict wrapper over it.
- `partial.rs`: same resolve / count / stage steps as `read_positions`, then `read_each` with `Share`, writing `out[i]` per target. Refusals (`NotOnBus`, `DuplicateId`, `BufferLength`) fail the call with zero bytes and `out` unchanged. Per-ID errors inside a P2 Sync Read (timeout, corrupted reply, status) stay in their own slot; P1 is per slot by construction. The engine is a free function over `S: Slots` and caller-owned stages, so `DynBus` can wrap it (T072/T084).
- `raw.rs`: through `Wire::read_at` / `Wire::write_at` (new crate-internal raw-address variants; `Wire::read` / `write` now delegate to them), so retries apply as for any unicast. Each result passes `observe`. No guard, no limit or mode check, no cache or torque-state change.
- `ActuatorRef`'s fields are `pub(super)` so `raw.rs` (a sibling module) can reach them.
- Tests: 9 in `partial.rs` (P1 silent + overload slot; P2 silent slot, alert slot `Faulted`, failing transport fills every slot of the group, mixed layouts, typed and raw, refusals send nothing, empty targets; `read_each` with a scripted `ReadMany` that fails mid-layout), 8 in `raw.rs` (write while `Off` byte-exact, outside the limits, Torque Enable write leaves `Off`, read while `Faulted`, alert on read and on write → `Faulted`, timeout is not a fault, retries, P1 round trip and address above 255 sends nothing). `tests/bus_no_alloc.rs` gains `p2_partial_reads_never_allocate` (with a timeout slot). A `read_positions_partial` doc example (mock, x-series).
- `compile_fail,E0599` doc examples, built only with the feature off: `read_positions_partial` on `Bus` (src/bus.rs), `read_raw` and `write_raw` on `ActuatorRef`.

## Architecture

Follows `specs/003-bus` (contracts/bus-api.md "`partial-reads` feature", "`raw-registers` feature") and module-interfaces §2.5, §6, D2.

- FR-4.7: one result per actuator inside a P2 sync group; only a failed instruction fails the group (architect 2026-10-28, SRS patch pending in T087).
- FR-8.1/8.2: raw replies run the fault check; raw writes never update the mode or torque state (architect 2026-10-28, research R-21; documented on `write_raw`).
- `DynBus` partial wrappers (T072): skipped, bus-6 is not merged; T084 adds them in bus-6 if this PR merges first.
- Shared-file touches: `Cargo.toml` (`[features]` `partial-reads`, `raw-registers`), `tests/bus_no_alloc.rs` (one new test). `src/lib.rs` is unchanged: the new items are methods. `src/bus.rs` gains two `mod` lines and one `cfg_attr` doc block on `Bus`, placed away from where bus-6 adds its own.

Deviations from the task text, for the architect:

- A failed layout instruction gives its error to the targets of that layout still waiting for a reply, not to every target of the layout: a target whose reply was already read before the transport failed keeps its value. On P2 the transport usually fails before any reply (all slots get the error, as US7-3 expects); on P1 a failing line after the first Read now keeps the first value. Partial reads then go on with the next layout rather than stopping. Question: is that what "a group whose instruction fails gives that error to all its slots" means, or should the earlier replies be overwritten too?
- `ActuatorRef` fields are `pub(super)` (they were private) so the feature module can use them.
- `Wire::read_at` / `write_at` (crate-internal, not in the plan) are the raw-address variants; they are not feature-gated because `read` / `write` call them.

## Checklist
- [x] `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass (also `--features mock`, `--features mock,partial-reads,raw-registers`, each new feature alone, `--all-features`)
- [x] `cargo test` passes, including `cargo test --features mock,partial-reads,raw-registers` and `--all-features`; line coverage ≥ 70 % (`cargo llvm-cov --workspace --features mock,partial-reads,raw-registers`, models excluded: 98.9 % lines; `partial.rs` 99.2 %, `raw.rs` 100 %, `group.rs` 99.1 %)
- [x] `no_std` build passes (`--no-default-features --features ax,mx,rx,x-series,y-series,pro,mock,partial-reads,raw-registers --target thumbv7em-none-eabihf`, clippy `-D warnings`); `partial-reads` and `raw-registers` each alone, host and thumbv7em, with `-D warnings`
- [x] `cargo xtask codegen` leaves `src/models.rs` and `src/models/` unchanged
- [x] Public methods documented (`cargo doc --no-deps` with `-D warnings`, all features and no features)
- [x] Docs updated if behaviour or setup changed
- [x] Family-off leg `cargo test --no-default-features --features std,mock,ax,mx,rx,x-series,y-series`; per-feature lib tests (`std` + each family, `std,partial-reads`, `std,raw-registers`, `std,mock`, `std` alone)
- [x] Doc tests with `RUSTC_BOOTSTRAP=1` on 1.97.1 (so the `E0599` codes are checked): all features, defaults, `std` off (with and without the two features)
- [ ] Manual hardware check (quickstart §4): not run
