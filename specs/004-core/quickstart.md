# Quickstart: validating the core module

**Feature**: [spec.md](spec.md) | **API**: [contracts/core-api.md](contracts/core-api.md) |
**Model**: [data-model.md](data-model.md)

## Prerequisites

- Latest stable Rust (dev uses 1.97.1) and the `thumbv7em-none-eabihf` target.
- `cargo-llvm-cov` for the coverage gate.
- core-1: protocol PR #7 merged, with `ProtocolVersion` and its root re-export (done: merged into
  `main` as f338e2a). core-2: xtask X1 merged.
- No hardware. Core does no I/O.

## 1. Gates every core PR runs

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
cargo clippy --no-default-features --features ax,mx,x-series,y-series,pro --target thumbv7em-none-eabihf -- -D warnings
RUSTFLAGS="-D warnings" cargo build --lib --no-default-features      # no family enabled
cargo xtask codegen && git diff --exit-code -- src/models.rs src/models
cargo llvm-cov --workspace --ignore-filename-regex 'src/models(\.rs|/)' --fail-under-lines 70
```

Expected: all pass. The no-family line fails on `dev` today ("associated function `new` is never
used"); it passes from core-1 on (research R-4). The codegen line shows no diff: core never
changes generated output.

core-1 only, protocol touch (research R-19), first: with only PR #7's `expect(dead_code)` on
`mod protocol;` deleted, both clippy lines report exactly one error, "associated function
`broadcast_ping` is never used"; any other dead item stops core-1. After `P2::broadcast_ping` and
its tests are deleted, all gates above pass as before and `rg -n "broadcast_ping" src/` finds
nothing.

core-1 only, simulate X1: in a scratch copy, change the generated code so it no longer calls
`Register::new` (build each register as a struct literal). `cargo clippy --all-targets -- -D
warnings` stays clean, as does the no-family line. This proves the shim's `allow` holds after X1
regenerates (research R-4).

## 2. Scenario checks, core-1 (all in `cargo test`)

| Spec item | Test lives in | Expected |
|---|---|---|
| US1-1 | `src/units/position.rs` | XM430 scale (2048, 0..=4095, ±π): `to_raw(ZERO)` is 2048 |
| US1-2, SC-001 (per scale) | `position.rs` | every tick of each of the 8 table scales (research R-6) survives `to_position` → `to_raw` |
| US1-3, US1-4, SC-002 | `src/actuator.rs` (`Cache`) | limits 1000..=3000 on Id 3: 3001 and 999 give `OutOfLimits` naming 3, the tick and both limits; 1000 and 3000 pass |
| US1-5 | `position.rs` | NaN, +∞, −∞ give `NotFinite`; `proptest` over any `f32` never panics |
| US1-6, FR-007 | `actuator.rs` | `Cache::new(id).check_goal(_)` gives `LimitsUnknown` |
| US1-7, FR-021 | `actuator.rs` (`LimitMode`) | limits 1000..=3000 and mode `Unbounded`: 5000, 999 and `i32::MIN` give `Ok`; with limits unknown too |
| US1-8, FR-021 | `actuator.rs` | mode `NoPosition`: 2000 (inside the limits) and any tick give `NotPositionMode { id }`, limits known or not |
| FR-021 | `actuator.rs` | `Cache::new(id)` is `Bounded`; `record_mode` leaves the limits as they were |
| US1-9, FR-022, FR-023, SC-009 | `actuator.rs` (`ModeValues`) | hand-built fixtures with the X, RH-P12-RN and XL-320 lists of research R-17 (core holds no list), all 256 values: a value in a list gives that list's mode, every other one `None`; value 5 is `Unbounded` with the X lists and `Bounded` with the RH-P12-RN lists; empty lists give `None` for every value |
| US1-10, FR-022 | `actuator.rs` | `from_angle_limits`: (0, 0) → `NoPosition`; (4095, 4095) → `Unbounded`; (0, 1023) and (1, 1023) → `Bounded`; `AngleLimits.register()` is `None` and `limit_mode(_)` is `None` |
| FR-005 edge | `actuator.rs` | `record_limits(3000, 1000)` gives `InvertedLimits` and leaves limits `None` |
| AX edge | `position.rs`, `register.rs` | AX scale: an angle more than 512 ticks below zero gives a negative tick; encoding it in 2 bytes gives `DoesNotFit` |
| US2-3…5, edge cases | `actuator.rs` (`TorqueState`) | default `Off`; every cell of the transition table in data-model.md; `fault()` from any state is `Faulted`; `enable` from `Faulted` is `ActuatorFaulted` |
| SC-008 | `actuator.rs` | `size_of::<TorqueState>() == 1`, `size_of::<LimitMode>() == 1`, `size_of::<Cache>() == 16`; `Cache` has fields `id`, `mode` and `limits` only |
| US4-1 | `src/register.rs` | 2500 into a 4-byte register gives `C4 09 00 00` |
| US4-2 | `actuator.rs` | raw and typed goals hit the same `Cache::check_goal` error |
| US4-3, FR-013 | `register.rs` | 70 000 into 2 bytes and −1 into 1 byte give `DoesNotFit`; `proptest`: `decode(encode(x)) == x` when it fits |
| decode sign | `register.rs` | `FF FF` (size 2) is 65535; `FF FF FF FF` (size 4) is −1; wrong slice length is `DataLength` |
| US3-1…8, SC-004, FR-017 | `src/error.rs` | each variant's message equals the table in contracts/core-api.md; each converts into `Error` under its category; `Error: Copy + Eq` |
| SC-005 | doctest on `Position` | `goal + Position::from_radians(0.1)` compiles and moves the goal |
| FR-011 shape | `actuator.rs` | `ActuatorKind::Xm430W350(Xm430W350::new(id))` still answers `id`, `model_number`, `name` (D2) |
| §2.8: `Describe` never re-exported | `compile_fail,E0603` doctest on `Actuator` | `use dxl_rs_v2::actuator::Describe;` fails (the `actuator` module is private) |
| module-interfaces §1 (protocol touch, research R-19) | `src/protocol/p2.rs`, `src/protocol/no_alloc.rs` | `P2::broadcast_ping` is gone; `scan_fails_on_the_transport` gives `Err(Transport(Io(BrokenPipe)))`; the no-alloc test counts 5 replies and allocates nothing |

## 3. Scenario checks, core-2

| Spec item | Test lives in | Expected |
|---|---|---|
| SC-003, US2-7 | `src/actuator.rs` | one generic check runs on every variant found through `ActuatorKind::new` (all model numbers, both protocols, every firmware when `firmware_register` is `Some`); the answers of the enum equal those of the inner model |
| SC-001 (per model) | `actuator.rs` | every tick in each model's `min_raw..=max_raw` survives the round trip with that model's `position_scale()` |
| US2-1 | `actuator.rs` | XM430-W350 `data().goal_position` is (116, 4); AX-12A is (30, 2) |
| US2-6 | `actuator.rs` | AX-12A `data().limits.min()` / `max()` are CW (6) / CCW (8); XM430 are Min (52) / Max (48) |
| US2-2 | `actuator.rs` | a new model's `position_limits_raw()` is `None`; after `record_limits` it is `Some(min..=max)`, and stays so after `record_mode` to `Unbounded` or `NoPosition` (the EEPROM limits in every mode, 2026-10-28) |
| FR-022 (per model) | `actuator.rs` | every variant: the mode register (if any) is 1 byte, RW, EEPROM, and its three lists are disjoint; `AngleLimits` only with CW/CCW limits. `Register` at 11 with lists (bounded / unbounded / no position) `[3]` / `[4, 5]` / `[0, 1, 16]` on XM430-W350 and MX-64(2.0), `[3]` / `[4]` / `[0, 1, 16]` on H54-200-S500-R(A), `[5]` / `[]` / `[0]` on RH-P12-RN, `[2]` / `[]` / `[1]` on XL-320 (Control Mode); at 33 with `[3]` / `[]` / `[0, 1]` on YM070-210-M001; AX-12A: `AngleLimits` |
| Sealed trait (FR-011, research R-12) | `compile_fail,E0277` doctest on `Actuator` | `struct Mine; impl dxl_rs_v2::Actuator for Mine {}` fails: `Mine: Describe` is not satisfied. This, not the E0603 example, proves users cannot implement `Actuator` |
| FR-012 | review in the docs pass (T055) | the public `Actuator` declares only `id`, `model_number`, `name`, `position_scale`, `position_limits_raw`, `position_limits` (contracts/core-api.md); no register is reachable through it |
| FR-016 | `error.rs` | `Error::UnsupportedModel` no longer exists; the setup variant is matched as `Error::Setup(SetupError::UnsupportedModel { .. })` |

## 4. From a user crate

In a scratch crate depending on this one (default features):

1. Write `fn describe<A: dxl_rs_v2::Actuator>(a: &A) -> (u8, &'static str)` returning
   `(a.id().get(), a.name())`.
2. Build a `Position` with `Position::from_radians(0.5)`, add `Position::from_radians(0.1)`, print
   `radians()`.
3. Match an `Error` as `Error::Setup(SetupError::Missing { id })`, with a `_` arm.

Expected: it compiles with no import of any crate-internal item, and the `_` arm is required
(`#[non_exhaustive]`). Repeat with `default-features = false, features = ["x-series"]` for a
`no_std` library crate: same result.
