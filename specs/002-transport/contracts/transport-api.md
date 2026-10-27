# Contract: Transport public API

**Feature**: [../spec.md](../spec.md) | **Model**: [../data-model.md](../data-model.md)

Signatures are the public surface this PR adds. Bodies, private items and tests are not part of
the contract.

## Always available (`no_std`)

```rust
/// A byte channel to a Dynamixel bus: blocking read and write, plus a
/// non-blocking check for pending input. Timeouts are the bus's job.
pub trait Transport: embedded_io::Read + embedded_io::Write + embedded_io::ReadReady {}

impl<T: embedded_io::Read + embedded_io::Write + embedded_io::ReadReady> Transport for T {}
```

Re-exported as `dxl_rs_v2::Transport`.

Doc requirements: the implementor rules of [data-model.md § Transport](../data-model.md#transport-contract),
the half-duplex direction-control note, the no-echo note, an example that bounds a generic
function on `Transport`, and a `compile_fail` example for a type that lacks `ReadReady`.

## `std` feature

```rust
pub struct SerialTransport { /* NativePort */ }

impl SerialTransport {
    /// Opens `path` at `baud_rate`, 8N1, no flow control.
    pub fn open(path: &str, baud_rate: u32) -> serialport::Result<Self>;
    /// The underlying OS port, for settings this type does not cover.
    pub fn port_mut(&mut self) -> &mut NativePort;
    /// Gives the port back.
    pub fn into_inner(self) -> NativePort;
}

impl From<NativePort> for SerialTransport;      // keeps the port's own settings

impl embedded_io::ErrorType for SerialTransport { type Error = std::io::Error; }
impl embedded_io::Read      for SerialTransport;
impl embedded_io::Write     for SerialTransport;
impl embedded_io::ReadReady for SerialTransport;
```

`NativePort` is a private alias for `serialport::TTYPort` (Unix) or `serialport::COMPort`
(Windows); rustdoc shows the concrete type. `write` maps `Ok(0)` for a non-empty buffer to
`WriteZero`. `reply`/`reply_at` on the mock wait for the next send after they are scripted.

Re-exported as `dxl_rs_v2::SerialTransport`, with the `serialport` crate as
`dxl_rs_v2::serialport` (std) and `embedded-io` as `dxl_rs_v2::embedded_io` (always). Docs include the FTDI latency-timer note (16 ms
default on Linux; set 1 ms through `/sys/bus/usb-serial/devices/<tty>/latency_timer`).

## `mock` feature

```rust
pub struct MockTime { /* Cell<u64> */ }

impl MockTime {
    pub const fn new() -> Self;
    pub fn now(&self) -> u64;
    pub fn set(&self, us: u64);
    pub fn advance(&self, us: u64);
    pub fn ticker(&self, step_us: u64) -> impl FnMut() -> u64 + '_;
}
impl Default for MockTime;

pub struct MockTransport<'t, const RX: usize = 4096, const TX: usize = 4096, const CHUNKS: usize = 32> { /* … */ }

impl<'t> MockTransport<'t> {
    pub fn new(time: &'t MockTime) -> Self;
}

impl<'t, const RX: usize, const TX: usize, const CHUNKS: usize> MockTransport<'t, RX, TX, CHUNKS> {
    pub fn with_capacity(time: &'t MockTime) -> Self;

    // scripting; panics (track_caller) past RX or CHUNKS
    pub fn stale(&mut self, bytes: &[u8]) -> &mut Self;
    pub fn reply(&mut self, bytes: &[u8]) -> &mut Self;
    pub fn reply_at(&mut self, at_us: u64, bytes: &[u8]) -> &mut Self;
    pub fn reply_to(&mut self, nth_flush: usize, bytes: &[u8]) -> &mut Self;
    pub fn fail_with(&mut self, kind: embedded_io::ErrorKind) -> &mut Self;
    pub fn end_of_stream(&mut self) -> &mut Self;

    // inspection
    pub fn sent(&self) -> &[u8];
    pub fn reads(&self) -> usize;
    pub fn writes(&self) -> usize;
    pub fn flushes(&self) -> usize;
    pub fn unread(&self) -> usize;
    pub fn check_sent(&self, expected: &[u8]) -> Result<(), Mismatch>;
    #[track_caller] pub fn assert_sent(&self, expected: &[u8]);
    #[track_caller] pub fn assert_done(&self);
}

impl embedded_io::ErrorType for MockTransport<'_, ..> { type Error = embedded_io::ErrorKind; }
impl embedded_io::Read      for MockTransport<'_, ..>;
impl embedded_io::Write     for MockTransport<'_, ..>;
impl embedded_io::ReadReady for MockTransport<'_, ..>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mismatch { pub offset: usize, pub expected: Option<u8>, pub actual: Option<u8> }
impl core::fmt::Display for Mismatch;
```

Re-exported as `dxl_rs_v2::{MockTransport, MockTime, Mismatch}`.

## Cargo features

| Feature | Default | Adds |
|---|---|---|
| `std` | on | `SerialTransport`; forwards `embedded-io/std` |
| `mock` | off | `MockTransport`, `MockTime`, `Mismatch` (no dependencies, `no_std`-clean) |

No existing method changes behaviour with either feature (constitution VII).

## Error flow (unchanged code, for reference)

Transport `E: embedded_io::Error` → `kind()` → protocol `io_error`: `TimedOut` →
`TransportError::Timeout { id }`, anything else → `TransportError::Io(kind)`. A `read` of 0 bytes
after `read_ready` → `TransportError::Io(END_OF_STREAM)`.
