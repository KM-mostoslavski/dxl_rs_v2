# Research: xtask codegen

**Feature**: [spec.md](spec.md) | **Date**: 2026-10-27, updated 2026-10-28

Sources: `xtask/src/{main.rs,codegen.rs}` and `src/{lib.rs,actuator.rs,register.rs,error.rs}`
on `dev`; the generated `src/models*` (69 tables, 63 model numbers); `control_tables/*.ron`; the
protocol module on `main` (PR #7, merged); `specs/ideas/module-interfaces.md` (the contract,
cited as "contract §n"), including the 2026-10-28 decisions (§2.4b, §9); the SRS patch
`/tmp/dxl-srs-bus.patch`; constitution v1.3.0. Toolchain: rustc, clippy and rustfmt 1.97.1 /
rustfmt 1.9.0, ron 0.12.2.

Prototypes ran in scratch crates under `/tmp/claude-1000/xtaskplan/` (never in the repo):
`ronproto` (input schema) and `genproto` (generated-code shape, lints, feature combinations).

## R-1 Reading the new input schema

**Decision**: xtask reads exactly the schema of contract §3 with serde + ron 0.12.2.

- Tables keep the existing brace-to-paren rewrite, then are read with the same options as the
  map (below), so `operating_modes` is a plain optional field
  (`#[serde(default)] Option<OperatingModes>`, with `OperatingModes { bounded: Vec<u8>,
  unbounded: Vec<u8>, no_position: Vec<u8> }`).
- `dynamixel.ron` is read with `ron::Options::default().with_default_extension(Extensions::IMPLICIT_SOME)`,
  so `p1`, `p2` and `default` are plain optional fields (`#[serde(default)] Option<_>`).
- `source` is an enum `Unverified | Verified { url, verified }`. `protocols` is a list of the
  enum `P1 | P2`. Rows need `access` (`R | RW`) and `area` (`EEPROM | RAM`, serde renames).
- Unknown fields are ignored, not denied. That keeps today's map field `tables:` (legacy, until
  T2), `unit_info`, `instructions` and the per-model `range`/`default`/`unit` fields that come
  with verification readable without a codegen change.
- `family` stays a `String` and is checked against the five names by hand, so the message names
  the entry and the bad family (FR-005) instead of a serde "unknown variant" span.

**Rationale**: Prototype `ronproto` parsed a T1-shaped table and map:
`Verified(url: "...", verified: "...")` and `Unverified` both parse; a missing `access` gives
"Unexpected missing field named `access` in `Row`"; a missing `protocols` gives the same for
`Table`; `default: P3` names the bad variant; the legacy `tables:` field is ignored. Without
`IMPLICIT_SOME`, `p1: [...]` fails with `ExpectedOption`, so the extension is required. The
same holds for `operating_modes` in a table (tables research R-1, prototype 2026-10-28): without
the extension, `operating_modes: (bounded: [3], …)` fails with "Expected option"; with it, the
field parses, an absent one is `None`, a value above 255 fails with "Integer is out of bounds",
and a missing list with "Unexpected missing field" (both check 1). Today's xtask ignores the field:
with `operating_modes` added to three tables, `cargo xtask codegen` changed no file.

**Alternatives considered**:
- `p1: Vec<_>` with `#[serde(default)]`. Rejected: an explicit `p1: []` and a missing `p1` look
  the same. With `Option`, an empty list is reported as its own error.
- `#[serde(deny_unknown_fields)]`. Rejected: it would break on `tables:` during the T1 → T2
  transition and on every metadata field the tables module adds while verifying.
- A family enum in serde. Rejected: worse message, and the string is also the `cfg` feature name.

## R-2 Validate everything, then write only what changed

**Decision**: one pipeline, all in memory until the last step:

1. Parse the map and every referenced table.
2. Resolve and check (R-3, R-4). Collect every problem.
3. Render every file as a `String`.
4. Format each string with `rustfmt --edition 2024 --emit stdout`, fed on stdin.
5. Only if 1–4 all succeed: write each file whose bytes differ, and delete any `.rs` file in
   `src/models/` that is no longer generated. Identical files are not touched.

