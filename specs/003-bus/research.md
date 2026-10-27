# Research: Bus

**Feature**: [spec.md](spec.md) | **Date**: 2026-10-27

Sources:
- `specs/ideas/module-interfaces.md` (the cross-module contract) and `specs/ideas/bus-proposal.md`.
- The protocol module on branch `001-protocol-packets` (PR #7): `src/protocol.rs`, `p1.rs`,
  `p2.rs`, `clock.rs`, `no_alloc.rs`, `src/error.rs`, `src/lib.rs`.
- `src/` on `dev`; the SRS with the 2026-10-27 patch (`/tmp/dxl-srs-bus.patch`); constitution
  v1.2.0.
- The ROBOTIS e-Manual pages for the XM430-W350, the AX-12A and Protocol 2.0, fetched
  2026-10-27.
- A scratch crate (`/tmp/claude-1000/bus-proto`, heapless 0.9.3, rustc 1.97.1) that checked the
  type shapes below on the host and on `thumbv7em-none-eabihf`.

## R-1 The clock is the fourth type parameter

**Decision**: `Bus<P, T, const N: usize = 32, C = DefaultClock>`. `DefaultClock` is `StdClock`
under `std` and `NoClock` (an empty enum, not a `Clock`) without it. `new` takes the clock by
value. `ActuatorRef<'b, P, T, C>` carries `C` but not `N`.

**Rationale**: This is the architect's decision (spec Clarifications Q1). A stateless clock
takes zero bytes, a `fn() -> u64` or closure is called directly, and an owned hardware timer
works. The scratch crate confirms:
- a const default followed by a type default compiles;
- `Bus<P2, T>` names a std bus without the clock;
- a `fn` item, a capturing closure and `StdClock` are all inferred as `C`;
- `SerialBus::<P2>::open()` resolves through an alias with an inherent impl.

`NoClock` keeps features additive (module-interfaces conflict 15). Without `std`, `Bus::new`
needs a named clock. Enabling `std` only makes more code compile.

**Alternatives considered**:
- `C = fn() -> u64` (proposal). Rejected: an indirect call, and a no_std default that silently
  changes type when `std` turns on.
- A clock inside `Config` or passed to each call. Rejected: either dynamic dispatch or noise on
  every call.

## R-2 No `Clock` bound on the struct itself

**Decision**: The struct is `pub struct Bus<P: Protocol, T: Transport, const N: usize = 32, C = DefaultClock>`.
Every `impl` block bounds `C: Clock`. This deviates from module-interfaces §6 (see plan.md).

**Rationale**: In the scratch crate, `pub struct Bus<…, C: Clock = DefaultClock>` with
`DefaultClock = NoClock` fails every no_std build:

```
error[E0277]: the trait bound `NoClock: Clock` is not satisfied
note: required by a bound in `Bus`
```

rustc checks a default against the parameter's bound at the definition. With the bound moved to
the impls, the no_std build compiles on the host and on `thumbv7em-none-eabihf`.
`Bus::<P2, Uart>::new(uart, …)` without a clock still fails with `E0277`, which is the intended
"name your clock" error.

**Alternatives considered**:
- Make `NoClock` a `Clock` with an empty `match`. Rejected: then `Bus<P2, Uart>` is a valid type
  that can never be built, and the error moves to a confusing type mismatch at `new`.
- Drop the default under no_std. Rejected: a cfg-dependent parameter count is worse.

## R-3 Storage, slot lookup and capacity

**Decision**:
- `actuators: heapless::Vec<ActuatorKind, N>`, sorted by `Id`.
- `states: [TorqueState; N]`. `states[i]` belongs to `actuators[i]`.
- Lookup is `binary_search_by_key` on `Id`. Slot indexes are `u8`.
- `Bus::new` holds `const { assert!(N <= 253) }`, a post-monomorphization compile error.
- heapless 0.9 with its default length type. The scratch crate builds it on
  `thumbv7em-none-eabihf`.

**Rationale**:
- One byte per slot outside the actuator structs is the architect's decision (Q4 and the
  storage clarification).
- Binary search is the "Vec index" lookup NFR-4 allows. It needs no 253-byte slot table
  (proposal).
- Only 253 IDs exist, so `N > 253` can never fill. Rejecting it at compile time lets slot
  indexes, staging and the duplicate bitset stay small. It is not a runtime panic.

