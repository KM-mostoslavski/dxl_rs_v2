# Implementation Plan: Bus (setup, single- and multi-actuator access, faults)

**Branch**: `003-bus` (planned on `dev`; one branch per bus PR, see PR split) | **Date**: 2026-10-27 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/003-bus/spec.md`

## Summary

Build the `bus` module: the public entry point of the crate.

- **`Bus<P, T, C = DefaultClock, N = 32>`** owns one wire: the transport, the clock, a `Config`
  (reply timeout, scan wait, retries), up to `N` actuators sorted by `Id`, and one
  `TorqueState` byte per slot in an array beside them. Creating it sends nothing.
- **Setup** fills the list. `declare` pings only the listed IDs. `autoscan` uses the protocol's
  `scan` hook (P2 broadcast ping, P1 sweep) and works without `std`. Each actuator gets its
  model (firmware variant included), its limits read from EEPROM and `TorqueState::Off`. Any
  failure leaves the list empty.
- **`ActuatorRef`** is the short-lived single-actuator handle. It holds torque on/off, goal and
  present position (typed and raw), goal read-back and `clear_fault`. Every refusal sends zero
  bytes. Every status reply goes through one `observe` rule that marks hardware faults.
- **Multi-actuator ops.** Easy tier (`enable_torque`, `disable_torque`, `write_goal_positions`,
  `read_positions`, raw forms) exists on every protocol. The explicit tier is `sync_write` on
  both protocols and `sync_read` on `Bus<P2, …>` only. Targets are resolved, validated as a whole,
  staged on the stack, then grouped by `(addr, size)`, with one instruction per group. Strict
  reads are transactional.
- **Opt-in features.** `partial-reads` and `raw-registers` add methods. Under `std`, `DynBus`
  adds an uncapped bus without changing `Bus`.

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
- Byte-exact status-packet builders for P1 and P2 in a test-only `src/bus/testing.rs`, checked
  against the e-Manual packet examples (research R-13).
- `proptest` invariant: no goal byte is sent for a slot that is not `On`.
- Rustdoc `compile_fail` examples for each misuse SC-006 lists.
- Doc examples that use std-only items (`SerialBus`, `DynBus`, `SerialTransport`, `StdClock`) are
  gated with `#[cfg_attr(feature = "std", doc = …)]`. `compile_fail` examples that must fail only
  with a feature or `std` off are gated with `#[cfg_attr(not(feature = "…"), doc = …)]`. ci's
  std-off doc pass builds every example (specs/007-ci contracts).
- `tests/bus_no_alloc.rs`: a counting global allocator around every Cycle call (needs
  `--features mock`; research R-12).
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

**Constraints**:
- `no_std`, heap-free. No `dyn`, `Rc`, `RefCell` or stored lifetime in `Bus`.
- No panic on any I/O, protocol or hardware outcome. The only compile-time assertion is
  `N <= 253` (research R-3).
