## What
Adds the `protocol` module: Dynamixel Protocol 1 and Protocol 2 packet encoding, status parsing
and exchanges (ping, read, write, sync write; sync read and broadcast ping on P2 only), with
reply waits bounded by a new `Clock` trait.

## Why
Every byte the crate sends or receives goes through this module; it is step 1–2 of the SRS
delivery order and unblocks the `bus` module. Spec: `specs/001-protocol-packets/spec.md`.

## How
- Sealed `Protocol` trait with zero-sized `P1`/`P2`. P2-only instructions are inherent functions
  on `P2`, so they do not exist for `P1`.
- Packets are streamed through a 32-byte stack chunk (CRC/checksum and P2 byte stuffing on the
  fly); replies are parsed byte by byte into a caller-owned slice. No buffer depends on the bus
  capacity `N`; nothing allocates.
- Built on the merged `transport` module: exchanges take any `crate::Transport`, and tests use
  `MockTransport`/`MockTime` (the module's own `FakeLine` is gone).
- `Exchange` is a sealed supertrait of `Protocol`, so bus code bounded only by `P: Protocol`
  reaches ping/read/write (FR-1.1) without a `private_bounds` lint.
- Waits poll `ReadReady` against a `Clock` deadline; `ErrorKind::TimedOut` from the transport
  also counts as a timeout. Any `FnMut() -> u64` closure is a `Clock`; `StdClock` under `std`.
- Errors: new `thiserror` categories `Transport`, `Protocol`, `Status` under `Error`;
  `StatusError::is_hardware_fault()` for the bus's fault state. Every public field is documented,
  and `#![warn(missing_docs)]` now enforces it.
- Review follow-ups (`review-2026-10-27.md`, tasks T046–T053):
  - a set error byte wins over a wrong length, and a P2 alert reply keeps its data in `out`, so
    Hardware Error Status can be read while the alert is latched;
  - a corrupted P2 LEN no longer swallows the replies after it;
  - sync read discards a late reply from an ID that already timed out;
  - broadcast ping reports a set error byte in `Pong::fault`;
  - P1 error bits outside the seven known flags are displayed, not shown as "no error".
- Tests: every e-Manual example packet vendored in `src/protocol/vectors.rs` (URL + anchor,
  fetched 2026-10-26; the e-Manual is being retired); byte-exact encode/decode; every
  single-byte corruption rejected; 1M random/mutated inputs; a counting global allocator
  proving zero allocations.

### Additions for the bus
From `specs/ideas/module-interfaces.md` §5 (tasks T054–T057):
- `ProtocolVersion { P1, P2 }` (Display "Protocol 1" / "Protocol 2"), re-exported at the crate
  root, and `Protocol::VERSION` on `P1` and `P2`.
- `Exchange::scan`: P2 sends one broadcast ping; P1 pings IDs 0..=252 in turn, each bounded by
  the link timeout. A set error byte gives a `Pong` with `fault` (on P1 the `Status` error of the
  ping becomes that `Pong`). Corrupted replies reach the callback as `Err` instead of being
  skipped; `P2::broadcast_ping` now delegates to `scan` and keeps skipping them. `Err` only on
  transport failure.
- `Exchange::read_many`: P2 delegates to `P2::sync_read` (kept as the P2-only explicit entry);
  P1 sends one Read per ID. One callback per ID, in order; per-ID errors go to that ID; `Err`
  only for an invalid request (before sending) or a transport failure.
- `#[diagnostic::on_unimplemented]` on `Clock`.

## Architecture
Follows docs/ARCHITECTURE.md and the SRS, with these deviations:
- **Transport bound is `Read + Write + ReadReady` plus a `Clock`**: a plain `embedded_io::Read`
  blocks without bound, so the bus could not enforce timeouts. Now stated in SRS 3.3, 4.4, §7
  and FR-1.1, constitution 1.1.0 and docs/ARCHITECTURE.md (amended 2026-10-26; the
  ARCHITECTURE.md change ships with the transport PR).
- **Cross-module edits**: `src/error.rs` and `src/lib.rs` (core) and `Cargo.toml` (`proptest`
  dev-dependency). They are prerequisites of this module. Also a one-line fix to the transport
  contract (`Ok(0)` writes are `TimedOut`, not `WriteZero`).
- P2 byte-stuffing scope (instruction + parameters) is not stated by the e-Manual; documented
  on `Stuffing` in `src/protocol/p2.rs`.
- `sync_read` takes a caller-owned `scratch: &mut [u8]` for one reply (no internal buffer), and
  `ValidationError::DataLength` was added for sync-write entries / scratch not matching `size`.
  Contract and data model updated.
- SC-005's compile-fail test moves to the `bus` feature (first public surface for sync read).
- Noted for `bus`: a Protocol 1 Ping reply carries no model number; P1 setup must read the
  Model Number register.
- Review items left for the architect: sync read still drops the data of an error reply (the
  callback type would change), and where the bus keeps its clock is open for spec 003. The P1
  parser still trusts its one-byte LEN (at most 253 bytes swallowed, no multi-reply P1 instruction).

## Checklist
- [x] `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass
- [x] `cargo test` passes; line coverage ≥ 70 % (`cargo llvm-cov`): 96.4 %
- [x] `no_std` build passes (`--no-default-features --features ax,mx,x-series,y-series,pro --target thumbv7em-none-eabihf`)
- [x] `cargo xtask codegen` leaves `src/models.rs` and `src/models/` unchanged
- [x] Public methods documented
- [x] Docs updated if behaviour or setup changed (spec contract, data model and tasks; the C4
  protocol components move to `docs/c4/` with the ov tooling PR)
