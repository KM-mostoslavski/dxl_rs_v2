Stacked on #8 (core-1) and #9 (tables T1); its diff shrinks once they merge.

## What

X1 of the xtask module (spec 006): `cargo xtask codegen` now reads the T1 schema and generates, for every mapped table, a model type that describes itself to the bus, plus a per-protocol lookup. Concretely:

- Every model type holds core's `Cache` and a crate-internal `DATA: ModelData`: name, number, protocols, Torque Enable / Goal Position / Present Position, the limit form (`MinMax` or `CwCcw`), the hardware-fault register or `None`, the **mode source** (`ModeSource::Register { register, values: ModeValues { bounded, unbounded, no_position } }` with the lists copied from the table's `operating_modes`, or `ModeSource::AngleLimits`), and the position scale from `type_info`. Model types implement `Describe` and `Actuator`; `ActuatorKind` delegates both by `match`.
- Every register constant carries access and area (`Area::Hybrid` included), built by a private generated `reg()` helper.
- `ActuatorKind::new(ProtocolVersion, ModelNumber, firmware, Id) -> Result<_, SetupError>`, `ActuatorKind::firmware_register`, and `MODEL_NUMBER`.
- The 17 checks of data-model §3 run before anything is written, and one run reports all of them. Output is formatted through rustfmt on stdin, and only changed files are written.
- Generated docs give each model's name, model number, firmware range per protocol, source table, and an Unverified or Verified (URL, date) line.
- A generated `src/models/tests.rs` (234 tests with all families) checks every (protocol, model number) at firmware 0, at each `from_fw` boundary and the value before it, and at 255. It also covers family-off errors, unknown numbers, numbers with no tables on a protocol, a cache round trip per model type, and a concrete `match` per model type.
- `src/models*` is regenerated from T1's tables: 70 model files, `models.rs` and `tests.rs`.

## Why

Spec 006, delivery step 3. The bus (core-2, then bus-1 to bus-6) needs these per-model facts without any hand-written model code. Limits follow the operating mode (contract §2.4b, architect 2026-10-28), so every model also says where Setup finds its mode and what each mode value means.

## How

- **Validate, then write** (research R-2, R-3): parse, resolve and check, render, then format each file through `rustfmt --emit stdout` on stdin. Only after all of that succeeds are the changed files written and the stale ones removed. If any step fails, every file under `src/models*` keeps its bytes and mtime. Diagnostics are sorted `error: control_tables/<file>.ron: …` lines.
- **`reg()` instead of `Register::new`** (deviation D1, R-5): a private `const fn reg(addr, size, access, area)` in `models.rs`, which model files call as `super::reg`. It works beside core-1's 2-argument shim, and core-2 needs no regeneration.
- **Lookup** (R-8, R-9): one `match (protocol, model_number.get())`. It has one arm per (protocol, table) under the family `cfg`, `if firmware < next` guards, one `cfg(not(family))` arm per (protocol, number) that names the feature, and a final `feature: None` arm.
- **Three test layers** (R-11):
  - xtask fixture tests: one `Failing` root per check, 1 to 17, each asserting the diagnostic and untouched output.
  - A real-tables oracle that re-finds every role row by e-Manual name and compares it with the resolver and the rendered `reg(...)` / `mode:` / `scale:` text.
  - The generated crate tests.
- **Mode source** (R-17): the `Operating Mode` row, else the `Control Mode` row, each with the table's `operating_modes`. Otherwise `AngleLimits` (CW/CCW limits, no mode row, no `operating_modes`). Check 17 refuses everything else.
- `xtask/src/codegen.rs` is split into `codegen/{schema,names,resolve,render,render_tests,output,fixtures}.rs` (R-13). xtask line coverage is 97.98 %; `main.rs` is the only untested file.

## Architecture

Follows `specs/006-xtask-codegen` and module-interfaces §4. These items need the architect's approval (task T062):

