# Feature Specification: Core (typed values, Actuator trait, errors)

**Feature Branch**: `004-core` (not created; this spec was written on `dev`)

**Created**: 2026-10-27

**Status**: Draft, updated 2026-10-28 with the architect's decisions (Clarifications, session
2026-10-28)

**Input**: User description: "core module of @docs/srs/modules/core.md"
→ SRS module `core` (SRS §9): FR-3.2 (the `Actuator` trait and the `Register` type), FR-3.6,
FR-3.7, FR-6.4, NFR-3, NFR-4; delivery steps 1 and 5. Paths: `src/lib.rs`, `src/id.rs`,
`src/error.rs`, `src/actuator.rs`, `src/register.rs`, `src/units.rs`, `src/units/**`. No-go:
`src/models.rs`, `src/models/**` unless a task names them.

## Clarifications

### Session 2026-10-27

- Q: Does the `Actuator` trait carry the I/O operations or only describe the model? → A: It only describes the model (registers, scale, cache); I/O lives on `ActuatorRef` in the bus (single responsibility).
- Q: Where do torque state and fault state live? → A: In one combined 1-byte state enum (torque off, torque on, faulted; faulted always means off), not in the actuator structs, to keep memory low (no padding). Core defines the enum and its rules; the bus stores one byte per actuator slot in its own array beside the actuator list. No fault report is kept per actuator. This replaces the two flags of SRS 3.3.
- Q: Where do Init failures that are not transport, protocol or status errors go? → A: Option B: a new `Setup` category beside `Transport`, `Protocol`, `Status` and `Validation`. It holds more than `N` actuators found, a declared actuator missing, a declared actuator answering with another model, a target `Id` not on the bus, and `UnsupportedModel { model_number, feature }`, which moves from a top-level `Error` variant into `Setup` (breaking change, fine during alpha). (Its fields grew on 2026-10-28, below.)

### Session 2026-10-28 (architect decisions)

- Q: Do the EEPROM position limits apply in every operating mode? → A: No. Limits follow the operating mode, as the firmware does. Each actuator's cache holds a one-byte limit mode, decided at Setup: position mode checks the goal against the limits (as before); extended position, current-based position and MX multi-turn check the register size only; velocity, current, PWM and wheel mode refuse every goal position with a validation error, since the firmware ignores it. The mode comes from the Operating Mode register (X, MX(2.0), PRO, PRO+, Y), the XL-320 Control Mode register, or the CW/CCW Angle Limits (AX, MX on Protocol 1). A value not listed in the family's e-Manual fails Setup with a typed error naming the actuator and the value. The cache stays 16 bytes.
- Q: Does a hardware fault reported on a `raw-registers` reply set the faulted state? → A: Yes. Raw replies get the same fault check as every other reply; constitution III has no exception. Core is unchanged: recording a fault already gives faulted from any state.
- Q: Does a `partial-reads` result fail every slot of a Protocol 2 sync group when one actuator fails? → A: No. Each actuator keeps its own result; only a failed instruction fails every slot of the group. Core is unchanged (bus types).
- Q: `UnsupportedModel` gains the actuator's `Id` and the protocol beyond `{ model_number, feature }` (cross-module conflict 19). Accepted? → A: Yes ("fine by me"). The setup error is `UnsupportedModel { id, model_number, protocol, feature }`.
- Q: Where do the operating mode value lists live: in core per family, or in the control tables? → A: In the tables. Each table carries its own model's lists (position, unbounded, no position), copied from that model's e-Manual page; codegen emits them with the model. Core decodes a value with them and holds no per-family list, so a new model or family is a data change only. Value 5 shows why: unbounded on X and MX(2.0), position mode on RH-P12-RN.
- Q: Should the position limits a developer reads return nothing outside position mode, where the firmware ignores them? → A: No. They return the EEPROM limits in every mode; their documentation says the limits are applied to goals only in position mode.
- Q: Who removes the protocol module's unused broadcast ping helper and the lint expectation on the protocol module that only it keeps fulfilled? → A: The first core PR, as a flagged touch of the protocol module.

