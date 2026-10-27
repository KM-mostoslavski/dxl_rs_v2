# Feature Specification: Bus (setup, single- and multi-actuator access, faults)

**Feature Branch**: `003-bus` (spec written on `dev`; no branch created yet)

**Created**: 2026-10-27

**Status**: Draft

**Input**: User description: "spec for the bus module"
→ SRS module `bus` (`docs/srs/modules/bus.md`): FR-1, FR-2, FR-3.1 to FR-3.5, FR-3.8, FR-4,
FR-6.2, FR-6.3, FR-6.5, FR-8, NFR-1, NFR-2; delivery steps 4, 5, 6, 9, 10. Design input:
`specs/ideas/bus-proposal.md`. No-go: `src/models.rs`, `src/models/**`, `control_tables/**`.

## Clarifications

### Session 2026-10-27

- Q: Q1: is the time source part of the bus's type or carried another way? → A: A fourth type parameter with a default (system clock under std): zero bytes for a stateless clock, no indirect call, works with an owned hardware timer.
- Q: Q2: is autoscan available in no_std? → A: Yes, if possible; checked: it needs no allocation (broadcast ping, P1 sweep, model lookup, fixed-capacity list), so it is available.
- Q: Q4: one combined torque/fault state or two flags? → A: One combined state (off / on / faulted) in one byte; flags cost padding and a low memory footprint is preferred.

## User Scenarios & Testing *(mandatory)*

The "users" of this module are robot application developers on a PC or SBC, and embedded
firmware developers on a microcontroller (SRS 1.4). The bus is the first public surface they
touch: everything they do with actuators goes through it.

Every story below is tested without hardware against the byte-level mock transport (`mock`
feature) and a test clock. Stories that also want a manual hardware check say so; none
depends on one.

### User Story 1 - Declare my actuators and move one safely (Priority: P1, MVP)

A developer lists the actuators they expect (ID and model number), creates a bus on one wire,
and runs setup. Setup checks each listed actuator answers with the declared model and reads
its real limits from the actuator. Torque starts off. The developer enables torque on one
actuator, writes a goal position, and reads the present position back. Any goal write while
torque is off, or outside the actuator's limits, is refused before anything is sent.

**Why this priority**: This is delivery step 4 and the smallest slice that moves a motor. It
carries the three safety rules the whole crate is built on: torque off at start, limits from
the actuator, nothing hidden.

**Independent Test**: Script the mock with ping replies and limit-read replies for two declared
actuators. Run setup, then: a goal write before enabling torque (refused, zero bytes sent);
enable torque (one acknowledged write); a goal inside limits (sent byte-exact); a goal outside
limits (refused, zero bytes sent); a present-position read (decoded). Optional manual check:
the same on one XM430-W350 through a U2D2.

**Acceptance Scenarios**:

1. **Given** a declared list whose actuators all answer with the declared model, **When**
   setup runs, **Then** each one is held by the bus with its limits read from the actuator,
   torque off and no fault, and no torque write was sent.
2. **Given** a declared actuator that does not answer, or answers with another model number,
   **When** setup runs, **Then** setup fails with a typed error naming that ID and the bus
   holds no actuators.
3. **Given** a limit read that times out or fails during setup, **When** setup runs, **Then**
   setup fails; the bus never falls back to control-table defaults.
4. **Given** more declared actuators than the bus capacity, **When** setup runs, **Then** it
   fails with a capacity error, not a panic.
5. **Given** an actuator with torque off, **When** the developer writes a goal position,
   **Then** they get "actuator is off" and nothing is sent.
6. **Given** an actuator with torque on, **When** the developer writes a goal outside its
   limits, **Then** they get a validation error and nothing is sent.
7. **Given** an actuator with torque on, **When** the developer writes a goal inside its limits,
   **Then** exactly one write goes on the wire and no read-back follows.
8. **Given** an ID the bus does not hold, **When** the developer asks for that actuator,
   **Then** they get "none", not an error and not a panic.
9. **Given** a developer holding access to one actuator, **When** they try to hold access to a
   second one at the same time, **Then** the code does not compile.
10. **Given** a newly created bus, **When** nothing else is called, **Then** no byte has been
    sent, and the developer can take the transport back out of the bus.

---

