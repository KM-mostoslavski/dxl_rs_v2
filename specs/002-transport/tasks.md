---

description: "Task list for the transport module (std serial adapter and mock)"
---

# Tasks: Transport (std serial adapter and mock)

**Input**: Design documents from `specs/002-transport/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/transport-api.md, quickstart.md

**Tests**: Requested by the spec (FR-009, FR-010, SC-002, SC-003, SC-005, SC-006). Test tasks
come first in each story and must fail (or not compile) before the implementation task after
them.

**Organization**: Tasks are grouped by user story so each story can be implemented and tested on
its own.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: US1 = std serial adapter, US2 = mock transport, US3 = bring-your-own UART contract

## Rules for every task

- Rust 2018+ layout: `src/transport.rs` + `src/transport/*.rs`, never `mod.rs`.
- Only these files may change: `src/transport.rs`, `src/transport/**`, the transport lines of
  `src/lib.rs`, and the transport lines of `Cargo.toml`. Do not touch `src/protocol.rs`,
  `src/protocol/**` or `src/error.rs` (research R-1, plan Structure Decision).
- Document every public item; no comments inside method bodies unless resolving a real ambiguity.
- No `dyn`, no `Box`. No allocation in the contract or the mock. No `panic!`, `unwrap`, `expect`
  or out-of-bounds indexing on any `read`/`write`/`flush`/`read_ready` path. Scripting past
  capacity and the `assert_*` helpers are the only panics, each `#[track_caller]` with a `# Panics`
  doc section.
- **Shared working tree**: the uncommitted protocol work (spec 001) lives in this tree, and
  `Cargo.toml`, `Cargo.lock` and `src/lib.rs` already carry protocol edits. When committing on
  `002-transport`, stage only transport files and transport hunks: build a patch of the hunks and
  apply it with `git apply --cached`, since interactive `git add -p` is unavailable. Never stage
  `src/protocol*`, `src/error.rs`, `docs/`, `.specify/`, `.claude/` or `.aiignore`.
- Commits carry no co-authorship or AI attribution lines.

---

## Phase 1: Setup

**Purpose**: Dependencies, feature flag and module skeleton

- [X] T001 In Cargo.toml: ensure `embedded-io = { version = "0.7", default-features = false }` is in `[dependencies]` and the `std` feature lists `"embedded-io/std"` (both may already exist from the uncommitted protocol diff; keep one copy). Add the feature `mock = []` with a comment `# Scripted, allocation-free MockTransport for tests without hardware.` Do not add any dependency to `mock`
- [X] T002 Create the module skeleton: `src/transport.rs` with module doc (one paragraph: what a transport is; timeouts are the bus's job), `#[cfg(feature = "std")] mod serial;`, `#[cfg(any(test, feature = "mock"))] mod mock;`, plus files `src/transport/serial.rs` and `src/transport/mock.rs` (no `no_alloc.rs`, see T015). Add `mod transport;` to src/lib.rs after `mod register;`
- [X] T003 Verify the skeleton builds: `cargo build`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --all-targets --features mock -- -D warnings`, and `cargo clippy --no-default-features --features ax,mx,x-series,y-series,pro,mock --target thumbv7em-none-eabihf` all pass. Fix lint fallout only in files from T001–T002

---

## Phase 2: Foundational (blocking prerequisites)

**Purpose**: The contract every story builds on

**⚠️ CRITICAL**: No user-story work starts until this phase is complete

- [X] T004 In src/transport.rs define `pub trait Transport: embedded_io::Read + embedded_io::Write + embedded_io::ReadReady {}` and the blanket `impl<T: embedded_io::Read + embedded_io::Write + embedded_io::ReadReady> Transport for T {}`. The trait doc lists the implementor rules from data-model.md § Transport verbatim in substance: "`read_ready` never blocks"; "`read` called after `read_ready() == Ok(true)` returns at least one byte without blocking; `Ok(0)` means end of stream"; "`flush` returns only after every written byte has left the implementor's buffers; direction pins (DE/RE) switch back to receive before `flush` returns"; "the transport does not echo sent bytes back"; "no internal retry; a timeout, if any, is reported as `ErrorKind::TimedOut`". Add a `# Half-duplex` section (U2D2 and auto-direction RS-485 do it in hardware; MCU users toggle DE/RE inside `write`/`flush`). Re-export with `pub use transport::Transport;` in src/lib.rs
- [X] T005 Add a unit test module in src/transport.rs proving the blanket impl: a private zero-sized `Loop` type implementing `ErrorType<Error = ErrorKind>`, `Read`, `Write`, `ReadReady` (no buffering needed) is accepted by a `fn takes<T: Transport>(_: &mut T)` helper

**Checkpoint**: The contract exists. US1, US2 and US3 can start in parallel.

---

## Phase 3: User Story 1 - Open a serial port and talk to actuators from a desktop (Priority: P1) 🎯 MVP

**Goal**: `SerialTransport::open(path, baud)` gives a `Transport` over the OS serial port, with no
`dyn`.

**Independent Test**: Pseudo-terminal pair tests in src/transport/serial.rs (quickstart §2, US1 rows).

### Tests for User Story 1 ⚠️

> Write these first, in `#[cfg(all(test, unix))] mod tests` at the bottom of src/transport/serial.rs; they must fail to compile or fail before T008.

- [X] T006 [US1] Add a `pair()` helper building two `SerialTransport`s from `serialport::TTYPort::pair()` via `From<TTYPort>`, then tests: (a) `bytes_cross_the_link`: `write_all(&[0xFF,0xFF,0xFD,0x00])` + `flush` on one end, `read_exact` of 4 bytes on the other is equal; (b) `idle_line_not_ready`: `read_ready()` is `Ok(false)` on a fresh pair; (c) `ready_after_write`: after the peer writes and flushes, poll `read_ready()` for at most 50 ms and expect `Ok(true)`; (d) `from_keeps_port_settings`: a pair port given `set_timeout(Duration::from_millis(37))` before `From` still reports 37 ms via `port_mut().timeout()` (needs `serialport::SerialPort` in scope); (e) `into_inner_returns_port`
- [X] T007 [US1] Add tests: (f) `open_missing_port_is_not_found`: `SerialTransport::open("/dev/does-not-exist", 1_000_000)` is `Err(e)` with `e.kind() == serialport::ErrorKind::Io(NotFound)` (corrected during implementation: serialport maps `ENOENT` to `Io(NotFound)` and `EBUSY` to `NoDevice`); (g) `idle_read_times_out_fast` (SC-004): on a pair whose port timeout is set to 1 ms, `embedded_io::Read::read` of 1 byte on an idle end returns `Err(e)` with `embedded_io::Error::kind(&e) == ErrorKind::TimedOut` and takes < 2 ms measured with `std::time::Instant`; (h) `read_ready_is_fast`: 100 calls of `read_ready()` on an idle line each take < 1 ms; (i) `closed_peer_is_an_error_not_a_hang` (SC-005): drop the peer, then `read_ready()` or `read` returns either `Err(_)` or `Ok(0)`/`Ok(true)` followed by `Ok(0)`, within 50 ms, never panicking

### Implementation for User Story 1

- [X] T008 [US1] Implement src/transport/serial.rs: private `#[cfg(unix)] type NativePort = serialport::TTYPort;` / `#[cfg(windows)] type NativePort = serialport::COMPort;`; `pub struct SerialTransport { port: NativePort }`; `pub fn open(path: &str, baud_rate: u32) -> serialport::Result<Self>` = `serialport::new(path, baud_rate).data_bits(DataBits::Eight).parity(Parity::None).stop_bits(StopBits::One).flow_control(FlowControl::None).timeout(Duration::from_millis(1)).open_native()`; `impl From<NativePort> for SerialTransport` (settings unchanged); `pub fn port_mut(&mut self) -> &mut NativePort`; `pub fn into_inner(self) -> NativePort`. Public signatures name the concrete port type under `cfg`, not a public alias (contracts/transport-api.md)
- [X] T009 [US1] Implement the `embedded_io` traits for `SerialTransport` in src/transport/serial.rs: `ErrorType { type Error = std::io::Error; }`; `Read::read` → `std::io::Read::read(&mut self.port, buf)`; `Write::write`/`flush` → `std::io::Write` on the port; `ReadReady::read_ready` → `Ok(self.port.bytes_to_read().map_err(std::io::Error::from)? > 0)` (needs `serialport::SerialPort` in scope)
- [X] T010 [US1] Write the `SerialTransport` docs in src/transport/serial.rs: struct doc with a `no_run` example `let mut port = SerialTransport::open("/dev/ttyUSB0", 1_000_000)?;`; a `# Latency` section (FTDI adapters including the U2D2 hold input up to the latency timer, 16 ms by default on Linux, longer than the bus's 10 ms default status timeout; set 1 ms with `echo 1 | sudo tee /sys/bus/usb-serial/devices/ttyUSB0/latency_timer`; the crate never changes OS driver settings); on `open`, a `# Errors` section (Unix: `Io(NotFound)` for a missing port, `NoDevice` for a busy one, `InvalidInput` for an unsupported baud); on `From`, a note that the user's timeout only matters if a read races an empty buffer
- [X] T011 [US1] Re-export `#[cfg(feature = "std")] pub use transport::SerialTransport;` from src/transport.rs (`pub use serial::SerialTransport;`) and src/lib.rs; run `cargo test transport::serial` until T006–T007 pass

**Checkpoint**: A desktop user can open a port and exchange raw bytes (US1 done).

---

## Phase 4: User Story 2 - Test my code against a mock transport, no hardware (Priority: P1)

**Goal**: Public, allocation-free `MockTransport` + `MockTime` + `Mismatch` behind the `mock` feature.

**Independent Test**: Unit tests in src/transport/mock.rs and the zero-allocation test (quickstart §2, US2 and FR-010 rows).

### Tests for User Story 2 ⚠️

> Write these first in `#[cfg(test)] mod tests` at the bottom of src/transport/mock.rs; they must not compile or must fail before T016–T018.

- [X] T012 [P] [US2] Gate tests in src/transport/mock.rs: (a) `stale_is_readable_before_any_send`; (b) `reply_waits_for_first_flush` (`read_ready` false after `write`, true after `flush`); (c) `reply_at_waits_for_time` (`reply_at(50, …)`: not readable at t = 49 after flush, readable at t = 50 via `MockTime::set`); (d) `reply_to_nth_flush` (second reply opens only after the 2nd flush); (e) `closed_chunk_hides_later_chunks` ("chunks open strictly in script order"); (f) `silent_line_never_ready`; (g) `read_copies_min_of_buf_and_readable` with a 1-byte buffer over a 3-byte reply
- [X] T013 [P] [US2] Failure and inspection tests in src/transport/mock.rs: (a) `fail_with_reaches_every_call` (`read_ready`, `read`, `write`, `flush` all return `Err(ErrorKind::BrokenPipe)` once set); (b) `end_of_stream_is_ready_but_reads_zero`; (c) `write_past_tx_capacity_returns_zero` using `MockTransport::<4, 4, 2>::with_capacity` and `embedded_io::Write::write_all` → `Err(WriteAllError::WriteZero)` or the 0.7 equivalent; (d) counters `reads()`, `writes()`, `flushes()`; (e) `sent()` returns all written bytes in order
- [X] T014 [P] [US2] `Mismatch` and assertion tests in src/transport/mock.rs: `check_sent` returns `Ok(())` on equal bytes; first differing byte gives `Mismatch { offset, expected: Some(e), actual: Some(a) }`; more sent than expected gives `expected: None`; fewer sent gives `actual: None`; `Display` is `sent byte 7 is 0x03, expected 0x02` (and readable forms for the two length cases); `#[should_panic(expected = "sent byte")] assert_sent` on a mismatch; `#[should_panic] assert_done` when `unread() > 0`; `#[should_panic(expected = "MockTransport")] stale` past `RX` capacity and `reply` past `CHUNKS`; `MockTime`: `advance`, `set` backwards is ignored, `ticker(7)` returns 7, 14 and moves `now()` with it
- [X] T015 [P] [US2] Zero-allocation proof for the mock. **Decision during implementation**: the uncommitted protocol work already installs a counting `#[global_allocator]` (src/protocol/no_alloc.rs), and a second one in src/transport/no_alloc.rs cannot coexist in one test binary. The proof is therefore static: the no_std clippy build `--no-default-features --features ax,mx,x-series,y-series,pro,mock --target thumbv7em-none-eabihf` compiles the mock without `alloc` or `std`, so it cannot name an allocating type. No src/transport/no_alloc.rs is created; the protocol PR may add the mock to its runtime counter test. Record this in the PR

### Implementation for User Story 2

- [X] T016 [US2] Implement `MockTime` in src/transport/mock.rs per data-model.md: `pub struct MockTime { now: Cell<u64> }`; `pub const fn new()`, `Default`, `now()`, `set(us)` ("Setting it backwards is ignored, to keep it monotonic"), `advance(us)` (saturating), `ticker(&self, step_us) -> impl FnMut() -> u64 + '_` (advances by `step_us`, then returns the new time). Doc example shows `ticker` used as a clock closure
- [X] T017 [US2] Implement `MockTransport<'t, const RX: usize = 4096, const TX: usize = 4096, const CHUNKS: usize = 32>` in src/transport/mock.rs with the fields of data-model.md (`time: &'t MockTime`, `rx: [u8; RX]` + len + pos, `chunks: [(usize, Gate); CHUNKS]` + count, `tx: [u8; TX]` + len, `reads`, `writes`, `flushes`, `fail: Option<ErrorKind>`, `eof: bool`) and private `enum Gate { Always, AfterTx { flushes: usize, at: u64 } }`; `impl<'t> MockTransport<'t> { pub fn new(time) }` (default capacities only, so inference works) and generic `with_capacity(time)`; scripting `stale`, `reply` (`AfterTx { flushes: 1, at: 0 }`), `reply_at(at_us, …)` (`AfterTx { flushes: 1, at: at_us }`), `reply_to(n, …)` (`AfterTx { flushes: n, at: 0 }`), `fail_with`, `end_of_stream`, each returning `&mut Self`; over-capacity scripting panics `#[track_caller]` with a message starting `MockTransport` and naming `RX` or `CHUNKS`. Private `available_end()` implements "chunks open strictly in script order; a closed chunk hides every chunk after it"
- [X] T018 [US2] Implement `embedded_io::{ErrorType<Error = ErrorKind>, Read, Write, ReadReady}` for `MockTransport` in src/transport/mock.rs exactly per data-model.md "Trait behaviour" (`fail` checked first on every call; `write` stores `min(buf.len(), TX − len)` and returns 0 when full; `flush` only counts). Use slice `get`/`min` so no path can index out of bounds or panic
- [X] T019 [US2] Implement inspection in src/transport/mock.rs: `sent()`, `reads()`, `writes()`, `flushes()`, `unread()`, `check_sent(expected) -> Result<(), Mismatch>`, `#[track_caller] assert_sent` (panics with the `Mismatch` `Display` text plus both byte slices in hex), `#[track_caller] assert_done` (panics naming the number of unread bytes); `pub struct Mismatch { pub offset: usize, pub expected: Option<u8>, pub actual: Option<u8> }` with `Debug, Clone, Copy, PartialEq, Eq` and `Display`
- [X] T020 [US2] Re-export from src/transport.rs (`#[cfg(feature = "mock")] pub use mock::{Mismatch, MockTime, MockTransport};`) and src/lib.rs (`#[cfg(feature = "mock")] pub use transport::{Mismatch, MockTime, MockTransport};`). Under `cfg(all(test, not(feature = "mock")))` the module stays crate-private without `dead_code` warnings (use `#[cfg_attr(not(feature = "mock"), expect(dead_code, reason = "used by tests only"))]` or equivalent). Add a struct-level doc example on `MockTransport` (runs with `cargo test --features mock --doc`): script a reply, write + flush, read it back, `assert_sent`, `assert_done`
- [X] T021 [US2] Run `cargo test transport::mock`, `cargo test --features mock --doc` and the no_std clippy with `mock` until T012–T015 pass

**Checkpoint**: Tests across the crate (and in user crates with `mock`) can script the wire (US2 done).

---

## Phase 5: User Story 3 - Bring my own microcontroller UART (Priority: P2)

**Goal**: The contract is documented and proven for `no_std`, heap-free user types.

**Independent Test**: The no_std clippy build plus the doc tests on `Transport` (quickstart §1 and §2, US3 row).

### Tests for User Story 3 ⚠️

- [X] T022 [US3] Add to the `Transport` doc in src/transport.rs: (a) a runnable example defining a minimal `Uart` type that implements `ErrorType`, `Read`, `Write`, `ReadReady` with `embedded_io::ErrorKind` as its error, and passing it to `fn needs_transport<T: dxl_rs_v2::Transport>(_: &mut T)`; (b) a `compile_fail` example of the same type without `ReadReady` failing that bound (US3 scenario 2)

### Implementation for User Story 3

- [X] T023 [US3] Finish the module doc in src/transport.rs: a "Bring your own UART" section listing the three required traits, that no wrapper from this crate is needed, that the bus owns timeouts and needs a `Clock`, the half-duplex direction-control note, and "the transport must not echo; single-wire UARTs without automatic direction control are out of scope". Link `SerialTransport` (std) and `MockTransport` (mock) with `cfg`-aware intra-doc links that do not break under `--no-default-features`
- [X] T024 [US3] Verify: `cargo test --doc` passes (incl. the `compile_fail` case), and `cargo clippy --no-default-features --features ax,mx,x-series,y-series,pro,mock --target thumbv7em-none-eabihf -- -D warnings` passes

**Checkpoint**: All three stories work on their own.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T025 Run every gate in quickstart.md §1 (`fmt --check`, both clippy variants, `test --workspace`, `test --features mock --doc`, the no_std clippy with `mock`, `llvm-cov` ≥ 70 %), and fix fallout only in transport files
- [X] T026 Verify the branch builds without the uncommitted protocol work: after committing (see Rules), `git worktree add ../dxl_rs_v2-002-check 002-transport`, run the quickstart §1 gates there, then `git worktree remove ../dxl_rs_v2-002-check`. Report any gap; do not pull protocol files in to fix it
- [X] T027 Fill specs/002-transport/pr-description.md from .github/pull_request_template.md: summary, SRS coverage (module `transport`: 4.3, §7 Transport row, NFR-10; FR-001…FR-014), the Architecture section with the three Complexity Tracking items of plan.md (`ReadReady` deviation pending SRS §7 / constitution wording; `src/lib.rs` + `Cargo.toml` cross-module lines; mock-helper panics), the follow-up for the protocol PR (replace its `pub(crate) trait Transport` with `crate::transport::Transport`; consider migrating `FakeLine` to `MockTransport`), the note that `feature/transport` (2026-10-13) is superseded, and the quickstart §4 manual hardware check as an unchecked item. No AI attribution
- [ ] T028 Run the manual hardware check in quickstart.md §4 if a U2D2 and an actuator are available; otherwise leave it unchecked in the PR description

---

## Phase 7: Review fixes (2026-10-27)

**Purpose**: Apply the findings an adversarial review of PR #6 confirmed (research R-9)

- [X] T029 Map `Ok(0)` from a non-empty write to `TimedOut` (the contract's timeout kind; a Windows write timeout reports `Ok(0)`) in src/transport/serial.rs (private `accepted` helper + unit test)
- [X] T030 Raise the port timeout to 50 ms (`PORT_TIMEOUT`) and document what it bounds in src/transport/serial.rs; drop the unverified `InvalidInput` baud claim
- [X] T031 Use the private `NativePort` alias in `port_mut`, `into_inner` and `From` in src/transport/serial.rs (no duplicated cfg signatures)
- [X] T032 Replace wall-clock < 1 ms / < 2 ms asserts with 500 ms hang bounds; add open-settings and busy-port tests in src/transport/serial.rs
- [X] T033 Gate `reply`/`reply_at` on the next send after scripting; compact `Debug`; tests for both and for `reply_at` needing send and time in src/transport/mock.rs
- [X] T034 Move the user guide (bring your own UART, half-duplex, no echo) onto the `Transport` trait docs; pin the `compile_fail` doctest to `E0277` in src/transport.rs
- [X] T035 Re-export `embedded_io` (always) and `serialport` (std) from src/lib.rs; doc examples use the re-exports
- [X] T036 Build the `mock` feature in CI: clippy and doc tests in the check job, `mock` in the no_std job (.github/workflows/ci.yml, cross-module: ci)
- [X] T038 Second verification pass: `TimedOut` instead of `WriteZero`, SC-005 revised for the 50 ms port timeout, leftover baud claims removed, `From` doc reworded, Windows-safe `port_mut` doc (src/transport/serial.rs, spec.md, data-model.md, research.md)
- [X] T037 Update research (R-3, R-9), plan, spec SC-004, data-model, contract and quickstart; correct the MSRV to 1.85

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: none.
- **Foundational (Phase 2)**: after Setup. Blocks every story.
- **US1, US2, US3 (Phases 3–5)**: each depends only on Phase 2. They touch different files
  (`serial.rs`, `mock.rs`, `transport.rs` docs), except the re-export lines in
  `src/transport.rs` and `src/lib.rs` (T011, T020), which must be applied one after the other.
- **Polish (Phase 6)**: after the stories you ship.

### Within Each Story

- Tests (T006–T007, T012–T015, T022) before implementation; they fail or do not compile first.
- US1: T008 → T009 → T010 → T011.
- US2: T016 → T017 → T018 → T019 → T020 → T021.
- US3: T022 → T023 → T024.

## Parallel Example: User Story 2

```bash
# Tests in parallel (T012–T014 are separate test functions in one file; T015 is another file):
Task: "Gate tests in src/transport/mock.rs (T012)"
Task: "Failure and inspection tests in src/transport/mock.rs (T013)"
Task: "Mismatch and assertion tests in src/transport/mock.rs (T014)"
Task: "Zero-allocation proof via the no_std build (T015)"

# Across stories after Phase 2:
Task: "US1 SerialTransport in src/transport/serial.rs (T006–T010)"
Task: "US2 MockTransport in src/transport/mock.rs (T012–T019)"
Task: "US3 contract docs in src/transport.rs (T022–T023)"
```

## Implementation Strategy

### MVP First

1. Phases 1–2 (contract).
2. Phase 4 (US2, mock): it unblocks `bus` tests and the coverage gate with no hardware. The
   spec ranks US1 and US2 both P1; US2 is the better first slice because CI can verify all of it.
3. Phase 3 (US1, desktop adapter).
4. Phase 5 (US3, docs and compile proofs), then Phase 6.

### Incremental Delivery

Each phase ends at a checkpoint that passes the quickstart gates. The whole module ships as one
PR (one module per PR); checkpoints are local commits on `002-transport`.

## Notes

- [P] = different files or independent test functions, no dependency on unfinished tasks.
- Commit after each phase, staging only transport files and hunks (see Rules).
- Stop at any checkpoint to validate a story on its own.
