# Implementation Plan: Bus (setup, single- and multi-actuator access, faults)

**Branch**: `003-bus` (planned on `dev`; one branch per bus PR, see PR split) | **Date**: 2026-10-27, updated 2026-10-28 (architect decisions) | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/003-bus/spec.md`

## Summary

Build the `bus` module: the public entry point of the crate.

- **`Bus<P, T, C = DefaultClock, N = 32>`** owns one wire: the transport, the clock, a `Config`
  (reply timeout, scan wait, retries), up to `N` actuators sorted by `Id`, and one
  `TorqueState` byte per slot in an array beside them. Creating it sends nothing.
- **Setup** fills the list. `declare` pings only the listed IDs. `autoscan` uses the protocol's
  `scan` hook (P2 broadcast ping, P1 sweep) and works without `std`. Each actuator gets its
  model (firmware variant included), its limits read from EEPROM, its limit mode (from the
  Operating Mode or Control Mode register, or from the AX / MX P1 angle limits) and
  `TorqueState::Off`. Any failure leaves the list empty.
- **`ActuatorRef`** is the short-lived single-actuator handle. It holds torque on/off, goal and
  present position (typed and raw), goal read-back and `clear_fault`. The goal check follows
  the limit mode (`Cache::check_goal`). Every refusal sends zero bytes. Every status reply,
  raw ones included, goes through one `observe` rule that marks hardware faults.
- **Multi-actuator ops.** Easy tier (`enable_torque`, `disable_torque`, `write_goal_positions`,
  `read_positions`, raw forms) exists on every protocol. The explicit tier is `sync_write` on
  both protocols and `sync_read` on `Bus<P2, …>` only. Targets are resolved, validated as a whole,
  staged on the stack, then grouped by `(addr, size)`, with one instruction per group. Strict
  reads are transactional.
- **Opt-in features.** `partial-reads` (one result per actuator) and `raw-registers` (sends
  unchecked, replies still observed) add methods. Under `std`, `DynBus` adds an uncapped bus
  without changing `Bus`.

Only `src/bus/wire.rs` talks to the protocol layer. It holds the retry loop. The bus does no
packet work of its own.

## Technical Context

**Language/Version**: Rust stable, edition 2024. No MSRV (architect, 2026-10-27). Development
uses rustc 1.97.1.

**Primary Dependencies**:
- `heapless` 0.9, no default features. New; bus-1 adds it to `Cargo.toml`.
- Existing: `embedded-io` 0.7, `thiserror` 2 (no_std), `serialport` 4.9 (via `SerialTransport`,
  `std` only).
- No new dev-dependency. Compile-fail cases are rustdoc `compile_fail,EXXXX` examples, not
  `trybuild` (module-interfaces conflict 13). `proptest` already comes with PR #7.

**Storage**: N/A. In memory only: `heapless::Vec<ActuatorKind, N>` plus `[TorqueState; N]`.
`DynBus` uses `alloc::Vec`, grown at Setup only.

**Testing**:
- Unit tests in `src/bus/**`, all against `MockTransport` and `MockTime::ticker`. No hardware.
- Byte-exact status- and instruction-packet builders for P1 and P2 in a test-only
  `src/bus/testing.rs`, checked against the e-Manual packet examples (research R-13). Status
  builders script replies; instruction builders give the expected `sent()` bytes.
- `proptest` invariant: no goal byte is sent for a slot that is not `On`.
- Rustdoc `compile_fail` examples for each misuse SC-006 lists.
- Doc examples that use std-only items (`SerialBus`, `DynBus`, `SerialTransport`, `StdClock`) are
  gated with `#[cfg_attr(feature = "std", doc = …)]`. `compile_fail` examples that must fail only
  with a feature or `std` off are gated with `#[cfg_attr(not(feature = "…"), doc = …)]`. ci's
  std-off doc pass builds every example (specs/007-ci contracts).
- `tests/bus_no_alloc.rs`: a counting global allocator around every Cycle call (needs
  `--features mock`; research R-12). bus-4 creates it; bus-5 and bus-6 add cases.
- A family-off leg, `cargo test --no-default-features --features std,mock,ax,mx,x-series,y-series`,
  runs the `UnsupportedModel { feature: Some("pro") }` case (US2-4, quickstart §1).
- The no_std clippy check on `thumbv7em-none-eabihf`, and `cargo llvm-cov` ≥ 70 % per PR.

**Target Platform**: Linux, macOS and Windows (`std`, `SerialBus`, `DynBus`).
`thumbv7em-none-eabihf` without heap (everything else).

**Project Type**: library crate (workspace with `xtask/`).

**Performance Goals**:
- Zero allocation per Cycle in every build (NFR-2).
- Static dispatch only. Slot lookup is a binary search on the sorted list.
- A multi-actuator write to K targets across G layouts sends G instructions. A P2 multi-read
  sends one request per layout; a P1 one sends K (SC-004).
- Bounded waits. A unicast attempt waits at most one reply timeout on a quiet line. Retries
  multiply that by `retries + 1` (SC-008). A P2 autoscan waits one scan wait, a P1 autoscan
  253 scan waits (SC-007). Latency targets wait for the NFR-5 bench.
- Setup reads per actuator: ping (or scan reply), model number on P1, firmware where needed,
  two limit registers, and one 1-byte mode register unless the model derives its mode from the
  angle limits (AX, MX on P1). The limit mode costs no memory (it sits in `Cache` padding).

**Constraints**:
- `no_std`, heap-free. No `dyn`, `Rc`, `RefCell` or stored lifetime in `Bus`.
- No panic on any I/O, protocol or hardware outcome. The only compile-time assertion is
  `N <= 253` (research R-3).
- Paths: `src/bus.rs` and `src/bus/**`. Declared cross-module touches: `src/lib.rs` (bus lines
  only), `Cargo.toml` (`heapless`, the two features) and `tests/bus_no_alloc.rs`.

**Scale/Scope**: one bus type plus `DynBus`, one handle type, 2 protocols, 72 generated model
types behind `ActuatorKind`, and 2 register fields for now (goal and present position, OQ-2).
About 14 source files, delivered in 6 PRs.

### Prerequisites from other modules (all merged before bus-1 unless noted)

| Item | From | Used by |
|---|---|---|
| `ProtocolVersion`, `Protocol::VERSION`, `Exchange::{scan, read_many}`, `P2::sync_read`, `Clock` with `#[diagnostic::on_unimplemented]` | protocol PR #7, merged into `main` 2026-10-28 (module-interfaces §5) | bus-1, bus-2 (`scan`), bus-4 (`read_many`, `sync_read`) |
| `ModelNumber`, `Position`, `PositionScale`, `TorqueState`, `Cache`, `Limits`, `Register` (+ `encode`/`decode`), `ModelData`, `LimitRegisters`, `Describe`, `Actuator` with provided methods | core-1 + core-2 | bus-1 |
| `LimitMode`, `Cache::record_mode`, mode-aware `Cache::check_goal`, the mapping to `LimitMode`: `ModeSource::register()`, `ModeSource::limit_mode(value) -> Option<LimitMode>`, `LimitMode::from_angle_limits(Limits)` (module-interfaces §2.4b, specs/004-core contracts) | core-1 | bus-1 |
| `ValidationError::{ActuatorIsOff, ActuatorFaulted, OutOfLimits, LimitsUnknown, InvertedLimits, NotFinite, DoesNotFit, BufferLength, DuplicateId, NotPositionMode}`, `SetupError` (with `UnknownOperatingMode { id, value }`) | core-1 | bus-1 |
| `ActuatorKind::new(ProtocolVersion, ModelNumber, fw, Id)`, `ActuatorKind::firmware_register`, `models::MODEL_NUMBER`, generated `Describe` impls, `ModelData::mode: ModeSource` (with each model's `ModeValues` lists from its table's `operating_modes`) | xtask X1 (regenerated) | bus-1 |

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Check | Status |
|---|---|---|
| I. no_std and Heap-Free | Storage is `heapless::Vec` + a fixed array. Staging for multi-actuator ops is a stack array of `N` entries. `DynBus` and `SerialBus` are `cfg(feature = "std")`. The no_std clippy build proves the rest cannot allocate. `tests/bus_no_alloc.rs` counts zero allocations per Cycle for `Bus` and `DynBus`, whose Vecs are sized at Setup. No `Rc`, `RefCell` or stored lifetime. Single-actuator access borrows `&mut Bus`. | PASS |
| II. Absent Over Failing | `sync_read` exists only on `impl Bus<P2, …>`. Partial and raw methods exist only with their features, `DynBus`/`SerialBus` only with `std`. A model with no table on the bus protocol, or compiled out, fails Setup with `SetupError::UnsupportedModel`, before any byte for a declared list. `N > 253` is a compile error. A no_std bus without a named clock does not compile. | PASS |
| III. Nothing Hidden | Setup sends no torque write and starts every slot `Off`. Limits come only from EEPROM; a failed read fails Setup. The operating mode is read at Setup; a failed read or an unlisted value fails Setup (`UnknownOperatingMode`), with no fallback mode. A goal while `Off` or `Faulted` returns `ActuatorIsOff`, and a goal in a non-position mode returns `NotPositionMode`, with zero bytes sent. No read-back after writes. Every unicast, sync-read and `raw-registers` reply passes `observe` (raw: architect, 2026-10-28). A failed torque write applies `disable()`: `Off` from `On`/`Off`, and `Faulted` stays `Faulted`. Retries are only the configured count, only on timeout or corruption. Faults at Setup fail Setup. | PASS |
| IV. e-Manual Is the Only Source | The bus adds no register data. It uses generated `ModelData` (mode source included). The behaviour it relies on (limits per operating mode, reboot-only recovery on X-series, AX angle-limit error, P2 Sync Read reply order, alert bit) is cited from the e-Manual in research R-9, R-10 and R-11. The mode-value lists are table data (each table's `operating_modes`, generated into `ModeValues`; architect 2026-10-28), decoded by core and cited in core research R-17. | PASS |
| V. Committed, Data-Driven Codegen | No generated file is edited. The bus reads `ModelData` through `Describe`. | PASS (N/A for edits) |
| VI. Typed, Static, Panic-Free | `Position` and raw `i32` cross the API. `match`-based `ActuatorKind`, no `dyn`. Every fallible path returns `Result<_, Error>`. No `unwrap` outside tests. | PASS |
| VII. DX First | `SerialBus::<P2>::open(path, baud)` and `autoscan()` give a working bus. `bus.enable_torque(bus.ids())` works because `IdSet` is `Copy` (prototype-checked). Every public item is documented. Features only add methods or types. | PASS |
| Technical constraints | Timeouts are enforced by the bus through `Link` and its `Clock`. One bus = one wire, one protocol, one baud. Rust 2018 layout (`bus.rs` + `bus/`). | PASS |
| Workflow: one module per PR | Six PRs for one module. Each also touches `src/lib.rs` (core), and two touch `Cargo.toml`. bus-4 adds `tests/bus_no_alloc.rs`; bus-5 and bus-6 extend it. | **DEVIATION**: see Complexity Tracking |

**Post-design re-check (after Phase 1, and after the 2026-10-28 decisions)**: unchanged.
Everything passes except the recorded workflow deviation. The raw-registers exception to III is
gone: raw replies run `observe` (architect, 2026-10-28). The design added no allocation, `dyn`
or panic path; the limit mode adds one Setup read and no memory. The open items below are
behaviour the architect may change, not gate failures.

## Deviations from module-interfaces.md

1. **No `C: Clock` bound on the `Bus` struct (§6).** The contract writes
   `pub struct Bus<…, C: Clock = DefaultClock>` with `DefaultClock = NoClock` under no_std, and
   `NoClock` is not a `Clock`. rustc checks defaults against struct bounds: a scratch crate
   fails with `E0277: the trait bound NoClock: Clock is not satisfied` at the struct definition,
   in every no_std build (research R-2). The plan declares
   `pub struct Bus<P: Protocol, T: Transport, C = DefaultClock, const N: usize = 32>`, with
   `C: Clock` on every `impl`. The intent is unchanged: a no_std user who does not name a clock
   cannot call `Bus::new`. `ActuatorRef` keeps `C: Clock`, because it has no default.
2. **Six bus PRs, not seven (§7, "bus 1–7").** Fault observation and the retry loop move into
   bus-1. Otherwise `main` would carry a bus that ignores hardware faults between PRs
   (constitution III). `clear_fault` joins bus-1 for the same reason. The proposal's
   separate PR 3 is gone. As a result `partial-reads`/`raw-registers` enter `Cargo.toml` in
   bus-5, not "PR 6".
3. **A file outside the module paths: `tests/bus_no_alloc.rs`.** The lib test binary already
   has the protocol's `#[global_allocator]`, its counter is private, and a second global
   allocator does not compile. An integration test is a separate binary with its own allocator
   (research R-12). It has no owner in §1, so it is flagged here and in the PR.
4. **Additions, not redefinitions** (§6 leaves names to this plan). New public items: `IdSet`,
   the `field` module (`GoalPosition`, `PresentPosition`, `Raw<F>`, `ReadField`, `WriteField`),
   `DynBus` (std), `ActuatorRef::{goal_position, goal_position_raw}`,
   `Bus::{ids, torque_state, torque_states, config}`, and `impl Default for Config`. `lib.rs` gains the
   matching `pub use` lines (§2.8 list plus these). `ValidationError::DuplicateId` is also used
   for two autoscan replies with one ID. The contract names it for targets and declared lists.
5. **A failed torque write applies `disable()`, not `Off` (§2.5).** §2.5 says "Any other error
   gives `Off`". On a `Faulted` slot, a `disable_torque` that times out or gets a bad CRC would
   then drop the fault, and `enable_torque` would pass with no `clear_fault` check. That breaks
   §2.5's own table (`Faulted` --disable--> `Faulted`; `Faulted` → `Off` only via `clear()`,
   FR-028). The plan sets `*state = state.disable()` after a non-hardware-fault error: `Off`
   from `On`/`Off`, `Faulted` kept. A hardware fault still gives `fault()`. FR-027 holds (never
   `On`). module-interfaces §2.5 now says the same (research R-7).
6. **No bus PR touches PR #7's `expect(dead_code)` on `mod protocol;`: core-1 removes it.**
   Checked on `main` (2026-10-28) by building the lib without
   `#[cfg_attr(not(test), expect(dead_code, reason = "exchanges are called by the bus module"))]`:
   the only dead protocol item is `P2::broadcast_ping`. The bus never calls it. Autoscan uses
   `Exchange::scan`, which passes corrupted replies on (module-interfaces conflict 16);
   `broadcast_ping` skips them. The architect decided on 2026-10-28 that core-1 deletes both
   `P2::broadcast_ping` and the attribute, as a protocol touch listed in the core-1 PR
   (module-interfaces §1, §5; specs/004-core research R-19). bus-1 lands after core-2, so no bus
   PR sees either, and the bus edits only its own lines in `src/lib.rs`. This is no longer a
   deviation: module-interfaces §1 says the same since commit 96b317a.
7. **module-interfaces.md and the 2026-10-28 decisions.** Resolved: the contract now carries
   raw `observe` (§2.5), the mode step in Setup (§6), `NotPositionMode` and
   `UnknownOperatingMode` (§2.7), core-1's removal of `P2::broadcast_ping` and PR #7's
   `expect(dead_code)` (§1, §5), and the mode value lists in the tables (§2.4b, §3.1).

## Deviations from the SRS

**Decided by the architect 2026-10-28.** The SRS text still says otherwise; the human patches
it (tasks, Polish).

1. **FR-4.7: one result per actuator inside a P2 sync group.** The SRS says "On P2 a failed
   sync group fails all slots in that group". Decided reading: only a failed instruction
   (`read_many` returns `Err`, e.g. a transport error) gives its error to every slot of the
   group. A silent or corrupt actuator inside a P2 sync read gets its own per-slot error
   (`Timeout`, `Protocol`, `Status`), because PR #7's `sync_read` resolves replies per ID; the
   other slots keep their values. Built in bus-5 (`src/bus/partial.rs`, research R-8 step 6).
2. **FR-8.1: raw replies run `observe`.** A hardware fault on a raw reply sets `Faulted` (FR-6.2,
   constitution III). Raw still bypasses the limits, the torque guard and the cache on what it
   sends. A raw write to Operating Mode or Torque Enable does not update the cached mode or
   torque state; `write_raw` documents it (FR-8.2). Built in bus-5 (`src/bus/raw.rs`, research
   R-21).
3. **FR-3.7 and FR-2.6: limits follow the operating mode.** Setup records the mode after the
   limits; `Cache::check_goal` applies the limits in position mode only, checks the register
   size only in extended or current-based position mode (and MX multi-turn), and refuses every
   goal with `NotPositionMode` in a non-position mode. An unlisted mode value fails Setup with
   `UnknownOperatingMode`. Built in bus-1 (`src/bus/setup.rs`, research R-9).

**Earlier deviations and stale SRS lines.** The full list is in tasks.md (T087). In short: FR-1.1 (`Clock` bound on the struct, deviation 1), FR-1.3
(`DynBus` is its own type), FR-1.4 (scan wait), FR-1.5 (`ActuatorRef<'_, P, T, C>`), FR-2.4
(`SetupError::UnsupportedModel` with `id` and `protocol`), FR-2.7 (a fault at Setup fails
Setup), FR-4.2 (torque Easy tier as unicast), FR-4.5 (plain-value buffer), FR-4.8 (not atomic on
the wire across groups), FR-6.2 (state byte held by the bus), FR-6.4 (new variants), FR-6.5
(ping on P1, enable refused while faulted), 2.1, 3.3, 4.1, §7 and §8 (`Bus<P, T, C, N>`, clock
third, `torque_state()`). Spec Assumptions list them as conflicts 1, 4–6, 9–11.

## Spec points the architect should confirm (no gate impact)

- **Retries on multi-actuator reads.** Retries wrap unicast exchanges made by `Wire`. The
  sequential P1 reads inside `read_many` are one multi-actuator call and are not retried, so P1
  and P2 Easy-tier reads behave alike (research R-5).
- **`disable_torque(ids)` keeps going after an error, `enable_torque(ids)` stops.** Disabling
  as many actuators as possible is the safe side (research R-7).

## Project Structure

### Documentation (this feature)

```text
specs/003-bus/
├── plan.md              # this file
├── research.md          # Phase 0
├── data-model.md        # Phase 1
├── quickstart.md        # Phase 1
├── contracts/
│   └── bus-api.md       # Phase 1: public Rust API
├── checklists/
│   └── requirements.md
└── tasks.md             # /speckit-tasks output
```

### Source Code (repository root)

```text
src/
├── lib.rs                  # + mod bus; pub use bus::{…} lines only (core-owned file)
├── bus.rs                  # Bus, Config, NoClock, DefaultClock, new/into_parts/config,
│                           # actuators/actuator/ids/torque_state(s), module docs (fault
│                           # limitation FR-029, timing formulas), compile_fail examples
└── bus/
    ├── wire.rs             # Wire: builds protocol::Link, retry loop; ONLY caller of Exchange
    ├── slots.rs            # crate-internal Slots trait + FixedSlots (heapless Vec + array)
    ├── observe.rs          # observe(), torque-write outcome rule, clear-fault check
    ├── actuator_ref.rs     # ActuatorRef + Describe/Actuator delegation
    ├── setup.rs            # declare, autoscan, init_one (model, firmware, limits, mode)
    ├── id_set.rs           # IdSet (Copy bitset of 253 IDs)
    ├── field.rs            # pub mod field: GoalPosition, PresentPosition, Raw<F>, sealed traits
    ├── group.rs            # target resolution, whole-call validation, Stage, (addr, size) groups
    ├── multi.rs            # Easy tier + Explicit tier
    ├── partial.rs          # #[cfg(feature = "partial-reads")]
    ├── raw.rs              # #[cfg(feature = "raw-registers")] ActuatorRef::{read_raw, write_raw}
    ├── serial.rs           # #[cfg(feature = "std")] SerialBus alias + open
    ├── dyn_bus.rs          # #[cfg(feature = "std")] DynBus + DynSlots
    └── testing.rs          # #[cfg(test)] P1/P2 status and instruction builders, fixtures
tests/
└── bus_no_alloc.rs         # #![cfg(feature = "mock")] counting allocator around Cycle calls
Cargo.toml                  # + heapless (bus-1); + partial-reads, raw-registers features (bus-5)
```

**Structure Decision**: Rust 2018+ layout (`bus.rs` + `bus/`, no `mod.rs`), all under the SRS
paths `src/bus.rs src/bus/**`, plus the declared touches above. No file of core, protocol,
transport, models or xtask is edited. A missing item there is raised to its owner.

### PR split (≥ 70 % line coverage each, branched from `main`, PR into `main`)

| PR | SRS step | Stories | Content | Needs | Shared-file touches |
|---|---|---|---|---|---|
| bus-1 | 4, 5 | US1, US3, US4 | `Bus`, `Config`, `NoClock`/`DefaultClock`, `Wire` + retries, `Slots`, `observe`, `declare` (model check, firmware, limits, limit mode, protocol reach), `ActuatorRef` (torque, goal per limit mode, present, read-back, `clear_fault`), `torque_state(s)`, compile-fail: two refs, ref outliving bus, no_std bus without clock | core-2 (with `LimitMode`), X1 (with `ModelData::mode`); PR #7 merged | `lib.rs`, `Cargo.toml` (`heapless`) |
| bus-2 | 4, 8 | US2 | `autoscan` P1/P2 (no_std too), duplicate-ID and fault-at-scan handling, `SerialBus` + `open`, family-off test leg | bus-1 | `lib.rs` |
| bus-3 | 6 | US5 | `IdSet`, `field` module, `group.rs`, `enable_torque`/`disable_torque` (ids), `write_goal_positions(_raw)`, `sync_write<F>`, proptest guard invariant | bus-1 | `lib.rs` |
| bus-4 | 6 | US6 | `read_positions(_raw)`, transactional staging, `sync_read<F>` on P2, compile-fail `sync_read` on P1, `tests/bus_no_alloc.rs`, the SC-001 five-line example | bus-2, bus-3 | `lib.rs`, `tests/bus_no_alloc.rs` (new) |
| bus-5 | 9 | US7, US8 | `partial.rs` (one result per actuator, FR-4.7 decided), `raw.rs` (replies run `observe`), compile-fail without features, no-alloc case for `read_positions_partial` | bus-4 | `lib.rs`, `Cargo.toml` (features), `tests/bus_no_alloc.rs` |
| bus-6 | 10 | US9 | `DynBus` on `DynSlots`, no-alloc cases for it, compile-fail without `std` | bus-4 | `lib.rs`, `tests/bus_no_alloc.rs` |

bus-5 and bus-6 are independent of each other. bus-2 and bus-3 are too.

**SC-001 example in bus-4.** The five-line desktop path uses `SerialBus::open` and `autoscan`
(bus-2) and `enable_torque(ids)` (bus-3). bus-4 is the first PR that needs both, so it carries
the doctest and waits for bus-2 as well as bus-3.

**`DynBus` partial reads: whichever of bus-5 and bus-6 merges second.** They need both
`partial.rs` (bus-5) and `DynBus` (bus-6). The second of the two to merge adds
`DynBus::read_positions_partial` / `read_positions_raw_partial` and their no-alloc case
(T072 in bus-5, T084 in bus-6); the first skips its task.

**`expect(dead_code)` on `mod protocol;`** (deviation 6). core-1 removes it together with
`P2::broadcast_ping`, before any bus PR. bus-1 uses `ping`/`read`/`write`, bus-2 `scan`, bus-3
`sync_write`, bus-4 `read_many`/`P2::sync_read`. Every bus PR runs
`cargo clippy --all-targets -- -D warnings` (quickstart §1) and changes nothing in `src/lib.rs`
beyond its own lines; anything wrong in `src/protocol*` is raised to the protocol owner, never
fixed in a bus PR.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| One module in six PRs | Steps 4, 5, 6, 9 and 10 depend on other modules landing in order (core-1, X1, core-2; the protocol hooks are merged). Each PR stays reviewable and reaches 70 % coverage alone. | One PR would sit open until every prerequisite merged and be too large to review. The seven-PR proposal would leave `main` with a bus that ignores faults. |
| `src/lib.rs` (core) touched by every bus PR | `mod bus;` and the `pub use` lines must be in the crate root. Nothing else in `lib.rs` changes; PR #7's `expect(dead_code)` is core-1's to remove (deviation 6). | No alternative (module-interfaces §1). |
| `Cargo.toml` touched (bus-1, bus-5) | `heapless` is the storage. `partial-reads`/`raw-registers` gate API. | Allowed by §1 split ownership. Flagged as a shared-file touch. |
| `tests/bus_no_alloc.rs` outside `src/bus/**` (bus-4 creates it, bus-5 and bus-6 extend it) | A runtime zero-allocation check needs its own global allocator, so its own test binary (R-12). | An in-crate test cannot install a second allocator or read the protocol's private counter. The static no_std proof does not cover `DynBus`. |
| Test-only status and instruction builders duplicate a little protocol code | Protocol's builders and vectors are private test modules (R-13). Byte-exact `sent()` checks need the instruction side too. | Editing protocol is forbidden to this module. The builders are checked against e-Manual examples. |