**Rationale**: FR-003 and SC-006 require the committed output to stay unchanged on any error.
Today's `run_codegen` deletes `src/models*` and then runs `rustfmt` on disk, so a rustfmt failure
leaves unformatted or partial output. Formatting on stdin moves that failure before the first
write. Checked on `dev`: formatting each of the 70 committed files through stdin gives
byte-identical output, and all 70 take 0.7 s. In `genproto`, stdin and on-disk formatting agree,
and the result passes `cargo fmt --check`. Writing only changed files keeps file times stable, so
an unchanged run (SC-001) does not trigger a rebuild, and a one-register change touches one file
(SC-007).

**Alternatives considered**:
- Format in a temp directory with one `rustfmt` call. Rejected: rustfmt follows `mod x;` lines
  from `models.rs` and fails when the children are not beside it (seen in the prototype).
- Delete-and-rewrite as today. Rejected: leaves partial output on a late failure.
- Write to a sibling directory and rename. Rejected: more moving parts for the same guarantee;
  the only failure left after step 4 is a disk write error, which `git checkout` repairs.

## R-3 Diagnostics

**Decision**: checks collect every problem, not just the first. Each is a
`Diagnostic { file, message }`, printed as `error: control_tables/<file>.ron: <message>`
(sorted by file, then message), and codegen exits with status 1. Unmapped tables print
`warning: control_tables/<file>.ron: ...` and do not change the exit status. A parse error stops
that file's further checks, but the other files are still checked.

**Rationale**: a maintainer fixing T1 data for 69 tables wants all gaps in one run. Sorting keeps
the output stable for tests and for CI logs. FR-003 asks that the message name the file.

**Alternatives considered**: fail fast with `Result<_, String>` (today). Rejected: one round trip
per problem.

## R-4 Matching common roles to rows

**Decision**: roles are matched by exact `data_name`, never by address:

| Role | `data_name` | Required |
|---|---|---|
| Model Number | `Model Number` | yes; must be address 0, size 2 in every table |
| Firmware Version | `Firmware Version` | yes; same location across the variants of one (model number, protocol) |
| Torque Enable | `Torque Enable` | yes |
| Goal Position | `Goal Position` | yes |
| Present Position | `Present Position` | yes |
| Limits, min/max form | `Min Position Limit` + `Max Position Limit` | one complete form |
| Limits, CW/CCW form | `CW Angle Limit` + `CCW Angle Limit` | one complete form |
| Hardware fault | `Hardware Error Status` | no; absent gives `None` |
| Operating mode | `Operating Mode` or `Control Mode`, with the table's `operating_modes` | at most one; neither (and no `operating_modes`) only with the CW/CCW form (R-17) |

Both limit forms present, or one half of a form, is an error. Role attributes are checked as in
contract §4.3 (Torque Enable and Goal Position `RW`/`RAM`, Present Position `R`/`RAM`, limits
`RW`/`EEPROM`, a mode register 1 byte, `RW`, `EEPROM`; plan C5). A table listed under `p1`
must have every role register at address ≤ 255.

**Rationale**: FR-011 and constitution III/IV: no guessed address. A scan of all 72 tables on
`dev` found every role in every table:

- X, MX(2.0), PRO and Y use min/max limits; AX and XL-320 use CW/CCW.
- AX has no Hardware Error Status, so it gets `None`. XL-320 has it at 50; X and MX(2.0) at 70;
  PRO at 518.
- Y-series names its fault register `Error Code` (153). It stays `None` until the tables module
  confirms it against the e-Manual (spec Assumptions; contract §2.6).
- Model Number is (0, 2) everywhere. Firmware Version is (2, 1) on AX and XL-320, (6, 1)
  elsewhere, and the same within each fw52/fw53 pair.
- PRO and Y roles sit above 255 (512+), so they can never be listed under `p1`. X roles (≤ 132)
  can.
- Mode rows: R-17.

**Alternatives considered**: a per-family role table in xtask. Rejected: it duplicates table data
and is a guess when a family has two layouts.

## R-5 How generated code builds a `Register` (deviation D1)

**Decision**: `src/models.rs` gets one private helper, and every register constant uses it:

```rust
const fn reg(addr: u16, size: u8, access: Access, area: Area) -> Register {
    Register { addr, size, access, area }
}
```

Model files are child modules of `models`, so they can call `super::reg`. The helper relies only on
the four `pub(crate)` fields in contract §2.2.