## User Scenarios & Testing *(mandatory)*

The "users" of this module are robot application developers and embedded firmware developers
(SRS 1.4), who see its types in every call they make, and the `bus` module, which builds its
guards and its multi-actuator ops on top of them. Core does no I/O. Everything here is tested
without a bus, a transport or hardware.

### User Story 1 - Give a goal in radians and have it checked against the real limits (Priority: P1)

A developer works in radians, not in encoder ticks. They give a goal position as an angle, and
the crate turns it into the right tick count for that actuator's model. If the angle is outside
the limits read from that actuator's EEPROM, they get a typed error that names the actuator, the
requested value and the limits, and nothing is sent.

**Why this priority**: FR-3.6 and FR-3.7 are Decided, and the `bus` module's goal writes
(FR-3.4, FR-4.8) cannot ship without a typed position and a limit check. A wrong conversion
or a missing check drives a motor where the user did not ask.

**Independent Test**: For two models with different scales (e.g. XM430-W350, 4096 ticks per
turn with zero radians at tick 2048; AX-12A, 1024 ticks over 300° with zero radians at tick
512), convert angles to ticks and back, and check accepted and rejected goals against a set of
recorded limits. No bus involved.

**Acceptance Scenarios**:

1. **Given** an XM430-W350, **When** the developer gives a goal of 0 rad, **Then** it becomes
   the model's zero-radian tick (2048).
2. **Given** any model, **When** a tick inside the model's position range is turned into an
   angle and back, **Then** the same tick comes out.
3. **Given** limits of ticks 1000..=3000 recorded for actuator 3, **When** the developer gives
   a goal that converts to tick 3001, **Then** a validation error names actuator 3, the
   requested tick and both limits.
4. **Given** the same limits, **When** the goal converts exactly to tick 1000 or 3000, **Then**
   it is accepted.
5. **Given** an angle that is not a number or is infinite, **When** it is used as a goal,
   **Then** a validation error is returned, never a panic.
6. **Given** an actuator whose limits were never recorded, **When** a goal is checked, **Then**
   a validation error is returned; no default from the control table is used.
7. **Given** limits 1000..=3000 recorded for actuator 3 and its mode recorded as extended
   position, **When** a goal converts to tick 5000, **Then** it is accepted (the firmware does
   not use the limits in that mode).
8. **Given** actuator 4 whose mode is recorded as velocity (no position), **When** any goal is
   checked, inside the limits or not, **Then** a validation error names actuator 4 and says it
   is not in a position mode.
9. **Given** the value lists of an X-series table (3 position; 4, 5 extended; 0, 1, 16 no
   position), **When** core maps an Operating Mode value with them, **Then** 3 gives position
   mode, 4 and 5 give extended (no limit check), 0, 1 and 16 give no position, and any other
   value gives no mode, which Setup reports as an error naming the actuator and the value. With
   the RH-P12-RN lists (5 position, 0 no position), the same value 5 gives position mode.
10. **Given** an AX actuator whose CW and CCW Angle Limits are both 0, **When** its mode is
    derived, **Then** it is no position (wheel mode); both 4095 on an MX on Protocol 1 gives
    extended (multi-turn); any other pair gives position mode.

---

### User Story 2 - Write code once that works on any actuator model (Priority: P1)

A developer writes a function that takes "any actuator" and uses what all models share: its
`Id`, model number and name, its position limits, and where its torque, goal position, present
position and limit registers are. The same function works on an XM430, an AX-12A or a PRO
model, and on the closed enum that holds any of them. Calls resolve at compile time.

Torque and fault state are not part of the actuator. They are one small state value per
actuator (torque off, torque on, or faulted), defined here with its rules and stored by the
`bus` module beside the actuator list. This keeps each actuator small.

**Why this priority**: FR-3.2 is Tier 1 of the API (SRS 4.2). The `bus` module's
`ActuatorRef` and every multi-actuator op are generic over models through it. Without it each
model needs its own code path.