**Alternatives considered**:
- `TorqueState` in each model struct. Rejected by the architect: padding.
- A `u8` length type in heapless. Rejected: it changes the type the contract names, for a few
  bytes.

## R-4 `Wire`: the only caller of the protocol

**Decision**: `src/bus/wire.rs` defines crate-internal
`Wire<'b, P, T, C> { transport: &'b mut T, clock: &'b mut C, config: &'b Config }`.
- Each call builds a `protocol::Link`. Unicast calls use `config.timeout`. `scan` uses
  `config.scan_timeout`.
- Methods: `ping`, `read`, `write` (unicast, retried), `sync_write`, `read_many`, `scan` (never
  retried).
- A unicast call is retried at most `config.retries` times, and only when the error is
  `Error::Transport(TransportError::Timeout { .. })` or `Error::Protocol(_)`.
- `Error::Status`, `Error::Validation` and `TransportError::Io` return at once.

**Rationale**:
- This follows constitution III (no retry beyond the configured count) and spec FR-004.
- One place for the retry rule means `observe` sees the final result only, so a fault is
  marked once.
- Retrying a lost torque or goal write is safe, since writing the same value twice is
  idempotent.

**Timing (documented on `Config`)**:
- A unicast attempt does two waits. First it drains stale input, which ends as soon as the line
  is quiet and never runs past one timeout. Then it waits for the reply, bounded by one timeout.
- Worst case per unicast instruction:
  - on a quiet line: `(retries + 1) × timeout`;
  - on a line that never stops talking: `(retries + 1) × 2 × timeout`.
- The spec's SC-008 bound is measured on the quiet-line case. The talking-line case is
  documented.

**Alternatives considered**: retries inside the protocol layer. Rejected: the protocol has no
`Config`, and the contract puts the loop in `wire.rs`.

## R-5 Retries do not apply to multi-actuator reads

**Decision**: Wire does not retry `read_many` (P1 sequential reads or P2 sync read), `scan` or
`sync_write`. Single-actuator calls, Setup reads and the Easy-tier torque writes (one unicast
each) are retried.

**Rationale**:
- `read_many` is a protocol hook. Retrying single slots would mean the bus re-reads failed IDs
  with unicast reads.
- On P2 that would turn one broadcast instruction into extra unicast traffic. On P1 it would
  make the two protocols behave differently at the Easy tier.
- Spec FR-004 limits retries to "unicast instructions that time out or get a corrupted reply
  only". It does not require every unicast to be retried.

Flagged in plan.md for the architect.

**Alternatives considered**:
- Pass `retries` into `read_many`. Rejected: that is a protocol signature change outside
  module-interfaces §5.
- Re-read failed slots by unicast. Rejected: hidden traffic.

## R-6 Setup

**Decision**: Both setups clear the list, then fill it. On any error they clear it again
(FR-015). `init_one(id, pong)` runs the per-actuator steps of module-interfaces §6.

Declared setup (`declare(list)`), before any byte:
1. Collect the list into a stack `heapless::Vec<(Id, ModelNumber), N>`.
   - Overflow gives `SetupError::TooManyActuators { capacity: N }`.
   - A repeated `Id` gives `ValidationError::DuplicateId`.
2. Check every model: `ActuatorKind::new(P::VERSION, model, 0, id)`.
   - A model compiled out, or with no table on this protocol, fails here with
     `UnsupportedModel` (constitution II: Init errors before I/O).
   - `from_fw` starts at 0, so firmware 0 always resolves a supported model.

Declared setup, then per entry:
1. Ping with retries.
   - A `Timeout` gives `SetupError::Missing { id }`. Any other error propagates.
2. Model number.
   - P2: from the `Pong`.
   - P1: read `models::MODEL_NUMBER`.
   - A different number gives `SetupError::WrongModel`.
3. `init_one` reads the firmware and limits and builds the slot (below).

Autoscan (`autoscan()`):
1. `wire.scan` collects replies into a stack `heapless::Vec<Pong, N>`.
2. Errors from the scan:
   - The first `Err` reply (a corrupted packet, module-interfaces conflict 16) is kept and fails
     Setup.
   - A reply past capacity gives `TooManyActuators`.
   - Two replies with the same `Id` give `DuplicateId`. SRS 4.5 says duplicates are caught only
     as far as the protocol shows them, and two replies are exactly that.
   - A `Pong` with `fault: Some(report)` fails Setup with `Error::Status(StatusError { id, report })`.
