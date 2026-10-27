# Feature Specification: Core (typed values, Actuator trait, errors)

**Feature Branch**: `004-core` (not created; this spec was written on `dev`)

**Created**: 2026-10-27

**Status**: Draft

**Input**: User description: "core module of @docs/srs/modules/core.md"
→ SRS module `core` (SRS §9): FR-3.2 (the `Actuator` trait and the `Register` type), FR-3.6,
FR-3.7, FR-6.4, NFR-3, NFR-4; delivery steps 1 and 5. Paths: `src/lib.rs`, `src/id.rs`,
`src/error.rs`, `src/actuator.rs`, `src/register.rs`, `src/units.rs`, `src/units/**`. No-go:
`src/models.rs`, `src/models/**` unless a task names them.

## Clarifications

### Session 2026-10-27

- Q: Does the `Actuator` trait carry the I/O operations or only describe the model? → A: It only describes the model (registers, scale, cache); I/O lives on `ActuatorRef` in the bus (single responsibility).

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

---

### User Story 2 - Write code once that works on any actuator model (Priority: P1)

A developer writes a function that takes "any actuator" and uses what all models share: its
`Id`, model number and name, its torque state and fault state, its position limits, and where
its torque, goal position, present position and limit registers are. The same function works
on an XM430, an AX-12A or a PRO model, and on the closed enum that holds any of them. Calls
resolve at compile time.

**Why this priority**: FR-3.2 is Tier 1 of the API (SRS 4.2). The `bus` module's
`ActuatorRef` and every multi-actuator op are generic over models through it. Without it each
model needs its own code path.

**Independent Test**: A generic test function, run over every compiled-in model, reads the
common registers and position scale and checks them against the model's control table; it
records limits and toggles cached torque and fault state, and checks the cache rules.

**Acceptance Scenarios**:

1. **Given** an XM430-W350 and an AX-12A, **When** the same generic function asks each for its
   goal position register, **Then** it gets address 116 (4 bytes) and address 30 (2 bytes).
2. **Given** a freshly created actuator of any model, **When** its cache is read, **Then**
   torque state is off, fault state is clear and limits are not yet known.
3. **Given** an actuator with torque state on, **When** a fault is recorded, **Then** fault
   state is set and torque state is off.
4. **Given** an AX model, whose limits are the CW and CCW Angle Limit registers, **When** the
   generic function asks for the minimum and maximum position limit registers, **Then** it gets
   them under the same names as for an X-series model.
5. **Given** the closed enum holding any model, **When** a common operation is called on it,
   **Then** it gives the same answer as the model inside it.

---

### User Story 3 - Handle every failure by its category (Priority: P2)

A developer matches one error type from every fallible call. They first match the category
(transport, protocol, status, validation, unsupported model), then the detail. Each message
says which actuator it is about, when one is involved. The errors work the same in `no_std`
and `std` builds.

**Why this priority**: FR-6.4 is Decided. Most categories already exist (see Assumptions);
the gap is the validation errors the `bus` module needs (`ActuatorIsOff`, out of limits, bad
buffer) and a home for Init failures.

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
  stays linear. Whether the limit check applies in that mode is the `bus` module's decision
  (see Assumptions).
- Limits read from EEPROM with the minimum above the maximum: recording them fails with a
  validation error, so Init fails (FR-2.6).
- A fault is recorded while torque state is already off: fault state is set; torque stays off.
- Torque is set on in the cache while fault state is set: refused by the cache, so the
  "faulted means torque off" rule cannot be broken from outside.
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
  `Position`. Recording a minimum above the maximum MUST fail with a validation error.
- **FR-006**: The module MUST provide one limit check, used by both the typed and the raw path,
  that accepts a tick inside the limits (bounds included) and otherwise returns a validation
  error naming the `Id`, the requested tick and both limits. The check needs no bus and sends
  nothing.
- **FR-007**: Until limits are recorded, the limit check MUST return a validation error. Core
  MUST NOT offer any way to fill limits from control-table defaults.

**Actuator cache**

- **FR-008**: The module MUST provide the per-actuator cache record of SRS 3.3: `Id`, torque
  state, fault state and limits. A new cache has torque state off, fault state clear and no
  limits.
- **FR-009**: Recording a fault MUST set torque state off in the same step. Setting torque
  state on while fault state is set MUST be refused. Clearing fault state is a separate,
  explicit step.

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

- **FR-013**: The `Register` type MUST keep each register's address and size, and MUST turn a
  raw value into the little-endian bytes the register occupies and back. A value that does not
  fit the register MUST give a validation error, never a truncation.

**Errors (FR-6.4)**

- **FR-014**: The crate MUST keep one error enum, matched by category first: `Transport`,
  `Protocol`, `Status`, `Validation`, and `UnsupportedModel`.
