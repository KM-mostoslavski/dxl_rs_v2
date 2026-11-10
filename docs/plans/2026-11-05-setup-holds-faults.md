# Setup holds hardware-faulted actuators: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. In this repo the bus row runs in mode `loop`: the build is a `/goal` loop under `AGENTS.md`.

**Goal:** Setup (`declare` and `autoscan`, on `Bus` and `DynBus`) holds an actuator whose replies carry only hardware fault flags as `TorqueState::Faulted` instead of failing the whole bus, and returns the faulted `Id`s beside the held list, so the caller can fix the actuator's EEPROM with `write_raw` after Setup.

**Architecture:** Setup reads tolerate a reply whose error byte is hardware flags only: the reply's data is used (as `diagnose.rs` already does since 2026-11-04) and the actuator's slot is pushed `Faulted` instead of `Off`. Any other error byte still fails Setup. Setup does not measure a held fault; `clear_fault()` already measures it on request.

**Tech Stack:** Rust 2024, `heapless`, `MockTransport` (feature `mock`) for every test.

**Spec:** `docs/deferred.md` D-01; `docs/srs/SRS.md` FR-2.7, FR-6.2, FR-6.5, § 8 "Typed EEPROM writes"; `specs/ideas/eeprom-writes.md` ("A way to reach a faulted actuator for recovery"); kmr `docs/deferred.md` row B2.

## Triage

Changes a merged contract (`declare`/`autoscan` return type and FR-2.7), so per `PROCEDURE.md` it re-enters the cycle at step 3: this plan, then T091 (architect), then `/contract bus` (T092), then the loop (T093 to T095), step 6, step 7. Breaking change before 1.0: call it out in the PR.

## Decisions for the architect (T091)

The pub surface and the bus's fault handling are not an agent's call (`AGENTS.md`). Each one below has a proposed default, in the SRS 5.0 sense: built as written unless overruled.

- **D1. Hold by default.** `declare` and `autoscan` themselves hold the faulted actuator; no opt-in variant. Safety is unchanged: a `Faulted` actuator refuses `enable_torque` and every goal with nothing sent (FR-6.5), so holding it moves nothing. Rejected: a second pair `declare_holding`/`autoscan_holding`, which doubles four Setup entry points to keep a behaviour whose only effect is to lock the caller out of recovery.
- **D2. Return the faulted `Id`s, not the measured faults.** New `pub struct SetupReport<'a> { pub actuators: &'a [ActuatorKind], pub faulted: IdSet }`: 32 bytes for the set, heap-free, one type for `Bus` and `DynBus`. The measured fault (voltage measured against its limits, etc.) comes from `bus.actuator(id)?.clear_fault()`, which already returns the diagnosed `FaultError`; SRS line 78 already says "Hardware Error Status can be read again". Rejected: `heapless::Vec<FaultError, N>` by value, 56 bytes × N (1.8 KB at N = 32) on the stack of a microcontroller, and a separate type for `DynBus`.
- **D3. "Only hardware flags" means:** Protocol 1, the error byte has no bit outside Input Voltage, Overheating and Overload (`P1Flags::hardware()`); Protocol 2, the alert bit with no error code. Angle Limit, Range, Checksum, Instruction (P1) or any P2 error code still fails Setup, as today: the reply's data cannot be trusted.

Proposed SRS text, for the architect to adjust and write (SRS is human-only):

- **FR-2.7** At Init, every actuator's torque state (one byte per slot, held by the bus) starts off, whatever the hardware reports. An actuator whose Setup replies report only hardware faults (Protocol 1: Input Voltage, Overheating, Overload; Protocol 2: the alert bit with no error code) is held with torque state faulted, and Setup returns its `Id` beside the held list; `clear_fault()` (FR-6.5) measures and returns the fault. Any other error in a Setup reply fails Init with that error. Decided <date> (was: a hardware fault fails Init). **\[decision\]** the hardware's actual Torque Enable is not read at Init; a user who needs it reads it explicitly.
- **FR-6.2**, append: "Setup is the exception: it holds the actuator faulted and returns its `Id` (FR-2.7)."
- **§ 8 Typed EEPROM writes**, the "Meanwhile" sentence: "Setup keeps failing on a fault (FR-6.2)" → "Setup holds such an actuator `Faulted` (FR-2.7), reachable with `write_raw`".

## Global Constraints