- **D1**: generated private `reg()` helper instead of a 4-argument `Register::new`.
- **C1**: `Describe` is named as `crate::actuator::Describe` (core-1 provides `pub(crate) use sealed::Describe`).
- **C2**: not needed. core-1 has no `ActuatorKind::new(u16, u8, Id)` tests, so this PR touches no file outside `xtask/**` and `src/models*`.
- **C3**: with a family off, `firmware_register` returns `None` for that family's numbers.
- **C4**: `MODEL_NUMBER` takes its access and area from the tables, and check 12 makes them agree. A table that differs from the most common pair is named.
- **C5**: the mode-source rules are check 17. Check 11 requires a mode register to be 1 byte, `RW`, `EEPROM`.
- **Complexity Tracking** (plan): the module-level `#![allow(dead_code, unused_variables, clippy::approx_constant, reason = …)]` in `models.rs`, the `reg()` helper, and crate tests generated into `src/models/tests.rs`.
- **Spec items aligned with contract §8** (plan table): `Cache` without torque or fault state, the per-protocol lookup returning `SetupError` with `id` and `protocol`, access and area on every register, MX on P1 in scope, and the crate-internal fault, Firmware Version and Model Number registers.

Deltas from module-interfaces §9 (2026-10-28/29) that the task list predates, applied here:

- **Family `rx`** (RX-28, model 28, P1 only, `AngleLimits`) is valid. The family check accepts `ax, mx, rx, x-series, y-series, pro`.
  - The family matrix is therefore 16 builds: 6 families alone, all, and none, each std and no_std.
  - The no_std set used locally is `ax,mx,rx,x-series,y-series,pro`.
- **`area: Hybrid`** (11 rows per Y table) maps to `Area::Hybrid`. No role register uses it.
- **Counts come from the data**: 73 tables, 64 model numbers, 70 mapped files, 3 `not_mapped` (`ax_12_plus`, `ax_18f`, `xw430_t200`), and 2 verified (`ax_12a`, `rx_28`). No test hard-codes a count; the oracle compares against the number of distinct mapped files.
- **`cache_mut`** stays as specified, with no token argument (provisional decision of 2026-10-29).

Smaller choices made while implementing, none of which changes the contract:

- Check 15 also refuses non-finite radians. `{:?}` of an infinite value would render `inf`, which is not Rust.
- Check 14 covers every role row: Model Number, Firmware Version, Torque Enable, Goal and Present Position, both limits, the fault register and the mode register.
- The family-off half of the generated tests is one test per family. It loops over that family's (protocol, number) pairs, and each `assert_eq!` shows the pair.
- The Verified doc line writes the URL as `<url>`, so rustdoc's `bare_urls` lint stays quiet.
- `is_reserved_module` includes the 2024 reserved keyword `gen`.

Other notes:

- **T014**: the new pipeline reproduced the old output byte for byte (0 written, 71 unchanged). T1 maps no file only under `p1` that the legacy `tables` lacks, so there is no extra model file.
- **quickstart §5**, run in a scratch crate:
  - Naming `dxl_rs_v2::models::Ax12a` with `--no-default-features --features std,x-series` fails with E0425 (cannot find type).
  - `dxl_rs_v2::models::Xm430W350::GOAL_POSITION` fails with E0624 (associated constant is private).
  - The rustdoc `compile_fail,E0004` example on `ActuatorKind` passes.
- **Note for ci (spec 007)**: today's `ci.yml` no_std line does not list `rx`, so RX-28 is not built for no_std there. It passes locally. The per-family `cargo test --lib --no-default-features --features std,<family> models::tests` legs pass for all six families and for none, under `-D warnings`. This PR does not touch `ci.yml`.
- **Follow-up X2** (optional, after core-2): drop the generated `id`/`model_number`/`name` overrides in the `impl Actuator` blocks and regenerate.
- `xtask/Cargo.lock` (stale; the workspace lock is the real one) was left alone. No dependency was added.
- **Open for the architect** (T061, T062, `[human]`): the SRS update for conflict 19 and the per-protocol lookup, and approval of D1 and C1–C5.

## Checklist
- [x] `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass
- [x] `cargo test` passes; line coverage ≥ 70 % (`cargo llvm-cov`): workspace 98.20 %, xtask 97.98 %
- [x] `no_std` build passes (`--no-default-features --features ax,mx,x-series,y-series,pro --target thumbv7em-none-eabihf`, and with `rx`)
- [x] `cargo xtask codegen` leaves `src/models.rs` and `src/models/` unchanged
- [x] Public methods documented (`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`, all families and `std,x-series`)
- [x] Docs updated if behaviour or setup changed (generated model docs; no README change needed)