### User Story 2 - Find what is on the wire without listing it (Priority: P1)

A developer creates a bus and runs autoscan. The bus finds every actuator that answers at the
bus's baud rate, picks the right model (and the right table for its firmware version), reads
each one's limits, and holds it with torque off. A model this build was compiled without is
reported with the feature that would add it.

**Why this priority**: Autoscan is the default setup path (FR-2.1) and the fastest way from a
fresh install to a moving robot. It is delivery step 4, but it builds on Story 1's setup rules.

**Independent Test**: On a Protocol 2 bus, script one broadcast-ping round with three replies
plus their limit reads; on a Protocol 1 bus, script a sweep where only some IDs answer. Check
the held actuators, their models and limits, and the time the sweep takes on the test clock.
Optional manual check: autoscan a real chain of mixed X-series actuators.

**Acceptance Scenarios**:

1. **Given** a Protocol 2 bus with three actuators, **When** autoscan runs, **Then** one
   broadcast ping is sent and the bus holds exactly those three, sorted by ID.
2. **Given** a Protocol 1 bus, **When** autoscan runs, **Then** each ID 0..=252 is pinged in
   turn, and each wait is bounded by the configured scan wait.
3. **Given** a model whose control table depends on firmware version, **When** autoscan finds
   it, **Then** the bus reads its firmware version and uses the table for that version.
4. **Given** an actuator whose model number has no compiled-in model, **When** autoscan finds
   it, **Then** setup fails with "unsupported model" naming the model number and the feature
   that would enable it.
5. **Given** an empty bus, **When** autoscan runs, **Then** it ends with zero actuators held,
   within the time bound of SC-007.
6. **Given** a Protocol 1 bus and an actuator whose model only speaks Protocol 2, **When**
   setup finds it, **Then** setup fails naming that ID, not a later call.
7. **Given** a `std` build, **When** the developer creates a bus from a port name and a baud
   rate, **Then** they get a bus on the std serial adapter in one call, with nothing sent yet.

---

### User Story 3 - Notice a hardware fault and recover from it (Priority: P2)

An actuator overheats or overloads mid-run. The very call that received its reply returns a
typed hardware error, the bus marks that actuator faulted and torque off, and further goal
writes are refused. The developer can see the fault state per actuator, fix the cause, and
clear the fault explicitly; clearing succeeds only if the actuator itself no longer reports
one.

**Why this priority**: Fault handling is delivery step 5 and a constitution guarantee
(Principle III). It needs Story 1's held actuators to have a state to change.

**Independent Test**: Script a reply with the alert bit (P2) or an error flag (P1) on a
present-position read; check the returned error, the faulted state, a refused goal write and a
refused torque enable (zero bytes sent). Then script the fault check for "cleared" and for
"still faulted".

**Acceptance Scenarios**:

1. **Given** an actuator with torque on, **When** any reply from it reports a hardware fault,
   **Then** the call returns a typed hardware error, and the actuator is marked faulted with
   torque off.
2. **Given** a faulted actuator, **When** the developer writes a goal, **Then** they get
   "actuator is off" and nothing is sent.
3. **Given** a faulted actuator, **When** the developer enables torque, **Then** it is refused
   with a typed error and nothing is sent.
4. **Given** a faulted actuator whose hardware no longer reports a fault, **When** the developer
   clears the fault, **Then** the fault state clears and torque stays off.
5. **Given** a faulted actuator whose hardware still reports a fault, **When** the developer
   clears the fault, **Then** the call returns the hardware error and the actuator stays
   faulted.
6. **Given** a torque enable whose reply times out or is corrupted, **When** the call returns,
   **Then** the bus records torque as off, never as on.
7. **Given** a write-only loop using multi-actuator writes, **When** an actuator faults,
   **Then** the fault is seen on the next read of that actuator (documented limitation).

---

### User Story 4 - Work in real units, raw always reachable (Priority: P2)

A developer writes goals and reads positions in radians, and reads each actuator's limits in
the same unit. When they need the exact register value they use the raw form of the same
call, with the same torque and limit checks.

**Why this priority**: Typed values are delivery step 5. Story 1 already works in raw values;
this story adds the unit layer on top.

