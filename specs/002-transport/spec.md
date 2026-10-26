# Feature Specification: Transport (std serial adapter and mock)

**Feature Branch**: `002-transport`

**Created**: 2026-10-26

**Status**: Draft

**Input**: User description: "for transport @docs/srs/SRS.md"
→ SRS module `transport` (SRS §9): 4.3 adapters, the Transport row of §7, NFR-10 (mock);
delivery step 8 (std adapter), mock from step 1. No-go: everything outside `src/transport/`.

## User Scenarios & Testing *(mandatory)*

The "users" of this module are robot application developers on a PC or SBC (std adapter),
embedded firmware developers who bring their own UART (the contract their UART must meet),
and anyone testing code that talks to actuators without hardware (mock).

### User Story 1 - Open a serial port and talk to actuators from a desktop (Priority: P1)

A robot application developer names a serial port (e.g. `/dev/ttyUSB0`, `COM3`) and a baud
rate, gets a transport the bus accepts, and the bus can then ping, read and write actuators
through it. They write no adapter code of their own.

**Why this priority**: This is the default path for the largest user class (SRS 1.4) and the
only way to reach real hardware from a desktop. Without it, the `std` build has no out-of-box
transport.

**Independent Test**: Open the adapter on a pseudo-terminal pair (or a loopback serial device),
write a packet on one end and read it on the other; check byte-exact delivery, the "bytes
pending" answer, and the error on a missing port. With hardware (optional, manual): ping one
XM430-W350 through a U2D2 at 1 Mbaud.

**Acceptance Scenarios**:

1. **Given** an existing serial port and a baud rate, **When** the developer opens the adapter,
   **Then** they get a transport configured at that baud, 8 data bits, no parity, 1 stop bit,
   no flow control.
2. **Given** a port name that does not exist or is already in use, **When** the developer
   opens the adapter, **Then** they get a typed error naming the cause, and nothing panics.
3. **Given** an opened adapter, **When** bytes are written and flushed, **Then** the flush
   returns only after every byte has left the host side of the port.
4. **Given** an opened adapter with no input pending, **When** the bus asks whether input is
   pending, **Then** the answer is "no" without blocking.
5. **Given** an opened adapter whose device is unplugged, **When** the bus reads or writes,
   **Then** a typed transport error is returned, not a panic and not a hang.
6. **Given** a port the developer already opened and configured themselves, **When** they hand
   it to the adapter, **Then** it is used as-is with their settings.

---

### User Story 2 - Test my code against a mock transport, no hardware (Priority: P1)

A developer (crate contributor or application developer) scripts what the wire should carry:
the bytes they expect the crate to send and the bytes the "actuator" replies with, including
replies that come late, never come, or are corrupted. They run their code against it and
then check that what was sent matches what was expected.

**Why this priority**: NFR-10 makes the mock a requirement, and every later module (`bus`
especially) needs it for its 70 % coverage gate without hardware.

**Independent Test**: Script an exchange, run a ping through the protocol layer against the
mock, and assert on the recorded bytes and the decoded result.

**Acceptance Scenarios**:

1. **Given** a mock with a scripted reply, **When** an instruction is sent through it, **Then**
   the reply becomes readable only after the instruction was sent, and the sent bytes are
   recorded for inspection.
2. **Given** a mock with bytes already waiting before anything is sent, **When** the next
   exchange runs, **Then** those bytes are readable first (so the bus's "clear before send"
   rule can be tested).
3. **Given** a mock scripted to reply late, **When** the exchange runs with a test clock,
   **Then** the reply becomes readable only once the test clock reaches the scripted time.
4. **Given** a mock scripted to fail with a given I/O error kind or to report end of stream,
   **When** the crate reads or writes, **Then** that exact error reaches the crate.
5. **Given** a mock with expected outgoing bytes, **When** the crate sends something else,
   **Then** the test reports the first differing byte and its offset.
6. **Given** a finished test, **When** scripted replies were never consumed, **Then** the test
   can detect that.

---

