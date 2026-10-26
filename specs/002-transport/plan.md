# Implementation Plan: Transport (std serial adapter and mock)

**Branch**: `002-transport` | **Date**: 2026-10-26 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/002-transport/spec.md`

## Summary

Build the `transport` module. It has three parts:

- **Contract**: a public `Transport` trait, `embedded_io` `Read + Write + ReadReady`, with a
  blanket impl, so any HAL UART works unwrapped.
- **Std adapter**: `SerialTransport` over serialport's native port type, with no `dyn`. It opens
  by path and baud (8N1, 1 ms poll bound) or wraps an already-open port, and its "input pending"
  check is `bytes_to_read`.
- **Mock**: allocation-free, public behind an opt-in `mock` feature. Bytes come in by script,
  outgoing bytes are recorded, timing is gated on flushes plus a shared `MockTime`, failures and
  end of stream can be injected, and the sent bytes are checked after the call.

Timeouts stay with the bus. The transport has no clock and no retries.

## Technical Context

**Language/Version**: Rust stable, edition 2024. MSRV ≥ 1.81 (`embedded-io` 0.7).

**Primary Dependencies**: `embedded-io` 0.7 (no default features; `std` forwards
`embedded-io/std`). `serialport` 4.9 (existing, optional, `std` only; `open_native`, `TTYPort` /
`COMPort`).

**Storage**: N/A

**Testing**: `cargo test`:
- unit tests in `src/transport/**`
- `SerialTransport` tests on a pseudo-terminal pair (`cfg(all(test, unix))`)
- a counting-allocator test for the mock
- a `compile_fail` doctest for a missing `ReadReady`

Also the no_std clippy check with `--features mock`, and `cargo llvm-cov` at ≥ 70 %.

**Target Platform**: Linux, macOS and Windows (std adapter); `thumbv7em-none-eabihf` (contract and
mock, no heap).

**Project Type**: library crate (workspace with `xtask/`).

**Performance Goals**: with nothing arriving, `read_ready` returns in < 1 ms and `read` in
< 2 ms (SC-004). The mock does constant work per call, with no allocation.

**Constraints**: no `dyn`, no allocation outside `std`, no panic on any I/O outcome, no retries,
no clock in the transport. Module paths are `src/transport.rs` and `src/transport/**` only, plus
the declared cross-module lines.

**Scale/Scope**: one trait, one std adapter (two platform port types behind one alias), one mock
with three helper types.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Check | Status |
|---|---|---|
| I. no_std and Heap-Free | Contract and mock use only `core` + `embedded-io`; mock storage is fixed arrays; `SerialTransport` is `cfg(feature = "std")`; a counting-allocator test proves the mock allocates nothing. | PASS |
| II. Absent Over Failing | A type without `ReadReady` is not a `Transport`: compile error (`compile_fail` doctest). `SerialTransport` is absent without `std`, the mock absent without `mock`. | PASS |
| III. Nothing Hidden | No retries, no clock, no implicit flush or clear in the transport; RX clearing and timeouts stay in the protocol/bus (FR-5.2, FR-1.4). An already-open port's settings are used unchanged. | PASS |
| IV. e-Manual Is the Only Source | No Dynamixel protocol facts in this module (FR-012). | N/A |
| V. Committed, Data-Driven Codegen | Not touched. | N/A |
| VI. Typed, Static, Panic-Free API | `open_native` concrete port, no `dyn`. Errors are `std::io::Error` / `ErrorKind` (typed kinds) mapped by the existing `io_error`. No panic in `read`/`write`/`flush`/`read_ready`. | PASS, with one justified exception (mock test helpers, see Complexity Tracking) |
| VII. DX First | Desktop setup is one line: `SerialTransport::open("/dev/ttyUSB0", 1_000_000)?`. HAL UARTs need no wrapper. `MockTransport::new(&time)` infers its defaults. Every public item documented. | PASS |
| Technical Constraints: "any blocking `embedded_io::{Read, Write}`" | Contract adds `ReadReady`. | **DEVIATION**: approved by the architect 2026-10-26 (plan 001); wording amendment to SRS §7 and the constitution is still pending. |
| Workflow: one module per PR | Touches `src/lib.rs` (core) and `Cargo.toml` (ci). | **DEVIATION**: see Complexity Tracking. |

Post-design re-check (after Phase 1): unchanged. Everything passes except the two recorded
deviations and the mock-helper panic exception.

## Project Structure

### Documentation (this feature)

```text
specs/002-transport/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── transport-api.md
├── checklists/
│   └── requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
src/
├── lib.rs               # + mod transport; re-exports (core, cross-module)
├── transport.rs         # Transport trait + blanket impl, module docs (contract, half-duplex,
│                        # no echo, latency timer), compile_fail doctest
└── transport/
    ├── serial.rs        # #[cfg(feature = "std")] SerialTransport over NativePort
    │                    # (TTYPort | COMPort), open / From / into_inner / port_mut,
    │                    # Read + Write + ReadReady; pty tests #[cfg(all(test, unix))]
    ├── mock.rs          # #[cfg(any(test, feature = "mock"))] MockTransport, MockTime,
    │                    # Mismatch; unit tests
    └── no_alloc.rs      # #[cfg(test)] counting allocator: mock does zero allocations
Cargo.toml               # + embedded-io dep, std → embedded-io/std, mock = [] (ci, cross-module)
```

**Structure Decision**: Rust 2018+ layout (`transport.rs` + `transport/`, no `mod.rs`), all
under the SRS paths `src/transport.rs src/transport/**`, plus the two cross-module files above.
The protocol module's private `FakeLine` and `Transport` alias are left alone. The protocol PR
switches to `crate::transport::Transport` (research R-1).

**Working-tree note**: the uncommitted protocol work (spec 001) shares this working tree. Only
transport files, the `lib.rs` lines and the `Cargo.toml` lines of this plan are committed on
`002-transport`.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| Contract `Read + Write + ReadReady` vs SRS/constitution "any `Read + Write`" | Bus-enforced timeouts need a non-blocking readiness check (plan 001, R-1). | Bare `Read + Write` hangs on a silent actuator. A custom trait with `set_timeout` (old `feature/transport`) moves timeouts into the transport, against SRS 4.4, and forces every MCU user to write an impl. Approved 2026-10-26. |
| Edits to `src/lib.rs` (core) and `Cargo.toml` (ci) in a transport PR | `mod transport` and re-exports must live in `lib.rs`; the `mock` feature and `embedded-io` dependency must live in `Cargo.toml`. | No alternative: a module cannot be wired in without its parent declaring it. Called out in the PR's Architecture section. |
| Panics in the public mock: over-capacity scripting and `assert_sent` / `assert_done` | Test setup errors must fail at the test's own line with a readable message (spec edge case, FR-009). | Returning `Result` from every scripting call makes tests noisy and still ends in `unwrap()`. The non-panicking `check_sent` / `unread` exist alongside. Paths the crate drives (`read`, `write`, `flush`, `read_ready`) never panic. |
