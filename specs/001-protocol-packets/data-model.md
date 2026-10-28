# Data Model: Protocol Packets (P1 and P2)

Facts and constants come from the e-Manual pages listed in `research.md`.

## Protocol markers

| Entity | Kind | Notes |
|---|---|---|
| `P1` | zero-sized, `pub` | Header `FF FF`; 1-byte length = params + 2; checksum `!(ID + LEN + INST/ERR + params) as u8`. Addresses and data lengths are 1 byte. |
| `P2` | zero-sized, `pub` | Header `FF FF FD 00`; 2-byte LE length = params + 3 (instruction) / + 4 (status), counting stuffed bytes; CRC-16 LE over header..params. Addresses and lengths 2 bytes LE. |
| `Protocol` | sealed `pub` trait | Implemented by `P1`, `P2` only. `BROADCAST: u8 = 0xFE`; `VERSION: ProtocolVersion`. |
| `ProtocolVersion` | `pub enum { P1, P2 }` | `Debug, Clone, Copy, PartialEq, Eq, Hash`; Display "Protocol 1" / "Protocol 2". The runtime value of a `Protocol`, for messages and checks. |

## Instruction codes in scope

| Instruction | P1 | P2 | Broadcast | Reply |
|---|---|---|---|---|
| Ping | 0x01 | 0x01 | P2 only (broadcast ping) | one per actuator; P1: no params; P2: model number (u16 LE), firmware (u8) |
| Read | 0x02 | 0x02 | never | one, `len` data bytes |
| Write | 0x03 | 0x03 | allowed, no reply | one when unicast |
| Sync Write | 0x83 | 0x83 | always (ID 0xFE) | none |
| Sync Read | absent | 0x82 | always (ID 0xFE) | one per listed ID, in request order |
| Status | n/a | 0x55 | n/a | P2 status packets carry it; P1 status packets have no instruction byte |

Out of scope (vendored as CRC/checksum vectors only): Reg Write 0x04, Action 0x05, Factory Reset
0x06, Reboot 0x08, Clear 0x10, Control Table Backup 0x20, Bulk Read 0x92, Bulk Write 0x93,
Fast Sync Read 0x8A, Fast Bulk Read 0x9A.

## Exchange context

| Field | Type | Rule |
|---|---|---|
| transport | `&mut T`, `T: crate::Transport` (`Read + Write + ReadReady`) | RX drained (`read_ready` + `read` until not ready) before every TX (FR-006). |
| clock | `&mut C`, `C: Clock` | `now_us()` monotonic microseconds. |
| timeout | `core::time::Duration` | Per reply. Unicast: one deadline. Sync read: one deadline per expected reply. Broadcast ping and P2 scan: one window, all replies inside it collected. P1 scan: one deadline per ID pinged. Broadcast writes: no read at all (FR-008). |

## Values returned

- **`Pong`**: `id: Id`, `model_number: Option<u16>`, `firmware: Option<u8>` (`None` on P1, R-11),
  `fault: Option<StatusReport>` (a set error byte on a scan or broadcast ping reply; always `None`
  on unicast ping, which returns `Error::Status` instead).
- **Scan replies**: delivered one at a time to a caller callback as `Result<Pong, Error>`: one `Ok`
  per actuator found (P1 in ID order), one `Err` per corrupted reply. Silent IDs produce nothing.
- **Read data**: written into the caller's `&mut [u8]`; its length is the requested length. After
  `Error::Status` it still holds the reply's data when the reply had that length, else zeroes.
- **Sync read and `read_many` replies**: delivered one at a time to a caller callback as
  `(Id, Result<&[u8], Error>)`, once per ID, in request order. Strict/partial policy belongs to
  the `bus` module.

## Errors (all `thiserror`, no_std, `Copy`, `#[non_exhaustive]`)

Added to the existing `Error` in `src/error.rs` (core; cross-module change flagged in the PR):

```text
Error
├── Validation(ValidationError)        existing; new variants:
│     ├── InvalidId(u8)                existing
│     ├── AddressOutOfRange { addr: u16 }        P1 address > 0xFF
│     ├── PacketTooLarge { len: usize, max: usize }  params exceed the length field
│     └── DataLength { expected: usize, got: usize } sync entry or sync-read scratch ≠ size
├── UnsupportedModel { .. }            existing
├── Transport(TransportError)          new
│     ├── Io(embedded_io::ErrorKind)   any transport error except TimedOut
│     └── Timeout { id: Option<Id> }   deadline passed (or TimedOut); id = who we waited for
├── Protocol(ProtocolError)            new
│     ├── Checksum { id: u8 }          P1 checksum mismatch
│     ├── Crc { id: u8 }               P2 CRC mismatch
│     ├── WrongId { expected: Id, got: u8 }
│     ├── WrongLength { expected: usize, got: usize }  reply payload ≠ requested
│     └── Malformed                    only impossible headers seen before the deadline;
│                                      also a P2 scan reply from an ID above 252
└── Status(StatusError)                new
      StatusError { id: Id, report: StatusReport }
      StatusReport
      ├── P1(P1Flags)                  bit set over: InputVoltage(0) AngleLimit(1) Overheating(2)
      │                                Range(3) Checksum(4) Overload(5) Instruction(6)
      └── P2 { alert: bool, code: Option<P2Code> }
            P2Code: ResultFail(1) Instruction(2) Crc(3) DataRange(4) DataLength(5)
                    DataLimit(6) Access(7) Unknown(u8)
```

- `StatusError::is_hardware_fault()`: P2 `alert`; P1 any of Overload, Overheating, InputVoltage.
  The `bus` module uses it to set fault state and torque off (SRS FR-6.2).
- `P1Flags::iter()` yields every set flag (acceptance: "every set flag is reported").
- Display strings name the actuator ID and every reported condition.

## Parser states (receive, per protocol)

```text
Hunt ──header seen──▶ Fields (ID, LEN[, INST]) ──▶ Error byte ──▶ Params ──▶ Integrity ──▶ Done
  ▲                        │ impossible LEN / P2 INST ≠ 0x55             │ mismatch
  └──────── resync ────────┴───────────────────────────────────────────┘ (error returned
                                                                         only if the deadline
                                                                         passes without a
                                                                         valid packet)
```

- P2 receive tracks the stuffing pattern to drop the extra `FD` from params, while the CRC runs
  over the raw bytes (R-5).
- A complete, integrity-valid packet with the wrong ID or wrong payload length is an error
  immediately (not a resync): the line is in an unexpected state.
- A checksum/CRC mismatch on a packet with plausible fields is returned as an error (SC-002), not
  silently resynced.
