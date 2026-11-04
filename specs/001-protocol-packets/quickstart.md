# Quickstart: validating the protocol module

No hardware needed. All commands run from the repository root.

## Prerequisites

- Stable Rust (edition 2024) with the `thumbv7em-none-eabihf` target:
  `rustup target add thumbv7em-none-eabihf`
- `cargo-llvm-cov` for the coverage gate.

## 1. Byte-exact e-Manual vectors (SC-001, G11)

```sh
cargo test protocol::p2::tests
cargo test protocol::p1::tests
```

Expected: every vector in `src/protocol/vectors.rs` encodes to its exact bytes and decodes to its
fields. The vectors cite their e-Manual URL and anchor; see `research.md` for the pages.

## 2. Corruption is always rejected (SC-002)

```sh
cargo test every_single_byte_corruption
```

Expected: for every example status packet and every byte position, the corrupted packet yields a
typed error, never `Ok`.

## 3. Robustness, 1M inputs (SC-003)

```sh
cargo test protocol::tests::robustness
```

Expected: completes without panic in under a second (debug build); on failure the printed seed reproduces it with
`DXL_FUZZ_SEED=<seed>`.

## 4. Exchange behaviour on a scripted line (US4, G1–G5)

```sh
cargo test protocol::tests::exchange
```

Expected: stale RX bytes are drained before TX; a silent actuator times out at the scripted
clock deadline; sync write never reads; broadcast ping collects all replies in its window.

## 5. Fault decoding (US3, G6)

```sh
cargo test protocol::tests::status
```

Expected: P2 alert bit → hardware fault; each P2 error code → its `P2Code`; P1 multi-flag byte →
every flag reported; zero byte → no error.

## 6. Zero allocation and no_std (SC-004)

```sh
cargo test protocol::no_alloc
cargo clippy --no-default-features --features ax,mx,x-series,y-series,pro --target thumbv7em-none-eabihf
```

Expected: the counting allocator records 0 allocations; the no_std build has no warnings.

## 7. Full gate (as CI)

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
cargo xtask codegen && git diff --exit-code src/models.rs src/models/
cargo llvm-cov --workspace --ignore-filename-regex 'src/models(\.rs|/)' --fail-under-lines 70
```

Expected: all pass; coverage ≥ 70 % (SC-006).