- `no_std` and heap-free: nothing added allocates; `SetupReport` holds a slice and an `IdSet` only (Principle I).
- No panic on any I/O or hardware error; `Error` stays `Copy` and ≤ 64 bytes (`src/error.rs` size test).
- Torque is never written at Setup; a held actuator is `Off` or `Faulted`, nothing else (Principle III).
- No-go files: `src/models.rs`, `src/models/**`, `control_tables/**`. `src/error.rs` is core's: put the D3 predicate in `src/bus/setup.rs` as a private helper.
- Branch `feature/bus` from `dev`. Commits `feat(bus): T09n <what>`, no AI attribution.
- Every internal choice gets a `// DECISION:` line; green = every command in `AGENTS.md` § Commands.

## Review Focus

1. A P1 reply mixing a hardware flag with Instruction, Range, Checksum or Angle Limit: Setup must still fail, the data is not trusted (T093 declare, T094 autoscan).
2. A P2 reply with the alert bit and an error code (e.g. `0x84`, Data Range): Setup must still fail (T093).
3. A fault reported only on a later Setup read (Operating Mode), with a clean ping: the actuator is still held `Faulted` (T093).
4. A held-faulted actuator whose data fails a Setup check (unknown mode value): Setup fails with that check's error, the fault does not mask it (T093).
5. A whole-bus call over `bus.ids()` with one held-faulted actuator (`enable_torque`): refused with `ActuatorFaulted`, nothing sent (T095).

Bench, not mock (add to T090's run): an AX-12A below its Min Voltage Limit. Whether the reply to the `write_raw` of the new limit still carries Input Voltage, and whether `clear_fault()` then clears without a power cycle, is hardware behaviour no e-Manual page pins.

---

### Task T091 [human]: Architect: decide D1 to D3, amend the SRS

- [x] Accept or overrule D1, D2, D3 above. Accepted 2026-11-10 (maintainer delegated).
- [x] Write FR-2.7, FR-6.2 and the § 8 sentence in `docs/srs/SRS.md`. This supersedes the "FR-2.7 (fault at Setup fails Setup)" item of T087.
- [ ] Triage D-01 in `docs/deferred.md` at the next `/start` as `next`, module `bus`, step 4.

Gate: T092 does not start before this. Building against today's FR-2.7 would be a conflict, which is a deferral.

### Task T092: Contract (step 4, `/contract bus`)

**Files:**
- Modify: `src/bus/setup.rs` (new `pub struct SetupReport<'a>`, test stubs), `src/bus.rs:391-429` (`declare`, `autoscan`, re-export), `src/bus/dyn_bus.rs:136-157`, `src/lib.rs:24-28` (re-export beside `Bus`)
- Modify callers: `examples/{ax12_sine,ax_wheel,mx64_sine,rx28_gain_ramp,rx28_sine,rx28_telemetry}.rs`, the doc example in `src/bus/serial.rs:14`, tests that use the returned slice

**Interfaces:**
- Produces:
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq)] #[non_exhaustive] pub struct SetupReport<'a> { pub actuators: &'a [ActuatorKind], pub faulted: IdSet }`, every field documented.
  - `Bus::declare<I>(&mut self, list: I) -> Result<SetupReport<'_>, Error>`, `Bus::autoscan(&mut self) -> Result<SetupReport<'_>, Error>`, same two on `DynBus`.

- [ ] **Step 1:** Add `SetupReport` and change the four signatures. Bodies keep today's behaviour with `faulted: IdSet::new()` (`// DECISION:` contract keeps the old behaviour until T093).
- [ ] **Step 2:** Update every caller: `bus.autoscan()?` used as a slice becomes `bus.autoscan()?.actuators`. Find them with `cargo build --workspace --all-targets --all-features`.
- [ ] **Step 3:** Rename the four tests that pin the old behaviour and empty them to `todo!()`: `fault_at_setup_fails_setup` → `fault_at_setup_holds_faulted`, `p1_model_read_fault_fails_setup` → `p1_model_read_fault_holds_faulted`, `pong_with_alert_fails` → `pong_with_alert_is_held`, `p1_scan_voltage_fault_is_measured_below_min` → `p1_scan_voltage_fault_is_held_and_measured_by_clear_fault`. Add `todo!()` stubs for every other test named in T093 to T095.
- [ ] **Step 4:** `cargo build --workspace --all-targets --all-features` green, `cargo test --workspace --all-features --no-run` compiles. Print the pub diff in the summary.
- [ ] **Step 5:** Commit `feat(bus)!: T092 Setup returns a SetupReport`.

