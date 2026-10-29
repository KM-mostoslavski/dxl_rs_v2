---

description: "Task list for the bus module (setup, single- and multi-actuator access, faults)"
---

# Tasks: Bus (setup, single- and multi-actuator access, faults)

**Input**: Design documents from `specs/003-bus/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/bus-api.md, quickstart.md,
`specs/ideas/module-interfaces.md` (§1 ownership, §2 core items incl. §2.4b limit mode, §5 protocol
hooks, §6 bus shape, §7 delivery order, §9 decisions), constitution v1.3.0.

**Tests**: Required. Constitution gate (≥ 70 % line coverage per PR), SC-002…SC-009, and every
acceptance scenario of spec.md has a test row in quickstart.md §2. Compile-fail cases (SC-006)
are rustdoc `compile_fail,EXXXX` examples, not `trybuild` (module-interfaces conflict 13). Test
tasks come first in each story; they must fail or not compile before the implementation tasks
after them.

**Organization**: Grouped by user story, and by PR. The plan splits this module into six PRs
(bus-1 … bus-6). Task IDs follow execution order, so the phases follow the PR order, which keeps
spec priority order except one move: US3 and US4 (P2) ship in bus-1 with US1, before US2 (P1,
bus-2), because the plan puts fault handling into bus-1 (plan deviation 2: `main` must never carry
a bus that ignores hardware faults) and bus-2 needs bus-1.

**Architect decisions of 2026-10-28** are folded into the task text: raw replies run `observe`
(T073, T075), `partial-reads` keeps one result per actuator inside a P2 sync group (T068, T071),
limits follow the operating mode (T019, T021, T023, T034, T046, T051), the mode value lists come
from the tables through the generated `ModeValues` (T002), and core-1, not a bus PR, removes
`P2::broadcast_ping` and PR #7's `expect(dead_code)` on `mod protocol;` (rules, T035).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1 = declare + one actuator, US2 = autoscan + `SerialBus`, US3 = faults,
  US4 = typed units, US5 = multi-actuator writes, US6 = strict multi-actuator reads,
  US7 = `partial-reads`, US8 = `raw-registers`, US9 = `DynBus`
- **[human]**: Only the architect or the human owner does this task. An agent never edits the
  path it names.

## Rules for every task

- Rust edition 2024, Rust 2018+ layout: `src/bus.rs` + `src/bus/*.rs`, never `mod.rs`
  (`clippy::mod_module_files`).
- Files this module may change: `src/bus.rs`, `src/bus/**`, `tests/bus_no_alloc.rs`, its own
  `mod bus;` / `pub use bus::…` lines in `src/lib.rs`, and the `heapless` dependency (bus-1) and
  the `partial-reads` / `raw-registers` features (bus-5) in `Cargo.toml`. Every shared-file touch
  is listed in that PR's description. PR #7's `expect(dead_code)` on `mod protocol;` and
  `P2::broadcast_ping`, which the bus never calls, are removed by core-1 (architect 2026-10-28,
  plan deviation 6); no bus PR touches `src/lib.rs` beyond its own lines.
- Never edit core (`src/error.rs`, `src/id.rs`, `src/actuator.rs`, `src/register.rs`,
  `src/units*`), protocol (`src/protocol*`), transport, `xtask/**`, `control_tables/**` or the
  generated `src/models.rs` / `src/models/**` (only `cargo xtask codegen` writes those). A missing
  item there is raised to its owner, not patched here.
- Human-only paths (`.aiignore`): `.aiignore`, `.claude/settings.json`,
  `.github/workflows/ov-*.yml`, `.github/ov-ruleset-main.json`, `.specify/extensions/ov/**`,
  `docs/srs/SRS.md`. Also architect-owned (module-interfaces §1): the constitution and
  `docs/ARCHITECTURE.md`. Any change there is a `[human]` task.
- The protocol additions (`ProtocolVersion`, `Protocol::VERSION`, `Exchange::scan`,
  `Exchange::read_many`, the `#[diagnostic::on_unimplemented]` on `Clock`) are on `main` (PR #7,
  merged 2026-10-28). The limit-mode items (`LimitMode`, `Cache::record_mode`, the mode-aware
  `Cache::check_goal`, `ModeSource`, the value → `LimitMode` mapping, `NotPositionMode`,
  `UnknownOperatingMode`) come from core-1 and X1. Consume them; never re-implement them.
- Only `src/bus/wire.rs` builds a `protocol::Link` and calls `Exchange` / `P2::sync_read`
  (plan Summary, research R-4).
- Document every public item. No comments inside fn bodies unless they resolve a real
  ambiguity. No `unwrap`/`expect`/`panic!`/out-of-bounds indexing outside tests; the only
  compile-time assertion is `N <= 253` (research R-3). No `dyn`, `Rc`, `RefCell` or stored
  lifetime in `Bus` (FR-007).
- Unit tests live in a `#[cfg(test)] mod tests` at the bottom of the file they test, against
  `MockTransport` and `MockTime::ticker` (no hardware). "Zero bytes" means `line.sent()` is empty
  and `line.flushes() == 0` after the call (quickstart §2).
- A crate-internal helper is added in the PR that first calls it, so `-D warnings` never sees
  dead code (e.g. `Wire::scan` in bus-2, `Wire::sync_write` in bus-3, `Wire::read_many` in bus-4,
  `Slots::stages` in bus-6).
- Commits and PRs carry no co-authorship or AI attribution (constitution, Workflow).

---

## Phase 1: Setup (PR bus-1)

**Purpose**: Approval, prerequisites, dependency and module skeleton

- [x] T001 [human] Architect: approve the six-PR split (approved 2026-10-28: "you can touch multiple modules its fine") of this module (plan Complexity Tracking, module-interfaces §7 and §9). Needed before T036 opens bus-1. The 2026-10-28 decisions (raw replies run `observe`, FR-4.7 one result per actuator, limits follow the operating mode) need no further answer; they are in the tasks below
- [X] T002 Check the prerequisites on `origin/main` (module-interfaces §7) and stop with a report to the caller if any is missing: protocol PR #7 is merged (2026-10-28), so confirm `src/protocol.rs` still has `ProtocolVersion`, `Protocol::VERSION`, `Exchange::{ping, read, write, sync_write, scan, read_many}`, `P2::sync_read`, `Link`, `Pong { id, model_number, firmware, fault }`; core-1 + core-2 merged (`src/actuator.rs`: `ModelNumber`, `TorqueState`, `Cache` with `record_mode` and a mode-aware `check_goal`, `LimitMode { Bounded, Unbounded, NoPosition }` with `LimitMode::from_angle_limits`, `ModeSource` (`Register { register, values: ModeValues }` or `AngleLimits`) with `ModeSource::register()` and `ModeSource::limit_mode(value) -> Option<LimitMode>` (module-interfaces §2.4b, specs/004-core contracts), `ModelData` with `mode: ModeSource`, `LimitRegisters`, `Describe`, `Actuator` with provided methods; `src/units/position.rs`: `Position`, `PositionScale`, `Limits`; `src/error.rs`: `SetupError` with `UnknownOperatingMode { id, value }` and the `ValidationError` variants `ActuatorIsOff, ActuatorFaulted, OutOfLimits, LimitsUnknown, InvertedLimits, NotFinite, DoesNotFit, BufferLength, DuplicateId, NotPositionMode`); xtask X1 merged with regenerated `src/models.rs` (`ActuatorKind::new(ProtocolVersion, ModelNumber, u8, Id)`, `ActuatorKind::firmware_register`, `models::MODEL_NUMBER`, a `ModeSource` in every model's `DATA`, its `ModeValues` lists copied from the table's `operating_modes`). Then create branch `003-bus-1` from `origin/main`
- [X] T003 Add `heapless = { version = "0.9", default-features = false }` to `[dependencies]` in Cargo.toml (shared-file touch, module-interfaces §1)
- [X] T004 Create the bus-1 skeleton: src/bus.rs (module doc placeholder; `mod actuator_ref; mod observe; mod setup; mod slots; mod wire; #[cfg(test)] mod testing;`) and empty files src/bus/actuator_ref.rs, src/bus/observe.rs, src/bus/setup.rs, src/bus/slots.rs, src/bus/wire.rs, src/bus/testing.rs; add `mod bus;` to src/lib.rs after `mod actuator;`. Files of later PRs (`id_set.rs`, `field.rs`, `group.rs`, `multi.rs`, `partial.rs`, `raw.rs`, `serial.rs`, `dyn_bus.rs`) are created by those PRs
- [X] T005 Verify the skeleton: `cargo build`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --no-default-features --features ax,mx,x-series,y-series,pro,mock --target thumbv7em-none-eabihf -- -D warnings`, `RUSTFLAGS="-D warnings" cargo build --lib --no-default-features` all pass; fix fallout only in files from T003–T004

---

## Phase 2: Foundational (PR bus-1, blocking prerequisites)

**Purpose**: Test packet builders, `Config`, clocks, the `Bus` struct, `Wire` with retries, `Slots`
and `observe`, which every story uses

**⚠️ CRITICAL**: No user-story work starts until this phase is complete

### Tests for the foundation ⚠️

- [X] T006 [P] Write src/bus/testing.rs (`#[cfg(test)]`, research R-13): `p2_status(id: u8, error: u8, params: &[u8]) -> std::vec::Vec<u8>` (header `FF FF FD 00`, LE length, instruction `0x55`, byte stuffing, CRC-16 poly 0x8005), `p1_status(id, error, params)` (checksum `!(ID + LEN + ERR + params)`), the matching instruction builders `p2_instruction(id, inst, params)` and `p1_instruction(id, inst, params)` for byte-exact `sent()` checks, `p2_pong(id, model: u16, fw: u8)`, `fn id(n: u8) -> Id`, and fixtures that script one declared actuator's replies with `reply_to(n, …)`: `xm430_p2(line, id, min, max, mode)` (ping, Min/Max Position Limit reads, Operating Mode read at 11), `xl320_p2(line, id, cw, ccw, control_mode)` (ping, CW/CCW Angle Limit reads, Control Mode read at 11), `ax12_p1(line, id, cw, ccw)` (ping, P1 model read, CW/CCW Angle Limit reads, no mode read). Self-tests in the same file: `p2_status(1, 0, &[0x06, 0x04, 0x26])` equals the e-Manual P2 ping reply `FF FF FD 00 01 07 00 55 00 06 04 26 65 5D`; `p2_instruction(1, 0x01, &[])` equals `FF FF FD 00 01 03 00 01 19 4E`; `p1_status(1, 0, &[])` equals `FF FF 01 02 00 FC`; `p1_status(1, 0, &[0x20])` equals `FF FF 01 03 00 20 DB`; `p1_instruction(1, 0x01, &[])` equals `FF FF 01 02 01 FB`
- [X] T007 [P] Write tests in src/bus/wire.rs (SC-008, data-model.md "Retry rule": "there are at most `retries` extra attempts. A retry happens only after `Transport(Timeout)` or `Protocol(_)`. `Status`, `Validation` and `Transport(Io)` return at once."): `retries_1_timeout_then_reply_is_ok` (2 flushes); `retries_1_corrupt_then_reply_is_ok`; `status_error_is_not_retried` (1 flush, `Err(Status)`); `io_error_is_not_retried` (`fail_with(BrokenPipe)`, 1 call); `retries_0_sends_once`; `silent_line_is_bounded` (`retries = 1`, `timeout = 10 ms`, `MockTime::ticker(10)`: `Err(Transport(Timeout { id: Some(_) }))` and `time.now() <= 2 * 10_000 + 1_000`); `unicast_uses_config_timeout`
- [X] T008 [P] Write tests in src/bus/observe.rs (data-model.md "TorqueState as the bus applies it", module-interfaces §2.5): `observe` on `Err(Status)` with `is_hardware_fault()` turns `Off`/`On`/`Faulted` into `Faulted`; on a non-hardware status (P2 code without alert, P1 `AngleLimit` only), `Ok`, `Timeout` or `Protocol` it leaves the state as is; `after_torque_write(state, wanted, &result)`: `Ok` → `wanted`; hardware fault → `Faulted`; any other error → `state.disable()` (`On` → `Off`, `Off` → `Off`, `Faulted` stays `Faulted`, never `On`, FR-027, plan deviation 5)
- [X] T009 [P] Write tests in src/bus/slots.rs for `FixedSlots`: `push` keeps `kinds()` and `split_mut()` the same length and order; `push` past `N` gives `SetupError::TooManyActuators { capacity: N }`; `clear` empties the list and resets every state to `Off` ("Entries at `i >= len` are unused and kept `Off`"); `size_of::<TorqueState>() == 1` and `states` is `[TorqueState; N]`
- [X] T010 [P] Write tests in src/bus.rs: `Config::new()` is `timeout = 10 ms, scan_timeout = 10 ms, retries = 0` and equals `Config::default()`; the `const` builders set each field; US1-10 / FR-005 `new_then_into_parts_sends_nothing` (`Bus::<P2, _, _>::new(line, time.ticker(10), Config::new())`, then `into_parts()` gives back the transport with `sent()` empty and `flushes() == 0`, and the clock); `config()` returns what `new` got; a new bus has `actuators()` and `torque_states()` empty

### Implementation for the foundation

- [X] T011 Implement in src/bus.rs (contracts/bus-api.md "Clocks and config"): `pub enum NoClock {}` (doc: "No clock. Implements no `Clock`, so a no_std bus must name its clock."); `#[cfg(feature = "std")] pub type DefaultClock = StdClock;` / `#[cfg(not(feature = "std"))] pub type DefaultClock = NoClock;`; `#[derive(Debug, Clone, Copy, PartialEq, Eq)] #[non_exhaustive] pub struct Config { pub timeout: Duration, pub scan_timeout: Duration, pub retries: u8 }` with defaults "10 ms, 10 ms, 0", `pub const fn new()`, `pub const fn timeout(self, t)`, `pub const fn scan_timeout(self, t)`, `pub const fn retries(self, n)`, `impl Default for Config` = `new()`. The `Config` doc carries the timing formulas of research R-4 (quiet line: `(retries + 1) × timeout`; talking line: `(retries + 1) × 2 × timeout`) and "Retries apply to unicast instructions that time out or get a corrupted reply only" (FR-004)
- [X] T012 Implement the `Bus` struct in src/bus.rs per data-model.md: `pub struct Bus<P: Protocol, T: Transport, C = DefaultClock, const N: usize = 32>` with **no `Clock` bound on the struct** (plan deviation 1, research R-2) and fields `transport: T`, `clock: C`, `config: Config`, `actuators: heapless::Vec<ActuatorKind, N>` ("Sorted by `Id`, no duplicates"), `states: [TorqueState; N]` ("`states[i]` belongs to `actuators[i]`. Entries at `i >= len` are unused and kept `Off`."), `_protocol: PhantomData<P>`. No limit-mode field: the mode lives in each `Cache` (data-model.md "LimitMode as the bus applies it"). In `impl<P: Protocol, T: Transport, C: Clock, const N: usize>`: `new(transport, clock, config)` with `const { assert!(N <= 253) }` and no I/O; `into_parts(self) -> (T, C)`; `config(&self) -> Config`; `actuators(&self) -> &[ActuatorKind]` ("Held actuators, ascending by `Id`"); `torque_state(&self, id) -> Option<TorqueState>` (binary search on `Id`); `torque_states(&self) -> &[TorqueState]` ("Same order and length as `actuators()`"); crate-internal `split(&mut self) -> (Wire<'_, P, T, C>, FixedSlots<'_, N>)` with disjoint field borrows. Re-export `pub use bus::{Bus, Config, DefaultClock, NoClock};` in src/lib.rs
- [X] T013 Implement src/bus/slots.rs (data-model.md "Slots"): crate-internal `trait Slots { fn kinds(&self) -> &[ActuatorKind]; fn split_mut(&mut self) -> (&mut [ActuatorKind], &mut [TorqueState]); fn clear(&mut self); fn push(&mut self, kind: ActuatorKind) -> Result<(), SetupError>; }` ("`clear`: Empties the list. States become `Off`." / "`push`: `TooManyActuators` past capacity.") and `FixedSlots<'a, N>` borrowing `Bus`'s `actuators` and `states`; a crate-internal `fn find(kinds: &[ActuatorKind], id: Id) -> Option<usize>` (`binary_search_by_key` on `Id`, research R-3). `stages` is added in bus-6 (T081)
- [X] T014 Implement src/bus/wire.rs (research R-4): crate-internal `struct Wire<'b, P, T, C> { transport: &'b mut T, clock: &'b mut C, config: &'b Config }` and `ping(id)`, `read(id, reg: Register, out)`, `write(id, reg: Register, bytes)`, each building `protocol::Link { transport, clock, timeout: config.timeout }` per attempt and calling `P::ping` / `P::read` / `P::write`; the retry loop per the data-model.md "Retry rule" (T007). No other bus file calls `Exchange`
- [X] T015 Implement src/bus/observe.rs (research R-7): `fn observe<V>(state: &mut TorqueState, result: &Result<V, Error>)` sets `*state = state.fault()` when the error is `Error::Status(e)` with `e.is_hardware_fault()`; `fn after_torque_write(state: &mut TorqueState, wanted: TorqueState, result: &Result<(), Error>)` per T008. Run T006–T010 until they pass

**Checkpoint**: Foundation ready. US1 can start.

---

## Phase 3: User Story 1 - Declare my actuators and move one safely (Priority: P1, PR bus-1) 🎯 MVP

**Goal**: `declare` holds the listed actuators with their EEPROM limits, their limit mode and
torque off; one actuator can be enabled, sent a raw goal and read back, and every refusal sends
nothing.

**Independent Test**: Script two declared actuators (XM430-W350 on P2, AX-12A on P1, own bus);
run setup; goal before enable (refused, zero bytes); enable (one acknowledged write); goal inside
limits (byte-exact); goal outside limits (zero bytes); present read (decoded); the same goal
checks in extended position, velocity and AX wheel mode. Quickstart §2 rows US1-1 … US1-13.

### Tests for User Story 1 ⚠️

- [X] T016 [P] [US1] Write declare tests in src/bus/setup.rs (quickstart §2): US1-1 / FR-014 `declare_reads_limits_and_starts_off` (XM430 at ID 1 on `Bus<P2, …>` in Operating Mode 3: ping, Min/Max Position Limit reads at 52/48, Operating Mode read at 11 (1 byte), byte-exact via `p2_instruction`; AX-12A at ID 1 on `Bus<P1, …>`: ping, `MODEL_NUMBER` read (0, 2), CW/CCW Angle Limit reads, no mode read; `torque_states()` all `Off`; no Torque Enable address in `sent()`; "After a successful Setup, every slot is `Off` and every `Cache` has limits and the limit mode read at Setup"); US1-2 `silent_declared_id_is_missing` (`SetupError::Missing { id }`) and `other_model_is_wrong_model` (`WrongModel { id, declared, found }`), `actuators()` empty afterwards; US1-2 early `unsupported_declared_model_sends_nothing` (unknown model number, and XL-320 (350, P2 only) on `Bus<P1, …>` → `UnsupportedModel { protocol: ProtocolVersion::P1, feature: None, .. }`, zero bytes); US1-3 `limit_read_timeout_fails_setup` (`Err(Transport(Timeout))`, list empty, no default used); US1-4 `too_many_declared` (3 entries on `Bus<P2, MockTransport, _, 2>` → `TooManyActuators { capacity: 2 }`, zero bytes); `duplicate_declared_id` (`ValidationError::DuplicateId { id }`, zero bytes); `inverted_limits_fail_setup` (`InvertedLimits`); `fault_at_setup_fails_setup` (ping reply with alert → `Err(Status)`, list empty); FR-015 `second_declare_replaces_and_failed_one_empties`; invariant "`actuators` is strictly ascending by `Id`" for a list given as IDs 3, 1, 2
- [X] T017 [P] [US1] Write `ActuatorRef` tests in src/bus/actuator_ref.rs (actuators in position mode): US1-5 `goal_while_off_is_refused` (`set_goal_position_raw` → `ValidationError::ActuatorIsOff { id }`, zero bytes); US1-6 `goal_outside_limits_is_refused` (after enable, raw below min and above max → `OutOfLimits { id, raw, min, max }`, zero bytes; bounds themselves pass); US1-7 / FR-022 `goal_inside_limits_sends_one_write` (exactly one Write packet to Goal Position 116, 4 bytes LE, equal to `p2_instruction`; `assert_done`, nothing read after); `enable_torque_sends_one_acknowledged_write` (Torque Enable 64 = 1, state `On`); `disable_torque_turns_off`; `present_position_raw_decodes` (4-byte two's complement); `goal_position_raw_reads_back` (FR-3.8)
- [X] T018 [P] [US1] Write tests in src/bus.rs: US1-8 `unknown_id_gives_none` (`actuator(id(9))` is `None`, no panic, zero bytes); `known_id_gives_a_ref` (`kind()` id matches); `torque_state(unknown)` is `None`
- [X] T019 [US1] Write the limit-mode tests (research R-9, module-interfaces §2.4b, quickstart rows FR-013 and US1-11 … US1-13) in src/bus/setup.rs and src/bus/actuator_ref.rs, after T016/T017 (same files). Setup: `mode_is_recorded_at_setup` (XM430 Operating Mode 3, 4, 5, 1, 0 and 16, checked with `cache().check_goal` on a raw value outside the EEPROM limits: 3 → `OutOfLimits`; 4, 5 → `Ok`; 1, 0, 16 → `NotPositionMode`; AX-12A with CW = CCW = 0 → `NotPositionMode` and no mode read in `sent()`); US1-13 `unknown_mode_fails_setup` (XM430 Operating Mode 2 → `SetupError::UnknownOperatingMode { id, value: 2 }`, list empty); `mode_read_timeout_fails_setup` (`Err(Transport(Timeout))`, list empty). Goal path, torque on: US1-11 `extended_mode_accepts_goal_outside_limits` (Operating Mode 4, raw goal above the EEPROM max → one Write packet, byte-exact via `p2_instruction`); US1-12 `velocity_mode_is_not_position_mode` (Operating Mode 1 → `ValidationError::NotPositionMode { id }`, zero bytes) and `ax_wheel_mode_is_not_position_mode` (CW = CCW = 0 → `NotPositionMode { id }`, zero bytes)

### Implementation for User Story 1

- [X] T020 [US1] Implement declared setup in src/bus/setup.rs (research R-6), generic over `Slots`: `fn declare<P, T, C, S: Slots>(wire, slots, list) -> Result<(), Error>`. Before any byte: collect into a stack `heapless::Vec<(Id, ModelNumber), N>` ("Rejects overflow and duplicates before any byte": overflow → `TooManyActuators { capacity }`, repeat → `DuplicateId`); check each model with `ActuatorKind::new(P::VERSION, model, 0, id)` (FR-011, FR-012). Then per entry: ping with retries (`Timeout` → `SetupError::Missing { id }`, other errors propagate); model number from the `Pong` (P2) or a `models::MODEL_NUMBER` read (P1), mismatch → `WrongModel`; a `Pong` or reply with a hardware fault fails Setup; then `init_one`. `slots.clear()` first and again on any error (FR-015)
- [X] T021 [US1] Implement `init_one(wire, slots, id, model, pong)` in src/bus/setup.rs ("`init_one` steps" of data-model.md): 1. firmware from the `Pong` (P2) or a read of `ActuatorKind::firmware_register(P::VERSION, model)` when it is `Some`; 2. `ActuatorKind::new(P::VERSION, model, fw, id)`; 3. read `data().limits.min()` and `.max()` (CW maps to min, CCW to max), `Register::decode`, `cache_mut().record_limits(min, max)`, any error (incl. `InvertedLimits`) fails Setup, never a table default (FR-013); 4. the limit mode (research R-9): if `data().mode.register()` is `Some(r)` (Operating Mode or Control Mode), one 1-byte `wire.read` of `r`, then `data().mode.limit_mode(value)`, where `None` fails Setup with `SetupError::UnknownOperatingMode { id, value }` (a failed read fails Setup too); if it is `None` (`AngleLimits`), no read, `LimitMode::from_angle_limits(limits)` on the limits of step 3; then `cache_mut().record_mode(mode)`; 5. `push` with slot `Off`, no torque write (FR-014)
- [X] T022 [US1] Implement `Bus::declare<I: IntoIterator<Item = (Id, ModelNumber)>>(&mut self, list: I) -> Result<&[ActuatorKind], Error>` in src/bus.rs over `split()` + `setup::declare` (doc: "Setup from a list. Pings only the listed IDs. Replaces the held list; empty on `Err`.", the model, firmware, limits and operating mode read per actuator, and FR-008 "never sweeps the bus")
- [X] T023 [US1] Implement src/bus/actuator_ref.rs: `pub struct ActuatorRef<'b, P: Protocol, T: Transport, C: Clock> { kind: &'b mut ActuatorKind, state: &'b mut TorqueState, wire: Wire<'b, P, T, C> }` (no `N`); `kind(&self) -> &ActuatorKind` (Tier 2, FR-018); `torque_state(&self)`; `enable_torque` (`state.enable(id)?` before any byte, then write `data().torque_enable` = 1, `after_torque_write(…, On, …)`); `disable_torque` (always sent, value 0, `after_torque_write(…, Off, …)`); `set_goal_position_raw(raw)` (`state.check_goal(id)?` → `cache().check_goal(raw)?`, which follows the limit mode (`OutOfLimits`, `NotPositionMode`, or pass) → `Register::encode` (`DoesNotFit`) → `wire.write`, no read-back); `goal_position_raw()` and `present_position_raw()` (`wire.read` + `Register::decode`). Every exchange result passes `observe` (research R-7). Docs per contracts/bus-api.md "ActuatorRef"
- [X] T024 [US1] Implement `Bus::actuator(&mut self, id) -> Option<ActuatorRef<'_, P, T, C>>` in src/bus.rs (find slot, build the ref from `split()`; `None` for an `Id` not held, FR-016). Re-export `ActuatorRef` from src/lib.rs
- [X] T025 [US1] Add the bus-1 compile-fail docs (research R-14, SC-006): on `Bus::actuator` a `compile_fail,E0499` example holding two refs at once (US1-9); on `ActuatorRef` a `compile_fail,E0597` example (or `E0505`, whichever rustc emits; check it) of a ref outliving its bus; on `Bus` a `compile_fail,E0277` example of `Bus::<P2, Uart>::new(uart, …)` without a clock, gated `#[cfg_attr(not(feature = "std"), doc = …)]` in src/bus.rs, src/bus/actuator_ref.rs. Run `cargo test --doc` and the std-off doc line of quickstart §1

**Checkpoint**: US1 works: declared setup with limits and limit mode, torque, raw goal and
present, all refusals zero bytes.

---

## Phase 4: User Story 3 - Notice a hardware fault and recover from it (Priority: P2, PR bus-1)

**Goal**: Any reply with a hardware fault returns `Err(Status)` and marks the slot `Faulted`;
goals and enable are refused while faulted; `clear_fault` asks the actuator.

**Independent Test**: Alert (P2) / overload (P1) on a present read → error, `Faulted`, refused
goal and enable (zero bytes); clear-fault check for "cleared" and "still faulted". Quickstart §2
rows US3-1 … US3-6, FR-028.

### Tests for User Story 3 ⚠️

- [X] T026 [P] [US3] Write fault tests in src/bus/actuator_ref.rs: US3-1 / SC-003 `alert_on_present_read_faults` (P2 error byte `0x80` → `Err(Status)`, `torque_state() == Faulted` on that call) and `p1_overload_on_present_read_faults` (`P1Flags` Overload); `hardware_fault_on_torque_write_faults`; `angle_limit_flag_is_not_a_fault` (P1 `AngleLimit` → `Err(Status)`, state unchanged, research R-9); US3-2 `goal_while_faulted_is_refused` (`ActuatorIsOff`, zero bytes); US3-3 `enable_while_faulted_is_refused` (`ActuatorFaulted { id }`, zero bytes); US3-6 / FR-027 `enable_timeout_leaves_off` and `enable_bad_crc_leaves_off` (never `On`); plan deviation 5 `faulted_disable_timeout_stays_faulted` (then `enable_torque` → `ActuatorFaulted`, zero bytes)
- [X] T027 [P] [US3] Write clear-check tests in src/bus/observe.rs (research R-10): US3-4 XM430 Hardware Error Status (70) read gives 0 → `Off`, torque stays off; US3-5 alert reply → returns `Err(Status)`, stays `Faulted`; `Ok` non-zero → `Err(Status(StatusError { id, report: StatusReport::P2 { alert: true, code: None } }))`, stays `Faulted`; AX-12A (no `hardware_error`) ping clean → `Off`, ping with `Overheating` → `Err(Status)`, stays `Faulted`; `clear_fault` on an `Off` slot still sends the check (FR-028)

### Implementation for User Story 3

- [X] T028 [US3] Implement the clear check in src/bus/observe.rs: `fn check_clear(wire, kind, state) -> Result<(), Error>`: if `data().hardware_error` is `Some(reg)`, `wire.read` it (an alert reply still carries the data, PR #7 `Exchange::read` doc); 0 → `*state = state.clear()`; non-zero → the `StatusError` of T027, state kept; `Err(Status)` → returned, state kept `Faulted` via `observe`; if `None`, `wire.ping` and clear on `Ok`
- [X] T029 [US3] Implement `ActuatorRef::clear_fault(&mut self) -> Result<(), Error>` in src/bus/actuator_ref.rs over `check_clear` (doc: "Asks the actuator (Hardware Error Status, or a ping where the model has none). Clears to `Off` only if it reports no fault; otherwise returns its `Status` error." and the X-series reboot note)
- [X] T030 [US3] Write the module docs of src/bus.rs: FR-029 ("broadcast writes get no reply, so a write-only loop sees faults on its next read", US3-7), "a X-series fault needs a reboot outside this crate" (research R-10), and the single-actuator usage example (declare, actuator, enable, goal, present) as a doctest on `MockTransport` gated `#[cfg_attr(feature = "mock", doc = …)]`

**Checkpoint**: US1 + US3 work: faults are seen on the call that received them and cleared only by asking.

---

## Phase 5: User Story 4 - Work in real units, raw always reachable (Priority: P2, PR bus-1)

**Goal**: Typed goals and reads in radians with the same guard and limit-mode check as raw;
limits visible in both units.

**Independent Test**: XM430-W350 and AX-12A: radians ↔ register value on the wire; limits typed
and raw agree; a typed goal that rounds outside is refused in position mode. Quickstart §2 rows
US4-1 … US4-4.

### Tests for User Story 4 ⚠️

- [X] T031 [P] [US4] Write typed tests in src/bus/actuator_ref.rs: US4-2 `typed_goal_is_the_register_value` (XM430: `Position::ZERO` → raw 2048 on the wire; AX-12A: `Position::ZERO` → raw 512; one non-zero angle each, expected raw from `PositionScale::to_raw`); US4-3 `typed_goal_rounding_outside_is_refused` (position mode, `OutOfLimits`, zero bytes) and `nan_goal_is_not_finite` (`NotFinite`, zero bytes); `typed_goal_follows_limit_mode` (Operating Mode 4: a typed goal outside the EEPROM limits is sent with the raw value from `to_raw`; Operating Mode 1: `NotPositionMode`, zero bytes); US4-4 `present_position_typed_and_raw` (same reply: radians via `scale.to_position`, raw as register value); US4-1 `limits_typed_and_raw_agree` (`position_limits_raw()` equals the setup read, `position_limits()` is its scaled form); `goal_position_typed_reads_back`

### Implementation for User Story 4

- [X] T032 [US4] Implement `set_goal_position(goal: Position)`, `goal_position()` and `present_position()` in src/bus/actuator_ref.rs (research R-19): `state.check_goal(id)` → `scale.to_raw` (rounds to nearest, `NotFinite`, `DoesNotFit`) → `cache.check_goal(raw)` (limit mode) → `Register::encode` → `wire.write`; reads decode with `Register::decode`, then `scale.to_position`
- [X] T033 [US4] Implement `sealed::Describe` for `ActuatorRef` by delegating to its `ActuatorKind`, and `impl<P: Protocol, T: Transport, C: Clock> Actuator for ActuatorRef<'_, P, T, C> {}` in src/bus/actuator_ref.rs (research R-20: `id`, `model_number`, `name`, `position_scale`, `position_limits[_raw]`)
- [X] T034 [US4] Document on `set_goal_position` / `set_goal_position_raw` in src/bus/actuator_ref.rs (contracts/bus-api.md "Documented guarantees"): "Limits and operating mode are those read at Setup. Position mode applies the EEPROM limits. Extended and current-based position modes and MX multi-turn apply none, as the firmware does; only the register size is checked. Velocity, current, PWM and wheel modes refuse every goal (`NotPositionMode`). A mode changed after Setup is not seen until the next Setup."

**Checkpoint**: bus-1 content complete (US1, US3, US4).

---

## Phase 6: Close-out PR bus-1

- [X] T035 Run the bus-1 gates (PR map, gate set G-base) and `cargo llvm-cov … --fail-under-lines 70`; fix fallout only in bus files. PR #7's `expect(dead_code)` on `mod protocol;` and `P2::broadcast_ping` are already gone (core-1, plan deviation 6); `src/lib.rs` changes only by the bus lines
- [X] T036 Write specs/003-bus/pr-bus-1.md from .github/pull_request_template.md: What/Why (SRS steps 4, 5; FR-001…FR-029 as covered, US1/US3/US4), Architecture (plan deviations 1, 2, 5; shared-file touches `src/lib.rs`, `Cargo.toml` `heapless`; six-PR split (T001); limits follow the operating mode, decided 2026-10-28, SRS FR-3.7 patch pending (T087)), checklist with the gate results, the optional quickstart §4 hardware check unchecked. No AI attribution. Push `003-bus-1` and open the PR into `main` with that body

---

## Phase 7: User Story 2 - Find what is on the wire without listing it (Priority: P1, PR bus-2)

**Goal**: `autoscan` (P2 broadcast ping, P1 sweep, no_std too) and `SerialBus::open`.

**Independent Test**: P2 broadcast ping with three replies plus limit and mode reads; P1 sweep
where some IDs answer; time bounds on the test clock. Quickstart §2 rows US2-1 … US2-7, US2 edge.

Branch `003-bus-2` from `origin/main` after bus-1 merged. Add `mod serial;` (`#[cfg(feature = "std")]`) to src/bus.rs.

### Tests for User Story 2 ⚠️

- [X] T037 [P] [US2] Write autoscan tests in src/bus/setup.rs: US2-1 `p2_autoscan_sorts_three` (one broadcast ping `FF FF FD 00 FE 03 00 01 31 42` in `sent()`, pongs for IDs 3, 1, 2 scripted out of order plus their limit and mode reads → three held, ascending; "Sorted by `Id` before `init_one`"); US2-2 / SC-007 `p1_autoscan_empty_is_bounded` (253 pings 0..=252, `time.now() <= 253 * scan_timeout + 10 ms` with `MockTime::ticker`) and `p2_autoscan_empty_is_bounded` (`<= scan_timeout + 1 ms`); `p1_autoscan_some_answer` (IDs 1 and 5 answer: P1 model read for each); US2-3 `firmware_picks_the_table` (XL330-M077 (1190) firmware 52 → `ActuatorKind::Xl330M077Fw52`, 53 → `Xl330M077`); US2-6 `p2_only_model_on_p1_fails_setup` (P1 scan finds ID 4 with model 350 → `UnsupportedModel { id, protocol: P1, feature: None }`, list empty); US2-4 `family_off_names_feature`, gated `#[cfg(not(feature = "pro"))]` (P2 pong from ID 7 with model 2000, PH42-020-S300 → `UnsupportedModel { id, model_number: 2000, protocol: P2, feature: Some("pro") }`, list empty), run by the family-off line of quickstart §1 (`cargo test --no-default-features --features std,mock,ax,mx,x-series,y-series`); edges: `corrupt_scan_reply_fails` (`Err(Protocol)`), `two_pongs_one_id` (`DuplicateId`), `pong_with_alert_fails` (`Err(Status)`), `scan_past_capacity` (`TooManyActuators`), list empty after each; US2-5 empty bus → `Ok(&[])`
- [X] T038 [P] [US2] Write the US2-7 test in src/bus/serial.rs (`#[cfg(all(test, unix))]`): `serialport::TTYPort::pair()`, `SerialBus::<P2>::open(<pty path>, 1_000_000)` returns `Ok`, and the peer end reads nothing (no byte written)

### Implementation for User Story 2

- [X] T039 [US2] Add `Wire::scan(on_reply)` in src/bus/wire.rs: `Link { timeout: config.scan_timeout }`, calls `P::scan`, never retried (research R-5)
- [X] T040 [US2] Implement autoscan in src/bus/setup.rs (research R-6), generic over `Slots`: collect replies into a stack `heapless::Vec<Pong, N>`; keep the first `Err` reply and fail Setup with it; past capacity → `TooManyActuators`; same `Id` twice → `DuplicateId`; `Pong { fault: Some(report) }` → `Error::Status(StatusError { id, report })`; sort by `Id`; per reply: P1 model read of `models::MODEL_NUMBER` (P2 uses `Pong::model_number`), then `init_one` (T021, limits and mode). No `std` gate (FR-009, Q2). Clear before and on any error (FR-015)
- [X] T041 [US2] Implement `Bus::autoscan(&mut self) -> Result<&[ActuatorKind], Error>` in src/bus.rs (doc: "Setup by scanning (P2 broadcast ping, P1 sweep 0..=252). Available without `std`."). Extend the `Config::scan_timeout` doc (research R-11): replies share one wire; each P2 ping reply is 14 bytes, about 140 bit-times plus Return Delay Time; a large chain at low baud needs a longer window; a P1 sweep at the default takes about 2.5 s
- [X] T042 [US2] Implement src/bus/serial.rs (research R-18): `pub type SerialBus<P, const N: usize = 32> = Bus<P, SerialTransport, StdClock, N>;` and `impl<P: Protocol, const N: usize> Bus<P, SerialTransport, StdClock, N> { pub fn open(path: &str, baud: u32) -> Result<Self, serialport::Error> }` (= `SerialTransport::open`, `StdClock::default()`, `Config::new()`, no I/O), docs with a `no_run` example gated `#[cfg_attr(feature = "std", doc = …)]`. Re-export `#[cfg(feature = "std")] pub use bus::SerialBus;` in src/lib.rs

### Close-out PR bus-2

- [X] T043 [US2] Run the bus-2 gates (G-base, now with the family-off line) and coverage; confirm the no_std clippy line builds `autoscan` (US2 no_std row)
- [X] T044 [US2] Write specs/003-bus/pr-bus-2.md (SRS steps 4, 8; FR-006, FR-009…FR-012; `DuplicateId` for scan replies (plan deviation 4); autoscan reads the mode like `declare`; shared-file touch `src/lib.rs`), push `003-bus-2`, open the PR into `main`

**Checkpoint**: US2 works: a desktop user goes from a port name to a set-up bus.

---

## Phase 8: User Story 5 - Command many actuators in one call (Priority: P3, PR bus-3)

**Goal**: Easy-tier torque and goal writes and the explicit `sync_write`, validated as a whole,
one instruction per register layout.

**Independent Test**: XM430 (116/4) + XL320 (30/2) → exactly two byte-exact Sync Writes; one
target `Off` or not in position mode → zero bytes. Quickstart §2 rows US5-1 … US5-6, proptest.

Branch `003-bus-3` from `origin/main` after bus-1 merged (independent of bus-2). Add `mod field; mod group; mod id_set; mod multi;` to src/bus.rs (`pub mod field` re-exported).

### Tests for User Story 5 ⚠️

- [X] T045 [P] [US5] Write `IdSet` tests in src/bus/id_set.rs (data-model.md "IdSet": "Bit `n` set ⇔ `Id(n)` present. Only bits 0..=252 are used."): `new()`/`default()` empty; `range(id(2), id(5))` holds 2..=5, `range(id(5), id(2))` empty, `range(id(0), id(252))` has `len() == 253`; `insert` returns `false` on a repeat; `remove`; `contains`; iteration ascending whatever the insert order; `FromIterator<Id>`; `From<[Id; K]>`; `Copy` (use after move); `size_of::<IdSet>() == 32`
- [X] T046 [P] [US5] Write `group` tests in src/bus/group.rs (research R-8): resolve: unknown `Id` → `SetupError::NotOnBus { id }`, repeat → `ValidationError::DuplicateId { id }`; count: `values.len() != targets` → `BufferLength { targets, got }`; validate in target order, first failure wins (`ActuatorIsOff`, `OutOfLimits`, `NotPositionMode`, `NotFinite`); an `Unbounded` (extended-mode) target outside its EEPROM limits passes and is encoded; groups are "the distinct `(addr, size)` keys of the staged targets, in first-appearance order"; the per-group stage iterator is `Clone`; `size_of::<Stage>() == 8` ("`slot: u8`, `addr: u16`, `size: u8` (1, 2 or 4), `bytes: [u8; 4]`")
- [X] T047 [P] [US5] Write multi-write tests in src/bus/multi.rs: US5-1 one layout → one Sync Write, no reply awaited (`reads` unchanged); US5-2 / SC-004 XM430 (116/4) + XL320 (30/2) → exactly two Sync Write packets, byte-exact, no read; US5-3 / SC-002 one target `Off` → `ActuatorIsOff { id }` naming it, zero bytes to anyone; US5-6 one target in velocity mode → `NotPositionMode { id }` naming it, zero bytes to anyone; US5-4 short value slice, unknown ID, repeated ID → matching error, zero bytes; empty target list → `Ok(())`, zero bytes; `write_goal_positions_raw` same rules; US5-5 `sync_write(GoalPosition, …)` and `sync_write(Raw(GoalPosition), …)` on `Bus<P1, …>` and `Bus<P2, …>`; `enable_torque(ids)`: acknowledged unicast writes in target order, refused with zero bytes if any target is `Faulted`, stops at the first error; `disable_torque(ids)`: tries every target after an error, returns the first error with a hardware fault first, each slot updated by `after_torque_write`; `bus.enable_torque(bus.ids())` compiles (research R-16); proptest `no_goal_byte_for_a_slot_not_on` (random `TorqueState`s and goals over 1–8 held actuators: `write_goal_positions` sends no byte unless every target is `On`)

### Implementation for User Story 5

- [X] T048 [P] [US5] Implement src/bus/id_set.rs per contracts/bus-api.md "IdSet": `#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)] pub struct IdSet { bits: [u32; 8] }`, `const fn new`, `const fn range(first, last)` ("empty if `first > last`"), `insert` ("Returns `false` if already present"), `remove`, `const fn contains`, `const fn len`, `const fn is_empty`, `IntoIterator<Item = Id, IntoIter = IdSetIter>` (ascending), `FromIterator<Id>`, `From<[Id; K]>`, `pub struct IdSetIter` (`Iterator<Item = Id> + Clone`)
- [X] T049 [P] [US5] Implement src/bus/field.rs (public module `field`, research R-15): sealed `ReadField` / `WriteField` (`type Value: Copy`), markers `GoalPosition` (`WriteField`, `Position`, register `goal_position`), `PresentPosition` (`ReadField`, `Position`, register `present_position`), `Raw<F>(pub F)` (`Value = i32`, same register as `F`); crate-internal `fn register(data: &ModelData) -> Register` and value conversion on the sealed trait. `TorqueEnable` is deliberately absent (research R-7)
- [X] T050 [US5] Add `Wire::sync_write(addr, size, entries)` in src/bus/wire.rs: `Link { timeout: config.timeout }` (drain only), calls `P::sync_write`, never retried
- [X] T051 [US5] Implement src/bus/group.rs (research R-8 steps 1–4), one N-free core over slices: `struct Stage { slot: u8, addr: u16, size: u8, bytes: [u8; 4] }`; `resolve` (lookup in target order, repeats via an `IdSet`); `validate_writes` (`state.check_goal(id)` → `scale.to_raw` for typed values → `cache.check_goal(raw)`, which follows each target's limit mode → `Register::encode` into the stage); `groups(stages)` yielding each distinct `(addr, size)` in first-appearance order with a `Clone` iterator of `(Id, &[u8])`. `Bus` calls it with a stack `[Stage; N]`
- [X] T052 [US5] Implement the Easy-tier writes in src/bus/multi.rs on `impl<P: Protocol, T: Transport, C: Clock, const N: usize> Bus<P, T, C, N>`: `enable_torque<I: IntoIterator<Item = Id>>`, `disable_torque`, `write_goal_positions(ids, &[Position])`, `write_goal_positions_raw(ids, &[i32])` per contracts/bus-api.md "Easy tier" and research R-7 (torque: whole list validated first, unicast acknowledged writes via `Wire::write`, each result through `after_torque_write`); goal writes: resolve → count → validate → one `Wire::sync_write` per group; no `observe` (no reply). Docs: "Not atomic across layouts on the wire" (spec edge case) and FR-029
- [X] T053 [US5] Implement `sync_write<F: WriteField, I: IntoIterator<Item = Id>>(&mut self, field: F, ids: I, values: &[F::Value])` in src/bus/multi.rs (same validation as the Easy tier, both protocols, FR-032)
- [X] T054 [US5] Add `Bus::ids(&self) -> IdSet` ("Held IDs, as an owned set (so `bus.enable_torque(bus.ids())` compiles)") in src/bus.rs; re-export `pub use bus::{IdSet, field};` in src/lib.rs; add a multi-write doctest on `MockTransport` gated `#[cfg_attr(feature = "mock", doc = …)]`

### Close-out PR bus-3

- [X] T055 [US5] Run the bus-3 gates (G-base) and coverage
- [X] T056 [US5] Write specs/003-bus/pr-bus-3.md (SRS step 6; FR-030…FR-034, SC-002, SC-004; plan deviation 4 new public items `IdSet`, `field`; torque Easy tier as unicast (spec Assumptions); goal validation follows each target's limit mode; shared-file touch `src/lib.rs`), push `003-bus-3`, open the PR into `main`

**Checkpoint**: US5 works: multi-actuator writes are all-or-nothing at validation.

---

## Phase 9: User Story 6 - Read many actuators into my own buffer (Priority: P3, PR bus-4)

**Goal**: Strict Easy-tier reads and P2-only `sync_read` into a caller buffer, transactional,
zero allocation; plus the SC-001 desktop example, which needs bus-2 and bus-3.

**Independent Test**: P2 one Sync Read in target order; P1 K Reads; slot 1 timeout + slot 3 alert
→ slot 3's fault, slot 3 `Faulted`, buffer unchanged; `sync_read` absent on P1; zero
allocations. Quickstart §2 rows US6-1 … US6-6, SC-005, and §3 (SC-001).

Branch `003-bus-4` from `origin/main` after bus-2 and bus-3 merged.

### Tests for User Story 6 ⚠️

- [X] T057 [P] [US6] Write read tests in src/bus/multi.rs: US6-1 / SC-004 three targets on P2 → one Sync Read packet, `out` in target order (targets given as 3, 1, 2); US6-2 three targets on P1 → three Read packets; US6-3 one target times out → `Err(Timeout)`, `out` unchanged (pre-filled sentinel); US6-4 slot 1 times out and slot 3 alerts → returns slot 3's `Err(Status)`, slot 3 `Faulted`, `out` unchanged (FR-036 "a hardware fault is reported ahead of a timeout"); US6-5 `out` shorter than targets → `BufferLength`, zero bytes; extra `out` slots stay untouched; empty targets → `Ok`, zero bytes; `read_positions` typed (radians) and `read_positions_raw`; XM430 + XL320 mixed → two Sync Reads (one per layout); a transport `Io` error from `read_many` aborts at once; `sync_read(PresentPosition, …)` and `sync_read(Raw(PresentPosition), …)` on `Bus<P2, …>`; SC-003 the alert slot is `Faulted` on that same call
- [X] T058 [US6] Add the US6-6 / SC-006 `compile_fail,E0599` doc example on `sync_read` in src/bus/multi.rs: `Bus<P1, …>::sync_read(PresentPosition, …)` does not compile
- [X] T059 [P] [US6] Write tests/bus_no_alloc.rs (`#![cfg(feature = "mock")]`, research R-12; new file, shared-file touch): a counting `#[global_allocator]` wrapping `std::alloc::System` (like `src/protocol/no_alloc.rs`); set up a `Bus<P2, MockTransport, _>` and a `Bus<P1, …>` through `declare` on the public API; assert zero allocations across `ActuatorRef` `enable_torque`, `set_goal_position(_raw)`, `present_position(_raw)`, `write_goal_positions(_raw)`, `read_positions(_raw)`, `sync_write`, `sync_read` (P2). Count only around the Cycle calls, not the scripting (SC-005, FR-043)

### Implementation for User Story 6

- [X] T060 [US6] Add `Wire::read_many(addr, size, ids, scratch, on_reply)` in src/bus/wire.rs (`config.timeout`, calls `P::read_many`, never retried, research R-5) and a P2-only `Wire::sync_read` over `P2::sync_read` for the explicit tier
- [X] T061 [US6] Add read staging to src/bus/group.rs (research R-8 step 5): count `out.len() >= targets`; per group call `read_many` with a 4-byte stack scratch; a cursor over the group's stages maps each callback (target order, e-Manual Sync Read reply order, research R-11) to its slot; decoded raw values go into the stage; every slot result passes `observe`; bookkeeping "the first error and whether it is a hardware fault, so a hardware fault wins"; a `read_many` `Err` aborts at once
- [X] T062 [US6] Implement `read_positions(ids, &mut [Position])` and `read_positions_raw(ids, &mut [i32])` in src/bus/multi.rs: on success convert and copy every stage into `out` (transactional); otherwise return the first hardware fault in target order, else the first error, with `out` unchanged (FR-035, FR-036). Docs per contracts/bus-api.md
- [X] T063 [US6] Implement `impl<T: Transport, C: Clock, const N: usize> Bus<P2, T, C, N> { pub fn sync_read<F: ReadField, I: IntoIterator<Item = Id>>(&mut self, field: F, ids: I, out: &mut [F::Value]) -> Result<(), Error> }` in src/bus/multi.rs, same staging and strict rules (FR-032: absent on P1)

### Close-out PR bus-4

- [X] T064 [US6] Add the SC-001 five-line desktop example (quickstart §3) as a `no_run` doctest on `Bus` in src/bus.rs, gated `#[cfg_attr(feature = "std", doc = …)]`: `SerialBus::<P2>::open("/dev/ttyUSB0", 1_000_000)?`, `autoscan()?`, `enable_torque([ID])?`, `actuator(ID)` → `set_goal_position(Position::from_radians(0.5))?`, `actuator(ID)` → `present_position()?`. It needs `SerialBus` and `autoscan` (bus-2) and `enable_torque(ids)` (bus-3), both merged before bus-4 branches
- [X] T065 [US6] Run the bus-4 gates (G-base) including `cargo test --features mock` (runs tests/bus_no_alloc.rs) and coverage
- [X] T066 [US6] Write specs/003-bus/pr-bus-4.md (SRS step 6; FR-031, FR-032, FR-035…FR-037, FR-043, SC-001, SC-004, SC-005, SC-006; plan deviation 3 `tests/bus_no_alloc.rs`; retries not applied to multi-reads (research R-5); shared-file touches `src/lib.rs`, `tests/bus_no_alloc.rs` (new)), push `003-bus-4`, open the PR into `main`

**Checkpoint**: US5 + US6: a control loop can write and read many actuators with no allocation.

---

## Phase 10: Setup for PR bus-5 (features)

Branch `003-bus-5` from `origin/main` after bus-4 merged (independent of bus-6).

- [X] T067 Add to `[features]` in Cargo.toml: `partial-reads = []` with comment `# Opt-in: one result per slot for multi-actuator reads.` and `raw-registers = []` with comment `# Opt-in: read/write any register on one actuator, bypassing the bus's checks.` (shared-file touch, module-interfaces §1). Add `#[cfg(feature = "partial-reads")] mod partial;` and `#[cfg(feature = "raw-registers")] mod raw;` to src/bus.rs with empty files src/bus/partial.rs and src/bus/raw.rs

---

## Phase 11: User Story 7 - Read many actuators and see which ones failed (Priority: P4, PR bus-5)

**Goal**: `read_positions_partial` / `read_positions_raw_partial` behind `partial-reads`.

**Independent Test**: P1 silent slot → only that slot fails; P2 group instruction failure → every
slot of the group fails; P2 one silent ID → that slot only (FR-4.7, decided 2026-10-28); absent
without the feature. Quickstart §2 rows US7-1 … US7-4.

### Tests for User Story 7 ⚠️

- [X] T068 [P] [US7] Write tests in src/bus/partial.rs: US7-2 P1, one silent target → that slot `Err(Transport(Timeout))`, others `Ok(value)`; US7-3 P2, the group's instruction fails (`fail_with(BrokenPipe)` → `read_many` `Err`) → every slot of that group holds that error; US7-4 / FR-4.7 (decided 2026-10-28) P2 sync read with one ID silent → that slot `Err(Timeout)`, others `Ok`; alert slot → `Err(Status)` and slot `Faulted`; validation errors (unknown, duplicate, short `out`) still fail the whole call with zero bytes; typed and raw forms
- [X] T069 [P] [US7] Add the US7-1 `compile_fail,E0599` doc example (`read_positions_partial` without the feature) on `Bus` in src/bus.rs, gated `#[cfg_attr(not(feature = "partial-reads"), doc = …)]`
- [X] T070 [P] [US7] Add a `#[cfg(feature = "partial-reads")]` zero-allocation case for `Bus::read_positions_partial` to tests/bus_no_alloc.rs (research R-12; shared-file touch listed in the plan's bus-5 row)

### Implementation for User Story 7

- [X] T071 [US7] Implement src/bus/partial.rs (research R-8 step 6) on `impl<P: Protocol, T: Transport, C: Clock, const N: usize> Bus<P, T, C, N>`: `read_positions_partial(ids, out: &mut [Result<Position, Error>])` and `read_positions_raw_partial(ids, out: &mut [Result<i32, Error>])`, reusing the T061 staging; a group whose `read_many` returns `Err` gives that error to all its slots; per-ID errors inside a P2 sync read stay in their own slot (architect, 2026-10-28); faults mark state via `observe`. Docs per contracts/bus-api.md "`partial-reads` feature" (SRS FR-4.7, decided 2026-10-28)
- [X] T072 [US7] If bus-6 is already merged (bus-5 merges second), add the `#[cfg(feature = "partial-reads")]` `DynBus` wrappers `read_positions_partial` / `read_positions_raw_partial` in src/bus/dyn_bus.rs (contracts/bus-api.md `DynBus`) and their zero-allocation case in tests/bus_no_alloc.rs; otherwise skip it, T084 adds them in bus-6

**Checkpoint**: US7 works with the feature on and is absent with it off.

---

## Phase 12: User Story 8 - Reach any register when the typed API does not cover it (Priority: P4, PR bus-5)

**Goal**: `ActuatorRef::read_raw` / `write_raw` behind `raw-registers`: no guard, no limit check,
no state change on what is sent; replies still pass `observe`.

**Independent Test**: Raw write while `Off` sent byte-exact, state still `Off`; raw read back; raw
alert reply marks the slot `Faulted`; absent without the feature. Quickstart §2 rows US8-1 … US8-3.

### Tests for User Story 8 ⚠️

- [X] T073 [P] [US8] Write tests in src/bus/raw.rs: US8-2 `raw_write_while_off_is_sent` (`write_raw(116, &[0x00, 0x08, 0x00, 0x00])` on an `Off` slot → one Write packet byte-exact, no guard, no limit check, state still `Off`); `raw_torque_write_does_not_change_state` (`write_raw(64, &[1])` → sent, `torque_state()` still `Off`, FR-8.2); `raw_read_returns_bytes` (`read_raw(132, &mut [0; 4])`); US8-3 `raw_alert_reply_faults` (alert reply to `read_raw` and to `write_raw` → `Err(Status)` and `torque_state() == Faulted` on that call, like every other path, architect 2026-10-28); retries apply as for any unicast
- [X] T074 [P] [US8] Add the US8-1 `compile_fail,E0599` doc examples (`read_raw`, `write_raw` without the feature) on `ActuatorRef` in src/bus/actuator_ref.rs, gated `#[cfg_attr(not(feature = "raw-registers"), doc = …)]`

### Implementation for User Story 8

- [X] T075 [US8] Implement src/bus/raw.rs (research R-21): `#[cfg(feature = "raw-registers")] impl<P: Protocol, T: Transport, C: Clock> ActuatorRef<'_, P, T, C> { pub fn read_raw(&mut self, addr: u16, out: &mut [u8]) -> Result<(), Error>; pub fn write_raw(&mut self, addr: u16, data: &[u8]) -> Result<(), Error>; }` through `Wire` (crate-internal raw-address variants of `read`/`write`); each result passes `observe`, so a hardware fault sets `Faulted`; no torque guard, no limit check, no cache change (FR-039). Docs per contracts/bus-api.md "`raw-registers` feature": a reported fault is returned and recorded like on every other path; a raw write to Operating Mode leaves the recorded limit mode stale until the next Setup, and a raw write to Torque Enable leaves `torque_state()` as it was (FR-8.2, documented escape hatch)

### Close-out PR bus-5

- [X] T076 [US8] Run the bus-5 gates (G-full: G-base plus the `partial-reads` / `raw-registers` lines of quickstart §1, the std-off doctest line with `+ partial-reads,raw-registers`) and coverage with `--features mock,partial-reads,raw-registers`
- [X] T077 [US8] Write specs/003-bus/pr-bus-5.md (SRS step 9; FR-038, FR-039, FR-041; FR-4.7 one result per actuator and raw replies running `observe`, both decided 2026-10-28, SRS patch pending (T087); `DynBus` partial wrappers here only if bus-6 merged first (T072); shared-file touches `Cargo.toml` features, `src/lib.rs`, `tests/bus_no_alloc.rs`), push `003-bus-5`, open the PR into `main`

**Checkpoint**: US7 and US8 work behind their features; builds without them are unchanged.

---

## Phase 13: User Story 9 - Hold any number of actuators on a desktop (Priority: P5, PR bus-6)

**Goal**: `DynBus<P, T, C = StdClock>` under `std`, same methods as `Bus`, storage grown at
Setup only.

**Independent Test**: `DynBus` autoscan of 40 actuators (more than 32), all held; zero
allocations per Cycle; absent without `std`. Quickstart §2 rows US9-1, US9-2, SC-005.

Branch `003-bus-6` from `origin/main` after bus-4 merged (independent of bus-5). Add `#[cfg(feature = "std")] mod dyn_bus;` to src/bus.rs.

### Tests for User Story 9 ⚠️

- [X] T078 [P] [US9] Write tests in src/bus/dyn_bus.rs: US9-1 `dyn_bus_holds_40` (P2 autoscan, 40 pongs scripted as one reply chunk plus 80 limit reads and 40 mode reads on `MockTransport::<'_, 4096, 4096, 128>::with_capacity` → 40 held, ascending); parity with `Bus` on `declare`, `actuator`, `ids`, `torque_state(s)`, `enable_torque`, `write_goal_positions`, `read_positions`, `sync_write`, `sync_read` (P2) using the same scripts as T016/T047/T057; failed Setup leaves all three Vecs empty
- [X] T079 [P] [US9] Add the US9-2 `compile_fail` doc example (naming `dxl_rs_v2::DynBus` without `std`, `E0412` or `E0433`, whichever rustc emits) gated `#[cfg_attr(not(feature = "std"), doc = …)]`, on `Bus` in src/bus.rs
- [X] T080 [P] [US9] Add `DynBus` cases to tests/bus_no_alloc.rs: set up through `declare` (allocations allowed there), then zero allocations across the same Cycle calls as T059

### Implementation for User Story 9

- [X] T081 [US9] Add `fn stages(&mut self) -> &mut [Stage]` to the `Slots` trait in src/bus/slots.rs ("Staging for multi-actuator ops (`DynBus` only; `Bus` uses a stack array)"), and switch the multi-op engine in src/bus/group.rs / src/bus/multi.rs to take staging from the caller so `Bus` passes its stack `[Stage; N]` and `DynBus` its Vec
- [X] T082 [US9] Implement `DynSlots<'a>` in src/bus/dyn_bus.rs borrowing `DynBus`'s `actuators: alloc::Vec<ActuatorKind>`, `states: alloc::Vec<TorqueState>`, `stages: alloc::Vec<Stage>` ("length = actuator count after Setup"); `push` grows the Vecs (Setup only), `clear` empties them. "The Vecs grow only during Setup. No Cycle call allocates." No limit-mode Vec: the mode is in each `Cache`
- [X] T083 [US9] Implement `pub struct DynBus<P: Protocol, T: Transport, C = StdClock>` in src/bus/dyn_bus.rs with the same public methods, signatures and behaviour as `Bus` without `N` (contracts/bus-api.md `DynBus`): `new`, `into_parts`, `config`, `declare`, `autoscan`, `actuators`, `ids`, `actuator` (returns the same `ActuatorRef`), `torque_state`, `torque_states`, `enable_torque`, `disable_torque`, `write_goal_positions(_raw)`, `read_positions(_raw)`, `sync_write`, and `impl<T, C> DynBus<P2, T, C> { sync_read }`, all thin wrappers over the shared engine (research R-17). Docs: "A bus with no capacity limit. Storage grows at Setup only."; `std` usage doctest gated `#[cfg_attr(feature = "std", doc = …)]`. Re-export `#[cfg(feature = "std")] pub use bus::DynBus;` in src/lib.rs
- [X] T084 [US9] If bus-5 is already merged (bus-6 merges second), add the `#[cfg(feature = "partial-reads")]` `DynBus::read_positions_partial` / `read_positions_raw_partial` in src/bus/dyn_bus.rs and their zero-allocation case in tests/bus_no_alloc.rs; otherwise skip it, T072 adds them in bus-5

### Close-out PR bus-6

- [X] T085 [US9] Run the bus-6 gates (G-base, or G-full if bus-5 is merged) including `cargo test --features mock` and coverage; check `Bus`'s public API is unchanged (FR-040, FR-041)
- [X] T086 [US9] Write specs/003-bus/pr-bus-6.md (SRS step 10; FR-040, FR-041, FR-043; spec conflict 10, `DynBus` as its own type (research R-17); `DynBus` partial wrappers here only if bus-5 merged first (T084); shared-file touches `src/lib.rs`, `tests/bus_no_alloc.rs`), push `003-bus-6`, open the PR into `main`

**Checkpoint**: All nine stories work.

---

## Phase 14: Polish & Cross-Cutting Concerns

- [ ] T087 [human] Architect: patch `docs/srs/SRS.md` (human-only path) to this module's decisions and deviations (plan.md "Deviations from the SRS"): decided 2026-10-28: FR-3.7 and FR-2.6 (limits follow the operating mode), FR-4.7 (one result per actuator inside a P2 sync group), FR-8.1, FR-8.2 and FR-6.2 (raw replies run the fault check; raw writes do not update mode or torque state); earlier: FR-1.1 (no `Clock` bound on the struct), FR-1.3 (`DynBus` its own type), FR-1.4 (scan wait), FR-1.5 (`ActuatorRef<'_, P, T, C>`), FR-2.4 (`SetupError::UnsupportedModel` with `id`, `protocol`), FR-2.7 (fault at Setup fails Setup), FR-4.2 (torque Easy tier as unicast), FR-4.5 (plain-value buffer), FR-4.8 (validation all-or-nothing, wire not), FR-6.4 (new variants), FR-6.5 (P1 ping, enable refused while faulted), 2.1 (setup category), 3.3, 4.1, §7, §8 (`Bus<P, T, C, N>`, `new(transport, clock, config)`, `torque_state()`, clock as third parameter); then regenerate `docs/srs/modules/*.md` with `/speckit-ov-srs`
- [ ] T088 [human] Architect: update `docs/ARCHITECTURE.md` (architect-owned) to `Bus<P, T, C = DefaultClock, const N: usize = 32>`, torque state in a bus-owned byte array, the limit mode in each actuator's cache (module-interfaces §2.4b), autoscan in no_std, `DynBus` (module-interfaces §8 conflict 20, §9)
- [X] T089 [P] Check with the ci module (specs/007-ci) that `.github/workflows/ci.yml` runs `cargo test --features mock` (tests/bus_no_alloc.rs), the family-off test leg of quickstart §1, the `partial-reads` / `raw-registers` feature-set legs and the std-off doctest pass (research R-12); raise any gap to the ci owner, do not edit the workflow from a bus PR
- [ ] T090 Run the manual hardware check of specs/003-bus/quickstart.md §4 (XM430-W350 on P2 in modes 3, 4 and 1, AX-12A on P1, U2D2) if hardware is available; otherwise leave it unchecked in the PR descriptions

---

## PR map

Gate sets (from quickstart.md §1):

- **G-base** (bus-1 … bus-4, bus-6 before bus-5): `cargo fmt --check`;
  `cargo clippy --all-targets -- -D warnings`; `cargo test --workspace`;
  `cargo test --features mock`; `cargo test --doc`;
  `cargo test --doc --no-default-features --features alloc,ax,mx,x-series,y-series,pro,mock`;
  from bus-2 on, the family-off leg
  `cargo test --no-default-features --features std,mock,ax,mx,x-series,y-series`;
  `cargo clippy --no-default-features --features ax,mx,x-series,y-series,pro,mock --target thumbv7em-none-eabihf -- -D warnings`;
  `RUSTFLAGS="-D warnings" cargo build --lib --no-default-features`;
  `RUSTFLAGS="-D warnings" cargo build --lib --no-default-features --features std`;
  `RUSTFLAGS="-D warnings" cargo build --lib --no-default-features --target thumbv7em-none-eabihf`;
  `cargo xtask codegen && git diff --exit-code src/models.rs src/models/`;
  `cargo llvm-cov --workspace --features mock --ignore-filename-regex 'src/models(\.rs|/)' --fail-under-lines 70`.
- **G-full** (bus-5, and bus-6 once bus-5 is merged): G-base plus
  `cargo clippy --all-targets --features mock,partial-reads,raw-registers -- -D warnings`;
  `cargo test --features mock,partial-reads,raw-registers`; `cargo test --doc --all-features`;
  the std-off doctest line with `,partial-reads,raw-registers` added;
  `cargo clippy --no-default-features --features ax,mx,x-series,y-series,pro,partial-reads,raw-registers --target thumbv7em-none-eabihf -- -D warnings`;
  `RUSTFLAGS="-D warnings" cargo build --lib --no-default-features --features partial-reads` and
  `--features raw-registers`, each also with `--target thumbv7em-none-eabihf`.

| PR | Branch (off `main`) | Tasks | Count | Stories | Must be merged first | Gates |
|---|---|---|---|---|---|---|
| bus-1 | `003-bus-1` | T001–T036 | 36 | US1, US3, US4 | core-1, tables T1, xtask X1 (regenerated `src/models*` with `ModelData::mode`), core-2; protocol PR #7 is merged | G-base |
| bus-2 | `003-bus-2` | T037–T044 | 8 | US2 | bus-1 | G-base (with the family-off leg) |
| bus-3 | `003-bus-3` | T045–T056 | 12 | US5 | bus-1 | G-base |
| bus-4 | `003-bus-4` | T057–T066 | 10 | US6 (+ SC-001) | bus-2, bus-3 | G-base |
| bus-5 | `003-bus-5` | T067–T077 | 11 (T072 only if bus-6 merged first) | US7, US8 | bus-4 | G-full |
| bus-6 | `003-bus-6` | T078–T086 | 9 (T084 only if bus-5 merged first) | US9 | bus-4 | G-base (G-full if bus-5 merged) |
| none | — | T087–T090 | 4 | — | T087, T088, T089 any time; T090 after bus-2 | — |

Total: 90 tasks. T001 (split approval) is answered before T036 opens bus-1; T002–T005 start
bus-1.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: T001 is human and runs alongside; T002 → T003 → T004 → T005.
- **Foundational (Phase 2)**: after Setup. Blocks every story.
- **US1 (Phase 3)**: after Phase 2. **US3 (Phase 4)** and **US4 (Phase 5)**: after US1 (they extend
  `ActuatorRef`); US3 and US4 touch the same file, so run them one after the other.
- **Close-out bus-1 (Phase 6)**: after US1, US3, US4.
- **US2 (Phase 7, bus-2)** and **US5 (Phase 8, bus-3)**: each after bus-1 merges; independent of
  each other (both add one `mod` line to src/bus.rs and one `pub use` line to src/lib.rs; rebase
  the second).
- **US6 (Phase 9, bus-4)**: after bus-2 and bus-3 merge (bus-3 for the engine, bus-2 for the
  SC-001 example, T064).
- **bus-5 (Phases 10, 11, 12)** and **US9 (Phase 13, bus-6)**: each after bus-4 merges; independent
  of each other except T072/T084: whichever lands second adds the `DynBus` partial wrappers and
  their no-alloc case.
- **Polish (Phase 14)**: T087, T088, T089 any time; T090 after bus-2.

### User Story Dependencies

- US1: foundation only. US3, US4: US1. US2: bus-1 (US1 setup engine `init_one`). US5: bus-1.
  US6: US5 (`group.rs`, `field.rs`, `IdSet`); its PR also needs US2 for SC-001. US7: US6 (read
  staging). US8: US1 (`ActuatorRef`). US9: US6 (whole engine).

### Within Each Story

- Tests first; they fail or do not compile before the implementation tasks.
- bus-1: T006–T010 → T011 → T012 → T013 → T014 → T015 → T016–T018 → T019 → T020 → T021 → T022 → T023 → T024 → T025 → T026–T027 → T028 → T029 → T030 → T031 → T032 → T033 → T034 → T035 → T036.
- bus-2: T037–T038 → T039 → T040 → T041 → T042 → T043 → T044.
- bus-3: T045–T047 → T048, T049 (parallel) → T050 → T051 → T052 → T053 → T054 → T055 → T056.
- bus-4: T057, T059 (parallel), then T058 → T060 → T061 → T062 → T063 → T064 → T065 → T066.
- bus-5: T067 → T068–T070 → T071 → T072 → T073–T074 → T075 → T076 → T077.
- bus-6: T078–T080 → T081 → T082 → T083 → T084 → T085 → T086.

---

## Parallel Examples

### Foundation (bus-1)

```bash
Task: "T006 status/instruction builders and fixtures in src/bus/testing.rs"
Task: "T007 retry tests in src/bus/wire.rs"
Task: "T008 observe and torque-write rule tests in src/bus/observe.rs"
Task: "T009 FixedSlots tests in src/bus/slots.rs"
Task: "T010 Config and new/into_parts tests in src/bus.rs"
```

### User Story 1

```bash
Task: "T016 declare tests in src/bus/setup.rs"
Task: "T017 ActuatorRef raw tests in src/bus/actuator_ref.rs"
Task: "T018 actuator(id) tests in src/bus.rs"
# then T019 (limit-mode tests, same files as T016/T017)
```

### User Story 3

```bash
Task: "T026 fault tests in src/bus/actuator_ref.rs"
Task: "T027 clear-check tests in src/bus/observe.rs"
```

### User Story 4

```bash
Task: "T031 typed position tests in src/bus/actuator_ref.rs"
```

### User Story 2

```bash
Task: "T037 autoscan tests in src/bus/setup.rs"
Task: "T038 SerialBus::open pty test in src/bus/serial.rs"
```

### User Story 5

```bash
Task: "T045 IdSet tests in src/bus/id_set.rs"
Task: "T046 group tests in src/bus/group.rs"
Task: "T047 multi-write tests in src/bus/multi.rs"
# then, in parallel:
Task: "T048 IdSet in src/bus/id_set.rs"
Task: "T049 field markers in src/bus/field.rs"
```

### User Story 6

```bash
Task: "T057 read tests in src/bus/multi.rs"
Task: "T059 counting allocator test in tests/bus_no_alloc.rs"
# then T058 (compile_fail sync_read on P1, same file as T057)
```

### User Stories 7 and 8 (bus-5)

```bash
Task: "T068 partial-read tests in src/bus/partial.rs"
Task: "T069 compile_fail partial read without feature in src/bus.rs docs"
Task: "T070 partial-read no-alloc case in tests/bus_no_alloc.rs"
Task: "T073 raw tests in src/bus/raw.rs"
Task: "T074 compile_fail raw without feature in src/bus/actuator_ref.rs docs"
```

### User Story 9 (bus-6, parallel with bus-5)

```bash
Task: "T078 DynBus tests in src/bus/dyn_bus.rs"
Task: "T079 compile_fail DynBus without std in src/bus.rs docs"
Task: "T080 DynBus no-alloc cases in tests/bus_no_alloc.rs"
```

---

## Implementation Strategy

### MVP First (PR bus-1)

1. T001 to the architect; T002 confirms every prerequisite is on `main`.
2. Phases 1–2: dependency, skeleton, foundation (`Wire`, `Slots`, `observe`, `Config`, `Bus`).
3. Phase 3 (US1): declared setup with limits and limit mode, one actuator, raw values. **Stop and
   validate** US1 alone.
4. Phases 4–5 (US3, US4): faults and typed units, so `main` never holds a bus that ignores faults
   (plan deviation 2).
5. Phase 6: gates, PR bus-1. This is the MVP: a declared bus that moves one motor safely.

### Incremental Delivery

1. bus-2 (autoscan, `SerialBus`) and bus-3 (multi-writes) in parallel after bus-1.
2. bus-4 (strict reads, `sync_read`, zero-allocation proof, SC-001 example) after both.
3. bus-5 (opt-in features) and bus-6 (`DynBus`) in parallel after bus-4; the second of the two
   adds the `DynBus` partial wrappers.
4. Each PR passes its gate set and ≥ 70 % coverage on its own.

### Parallel Team Strategy

After bus-1: developer A on bus-2, developer B on bus-3. bus-4 after both. After bus-4: A on
bus-5, B on bus-6.

---

## Notes

- [P] = different files or independent test functions, no dependency on unfinished tasks.
- Commit per task group on the PR branch, staging only that PR's files. No AI attribution.
- Every refusal test asserts zero bytes; every fault test asserts the state on the same call.
- Do not run `cargo xtask codegen` to change generated files; it is only a no-diff check.