3. Sort by `Id`, then run `init_one` for each.
4. No `std` gate (Q2): `scan`, the stack Vec and the lookup allocate nothing.

`init_one(id, model, pong)`:
1. If `ActuatorKind::firmware_register(P::VERSION, model)` is `Some`, get the firmware: from
   the `Pong` on P2, otherwise read it.
2. `ActuatorKind::new(P::VERSION, model, fw, id)`. This is the protocol-reach check (FR-012).
   Codegen guarantees P1 tables keep their role registers at or below address 255.
3. Read the min and max limit registers through `Describe::data().limits`. Decode them and call
   `cache_mut().record_limits(min, max)`.
   - CW maps to min and CCW maps to max.
   - Any error, `InvertedLimits` included, fails Setup.
4. Push the actuator and set its slot to `TorqueState::Off`. No torque write is sent (FR-014).

A hardware fault reported at Setup fails Setup (spec Assumptions, proposal Q6).

**Rationale**: This follows the contract steps exactly. Every check that needs no wire runs
before the first byte.

**Alternatives considered**:
- Instantiate faulted actuators as `Faulted`. Rejected by the spec (Assumptions).
- Read both limits in one span read when they are adjacent. Rejected for now: Init is one-off,
  and two reads are simpler.

## R-7 Applying the torque state

**Decision**: `src/bus/observe.rs` applies the module-interfaces §2.5 rules in one place.

- `observe(state, &result)`: `Err(Error::Status(e))` with `e.is_hardware_fault()` sets
  `*state = state.fault()`. It runs on:
  - every `ActuatorRef` exchange;
  - every per-slot reply of a multi-read;
  - every Easy-tier torque write;
  - every `clear_fault` check.
- It does not run on `sync_write` (no reply, FR-029) or on `raw-registers` calls (FR-8.1). The
  raw case is an exception to constitution III, flagged in plan.md Complexity Tracking.