### Task T093: Declared Setup holds a hardware-faulted actuator

**Files:**
- Modify: `src/bus/setup.rs` (`declare_all`, `init_one`, new private helpers), `src/bus/slots.rs` (`Slots::push`), `src/bus/dyn_bus.rs:53` (its `push`)
- Test: `src/bus/setup.rs` tests, `mod x_series` and `mod ax`

**Interfaces:**
- Consumes: `SetupReport` (T092).
- Produces (crate-private, used by T094):
  - `fn hardware_only(status: &StatusError) -> bool` (D3).
  - `Slots::push(&mut self, kind: ActuatorKind, bound: Bound, state: TorqueState) -> Result<(), SetupError>`
  - `init_one(..., fault: bool) -> Result<bool, Error>`: takes whether the ping or Model Number reply was hardware-only faulted, returns whether the actuator is held `Faulted`.
  - `setup::declare` / `setup::autoscan` return `Result<IdSet, Error>` (the faulted set); `Bus`/`DynBus` wrap it in `SetupReport`.

- [ ] **Step 1: Write the failing tests** (XM430 at ID 1, mode 3, replies scripted like `xm430_p2` but with error byte `0x80`):

```rust
// fault_at_setup_holds_faulted: alert on ping, both limits and the mode
let report = bus.declare([(id(1), model(XM430))]).unwrap();
assert_eq!(report.faulted, ids(&[1]));
assert_eq!(report.actuators[0].position_limits_raw(), Some(1000..=3000));
assert_eq!(bus.torque_states(), [TorqueState::Faulted]);
assert_eq!(sent(&bus), xm430_p2_sent(1)); // no diagnosis traffic at Setup

// fault_on_mode_read_only_holds_faulted: clean ping and limits, 0x80 on the mode reply
assert_eq!(report.faulted, ids(&[1]));

// p2_alert_with_error_code_fails_setup: 0x84 on the ping reply
assert!(bus.declare([(id(1), model(XM430))]).is_err());
assert!(bus.actuators().is_empty());

// faulted_with_unknown_mode_fails_setup: 0x80 on every reply, mode value 99
assert_eq!(
    bus.declare([(id(1), model(XM430))]).map(|r| r.faulted),
    Err(SetupError::UnknownOperatingMode { id: id(1), value: 99, register: "Operating Mode" }.into()),
);

// second_declare_clears_a_held_fault: faulted declare, then a clean one
assert_eq!(report.faulted, IdSet::new());
assert_eq!(bus.torque_states(), [TorqueState::Off]);

// p1_model_read_fault_holds_faulted (old test's line, Model Number reply 0x20, rest scripted clean)
assert_eq!(report.faulted, ids(&[1]));
assert_eq!(bus.torque_states(), [TorqueState::Faulted]);

// p1_instruction_and_voltage_flags_fail_setup: Model Number reply 0x41
// expected: the exact error dev returns today for this reply (run it on dev first, pin that value)
assert!(bus.actuators().is_empty());
```

- [ ] **Step 2:** `cargo test --features mock --lib bus::setup` → the seven tests fail (`todo!()` or assertion).
- [ ] **Step 3:** Implement. Every Setup read in `declare_all` and `init_one` goes through one private reader that, on `Err(Error::Status(s))` with `hardware_only(&s)`, keeps the data the reply carried (as `diagnose::read_tolerant` does) and notes the fault; anything else is returned as today (still through `diagnose_unknown`/`diagnosed`). `slots.push(kind, bound, if faulted { Faulted } else { Off })`.
- [ ] **Step 4:** `cargo test --workspace --all-features` → PASS, including every pre-existing Setup test unchanged.
- [ ] **Step 5:** Commit `feat(bus): T093 declared Setup holds a hardware-faulted actuator`.

### Task T094: Autoscan holds a hardware-faulted actuator

**Files:**
- Modify: `src/bus/setup.rs` (`autoscan_all`, `collect`)
- Test: `src/bus/setup.rs` tests

**Interfaces:**
- Consumes: `hardware_only`, `init_one(..., fault)`, `Slots::push(.., state)` (T093).

- [ ] **Step 1: Write the failing tests:**