**Independent Test**: A generic test function, run over every compiled-in model, reads the
common registers and position scale and checks them against the model's control table, and
records limits in the cache. A separate test drives the state value through every change and
checks its rules.

**Acceptance Scenarios**:

1. **Given** an XM430-W350 and an AX-12A, **When** the same generic function asks each for its
   goal position register, **Then** it gets address 116 (4 bytes) and address 30 (2 bytes).
2. **Given** a freshly created actuator of any model, **When** its cache is read, **Then** it
   holds its `Id`, its limit mode is position mode until Setup records the real one, limits are
   not yet known, and it holds no torque or fault state.
3. **Given** a new state value, **When** it is read, **Then** it is torque off.
4. **Given** a state value of torque on, **When** a fault is recorded, **Then** it becomes
   faulted in one step, and faulted counts as off.
5. **Given** a state value of faulted, **When** torque on is requested, **Then** the change
   is refused and the value stays faulted.
6. **Given** an AX model, whose limits are the CW and CCW Angle Limit registers, **When** the
   generic function asks for the minimum and maximum position limit registers, **Then** it gets
   them under the same names as for an X-series model.
7. **Given** the closed enum holding any model, **When** a common operation is called on it,
   **Then** it gives the same answer as the model inside it.

---

### User Story 3 - Handle every failure by its category (Priority: P2)

A developer matches one error type from every fallible call. They first match the category
(transport, protocol, status, validation, setup), then the detail. Each message
says which actuator it is about, when one is involved. The errors work the same in `no_std`
and `std` builds.

**Why this priority**: FR-6.4 is Decided. Most categories already exist (see Assumptions);
the gap is the validation errors the `bus` module needs (`ActuatorIsOff`, out of limits, bad
buffer) and the new `Setup` category for Init failures, which also takes in the existing
unsupported model error.

**Independent Test**: Build each error, match it by category, compare it, copy it, and check
its message text. No bus involved.

**Acceptance Scenarios**:

1. **Given** a goal write refused because torque is off on actuator 4, **When** the error is
   shown, **Then** the message names actuator 4 and says torque is off.
2. **Given** a multi-actuator read whose buffer holds 2 slots for 3 targets, **When** the
   error is shown, **Then** it gives both numbers.
3. **Given** any error, **When** the developer matches on it, **Then** the category comes
   first and the detail second, and a new variant added later does not break their match.
4. **Given** a `no_std` build, **When** an error is created and compared, **Then** it needs no
   allocation.
5. **Given** a declared actuator 5 that does not answer at Init, **When** the error is shown,
   **Then** it is in the setup category and the message names actuator 5.
6. **Given** a model number the build does not support, **When** the error is matched,
   **Then** it is found under the setup category, not at the top level, and its message names
   the actuator, the model number and the protocol.
7. **Given** actuator 5 reporting an operating mode value its family does not list, **When**
   the error is shown, **Then** it is in the setup category and names actuator 5 and the value.
8. **Given** a goal refused because actuator 4 is not in a position mode, **When** the error is
   shown, **Then** it is in the validation category and names actuator 4.

---

### User Story 4 - Drop to raw register values when needed (Priority: P3)

A developer who needs exact ticks (calibration, debugging, a model whose scale they distrust)
uses the raw form of every typed value. Raw values reach the wire unchanged, and the same limit
check applies to them.

**Why this priority**: FR-3.6 requires a raw form of each typed value. It is a second door to
the same check, so it comes after the typed path.

**Independent Test**: Pass raw ticks through the limit check and the register's byte encoding;
check they come out unchanged and that out-of-limit ticks are refused.

**Acceptance Scenarios**:

1. **Given** a raw goal of 2500 ticks within limits, **When** it is checked and encoded for a
   4-byte register, **Then** the bytes encode exactly 2500.
2. **Given** a raw goal outside the recorded limits, **When** it is checked, **Then** the same
   validation error as the typed path is returned.