**Rationale**: the contract asks for two things that cannot both hold. Core-1 keeps the old
2-argument `Register::new(addr, size)` as a shim, so today's generated code still builds. X1 then
emits a 4-argument `Register::new` (§4.1, §7) while that shim still exists, before core-2. Rust has
no overloading, so the 4-argument call does not compile in that window. The helper works whatever
core-1 calls its constructors, and core-2 needs no regeneration. It reads as short as
`Register::new` (`reg(116, 4, Access::RW, Area::Ram)`).

**Alternatives considered**:
- A struct literal in every constant. Rejected: rustfmt splits many of them over 6 lines, and
  about 100 rows per table make large, noisy diffs.
- Ask core-1 to name the 4-argument constructor differently (`Register::with`). Workable, but it
  changes core's final API, which is core's call. If core-1 does that, X1 may call it instead. The
  rendered text changes, the data does not.

## R-6 Emitting the position scale

**Decision**: `type_info` is parsed as `i32` (raw values) and `f32` (radians). Radians are
printed with Rust's shortest round-trip `f32` form (`{:?}`), e.g. `3.14159265` becomes
`3.1415927`. `src/models.rs` carries `clippy::approx_constant` in its module-level `allow`, with a
reason.

The values are the tables' corrected `type_info` after T1 (tables research R-8, data-model):
angle = (raw − zero) × rad_per_tick. In table text, X-series, MX(2.0), 2XL/2XC give `(2048, 0,
4095, -3.1415927, 3.1400587)` (2π/4096 per tick), and AX and XL-320 give `(512, 0, 1023,
-2.6205530, 2.6154347)`. The generated literal is `{:?}` of the parsed `f32`, not the table
text: `-3.1415927`, `3.1400588`, `-2.620553` and `2.6154346` (checked with rustc 1.97.1). Tests
never hard-code a radian literal: they build the expected text with `format!("{:?}", v)` from
the table or fixture value. Examples in the contracts show the `{:?}` form of T1's values, never
±π.

**Rationale**: in `genproto`, `PositionScale::new(2048, 0, 4095, -3.14159265, 3.14159265)` failed
clippy twice. `clippy::approx_constant` is deny-by-default, so it is an error even without
`-D warnings`. `clippy::excessive_precision` also fired on the 9-digit `f32` literal. With the
shortest form and the allow, all four feature combinations in the prototype had zero lints in
generated files. The value is table data, not a use of π, so the lint does not apply.

**Alternatives considered**:
- Emit `core::f32::consts::PI` when a value equals it. Rejected: it special-cases one constant,
  the next angle (AX -2.6205530) still needs plain literals, and the diff no longer shows the table
  value.
- Parse as `f64` and cast. Rejected: no gain; `PositionScale` takes `f32` (contract §2.3).

## R-7 Static model data

**Decision**: each model type has `pub(crate) const DATA: ModelData`, and
`Describe::data` returns `&Self::DATA`. `ModelData` refers to the role constants by name
(`goal_position: Self::GOAL_POSITION`, `mode: ModeSource::Register { register:
Self::OPERATING_MODE, values: ModeValues { … } }`), so each address appears once per file.

**Rationale**: the prototype compiled `fn data(&self) -> &'static ModelData { &Self::DATA }`.
Constant promotion applies, since `ModelData` has no `Drop` and no interior mutability. No `static`
is needed, nothing is allocated, and there is no runtime lookup (NFR-4). Referring to the
constants by name means a register change edits one line.

**Alternatives considered**: a `static DATA` per model. Rejected: same result, plus one more item.
Literal registers inside `DATA`: rejected, since every address would then appear twice.

## R-8 The per-protocol lookup

**Decision**: `ActuatorKind::new(protocol, model_number, firmware, id)` is one `match` on
`(protocol, model_number.get())`:

- One arm per (protocol, table) under that family's `cfg`. Every table except the last in a list
  gets a guard `if firmware < <next from_fw>`.
- One `#[cfg(not(feature = "<family>"))]` arm per (protocol, model number) that has tables,
  returning `UnsupportedModel { feature: Some(family) }`.
- A final `_` arm returning `UnsupportedModel { feature: None }`. That covers unknown numbers and
  known numbers with no tables on this protocol.
