# dxl\_rs\_v2 SRS

Oct 26, 2026 · @David

## 1. Purpose

1.1 dxl\_rs\_v2 is a Rust hardware abstraction layer (HAL) for Dynamixel servo actuators speaking Protocol 1 or Protocol 2 over a half-duplex serial bus.

1.2 It is a library crate. It has no robot model, no kinematics, no async runtime and no scheduler. The robot application owns all of that.

1.3 Goal: give Rust developers safe, typed, low-overhead control of Dynamixel servos, on a desktop (`std`) or a microcontroller (`no_std`, no heap), from one code base.

1.4 User classes:

| Class | Who | What they need from the crate |
| --- | --- | --- |
| Robot application developer | Writes the master app on a PC or SBC with an OS serial port. | `std` build, autoscan, typed single- and multi-actuator ops, precise errors (FR-6.6). |
| Embedded firmware developer | Runs the same control code on a microcontroller UART. | `no_std` + heap-free build, `embedded-io` transport, static capacity `N`, no hidden allocation. |
| Crate maintainer | Adds or fixes actuator models. | RON control tables, `cargo xtask` codegen, readable committed `.rs` output. |

1.5 Readers of this spec: the architect (owner of every decision), and AI coding agents that build from it. Items are tagged **Decided** or **\[decision\]** (proposed, awaiting the architect). Convention for agents is stated in 5.0.

## 2. Scope

2.1 In scope (this version):

