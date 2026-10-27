# Research: Protocol Packets (P1 and P2)

All protocol facts below were fetched online from the ROBOTIS e-Manual on 2026-10-26 (raw HTML,
not a summariser) and cross-checked by recomputing the CRC/checksum of every example packet:
35 P2 packets and 16 P1 packets, all consistent.

- P2: <https://emanual.robotis.com/docs/en/dxl/protocol2/>
- P1: <https://emanual.robotis.com/docs/en/dxl/protocol1/>
- CRC: <https://emanual.robotis.com/docs/en/dxl/crc/>

The e-Manual carries a banner saying it is moving to "ROBOTIS Docs" and will be removed. The test
vectors are therefore vendored into the repo with their source URL and anchor (R-10).

## R-1 Timeouts on a blocking transport

- **Decision**: `T: embedded_io::Read + Write + ReadReady` plus a crate-owned `Clock` trait.
  An exchange computes `deadline = clock.now() + timeout`, then loops: if `read_ready()` is true,
  `read` into a small stack chunk and feed the parser; stop on a complete packet or when the
  deadline passes. A `read` that returns `ErrorKind::TimedOut` is also treated as a timeout, so
  transports with their own OS timeout (serialport) stay safe.
  ```rust
  pub trait Clock { fn now_us(&mut self) -> u64; }
  impl<F: FnMut() -> u64> Clock for F { /* calls the closure */ }
  #[cfg(feature = "std")] pub struct StdClock; // Instant-based
  ```