- **FR-015**: `Validation` MUST cover, besides the existing invalid `Id`: an actuator whose
  torque state is off (`ActuatorIsOff`, naming the `Id`); a tick outside the limits (FR-006);
  limits not yet known (FR-007); limits recorded with minimum above maximum (FR-005); a value
  that is not finite or does not fit its register (FR-003, FR-013); and a buffer whose length
  does not match the target count (both numbers given).
- **FR-016**: Init failures that are neither transport, protocol nor status errors MUST each
  have a typed error: more than `N` actuators found (FR-1.2), a declared actuator missing, a
  declared actuator answering with another model (FR-2.5), and a target `Id` not on the bus.
  [NEEDS CLARIFICATION: Where do these go? Options: inside `Validation`; a new `Setup` category
  (adds to the FR-6.4 list); or top-level variants beside `UnsupportedModel`.]
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

### Key Entities

- **Position**: an angle in radians; the SI form of every position goal, reading and limit.
- **Raw ticks**: the integer a position register holds; the raw form of `Position`.
- **Position scale**: per model, the zero-radian tick and the radians per tick. Data from the
  control table.
- **Limits**: minimum and maximum position in raw ticks, read from EEPROM at Init. Unknown
  until recorded.
- **Actuator cache**: per actuator, `Id`, torque state (on/off), fault state (set/clear) and
  limits. Rule: fault set means torque off.
- **`Actuator` trait**: what every model shares (identity, the four OQ-2 registers, position
  scale, cache).
- **Register**: one control-table location: address and size, with byte encoding of a raw value.
- **Error**: one enum by category; `Validation` holds every "rejected before sending" case.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For every compiled-in model, every tick in the model's documented position range
  survives tick → `Position` → tick unchanged (100 % of ticks tested).
- **SC-002**: For every model, a goal one tick inside each limit is accepted and one tick
  outside is rejected, and non-finite angles are rejected, with zero panics across the test.
- **SC-003**: One generic test function, written once, passes for 100 % of compiled-in models
  and for the closed enum holding each of them.
- **SC-004**: Every error named in FR-015 and FR-016 exists, and each has a test that checks
  its category and its message (100 %).
- **SC-005**: A developer moves a goal by a small angle in one expression, with no tick
  arithmetic in their code.
- **SC-006**: The `no_std` CI build passes with core compiled in; core contains zero dynamic
  dispatch and zero allocation.
- **SC-007**: Core reaches at least 70 % line coverage without hardware.

## Assumptions

- **Already built (PR #3, on `dev`)**: `Id` (`0..=252`, const-checked, `TryFrom<u8>`);
  `Error` with `Validation(ValidationError::InvalidId)` and
  `UnsupportedModel { model_number, feature }`; the `Actuator` trait with `id`, `model_number`
  and `name`, implemented by every generated model type and by `ActuatorKind`; and a
  crate-private `Register` with address and size, emitted by codegen as associated consts.
  The plan covers only the gaps against FR-001 to FR-020.
- **Protocol PR #7** (branch `001-protocol-packets`, not on `dev`) adds `Transport`,
  `Protocol` and `Status` categories to `src/error.rs` (`TransportError`, `ProtocolError`,
  `StatusError` with `StatusReport`, `P1Flags`, `P2Code`, `is_hardware_fault`) and three
  validation variants (address out of range, packet too large, data length). This feature
  reuses that shape. If PR #7 is not merged when core is planned, the plan takes those
  definitions as they are rather than redefining them.
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
- **Torque and fault state** stay two fields as in SRS 3.3 (bool plus flag). The `TorqueState`
  enum of the bus proposal (`specs/ideas/bus-proposal.md`, Q4) is not adopted here; if the
  architect adopts it, the cache record changes in the `bus` spec.
- **Fault state is a flag.** The status report that set it is returned in the error of the
  call that saw it (FR-6.2); the cache does not store it.
- **Register** carries address and size only. SRS 3.3 also lists access, range and unit, but
  the RON tables hold no access field yet, and range and unit are carried by typed values and
  the position scale. This is a known gap against SRS 3.3, flagged for the architect.
- **I/O stays in the `bus` module**: torque writes, goal writes, reads, fault detection
  (FR-6.2), `clear_fault` and the "goal while off" guard (FR-3.4) use core types but are not
  built here. Whether the limit check applies in extended position or wheel mode (bus proposal
  R3) is decided there.
- **`ModelNumber`** (FR-2.5) is a `bus` module item; it may land in core if the bus plan asks.
- **Generated side**: extending the trait needs every generated model type to implement the new
  items, and `cargo xtask` to emit the scale, the limit register mapping and the cache field.
  That is `xtask` module work (SRS §9). The core plan must keep the crate building: either a
  task names `src/models.rs`/`src/models/**` for one regeneration, or the xtask change lands
  with or before core. This sequencing is a plan decision for the architect.
- `src/units.rs` and `src/units/**` are new paths in this feature.