**Independent Test**: For one X-series and one AX model, check that a goal in radians becomes
the expected register value on the wire, that a read register value comes back as the expected
radians, and that the limit check gives the same answer in both forms.

**Acceptance Scenarios**:

1. **Given** an actuator with known limits, **When** the developer asks for its limits, **Then**
   they get them in radians and as raw values, both from the setup read.
2. **Given** a goal in radians inside the limits, **When** it is written, **Then** the value on
   the wire is the matching register value for that model.
3. **Given** a goal in radians that rounds to a register value outside the limits, **When** it
   is written, **Then** it is refused like a raw out-of-limits goal.
4. **Given** a present-position reply, **When** the developer reads it typed, **Then** they get
   radians; read raw, they get the register value.

---

### User Story 5 - Command many actuators in one call (Priority: P3)

A developer writes goal positions to a set of actuators in one call, given as a range or a
sparse list of IDs plus one value per ID. The bus sends one broadcast instruction per register
layout among the targets. If any target has torque off, is unknown, or gets a value outside
its limits, the whole call is refused and nothing is sent to anyone. A developer who wants to
name the instruction uses the explicit sync write instead.

**Why this priority**: Delivery step 6. Control loops need it, but single-actuator access
(Story 1) already makes the crate usable.

**Independent Test**: Target an XM430 (4-byte goal) and an XL320 (2-byte goal) together;
check that exactly two byte-exact sync writes are sent. Then put one target in torque-off
state and check that zero bytes are sent.

**Acceptance Scenarios**:

1. **Given** targets that share one register layout, **When** the developer writes their goals,
   **Then** exactly one broadcast instruction is sent and no reply is awaited.
2. **Given** targets with two different register layouts, **When** the developer writes their
   goals, **Then** exactly two broadcast instructions are sent, one per layout.
3. **Given** a target with torque off, **When** the developer writes goals, **Then** the call
   returns "actuator is off" naming that ID and nothing is sent to any target.
4. **Given** fewer values than targets, an unknown ID or a repeated ID, **When** the developer
   writes goals, **Then** the call returns a validation error and nothing is sent.
5. **Given** a Protocol 1 or Protocol 2 bus, **When** the developer uses the explicit sync
   write, **Then** it is available on both.

---

### User Story 6 - Read many actuators into my own buffer (Priority: P3)

A developer reads present positions from a set of actuators into a buffer they own and reuse
every cycle. On Protocol 2 the bus uses one sync read; on Protocol 1 it reads each actuator in
turn. The read is strict: any failure fails the whole call, and the buffer is left as it was.
No call allocates memory. The explicit sync read exists on Protocol 2 only.

**Why this priority**: Delivery step 6, the read half of a control loop. Independent of
Story 5: a monitoring tool reads without writing.

**Independent Test**: On P2, script one sync read with three replies and check the buffer
order; script slot 1 timing out and slot 3 reporting a fault, and check the returned error is
slot 3's fault, slot 3 is faulted, and the buffer is unchanged. On P1, check that N unicast
reads are sent. A compile-fail test checks the explicit sync read is absent on P1. A counting
allocator records zero allocations.

**Acceptance Scenarios**:

1. **Given** three targets on a Protocol 2 bus, **When** the developer reads positions, **Then**
   one sync read is sent and the buffer holds the values in target order.
2. **Given** three targets on a Protocol 1 bus, **When** the developer reads positions, **Then**
   three unicast reads are sent and the buffer holds the values in target order.
3. **Given** one target that times out, **When** the developer reads positions, **Then** the
   whole call fails and the buffer is unchanged.
4. **Given** one target timing out and another reporting a hardware fault, **When** the
   developer reads, **Then** the returned error is the hardware fault and that actuator is
   marked faulted.
5. **Given** a buffer shorter than the target list, **When** the developer reads, **Then** the
   call returns a validation error and nothing is sent.
6. **Given** a Protocol 1 bus, **When** the developer calls the explicit sync read, **Then**
   the code does not compile.

---

### User Story 7 - Read many actuators and see which ones failed (Priority: P4)

With the opt-in `partial-reads` feature, a developer reads a set of actuators and gets one
result per slot, so one silent actuator does not hide the others' values.

**Why this priority**: Delivery step 9, a "Could". Strict reads (Story 6) cover the normal case.

