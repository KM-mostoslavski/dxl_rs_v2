# Quickstart: validating xtask codegen

**Feature**: [spec.md](spec.md) | **Contracts**: [contracts/codegen-cli.md](contracts/codegen-cli.md),
[contracts/generated-api.md](contracts/generated-api.md)

## Prerequisites

- Latest stable Rust (dev: 1.97.1) with `rustfmt` and `clippy`, plus the
  `thumbv7em-none-eabihf` target.
- `cargo-llvm-cov` for the coverage gate.
- Merged first: core-1 (with the contract §2.4b items `ModeSource { Register { register, values },
  AngleLimits }`, `ModeValues { bounded, unbounded, no_position }`, `LimitMode`,
  `ModelData::mode`, `Cache::record_mode`) and tables T1, `operating_modes` included (contract
  §7). Protocol PR #7
  (`ProtocolVersion`, contract §5) is already merged into `main`. Before T1, only the xtask
  fixture tests (§2) can pass; codegen on the real tables fails on the missing
  `protocols`/`source`/`access`/`area`/`operating_modes`. That is the intended FR-012 behaviour.

## 1. Gates every PR runs

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo clippy --no-default-features --features ax,mx,x-series,y-series,pro --target thumbv7em-none-eabihf -- -D warnings
cargo llvm-cov --workspace --ignore-filename-regex 'src/models(\.rs|/)' --fail-under-lines 70
cargo xtask codegen && git add --intent-to-add src/models.rs src/models && git diff --exit-code -- src/models.rs src/models
```

Expected: all pass, and the last line shows no diff (SC-001).

## 2. xtask tests (`cargo test -p xtask`)

| Spec item | Expected |
|---|---|
| US4 AS1, SC-001 | running `run` twice on a temp root leaves every file byte-identical with an unchanged mtime |
| US4 AS2, SC-007 | changing one row address in one fixture table rewrites exactly that model's file |
| US4 AS3, SC-006 | one fixture per check of [data-model.md §3](data-model.md#3-checks-all-before-any-write): exit status error, a message naming the file, and sentinel files under `src/models*` unchanged |
| US4 AS4, FR-004 | unmapped fixture tables give one warning each, and the run succeeds |
| US4 AS5, FR-016 | a rendered model file starts with the `@generated` header and `Source:` line, and its docs carry name, number, firmware range per protocol, source table and the Unverified/Verified line |
| US3 AS3, FR-005 | family `x_series` fails, naming the model number and the bad family |
| SC-002 | the real-tables test: every role of all 69 mapped tables, mode source included, matches an independent name lookup and the rendered `reg(...)` line |
| US1 AS1–AS4 | real-tables spot checks: XM430-W350 Goal Position `reg(116, 4, RW, Ram)`; AX-12A `CwCcw` at 6/8; AX-12A `hardware_error: None`; scale from `type_info`, radians as `{:?}` of the table value |
| US1 AS6, FR-019 | real-tables spot checks: XM430-W350 `mode: ModeSource::Register { register: Self::OPERATING_MODE, values: ModeValues { .. } }` with `OPERATING_MODE` at `reg(11, 1, RW, Eeprom)` and the slices equal to its table's `operating_modes`; XL-320 `Register { register: Self::CONTROL_MODE, .. }` at 11 with its lists; AX-12A `AngleLimits`; YM070-210-M001 `Register` at 33 with its lists. Check 17 fixtures (no mode source; `Operating Mode` beside CW/CCW; two mode rows; a mode row without `operating_modes`; `operating_modes` without a mode row; a value in two lists) fail and name the table |
| SC-008 | `cargo llvm-cov -p xtask` reports at least 70 % lines |

## 3. Generated crate tests (`src/models/tests.rs`)

```sh
cargo test --lib models::tests                                           # all families on
cargo test --lib --no-default-features --features std,x-series models::tests   # family-off checks for ax, mx, y-series, pro
cargo test --lib --no-default-features --features std models::tests     # no family: every lookup fails
```

| Spec item | Expected |
|---|---|
| US2 AS1, SC-004 | `new(P2, 1190, 52, id)` is `Xl330M077Fw52`; with firmware 53 and 255 it is `Xl330M077`; every (protocol, number) is checked at 0, at each boundary and the value before it, and at 255 |
| US2 AS2, FR-008 | `firmware_register(P2, 1190)` is `Some(FIRMWARE_VERSION)`; `firmware_register(P2, 1020)` is `None` |
| US2 AS3, FR-007 | with `x-series` off, `new(P2, 1020, 0, id)` is `UnsupportedModel { feature: Some("x-series"), .. }` |
| US2 AS4 | an unknown number gives `UnsupportedModel { feature: None, .. }` |
| US2 AS5, FR-012 | `ActuatorKind::new(P1, 350, ..)` (XL-320, P2 only) gives `feature: None` |
| US2 AS6 | `new` takes `ModelNumber` directly |
| US1 AS5, FR-009 | a limit mode and limits recorded through `kind.cache_mut()` read back through `kind.cache()` (equal to a `Cache` built directly with the same calls); `kind.data()` equals `Type::DATA` |

## 4. Every family combination builds clean (SC-003)

```sh
for f in ax mx x-series y-series pro; do
  cargo clippy --lib --no-default-features --features std,$f -- -D warnings
  cargo clippy --lib --no-default-features --features $f --target thumbv7em-none-eabihf -- -D warnings
done
cargo clippy --lib --no-default-features --features std -- -D warnings
cargo clippy --lib --no-default-features --target thumbv7em-none-eabihf -- -D warnings
cargo clippy --lib -- -D warnings
```

Expected: no warnings from any of these 13 commands, nor from the all-families no_std line in
§1. Together they are the 14 builds of SC-003: 7 family combinations, each std and no_std (US3
AS1, AS2). The no-family lines also depend on core-1's `Register::new` fix (contract conflict
14).

## 5. Compile-time checks (US5)

- In a scratch crate depending on this one, `match kind { ActuatorKind::Xm430W350(_) => {} }`
  without a `_` arm fails with E0004, because the enum is `#[non_exhaustive]`.
- Naming `dxl_rs_v2::models::Ax12a` with `--no-default-features --features std,x-series` fails
  to resolve (family off).
- `Xm430W350::GOAL_POSITION` from outside the crate fails with E0624 (crate-internal constant,
  FR-015).

## 6. Add a model (SC-005, manual)

1. Add `control_tables/<new>.ron` with `source`, `protocols`, `type_info`, and rows with
   `access`/`area`, including the common roles and an `Operating Mode` or `Control Mode` row with
   its `operating_modes` (or CW/CCW Angle Limits with no mode row and no `operating_modes`)
   (tables module work, e-Manual cited).
2. Add its entry to `dynamixel.ron` under `p1` and/or `p2`.
3. Run `cargo xtask codegen`.

Expected: one new `src/models/<new>.rs`, and `src/models.rs` and `src/models/tests.rs` gain the
model's lines. No hand-written Rust. `cargo test --lib models::tests` covers the new number.

## 7. See a failure (manual)

1. Rename `"Goal Position"` to `"Goal Pos"` in `control_tables/xm430_w350.ron`.
2. Run `cargo xtask codegen`.

Expected: exit status 1, and `error: control_tables/xm430_w350.ron: missing role Goal Position ...`.
`git status` shows only the `.ron` change. Revert it.