### User Story 3 - Bring my own microcontroller UART (Priority: P2)

An embedded firmware developer uses a UART driver from their HAL, with no heap and no `std`,
as the bus transport. The crate states exactly what that UART must offer, and any UART that
offers it works without wrapper code from the crate.

**Why this priority**: The `no_std` user class is a binding constraint (constitution I), but
this story is a contract and its documentation, not new adapter code: HAL crates already
implement the standard blocking I/O traits.

**Independent Test**: A `no_std`, heap-free build on `thumbv7em-none-eabihf` that plugs a
user-supplied type meeting the contract into the bus type compiles; a type missing part of the
contract fails to compile with an error naming the missing capability.

**Acceptance Scenarios**:

1. **Given** a UART type that offers blocking read, blocking write with flush, and a
   non-blocking "input pending" check, **When** it is used as the bus transport in a `no_std`
   build, **Then** it compiles with no adapter.
2. **Given** a UART type that lacks the "input pending" check, **When** it is used as the bus
   transport, **Then** the build fails at compile time, not at runtime.
3. **Given** the crate docs, **When** an embedded developer reads the transport section,
   **Then** it lists every capability the UART must offer, the half-duplex direction-control
   expectation, and that timeouts are the bus's job.

---

### Edge Cases

- A read is asked for while no input is pending: the adapter MUST NOT block past a short,
  bounded interval, so the bus's own timeout stays in charge.
- A read returns zero bytes: treated as end of stream (port gone), which the protocol layer
  already maps to a transport error.
- A write is accepted only in part: the remaining bytes are still delivered, or a typed error
  is returned; bytes are never silently dropped.
- The OS reports a timeout on a read or write: reported as the transport-level timeout kind so
  the protocol layer can map it to its own timeout error.
- USB-serial adapters with a receive latency timer (FTDI default 16 ms, larger than the 10 ms
  default status timeout of FR-1.4): the adapter does not change OS driver settings; the docs
  tell the user how to lower it.
- Adapters or wiring that echo transmitted bytes back on receive (single-wire UART without
  automatic direction control): out of scope; the docs state that the transport must not echo.
- The mock runs out of scripted capacity: the test fails loudly at setup, never inside the
  code under test.
- The adapter is opened at a baud rate the OS or device does not support: a typed error at
  open, not on the first exchange.

## Clarifications

### Session 2026-10-26

- Q: Who can use the mock? → A: Public, behind an opt-in `mock` feature.
- Q: How much should the mock simulate? → A: Scripted bytes only now; a simulated actuator
  later in its own module.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The module MUST state the transport contract as one bound the bus and protocol
  layers use: blocking byte read, blocking byte write with a flush that waits until the bytes
  have left, and a non-blocking "is input pending" check. Timeouts are not part of the
  contract; the bus enforces them (SRS 4.4, §7).
- **FR-002**: Any user type that meets the contract MUST be usable as the bus transport in a
  `no_std`, heap-free build with no wrapper from the crate.
- **FR-003**: Under the `std` feature, the module MUST provide a serial adapter that opens a
  port by name and baud rate with 8N1 framing and no flow control, and returns a transport
  meeting FR-001.
- **FR-004**: The std adapter MUST also accept a port the user already opened, and use its
  settings unchanged.
- **FR-005**: The std adapter's "input pending" check MUST NOT block, and its read MUST return
  within a short bounded interval when nothing arrives, so the bus's timeout is the only wait
  that matters.
- **FR-006**: Every failure of the std adapter (open, read, write, flush, "input pending") MUST
  be returned as a typed error whose kind is one the crate's transport error category can
  carry; a timeout reported by the OS MUST map to the timeout kind.
- **FR-007**: The std adapter MUST NOT be present in builds without the `std` feature, and
  adding it MUST NOT change any existing API (features add API, constitution VII).
- **FR-008**: The module MUST provide a mock transport that replays scripted incoming bytes and
  records outgoing bytes, with: bytes waiting before anything is sent; replies released after
  the next send; replies released after the next send once a test clock reaches a given time;
  injected I/O error kinds; end of stream; and silence (no reply at all).