3. **Given** a raw value that does not fit the register's size (e.g. 70 000 for a 2-byte
   register), **When** it is encoded, **Then** a validation error is returned and nothing is
   truncated.

---

### Edge Cases

- An angle falls between two ticks: it rounds to the nearest tick. The limit check runs on the
  rounded tick.
- An angle below zero on a model whose ticks are unsigned with a centered zero (AX: tick 512):
  it gives a tick below the center, and ticks below 0 are refused as not representable.
- An angle beyond one turn on a model in extended position (multi-turn) mode: the conversion
  stays linear, and the EEPROM limits are not applied in that mode (decided 2026-10-28). Only the
  register size check stops a value the register cannot hold.
- A goal for an actuator in velocity, current, PWM or wheel mode: refused with a validation
  error naming the actuator, even when its limits are unknown; nothing is sent.
- An Operating Mode or Control Mode value in none of the model's value lists: no mode is
  guessed; Setup fails with a setup error naming the actuator and the value.
- The operating mode changes after Setup (a `raw-registers` write to it): the recorded mode
  stays stale until the next Setup. The `bus` module documents this on its raw write.
- Limits read from EEPROM with the minimum above the maximum: recording them fails with a
  validation error, so Init fails (FR-2.6).
- A fault is recorded while the state is already torque off: it becomes faulted, which is
  still off.
- A fault is recorded while the state is already faulted: it stays faulted.
- Torque on is requested while the state is faulted: refused by the state rules, so the
  "faulted means off" rule cannot be broken from outside.
- A fault is cleared: faulted becomes torque off, never torque on. Clearing a state that is not
  faulted changes nothing.
- The status report that caused a fault is not kept: the call that saw it returns it in its
  error, and Hardware Error Status can be read again.
- A status packet carries an ID byte that is not a valid `Id` (corrupted): the protocol error
  keeps the raw byte (existing behaviour, kept).
- `Id` 253 on Protocol 1 stays invalid (existing decision, kept).

## Requirements *(mandatory)*

### Functional Requirements

**Typed position (FR-3.6, OQ-3)**

- **FR-001**: The module MUST provide a `Position` value whose SI form is radians as a
  single-precision float. It MUST offer a zero constant, comparison, and adding or subtracting
  an angle, so a goal can be moved by an offset in one expression.
- **FR-002**: The module MUST convert between `Position` and raw ticks with a per-model
  position scale: the tick that means zero radians and the radians per tick. Each model's scale
  is data from its control table, not code in core.
- **FR-003**: Converting an angle to ticks MUST round to the nearest tick. An angle that is not
  finite, or whose tick is outside what the target register can hold, MUST give a validation
  error. No conversion may panic.
- **FR-004**: Every typed value MUST have a raw form usable in both directions. A raw value MUST
  reach the register encoding unchanged.

**Limits and the limit check (FR-3.7, FR-2.6)**

- **FR-005**: The module MUST provide a limits record holding a minimum and maximum position in
  raw ticks, as read from the actuator's EEPROM. It MUST expose them both raw and as
  `Position`, in every limit mode (the documentation says they are applied to goals only in
  position mode, FR-021; decided 2026-10-28). Recording a minimum above the maximum MUST fail
  with a validation error.
- **FR-006**: The module MUST provide one limit check, used by both the typed and the raw path,
  that, in position mode (FR-021), accepts a tick inside the limits (bounds included) and
  otherwise returns a validation error naming the `Id`, the requested tick and both limits. The
  check needs no bus and sends nothing.
- **FR-007**: In position mode, until limits are recorded, the limit check MUST return a
  validation error. Core MUST NOT offer any way to fill limits from control-table defaults.

**Actuator cache**

- **FR-008**: The module MUST provide the per-actuator cache record, held in each actuator:
  `Id`, limit mode (FR-021) and limits. A new cache is in position mode and has no limits. The
  cache MUST NOT hold torque state or fault state (deviation from SRS 3.3, see Assumptions).
