## What
Adds the `transport` module:
- the public `Transport` contract (`embedded-io` `Read + Write + ReadReady`, with a blanket impl);
- `SerialTransport`, an OS serial-port adapter (feature `std`);
- `MockTransport`, a scripted, allocation-free mock (opt-in feature `mock`), with `MockTime` and
  `Mismatch`.

## Why
SRS module `transport`: 4.3 adapters, the Transport row of §7, NFR-10 (mockable transport);
delivery step 8 (std adapter) and the mock from step 1. The `bus` module needs the mock for its
tests and coverage without hardware. Spec: `specs/002-transport/spec.md`.

## How
- `Transport` has no methods. Any HAL UART with the three `embedded-io` traits is a transport, no
  wrapper needed. A type without `ReadReady` fails to compile (`compile_fail` doctest).
- `SerialTransport::open(path, baud)` opens 8N1 with no flow control through
  `serialport::open_native()`, so the port is a concrete `TTYPort` or `COMPort`: no `dyn`, no
  `Box`. `read_ready` is `bytes_to_read() > 0`. `write` maps an `Ok(0)` for a non-empty buffer (how Windows
  reports a write timeout) to `TimedOut`. The port timeout is 50 ms: it bounds write waits, flush retries and a read racing
  an empty buffer, never a reply, because the bus reads only after `read_ready`.
  `From<TTYPort | COMPort>` wraps a port the user opened, keeping its settings.
- `embedded_io` and (under `std`) `serialport` are re-exported, so users need no version-matched
  dependency of their own.
- `MockTransport` replays scripted incoming chunks and records sent bytes in fixed arrays
  (const-generic capacities, defaults 4096/4096/32).
  - Chunk kinds: stale bytes, replies to the next send (optionally at a `MockTime` instant),
    replies to the n-th send overall.
  - Failure injection: an error kind, or end of stream.
  - After the call: `check_sent` reports the first mismatching offset; `assert_sent` and
    `assert_done` panic with readable messages.
  - Writing past capacity returns `ErrorKind::WriteZero`, as embedded-io 0.7 requires (its
    `write_all` panics on `Ok(0)`).
- Tests: a Unix pseudo-terminal pair checks bytes crossing the link, readiness, `open` setting
  8N1 with no flow control, a busy port (`NoDevice`), a missing port (`Io(NotFound)`), an idle
  read timing out, a closed peer (an error, not a hang) and settings kept by `From`. Timing
  asserts only catch hangs (500 ms margin), so they hold on shared CI runners. The mock is
  unit-tested per behaviour.

## Architecture
Follows docs/ARCHITECTURE.md and the SRS, with these deviations:
- **Transport bound is `Read + Write + ReadReady`**, with the bus holding the `Clock`. This matches
  SRS 3.3, 4.4, §7 and FR-1.1, constitution 1.1.0 and docs/ARCHITECTURE.md, all amended
  2026-10-26 (the ARCHITECTURE.md change is in this PR).
- **Cross-module edits**: `src/lib.rs` (core: `mod transport` and re-exports), `Cargo.toml`
  (ci: `embedded-io` 0.7, `std` → `embedded-io/std`, new `mock` feature) and
  `.github/workflows/ci.yml` (ci: clippy and doc tests with `mock`, `mock` in the no_std job, so
  the public mock and its no-alloc proof are actually built in CI).
- **Panics in the public mock**: scripting past capacity, `assert_sent` and `assert_done`
  panic by design (`#[track_caller]`, documented). `read`, `write`, `flush` and `read_ready`
  never panic.
- **Zero allocation is proven statically**: the no_std check builds the mock with neither `std`
  nor `alloc`. There is no runtime counting allocator here because the protocol module installs
  one and a test binary can hold only one; the protocol PR can add the mock to its counter test.
- **Follow-ups for the protocol PR**: replace its private `pub(crate) trait Transport` with
  `crate::transport::Transport`. Fix `FakeLine::write`, which returns `Ok(0)` when full; embedded-io
  0.7 forbids that, and `write_all` panics on it. Consider replacing `FakeLine` with `MockTransport`.
- **Superseded**: the unmerged `feature/transport` branch (2026-10-13) used a different design:
  `clear_rx`/`set_timeout` on the transport, `Box<dyn SerialPort>`, an allocating mock. Its
  latency and half-duplex doc text and its pseudo-terminal test approach were reused. The branch
  and its worktree `../dxl_rs_v2-transport` can be deleted.
- Corrected during implementation: on Unix, `serialport` reports a missing port as `Io(NotFound)`;
  `NoDevice` means a busy port (`EBUSY`).
- **Review round (2026-10-27)**: an adversarial multi-agent review confirmed and this PR fixed:
  the `Ok(0)` write panic path, the 1 ms timeout (now 50 ms), mock replies gating on the first
  send instead of the next, CI never building `mock`, unpublished module docs, missing
  re-exports, a noisy `Debug`, flaky sub-millisecond timing tests, and a wrong baud-error doc.
  A second adversarial pass verified every fix and caught five leftovers, also fixed (T038).
  Details in `specs/002-transport/research.md` R-9 and tasks T029–T038.
- **Known limit**: the `compile_fail,E0277` pin is only enforced by nightly rustdoc; stable CI
  accepts any compile error there. The bus module's trybuild suite will pin it.

## Checklist
- [x] `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass (also with `--features mock`); `cargo doc` with `-D warnings` passes with and without default features
- [x] `cargo test` passes; line coverage ≥ 70 % (`cargo llvm-cov`): transport files 85–99 %
- [x] `no_std` build passes (`--no-default-features --features ax,mx,x-series,y-series,pro,mock --target thumbv7em-none-eabihf`)
- [x] `cargo xtask codegen` leaves `src/models.rs` and `src/models/` unchanged
- [x] Public methods documented
- [x] Docs updated if behaviour or setup changed (transport module docs; FTDI latency note on `SerialTransport`)
- [ ] Manual hardware check (quickstart §4: U2D2 + one Protocol 2 actuator, Ping ID 1) not run, no hardware attached