**Independent Test**: With the feature on, script a P2 sync read where one actuator times out;
check the per-slot results (on P2 the whole failed group fails). On P1, check that only the
silent slot fails. Without the feature, the method does not compile.

**Acceptance Scenarios**:

1. **Given** the feature is off, **When** the developer calls the partial read, **Then** the
   code does not compile.
2. **Given** a Protocol 1 bus and one silent target, **When** the developer reads partially,
   **Then** that slot holds a timeout and the other slots hold values.
3. **Given** a Protocol 2 bus and a failed sync group, **When** the developer reads partially,
   **Then** every slot of that group holds the error.

---

### User Story 8 - Reach any register when the typed API does not cover it (Priority: P4)

With the opt-in `raw-registers` feature, a developer reads or writes any address and size on
one actuator, bypassing the torque guard, the limit check and the bus's per-actuator state.

**Why this priority**: Delivery step 9, a "Could". An escape hatch for debugging and registers
the typed API does not wrap yet.

**Independent Test**: With the feature on, write raw bytes while torque is off and check they
are sent byte-exact; read raw bytes back; check the per-actuator state did not change. Without
the feature, the methods do not compile.

**Acceptance Scenarios**:

1. **Given** the feature is off, **When** the developer calls a raw access method, **Then** the
   code does not compile.
2. **Given** an actuator with torque off, **When** the developer writes raw bytes, **Then** they
   are sent as given, with no guard and no limit check.
3. **Given** a raw reply that reports a hardware fault, **When** the call returns, **Then** it
   returns the typed hardware error.

---

### User Story 9 - Hold any number of actuators on a desktop (Priority: P5)

Under `std`, a developer who does not want to pick a capacity uses a bus variant with no upper
bound on the number of actuators. The capped bus stays the default and keeps its behaviour.

**Why this priority**: Delivery step 10, a "Could". The default capacity (32) already covers
most robots.

**Independent Test**: Under `std`, set up more actuators than the default capacity on the
uncapped variant through the mock; without `std`, the variant does not compile.

**Acceptance Scenarios**:

1. **Given** a `std` build, **When** autoscan finds more actuators than the default capacity on
   the uncapped variant, **Then** all of them are held.
2. **Given** a build without `std`, **When** the developer names the uncapped variant, **Then**
   the code does not compile.

---

### Edge Cases

- A Protocol 1 sweep at the default 10 ms wait takes about 2.5 s; the separate scan wait lets
  the user shorten it.
- Two actuators answer the same broadcast ping with the same ID: the bus cannot tell them apart
  (SRS 4.5); a corrupted reply fails setup with a typed protocol error, never a panic.
- An actuator reports a hardware fault during setup: setup fails with that hardware error
  (see Assumptions, SRS conflict 6).
- A model that is not reachable on the bus's protocol (e.g. a Protocol 2-only model on a
  Protocol 1 bus): setup fails, it is not found mid-run.
- A multi-actuator write spans two register layouts and the transport fails between them: the
  first instruction may already be on the wire. Validation is all-or-nothing; the wire is not.
  Documented.
- An empty target list: multi-actuator calls succeed and send nothing.
- A Protocol 2 actuator keeps reporting a fault until it is rebooted: clearing keeps failing
  until the user resolves it outside this feature (reboot is out of scope).
- Retries: a timeout or a corrupted reply on a unicast instruction is retried up to the
  configured count; a reported hardware fault and any broadcast instruction are never retried.
- Limits read at setup may hold only for the operating mode the actuator was in at setup
  (see Assumptions, SRS conflict 8).

## Requirements *(mandatory)*

### Functional Requirements

**Bus and storage**

- **FR-001**: A bus MUST represent one wire with one protocol, one transport and one baud rate,
  with protocol, transport and capacity fixed at compile time (SRS FR-1.1, FR-1.6).
- **FR-002**: The bus MUST own the actuators it holds, up to a capacity `N` (default 32) that is
  an upper bound, not an exact count; exceeding it during setup is an error (FR-1.2).
- **FR-003**: The bus MUST bound every wait for a reply with a monotonic time source the user
  supplies (any function returning microseconds), or the system clock under `std`. The time
  source is part of the bus's compile-time type, as a fourth type parameter after capacity
  with a default (the system clock under `std`), so the std user never names it. This takes
  no space for a clock with no state, needs no indirect call, and accepts a clock that owns a
  hardware timer. SRS FR-1.1, 4.1 and §7 need the extra parameter (deviation).