- Protocol 1 and Protocol 2 packet encode/decode, checksum (P1) and CRC (P2), status packet parsing.
- A `Bus` that owns up to `N` actuators of mixed models on one wire, one protocol per bus, fixed at compile time.
- Discovery by autoscan (default) or by declared `(Id, model)` list. Autoscan works with and without std (it allocates nothing).
- Single-actuator access through a short-lived handle (\`ActuatorRef\`): it borrows \`&mut Bus\`, so it cannot be stored, cloned or outlive the bus borrow; that is what lets \`Bus\` own the actuators with no \`Rc\`, \`RefCell\` or stored lifetimes, and guarantees one active user of the wire at a time. Multi-actuator reads/writes go through the bus.
- Generated model types with control-table registers as associated consts, from `control_tables/*.ron`.
- EEPROM limits and a limit mode (from the operating mode) cached per actuator; one combined torque/fault state byte per actuator slot, held by the bus.
- Typed values: `Id`, `Position` and other SI newtypes; raw register values always reachable.
- Typed errors for every fallible operation, defined with the \`thiserror\` crate (no\_std), split into transport / protocol / status packet / validation / setup.
- `std` (default: `alloc` + `serialport`) and `no_std` heap-free builds from one crate.
- `cargo xtask` codegen producing committed `.rs` files.
- Opt-in features `partial-reads`, `raw-registers` and `mock`; model-family features (all on by default).

2.2 Out of scope:

- Robot model, joints, kinematics, trajectories. The crate knows actuators, not joints.
- Async I/O or any executor integration.
- Runtime protocol switching or mixed P1/P2 on one bus.
- Baud-rate scanning; the bus scans at its configured baud only.
- Status Return Level tracking (factory default level 2 is assumed).
- Implicit write confirmation or read-back after writes.
- Firmware update, ID assignment tooling, GUI.
- Build-time codegen (`build.rs`, proc macros).
- Heap allocation in the `no_std` build.

## 3. Definitions

3.1 Code names are canonical. A term in this table is used with exactly this meaning everywhere in the spec; retired names are listed so agents do not reintroduce them.

3.2 Moments (when something happens):

| Term | Meaning |
| --- | --- |
| Codegen time | The maintainer runs `cargo xtask`; RON tables become committed `.rs` files. Happens before any build. Never at build time. |
| Compile time | `cargo build` of the user's crate. Protocol `P`, transport `T`, capacity `N` and enabled model families are fixed here. |
| Init | The user constructs a `Bus` and runs Setup (autoscan or declared). Actuator list, EEPROM limits and limit modes are established; every torque state starts off. One-off, boot cost. |
| Cycle | Any read/write after Init, typically a control loop iteration. No allocation, no discovery, no limit re-read. |

3.3 Types and roles:

| Term | Meaning |  |
| --- | --- | --- |
| `Bus<P, T, C, N>` | Owns the actuators on one wire as `heapless::Vec<ActuatorKind, N>`, the transport, the clock, timeouts and retries. One protocol per bus. Not shared, not `Rc`. |  |
| `Id` | Newtype over the firmware ID, `0..=252`. 253 (P1-only) and 254 (broadcast) are never a valid `Id`. The array index inside `Bus` is not an `Id` and is never exposed. |  |
| Actuator | The physical Dynamixel unit on the wire. Not a Rust type; when a type is meant the spec names it. |  |
| `ActuatorKind` | Closed, generated enum with one variant per model type. The storage type inside `Bus`. Recovers the concrete model with `match`. |  |
| `Actuator` (trait) | What every model shares, described without I/O: `Id`, model number, name, position scale and the limits read at Init. Sealed. Torque, goal and position I/O live on `ActuatorRef`. |  |
| Model type | A generated struct per control table, e.g. `Xm430W350`, `Mx64`. Registers are associated `const`s. Holds the per-actuator cache. Tier 2 of the API. |  |
| Table variant | A model whose control table differs by firmware version gets one model type per table (e.g. XL330 before/after fw 53). `dynamixel.ron` maps model number + `from_fw` to the table. |  |
| `ActuatorRef` | Short-lived handle from `bus.actuator(id)`, borrowing `&mut Bus`. Tier 1 of the API. Retired names: handle, actuator handle. **\[decision\]** name proposed from the C4 model. |  |
| Cache | Per actuator, inside its model type: `Id` and limits read from EEPROM at Init. The torque/fault state is not in the model type (no padding): the bus holds it in a one-byte-per-slot array beside the actuator list. |  |
| Torque state | One-byte runtime enum per actuator: off, on, or faulted (faulted always means off). Off at Init. Not a typestate. |  |
| Fault state | The faulted value of the torque state, set when a status packet reports a hardware fault; it means torque off. No fault report is stored: the call that saw it returns it, and Hardware Error Status can be read again. |  |
| Limits | Position (and other) min/max read from the actuator's EEPROM at Init. Never the control-table defaults. |  |
| Protocol (`P`) | `P1` or `P2`, a compile-time type parameter of `Bus`. Packet layout, checksum/CRC, instruction set. |  |
| Transport (`T`) | Any `embedded_io::{Read, Write, ReadReady}`: blocking read and write, plus a non-blocking check for pending input. Default under `std`: `SerialTransport` over `serialport`. Mockable in tests with `MockTransport` (feature `mock`). |  |
| `Clock` | Monotonic microsecond time source. The bus polls `ReadReady` against it to bound every wait for a reply. Any `FnMut() -> u64` closure is a `Clock`; `StdClock` under `std`. |  |
| Control table | A model's register map, authored as `control_tables/<model>.ron`. Data, not a layer. |  |
| Register | One row of a control table: address, size, access, range, unit. Generated as an associated `const` on the model type. |  |
| Model family | A Cargo feature grouping model types (`ax`, `mx`, `x-series`, …). All on by default. |  |
| Setup | The Init step that fills the actuator list: Autoscan or Declared setup. |  |
| Autoscan | Setup by pinging the bus (P2 broadcast ping, P1 per-ID ping) and instantiating what answers. Default. |  |
| Declared setup | Setup from a user-supplied list of `(Id, model)`; a missing actuator or a model mismatch is an Init error. No ID sweep. |  |
| Easy tier | Multi-actuator methods on every `Bus<P, T>` (`read_positions`, `write_goal_positions`, …) whose wire strategy is chosen by `P` at compile time. |  |
| Explicit tier | Multi-actuator methods that name the instruction: `sync_write` (P1 + P2), `sync_read` (P2 only). Absent on an unsupporting protocol. |  |
| Sync write | One broadcast instruction writing the same `(addr, size)` on many IDs. No status reply. |  |
| Sync read | One P2 instruction reading the same `(addr, size)` from many IDs, one status reply per ID. |  |
| Group | Internal split of a multi-actuator op by `(addr, size)`. One sync instruction per group. |  |
| Strict read | Multi-actuator read that fails the whole call on any failure. The default form. |  |
| Partial read | Multi-actuator read returning a per-slot result. Behind the `partial-reads` feature. |  |
| Raw register access | Read/write any register by address, bypassing checks and cache. Behind the `raw-registers` feature. |  |
| Status packet | The reply an actuator sends to a unicast instruction. Carries an error byte that fault detection inspects. |  |
| Unicast / Broadcast | Instruction to one `Id` (waits for its status packet) / to ID 254 (never waits). |  |
| `xtask` | The `cargo xtask` binary that runs Codegen time. Separate workspace member. |  |

## 4. Overall description

4.1 Product perspective. The crate sits between the robot application and the serial adapter:

```
Robot application  (owns the robot model)
        │  Rust API
   Bus<P, T, C, N>  ── owns ──▶ heapless::Vec<ActuatorKind, N>
        │  P: packet encode/decode           Model types (generated from control_tables/*.ron)
        │  T: embedded_io::{Read, Write, ReadReady}
Serial adapter / UART
        │  half-duplex TTL / RS-485
Dynamixel actuators
```

Side channel: the maintainer edits `control_tables/*.ron` and `dynamixel.ron`, runs `cargo xtask`, and commits the generated model types and `ActuatorKind`.

4.2 User classes are in 1.4. The three API surfaces they use:

1. Tier 1, generic: `ActuatorRef` and `Actuator` trait, works on any model.
2. Tier 2, concrete: `match` on `ActuatorKind` to reach a model type and its model-specific methods.
3. Bus-level: Setup, Easy tier and Explicit tier multi-actuator ops.

4.3 Operating environment:

| Target | Transport | Storage | Build |
| --- | --- | --- | --- |
| Desktop / SBC (Linux, macOS, Windows) | `serialport` through the `std` adapter | `heapless::Vec` by default; `alloc::Vec` available under `std` | `default-features` on |
| Microcontroller | User-supplied `embedded_io` UART (`Read + Write + ReadReady`) | `heapless::Vec<_, N>` only | `default-features = false` |

4.4 Constraints:

- `no_std` and heap-free are requirements, not options. The `no_std` build is checked in CI.
- Every PR carries at least 70 % line coverage.
- **Hard requirement (Decided 2026-10-26):** every control table entry and every claimed actuator capability (supported instructions, register addresses, sizes, ranges, units, firmware-dependent variants, protocol support) is backed by the official ROBOTIS Dynamixel e-Manual (`emanual.robotis.com`), looked up online at the time it is written or changed. Memory, third-party SDKs and existing tables are not a source. See FR-7.
- Blocking I/O only. Timeouts are enforced by the bus, not by the transport: the bus polls the transport's non-blocking `ReadReady` against its `Clock`, and reads only bytes already waiting. Plain `Read + Write` is not enough, since `read` blocks without bound.
- No `build.rs`, no proc macro. Generated code is committed.
- No `Rc`, `RefCell` or stored lifetimes inside `Bus`.
- Breaking API changes are allowed until 1.0 (alpha).

4.5 Assumptions:

- Actuators ship with factory Status Return Level 2 (reply to every unicast instruction). The crate does not track or change it.
- One wire carries one protocol and one baud rate.
- The user keeps IDs unique on a wire; duplicate IDs are not detected beyond what the protocol exposes.
- The RON control tables are the source of truth for register addresses, sizes and defaults.

4.6 Dependencies (declared, not pinned here): `embedded-io`, `heapless`, `thiserror` (no\_std), `serialport` (std feature), `serde` + `ron` (xtask only).

## 5. Functional requirements

5.0 Convention for agents. Items marked **Decided** come from ARCHITECTURE.md (2026-10-12) or the architect in this spec; build them as written. Items marked **\[decision\]** are proposals: build the stated default unless overruled. An agent that hits a point the spec leaves open writes `unimplemented!("SRS <id>: <question>")` at that exact path, continues, and reports it. This marker is the one allowed exception to NFR-3 (no panics on the happy path). A wrong guess that would corrupt actuator state or EEPROM is severe: use `compile_error!` instead and stop.

### FR-1 Bus and storage

- FR-1.1 `Bus<P: Protocol, T: Transport, C = DefaultClock, const N: usize = 32>` (`Transport` = `Read + Write + ReadReady`; every `impl` requires `C: Clock`, the struct does not, because rustc checks a default against the bound; `DefaultClock` is the system clock under std; without std the clock has no default and is named, e.g. `Bus<P2, Uart, Mono>`) owns its actuators in `heapless::Vec<ActuatorKind, N>` and the clock that bounds every wait. The clock is passed to the constructor, per bus. Decided; clock parameter added 2026-10-27 (zero bytes for a stateless clock, no indirect call, accepts an owned hardware timer), moved before `N` 2026-10-28.
- FR-1.2 `N` is an upper bound, not an exact count. Setup finding more than `N` actuators is an Init error. Decided.
- FR-1.3 Under the `std` feature, an `alloc::Vec` storage with no cap is also available; `heapless` stays the default. Decided. **\[decision\]** exposed as a separate type `DynBus<P, T, C = StdClock>` with the same methods as `Bus` (an alias of `Bus` cannot swap its storage), not as a feature-switched change to `Bus` (features add API, never change it).
- FR-1.4 `Bus` holds timeouts and retry count, settable at construction, with defaults. Decided. **\[decision\]** defaults: 10 ms status timeout, 10 ms scan wait (P2 broadcast-ping window, P1 per-ID wait), 0 retries; retries apply only to unicast instructions that time out or get a corrupted reply; after the last attempt of a unicast instruction got no byte, the bus listens one more timeout to tell a late reply from none (FR-6.8), so a silent unicast instruction takes at most `(retries + 2) × timeout`; measured values replace these in NFR-5. Late-reply listen added 2026-11-04 (audit E2).
- FR-1.5 `Bus` exposes no `Rc`, `RefCell` or stored lifetime. Access to one actuator goes through `bus.actuator(id) -> Option<ActuatorRef<'_, P, T, C>>`. Decided.
- FR-1.6 One `Bus` means one wire, one protocol, one baud. Decided.

### FR-2 Setup (Init)

- FR-2.1 Autoscan is the default Setup. P2 sends one broadcast ping and collects replies; P1 pings each ID 0..=252 in turn. Decided.
- FR-2.2 Autoscan uses only the bus's configured baud. Decided.
- FR-2.3 Autoscan and declared setup instantiate one `ActuatorKind` variant per actuator, chosen by the bus protocol and the model number through that protocol's table list in `dynamixel.ron` (`p1` or `p2`); when that list has table variants, Setup reads Firmware Version and picks the table whose `from_fw` is the highest one not above it. Decided; per-protocol lists 2026-10-27.
- FR-2.4 A model number with no compiled-in model type on the bus protocol fails Setup with `SetupError::UnsupportedModel { id, model_number, protocol, feature }`; `feature` names the Cargo feature that would enable it when its family is off, and is `None` otherwise. Decided; under `Setup` 2026-10-27, `id` and `protocol` added 2026-10-28.
- FR-2.5 Declared setup takes `impl IntoIterator<Item = (Id, ModelNumber)>`, pings only those IDs, and fails Init if one is missing or answers with another model. No ID sweep. Decided. **\[decision\]** the declared model is a `ModelNumber` newtype, not the model type, so the list stays data.
- FR-2.6 At Init, Setup reads every instantiated actuator's limits from EEPROM into its cache, then its operating mode: one 1-byte read of Operating Mode (XL-320: Control Mode), or, for AX and MX on P1, derived from the CW/CCW Angle Limits just read. The meaning of each mode value comes from the model's control table. It never falls back to control-table defaults or to a default mode; a failed read fails Init, and a mode value the table does not list fails Init with `UnknownOperatingMode`. Decided; operating mode added 2026-10-28.
- FR-2.7 At Init, every actuator's torque state (one byte per slot, held by the bus) starts off, whatever the hardware reports; an actuator that reports a hardware fault during Setup fails Init with that error. Decided (torque off by default). **\[decision\]** the hardware's actual Torque Enable is not read at Init; a user who needs it reads it explicitly.

### FR-3 Single-actuator API (Tier 1 and Tier 2)

- FR-3.1 `ActuatorRef` is the only single-actuator entry point. It borrows `&mut Bus`, so two refs cannot coexist. Decided.
- FR-3.2 The `Actuator` trait describes what every model shares (identity, position scale, limits), with no I/O; the operations live on `ActuatorRef`. `ActuatorKind` implements it by `match` (static dispatch, no `dyn`). Decided; split 2026-10-27.
- FR-3.3 Enabling torque is always an explicit call (`enable_torque()`); nothing enables it implicitly. Decided.
- FR-3.4 A goal write while torque state is off returns `Error::ActuatorIsOff` and sends nothing. No model-specific side effect (no AX-style auto-enable). Decided.
- FR-3.5 Model-specific operations live on the model type (Tier 2), reached by `match` on `ActuatorKind`. Decided.
- FR-3.6 Values cross the API as typed newtypes (`Position`, …) with units and ranges; a raw-register-value form of each is also offered. Decided.
- FR-3.7 A goal write follows the operating mode recorded at Init (FR-2.6), as the firmware does. In position (joint) mode, a goal outside the cached limits returns a validation error and sends nothing. In extended position, current-based position and MX multi-turn mode the firmware does not use the limits, so only the register size is checked. In velocity, current, PWM and wheel mode the firmware ignores Goal Position, so every goal returns `NotPositionMode` and sends nothing. The limits getters return the EEPROM limits in every mode. Decided 2026-10-28 (limits follow the operating mode).
- FR-3.8 Writes are never confirmed implicitly. Read-back is a separate call the user makes. Decided.

### FR-4 Multi-actuator API

- FR-4.1 Multi-actuator targets are `impl IntoIterator<Item = Id>` (ranges and sparse lists alike). Decided.
- FR-4.2 Easy tier methods exist on every `Bus<P, T>`. Goal writes use sync write on P1 and P2; torque enable/disable on many actuators uses acknowledged unicast writes, so each torque state matches what its actuator confirmed. Reads use sync read on P2 and sequential unicast reads on P1. The choice is made by `P` at compile time. Decided.
- FR-4.3 Explicit tier: `sync_write` is implemented for `P1` and `P2`; `sync_read` is implemented for `Bus<P2, T>` only. An unsupported combination is a missing method, hence a compile error. Decided.
- FR-4.4 Multi-actuator ops group targets internally by `(addr, size)` and send one instruction per group. Decided.
- FR-4.5 Multi-actuator reads fill a caller-owned `&mut` buffer and allocate nothing. Decided. **\[decision\]** buffer type is `&mut [Position]` (or the op's value type, `i32` for raw), indexed in target order; a strict read leaves it unchanged on failure; length shorter than the target count is a validation error.
- FR-4.6 Strict read (`read_*`) fails the whole call on any failure. Decided.
- FR-4.7 Partial read (`read_*_partial`) returns a per-slot `Result` and exists only behind `partial-reads`. Each actuator gets its own result, inside a P2 sync group too; only a failed instruction (the sync read itself fails, e.g. a transport error) fails every slot of its group. Decided; per-actuator results 2026-10-28.
- FR-4.8 Easy tier writes are validated as a whole before any byte: a target with torque off returns `ActuatorIsOff` for that `Id`, and any other validation failure (unknown or repeated `Id`, value count, limits, operating mode) also sends nothing to anyone. A transport failure between two groups leaves the earlier group sent (documented). **\[decision\]** all-or-nothing, matching FR-3.4; the alternative (skip the off ones) hides mistakes.
- FR-4.9 A feature flag adds API; it never changes an existing method's behaviour. Decided (not binding until 1.0).

### FR-5 Protocol

- FR-5.1 `P1` and `P2` implement a `Protocol` trait: build instruction packets, parse status packets, compute checksum (P1) / CRC-16 (P2), per the e-Manual packet specification. Decided.
- FR-5.2 RX is cleared before every TX. Decided.
- FR-5.3 Unicast instructions wait for their status packet (bounded by the bus timeout). Broadcast instructions never wait. Decided.
- FR-5.4 Status Return Level is not read, tracked or changed. Decided.
- FR-5.5 The `Protocol` trait is sealed; users cannot add protocols. **\[decision\]** keeps `Bus` impls exhaustive.

### FR-6 Fault detection and errors

- FR-6.1 Every received status packet's error byte is checked: P2 alert bit 7 and error code; P1 per-bit error flags. Decided.
- FR-6.2 A reported hardware fault in any status packet, `raw-registers` replies included, sets the actuator's torque state to faulted (which means off) in the bus's state byte, and returns a typed hardware error from the call that received it. Decided; raw replies included 2026-10-28.
- FR-6.3 Sync writes get no reply, so a write-only loop sees faults only on its next read. Documented limitation, not fixed. Decided.
- FR-6.4 Errors are one `thiserror` enum (no\_std) split into `Transport`, `Protocol`, `Status` (the status packet's error byte), `Fault` (a hardware fault measured against its limits, FR-6.13), `Validation` (`ActuatorIsOff`, `ActuatorFaulted`, out-of-range, not in a position mode, bad buffer, repeated `Id`) and `Setup` (`UnsupportedModel`, too many actuators, declared actuator missing or of another model, `Id` not on the bus, unknown operating mode). Decided; `Setup` added 2026-10-27, mode errors 2026-10-28, `Fault` 2026-11-04.
- FR-6.5 Clearing a fault state is an explicit user call. **\[decision\]** `clear_fault()` asks the actuator: it re-reads Hardware Error Status, or pings on models without it (AX, MX on P1, whose status error flags carry the fault), and clears only if the hardware reports none. Enabling torque while faulted is refused with a typed error and sends nothing.
- FR-6.6 Every failure returns an explicit error. One variant per distinct cause, where a cause is distinct when a caller would check or fix something different. No catch-all variant (`Malformed`, `Other`, "communication error") merges causes. Decided 2026-11-03.
- FR-6.7 An error carries, as fields, everything a caller needs to act without a second run to probe: the actuator `Id` (absent only when no single actuator is addressed, e.g. a broadcast, whose instruction is still named); what was being done, as one `Request { instruction, id, register, addr, len, step }` carried by every transport, protocol, status, fault and request-shaped validation error (the instruction, the register by table name and address, the data length, and the step: during Init the Setup step, i.e. scan, ping, model number, firmware, position limits, operating mode or goal bound, and fault diagnosis while FR-6.13 measures a fault); and the values that decided the failure (expected and received, value and allowed range, timeout, bytes received before the failure, value measured and limit crossed). A value the hardware reports outside the known set is kept raw, never dropped. One exception: a transport I/O error keeps only its `embedded_io::ErrorKind` and its direction (write, flush, read-ready, read), because `embedded_io` does not expose the OS error code; end of stream is its own variant. Decided 2026-11-03; `Request` and the I/O exception 2026-11-04.
- FR-6.8 An error states what was observed, never a guessed cause: a reply that did not arrive before the deadline is reported as such, not as "actuator missing", unless the crate has observed that the actuator is absent. Decided 2026-11-03.
- FR-6.9 The `Display` text of every error is one short line, `id N: what happened`, with the values that decided it and, where it tells two calls apart, the short request (e.g. `(Write Goal Position)`), so a failing control loop does not flood the terminal; the line alone identifies the cause, without the source code. Every field of FR-6.7 stays in the error and `{:#?}` prints it. `{:?}`, which `?` out of `main` prints, prints the `Display` line, and bit sets name their flags, never raw bits. Decided 2026-11-03; short line, full context in `{:#?}`, 2026-11-04.
- FR-6.10 Error context is fixed-size and `Copy`: `Id`s, integers, durations and `&'static str` names. No allocation, no formatted strings stored in the error, in every build (NFR-1, NFR-2). Decided 2026-11-03.
- FR-6.11 The cases where the errors did not meet FR-6.6 to FR-6.10 (`specs/ideas/error-precision-audit.md`, E1 to E13) were each decided by the architect and built on 2026-11-04; the audit's Decisions table records how. New errors follow FR-6.6 to FR-6.13 from the start. An agent never merges errors further; a failure that cannot meet these rules, or one where two different splits would both meet them, is reported to the architect, not picked. Decided 2026-11-03, audit closed 2026-11-04.
- FR-6.12 Advice lives apart from the error: `Error::hint() -> Option<&'static str>` says what to check when the observed facts point at a usual cause, for example power, wiring, baud and ID for silence, the USB adapter's latency timer or the bus timeout for a late reply, wiring and noise for a corrupted instruction, supply and wiring for low voltage. `Display` carries no advice, and a hint never replaces or changes the observed facts of FR-6.8. The crate never changes an OS setting itself. Decided 2026-11-04 (audit E12).
- FR-6.13 A hardware fault is measured on the fault path. When a status packet reports one (FR-6.2), the call that received it reads from that actuator the quantity behind the fault and its EEPROM limits (Present Voltage with Min and Max Voltage Limit, Present Temperature with Temperature Limit; on P2, Hardware Error Status first), and returns `Error::Fault`, which says what was measured, against which limit and in which direction (e.g. voltage below its minimum, above its maximum, or inside its limits; temperature above its limit). These reads happen only after a reported fault, never otherwise, and carry the step `FaultDiagnosis`. A model whose table lacks the register gets `Unmeasurable` naming it; if a measurement read fails, the call returns the unmeasured `StatusError`. The actuator is faulted either way. Sync writes get no reply, so nothing is measured (FR-6.3). This is the only read the crate makes that the caller did not ask for (an exception to FR-9.6 and to 6.0 "nothing hidden"). Decided 2026-11-04 (audit E13).
- FR-6.14 `cargo xtask` errors (codegen and the table check) follow FR-6.6 to FR-6.9 and FR-6.11: one diagnostic per cause, observed facts only, and as context the file, model, register and field involved and the values that decided the failure, in place of the `Id` and `Request` of FR-6.7. FR-6.10 is waived: xtask is std only and may hold `String`s. FR-6.12 and FR-6.13 concern the crate at run time. Decided 2026-11-09.

### FR-7 Control tables, capabilities and codegen

- FR-7.1 Every control table lives in `control_tables/<model>.ron`; `dynamixel.ron` maps model number → one table list per protocol (`p1`, `p2`), each ordered by `from_fw`, with `default: P2` when both exist, and records in `not_mapped`, with a reason, every table it leaves out. Decided; per-protocol lists 2026-10-27.
- FR-7.2 **Hard requirement.** Every register (address, size, access, range, unit, default) and every capability claim (instructions supported, protocol, firmware-dependent layout, model number) in any `.ron` file comes from the official ROBOTIS e-Manual page for that model. No entry is written from memory, from a third-party SDK or by copying another model's table. Verification against the online page is deferred: a model is verified when it is first needed, one model at a time, and its `.ron` file then records the e-Manual URL and the date. A table not verified yet stays usable and is flagged explicitly as unverified (in the `.ron` metadata and in the generated docs). Priority: AX-12, MX-64. Decided 2026-10-26, amended 2026-10-27.
- FR-7.3 `cargo xtask` parses the RON (serde + ron) and writes plain `.rs` files: one model type per table, and `ActuatorKind`. Output is committed; CI fails if regeneration produces a diff. Decided.
- FR-7.4 A table with no model number in `dynamixel.ron` is skipped: silently when `not_mapped` lists it (`NoModelNumber` or `SameNumberAs(n)`), with a codegen warning otherwise, which the table check treats as an error. Decided.
- FR-7.6 Every control table gives exactly one operating-mode source, from the e-Manual: its `Operating Mode` register (XL-320: `Control Mode`) with an `operating_modes` field listing which values are position, extended-position-like and non-position modes, or, on models with neither (AX, MX on P1), the CW/CCW Angle Limit pair. Decided 2026-10-28.
- FR-7.5 Model types are grouped behind family features (`ax`, `mx`, `x-series`, …), all on by default. Decided. **\[decision\]** the exact family list is `ax`, `mx`, `x-series`, `y-series`, `pro` (from the brief); ARCHITECTURE.md lists only the first three.

### FR-8 Raw register access

- FR-8.1 Behind `raw-registers` (off by default): read/write any address and size on an actuator. What it sends bypasses limits, torque guard and cache; its replies are checked like every status packet, so a hardware fault sets the faulted state (FR-6.2). Decided; fault check on raw replies 2026-10-28.
- FR-8.2 Raw access is documented as an escape hatch that can desync the cache: a raw write to Operating Mode leaves the recorded mode stale until the next Setup, and a raw write to Torque Enable leaves the torque state as it was until the next typed torque call; the crate does not try to repair the cache after a raw write. Decided.

### FR-9 Typed registers beyond position

- FR-9.1 The registers below get typed access on `ActuatorRef`. Each value is an SI newtype, and
  a raw-register form is also offered (FR-3.6). The model's table supplies the address, size
  and scale. A model whose table lacks the register has no method for it at compile time
  where possible (Tier 2); at Tier 1, the call returns `Error::Unsupported { id, register }`
  and sends nothing. **\[decision\]** decided 2026-11-02;

  | Register (table names) | Newtype | SI unit | Access | Families |
  |---|---|---|---|---|
  | Present Temperature | `Temperature` | °C (`i16`/`f32`) | read | all |
  | Present Voltage / Present Input Voltage | `Voltage` | V | read | all |
  | Present Speed (P1) / Present Velocity (P2) | `Velocity` | rad/s | read | all |
  | Moving Speed (P1) / Profile Velocity (P2) | `Velocity` | rad/s | read/write | all |
  | Goal Velocity (P2, velocity mode) / Moving Speed in wheel mode (P1) | `Velocity` | rad/s | read/write | see FR-9.4 |
  | Present Load (P1, % of max) | `Load` | ratio −1..1 | read | AX, MX-P1, RX |
  | Present Current (P2) | `Current` | A | read | MX-P2, X, Y, PRO |
  | Goal Current | `Current` | A | read/write | current-capable models |
  | Moving | `bool` | — | read | all |
  | LED | `bool` (or `Rgb` on PRO/Y) | — | read/write | all |
  | Hardware Error Status (P2) | `HardwareErrors` (bit set) | — | read | MX-P2, X, Y, PRO |
  | Realtime Tick (P2) | `Tick` (ms, wrapping) | s | read | models whose table has it |
  | Position P / D gain | `Gain` (raw register units, per-model) | — | read/write, Tier 2 | models whose table has P and D gains |
  | CW/CCW Compliance Margin and Slope (AX, RX) | `ComplianceMargin`, `ComplianceSlope` | margin as `Position` | read/write, Tier 2 | AX, RX |

- FR-9.2 Scales come from `unit_info` in each `.ron` (e.g. MX-64 P2 velocity 0.229 rpm/LSB,
  current 3.36 mA/LSB). Codegen emits them as per-model consts. A register without `unit_info`
  has no typed form, only raw. **\[decision\]** decided 2026-11-02; no hand-written fallback scales (consistent with "never fall back to table defaults").
- FR-9.3 Writes check the register size and the EEPROM limit where one exists (Velocity Limit,
  Current Limit, Max Torque). Out of range returns a validation error and sends nothing, as in FR-3.7.
  Writes to RAM registers that do not move the actuator (LED, Profile Velocity, gains) are allowed with torque off. **\[decision\]** decided 2026-11-02;
- FR-9.4 A goal velocity write requires velocity mode (P2) or wheel mode (P1, CW = CCW limit = 0),
  recorded at Init (FR-2.6); otherwise `NotVelocityMode`. A goal current write requires a current or
  current-based position mode. This mirrors FR-3.7 for position.
- FR-9.5 Multi-actuator: `sync_write` and `sync_read` (P2 only, FR-5) accept any FR-9.1 register
  through the same Easy and Explicit tiers as position, grouped by (addr, size). One register per call;
  several registers in one transaction is Bulk Read/Write (roadmap). **\[decision\]** decided 2026-11-02;
- FR-9.6 Reads are never implicit: no register is polled or cached after Init except the limits
  and mode already cached by FR-2. Every typed read is one unicast or one sync read. The one exception is
  the fault measurement of FR-6.13, after a reported fault only.
- FR-9.7 Newtypes: `Temperature`, `Voltage`, `Velocity`, `Current`, `Load`, in the core module, same
  shape as `Position` (`from_si`, `to_si`, raw conversion through a per-model scale). **\[decision\]** decided 2026-11-02;
  `f32` everywhere (consistent with `Position`; Y-series precision question stays open in §8).

Out of scope for FR-9:

- Several registers in one packet (Bulk Read/Write, indirect addressing).
- Writing EEPROM registers (limits, ID, baud) through typed calls: still `write_raw`, because EEPROM
  writes need torque off and wear the flash. **\[decision\]** Revisit requested 2026-11-03: typed
  EEPROM writes under their own validation, and a way to reach an actuator whose EEPROM causes a
  fault (`specs/ideas/eeprom-writes.md`, open question in section 8).
- Changing the operating mode at runtime (would invalidate the FR-2.6 cache). Possible later FR.

## 6. Non-functional requirements

6.0 Design priorities. Developer experience first, performance second. Velocity of adding features is not a priority: an API that is awkward to use is not shipped faster. Two principles every NFR serves: nothing hidden (no implicit allocation, confirmation, torque change or fallback) and errors at the earliest moment (compile time over Init, Init over Cycle).

- NFR-1 `no_std` + heap-free: the crate builds with `default-features = false` on a `thumbv7em-none-eabihf` target in CI, and no code path reachable without the `std` and `alloc` features allocates. Decided.
- NFR-2 Zero per-Cycle allocation in every build: multi-actuator ops use caller-owned buffers and `heapless` internals. Decided.
- NFR-3 No panic on the happy path or on any I/O or hardware error; such errors are `Result`s. Exceptions: the `unimplemented!` marker of 5.0, and `N` overflow in `Declared setup` being an error, not a panic. Decided.
- NFR-4 Static dispatch everywhere: no `dyn`, no vtables, no runtime lookup from `Id` to register address beyond the `Vec` index. Decided.
- NFR-5 Latency and jitter targets are deferred until measured on a reference bench. **\[decision\]** reference bench: one named Linux host, kernel and `serialport` version recorded, U2D2 adapter, 1 Mbaud, 4 × XM430-W350 on one wire, 1 kHz sync write + sync read loop; figures from other machines are not targets.
- NFR-6 Coverage: every PR reports at least 70 % line coverage. Decided.
- NFR-7 Compile-time cost: model families are features so a user compiles only the tables they need. Decided.
- NFR-8 Readability of generated code: `cargo xtask` output is formatted `.rs` a human can review in a diff; table changes show up as source diffs. Decided.
- NFR-9 Table correctness: FR-7.2 applies to every table and capability claim; a PR that verifies or changes a table carries the e-Manual URL and verification date in the `.ron` metadata, and every other table states that it is unverified. Decided 2026-10-26, amended 2026-10-27.
- NFR-10 Testability: `Transport` is mockable, and the protocol layer is tested against the e-Manual packet examples without hardware. Decided.
- NFR-11 Responsibility split for safety:
  - Ours: torque off by default, limit checks against EEPROM in position mode (FR-3.7), fault detection on every status packet, typed errors.
  - User's: physical limits beyond the actuator's EEPROM, joint-level and robot-level safety, deciding when to enable torque, reacting to a returned fault.
  - Tooling we owe: explicit `enable_torque`/`disable_torque`, fault state readable per actuator, read-back calls for verification, raw access for debugging.
- NFR-12 Error precision is tested: every error variant has a test that builds it with distinct field values and checks that its `Display` line contains the `Id` and each deciding value (FR-6.9); every place that returns an error has a test that checks the variant and the context fields of FR-6.7; every hint of FR-6.12 and every measured outcome of FR-6.13 has a test. Decided 2026-11-03, amended 2026-11-04.

## 7. Interfaces

| Interface | Where | Direction | Contract |
| --- | --- | --- | --- |
| Rust API, bus level | `Bus<P, T, C, N>` | app → crate | `new(transport, clock, config)`, `autoscan()`, `declare(iter)`, `actuator(id)`, Easy tier `read_*`/`write_*`, Explicit tier `sync_write`/`sync_read`. |
| Rust API, actuator level | `ActuatorRef`, `Actuator` trait, model types | app → crate | Typed get/set, `enable_torque`, `disable_torque`, `torque_state()` (off / on / faulted), `clear_fault()`, raw forms. |
| `Protocol` trait | `P1`, `P2` (sealed) | internal | Encode instruction, decode status, checksum/CRC, broadcast ID, ping format. |
| Transport | `T: Transport` = `embedded_io::Read + Write + ReadReady` | crate → serial | Blocking byte I/O plus a non-blocking pending-input check; errors mapped to `Error::Transport`; timeout enforced by `Bus` with its `Clock`. |
| Serial adapter | `serialport` std adapter (default) or user UART | crate → hardware | Half-duplex TTL / RS-485 at the bus's single baud. |
| Control tables | `control_tables/*.ron`, `dynamixel.ron` | maintainer → xtask | Register rows (address, size, name, access, area) + table metadata: `source` (unverified, or e-Manual URL and verification date), protocols, `operating_modes`, position scale; the map: model number, per-protocol `from_fw` lists, default protocol, `not_mapped` (FR-7.1, FR-7.2, FR-7.6). |
| Codegen | `cargo xtask` | xtask → crate source | Writes model types and `ActuatorKind` as committed `.rs`; CI diff check (FR-7.3). |
| Cargo features | `Cargo.toml` | user → crate | `std` (default: `alloc` + `serialport`), family features (default on), `partial-reads` (off), `raw-registers` (off), `mock` (off: `MockTransport` for tests without hardware). |

## 8. Decisions, open questions, delivery order, roadmap

**Decisions** (ticked = decided, with date)

- [x] AoS storage, `ActuatorKind` closed enum, `Bus` owns actuators. ARCHITECTURE.md, 2026-10-12.
- [x] Torque state as runtime field, off by default, explicit enable only. 2026-10-12. One byte per slot (off / on / faulted) held by the bus, not in the model types. 2026-10-27.
- [x] Limits follow the operating mode (mode values from each table); `raw-registers` replies run the fault check; partial reads keep one result per actuator in a P2 sync group; `UnsupportedModel` gains `id` and `protocol`; `P2::broadcast_ping` removed in core-1. 2026-10-28.
- [x] `Bus` takes the clock as a type parameter with a default (third, before `N`, since 2026-10-28); autoscan available without std; setup errors in a `Setup` category; every table records access and memory area per register now, range/default/unit per model on verification. 2026-10-27.
- [x] Limits from EEPROM at Init, never table defaults. 2026-10-12.
- [x] Easy + Explicit tiers; `sync_read` P2 only as a missing method. 2026-10-12.
- [x] `Id` is `0..=252`, no protocol parameter. 2026-10-12.
- [x] Codegen via `cargo xtask`, committed output, no `build.rs`. 2026-10-12.
- [x] Status replies as SDK/v1; no Status Return Level tracking. 2026-10-12.
- [x] Fault detection on every status packet. 2026-10-12.
- [x] Every table and capability backed by the official e-Manual (FR-7.2, NFR-9). 2026-10-26. Online verification deferred to first use per model, unverified tables flagged. 2026-10-27.
- [x] A model that supports both protocols has tables for both; Protocol 2 is the default. 2026-10-27.
- [x] Transport contract is `embedded_io::{Read, Write, ReadReady}`; the bus bounds waits with a `Clock` (3.3, 4.4, §7). 2026-10-26.
- [x] `MockTransport` is public behind the opt-in `mock` feature, byte-level only; a simulated actuator is a later module. 2026-10-26.
- [ ] Handle type is named `ActuatorRef` (3.3). Proposed.
- [ ] `alloc` storage exposed as a separate alias, not a feature-switched `Bus` (FR-1.3). Proposed.
- [ ] Timeout/retry defaults 10 ms / 0 until benched (FR-1.4). Proposed.
- [ ] Declared setup takes `ModelNumber` newtypes (FR-2.5). Proposed.
- [ ] Init does not read hardware Torque Enable (FR-2.7). Proposed.
- [ ] Multi-read buffer is `&mut [Option<T>]` in target order (FR-4.5). Proposed.
- [ ] Easy-tier write with any torque-off target is all-or-nothing (FR-4.8). Proposed.
- [ ] `Protocol` trait sealed (FR-5.5). Proposed.
- [ ] `clear_fault()` re-reads Hardware Error Status (FR-6.5). Proposed.
- [x] FR-9 register list: FR-9.1 table plus Hardware Error Status, Realtime Tick, typed P/D gains and AX/RX compliance (Tier 2). 2026-11-02.
- [x] FR-9 velocity unit is rad/s. 2026-11-02.
- [x] FR-9 P1 Present Load is a ratio newtype (−1..1). 2026-11-02.
- [x] FR-9 missing register: `Unsupported` at Tier 1, method absent at Tier 2. FR-9 delivered split per module (core, xtask, bus). 2026-11-02.
- [ ] Family feature list `ax, mx, x-series, y-series, pro` (FR-7.5). Proposed; ARCHITECTURE.md and the brief differ.
- [ ] Reference bench definition (NFR-5). Proposed.
- [x] Precise errors: one variant per cause, full context as fixed-size fields, observed facts only, a short `Display` line with every field in `{:#?}` (FR-6.6 to FR-6.11, NFR-12). 2026-11-03, amended 2026-11-04.
- [x] `cargo xtask` errors follow the error rules, without FR-6.10's fixed size (FR-6.14). 2026-11-09.
- [x] Error precision cases E1 to E13 decided and built (audit Decisions table): `Request` context with the Setup step on every error (FR-6.7); `Timeout` split into no, partial, late and skipped reply, port timeout and end of stream, with the late-reply listen (FR-1.4); `Malformed` split by cause; `Missing` only for a silent ping; a `Fault` category measured on the fault path (FR-6.13); advice in `Error::hint()` (FR-6.12). 2026-11-04.

**Open questions** (each says what an agent does meanwhile)

- [ ] Does Tier 2 expose model types as public structs, or only through `ActuatorKind` variants? Meanwhile: public, `#[non_exhaustive]` on `ActuatorKind`.
- [ ] Which registers are in the `Actuator` trait beyond torque, goal position, present position, limits? Meanwhile: those four only; `unimplemented!("SRS OQ-2")` elsewhere.
- [ ] Does `Position` carry radians or degrees as its SI form? Meanwhile: radians (`f32`); raw ticks always available.
- [ ] What does a `partial-reads` result look like on P1 (sequential)? Meanwhile: per-slot `Result`, since each read is independent.
- [x] How is the e-Manual verification recorded in RON (field names)? `source: Unverified` or `source: Verified(url: "...", verified: "YYYY-MM-DD")` at table top level. 2026-10-27.
- [ ] Typed EEPROM writes (`specs/ideas/eeprom-writes.md`), added 2026-11-03: which registers (voltage and temperature limits, angle limits, Max Torque, Return Delay Time, Status Return Level, ID, baud); their own validation, apart from FR-3.7 and FR-9.3 (torque off, range from the e-Manual per FR-7.2, min ≤ max, ID free on the bus, baud the bus can reach); the Setup cache after a write (FR-2.6); behind a feature or not; EEPROM wear guarded or only documented; and how an actuator whose EEPROM causes a fault is reached (Setup holds it `Faulted` instead of failing the bus, or a recovery handle). Meanwhile: no typed EEPROM write, `write_raw` only, and Setup keeps failing on a fault (FR-6.2).

**Delivery order** (MoSCoW)

1. Must: `Id`, errors, `Protocol` trait with `P2` encode/decode + CRC, mock transport, packet tests from the e-Manual.
2. Must: `P1` encode/decode + checksum, same test shape.
3. Must: control-table RON format + `dynamixel.ron` + `cargo xtask` codegen, CI diff check; the first verified models are AX-12A and MX-64 (FR-7.2), and an X-series model is verified when first needed.
4. Must: `Bus`, `ActuatorKind`, `Actuator` trait, `ActuatorRef`, torque guard, limits at Init, autoscan + declared setup.
5. Must: fault detection, typed values (`Position`), `no_std` CI job, coverage gate.
6. Must: Easy tier + Explicit tier multi-actuator ops, caller-owned buffers, strict reads.
7. Should: remaining X-series, AX, MX tables (Protocol 1 and Protocol 2 for models that support both, Protocol 2 by default), family features.
8. Should: `serialport` std adapter, example binary.
9. Could: `partial-reads`, `raw-registers`.
10. Could: `alloc` storage alias.
11. Should: FR-9 typed registers, split core (newtypes) → xtask (scales, register roles) → bus (methods, sync ops).

**Roadmap** (not in this version; design must not block it)

- AX-12+, AX-18F, XW430-T200: the e-Manual gives numbers already used (12, 18, 1270, confirmed 2026-10-28; same layouts). They are recorded as `not_mapped` `SameNumberAs`, named in `also` on the existing entry, and skipped without a warning. Deleting the duplicate files is a later cleanup.
- Y-series and PRO families: feature names reserved now (FR-7.5) so adding tables is not a breaking change.
- Bulk read/write (per-ID address and size, P2): grouping by `(addr, size)` is already the internal shape; keep the group abstraction so bulk ops reuse it.

## 9. Modules

One section per deliverable unit. Paths are proposals **\[decision\]** until the architect confirms the crate layout.

## Module: core

Paths: src/lib.rs src/id.rs src/error.rs src/actuator.rs src/register.rs src/units.rs src/units/**

Covers: FR-3.2 (the `Actuator` trait and `Register` type), FR-3.6, FR-3.7, FR-6.4, FR-6.6 to FR-6.12 (error types, hints), NFR-3, NFR-4, NFR-12. Delivery steps 1, 5. No-go files: `src/models.rs`, `src/models/**` unless a task names them.

## Module: protocol

Paths: src/protocol.rs src/protocol/**

Covers: FR-5.1 to FR-5.5, FR-6.1, FR-6.6 to FR-6.11 (errors it returns), NFR-10, NFR-12. Delivery steps 1, 2. No-go files: `src/bus/**`, `src/models.rs`, `src/models/**`.

## Module: transport

Paths: src/transport.rs src/transport/**

Covers: 4.3 adapters, Transport row of section 7, FR-6.6 to FR-6.11 (errors it returns), NFR-10 (mock), NFR-12. Delivery step 8 (std adapter), mock in step 1. No-go files: everything outside `src/transport/`.

## Module: bus

Paths: src/bus.rs src/bus/**

(Bus, ActuatorRef, setup, multi-actuator ops, fault handling)

Covers: FR-1, FR-2, FR-3.1 to FR-3.5, FR-3.8, FR-4, FR-6.2, FR-6.3, FR-6.5, FR-6.6 to FR-6.11 (errors it returns, call and Setup step context), FR-6.13 (fault measurement), FR-8, NFR-1, NFR-2, NFR-12. Delivery steps 4, 5, 6, 9, 10. No-go files: `src/models.rs`, `src/models/**`, `control_tables/**`.

## Module: tables

Paths: control_tables/**

Covers: FR-7.1, FR-7.2, FR-7.4, FR-7.6, NFR-9. Delivery steps 3, 7. Rule: no row without an online e-Manual lookup (FR-7.2). No-go files: `src/**`.

## Module: xtask

Paths: xtask/** src/models.rs src/models/**

Covers: FR-6.14 (xtask errors), FR-7.3, FR-7.4 (skip warning), FR-7.5, NFR-7, NFR-8, NFR-12 (its diagnostics), and the generated side of FR-2.3, FR-2.4, FR-2.6 (mode source per model), FR-3.2. Delivery step 3. No-go files: any hand-written file under `src/` other than the `pub mod models;` declaration in `src/lib.rs`.

## Module: ci

Paths: .github/workflows/** (never ov-*.yml) Cargo.toml (`[package]`, package metadata and `[lints]` only)

Covers: NFR-1, NFR-6, NFR-7 (every feature set builds), 4.3 (`std` build and tests on Linux, macOS and Windows), FR-7.3 diff check, and the table check of FR-7.2/FR-7.6/NFR-9 on every PR. Delivery step 5. No-go files: `src/**`, `control_tables/**`.