```rust
// pong_with_alert_is_held: XM430 at ID 3, 0x80 on the broadcast-ping reply and every Setup read
let report = bus.autoscan().unwrap();
assert_eq!(report.faulted, ids(&[3]));
assert_eq!(sent(&bus), concat(&[broadcast_ping(), xm430_p2_reads_sent(3)]));

// p1_scan_voltage_fault_is_held_and_measured_by_clear_fault: AX-12A at ID 1, 0x01 on its
// ping and Setup reads, then on the clear_fault ping and the measurement reads (42, 12, 13 → 97, 105, 130)
assert_eq!(bus.autoscan().unwrap().faulted, ids(&[1]));
let err = bus.actuator(id(1)).unwrap().clear_fault().unwrap_err();
assert!(matches!(err, Error::Fault(FaultError::Voltage(VoltageError::BelowMin {
    measured: Decivolts(97), min: Decivolts(105), max: Decivolts(130), ..
}))));
assert_eq!(err.to_string(), "id 1: voltage 9.7 V below 10.5 V");
assert_eq!(bus.torque_states(), [TorqueState::Faulted]);

// p1_scan_instruction_and_voltage_flags_fail: AX-12A ping reply 0x41 during the sweep
assert!(bus.autoscan().is_err());
assert!(bus.actuators().is_empty());
```

- [ ] **Step 2:** `cargo test --features mock,ax,x-series --lib bus::setup` → the three fail.
- [ ] **Step 3:** Implement. In `collect`, a pong whose fault is hardware-only is pushed and its `Id` noted, not returned as an error; `autoscan_all` passes `fault: true` to `init_one` for those. Duplicate-`Id` and capacity checks apply to held-faulted pongs too.
- [ ] **Step 4:** `cargo test --workspace --all-features` → PASS.
- [ ] **Step 5:** Commit `feat(bus): T094 autoscan holds a hardware-faulted actuator`.

### Task T095: Recovery path, DynBus parity, docs

**Files:**
- Test: `src/bus/raw.rs` tests (`raw-registers`), `src/bus/dyn_bus.rs` tests
- Modify: rustdoc of `declare` and `autoscan` in `src/bus.rs` and `src/bus/dyn_bus.rs` (`# Errors`: "a hardware fault fails Setup" → what is held and what still fails, per D3); `docs/deferred.md` D-01 → done with the date

- [ ] **Step 1: Write the failing tests:**

```rust
// faulted_at_setup_is_fixed_with_write_raw (ax + raw-registers): AX-12A held Faulted by
// autoscan (Input Voltage), write_raw of Min Voltage Limit, clean ping, then torque
let before = mark(&bus);
let _ = bus.actuator(id(1)).unwrap().write_raw(12, &[95]); // reply may still carry 0x01
assert_eq!(sent_since(&bus, before), p1_write(1, 12, &[95]));
let mut ax = bus.actuator(id(1)).unwrap();
ax.clear_fault().unwrap();                  // clean ping reply
assert_eq!(ax.torque_state(), TorqueState::Off);
ax.enable_torque().unwrap();

// faulted_at_setup_refuses_enable_torque: two XM430, ID 2 held Faulted
let before = mark(&bus);
assert!(matches!(
    bus.enable_torque(bus.ids()),
    Err(Error::Validation(ValidationError::ActuatorFaulted { .. }))
));
assert_quiet(&bus, before);

// dyn_bus_holds_a_faulted_actuator_like_bus: same script as fault_at_setup_holds_faulted
assert_eq!(dyn_bus.declare([(id(1), model(XM430))]).unwrap().faulted, ids(&[1]));
assert_eq!(dyn_bus.torque_states(), [TorqueState::Faulted]);
```

- [ ] **Step 2:** Run them → fail (`todo!()`).
- [ ] **Step 3:** No production code is expected; if one fails, the fix belongs to T093/T094's code. Update the rustdoc.
- [ ] **Step 4:** Run every command in `AGENTS.md` § Commands → all green (coverage ≥ 70 %).
- [ ] **Step 5:** Commit `feat(bus): T095 recovery of a faulted actuator after Setup`, then step end (ROADMAP row, plan ticks, `docs/learnings.md`).

Out of scope, for the maintainer: kmr's row B2 (kmr's `ping` passes `SetupReport::faulted` on to `health`, where `Ids::voltage_limits` runs). Typed EEPROM writes stay § 8.