- **FR-004**: The bus MUST take a reply timeout, a scan wait and a retry count at construction,
  with defaults of 10 ms, 10 ms and 0 retries (FR-1.4). Retries apply to unicast instructions
  that time out or get a corrupted reply only.
- **FR-005**: Creating a bus MUST NOT touch the wire; the transport MUST be recoverable from the
  bus when the user is done with it.
- **FR-006**: Under `std`, the user MUST be able to create a bus on a named serial port and baud
  rate in one step, using the std serial adapter and the system clock.
- **FR-007**: The bus MUST NOT hold shared ownership, interior mutability or stored borrows
  (FR-1.5, constitution I).

**Setup**

- **FR-008**: Declared setup MUST take a list of (ID, model number), ping only those IDs, and
  fail if one is missing or answers with another model. It never sweeps the bus (FR-2.5).
- **FR-009**: Autoscan MUST find every actuator answering at the bus's baud: one broadcast ping
  on Protocol 2, a per-ID ping sweep of 0..=252 on Protocol 1 (FR-2.1, FR-2.2). Autoscan is
  available in `no_std` builds too: broadcast ping, the P1 sweep, the model lookup and the
  fixed-capacity actuator list allocate nothing. SRS 2.1's "only when std is present" is
  dropped (deviation).
- **FR-010**: Setup MUST pick each actuator's model from its model number, and, where the
  model's table depends on firmware, the table for its firmware version (FR-2.3).
- **FR-011**: A model number with no compiled-in model MUST fail setup with "unsupported model"
  naming the model number and the feature that would enable it (FR-2.4).
- **FR-012**: A model that cannot be driven on the bus's protocol MUST fail setup, not a later
  call (constitution II).
- **FR-013**: Setup MUST read each actuator's limits from the actuator itself; a failed limit
  read fails setup, and control-table defaults are never used (FR-2.6).
- **FR-014**: After setup, every actuator MUST start with torque off and no fault in the bus's
  state, and setup MUST send no torque write (FR-2.7).
- **FR-015**: A failed setup MUST leave the bus holding no actuators; a successful one replaces
  whatever it held before.

**Single-actuator access**

- **FR-016**: Access to one actuator MUST go through a short-lived handle obtained by ID; asking
  for an ID the bus does not hold returns "none". Only one handle can be live at a time, and a
  second one is a compile error (FR-1.5, FR-3.1).
- **FR-017**: Model-independent operations (torque on/off, goal position, present position,
  limits, fault state, clear fault) MUST work on any model with no dynamic dispatch (FR-3.2).
- **FR-018**: Model-specific operations MUST be reachable by matching on the actuator's model
  (FR-3.5).
- **FR-019**: Torque MUST only be enabled by an explicit call (FR-3.3).
- **FR-020**: A goal write while torque is off MUST return "actuator is off" and send nothing,
  with no model-specific side effect (FR-3.4).
- **FR-021**: A goal write outside the actuator's limits MUST return a validation error and send
  nothing (FR-3.7).
- **FR-022**: Writes MUST NOT be confirmed implicitly; reading a value back is a separate call
  (FR-3.8).
- **FR-023**: Values MUST cross the API in typed form (position in radians) and in raw register
  form, with the same guard and limit checks on both (FR-3.6).

**Torque and fault state**

- **FR-024**: The bus MUST expose, per actuator, one combined state: torque off, torque on, or
  faulted (faulted always means off). It is stored in one byte per actuator; no fault report
  is kept: the call that saw the fault returns it, and Hardware Error Status can be read
  again. This replaces SRS 3.3's two flags (deviation), for the smallest actuator footprint.
- **FR-025**: Any reply that reports a hardware fault MUST mark that actuator faulted with
  torque off, and the call that received it MUST return a typed hardware error (FR-6.2).
- **FR-026**: Enabling torque on a faulted actuator MUST be refused with a typed error and send
  nothing; disabling torque is always allowed.
