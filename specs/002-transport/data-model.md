# Data Model: Transport

**Feature**: [spec.md](spec.md) | **Research**: [research.md](research.md)

## Transport (contract)

| Aspect | Value |
|---|---|
| Kind | Marker trait, no methods |
| Supertraits | `embedded_io::Read + embedded_io::Write + embedded_io::ReadReady` |
| Implementors | Blanket: every `T` with the three supertraits |
| Error | `T::Error: embedded_io::Error`; only its `kind()` reaches the crate `Error` |

Behaviour the docs require of an implementor (not checkable by the compiler):

- `read_ready` never blocks.
- `read` called after `read_ready() == Ok(true)` returns at least one byte without blocking. A
  return of `Ok(0)` means end of stream.
- `flush` returns only after every written byte has left the implementor's buffers. Direction
  pins (DE/RE), if any, switch back to receive before `flush` returns.
- The transport does not echo sent bytes back on receive.
- No internal retry. A timeout, if the transport has one, is reported as `ErrorKind::TimedOut`.

## SerialTransport (`std` only)

| Field | Type | Notes |
|---|---|---|
| `port` | `NativePort` (private alias: `serialport::TTYPort` on Unix, `serialport::COMPort` on Windows) | Owned. No `Box`, no `dyn`. |

| Operation | Maps to | Errors |
|---|---|---|
| `open(path, baud)` | `serialport::new(path, baud)` + 8 data bits, no parity, 1 stop bit, no flow control, timeout 1 ms, `.open_native()` | `serialport::Error` (`NoDevice`, `InvalidInput`, `Io(kind)`) |
| `From<NativePort>` | wraps as-is | none |
| `into_inner` / `port_mut` | gives the port back / borrows it for extra settings | none |
| `read_ready` | `bytes_to_read()? > 0` | `serialport::Error` → `std::io::Error` |
| `read` | `std::io::Read::read` on the port (waits ≤ port timeout) | `std::io::Error`; `TimedOut` kind kept |
| `write` | `std::io::Write::write` | `std::io::Error` |
| `flush` | `std::io::Write::flush` (`tcdrain` on Unix) | `std::io::Error` |

State: none beyond the OS port. No caching, no clock.

## MockTime (`mock` feature or tests)

| Field | Type | Notes |
|---|---|---|
| `now` | `Cell<u64>` | Microseconds. Starts at 0 and only moves forward. |

| Operation | Effect |
|---|---|
| `now()` | current value |
| `set(t)` / `advance(dt)` | moves time forward. Setting it backwards is ignored, to keep it monotonic. |
| `ticker(step)` | `impl FnMut() -> u64 + '_`: each call advances by `step` and returns the new time. Usable as the protocol `Clock`. |

## MockTransport<'t, RX = 4096, TX = 4096, CHUNKS = 32> (`mock` feature or tests)

| Field | Type | Notes |
|---|---|---|
| `time` | `&'t MockTime` | shared with the clock |
| `rx` | `[u8; RX]` + length + read position | scripted incoming bytes, in order |
| `chunks` | `[(end: usize, Gate); CHUNKS]` + count | where each scripted chunk ends and when it opens |
| `tx` | `[u8; TX]` + length | everything written |
| `reads`, `writes`, `flushes` | `usize` | counters |
| `fail` | `Option<ErrorKind>` | when set, every later call returns it |
| `eof` | `bool` | ready, but reads return 0 |

**Gate** (private enum):
- `Always`: readable from the start (stale bytes).
- `AfterTx { flushes: n, at: t }`: readable once `flushes ≥ n` and `time.now() ≥ t`.

**Readability rule**: chunks open strictly in script order. A closed chunk hides every chunk
after it, so a reply never overtakes an earlier one.

**Scripting operations** (all return `&mut Self`; each one panics with `#[track_caller]` when it
would exceed `RX` or `CHUNKS`):

| Operation | Gate |
|---|---|
| `stale(bytes)` | `Always` |
| `reply(bytes)` | `AfterTx { flushes: 1, at: 0 }` |
| `reply_at(t, bytes)` | `AfterTx { flushes: 1, at: t }` |
| `reply_to(n, bytes)` | `AfterTx { flushes: n, at: 0 }` |
| `fail_with(kind)` | sets `fail` |
| `end_of_stream()` | sets `eof` |

A silent line is a mock with nothing scripted.

**Trait behaviour** (`Error = embedded_io::ErrorKind`; never panics):

| Call | Behaviour |
|---|---|
| `read_ready` | `Err(fail)` if set; else `eof` or readable bytes exist |
| `read(buf)` | `Err(fail)` if set; else copies `min(buf.len(), readable)` bytes, or 0 |
| `write(buf)` | `Err(fail)` if set; else appends `min(buf.len(), TX − len)` bytes. 0 when full, which `write_all` reports as `WriteZero`. |
| `flush` | `Err(fail)` if set; else `flushes += 1` |

**Inspection**:

| Operation | Result |
|---|---|
| `sent()` | `&[u8]` written so far |
| `reads()`, `writes()`, `flushes()` | counters |
| `unread()` | scripted bytes not yet read |
| `check_sent(expected)` | `Ok(())` or `Err(Mismatch)` |
| `assert_sent(expected)` | panics with the `Mismatch` text |
| `assert_done()` | panics if `unread() > 0` |

## Mismatch

| Field | Type | Notes |
|---|---|---|
| `offset` | `usize` | first index where the sent and expected bytes differ, or the shorter length |
| `expected` | `Option<u8>` | `None` when more bytes were sent than expected |
| `actual` | `Option<u8>` | `None` when fewer bytes were sent than expected |

Implements `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` and `Display` (e.g. `sent byte 7 is 0x03,
expected 0x02`). Not part of the crate `Error`: it is a test-assertion value.
