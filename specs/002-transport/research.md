# Research: Transport (std serial adapter and mock)

**Feature**: [spec.md](spec.md) | **Date**: 2026-10-26

Sources: `serialport` 4.9.0 and `embedded-io` 0.7.1 sources (local cargo registry), the
uncommitted protocol module (`src/protocol.rs`, `src/protocol/fake.rs`), spec/plan 001, and the
unmerged `feature/transport` branch (commit `53f3e47`, 2026-10-13).

## R-1 The transport contract

**Decision**: `src/transport.rs` defines `pub trait Transport: Read + Write + ReadReady {}` from
`embedded_io`, with a blanket impl for every type that has the three. It has no methods of its
own. The bus and protocol layers bound on `T: Transport`.

**Rationale**: This matches the bound the protocol layer already uses (`pub(crate) trait Transport`
in `src/protocol.rs`), which the architect approved on 2026-10-26 (plan 001, R-1). A blanket impl
means a HAL UART that implements the three `embedded-io` traits works with no wrapper (FR-002).
A missing `ReadReady` is a compile error at the `Bus` type, which satisfies US3 scenario 2.

**Alternatives considered**:
- Own trait with `clear_rx` and `set_timeout` (the `feature/transport` branch). Rejected: it moves
  timeout enforcement into the transport, against SRS 4.4 ("timeouts are enforced by the bus").
  Every MCU user would also have to write an impl, so it fails FR-002.
- Bare `Read + Write`. Rejected: an `embedded_io::Read` blocks with no bound, so a silent
  actuator hangs the bus (plan 001, R-1).

**Ordering with the protocol PR**: transport lands first (build order in the PR workflow). The
protocol PR then replaces its private trait with `use crate::transport::Transport`. That edit is
inside `src/protocol.rs`, in the protocol PR, so this PR does not touch `src/protocol/**`.

## R-2 No dynamic dispatch in the std adapter

**Decision**: `SerialTransport` wraps the platform's concrete port type through a private alias:
`serialport::TTYPort` on Unix and `serialport::COMPort` on Windows. Opening uses
`serialport::new(path, baud).open_native()`.

**Rationale**: `serialport::SerialPortBuilder::open()` returns `Box<dyn SerialPort>`, which breaks
NFR-4 (no `dyn`) and allocates. `open_native()` returns the concrete type (serialport `lib.rs`
lines 451 and 457). The old branch used `Box<dyn SerialPort>`, so that part is not reused.

**Alternatives considered**: generic `SerialTransport<P: SerialPort>`. Rejected: it adds a type
parameter users must name, for no gain, because exactly one native type exists per platform.

## R-3 "Input pending" and read/write timing on the std adapter

**Decision**:
- `read_ready()` returns `bytes_to_read()? > 0`. That is `FIONREAD` on Unix and
  `ClearCommError` on Windows, and it never blocks.
- `open()` sets the port timeout to 1 ms. `read()` polls up to that timeout, then reads. The bus
  only calls `read` after `read_ready` is true, so in practice `read` returns at once. The 1 ms
  bound covers races (FR-005, SC-004).
- `write` and `flush` go straight to the port. `flush` is `tcdrain` on Unix, which blocks until the
  bytes have left the host UART buffer (US1 scenario 3).

**Rationale**: The builder default timeout is 0 ms (serialport `lib.rs` line 903), so a write
would fail at once if the OS buffer were ever full. 1 ms is the smallest bound that still keeps
write robust. The largest packet the crate sends (P2 sync write to 253 IDs × 4 bytes, about
1.3 KB) fits the 4 KB Linux tty buffer, so in practice write does not wait at all.

**Alternatives considered**: a large timeout, e.g. 100 ms like `TTYPort::pair`. Rejected: a read
that races an empty buffer would stall the bus well past its 10 ms status timeout.

**Already-open ports (FR-004)**: `From<NativePort>` keeps the user's settings, timeout included.
The docs note that the timeout only matters for the race above.

## R-4 Errors

**Decision**:
- `SerialTransport`'s `embedded_io::ErrorType::Error` is `std::io::Error`. With `embedded-io/std`,
  `embedded-io` implements `embedded_io::Error` for it and maps `std::io::ErrorKind::TimedOut`
  to `ErrorKind::TimedOut` (embedded-io `lib.rs` lines 148–204). The protocol layer's `io_error`
  already turns `TimedOut` into `TransportError::Timeout` (FR-006).
- `serialport::Error` from `bytes_to_read()` converts through serialport's
  `From<serialport::Error> for io::Error`: `NoDevice` → `NotFound`, `InvalidInput` →
  `InvalidInput`, `Io(k)` → `k`.
- `open()` returns `serialport::Result<SerialTransport>`. That error is typed (on Unix: a missing port is
  `Io(NotFound)`, a busy port `NoDevice` from `EBUSY`, a rejected baud `InvalidInput`) and happens at startup, outside any bus call, so it is not wrapped in the
  crate `Error` (US1 scenario 2).
