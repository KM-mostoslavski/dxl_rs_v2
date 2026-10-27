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
- `open()` sets the port timeout to 50 ms (revised 2026-10-27 from 1 ms, see R-9). The bus only
  calls `read` after `read_ready` is true, so `read` returns at once and the timeout never delays
  a reply; it bounds a read racing an empty buffer (FR-005).
- `write` goes to the port, and a write that accepted nothing for a non-empty buffer becomes
  `io::ErrorKind::WriteZero` (embedded-io forbids `Ok(0)`; R-9). `flush` is `tcdrain` on Unix,
  which blocks until the bytes have left the host UART buffer (US1 scenario 3).

**Rationale**: serialport uses the one port timeout for more than reads: it bounds the wait for
the OS to accept written bytes (`poll` on Unix, `WriteTotalTimeoutConstant`, the whole write, on
Windows) and the retry window of a `flush` interrupted by a signal on Unix. 1 ms made those fail
spuriously (a 1.3 KB sync write drains for about 225 ms at 57 600 baud). 50 ms keeps writes and
flushes robust, and costs nothing on the reply path because reads follow `read_ready`.

**Alternatives considered**: 1 ms (the first version). Rejected after review: it turned slow
writes and interrupted flushes into fake timeouts. The builder default of 0 ms fails writes the
moment the OS buffer is full.

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
  line returns `TimedOut` within the port timeout, flush returns, opening a missing path gives `Io(NotFound)`,
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

## R-9 Review findings applied (2026-10-27)

An adversarial review of PR #6 (four reviewers, each batch checked by a skeptic) confirmed and
this branch fixed:

- `SerialTransport::write` could hand `Ok(0)` to embedded-io (a Windows write timeout reports
  that way), and `write_all` panics on it. It now maps to `WriteZero`, unit-tested through the
  private `accepted` helper.
- The 1 ms port timeout also bounded writes and flush retries, and the docs said otherwise. Now
  50 ms, with docs on `PORT_TIMEOUT` and `From` saying what it bounds (R-3).
- `reply`/`reply_at` gated on the first flush ever, not on the next send (FR-008). They now gate
  on the send after they are scripted; `reply_to(n)` stays absolute.
- CI never built the `mock` feature, so its clippy, doc tests and the no_std/no-alloc proof (T015)
  did not run. CI now runs clippy and doc tests with `mock` and adds it to the no_std job.
- `mod transport` is private, so its module docs were never published. The guide (bring your
  own UART, half-duplex, no echo) now lives on the `Transport` trait.
- The public API exposed `embedded_io` and `serialport` types without re-exporting the crates.
  Both are re-exported (`serialport` under `std`).
- Docs claimed an OS-rejected baud gives `InvalidInput`; Linux drivers round instead. Removed.
- `MockTransport`'s derived `Debug` printed both 4 KB buffers; it now shows counters and sent
  bytes.
- Timing tests asserted < 1 ms / < 2 ms wall-clock bounds (flaky on shared runners and under
  coverage instrumentation); they now catch hangs with a 500 ms margin.
- Duplicated `#[cfg(unix)]`/`#[cfg(windows)]` signatures now use the private `NativePort` alias.
- New tests: open configures 8N1 without flow control; a busy port is `NoDevice`; a reply scripted
  after an exchange waits for the next send; `reply_at` needs both the send and the time.
- `compile_fail` doctest pinned to `E0277`. MSRV corrected to 1.85 (edition 2024).

Rejected by the skeptics (not changed): a silent mock `read` returning `Ok(0)` (only `read_ready`
gated reads happen in the crate), a one-shot `fail_with` (the bus owns retries; can be added with
the bus), per-packet boundaries in `sent()`.