- **FR-009**: The module MUST provide one actuator state type, one byte in size, with three
  states: torque off, torque on, faulted. Faulted always means off. A new state is torque off.
  Recording a fault MUST give faulted in one step, from any state. Requesting torque on while
  faulted MUST be refused. Clearing a fault is a separate, explicit step that gives torque off.
  Core defines the type and its rules only; the `bus` module stores one state per actuator slot
  in its own array beside the actuator list. No fault report is stored per actuator.

**`Actuator` trait (FR-3.2, OQ-2)**

- **FR-010**: The `Actuator` trait MUST give, for every model: `Id`, model number and name
  (existing); the locations of the Torque Enable, Goal Position, Present Position, and minimum
  and maximum position limit registers; the position scale; and read and write access to the
  cache. Models whose limit registers have other names (AX/MX CW and CCW Angle Limit) map them
  to minimum and maximum. The trait only describes the model (registers, scale, cache); it
  performs no I/O. Enabling torque, setting a goal and reading a position live on `ActuatorRef`
  in the `bus` module (single responsibility).
- **FR-011**: The closed `ActuatorKind` enum MUST implement the trait by `match` over its
  variants. No dynamic dispatch, and no runtime lookup from `Id` or model number to a register
  address (NFR-4). The generated model types implement the trait through `cargo xtask`.
- **FR-012**: Registers beyond the four of OQ-2 MUST NOT be added to the trait in this feature.

**Register (FR-3.2)**

- **FR-013**: The `Register` type MUST keep each register's address, size, access and memory
  area, and MUST turn a raw value into the little-endian bytes the register occupies and back.
  A value that does not fit the register MUST give a validation error, never a truncation.

**Errors (FR-6.4)**

- **FR-014**: The crate MUST keep one error enum, matched by category first: `Transport`,
  `Protocol`, `Status`, `Validation`, and `Setup`.
- **FR-015**: `Validation` MUST cover, besides the existing invalid `Id`: an actuator whose
  state is torque off or faulted (`ActuatorIsOff`, naming the `Id`); a tick outside the limits (FR-006);
  limits not yet known (FR-007); limits recorded with minimum above maximum (FR-005); a value
  that is not finite or does not fit its register (FR-003, FR-013); a buffer whose length
  does not match the target count (both numbers given); and a goal for an actuator that is not
  in a position mode (`NotPositionMode`, naming the `Id`, FR-021).
- **FR-016**: A new `Setup` category MUST hold every Init failure that is neither a transport,
  protocol nor status error, each as a typed error: more than `N` actuators found (FR-1.2), a
  declared actuator missing, a declared actuator answering with another model (FR-2.5), a
  target `Id` not on the bus, an unsupported model (`UnsupportedModel { id, model_number,
  protocol, feature }`), and an operating mode value the family does not list
  (`UnknownOperatingMode { id, value }`, FR-023). `UnsupportedModel` moves from a top-level
  `Error` variant into `Setup` and gains `id` and `protocol` (cross-module conflict 19, accepted
  by the architect on 2026-10-28); this is a breaking change, accepted during alpha.
- **FR-017**: Every error MUST be copyable and comparable, need no allocation, display a
  message that names the actuator when one is involved, and be open to new variants without
  breaking a user's match.

**Cross-cutting (NFR-3, NFR-4, constitution I, VI, VII)**

- **FR-018**: No core code may panic on any input, allocate, or use dynamic dispatch. Core
  MUST build without `std` on the CI microcontroller target.
- **FR-019**: Every public core item MUST be documented, including the unit and zero
  reference of `Position`.
- **FR-020**: Enabling a Cargo feature MUST only add core API, never change an existing item's
  behaviour.

**Limit mode (FR-3.7, architect decision 2026-10-28: limits follow the operating mode)**