- **Rationale**: `embedded_io::Read::read` (0.7) blocks until at least one byte arrives and has
  no "no data yet" return, so a plain `Read + Write` bound cannot be bounded in time. This is the
  only option where the bus, not the transport, owns the time limit (constitution: "timeouts are
  enforced by the bus"). `ReadReady` is implemented by rp235x-hal, esp-hal and the embassy
  buffered UARTs. A closure-friendly `Clock` keeps MCU setup to one line (DX first).
- **Alternatives rejected**:
  - Rely on the transport returning `TimedOut`: most blocking MCU UART `read`s block forever, so
    a missing actuator would hang the robot; the bus would not own the limit.
  - Own transport trait with `read(buf, deadline)` (dynamixel2-rs `main`): clean, but every user
    must implement our trait instead of reusing their HAL's embedded-io impls.
  - Iteration/byte budget: meaningless on top of a blocking `read`.
  - `fugit` / `embedded-time` clock types: force tick-rate generics into our API; `embedded-time`
    is unmaintained. Users wrap them in a closure instead.
- **Formerly a deviation**: SRS section 7 and the constitution's Technical Constraints said "any
  blocking `embedded_io::{Read, Write}`". This plan adds `ReadReady` and a `Clock`. Approved by
  the architect on 2026-10-26 (chat) and written into SRS 3.3, 4.4, §7, FR-1.1 and constitution
  1.1.0 the same day.

## R-2 embedded-io version and error mapping

- **Decision**: `embedded-io = { version = "0.7", default-features = false }`; the crate's `std`
  feature enables `embedded-io/std`. Transport errors are stored as `embedded_io::ErrorKind`
  (`Copy`, non-generic), so `Error` stays `Copy` and free of `T`.
- **Rationale**: 0.7.1 is current (2025-09-30); 0.6 is outdated. `ErrorKind` includes
  `TimedOut`. HALs still on 0.6 (e.g. stm32f4xx-hal 0.23) need a small wrapper; acceptable.
- **Alternatives rejected**: storing `T::Error` makes `Error` generic over the transport (poor DX).

## R-3 Errors

- **Decision**: all new errors are `thiserror` (no_std) enums, nested under the existing
  `Error` (core module, `src/error.rs`): `Error::Transport(TransportError)`,
  `Error::Protocol(ProtocolError)`, `Error::Status(StatusError)`, plus new `ValidationError`
  variants. See `data-model.md`.
- **Rationale**: matches SRS FR-6.4 categories and the existing `Error`/`ValidationError` style.
  Each category matched first, detail second.

## R-4 Buffers without allocation and without `generic_const_exprs`

- **Decision**: no packet buffer sized by bus capacity `N`.
  - **TX**: stream-encode straight to `T::write_all` through a small fixed stack chunk, updating
    CRC/checksum and doing P2 byte stuffing on the fly. P2's length field counts stuffed bytes,
    so the encoder makes one counting pass over the parameters before writing the header.
  - **RX**: a byte-at-a-time state machine writes status parameters straight into a
    caller-owned `&mut [u8]`; a reply longer than that slice is a protocol error.
- **Rationale**: `generic_const_exprs` is unstable, so a buffer of `14 + k·N` bytes cannot be
  expressed on stable. Streaming makes the question disappear and keeps stack use constant.
- **Alternatives rejected**: an extra `const BUF: usize` checked by `const { assert! }`
  (post-monomorphisation error, worse DX); sizing for the protocol maximum (several KiB of stack).

## R-5 Byte stuffing scope (P2)

- **Decision**: stuffing is applied to the instruction and parameter fields (everything after
  the length field, before the CRC). The CRC is computed over the stuffed bytes; the length field
  counts stuffed bytes. On receive, the CRC is checked on the raw bytes, then stuffing is removed.
- **Rationale**: the e-Manual gives the order (stuff → update length → CRC) and the receive rule,
  but only says "in a packet" for scope. The header must not be stuffed (it would no longer be a
  header); reserved = 0x00 and `Id` ≤ 0xFC cannot start the pattern. A stuffed-packet test vector
  is built from the e-Manual rule since the page has none.
- **Flag**: e-Manual is silent on exact scope; marked in code with a doc comment citing this.

## R-6 Half-duplex echo and stray bytes

- **Decision**: the parser resyncs on the header and skips garbage. On P2 it also skips any
  packet whose instruction byte is not 0x55 (Status), which discards an echoed instruction. P1
  status packets carry no instruction byte, so an echo cannot be told apart; transports that echo
  their own TX must strip it (transport module's job).
- **Rationale**: single-wire MCU UARTs echo TX; U2D2-style adapters do not.

## R-7 CRC-16 (P2) and checksum (P1)

- **Decision**: hand-written `const fn` that builds the 256-entry table at compile time
  (poly 0x8005, init 0, not reflected, no final XOR = CRC-16/UMTS a.k.a. BUYPASS) and a 3-line
  update. P1 checksum `!(sum of ID..last param) as u8`.
- **Rationale**: no dependency, 512 B flash, testable against the e-Manual ping
  (`FF FF FD 00 01 03 00 01` → `19 4E`) and the catalogue check value `0xFEE8`.
- **Alternatives rejected**: `crc` 3.4 crate: equivalent size and speed, one more dependency.

## R-8 Sealed protocol and absent instructions

- **Decision**: `mod sealed { pub trait Sealed {} }` plus `pub trait Protocol: sealed::Sealed`
  with zero-sized `P1` and `P2`. Shared operations (ping, read, write, sync write) are trait
  items; P2-only ones (sync read, broadcast ping) are inherent functions on `P2`, so they do not
  exist for `P1`.
- **Rationale**: `pub(crate) trait Sealed` trips the `private_bounds` lint, which CI denies.
  Inherent functions give the plainest compiler error later ("no function named ... for P1").
- **Alternatives rejected**: a `SupportsSyncRead` capability marker: extra indirection for two
  protocols.

## R-9 Testing approach

- **Byte-exact vectors**: every e-Manual example in the feature's instruction set is a unit test
  (encode equals bytes; decode equals fields). Out-of-scope examples (Reg Write, Action, Reboot,
  Bulk, Fast) are kept in the vendored file as CRC/checksum vectors only.
- **Corruption**: every single-byte change of each example status packet must be rejected.
- **Robustness (1M inputs)**: seeded xorshift64 loop (≈0.6 s per 1M in debug builds), mixing
  pure random streams with mutated valid packets (byte flips, truncation, spliced headers,
  oversize length), so deep parser paths are reached. The seed is printed on failure.
  proptest (≈26 s per 1M) is used only for encode→decode round trips at its default 256 cases.
- **Zero allocation**: structural (protocol code never names `alloc`/`std`; the no_std CI build
  catches any use) plus `src/protocol/no_alloc.rs`, a `#[cfg(test)]` module with a counting
  `#[global_allocator]` (thread-local counter) asserting zero allocations across exchanges.
- **Compile-fail (SC-005)**: deferred to the `bus` feature; trybuild 1.0 will be added then.
- **Fake transport**: a scripted `FakeLine` (cfg(test)) in the protocol module, implementing
  `Read + Write + ReadReady`, with a scripted `Clock`. Replaced by the `transport` module's mock
  when it ships.

## R-10 Vendored test vectors

- **Decision**: `src/protocol/vectors.rs` (`#[cfg(test)]`) holds every e-Manual example as
  `const` byte arrays, each with a doc comment giving URL + anchor + fetch date.
- **Rationale**: the e-Manual is being retired; tests must not depend on the network.

## R-11 Protocol 1 ping has no model number

- **Finding**: P1 ping reply is `FF FF ID 02 ERR CHK`, no parameters. P2 ping returns model
  number (LE u16) and firmware version.
- **Consequence**: this module's P1 ping returns only "present + error byte". Setup on P1
  (`bus` module) must follow with a read of the Model Number register. Recorded for the `bus`
  feature.

## R-12 Identity of P1 ID 253

- **Finding**: P1 allows IDs 0..=253; `Id` caps at 252 (SRS decision). No change: an actuator
  at ID 253 on P1 is unreachable by design (documented on `Id::MAX`).
