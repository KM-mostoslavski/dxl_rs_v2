# Feature Specification: Protocol Packets (P1 and P2)

**Feature Branch**: `001-protocol-packets`

**Created**: 2026-10-26

**Status**: Draft

**Input**: User description: "you can choose the module. ideally ones that is the backbone"
→ SRS module `protocol` (`docs/srs/modules/protocol.md`): FR-5.1 to FR-5.5, FR-6.1, NFR-10;
delivery steps 1 and 2.

## User Scenarios & Testing *(mandatory)*

The "users" of this module are the crate's own higher layers (Setup, `ActuatorRef`,
multi-actuator ops) and, through them, robot application and embedded firmware developers.
Every byte the crate puts on or takes off the wire passes through this module.

### User Story 1 - Talk to a Protocol 2 actuator (Priority: P1)

A developer's code asks for one instruction to be sent to one Protocol 2 actuator (ping, read
a register, write a register) and gets back either the decoded reply or a typed error. The
packet on the wire matches the ROBOTIS e-Manual byte for byte.

**Why this priority**: Protocol 2 is the protocol of every current Dynamixel family (X, Y,
PRO, MX 2.0). Without it, no other module can reach an actuator.

**Independent Test**: Feed the e-Manual Protocol 2 packet examples through the encoder and
decoder with a scripted fake byte stream; no hardware needed.

**Acceptance Scenarios**:

1. **Given** an e-Manual Protocol 2 instruction example, **When** the same instruction is
   built, **Then** the produced bytes equal the example exactly, including the CRC.
2. **Given** an e-Manual Protocol 2 status packet example, **When** it is parsed, **Then** the
   decoded ID, error byte and parameters equal the example.
3. **Given** a parameter sequence that contains the header pattern, **When** the packet is
   built and later parsed, **Then** byte stuffing is added on send and removed on receive, and
   the round trip returns the original parameters.
4. **Given** a status packet with one corrupted byte, **When** it is parsed, **Then** a typed
   protocol error is returned and no data is handed to the caller.

---

### User Story 2 - Talk to a Protocol 1 actuator (Priority: P2)

The same as User Story 1 for Protocol 1 actuators (AX, MX on 1.0 firmware), with the Protocol 1
packet layout and checksum.

**Why this priority**: Needed for AX and older MX hardware, but those families come after the
X-series in the delivery order.

**Independent Test**: Same shape as User Story 1, against the e-Manual Protocol 1 examples.

**Acceptance Scenarios**:

1. **Given** an e-Manual Protocol 1 instruction example, **When** the same instruction is
   built, **Then** the produced bytes equal the example exactly, including the checksum.
2. **Given** an e-Manual Protocol 1 status packet example, **When** it is parsed, **Then** the
   decoded ID, error byte and parameters equal the example.
3. **Given** a status packet with a wrong checksum, **When** it is parsed, **Then** a typed
   protocol error is returned.

---

### User Story 3 - See every hardware fault the actuator reports (Priority: P2)

Every status packet carries an error byte. A developer gets a typed error naming what the
actuator reported (P2: the alert bit and the error code; P1: each set error flag) on the very
call that received it.

**Why this priority**: Fault detection is a safety guarantee (constitution Principle III). It
costs nothing extra because the byte is already parsed.

**Independent Test**: Parse status packets whose error byte has each defined value or flag set
and check the returned error.

**Acceptance Scenarios**:

1. **Given** a P2 status packet with the alert bit set, **When** it is parsed, **Then** the
   result reports a hardware fault for that ID.
2. **Given** a P2 status packet with a non-zero error code and no alert bit, **When** it is
   parsed, **Then** the result is a typed status error naming that code.
3. **Given** a P1 status packet with several error flags set, **When** it is parsed, **Then**
   every set flag is reported, not only the first.
4. **Given** a status packet with a zero error byte, **When** it is parsed, **Then** no error is
   reported.

---

### User Story 4 - Predictable exchanges on a shared wire (Priority: P3)

When the crate sends an instruction, stale bytes from earlier traffic never get mistaken for
the reply; a unicast instruction waits for its reply but never forever; a broadcast instruction
never waits.

**Why this priority**: Makes the higher layers correct on a real, noisy half-duplex bus. Builds
on Stories 1 and 2.

**Independent Test**: Use a scripted fake transport that holds leftover bytes, replies late, or
never replies.

**Acceptance Scenarios**:

1. **Given** leftover bytes waiting on the receive side, **When** an instruction is sent,
   **Then** those bytes are discarded before sending and never parsed as the reply.
2. **Given** a unicast instruction and an actuator that never answers, **When** the exchange
   runs, **Then** it ends with a timeout error once the caller-supplied time limit passes.
3. **Given** a broadcast instruction (sync write, broadcast ping excluded), **When** it is
   sent, **Then** the exchange returns as soon as the bytes are written, without reading.
4. **Given** a P2 broadcast ping, **When** it is sent, **Then** all replies that arrive within
   the time limit are collected, each with its ID and model number.

---

### Edge Cases