- Arms are ordered by model number, then P1 before P2.

`Describe` on `ActuatorKind` uses `match *self { Self::X(ref m) => ... }` (`ref mut` for
`cache_mut`).

**Rationale**: this gives exactly the result table of contract §4.2, and "highest `from_fw` not
above `firmware`" (FR-006) follows from ordered guards. Two prototype findings:

- `match self {}` on the empty enum (no family) fails with E0004 ("type `&ActuatorKind` is
  non-empty"). `match *self {}` compiles, which is why today's generator uses `match *self`.
- Firmware 255 and each boundary work with plain `<` guards. No arithmetic is needed, so nothing
  can overflow.

**Alternatives considered**:
- A data table plus a search loop. Rejected: it adds a runtime search and `cfg` filtering of
  table rows.
- Merging P1 and P2 arms into `P1 | P2` when the lists match. Rejected: more generator logic for a
  cosmetic gain; separate arms review more clearly.

## R-9 `firmware_register` and `MODEL_NUMBER`

**Decision**:

- `firmware_register(protocol, model_number)` has one arm per (protocol, model number) whose list
  has more than one table. The arm sits under the family `cfg` and returns
  `Some(<FirstType>::FIRMWARE_VERSION)`. Every other case is `None`, including a family that is
  off.
- `MODEL_NUMBER` in `models.rs` is a literal `reg(0, 2, …)`, with access and area from the tables.
  Codegen checks that all tables agree.

**Rationale**: pointing at the type's constant keeps the address in one file (SC-007). When the
family is off, `None` is still correct: the bus skips the firmware read, and `new` returns
`UnsupportedModel { feature: Some(..) }` anyway, with fewer bytes on the wire. `MODEL_NUMBER` must
exist with no family enabled, so it cannot point at a model type.

**Alternatives considered**: literal Firmware Version locations in `models.rs`. Rejected: one
table edit would then change two files.

## R-10 Warning-free builds for every family combination

**Decision**: generated code keeps one module-level attribute in `src/models.rs`:
`#![allow(dead_code, unused_variables, clippy::approx_constant, reason = "...")]`. Each generated
file has its own imports, and only the items it uses in every combination in which it compiles:

- `models.rs`: items used with no family too (`Actuator`, `Cache`, `Describe`, `ModelData`,
  `ModelNumber`, `SetupError`, `Id`, `ProtocolVersion`, `Access`, `Area`, `Register`). Not
  `LimitRegisters`, `Protocols`, `ModeSource`, `ModeValues` or `PositionScale`: only model files
  use those.
- A model file compiles only with its family, so it imports what its `DATA` and impls use:
  `ModeValues` only when the table has a mode register (an `AngleLimits` file does not use it).
- `tests.rs`: top-level imports only for the tests that compile in every combination (the
  unknown-number test). A family-gated test imports what it needs inside its body.

**Rationale**:

- `dead_code`: about 100 register constants per model, of which the bus uses a few.
- `unused_variables`: with no family, `firmware` is unused in `new`.
- `approx_constant`: R-6.

In `genproto`, clippy with `-D warnings` reported nothing from generated files in four
combinations: all families, none, `ax` only and `x-series` only. The remaining warnings came from
the stand-in core types, which core-1 owns (contract conflict 14).

**Alternatives considered**:
- `#[expect(...)]`. Rejected: a lint that does not fire in some feature combination makes
  `expect` itself warn.
- Per-item allows. Rejected: noisy diffs on every constant.

## R-11 Tests: xtask fixtures, real tables, and generated crate tests

**Decision**: three layers.

1. **xtask unit and fixture tests**: names, parsing, every check of data-model §3 (contract
   §4.3 plus check 17; one fixture per error class) and rendering snippets. Fixtures are inline
   RON strings written to a temp directory by the existing `tempdir` helper.
2. **xtask end-to-end tests**: `run` on a temp root. They check that a second run leaves every
   file byte-identical and untouched (mtime), that a failing fixture leaves sentinel files
   untouched, and that a one-row change rewrites one file. Then one test runs on the real
   `control_tables/`. For every mapped table it re-finds each role row with a simple name lookup
   written in the test, and checks the resolver result and the rendered `reg(...)` line against
   it, mode source included. That is SC-002. It also checks hand-written acceptance values:
   XM430-W350 Goal Position 116/4, AX-12A CW/CCW at 6/8, XL330-M077 boundary 53, AX-12A hardware
   fault `None`, and the mode sources of XM430-W350 (`Operating Mode` at 11, the lists of its
   `operating_modes`), XL-320 (`Control Mode` at 11, its lists), AX-12A (angle limits, no lists)
   and YM070-210-M001 (`Operating Mode` at 33, its lists). The oracle reads the lists from each
   table's `operating_modes` with its own parse, never from a family rule.
3. **Generated crate tests** in `src/models/tests.rs` (`#[cfg(test)] mod tests;` in
   `models.rs`). For every (protocol, model number) they check the lookup at firmware 0, at each
   `from_fw` and the value before it, and at 255. With the family off (`cfg(not)`), they check
   that the error names the family. They also check `firmware_register`, an unknown number (the
   largest `u16` not in the map), and one round trip per model type: `data()` equals
   `Type::DATA`, and a limit mode and limits recorded through `ActuatorKind::cache_mut` read back
   through `cache()`. Core gives no mode getter, so the test compares `*kind.cache()` with a
   `Cache` built directly through the same calls. That is SC-004 and US1 AS5. The file names
   registers and data only by constant (`<Type>::FIRMWARE_VERSION`, `<Type>::DATA`,
   `MODEL_NUMBER`), never by literal address, so SC-007 holds for it too.

**Rationale**: FR-017 asks for both fixture tests and crate tests against the real tables. No
hand-written file under `src/` other than the `lib.rs` line is allowed, so crate tests over all
63 numbers must be generated. Layer 2 supplies an oracle that does not come from the generator's
own resolver. The family-off half of layer 3 only compiles in a build with that family off. The
ci feature-set job must run `cargo test --lib` there, not just clippy (raised to ci, see
plan.md). Resolved: ci runs a `cargo test --lib --no-default-features --features std,<family>`
leg per family, plus a `std` leg, under `-D warnings` (specs/007-ci contracts). Generated tests
must be warning-free in each leg.

**Alternatives considered**:
- Integration tests under `tests/`. Rejected: they see only the public API, and `ModelData`,
  `new` and `firmware_register` are crate-internal.
- Keep the lookup tests in core's `src/actuator.rs`. Rejected: X1 changes the lookup signature,
  and xtask may not edit core files (clarification C2).
- A `ron` dev-dependency in the crate to compare at test time. Rejected: a new dependency for
  something xtask already does.

## R-12 Ordering with core-1, T1, core-2

**Decision**: X1 assumes core-1 and T1 are merged. Protocol PR #7 (contract §5,
`ProtocolVersion`) is merged into `main`. Core-1 brings the §2.4b items too (`ModeSource`,
`LimitMode`, `ModelData::mode`, `Cache::record_mode`), since contract §7 puts every type of
section 2 in core-1. X1 emits:

- `impl Describe` on every model type and on `ActuatorKind`.
- `impl Actuator` that still spells out `id`, `model_number` and `name`. Core-1's trait requires
  them, and core-2's provided methods accept the overrides.
- `ActuatorKind::new` returning `SetupError` (core-1 adds it).
- `mode: ModeSource::…` in every `DATA`, with `ModeValues` where the table has a mode register
  (R-17).
- Imports: every item by its owning module, as in the core plan's path table
  (`specs/004-core/contracts/core-api.md`, "Paths for xtask and bus"): `crate::actuator::{Actuator,
  Cache, Describe, LimitRegisters, ModeSource, ModeValues, ModelData, ModelNumber, Protocols}`,
  `crate::units::PositionScale`, `crate::error::SetupError`, `crate::id::Id`,
  `crate::protocol::ProtocolVersion` and `crate::register::{Access, Area, Register}`. No root
  re-export is used, so generated code does not depend on when core adds them (contract §2.8 is the
  list at the end of the chain). Each file imports only what it uses (R-10).

**Rationale**: contract §7. Two things the contract leaves open are raised as clarifications in
plan.md. The core plan (`specs/004-core/plan.md`, D1, D2 and its clarifications) resolves both
the same way:

- C1: `Describe` must be reachable as `crate::actuator::Describe`. A private `mod sealed` inside
  `actuator.rs` is not visible from `crate::models`.
