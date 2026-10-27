# Contract: protocol module API

Signatures are the contract for the `bus` module and for tests. Names may be polished during
implementation; shapes and guarantees may not. Visibility: items marked `pub` are re-exported at
the crate root; everything else is `pub(crate)`.

## Public items

```rust
/// Monotonic microsecond clock used to bound waits for replies.
pub trait Clock {
    fn now_us(&mut self) -> u64;
}
impl<F: FnMut() -> u64> Clock for F { /* calls self() */ }

#[cfg(feature = "std")]
pub struct StdClock { /* Instant origin */ }   // Default + Clock

pub trait Protocol: sealed::Sealed + Exchange { const BROADCAST: u8 = 0xFE; }
pub struct P1;
pub struct P2;

pub use error::{TransportError, ProtocolError, StatusError, StatusReport, P1Flags, P2Code};
```

DX notes:
- MCU: `let clock = || timer.now().ticks();` is a valid `Clock`.
- std: `StdClock::default()`.
- No new type parameter is visible to users in this feature; `Bus` wiring is the `bus` feature.

## Crate-internal exchange API

`Link`, `Pong` and `Exchange` are declared `pub` inside the private `protocol` module, so no one
outside the crate can name them. That lets `Protocol` carry `Exchange` as a supertrait: bus code
bounded only by `P: Protocol` (FR-1.1) calls `P::read` without a `private_bounds` lint.

```rust
pub struct Link<'a, T, C> {
    pub(crate) transport: &'a mut T,
    pub(crate) clock: &'a mut C,
    pub(crate) timeout: core::time::Duration,
}
// T: crate::Transport (embedded_io Read + Write + ReadReady), C: Clock

pub struct Pong { id: Id, model_number: Option<u16>, firmware: Option<u8>, fault: Option<StatusReport> }

pub trait Exchange: sealed::Sealed {            // implemented for P1 and P2
    fn ping<T, C>(link: &mut Link<T, C>, id: Id) -> Result<Pong, Error>;
    fn read<T, C>(link: &mut Link<T, C>, id: Id, addr: u16, out: &mut [u8]) -> Result<(), Error>;
    fn write<T, C>(link: &mut Link<T, C>, id: Id, addr: u16, data: &[u8]) -> Result<(), Error>;
    fn sync_write<'d, T, C, I>(link: &mut Link<T, C>, addr: u16, size: u16, entries: I)
        -> Result<(), Error>
    where I: Iterator<Item = (Id, &'d [u8])> + Clone;
}

impl P2 {
    pub(crate) fn sync_read<T, C, I, F>(link: &mut Link<T, C>, addr: u16, size: u16,
        ids: I, scratch: &mut [u8], on_reply: F) -> Result<(), Error>
    where I: Iterator<Item = Id> + Clone, F: FnMut(Id, Result<&[u8], Error>);

    pub(crate) fn broadcast_ping<T, C, F>(link: &mut Link<T, C>, on_reply: F)
        -> Result<(), Error>
    where F: FnMut(Pong);
}
// P1 has no sync_read and no broadcast_ping: they do not exist (FR-004, SC-005).
```

## Guarantees

| # | Guarantee | Spec |
|---|---|---|
| G1 | RX is drained before every TX. | FR-006 |
| G2 | `ping`/`read`/unicast `write` wait at most `timeout` for their reply, then `Transport(Timeout { id: Some(id) })`. | FR-007 |
| G3 | `sync_write` and broadcast `write` never call `read`. | FR-008 |
| G4 | `sync_read` calls `on_reply` exactly once per ID, in order; a missing reply yields `Err(Timeout)` for that ID and the next ID gets its own deadline. A reply from an ID later in the list gives every ID before it `Err(Timeout)` and is delivered to its own ID. A late reply from an ID that already timed out is discarded, not handed to the next ID. `scratch` must hold `size` bytes, else `Validation(DataLength)`; `size > 65531` is `Validation(PacketTooLarge)`; nothing is sent in either case. Otherwise returns `Err` only on transport I/O failure. | FR-007, spec edge case |
| G5 | `broadcast_ping` collects every valid reply until `timeout` elapses; returns `Ok(())` even with zero replies. A reply with its error byte set is still reported, with the decoded byte in `Pong::fault`. | US4-4, FR-009 |
| G6 | Every status packet's error byte is decoded; a non-zero byte returns `Status(StatusError)` naming the ID (for `sync_read`, delivered to that ID's callback). It wins over a wrong length. On a unicast `read`, `out` keeps the data of an error reply that carried `out.len()` bytes (so Hardware Error Status can be read while the P2 alert bit is set); any other error reply leaves `out` zeroed. | FR-009 |
| G7 | `write`/`sync_write` with params exceeding the length field return `Validation(PacketTooLarge)`; P1 with `addr > 0xFF` returns `Validation(AddressOutOfRange)`. Nothing is sent. | FR-010 |
| G8 | `read` with `out.len()` too large for a reply length field returns `Validation(PacketTooLarge)` before sending. | FR-010 |
| G9 | No allocation, no panic, for any bytes received. A header with an impossible length, or a P2 status LEN longer than the expected reply can be with worst-case stuffing, is skipped (resync), so one corrupted LEN never swallows the replies after it; if no valid packet follows before the deadline the error is `Protocol(Malformed)` instead of `Timeout`. A `read` of 0 bytes (end of stream) is `Transport(Io(NotConnected))`. | FR-011 |
| G10 | Status Return Level is never touched. | FR-012 |
| G11 | Encoded bytes equal the e-Manual examples exactly; decoded fields equal them. | FR-001..003, SC-001 |
