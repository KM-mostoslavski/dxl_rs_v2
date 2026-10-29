# Quickstart: validating the transport module

**Feature**: [spec.md](spec.md) | **API**: [contracts/transport-api.md](contracts/transport-api.md)

## Prerequisites

- Rust stable ≥ 1.85 (edition 2024) and the `thumbv7em-none-eabihf` target (`rustup target add thumbv7em-none-eabihf`).
- `cargo-llvm-cov` for the coverage gate.
- Linux or macOS for the pseudo-terminal tests (they are skipped on Windows).
- Optional, manual: a U2D2 (or any USB-TTL/RS-485 adapter) and one Protocol 2 actuator at ID 1,
  1 Mbaud.

## 1. Gates every PR runs

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features mock -- -D warnings
cargo test --workspace
cargo test --features mock --doc            # public mock examples
cargo clippy --no-default-features --features ax,mx,x-series,y-series,pro,mock --target thumbv7em-none-eabihf
cargo llvm-cov --workspace --ignore-filename-regex 'src/models(\.rs|/)' --fail-under-lines 70
```

Expected: all pass. The no_std line proves the contract and the mock build with no `std` and no
heap (FR-002, FR-010, SC-002).

## 2. Scenario checks (all in `cargo test`)

| Spec item | Test lives in | Expected |
|---|---|---|
| US1-1, US1-3 | `src/transport/serial.rs` (pty pair) | bytes written and flushed on one end read back identically on the other |
| US1-2 | `serial.rs` | `open("/dev/does-not-exist", 1_000_000)` is `Err` with kind `Io(NotFound)` (busy port: `NoDevice`) |
| US1-4, SC-004 | `serial.rs` | `read_ready()` is `Ok(false)` on an idle line and never waits; `read` on an idle line returns `TimedOut` within the port timeout |
| US1-1 | `serial.rs` | `open` on a pty path configures 8N1, no flow control, 50 ms timeout; a second `open` of the same path is `NoDevice` |
| US1-6, FR-004 | `serial.rs` | a port from `TTYPort::pair()` keeps its own timeout after `From` |
| US2-1…4, FR-008 | `src/transport/mock.rs` | each gate opens exactly when scripted; injected kind and end of stream reach the caller |
| US2-5, FR-009 | `mock.rs` | `check_sent` reports the first differing offset, and length differences |
| US2-6 | `mock.rs` | `unread()` > 0 and `assert_done` panics when a reply was never read |
| FR-010, SC-002 | no_std clippy line of §1 | the mock compiles with neither `std` nor `alloc`, so it cannot allocate |
| US3-2 | `compile_fail` doctest on `Transport` | a type without `ReadReady` does not satisfy `Transport` |

## 3. Use the mock from an application crate

In a scratch crate depending on this one with `features = ["mock"]` as a dev-dependency:

1. Create a `MockTime`, a `MockTransport::new(&time)`, script `stale` + `reply_at(…)`.
2. Write a packet, flush, then poll `read_ready` with `time.ticker(10)` until the reply opens.
3. `assert_sent(&expected)` and `assert_done()`.

Expected: it compiles with only the `mock` feature added (FR-011), and the reply is not readable
before the flush and the scripted time.

## 4. Manual hardware check (SC-001, optional)

1. Lower the FTDI latency timer: `echo 1 | sudo tee /sys/bus/usb-serial/devices/ttyUSB0/latency_timer`.
2. In a test binary: `let mut t = SerialTransport::open("/dev/ttyUSB0", 1_000_000)?;`. That is the
   one line of transport setup.
3. Write the e-Manual Protocol 2 Ping instruction for ID 1, `FF FF FD 00 01 03 00 01 19 4E`
   ([Protocol 2.0, Ping example 1](https://emanual.robotis.com/docs/en/dxl/protocol2/#example-1); same bytes as `src/protocol/vectors.rs`), then flush.
4. Poll `read_ready` for up to 10 ms, then read.

Expected: a 14-byte status packet starting `FF FF FD 00 01 07 00 55 00`. Without step 1, the
reply typically arrives about 16 ms late. Once the `bus` module lands, this becomes a one-line
`bus.ping`.
<!-- ci probe 1 (DO NOT MERGE) -->
<!-- ci probe 2 -->