- **FR-021**: The cache MUST hold one limit mode per actuator, in the cache's existing padding
  (the cache does not grow): position mode (goals checked against the limits, FR-006,
  FR-007), unbounded (extended position, current-based position, MX multi-turn: the limits are
  not applied; only the register size check of FR-013 stops a value), or no position (velocity,
  current, PWM, wheel mode: every goal is refused with `NotPositionMode`, whatever the limits).
  A new cache is in position mode; Setup records the real mode.
- **FR-022**: Every model MUST name where its mode comes from: its mode register (Operating Mode,
  or the XL-320 Control Mode) with the model's own value lists (position, unbounded, no
  position), or (AX, MX on Protocol 1) the CW and CCW Angle Limits already read (both 0: no
  position; both 4095: unbounded; else position). The value lists are control-table data,
  copied from the model's e-Manual page (decided 2026-10-28); core holds no per-family list.
  Core MUST map a value to the limit mode of the list that holds it. The source and lists per
  model are generated data; the mapping is core code. Both stay crate-internal (not on the
  public trait, FR-012).
- **FR-023**: A mode value in none of the model's lists MUST map to no mode. Setup then fails
  with `UnknownOperatingMode { id, value }`; no mode is guessed (constitution III).

### Key Entities

- **Position**: an angle in radians; the SI form of every position goal, reading and limit.
- **Raw ticks**: the integer a position register holds; the raw form of `Position`.
- **Position scale**: per model, the zero-radian tick and the radians per tick. Data from the
  control table.
- **Limits**: minimum and maximum position in raw ticks, read from EEPROM at Init. Unknown
  until recorded.
- **Actuator cache**: per actuator, held in its model type: `Id`, limit mode and limits. No
  torque or fault state.
- **Limit mode**: one byte per actuator, in the cache: position (limits checked), unbounded
  (register size only), or no position (goals refused). Decided at Setup from the operating
  mode; position until then.
- **Mode source**: per model, where Setup gets the limit mode: the Operating Mode (or XL-320
  Control Mode) register with the model's value lists from its control table, or the CW/CCW
  Angle Limits.
- **Actuator state**: one byte per actuator: torque off, torque on, or faulted (faulted means
  off). Starts torque off; a fault gives faulted in one step; torque on from faulted is refused.
  Defined in core, stored by the `bus` module in an array beside the actuator list.
- **`Actuator` trait**: what every model shares (identity, the four OQ-2 registers, position
  scale, cache).
- **Register**: one control-table location: address, size, access and memory area, with byte
  encoding of a raw value.
- **Error**: one enum by category (`Transport`, `Protocol`, `Status`, `Validation`, `Setup`);
  `Validation` holds every "rejected before sending" case; `Setup` holds every Init failure
  that is not transport, protocol or status, including unsupported model.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For every compiled-in model, every tick in the model's documented position range
  survives tick → `Position` → tick unchanged (100 % of ticks tested).
- **SC-002**: For every model in position mode, a goal one tick inside each limit is accepted
  and one tick outside is rejected, and non-finite angles are rejected, with zero panics across
  the test.
- **SC-003**: One generic test function, written once, passes for 100 % of compiled-in models
  and for the closed enum holding each of them.
- **SC-004**: Every error named in FR-015 and FR-016 exists, and each has a test that checks
  its category and its message (100 %).
- **SC-005**: A developer moves a goal by a small angle in one expression, with no tick
  arithmetic in their code.
- **SC-006**: The `no_std` CI build passes with core compiled in; core contains zero dynamic
  dispatch and zero allocation.
- **SC-007**: Core reaches at least 70 % line coverage without hardware.
- **SC-008**: The actuator state type is exactly one byte, and the actuator cache record holds
  no torque or fault field and stays 16 bytes with the limit mode in it.
- **SC-009**: For each set of value lists under test (at least the X-series, RH-P12-RN and
  XL-320 lists of the e-Manual), all 256 one-byte values are mapped in a test: every listed
  value gives its list's limit mode, every other value gives no mode (100 %).

## Assumptions