- After a torque write:
  - `Ok` gives the new state.
  - A hardware fault gives `fault()`.
  - Any other error gives `*state = state.disable()`, never `On` (FR-027). That is `Off` from
    `On` or `Off`, and `Faulted` stays `Faulted`. So a failed enable leaves `Off`, and a failed
    disable leaves `Off` or keeps the fault. Goals are refused until the user retries, which is
    the safe side.
  - A `Faulted` slot whose `disable_torque` times out or gets a bad CRC stays `Faulted`, so
    `enable` is still refused until `clear_fault` passes (core's table, FR-028).
  - module-interfaces §2.5 said "Any other error gives `Off`", which would drop the fault. The
    plan deviated (plan.md deviation 5), and §2.5 now applies `state.disable()`.
- `enable` is checked before any byte. While `Faulted` it returns
  `ValidationError::ActuatorFaulted`.
- Easy-tier torque (`enable_torque(ids)` / `disable_torque(ids)`):
  - Both send acknowledged unicast writes, in target order, after the whole target list is
    validated (unknown, duplicate, or any `Faulted` target for enable).
  - `enable_torque` stops at the first error.
  - `disable_torque` tries every target and returns the first error, a hardware fault first.

**Rationale**: One rule site means no path can skip it (SC-003). Stopping an enable sequence on
error avoids enabling more motors after a problem. Continuing a disable sequence turns off as
many motors as possible.

**Alternatives considered**: torque as a `WriteField` for `sync_write`. Rejected: no reply, so
the bus state could not match the hardware (proposal item 12).

## R-8 Multi-actuator ops: resolve, validate, stage, group

**Decision**: `group.rs` has one N-free core over slices, called with a stack
`[Stage; N]` (`Bus`) or a Setup-sized Vec (`DynBus`).

Steps:
1. **Resolve.** Each target `Id` is looked up in target order.
   - Unknown gives `SetupError::NotOnBus { id }`.
   - A repeat (found through an `IdSet` bitset) gives `ValidationError::DuplicateId { id }`.
2. **Count.**
   - Writes need `values.len() == targets`.
   - Reads need `out.len() >= targets`; extra slots stay untouched.
   - A mismatch is `ValidationError::BufferLength { targets, got }`.
3. **Validate** (writes), per target in order:
   - `state.check_goal(id)` (`ActuatorIsOff`);
   - `scale.to_raw` for typed values;
   - `cache.check_goal(raw)`;
   - `Register::encode` into the stage.

   The first failure refuses the call. Zero bytes are sent (FR-034, SC-002).
4. **Group.** The distinct `(addr, size)` keys come in first-appearance order. Each key sends one
   instruction, with an iterator that filters the stages by key. It is `Clone`, as
   `sync_write` and `read_many` need.
5. **Reads.**
   - Each group calls `wire.read_many`. The callback comes in target order, so a cursor over the
     group's stages maps each reply to its slot. Decoded raw values go into the stage.
   - Every slot result passes `observe`.
   - At the end, a strict read with no error converts and copies all stages into `out`
     (transactional).
   - Otherwise it returns the first hardware fault in target order, or else the first error.
     `out` stays unchanged (FR-036).
   - A transport error returned by `read_many` aborts at once.
6. **Partial reads** (`partial-reads`, bus-5): the same staging, but each slot of `out` gets its
   own `Result`.
   - A group whose `read_many` returns `Err` (the instruction failed) gives that error to every
     slot in the group.
   - Per-ID errors inside a P2 sync read stay per slot. This narrows SRS FR-4.7 ("a failed sync
     group fails all slots in that group", Decided) and is recorded as a deviation from the SRS
     in plan.md (SRS deviation 1).
   - Fallback if the architect keeps the strict reading: on P2, every slot of a group with any
     per-ID error gets the group's first per-ID error. P1 stays per slot.

On the wire, all-or-nothing holds for validation only. A transport failure between two groups
leaves the earlier group sent (spec edge case). This is documented on every multi-write.
An empty target list returns `Ok(())` and sends nothing.

**Rationale**:
- Staging costs 8 bytes × N on the stack (256 B at N = 32) and allocates nothing.
- Grouping by `(addr, size)` is the group shape the bulk-op roadmap reuses.

**Alternatives considered**:
- Encoding inside the instruction iterator. Rejected: `sync_write` needs `&'d [u8]` that outlive
  the iterator, and validation must finish before the first byte.
- A buffer of `Option<T>` (SRS FR-4.5 [decision]). Rejected by the spec: plain values plus an
  unchanged buffer on failure.

## R-9 Limits and the operating mode (e-Manual check)

**Finding**:
- XM430-W350, Min/Max Position Limit (48, 52): "These values are not used in Extended Position
  Control Mode and Current-based Position Control Mode", and "[Goal Position(116)] will also be
  limited by the position limit range" (<https://emanual.robotis.com/docs/en/dxl/x/xm430-w350/>).
- AX-12A: CW and CCW Angle Limit both 0 means wheel mode. A goal outside the limits makes the
  actuator answer with the Angle Limit error bit
  (<https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/>). That is not a hardware fault in
  `is_hardware_fault`.

**Decision**: Keep the unconditional limit check (SRS FR-3.7, Decided).
- In Position Control mode (the X-series default) it matches the hardware.
- In extended or current-based position modes, and in AX wheel mode (limits `0..=0`), it
  refuses goals the hardware would accept. It never sends a goal outside what was read.
- The rustdoc of `set_goal_position` says so.
- Mode-aware limits need an Operating Mode read at Setup, which is a model-specific register
  (OQ-2). That waits for the architect.

**Alternatives considered**: read Operating Mode at Setup and skip the check in modes 4 and 5.
Rejected for now: a register beyond OQ-2, and mode changes later go unseen.

## R-10 Clearing a fault (e-Manual check)

**Finding**:
- XM430-W350: "REBOOT is the only method to reset [Torque Enable(64)] to '1'(Torque ON) after the
  shutdown". The alert bit and Hardware Error Status (70) report the cause.
- AX-12A: after overheating or overload the output drops to 0, and it recovers when the cause is
  gone.

**Decision**: `clear_fault()` asks the actuator itself.
- If `data().hardware_error` is `Some(reg)`, it reads that register.
  - `Ok` with 0 clears the state to `Off`.
  - `Err(Status)` with the alert bit means still faulted. It returns that error and the state
    stays `Faulted`.
  - `Ok` with a non-zero value is also still faulted. It returns
    `StatusError { id, report: P2 { alert: true, code: None } }`, documented.
- If it is `None` (P1 models such as AX), it pings. A P1 status with a hardware flag comes back
  as `Err(Status)`: still faulted. `Ok` clears.
- `clear_fault` on a non-faulted actuator is a no-op that still checks (FR-028 asks the
  actuator).
- Reboot stays out of scope. X-series users must reboot outside this feature, as the spec edge
  case says.

**Alternatives considered**: `Result<bool>` (proposal). Rejected: the report would be easy to
drop.

## R-11 Autoscan window and reply order (e-Manual check)

**Finding** (Protocol 2.0, <https://emanual.robotis.com/docs/en/dxl/protocol2/>):
- Broadcast Ping: "all connected devices will respond with a status packet". Order and timing
  are not specified.
- Sync Read: "Status Packet will be returned in order, according to input ID in the Instruction
  Packet."

**Decision**:
- The P2 autoscan window is one `scan_timeout`, and every reply inside it is collected (PR #7
  behaviour).
- The docs of `Config::scan_timeout` say replies share one wire. A large chain at low baud needs
  a longer window: each P2 ping reply is 14 bytes, about 140 bit-times plus the actuator's
  Return Delay Time.
- Sync-read slot mapping relies on the documented reply order.

## R-12 Proving zero allocation per Cycle

**Decision**: Two proofs.
- **Static.** Everything except `DynBus` and `SerialBus` compiles in the no_std, no-alloc
  clippy build. It cannot call an allocator, and the same code runs under `std`.
- **Runtime.** `tests/bus_no_alloc.rs` (`#![cfg(feature = "mock")]`) installs a counting
  `#[global_allocator]` like `src/protocol/no_alloc.rs`. It sets up a `Bus` and a `DynBus`
  through the mock. Then it asserts zero allocations across:
  - `ActuatorRef` goal, present and torque calls;
  - `write_goal_positions`, `read_positions`, `sync_write`, `sync_read`;
  - partial reads when that feature is on.

  CI must run `cargo test --features mock` (and with `partial-reads` for full coverage). That
  is noted for the ci module.

**Rationale**:
- The lib test binary already has the protocol's global allocator. A second one fails to
  compile, and the protocol's counter is private.
- An integration test is its own binary.
- `MockTransport` is public behind `mock`, so the public API is enough.

**Alternatives considered**:
- A self dev-dependency with `features = ["mock"]`. Rejected: an extra `Cargo.toml` trick for one
  test.
- Asking protocol to expose its counter. Rejected: a protocol change outside §5.

## R-13 Status packets in bus tests

**Decision**: `src/bus/testing.rs` (`#[cfg(test)]`) builds P1 status packets (checksum) and P2
status packets (CRC-16, byte stuffing) from `(id, error, params)`. It also has helpers that
script a declared or scanned bus on `MockTransport` with `reply_to(n, …)`. Its own tests check
it against the e-Manual examples:
- P2 Ping reply `FF FF FD 00 01 07 00 55 00 06 04 26 65 5D`;
- P1 status examples from the Protocol 1.0 page.

**Rationale**: Protocol's `packet()` helpers and `vectors.rs` sit in private `#[cfg(test)]`
modules of `src/protocol/**`, and this module may not edit them. A tested builder keeps bus
tests byte-exact (SC-004).

## R-14 Compile-fail checks

**Decision**: Each SC-006 misuse is a rustdoc `compile_fail,EXXXX` example on the item it
concerns:
- two live `ActuatorRef`s (`E0499`);
- an `ActuatorRef` that outlives its bus (`E0597`/`E0505`);
- `sync_read` on `Bus<P1, …>` (`E0599`);
- `read_positions_partial`, `read_raw`/`write_raw` without their features (`E0599`);
- `DynBus` without `std` (`E0412`/`E0433`);
- `Bus::new` without a clock under no_std (`E0277`).

Feature-absent cases are `cfg`-gated docs that run only when the feature is off.

**Rationale**: This is module-interfaces conflict 13. Stable rustdoc does not check the error
code (spec 002 R-9), but CI's pinned toolchain does (ci FR-008). The scratch crate passed the
`E0599` and `E0499` cases on stable.

## R-15 Explicit tier: field markers

**Decision**: There is a public module `field` with sealed traits `ReadField` and `WriteField`
(`type Value: Copy`).
- Markers: `GoalPosition` (write, `Position`), `PresentPosition` (read, `Position`), and
  `Raw<F>(pub F)` (`Value = i32`) for either.
- The field is passed as a value, so nothing needs a turbofish:
  `bus.sync_write(GoalPosition, ids, &goals)` and
  `bus.sync_read(Raw(PresentPosition), ids, &mut out)`. The scratch crate checked the
  `Raw<F>` value-type mapping.
- `TorqueEnable` is deliberately not a field.

Each field maps to a `ModelData` register: `goal_position` or `present_position`.

**Rationale**:
- With two registers (OQ-2), this gives the explicit tier a typed surface that grows by adding
  markers.
- `Raw<F>` avoids a parallel set of raw markers.
- Explicit `sync_write` runs the same validation as the Easy tier. Only `raw-registers` bypasses
  checks.

**Alternatives considered**: passing register addresses. Rejected: register locations stay
crate-internal (xtask clarification).

## R-16 `IdSet`

**Decision**: `IdSet` is a `Copy` bitset of 253 IDs (`[u32; 8]`, 32 bytes).
- It iterates in ascending order and implements `IntoIterator<Item = Id>`,
  `FromIterator<Id>` and `From<[Id; K]>`.
- Methods: `new`, `range(first, last)`, `insert`, `remove`, `contains`, `len`, `is_empty`.
- `bus.ids()` returns the held IDs.

**Rationale**:
- `bus.enable_torque(bus.ids())` compiles with two-phase borrows because `ids()` returns an owned
  `Copy` value (scratch crate).
- Ascending order matches `actuators()` order, so `read_positions(bus.ids(), &mut out)` fills
  `out` in the same order.
- The bus also uses `IdSet` internally for duplicate detection.

## R-17 `DynBus` (uncapped, `std`)

**Decision**: `DynBus<P, T, C = StdClock>` is a separate type.
- Storage: `alloc::Vec<ActuatorKind>`, `alloc::Vec<TorqueState>` and an `alloc::Vec<Stage>`
  sized at Setup.
- It has the same public methods as `Bus`, as thin wrappers over the same crate-internal engine.
- The engine (`setup.rs`, `group.rs`, `multi.rs`, `observe.rs`) is generic over a crate-internal
  `Slots` trait: `kinds`, `split_mut`, `clear`, `push`, `stages`. It is implemented by
  `FixedSlots<'_, N>` (borrowing `Bus`'s fields) and `DynSlots<'_>`.
- `ActuatorRef` has no `N`, so both buses hand out the same handle type.

**Rationale**:
- SRS FR-1.3 and spec conflict 10: a plain alias cannot swap the storage.
- Module-interfaces §6 forbids changing `Bus`. A storage type parameter on `Bus` would change
  it.
- The `Slots` engine keeps one implementation of every rule.
- The Vecs grow at Setup (Init), never in a Cycle.

**Alternatives considered**: a hidden fifth type parameter on `Bus`. Rejected: it changes `Bus`'s
shape and error messages.

## R-18 `SerialBus`

**Decision**: `pub type SerialBus<P, const N: usize = 32> = Bus<P, SerialTransport, N, StdClock>;`
with `open(path, baud) -> Result<Self, serialport::Error>`. It wraps `SerialTransport::open`,
`StdClock::default()` and `Config::new()`. A user who needs another config calls `Bus::new`
with those three parts.

**Rationale**: FR-006 asks for one step. The error stays `serialport::Error`, as in spec 002
R-4: it happens at startup, outside any bus exchange.

## R-19 Typed position path

**Decision**: Every goal write follows module-interfaces §6:
`state.check_goal` → `scale.to_raw` (rounds to nearest, `NotFinite`, `DoesNotFit`) →
`cache.check_goal(raw)` → `Register::encode` → `wire.write`.
- The raw form skips only `to_raw`.
- Reads decode with `Register::decode`, then `scale.to_position`.
- The limits a user sees (`Actuator::position_limits[_raw]`) are the ones `check_goal` uses, so
  "rounds outside the limits" (US4 AS3) is refused in both forms.

## R-20 Tier 2 and `Actuator` on the handle

**Decision**:
- `ActuatorRef::kind()` returns `&ActuatorKind` for `match` (FR-018). Model types carry no
  public I/O today (OQ-2), so Tier 2 is description only.
- `ActuatorRef` implements `Describe` by delegating to its `ActuatorKind`, which gives it
  `Actuator` (`id`, `name`, `model_number`, `position_limits[_raw]`, `position_scale`).
- The bus needs `crate::actuator`'s sealed `Describe` reachable crate-wide. Generated models
  need the same, so core provides it.

**Rationale**: This is module-interfaces §6 as written, and the trait stays the one place for
model description.
