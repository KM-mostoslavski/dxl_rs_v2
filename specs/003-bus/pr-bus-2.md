Stacked on #30 (bus-1), which is stacked on #8 (core-1), #9 (tables T1), #10 (ci), #25 (xtask X1), #26 (transport macOS test fix) and #27 (core-2) through the `integration` branch; its diff shrinks to the bus-2 files once they merge.

## What

bus-2 of the bus module (spec 003, tasks T037–T044, user story US2): find what is on the wire without listing it.

- `Bus::autoscan()`: Protocol 2 sends one broadcast Ping and collects every reply within `Config::scan_timeout`; Protocol 1 pings IDs 0..=252 in turn. Each actuator found is set up like `declare` does (model, firmware variant, EEPROM limits, operating mode), in `Id` order, with torque `Off`. Available without `std` (FR-009).
- `SerialBus<P, const N: usize = 32>` = `Bus<P, SerialTransport, StdClock, N>`, with `SerialBus::<P2>::open(path, baud)` (FR-006): opens the port with `StdClock` and `Config::new()`, sends nothing.

## Why

SRS steps 4 and 8: a desktop user goes from a port name to a set-up bus in two calls, and an embedded user can scan without `std`. With bus-1 this drives a chain on either protocol, e.g. RX-28s on Protocol 1 (`AngleLimits` mode source, P1 sweep, P1 Model Number read).

## How

- `Wire::scan` uses `Config::scan_timeout` and is never retried (research R-5).
- `setup::autoscan`, generic over `Slots` like `declare`: replies go into a stack `heapless::Vec<Pong, N>`. The first of these fails Setup before any read is sent: a corrupted reply (`Error::Protocol`), a reply reporting an error (`Error::Status`), a second reply with the same `Id` (`DuplicateId`), a reply past capacity (`TooManyActuators`). Then: sort by `Id`, Model Number read on P1 (P2 uses the pong), and bus-1's `init_one` (limits and mode). Any error leaves the list empty (FR-015).
- Tests (14 new, byte-exact on `MockTransport`): P2 scan of three out-of-order pongs; empty P1 sweep within `253 × scan_timeout + 10 ms` and empty P2 scan within `scan_timeout + 1 ms` on the test clock (SC-007); P1 sweep where IDs 1 and 5 (RX-28) answer; XL330-M077 firmware 52 / 53 picks its table; XL-320 found on P1 is `UnsupportedModel { protocol: P1, feature: None }`; with `pro` off, a PRO pong is `UnsupportedModel { feature: Some("pro") }`; corrupted reply, two pongs with one ID, pong with alert, past capacity; unknown mode value. `SerialBus::open` on a pty at 1 Mbaud writes nothing (Unix; ignored on macOS like the transport's pty tests).
- `Config::scan_timeout` and `Bus::autoscan` document the scan timing (research R-11): one P2 window, a P1 sweep of about 2.5 s at the default, 14-byte P2 ping replies sharing one wire.

## Architecture

Follows `specs/003-bus` and module-interfaces §6.

- Plan deviation 4: `ValidationError::DuplicateId` is also used for two scan replies with one `Id`.
- Autoscan reads the operating mode exactly like `declare` (same `init_one`).
- Shared-file touch: `src/lib.rs` (`#[cfg(feature = "std")] pub use bus::SerialBus;` only).
- Test fixtures gained `xm430_p2_reads` / `angle_p1_reads` (the Setup reads after a ping) so scanned actuators can be scripted.

## Checklist
- [x] `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass (also `--features mock` and `--all-features`)
- [x] `cargo test` passes; line coverage ≥ 70 % (`cargo llvm-cov --workspace --features mock`: 98.4 % total; bus files 93.1–100 %)
- [x] `no_std` build passes (`--no-default-features --features ax,mx,rx,x-series,y-series,pro,mock --target thumbv7em-none-eabihf`, clippy `-D warnings`), so `autoscan` builds without `std`; every ci feature-set leg passes
- [x] `cargo xtask codegen` leaves `src/models.rs` and `src/models/` unchanged
- [x] Public methods documented (`cargo doc --no-deps` with `-D warnings`, all features and no features)
- [x] Docs updated if behaviour or setup changed
- [x] Family-off leg: `cargo test --no-default-features --features std,mock,ax,mx,rx,x-series,y-series` (runs the `pro`-off test); per-feature lib tests for every family
- [x] Doc tests with `RUSTC_BOOTSTRAP=1`: all features, defaults, `std` off
- [ ] Manual hardware check (quickstart §4): not run