- **FR-027**: A torque write that fails for any reason MUST leave the bus's state as torque off.
- **FR-028**: Clearing a fault MUST be an explicit call that asks the actuator whether it still
  reports a fault, and clears the state only if it does not (FR-6.5).
- **FR-029**: The docs MUST state that broadcast writes get no reply, so a write-only loop sees
  faults only on its next read (FR-6.3).

**Multi-actuator operations**

- **FR-030**: Multi-actuator targets MUST be given as any sequence of IDs, ranges and sparse
  lists alike (FR-4.1).
- **FR-031**: Easy-tier writes and reads MUST exist on every protocol, with the wire strategy
  chosen by the protocol at compile time: broadcast writes on both; one sync read on
  Protocol 2, sequential reads on Protocol 1 (FR-4.2).
- **FR-032**: The explicit sync write MUST exist on both protocols; the explicit sync read MUST
  exist on Protocol 2 only, so using it on Protocol 1 is a compile error (FR-4.3).
- **FR-033**: Multi-actuator operations MUST group targets by register layout (address, size)
  and send one instruction per group (FR-4.4).
- **FR-034**: A multi-actuator write MUST be validated as a whole before anything is sent:
  torque on for every target, every value inside limits, one value per target, no unknown or
  repeated ID. Any failure refuses the call and sends nothing to anyone (FR-4.8).
- **FR-035**: Multi-actuator reads MUST fill a caller-owned buffer, one slot per target in
  target order, and MUST NOT allocate. A buffer shorter than the target list is a validation
  error (FR-4.5).
- **FR-036**: A strict read MUST fail the whole call on any failure and leave the caller's
  buffer unchanged; when several targets fail, a hardware fault is reported ahead of a timeout
  (FR-4.6).
- **FR-037**: Faults seen in any multi-actuator reply MUST update that actuator's state as in
  FR-025.

**Opt-in features**

- **FR-038**: Behind `partial-reads` (off by default), a partial read MUST return one result per
  slot; on Protocol 2 a failed sync group fails all its slots (FR-4.7).
- **FR-039**: Behind `raw-registers` (off by default), a developer MUST be able to read or write
  any address and size on one actuator, bypassing the torque guard, the limit check and the
  bus's per-actuator state; the docs MUST say this can desync that state (FR-8.1, FR-8.2).
- **FR-040**: Under `std`, an uncapped storage variant of the bus MUST be available, added as
  new API without changing the capped bus (FR-1.3, FR-4.9).
- **FR-041**: Enabling any feature MUST only add API, never change an existing method's
  behaviour (FR-4.9, constitution VII).

**Build and runtime constraints**

- **FR-042**: The module MUST build without `std` and without a heap on `thumbv7em-none-eabihf`,
  and no path reachable without `std` may allocate (NFR-1).
- **FR-043**: No Cycle operation, in any build, may allocate (NFR-2).
- **FR-044**: No operation may panic on any I/O, protocol or hardware outcome; such outcomes are
  typed errors (constitution VI).

### Key Entities

- **Bus**: one wire. Holds the transport, the time source, the configuration and up to `N`
  actuators, sorted by ID.
- **Configuration**: reply timeout, scan wait, retry count. Fixed at construction.
- **Held actuator**: one actuator the bus owns, of one model, with its ID, torque and fault
  state, and the limits read at setup.
- **Actuator handle**: short-lived access to one held actuator; at most one at a time.
- **Declared list**: (ID, model number) pairs the user expects on the wire.
- **Target list**: the IDs a multi-actuator call addresses, in the user's order.
- **Caller buffer**: user-owned storage a multi-actuator read fills, one slot per target.
- **Group**: the targets of one multi-actuator call that share a register address and size;
  one instruction each.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A desktop developer goes from a connected bus to one moving actuator in at most 5
  lines of code (create, set up, enable torque, write goal, read position).
- **SC-002**: Every refusal case (torque off, faulted, out of limits, unknown or repeated ID,
  short buffer, value count mismatch) sends zero bytes, in 100 % of the tests that inject it.
- **SC-003**: Every hardware fault injected into any reply (setup, single read or write,
  multi-actuator read, clear fault) marks the actuator faulted on that same call, in 100 % of
  the tests.
- **SC-004**: A multi-actuator write to K targets across G register layouts sends exactly G
  instructions; a Protocol 2 multi-read of K targets sends one request, a Protocol 1 one sends K.