- **FR-009**: The mock MUST let a test assert the recorded outgoing bytes against expected
  bytes, reporting the first mismatch offset, and check that all scripted replies were
  consumed.
- **FR-010**: The mock MUST NOT allocate, so the zero-allocation tests (constitution I) can use
  it, and MUST build without `std`.
- **FR-011**: The mock MUST be public API behind an opt-in `mock` Cargo feature (off by default),
  so application developers can test their own code without hardware. Enabling the feature
  only adds API (constitution VII). The crate's own tests use the same mock.
- **FR-012**: The mock MUST work at the byte level only: scripted bytes in, recorded bytes
  out, timing and injected failures. It MUST NOT know any Dynamixel protocol. A simulated
  actuator that answers instructions from a register map is out of scope for this feature
  (roadmap, own module).
- **FR-013**: No transport code may use dynamic dispatch, panic on any I/O outcome, or retry on
  its own; retries belong to the bus (FR-1.4, constitution III and VI).
- **FR-014**: Public transport items MUST be documented, including the half-duplex direction
  control expectation (transport handles it, or the hardware does), the no-echo expectation,
  and the USB latency-timer note.

### Key Entities

- **Transport contract**: what any byte channel must offer to be a bus transport: blocking
  read, blocking write plus flush, non-blocking "input pending". No timeout, no protocol
  knowledge.
- **Std serial adapter**: the `std`-only transport over an OS serial port; created from a port
  name + baud rate or from an already-open port.
- **Mock transport**: public behind the `mock` feature; a scripted, allocation-free in-memory line: incoming script (bytes plus
  when they become readable), outgoing record, injected failures.
- **Test clock**: a manually advanced time source the mock and the bus share, so timeouts are
  tested without real waiting (already exists as the protocol layer's `Clock`).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A desktop developer goes from an installed crate to a successful ping of one
  real actuator with at most 3 lines of transport setup code.
- **SC-002**: The `no_std` CI build on `thumbv7em-none-eabihf` passes with the module
  compiled in, and every mock-based test records zero allocations.
- **SC-003**: Every scripted mock behaviour of FR-008 is exercised by at least one test, and a
  wrong outgoing byte is reported with its offset 100 % of the time.
- **SC-004**: With nothing arriving, the std adapter's "input pending" check returns in under
  1 ms and a read returns within 2 ms, measured on the NFR-5 reference host.
- **SC-005**: No transport failure (missing port, unplugged device, OS timeout, partial write)
  causes a panic or a wait longer than the bus timeout plus 2 ms, across a test that injects
  each failure.
- **SC-006**: The module reaches at least 70 % line coverage without real hardware.

## Assumptions

- The transport contract is `Read + Write + ReadReady`, and the bus bounds waits with its own
  `Clock` (SRS 3.3, 4.4, §7 and FR-1.1; constitution 1.1.0 Technical Constraints; amended by the
  architect 2026-10-26). FR-001 follows them.
- The `Clock` stays in the protocol module; this module does not move or redefine it.
- The protocol module's private test fake (`src/protocol/fake.rs`) is not replaced in this
  feature: that file is outside `src/transport/` (no-go). Migrating protocol tests to the mock
  is a follow-up the architect schedules.
- The adapter assumes hardware direction control (U2D2, RS-485 transceivers with auto
  direction). Manual DE/RE pin toggling on microcontrollers is the user's UART's job.
- A simulated actuator (register map answering ping/read/write) is a later, separate module.
- Baud-rate scanning, port discovery/listing and changing OS driver settings (latency timer,
  low-latency flags) are out of scope (SRS 2.2).
- Async I/O is out of scope (SRS 2.2).
- Wiring the std adapter as the default type parameter of `Bus` belongs to the `bus` module;
  this module only provides the adapter.
- The example binary of delivery step 8 belongs to whoever owns `examples/`; it is not part of
  this module's no-go-bounded scope.