- The mock's error type is `embedded_io::ErrorKind` itself. It implements `embedded_io::Error`,
  is `Copy`, and needs no allocation.

**Alternatives considered**: a crate `OpenError`. Rejected: it adds a type with nothing to add,
and would hide serialport's description string from the user.

## R-5 Mock design: allocation-free, public, protocol-agnostic

**Decision**: `MockTransport<'t, const RX: usize = 4096, const TX: usize = 4096, const CHUNKS:
usize = 32>`, generalised from the protocol's private `FakeLine`:

- Incoming script: chunks of bytes, each with a gate. **Always** means stale bytes. **After the
  nth flush, at time t** covers replies, late replies and multi-exchange scripts.
- Outgoing record: every written byte, plus write/read/flush counters.
- Injected failure kind (sticky from the moment it is scripted), end of stream (ready but reads
  0 bytes), and silence (nothing scripted).
- Time: a shared `MockTime` (a `Cell<u64>` of microseconds) borrowed by the mock. It hands out a
  ticking closure `FnMut() -> u64`, which is a protocol `Clock` through the blanket impl. The
  transport module therefore does not depend on `Clock`.
- Checks: `sent()`, `check_sent(expected) -> Result<(), Mismatch>` (first differing offset,
  expected and actual byte, or a length difference), `unread()`, and `assert_sent` /
  `assert_done` wrappers that panic with a readable message.
- `new(&time)` exists only on the default capacities, so `MockTransport::new(&time)` infers.
  `with_capacity` is the generic constructor.

**Rationale**: `FakeLine` already proves this shape works with the protocol's drain and receive
loops and with the zero-allocation test. Fixed arrays keep it no_std and heap-free (FR-010).
Const-generic capacities let a long test raise the limits without a crate change.

**Alternatives considered**:
- `VecDeque`/`Vec` script (the old branch). Rejected: it allocates, which breaks FR-010 and the
  no-alloc tests.
- An "expect request, then release reply" mock that panics on wrong bytes mid-call. Rejected: it
  panics inside the code under test. Checking after the call gives a better message and keeps
  the code under test panic-free.

**Panics**: scripting past capacity panics at the scripting call (`#[track_caller]`, documented
under `# Panics`), so the test fails at its setup line (spec edge case). The `assert_*` helpers
panic by design. None of the code paths the crate itself drives (`read`, `write`, `flush`,
`read_ready`) can panic. See Complexity Tracking in plan.md.

## R-6 Feature wiring

**Decision**: `mock = []` in `Cargo.toml`, off by default, no dependencies. The mock is compiled
under `cfg(any(test, feature = "mock"))`, so crate tests use it with default features and
coverage counts it. `SerialTransport` is under `cfg(feature = "std")`. `embedded-io` is added as
a dependency, and `std` forwards `embedded-io/std`. The same lines are in the uncommitted protocol
diff; whichever PR merges second drops its copy.

**Re-exports**: flat, like the rest of `lib.rs`: `dxl_rs_v2::Transport`, `dxl_rs_v2::SerialTransport`
(std), `dxl_rs_v2::{MockTransport, MockTime, Mismatch}` (mock).

## R-7 Testing without hardware

**Decision**:
- The serial adapter is tested on a Unix pseudo-terminal pair (`TTYPort::pair()`, serialport
  `tty.rs` line 292): bytes cross the link, `read_ready` is false then true, `read` on an empty
  line returns `TimedOut` within 2 ms, flush returns, opening a missing path gives `Io(NotFound)`,
  and `From<TTYPort>` keeps the timeout. Gated `cfg(all(test, unix))`.
- The mock gets unit tests for every gate, failure and check (SC-003).
- A compile-only `no_std` check with `--features mock` on `thumbv7em-none-eabihf` (SC-002), plus a
  counting-allocator test like the protocol's `no_alloc.rs`, local to `src/transport/`.
- A doc example on `Transport` shows the bound in use. The compile-fail case (a type without
  `ReadReady`) is a `compile_fail` doctest on `Transport`.

**Not testable in CI**: real hardware ping and the FTDI latency figure (SC-001, SC-004 on the
reference host) are covered by a manual quickstart step.

## R-8 The unmerged `feature/transport` branch

**Finding**: it predates the SRS (2026-10-13). It defines a different `Transport` (with
`clear_rx`/`set_timeout`), a `TransportError` with `UnexpectedEof`, a `Box<dyn SerialPort>`
adapter, an allocating mock, and ARCHITECTURE.md edits recording that design. The SRS (later,
higher authority) and the approved protocol design supersede it.

**Reused**: the latency-timer doc text, the half-duplex direction-control doc text, and the
pseudo-terminal test approach.

**Action for the architect**: close or delete `feature/transport` and its worktree
(`../dxl_rs_v2-transport`) once this lands. This plan does not touch it.