- **SC-005**: Every Cycle operation records zero memory allocations, and the `no_std` build on
  `thumbv7em-none-eabihf` passes in CI.
- **SC-006**: Each misuse the design forbids (explicit sync read on Protocol 1, two live handles,
  a handle outliving its bus, a feature-gated method without its feature) fails to compile in a
  dedicated compile-fail test.
- **SC-007**: A Protocol 2 autoscan of an empty bus ends within one scan wait plus 1 ms; a
  Protocol 1 one within 253 scan waits plus 10 ms (test clock).
- **SC-008**: No injected transport, protocol or hardware failure causes a panic or a wait
  longer than (retries + 1) × timeout per unicast instruction, plus 1 ms.
- **SC-009**: Each PR of this feature reaches at least 70 % line coverage without hardware.

## Assumptions

- Defaults taken from the SRS "Meanwhile" answers and the proposal:
  - Model types are public and reached by matching on the stored model (OQ-1).
  - The model-independent operations are torque, goal position, present position and limits
    only (OQ-2); others stay model-specific or wait for the architect.
  - Typed position is radians (OQ-3); raw register values are always reachable.
  - A partial read on Protocol 1 is a per-slot result, since each read is independent (OQ-4).
  - Strict reads write plain values and leave the buffer unchanged on failure (proposal Q3).
  - An actuator that reports a fault during setup fails setup (proposal Q6), consistent with
    FR-2.6 (a failed limit read fails Init) and FR-6.2.
  - Raw access returns a reported fault as a typed error but does not change the bus's
    per-actuator state, as SRS FR-8.1 says (proposal Q5).
  - Reboot is out of scope for this feature (proposal Q7).
  - Torque enable/disable on many actuators is sent as acknowledged unicast writes, not a
    broadcast, so the bus's torque state matches what the actuators confirmed.
- Dependencies, each in another module and merged before the bus PRs that need them:
  - protocol (spec 001, PR #7): exchanges reachable from the bus, a sync-read and a scan hook;
  - core: position type, model-number type, new error variants (capacity, not found, wrong
    model, unreachable, faulted, actuator is off, out of limits, bad buffer);
  - tables and xtask: per-model register layout (torque, goal, present, limits, hardware error,
    position scale, supported protocols), a per-actuator state slot, and the firmware-version
    register, each verified against the e-Manual (constitution IV). Position scale and the
    Protocol 1 model-number and firmware addresses are not in the tables yet.
- Delivery: the proposal splits this module into PRs 1 to 7 (steps 4, 5, 6, 9, 10), each with
  ≥ 70 % coverage. That needs the architect's approval under "one module per PR".
- Latency and jitter targets are deferred to the NFR-5 reference bench; no story needs hardware
  to pass.
- **Deviations / SRS conflicts** (to raise with the architect):
  1. FR-1.1, 4.1, §7: a time-source parameter may change the bus's shape and constructor (Q1).
  2. SRS 2.1 vs FR-2.1: autoscan is the default setup but limited to `std` (Q2).
  3. SRS 3.3 "torque state is a bool" vs a combined torque/fault state (Q4).
  4. FR-4.5 [decision] proposes a buffer of optional values; this spec uses plain values and a
     buffer left unchanged on failure.
  5. FR-6.5: refusing torque enable while faulted, and clearing on Protocol 1 (no hardware
     error register) by checking the error flags of a ping reply, are additions.
  6. FR-2.7 says caches start clear "whatever the hardware reports"; this spec fails setup on a
     reported fault.
  7. FR-8.1 (raw bypasses state) vs FR-6.2 (every fault sets fault state): raw replies return
     the error but do not set the state.
  8. FR-3.7 treats limits as unconditional; on the e-Manual they may apply in position mode
     only. To verify before the limit check ships.
  9. FR-4.4/FR-4.8: all-or-nothing holds for validation, not on the wire across groups.
  10. FR-1.3: the uncapped variant cannot be a plain alias of the capped bus; it needs a storage
      parameter or its own type.
  11. FR-012 (protocol reachability) and the separate scan wait (FR-004) are not in the SRS.
  12. §9 / constitution workflow: four prerequisite PRs in other modules and a seven-PR split
      of this module.