- C2: core-1's tests of `ActuatorKind::new(u16, u8, Id)` in `src/actuator.rs` break on the new
  signature.

**Alternatives considered**: keeping the old `new(u16, u8, Id)` beside the new one. Rejected: two
lookups that can disagree, against Principle II (one place decides).

## R-13 xtask layout and coverage

**Decision**: split `xtask/src/codegen.rs` (554 lines) into `codegen.rs` + `codegen/`, never
`mod.rs`:

- `codegen.rs`: the `run` and `generate` entry points, `Report` and `Diagnostic`.
- `schema.rs`: serde types and parsing (R-1).
- `names.rs`: identifiers.
- `resolve.rs`: checks and the resolved model.
- `render.rs`: model files and `models.rs`.
- `render_tests.rs`: renders `src/models/tests.rs`.
- `output.rs`: rustfmt and disk sync.
- `fixtures.rs`: `cfg(test)` fixture builders and the temp directory.

`main.rs` keeps only argument parsing, printing and the exit code. No new dependency, and no new
dev-dependency.

**Rationale**: each file has one job, and `main.rs` is the only part left untested, so xtask stays
well above 70 % line coverage (SC-008). `cargo llvm-cov --workspace` already measures xtask; only
`src/models*` is excluded.

**Alternatives considered**: `tempfile` or `insta` dev-dependencies. Rejected: the existing
40-line `tempdir` helper is enough, and snapshot files would duplicate the committed output that
the CI diff check already guards.

## R-14 Toolchain and rustfmt

**Decision**: latest stable Rust, with no MSRV (architect, 2026-10-27; dev uses 1.97.1). rustfmt
comes from the rustup component. If it is missing, `run` fails before writing, with
`rustfmt not found: run \`rustup component add rustfmt\``. A rustfmt error on one file names that
file and fails the run.

**Rationale**: SRS edge case "rustfmt is missing or fails". rustfmt can format differently between
stable releases. The CI diff check catches that, and the fix is a regeneration commit.

**Alternatives considered**: `prettyplease`. Rejected: a new dependency, and its output differs
from `cargo fmt`, so `cargo fmt --check` would fail on generated files.

## R-15 Generated documentation

**Decision**: each model type's doc comment is built only from table data and says:

1. `<name> (model number <n>[, also sold as <also…>])`.
2. Per protocol: `Protocol 1|2, firmware <range>`, where the range reads "all firmware",
   "up to v52" or "v53 and later".
3. `Source table: \`control_tables/<file>.ron\``.
4. Either `Unverified: not yet checked against the ROBOTIS e-Manual.` or
   `Verified against <url> on <date>.`

Each `ActuatorKind` variant gets the first line, the firmware range and the protocols. The file
header keeps `// @generated ...` plus `// Source: control_tables/<file>.ron`.

**Rationale**: FR-016, constitution IV (unverified tables flagged) and VII (every public item
documented). PR #7 turns on `#![warn(missing_docs)]`.

## R-16 Identifier collisions

**Decision**: on top of today's duplicate-type and duplicate-register checks, a module identifier
equal to `tests` (reserved for the generated test module) or to a Rust keyword is an error. Two
rows on one address with different names are allowed (aliases, e.g. Indirect Address 1 and
Indirect Address Write).

**Rationale**: the generated `tests` module must not collide with a table named `tests`. No
table name today is a keyword; the check makes that hold for the future too.

## R-17 Mode source (limits follow the operating mode; the values live in the tables)

**Decision**: each model's `DATA` gets `mode: ModeSource` (core's type, contract §2.4b, §2.6),
from the rows matched by exact `data_name` and the table's `operating_modes` (contract §3.1,
architect 2026-10-28):

1. Check 17 fails first if the table has an `Operating Mode` row beside CW/CCW Angle Limit rows;
   both an `Operating Mode` and a `Control Mode` row; neither a mode row nor CW/CCW Angle Limit
   rows; a mode row without `operating_modes`; `operating_modes` without a mode row; or one
   value in two of its lists.
2. An `Operating Mode` row, else a `Control Mode` row, gives
   `ModeSource::Register { register: Self::<CONST>, values: ModeValues { bounded: &[..],
   unbounded: &[..], no_position: &[..] } }`. `<CONST>` is the row's constant
   (`OPERATING_MODE` or `CONTROL_MODE`, built by `super::reg(..)` like every row); the three
   slices are the table's lists, in table order, printed as `u8` literals (`&[]` when empty).