- **Already built (PR #3, on `dev`)**: `Id` (`0..=252`, const-checked, `TryFrom<u8>`);
  `Error` with `Validation(ValidationError::InvalidId)` and a top-level
  `UnsupportedModel { model_number, feature }` (this feature moves it into `Setup`, FR-016;
  a breaking change, accepted during alpha); the `Actuator` trait with `id`, `model_number`
  and `name`, implemented by every generated model type and by `ActuatorKind`; and a
  crate-private `Register` with address and size, emitted by codegen as associated consts.
  The plan covers only the gaps against FR-001 to FR-020.
- **Protocol PR #7** (merged into `main`) adds `Transport`, `Protocol` and `Status` categories
  to `src/error.rs` (`TransportError`, `ProtocolError`, `StatusError` with `StatusReport`,
  `P1Flags`, `P2Code`, `is_hardware_fault`), three validation variants (address out of range,
  packet too large, data length), and `ProtocolVersion`, already re-exported from the crate
  root. This feature reuses that shape and does not redefine it. PR #7's `P2::broadcast_ping`
  is never called (Setup scans with `scan`), and the `expect(dead_code)` on `mod protocol;` in
  `src/lib.rs` exists only for it: core-1 deletes both, a flagged protocol touch (architect,
  2026-10-28).
- **OQ-2 default**: the trait covers Torque Enable, Goal Position, Present Position and the
  position limits only; anything else is `unimplemented!("SRS OQ-2")` where needed (SRS 5.0).
- **OQ-3 default**: `Position` is radians in `f32`; raw ticks stay available.
- **Zero reference**: zero radians is the tick the control table names as the zero-radian
  position (`type_info.value_of_zero_radian_position`, e.g. 2048 for XM430, 512 for AX-12A),
  so angles are signed around the center. The scale values come from the `tables` module and
  are verified against the e-Manual there (constitution IV). The AX tables' `type_info`
  (`min_radian: 0.0` with the zero at tick 512) looks inconsistent and needs that check.
- **Other SI newtypes** (velocity, current, voltage, temperature) are out of scope until the
  trait grows past OQ-2. Degree helpers on `Position` are not added now.
- **Deviation from SRS 3.3 (architect, 2026-10-27)**: SRS 3.3 puts torque state and fault
  state as two flags in each actuator's cache. This spec replaces them with one combined
  1-byte state (torque off, torque on, faulted) kept outside the actuator structs, because
  flags inside each struct add padding and low memory use comes first. The `bus` module holds
  the per-slot state array; SRS 3.3, FR-2.7 and FR-6.2 wording about "the cache" maps to that
  array.
- **No fault report is kept.** The status report that caused a fault is returned in the error
  of the call that saw it (FR-6.2); Hardware Error Status can be read again. Neither the state
  nor the cache stores it.
- **Register** carries address, size, access (R or RW) and memory area (EEPROM or RAM), as
  every table records them now (architect, 2026-10-27; cross-module conflict 2). Range, default
  and unit come per model when the model is verified against the e-Manual; until then range and
  unit are carried by typed values and the position scale.
- **I/O stays in the `bus` module**: the per-slot state array, torque writes, goal writes, reads, fault detection
  (FR-6.2), `clear_fault` and the "goal while off" guard (FR-3.4) use core types but are not
  built here. Reading the mode register at Setup and recording the mode is the bus's; the limit
  mode, its check and the mapping are core's; the value lists are table data (FR-021 to FR-023,
  decided 2026-10-28).
- **`ModelNumber`** (FR-2.5) is a core item in `src/actuator.rs` (cross-module conflict 9): the
  input type of declared setup and of the generated lookup.
- **Generated side**: extending the trait needs every generated model type to implement the new
  items, and `cargo xtask` to emit the scale, the limit register mapping, the mode source (with
  the value lists from each table's `operating_modes`) and the cache field. That is `xtask`
  module work (SRS §9). Sequencing is decided: core-1 (additive, with shims) → xtask X1 →
  core-2 (trait cutover); core never regenerates (plan, module-interfaces §7).
- `src/units.rs` and `src/units/**` are new paths in this feature.
