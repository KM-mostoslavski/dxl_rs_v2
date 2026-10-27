---

description: "Task list for the protocol module (P1 and P2 packets)"
---

# Tasks: Protocol Packets (P1 and P2)

**Input**: Design documents from `specs/001-protocol-packets/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/protocol-api.md, quickstart.md

**Tests**: Requested by the spec (FR-013, SC-001..SC-006). Test tasks come first in each story and
must fail before the implementation task that follows them.

**Organization**: Tasks are grouped by user story so each story can be implemented and tested on
its own.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: US1 = P2 packets, US2 = P1 packets, US3 = fault reporting, US4 = exchange rules

## Rules for every task

- Rust 2018+ layout: `src/protocol.rs` + `src/protocol/*.rs`, never `mod.rs`.
- Document every public item; no comments inside method bodies unless resolving a real ambiguity.
- Every error is a `thiserror` enum/struct (no_std). No `alloc`/`std` in `src/protocol/**` except
  `StdClock` behind `#[cfg(feature = "std")]`. No `panic!`, `unwrap`, indexing that can go out of
  bounds, or `unimplemented!` on any path reachable from received bytes.
- Protocol facts come only from the vendored e-Manual vectors and `research.md`; never from memory.
- Commits carry no co-authorship or AI attribution lines.

---

## Phase 1: Setup

**Purpose**: Dependencies and module skeleton

- [X] T001 Add `embedded-io = { version = "0.7", default-features = false }` to `[dependencies]`, make the `std` feature also enable `"embedded-io/std"`, and add `proptest = "1"` to `[dev-dependencies]` in Cargo.toml
- [X] T002 Create the module skeleton: `src/protocol.rs` (module doc citing the e-Manual P1/P2 URLs, `mod clock; mod crc; mod p1; mod p2;` plus `#[cfg(test)] mod fake; #[cfg(test)] mod vectors; #[cfg(test)] mod no_alloc;`) and empty files `src/protocol/{clock,crc,p1,p2,fake,vectors,no_alloc}.rs`; add `mod protocol;` to src/lib.rs
- [X] T003 Verify the skeleton builds: `cargo build`, `cargo clippy --all-targets -- -D warnings`, and the no_std clippy command from CLAUDE.md all pass (fix lint fallout only in files from T001–T002)

---

## Phase 2: Foundational (blocking prerequisites)

**Purpose**: Errors, clock, protocol trait, test doubles and vectors that every story uses

**⚠️ CRITICAL**: No user-story work starts until this phase is complete

- [X] T004 Extend src/error.rs per data-model.md "Errors": add `Error::Transport(TransportError)`, `Error::Protocol(ProtocolError)`, `Error::Status(StatusError)` (each `#[error(transparent)]` + `#[from]`); `TransportError { Io(embedded_io::ErrorKind), Timeout { id: Option<Id> } }`; `ProtocolError { Checksum { id: u8 }, Crc { id: u8 }, WrongId { expected: Id, got: u8 }, WrongLength { expected: usize, got: usize }, Malformed }`; `ValidationError` gains `AddressOutOfRange { addr: u16 }` and `PacketTooLarge { len: usize, max: usize }`. All `Debug, Clone, Copy, PartialEq, Eq, thiserror::Error`, `#[non_exhaustive]`; Display strings name the ID where one exists. Unit tests for each Display string in the same file
- [X] T005 Add the status error types to src/error.rs: `StatusError { id: Id, report: StatusReport }`, `StatusReport { P1(P1Flags), P2 { alert: bool, code: Option<P2Code> } }`, `P2Code { ResultFail = 1, Instruction = 2, Crc = 3, DataRange = 4, DataLength = 5, DataLimit = 6, Access = 7, Unknown(u8) }` with `P2Code::from_bits(u8 & 0x7F) -> Option<P2Code>` (0 → None); `P1Flags(u8)` with named consts for bits 0..6 = InputVoltage, AngleLimit, Overheating, Range, Checksum, Overload, Instruction, `iter()` over every set flag, and `StatusError::is_hardware_fault()` (P2: `alert`; P1: Overload, Overheating or InputVoltage set). Display lists every reported condition. Unit tests in the same file
- [X] T006 Re-export the new error types from src/lib.rs (`TransportError, ProtocolError, StatusError, StatusReport, P1Flags, P2Code`)
- [X] T007 [P] Implement `pub trait Clock { fn now_us(&mut self) -> u64; }`, the blanket `impl<F: FnMut() -> u64> Clock for F`, and `#[cfg(feature = "std")] pub struct StdClock` (`Default`, `Instant`-based origin) in src/protocol/clock.rs, with doc examples for the closure and std forms; re-export `Clock` and `StdClock` from src/lib.rs
- [X] T008 [P] Implement the CRC-16 in src/protocol/crc.rs: a `const fn` building the 256-entry table for poly 0x8005, init 0, not reflected, no final XOR, and `pub(crate) fn update(crc: u16, bytes: &[u8]) -> u16` using `i = ((crc >> 8) ^ b) & 0xFF; crc = (crc << 8) ^ TABLE[i]`. Tests: check value of `b"123456789"` is `0xFEE8`; e-Manual ping `FF FF FD 00 01 03 00 01` gives `0x4E19`
- [X] T009 [P] Vendor every e-Manual example from research.md into src/protocol/vectors.rs as `pub(crate) const` byte arrays, grouped P2 then P1, each with a doc comment giving the page URL, anchor and "fetched 2026-10-26", and the decoded meaning (ID, instruction, params or status fields). Include out-of-scope examples (Reg Write, Action, Reboot, Factory Reset, Clear, Backup, Bulk, Fast) marked "integrity vector only". Add a test that recomputes the CRC (P2) / checksum (P1) of every vector and the length field consistency
- [X] T010 [P] Implement the scripted test doubles in src/protocol/fake.rs: `FakeLine` implementing `embedded_io::{ErrorType, Read, Write, ReadReady}` with a queue of scripted RX chunks (each optionally gated to appear only after the clock passes a time), a record of every TX byte, an injectable I/O error, and a counter of `read` calls; `FakeClock` implementing `Clock` that advances by a fixed step on every `now_us()` call
- [X] T011 Define in src/protocol.rs: `mod sealed { pub trait Sealed {} }`; `pub trait Protocol: sealed::Sealed { const BROADCAST: u8 = 0xFE; }`; zero-sized `pub struct P1;` and `pub struct P2;` (in p1.rs / p2.rs, re-exported); `pub(crate) struct Link<'a, T, C> { transport, clock, timeout: core::time::Duration }`; `pub(crate) struct Pong { id: Id, model_number: Option<u16>, firmware: Option<u8> }`; and the `pub(crate) trait Exchange: Protocol` with `ping`, `read`, `write`, `sync_write` exactly as in contracts/protocol-api.md. Re-export `Protocol, P1, P2` from src/lib.rs
- [X] T012 Implement the shared transport helpers in src/protocol.rs: `drain(link)` (while `read_ready()?` read into a 32-byte stack chunk and discard), `send(link, bytes)` (`write_all` + `flush`), and `receive(link, parser, expected_id)` that sets `deadline = clock.now_us() + timeout`, polls `read_ready()`, reads into a 32-byte stack chunk, feeds bytes to a `Parser` trait object-free generic, returns on a complete packet or `TransportError::Timeout { id }` when the deadline passes; `ErrorKind::TimedOut` from `read` maps to `Timeout`, any other kind to `TransportError::Io(kind)`. Unit tests with `FakeLine`/`FakeClock`: timeout, I/O error mapping, TimedOut mapping

**Checkpoint**: Foundation ready; all story phases can start

---

## Phase 3: User Story 1 - Talk to a Protocol 2 actuator (Priority: P1) 🎯 MVP

**Goal**: Byte-exact P2 ping, read and write over a scripted line.

**Independent Test**: `cargo test protocol::p2` — every e-Manual P2 vector in scope encodes and
decodes exactly; stuffing round-trips; a corrupted reply is a typed error.

### Tests for User Story 1 ⚠️ (write first, must fail)

- [X] T013 [P] [US1] Encode tests in src/protocol/p2.rs `mod tests`: ping ID 1, read Present Position (addr 132, 4 B) ID 1, write Goal Position (addr 116) = 512 ID 1 each equal their vector from src/protocol/vectors.rs byte for byte
- [X] T014 [P] [US1] Decode tests in src/protocol/p2.rs `mod tests`: the ping reply decodes to model 1030, firmware 38; the read reply decodes to bytes `A6 00 00 00` (166); the no-error write reply decodes to ID 1, error 0, no params
- [X] T015 [P] [US1] Stuffing tests in src/protocol/p2.rs `mod tests`: a write whose params contain `FF FF FD` gets `FD` inserted after the pattern, the length field counts the inserted byte, the CRC covers the stuffed bytes, and the status parser removes it again (round trip returns the original params); a proptest round trip (default 256 cases) over random params up to 64 bytes
- [X] T016 [P] [US1] Corruption test in src/protocol/p2.rs `mod tests`: for every in-scope P2 status vector and every byte position from ID to the CRC, flipping one byte yields `Err` (never `Ok`), with `ProtocolError::Crc` whenever the length field is still plausible

### Implementation for User Story 1

- [X] T017 [US1] Implement the P2 encoder in src/protocol/p2.rs: header `FF FF FD 00`, ID, 2-byte LE length = params + 3 counting stuffed bytes (one counting pass first), instruction, params with stuffing applied to instruction + params (research.md R-5; doc comment notes the e-Manual is silent on scope), CRC-16 LE over all stuffed bytes from the header; streamed to `send` through a 32-byte stack chunk; `Validation(PacketTooLarge { len, max })` when the length would exceed `u16::MAX`
- [X] T018 [US1] Implement the P2 status parser in src/protocol/p2.rs as a byte-at-a-time state machine (data-model.md "Parser states"): hunt `FF FF FD 00`, read ID, length, instruction; skip and resync if the instruction is not 0x55 (echo); read error byte; copy unstuffed params into the caller's `&mut [u8]`; check CRC over the raw bytes; `ProtocolError::Crc { id }` on mismatch; `ProtocolError::Malformed` if length < 4; `ProtocolError::WrongLength` if the param count differs from the caller slice length
- [X] T019 [US1] Implement `Exchange for P2` `ping`, `read` (params `addr_L addr_H len_L len_H`, reply into `out`) and `write` (params `addr_L addr_H data..`) in src/protocol/p2.rs using `drain` → `send` → `receive`; unicast replies with a different ID return `ProtocolError::WrongId`; `write` to the broadcast ID is not offered (unicast only, `Id` ≤ 252); a non-zero error byte returns `Error::Status` built with `P2Code::from_bits` and the alert bit
- [X] T020 [US1] Exchange tests in src/protocol/p2.rs `mod tests`: `ping`, `read`, `write` against `FakeLine` scripted with the e-Manual replies return the decoded values and record exactly the vector bytes as TX

**Checkpoint**: US1 complete and testable on its own (MVP)

---

## Phase 4: User Story 2 - Talk to a Protocol 1 actuator (Priority: P2)

**Goal**: Byte-exact P1 ping, read, write and sync write.

**Independent Test**: `cargo test protocol::p1`.

### Tests for User Story 2 ⚠️

- [X] T021 [P] [US2] Encode tests in src/protocol/p1.rs `mod tests`: ping ID 1 `FF FF 01 02 01 FB`, read Present Temperature (addr 43, 1 B) ID 1 `FF FF 01 04 02 2B 01 CC`, the checksum worked example `FF FF 01 05 03 0C 64 AA DC`, and the MX-64 sync write vector equal their bytes in src/protocol/vectors.rs
- [X] T022 [P] [US2] Decode tests in src/protocol/p1.rs `mod tests`: ping reply `FF FF 01 02 00 FC` → ID 1, no error, `Pong { model_number: None, firmware: None }`; read reply `FF FF 01 03 00 20 DB` → `[0x20]`
- [X] T023 [P] [US2] Corruption test in src/protocol/p1.rs `mod tests`: every single-byte flip of each P1 status vector yields `Err`, with `ProtocolError::Checksum` whenever the length is plausible

### Implementation for User Story 2

- [X] T024 [US2] Implement the P1 encoder in src/protocol/p1.rs: `FF FF`, ID, LEN = params + 2, instruction, params, checksum `!(ID + LEN + INST + params) as u8` (wrapping sum); `Validation(AddressOutOfRange { addr })` when `addr > 0xFF`; `Validation(PacketTooLarge)` when LEN would exceed 255
- [X] T025 [US2] Implement the P1 status parser in src/protocol/p1.rs: hunt `FF FF`, ID, LEN (≥ 2 else `Malformed`), error byte, params into the caller slice, checksum `!(ID + LEN + ERR + params)`; `ProtocolError::Checksum { id }` on mismatch; `WrongLength` when the param count differs from the slice
- [X] T026 [US2] Implement `Exchange for P1` `ping` (returns `Pong` with `None` model/firmware, research.md R-11), `read` (1-byte addr, 1-byte len), `write`, and `sync_write` (ID 0xFE, params `addr, L, then [ID, L bytes]` per entry, no read) in src/protocol/p1.rs; a non-zero error byte returns `Error::Status` with `StatusReport::P1(P1Flags(byte))`
- [X] T027 [US2] Exchange tests in src/protocol/p1.rs `mod tests`: ping/read/write against `FakeLine` with the e-Manual replies; sync write TX equals the MX-64 vector and `FakeLine` records zero `read` calls

**Checkpoint**: US1 and US2 both work independently

---

## Phase 5: User Story 3 - See every hardware fault the actuator reports (Priority: P2)

**Goal**: Every status error byte becomes a precise typed error on the call that received it.

**Independent Test**: `cargo test protocol::status` plus the error.rs unit tests.

### Tests for User Story 3 ⚠️

- [X] T028 [P] [US3] Status tests in src/protocol.rs `mod tests` (module `status`): P2 reply with error byte `0x80` → `Status` with `alert: true`, `is_hardware_fault() == true`; each code `0x01..=0x07` without alert → matching `P2Code`, not a hardware fault; `0x85` → alert + `DataLength`; unknown code `0x09` → `P2Code::Unknown(9)`; error byte 0 → `Ok`
- [X] T029 [P] [US3] P1 status tests in src/protocol.rs `mod tests` (module `status`): the e-Manual packet `FF FF 01 02 24 D8` → `P1Flags` reporting exactly Overload and Overheating via `iter()`, `is_hardware_fault() == true`; a byte with only Range set is not a hardware fault; Display of a multi-flag error names every flag and the ID

### Implementation for User Story 3

- [X] T030 [US3] Route status decoding through one helper per protocol (`p2::status_error(id, byte)`, `p1::status_error(id, byte)`) in src/protocol/p2.rs and src/protocol/p1.rs and use it on every receive path (ping, read, write, and sync read once US4 lands), so no path returns `Ok` with a non-zero error byte
- [X] T031 [US3] Document on `StatusError::is_hardware_fault` in src/error.rs which bits count as hardware faults per protocol and why (P2 alert = hardware issue per e-Manual; P1 overload/overheating/input voltage), citing the e-Manual anchors

**Checkpoint**: Fault reporting complete for every implemented instruction

---

## Phase 6: User Story 4 - Predictable exchanges on a shared wire (Priority: P3)

**Goal**: Drain-before-send, bounded waits, broadcast writes that never wait, P2 sync read and
broadcast ping.

**Independent Test**: `cargo test protocol::tests::exchange`.

### Tests for User Story 4 ⚠️

- [X] T032 [P] [US4] Drain test in src/protocol.rs `mod tests` (module `exchange`): `FakeLine` pre-loaded with stale bytes containing a complete valid status packet for the target ID; `read` must ignore it, send, and return the scripted fresh reply
- [X] T033 [P] [US4] Timeout tests in src/protocol.rs `mod tests` (module `exchange`): silent actuator → `TransportError::Timeout { id: Some(id) }` once `FakeClock` passes the deadline; a reply scripted to arrive just before the deadline succeeds; one arriving just after times out
- [X] T034 [P] [US4] Sync tests in src/protocol/p2.rs `mod tests`: P2 `sync_write` TX equals the e-Manual Sync Write Goal Position vector and makes zero `read` calls; P2 `sync_read` IDs 1, 2 TX equals the Sync Read vector and `on_reply` receives `(1, Ok([A6 00 00 00]))` then `(2, Ok([1F 08 00 00]))`; with ID 2 silent, `on_reply` receives `(2, Err(Timeout { id: Some(2) }))`
- [X] T035 [P] [US4] Broadcast ping test in src/protocol/p2.rs `mod tests`: TX equals `FF FF FD 00 FE 03 00 01 31 42`; both e-Manual replies are delivered to `on_reply` with model 1030 and firmware 38; with no replies the call returns `Ok(())` after the window

### Implementation for User Story 4

- [X] T036 [US4] Implement `Exchange for P2::sync_write` in src/protocol/p2.rs: ID 0xFE, params `addr_L addr_H len_L len_H` then `[ID, data..]` per entry (two passes over the `Clone` iterator: count/stuff, then emit), `drain` → `send`, never `receive`
- [X] T037 [US4] Implement `impl P2 { pub(crate) fn sync_read(..) }` in src/protocol/p2.rs per contracts/protocol-api.md G4: ID 0xFE, params `addr_L addr_H len_L len_H ID1..IDn`; then for each ID in order a fresh deadline and `receive` into a `size`-byte stack-free view of the caller's chunk; call `on_reply` once per ID; return `Err` only on `TransportError::Io`
- [X] T038 [US4] Implement `impl P2 { pub(crate) fn broadcast_ping(..) }` in src/protocol/p2.rs per G5: send ping to 0xFE, then collect every valid status packet until one `timeout` window elapses, calling `on_reply(Pong)` per reply
- [X] T039 [US4] Confirm `P1` has no `sync_read` and no `broadcast_ping` (no trait item, no inherent fn) and add a doc comment on `P1` in src/protocol/p1.rs stating they are absent by design (SC-005; compile-fail test lands with the `bus` feature)

**Checkpoint**: All four stories complete

---

## Phase 7: Polish & cross-cutting

- [X] T040 [P] Robustness test `robustness_1m` in src/protocol.rs `mod tests`: seeded xorshift64 (seed from `DXL_FUZZ_SEED` env var when set under std tests, printed on failure), 1,000,000 inputs mixing pure random streams (0..64 bytes) and mutated in-scope vectors (byte flips, truncation, spliced second header, length field larger than the caller slice), fed to both parsers via `FakeLine`; asserts no panic and that every call returns within the scripted deadline
- [X] T041 [P] Zero-allocation test in src/protocol/no_alloc.rs (`#[cfg(test)] mod no_alloc;` in src/protocol.rs): a `#[cfg(test)] #[global_allocator]` counting allocator wrapping `std::alloc::System` with a `thread_local!` `const`-initialised counter (other test threads do not interfere); runs ping, read, write, sync write, sync read and broadcast ping through `FakeLine`/`FakeClock` and asserts the counter delta is 0. `Link`/`Exchange` stay `pub(crate)`; no test-only feature or public hook is added
- [X] T042 [P] ~~Add a CI step to .github/workflows/ci.yml that fails if `src/protocol/` (excluding the `StdClock` block) mentions `alloc::` or `std::`~~ Not added: a grep cannot tell `#[cfg(test)]` code (which may use `std`) from library code. The existing `no-std` CI job already compiles `src/protocol/**` without `std` or `alloc`, so any such use fails CI; `src/protocol/no_alloc.rs` covers runtime allocation
- [X] T043 Update the protocol components in docs/workspace.dsl (component descriptions: ReadReady + Clock bound, sealed P1/P2) so they match this implementation; leave the `ov:includes` markers untouched
- [X] T044 Run the full gate from specs/001-protocol-packets/quickstart.md section 7 (fmt, clippy -D warnings, no_std clippy, workspace tests, xtask diff, llvm-cov ≥ 70 %) and fix any failure in this feature's files.
- [X] T045 Fill the PR description from .github/pull_request_template.md: list the two plan deviations (ReadReady + Clock bound; cross-module edits to src/error.rs, src/lib.rs, Cargo.toml) under Architecture, and note that SRS §7 and the constitution's Technical Constraints wording need the architect's update

---

## Phase 8: Review follow-ups (2026-10-27)

From [review-2026-10-27.md](review-2026-10-27.md), after the transport module (spec 002) merged.

- [X] T046 Replace the crate-private `Transport` trait with `crate::Transport`, and the test doubles `FakeLine`/`FakeClock` (src/protocol/fake.rs, deleted) with `MockTransport`/`MockTime::ticker` from src/transport/mock.rs
- [X] T047 Make `Exchange` a sealed supertrait of `Protocol` (`pub` in the private `protocol` module, with `Link` and `Pong`); test that a function bounded only by `P: Protocol` pings over P1 and P2
- [X] T048 In `check`, let a set error byte win over a wrong length; keep an error reply's data in `out` when its length matches, zero `out` otherwise; tests for the P2 alert read of Hardware Error Status and a data-less error reply
- [X] T049 Reject a P2 status LEN longer than the expected reply with worst-case stuffing as `Malformed` and resync; sync read test where a flipped LEN_H no longer swallows the next reply
- [X] T050 In `P2::sync_read`, discard a late reply from an ID that already timed out instead of handing it to the next ID as `WrongId`; test with both replies late
- [X] T051 Add `Pong::fault` and fill it from the error byte of broadcast ping replies; test
- [X] T052 `P1Flags` Display names error bits outside the seven known flags instead of "no error"; test
- [X] T053 Document every public error field and enable `#![warn(missing_docs)]` in src/lib.rs

Not applied: the P1 status parser keeps trusting LEN (one byte, at most 253 bytes swallowed, and P1 has
no multi-reply instruction); sync read still drops the data of an error reply (the callback type is the
architect's call, review "status-error-discards-valid-data"); where the bus keeps its clock is decided in
spec 003.

## Dependencies & Execution Order

### Phase dependencies

- Setup (T001–T003) → Foundational (T004–T012) → US1..US4 → Polish.
- US1 (P2) and US2 (P1) are independent of each other after Foundational.
- US3 depends on the receive paths of US1 and/or US2 (it hardens them); its tests for a protocol
  can run as soon as that protocol's story is done.
- US4: T033/T032 need only Foundational + one `Exchange` impl; T034–T038 need US1 (P2 encoder and
  parser).

### Within Foundational

- T004 → T005 → T006 (same files). T007, T008, T009, T010 in parallel. T011 after T004–T006.
  T012 after T010 and T011.

### Parallel opportunities

- Foundational: T007, T008, T009, T010 together.
- US1 tests T013–T016 together; US2 tests T021–T023 together.
- Once Foundational is done, US1 and US2 can be built by two people in parallel (p2.rs vs p1.rs).
- US4 tests T032–T035 together. Polish T040–T042 together.

## Parallel Example: User Story 1

```text
T013 encode tests  |  T014 decode tests  |  T015 stuffing tests  |  T016 corruption test
→ T017 encoder → T018 parser → T019 Exchange impl → T020 exchange tests
```

## Implementation Strategy

### MVP first (User Story 1)

1. Setup + Foundational.
2. US1: P2 ping/read/write byte-exact over a scripted line.
3. Stop and validate with `cargo test protocol::p2`; this already unblocks the `bus` module's
   X-series path.

### Incremental delivery

1. US1 (P2) → MVP.
2. US2 (P1) → AX/MX 1.0 reachable.
3. US3 → every fault typed; required before the `bus` module's fault state.
4. US4 → sync ops and broadcast ping; required before multi-actuator ops.
5. Polish → robustness, zero-alloc proof, CI, C4 model, PR.