- Garbage bytes before a valid header: skipped until a header is found; the packet still parses.
- Reply from a different ID than the one addressed: rejected as a protocol error.
- Reply whose length field promises more bytes than arrive before the time limit: timeout error.
- Reply whose declared length exceeds the largest packet this build can hold: protocol error,
  no out-of-bounds access, no panic.
- Instruction whose parameters would exceed the largest packet this build can hold: validation
  error before anything is sent.
- Sync read or broadcast ping requested on Protocol 1: not possible to express; it fails to
  compile.
- Transport reports an I/O error mid-exchange: typed transport error; the partial packet is
  discarded.
- Sync read on P2 where one actuator in the group does not answer: the exchange reports which ID
  timed out; how the bus turns that into strict or partial results belongs to the `bus` module.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The module MUST build Protocol 2 instruction packets (header, ID, length,
  instruction, parameters, byte stuffing, CRC) exactly as specified in the ROBOTIS e-Manual.
- **FR-002**: The module MUST parse Protocol 2 status packets, verify the CRC, remove byte
  stuffing, and return the ID, error byte and parameters.
- **FR-003**: The module MUST build and parse Protocol 1 packets (header, ID, length,
  instruction, parameters, checksum) exactly as specified in the ROBOTIS e-Manual.
- **FR-004**: The supported instructions are: ping, read, write and sync write on both
  protocols; sync read and broadcast ping on Protocol 2 only. An instruction a protocol lacks
  MUST be absent for that protocol, so its use is a compile error.
- **FR-005**: The set of protocols MUST be closed: code outside the crate cannot add one.
- **FR-006**: The receive side MUST be cleared before every instruction is sent.
- **FR-007**: A unicast exchange MUST wait for its status packet for at most the time limit
  given by its caller, then fail with a timeout error.
- **FR-008**: A broadcast instruction other than broadcast ping MUST NOT wait for or read any
  reply.
- **FR-009**: Every parsed status packet's error byte MUST be checked: on P2, the alert bit and
  the error code; on P1, each error flag. A set fault MUST be reported as a typed error that
  names the actuator ID and what it reported.
- **FR-010**: Every failure MUST be returned as a typed error in one of the categories
  transport, protocol (malformed, CRC/checksum mismatch, wrong ID, oversize), status (fault or
  error reported by the actuator) or validation (rejected before sending).
- **FR-011**: The module MUST NOT allocate memory and MUST NOT panic on any input, however
  malformed.
- **FR-012**: The module MUST NOT read, track or change the actuators' Status Return Level; it
  assumes every unicast instruction gets a reply.
- **FR-013**: Encoding and decoding MUST be testable without hardware, against the e-Manual
  packet examples, through a scripted fake byte stream.
- **FR-014**: Every packet layout, instruction code, CRC/checksum rule and error-byte meaning
  MUST be verified online against the ROBOTIS e-Manual protocol pages, and the tests MUST cite
  the page each example comes from.

### Key Entities

- **Protocol**: P1 or P2. Fixes packet layout, integrity check (checksum or CRC), the
  instruction set and the error-byte meaning. Chosen at compile time, one per bus.
- **Instruction packet**: what is sent: target (one `Id` or broadcast), instruction, parameters.
- **Status packet**: what an actuator replies: its `Id`, error byte, parameters.
- **Exchange**: one instruction sent plus the replies it expects (none, one, or one per target),
  bounded by a time limit.
- **Status error**: what an actuator's error byte reports, decoded per protocol.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100 % of the packet examples on the e-Manual Protocol 1 and Protocol 2 pages
  are reproduced byte for byte on encode and decoded field for field on parse.
- **SC-002**: 100 % of single-byte corruptions of those example status packets are rejected
  with a typed error (none is accepted as valid data).
- **SC-003**: At least 1,000,000 random byte streams fed to the parser produce no crash and no
  unbounded wait.
- **SC-004**: The module builds for a microcontroller target with no heap, and performs zero
  memory allocations in every test.
- **SC-005**: Protocol 1 offers no way to build a Protocol 2-only instruction: no such
  function exists for it. (The user-facing compile-fail test lands with the `bus` module, the
  first public surface that exposes sync read.)
- **SC-006**: The module reaches at least 70 % line coverage.

## Assumptions

- Actuators keep the factory Status Return Level 2 (reply to every unicast instruction).
- Bulk read and bulk write, reboot, factory reset, reg write/action and clear are out of scope
  for this feature (bulk is on the roadmap; the others touch EEPROM or are not needed yet).
- The time limit and retry count belong to the bus; this module only honours the limit it is
  given and never retries on its own.
- Grouping multi-actuator operations by `(addr, size)` belongs to the `bus` module; this module
  sends one instruction per call.
- Transport, protocol and status error categories do not exist yet in `src/error.rs` (core
  module). This feature adds them there; that cross-module change is called out in the PR.
- Until the `transport` module ships its mock, tests use a scripted fake byte stream local to
  the protocol tests.
- The largest packet a build can hold is a compile-time bound sized for the largest sync
  instruction a bus of capacity `N` can send; the exact bound is chosen during planning.