3. Else (CW/CCW Angle Limits, no mode row, no `operating_modes`) it is `ModeSource::AngleLimits`.

Check 11 requires a mode register to be 1 byte, `RW`, `EEPROM` (contract §4.3, plan C5). A mode
row above address 255 in a table listed under `p1` fails check 14, like any role. A value outside
`u8` or a missing list key in `operating_modes` is a parse error (check 1, R-1). X1 does not check
the lists against a family, and does not ask for a value in any list: a value in no list is
decided at Setup (`UnknownOperatingMode`, core and bus).

Example, XM430-W350 after T1:

```rust
mode: ModeSource::Register {
    register: Self::OPERATING_MODE,
    values: ModeValues { bounded: &[3], unbounded: &[4, 5], no_position: &[0, 1, 16] },
},
```

**Rationale**: contract §2.4b (architect, 2026-10-28): the firmware applies position limits only
in position mode, so the goal check follows the mode, and Setup must know where to read it and
what each value means. One value means different modes in different families (5: unbounded on X
and MX(2.0), bounded on RH-P12-RN, a `pro` table), so neither the register nor the map's
`family` can say it. The tables carry the lists, copied from each model's page, and X1 copies
them unchanged: no rule keyed on a family or a file name (constitution III), and a new model is a
data change only (SC-005). A scan of all 72 tables on `dev`, with the lists T1 writes (tables
research R-15, expected from core research R-17; each table's page wins):

| Table has | Tables | `mode` | `values` (bounded / unbounded / no_position) |
|---|---|---|---|
| `Operating Mode` (1 byte), min/max limits | 66: X, MX(2.0), PRO, PRO+, RH at address 11 (54); Y at 33 (12) | `Register` | X, MX(2.0): `[3]` / `[4, 5]` / `[0, 1, 16]` (XL430-W250, 2XL430-W250: `[3]` / `[4]` / `[1, 16]`); PRO (A), PRO+: `[3]` / `[4]` / `[0, 1, 16]`; RH-P12-RN(A): `[5]` / `[]` / `[0]`; Y: `[3]` / `[]` / `[0, 1]` |
| `Control Mode` at 11 (1 byte), CW/CCW limits | `xl320` | `Register` | `[2]` / `[]` / `[1]` |
| no mode row, CW/CCW limits | `ax_12a`, `ax_12_plus`, `ax_12w`, `ax_18a`, `ax_18f`; MX(1.0) P1 tables after T2 | `AngleLimits` | none |

No table has an `Operating Mode` row beside CW/CCW limits, and none has both mode rows. After T1
every table with a mode row has `operating_modes` (the tables check enforces it, tables plan
deviation 4), so every table resolves and check 17 guards future tables only. Matching by name
keeps constitution III and IV: the register and the values come from the table. The Setup cost
(one 1-byte read on P2 and XL-320, none on AX and MX P1) is the bus's.

**Alternatives considered**:
- Derive the source from the limit form alone (min/max → `Operating Mode`, CW/CCW → angle
  limits). Rejected: XL-320 has CW/CCW limits and a `Control Mode` register, and a register
  inferred from the form is a guess.
- Per-family value lists in core (`ModeValues { X, Pro, Y, RhP12Rn }`), X1 picking the variant
  from the map's `family`. Rejected by the architect (2026-10-28): the family cannot single out
  RH-P12-RN (a `pro` table), and every new family would need core code. Splitting a family by
  file name or model number in xtask: rejected, constitution III.
- Separate `OperatingMode` and `ControlMode` variants. Rejected (contract §2.4b): with the lists
  in the table, XL-320's register decodes like any other, so one `Register` variant serves both.
- `register` as a literal `reg(addr, size, ..)` in `DATA`. Rejected: the row's constant already
  holds it (R-7), so the address stays on one line.
- A per-family mode register in xtask. Rejected, as in R-4: it duplicates table data.
- A warning instead of an error for two mode rows, or for a mode row without its lists.
  Rejected: Setup reads exactly one register and decodes it with exactly one set of lists, so
  picking one or using a default is a guess.
