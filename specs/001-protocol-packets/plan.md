# Implementation Plan: Protocol Packets (P1 and P2)

**Branch**: `001-protocol-packets` | **Date**: 2026-10-26 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/001-protocol-packets/spec.md`

## Summary

Build the `protocol` module: P1 and P2 instruction encoding and status decoding, byte-exact with
the ROBOTIS e-Manual, behind a sealed `Protocol` trait with zero-sized `P1`/`P2` markers. Packets
are streamed (no buffer sized by bus capacity), replies are parsed by a resyncing state machine
into caller-owned slices, and every wait is bounded by a crate-owned `Clock` polled together with
`embedded_io::ReadReady`. Every error is a `thiserror` enum under the crate `Error`.

## Technical Context

**Language/Version**: Rust stable, edition 2024 (MSRV ≥ 1.87 via heapless 0.9 later; this feature
needs ≥ 1.81 for embedded-io 0.7)

**Primary Dependencies**: `embedded-io` 0.7 (no default features; `std` feature forwards
`embedded-io/std`), `thiserror` 2 (existing, no_std). Dev: `proptest` 1 (round trips only).

**Storage**: N/A

**Testing**: `cargo test` (unit tests in `src/protocol/**`, e-Manual vectors vendored in
`src/protocol/vectors.rs`), test `src/protocol/no_alloc.rs` with a counting allocator (cfg(test)),
`cargo llvm-cov` gate ≥ 70 %.

**Target Platform**: desktop/SBC (`std`) and `thumbv7em-none-eabihf` (`no_std`, no heap).

**Project Type**: library crate (workspace with `xtask/`).

**Performance Goals**: CRC/encode cost negligible next to UART time at 1 Mbaud; constant stack
use per exchange (small fixed chunk, no `N`-sized buffers). Latency targets deferred (SRS NFR-5).

**Constraints**: no allocation, no panic on any input, static dispatch only, blocking I/O, timeout
owned by the caller of the exchange (the bus).

**Scale/Scope**: 2 protocols; 5 instructions (ping, read, write, sync write; sync read + broadcast
ping on P2); up to 253 IDs per sync instruction.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Check | Status |
|---|---|---|
| I. no_std and Heap-Free | Streaming TX, caller-slice RX, no `alloc`/`std` in `src/protocol/**` except `StdClock` under `cfg(feature = "std")`; `src/protocol/no_alloc.rs`. | PASS |
| II. Absent Over Failing | `sync_read` / `broadcast_ping` are inherent on `P2` only; `Protocol` sealed. | PASS |
| III. Nothing Hidden | No retries inside the module; RX drained before TX; every error byte decoded; broadcast writes never read; SRL untouched. | PASS |
| IV. e-Manual Is the Only Source | All facts fetched online 2026-10-26; vectors vendored with URL + anchor; stuffing scope gap flagged (R-5). | PASS |
| V. Committed, Data-Driven Codegen | Not touched. | N/A |
| VI. Typed, Static, Panic-Free API | `Id` in every signature; `thiserror` errors; generics only, no `dyn`. | PASS |
| VII. DX First | Closure is a `Clock`; `StdClock` under std; `Duration` timeouts; non-generic `Copy` errors. | PASS |
| Technical Constraints: transport is `embedded_io::{Read, Write, ReadReady}`, bus holds a `Clock` (constitution 1.1.0) | Plan requires exactly that. | PASS (amended 2026-10-26; was a deviation) |
| Workflow: one module per PR | Touches `src/error.rs`, `src/lib.rs` (core) and `Cargo.toml` (ci). | **DEVIATION**: see Complexity Tracking. |

Post-design re-check (after Phase 1): unchanged, all PASS except the recorded cross-module deviation.

## Project Structure

### Documentation (this feature)

```text
specs/001-protocol-packets/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── protocol-api.md
├── checklists/
│   └── requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
src/
├── lib.rs               # + mod protocol; re-exports (core, cross-module)
├── error.rs             # + Transport/Protocol/Status errors, new Validation variants (core)
├── protocol.rs          # Protocol (sealed), Exchange, Link, Pong, drain + receive loop
└── protocol/
    ├── clock.rs         # Clock trait, closure impl, StdClock (std)
    ├── crc.rs           # const-fn CRC-16 table + update (P2)
    ├── p1.rs            # P1 encode, checksum, status parser, Exchange impl
    ├── p2.rs            # P2 encode + stuffing, status parser + unstuffing, Exchange impl,
    │                    # inherent sync_read / broadcast_ping
    ├── no_alloc.rs      # #[cfg(test)] counting #[global_allocator], zero allocations
    └── vectors.rs       # #[cfg(test)] vendored e-Manual packets, each with URL + anchor
Cargo.toml               # + embedded-io, std feature forwarding, proptest dev-dep (ci)
```

**Structure Decision**: Rust 2018+ layout (`protocol.rs` + `protocol/`, no `mod.rs`), all under
the module's SRS paths `src/protocol.rs src/protocol/**`, plus the three cross-module files above.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| Edits to `src/error.rs`, `src/lib.rs` (core) and `Cargo.toml` (ci) in a protocol PR | New error categories and the dependency are prerequisites of this module; splitting them into separate PRs would merge dead code first. | A core-only PR first adds unused error variants (dead code, `-D warnings` noise) with no tests exercising them. Called out in the PR's Architecture section. |