- Paths: `src/bus.rs` and `src/bus/**`. Declared cross-module touches: `src/lib.rs` (bus lines,
  plus removing PR #7's `expect(dead_code)` on `mod protocol;` once, see PR split),
  `Cargo.toml` (`heapless`, the two features) and `tests/bus_no_alloc.rs`.

**Scale/Scope**: one bus type plus `DynBus`, one handle type, 2 protocols, 72 generated model
types behind `ActuatorKind`, and 2 register fields for now (goal and present position, OQ-2).
About 14 source files, delivered in 6 PRs.

### Prerequisites from other modules (all merged before bus-1 unless noted)

| Item | From | Used by |
|---|---|---|
| `ProtocolVersion`, `Protocol::VERSION` | protocol (PR #7 + module-interfaces §5) | bus-1 |
| `Exchange::scan` | protocol §5 | bus-2 |
| `Exchange::read_many` | protocol §5 | bus-4 |
| `ModelNumber`, `Position`, `PositionScale`, `TorqueState`, `Cache`, `Limits`, `Register` (+ `encode`/`decode`), `ModelData`, `LimitRegisters`, `Describe`, `Actuator` with provided methods | core-1 + core-2 | bus-1 |
| `ValidationError::{ActuatorIsOff, ActuatorFaulted, OutOfLimits, LimitsUnknown, InvertedLimits, NotFinite, DoesNotFit, BufferLength, DuplicateId}`, `SetupError` | core-1 | bus-1 |
| `ActuatorKind::new(ProtocolVersion, ModelNumber, fw, Id)`, `ActuatorKind::firmware_register`, `models::MODEL_NUMBER`, generated `Describe` impls | xtask X1 (regenerated) | bus-1 |

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Check | Status |
|---|---|---|
| I. no_std and Heap-Free | Storage is `heapless::Vec` + a fixed array. Staging for multi-actuator ops is a stack array of `N` entries. `DynBus` and `SerialBus` are `cfg(feature = "std")`. The no_std clippy build proves the rest cannot allocate. `tests/bus_no_alloc.rs` counts zero allocations per Cycle for `Bus` and `DynBus`, whose Vecs are sized at Setup. No `Rc`, `RefCell` or stored lifetime. Single-actuator access borrows `&mut Bus`. | PASS |
| II. Absent Over Failing | `sync_read` exists only on `impl Bus<P2, …>`. Partial and raw methods exist only with their features, `DynBus`/`SerialBus` only with `std`. A model with no table on the bus protocol, or compiled out, fails Setup with `SetupError::UnsupportedModel`, before any byte for a declared list. `N > 253` is a compile error. A no_std bus without a named clock does not compile. | PASS |
| III. Nothing Hidden | Setup sends no torque write and starts every slot `Off`. Limits come only from EEPROM; a failed read fails Setup. A goal while `Off` or `Faulted` returns `ActuatorIsOff` with zero bytes sent. No read-back after writes. Every unicast and sync-read reply passes `observe`. A failed torque write applies `disable()`: `Off` from `On`/`Off`, and `Faulted` stays `Faulted`. Retries are only the configured count, only on timeout or corruption. Faults at Setup fail Setup. Exception: `raw-registers` replies return a hardware fault but do not set `Faulted` (FR-8.1, spec Assumptions, proposal Q5). Constitution III says every fault sets fault state. | **DEVIATION** (raw only): see Complexity Tracking |
| IV. e-Manual Is the Only Source | The bus adds no register data. It uses generated `ModelData`. The behaviour it relies on (limits per operating mode, reboot-only recovery on X-series, AX angle-limit error, P2 Sync Read reply order, alert bit) is cited from the e-Manual in research R-9, R-10 and R-11. | PASS |
| V. Committed, Data-Driven Codegen | No generated file is edited. The bus reads `ModelData` through `Describe`. | PASS (N/A for edits) |
| VI. Typed, Static, Panic-Free | `Position` and raw `i32` cross the API. `match`-based `ActuatorKind`, no `dyn`. Every fallible path returns `Result<_, Error>`. No `unwrap` outside tests. | PASS |
| VII. DX First | `SerialBus::<P2>::open(path, baud)` and `autoscan()` give a working bus. `bus.enable_torque(bus.ids())` works because `IdSet` is `Copy` (prototype-checked). Every public item is documented. Features only add methods or types. | PASS |
| Technical constraints | Timeouts are enforced by the bus through `Link` and its `Clock`. One bus = one wire, one protocol, one baud. Rust 2018 layout (`bus.rs` + `bus/`). | PASS |
| Workflow: one module per PR | Six PRs for one module. Each also touches `src/lib.rs` (core), and two touch `Cargo.toml`. One adds `tests/bus_no_alloc.rs`. | **DEVIATION**: see Complexity Tracking |

**Post-design re-check (after Phase 1)**: unchanged. Everything passes except the two recorded
deviations (workflow, and the raw-registers exception to III). The design added no allocation,
`dyn` or panic path. The open items below are behaviour the architect may change, not gate
failures.

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
6. **Bus PR removes PR #7's `expect(dead_code)` on `mod protocol;` (§1).** §1 lets bus add only
   its own `mod`/`pub use` lines to `src/lib.rs`. PR #7 puts
   `#[cfg_attr(not(test), expect(dead_code, reason = "exchanges are called by the bus module"))]`
   on `mod protocol;`. Once the bus calls every protocol item, the expectation is unfulfilled
   and `unfulfilled_lint_expectations` fails `-D warnings`. The bus PR that makes the last
   protocol item used deletes that attribute (see PR split). Alternative for the architect:
   core-1 turns it into an `allow`, which that bus PR then removes.

## Deviations from the SRS

1. **FR-4.7 "On P2 a failed sync group fails all slots in that group" (Decided), narrowed.**
   - The plan reads "failed group" as the group's instruction failing (`read_many` returns
     `Err`, e.g. a transport error). Then every slot of the group gets that error.
   - A single silent or corrupt actuator inside a P2 sync read gets its own per-slot error
     (`Timeout`, `Protocol`, `Status`), because PR #7's `sync_read` resolves replies per ID.
     The other slots keep their values. This is what US7 is for ("one silent actuator does not
     hide the others' values"). The spec's US7 test note "(on P2 the whole failed group fails)"
     may mean the strict reading.
   - Implemented in bus-5 (`src/bus/partial.rs`, research R-8 step 6).
   - Fallback if the architect keeps the strict reading: `partial.rs` fills every slot of a P2
     group with the first per-ID error of that group. P1 stays per slot. Only `partial.rs`
     and its tests change.

## Spec points the architect should confirm (no gate impact)

- **Limits and operating mode (spec conflict 8, now checked).** X-series: "These values are not
  used in Extended Position Control Mode and Current-based Position Control Mode" (e-Manual,
  XM430-W350, Min/Max Position Limit). AX: CW = CCW = 0 is wheel mode. The plan keeps the
  SRS's unconditional check (FR-3.7, Decided). In those modes the bus refuses more goals than
  the hardware would, and never sends one outside the read limits. Mode-aware limits wait for
  OQ-2 (research R-9).
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
└── tasks.md             # /speckit-tasks (not created here)
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
    ├── setup.rs            # declare, autoscan, init_one (model, firmware, limits)
    ├── id_set.rs           # IdSet (Copy bitset of 253 IDs)
    ├── field.rs            # pub mod field: GoalPosition, PresentPosition, Raw<F>, sealed traits
    ├── group.rs            # target resolution, whole-call validation, Stage, (addr, size) groups
    ├── multi.rs            # Easy tier + Explicit tier
    ├── partial.rs          # #[cfg(feature = "partial-reads")]
    ├── raw.rs              # #[cfg(feature = "raw-registers")] ActuatorRef::{read_raw, write_raw}
    ├── serial.rs           # #[cfg(feature = "std")] SerialBus alias + open
    ├── dyn_bus.rs          # #[cfg(feature = "std")] DynBus + DynSlots
    └── testing.rs          # #[cfg(test)] P1/P2 status-packet builders, scripted fixtures
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
| bus-1 | 4, 5 | US1, US3, US4 | `Bus`, `Config`, `NoClock`/`DefaultClock`, `Wire` + retries, `Slots`, `observe`, `declare` (model check, firmware, limits, protocol reach), `ActuatorRef` (torque, goal, present, read-back, `clear_fault`), `torque_state(s)`, compile-fail: two refs, ref outliving bus, no_std bus without clock | core-2, X1, P (`VERSION`) | `lib.rs`, `Cargo.toml` (`heapless`) |
| bus-2 | 4, 8 | US2 | `autoscan` P1/P2 (no_std too), duplicate-ID and fault-at-scan handling, `SerialBus` + `open` | bus-1, P (`scan`) | `lib.rs` |
| bus-3 | 6 | US5 | `IdSet`, `field` module, `group.rs`, `enable_torque`/`disable_torque` (ids), `write_goal_positions(_raw)`, `sync_write<F>`, proptest guard invariant | bus-1 | `lib.rs` |
| bus-4 | 6 | US6 | `read_positions(_raw)`, transactional staging, `sync_read<F>` on P2, compile-fail `sync_read` on P1, `tests/bus_no_alloc.rs` | bus-3, P (`read_many`) | `lib.rs` (+ deletes the `cfg_attr(not(test), expect(dead_code, …))` on `mod protocol;`, see note), `tests/` |
| bus-5 | 9 | US7, US8 | `partial.rs` (FR-4.7 per-ID slots, SRS deviation 1), `raw.rs` (no `observe`, constitution III exception), compile-fail without features | bus-4 | `lib.rs`, `Cargo.toml` (features) |
| bus-6 | 10 | US9 | `DynBus` on `DynSlots`, no-alloc cases for it, compile-fail without `std` | bus-4 | `lib.rs`, `tests/bus_no_alloc.rs` |

bus-5 and bus-6 are independent of each other. bus-2 and bus-3 are too.

**`expect(dead_code)` on `mod protocol;`** (deviation 6). Each bus PR uses more protocol items:
bus-1 `ping`/`read`/`write`, bus-2 `scan`/`broadcast_ping`, bus-3 `sync_write`, bus-4
`read_many`/`P2::sync_read`. The PR whose merge leaves no protocol item unused (expected bus-4,
or whichever of bus-2/3/4 lands last) deletes PR #7's
`#[cfg_attr(not(test), expect(dead_code, reason = "exchanges are called by the bus module"))]`
line in `src/lib.rs`. It is a declared core-file touch, listed in that PR. Every bus PR runs
`cargo clippy --all-targets -- -D warnings` (quickstart §1). An `unfulfilled_lint_expectations`
error there shows the PR that must delete it. If some feature set still leaves a protocol item
unused after that, the item gets a `cfg` in protocol (raised to its owner), not a new `expect`.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| One module in six PRs | Steps 4, 5, 6, 9 and 10 depend on three other modules landing in order (protocol hooks, core-2, X1). Each PR stays reviewable and reaches 70 % coverage alone. | One PR would sit open until every prerequisite merged and be too large to review. The seven-PR proposal would leave `main` with a bus that ignores faults. |
| `src/lib.rs` (core) touched by every bus PR | `mod bus;` and the `pub use` lines must be in the crate root. One PR (expected bus-4, else the last of bus-2/3/4) also deletes PR #7's `cfg_attr(not(test), expect(dead_code, …))` on `mod protocol;`, which fails `-D warnings` once the bus uses every protocol item (deviation 6). | No alternative for the bus lines (module-interfaces §1). The attribute removal is outside §1: listed in that PR, or core-1 turns it into an `allow` that the bus PR removes. |
| `raw-registers` replies do not set `Faulted` (constitution III) | SRS FR-8.1 (raw bypasses "limits, torque guard and cache"), spec Assumptions and US8, module-interfaces §2.5 and proposal Q5 all say raw returns the hardware fault but leaves the state alone. Constitution III says "a fault sets fault state". The architect is asked to approve this exception to III. | Running `observe` on raw replies (proposal Q5's other answer) contradicts FR-8.1 and spec US8 ("check the per-actuator state did not change"). If the architect prefers it, `raw.rs` calls `observe` and the US8-3 row becomes "slot `Faulted`". |
| `Cargo.toml` touched (bus-1, bus-5) | `heapless` is the storage. `partial-reads`/`raw-registers` gate API. | Allowed by §1 split ownership. Flagged as a shared-file touch. |
| `tests/bus_no_alloc.rs` outside `src/bus/**` | A runtime zero-allocation check needs its own global allocator, so its own test binary (R-12). | An in-crate test cannot install a second allocator or read the protocol's private counter. The static no_std proof does not cover `DynBus`. |
| Test-only status-packet builders duplicate a little protocol code | Protocol's builders and vectors are private test modules (R-13). | Editing protocol is forbidden to this module. The builders are checked against e-Manual examples. |
