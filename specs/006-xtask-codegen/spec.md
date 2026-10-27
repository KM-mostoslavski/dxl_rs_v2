# Feature Specification: xtask codegen (generated models, dispatch and family features)

**Feature Branch**: `dev` (spec directory `specs/006-xtask-codegen/`)

**Created**: 2026-10-27

**Status**: Draft

**Input**: User description: "xtask (codegen) module of docs/srs/SRS.md"
→ SRS module `xtask` (SRS §9): paths `xtask/**`, `src/models.rs`, `src/models/**`. Covers
FR-7.3, FR-7.5, NFR-7, NFR-8, and the generated side of FR-2.3 and FR-3.2. Delivery step 3.
No-go: any hand-written file under `src/` other than the `pub mod models;` line in `src/lib.rs`.

## Clarifications

### Session 2026-10-27

- Q: Do the generated `Actuator` impls perform wire operations or only supply per-model data? → A: Only per-model data; I/O is the bus's job (single responsibility).
- Q: A mapped table lacks protocol-support data: fail or generate as "protocols unknown"? → A: Codegen fails and names the table; it is a priority issue for the table maintainers.
- Q: Are the register constants on each model type public API? → A: No, internal; users have get/set methods for that, which makes their lives easier.

## User Scenarios & Testing *(mandatory)*

The "users" of this module are the crate maintainer, who adds and fixes actuator models, and
the other crate modules (`bus` above all) that build on the generated code. Robot application
and embedded developers meet the result as model types, `ActuatorKind` and family features.

Codegen already exists (PR #4). This spec covers what it still lacks; see Assumptions for what
is in place.

### User Story 1 - Common operations work on every model (Priority: P1)

The bus needs the same facts about every actuator, whatever its model: where Torque Enable,
Goal Position and Present Position live, which registers hold the position limits, whether a
hardware-fault register exists, how raw position ticks map to radians, and where the
per-actuator cache is. Codegen produces these facts for every model type, and `ActuatorKind`
hands them out by `match`. No model needs hand-written code.

**Why this priority**: Delivery step 4 (bus, torque guard, limits at Init) cannot start
without it. Today the generated `Actuator` impls only give `id`, `model_number` and `name`.

**Independent Test**: For one model per layout tier (e.g. XM430-W350, AX-12A, XL-320,
PH54-200-S500, YM070-210-M001), build the `ActuatorKind`, ask it for each common register, the
limit form, the fault register and the position scale, and compare with the model's `.ron`
table. Write the cache through `ActuatorKind` and read it back.

**Acceptance Scenarios**:

1. **Given** an XM430-W350, **When** the bus asks for its Goal Position register, **Then** it
   gets address 116, size 4, exactly as the table lists it.
2. **Given** an AX-12A, **When** the bus asks for its position limits, **Then** it gets the CW
   and CCW Angle Limit registers, marked as the CW/CCW form, not the min/max form.
3. **Given** a model whose table has no hardware-fault register, **When** the bus asks for it,
   **Then** the answer is "none", not a guessed address.
4. **Given** any model, **When** the bus asks for its position scale, **Then** it gets the
   zero, minimum and maximum raw values and radians from that table's `type_info`.
5. **Given** an `ActuatorKind` holding any model, **When** the bus updates its cache (torque
   state, fault state, limits), **Then** the change is visible through the same value, with no
   copy and no allocation.

---

### User Story 2 - Setup turns any reply into the right model (Priority: P1)

During Setup the bus has a model number and, sometimes, a firmware version. It asks the
generated code for the matching model type. When a model has firmware-dependent tables, the bus
also needs to know that it must read Firmware Version, and where that register is, because a
P1 ping carries neither model number nor firmware. It also needs to know which protocols the
model speaks, so a model on the wrong bus fails at Init, not in the first Cycle.

**Why this priority**: Autoscan and declared setup (FR-2.3, FR-2.5) both depend on it. Errors
must surface at Init (constitution II).

**Independent Test**: For every model number in `dynamixel.ron`, call the lookup with the
firmware values around each `from_fw` boundary and check the variant; query the firmware flag,
the Firmware Version and Model Number locations and the protocol set, and compare with the
tables. Repeat with each family switched off.

**Acceptance Scenarios**:

1. **Given** model number 1190 (XL330-M077) and firmware 52, **When** the lookup runs,
   **Then** it returns the `Xl330M077Fw52` variant; with firmware 53 it returns `Xl330M077`.
2. **Given** model number 1190, **When** the bus asks whether it needs the firmware version,
   **Then** the answer is yes, with the Firmware Version register's address and size; for
   model 1020 (one table) the answer is no.
3. **Given** a build without the `x-series` feature, **When** the lookup runs for 1020,
   **Then** it returns `UnsupportedModel` naming `x-series`.
4. **Given** a model number not in `dynamixel.ron`, **When** the lookup runs, **Then** it
   returns `UnsupportedModel` with no feature.
5. **Given** a model, **When** the bus asks which protocols it speaks, **Then** it gets the set
   recorded for it in the control-table data (e.g. AX-12A: P1 only).
6. **Given** the declared-setup list type (`ModelNumber`), **When** it is passed to the
   lookup, **Then** it is accepted directly, with no conversion by the user.

---

### User Story 3 - Compile only the families I use (Priority: P2)

An embedded developer turns off default features and enables only `x-series`. Only X-series
model types are compiled. A model from a family that is off is still recognised at Setup and
fails with an error naming the feature to enable.

**Why this priority**: NFR-7 (compile time and size) and FR-7.5. Family gating exists, but
not every feature combination is proven to build cleanly.

**Independent Test**: Build the crate (std and `no_std`) with each family alone, with all of
them, and with none, under clippy with warnings denied. In each build, check that types from
disabled families cannot be named and that their model numbers return `UnsupportedModel`
naming the family.

**Acceptance Scenarios**:

1. **Given** only `x-series` enabled, **When** the crate builds, **Then** no AX, MX, Y-series
   or PRO model type is compiled, and the build has no warnings.
2. **Given** no family enabled, **When** the crate builds, **Then** it builds without warnings
   and every lookup returns `UnsupportedModel`.
3. **Given** a `dynamixel.ron` entry whose family is not one of `ax`, `mx`, `x-series`,
   `y-series`, `pro`, **When** codegen runs, **Then** it fails, naming the entry and the
   unknown family.
4. **Given** a family enabled on top of others, **When** the crate builds, **Then** existing
   API keeps its behaviour; the family only adds model types and `ActuatorKind` variants.

---

### User Story 4 - Change a table and review a readable diff (Priority: P2)

The maintainer edits or adds a `.ron` table, runs `cargo xtask codegen`, and reviews the
generated change as an ordinary source diff. Bad input stops codegen with a message that names
the file and the problem, and leaves the committed output untouched.

**Why this priority**: NFR-8 and constitution V. Most of it exists; this story adds the checks
the new generated data needs.

**Independent Test**: Run codegen on fixture tables: a valid set, then one fixture per
failure class (missing file, bad `from_fw` order, duplicate names, unknown family, missing
common register, missing scale or protocol data, Model Number at an unexpected address).
Check exit status, message and that existing output files are unchanged.

**Acceptance Scenarios**:

1. **Given** unchanged tables, **When** codegen runs twice, **Then** the second run produces
   byte-identical files.
2. **Given** one register's address changed in one table, **When** codegen runs, **Then**
   the diff is confined to that model's file.
3. **Given** a mapped table that lacks Goal Position, **When** codegen runs, **Then** it fails,
   naming the table and the missing role, and writes nothing.
4. **Given** tables on disk that no model number points to (today `ax_12_plus`, `ax_18f`,
   `xw430_t200`), **When** codegen runs, **Then** it succeeds and prints one warning per file.
5. **Given** generated files, **When** a reviewer opens one, **Then** it is formatted, says it
   is generated and from which table, and documents the model name, model number and firmware
   range.

---

### User Story 5 - Reach a concrete model for model-specific work (Priority: P3)

A robot developer matches on `ActuatorKind` to reach a concrete model type, e.g. to use an
XM430-W350-only feature. Model types are documented public types. The enum is
`#[non_exhaustive]`, so adding models later is not a breaking change.

**Why this priority**: Tier 2 (FR-3.5) is the escape route from the generic API, but the
model-specific methods themselves are built later by other modules.

**Independent Test**: In a crate test, build an `ActuatorKind` from model number 1020,
`match` it to `Xm430W350`, and read its `id`, `name` and model number. In a compile-fail
check outside the crate, a `match` on `ActuatorKind` without a wildcard arm does not compile.

**Acceptance Scenarios**:

1. **Given** an `ActuatorKind`, **When** the user matches it, **Then** each variant holds a
   public, documented model type named after its table (e.g. `Xm430W350`, `TwoXl430W250`).
2. **Given** a model type, **When** the user wants its register map, **Then** access follows
   FR-015.
3. **Given** user code outside the crate, **When** it matches `ActuatorKind`, **Then** it must
   have a wildcard arm.

---

### Edge Cases

- A table exists but no model number points to it: skipped with a warning naming the file;
  codegen still succeeds (FR-7.4).
- A model number points to a missing file, or tables are listed out of `from_fw` order or not
  starting at 0: codegen fails before writing anything.
- Firmware exactly at a `from_fw` boundary, and firmware 255: the table with the highest
  `from_fw` not above it is picked.
- A file name starting with a digit (`2xl430_w250`): the type name spells the digit
  (`TwoXl430W250`).
- Two register names that become the same identifier, or two tables that become the same type
  name: codegen fails, naming both.
- Two names on one address (e.g. Indirect Address 1 and Indirect Address Write, both at 168):
  allowed; they are aliases in the e-Manual table.
- Position limits in CW/CCW form (AX, XL-320) next to min/max form (X, MX, PRO, Y): both are
  carried as data, with the form named.
- A model with no hardware-fault register (AX): reported as "none"; fault detection then
  relies on the status packet error byte alone.
- A model claiming P1 support whose common registers sit above address 255 (P1 addresses are
  8-bit): codegen fails, naming the model and register.
- A table whose Model Number is not at the address and size every other table uses: codegen
  fails, since a P1 autoscan reads that location before it knows the model.
- No family enabled: `ActuatorKind` has no variants, every lookup fails with
  `UnsupportedModel`, and the build has no warnings.
- `rustfmt` is missing or fails: codegen fails with a clear message; the CI diff check
  catches any unformatted output.
- MX on Protocol 1 firmware (roadmap): one model number may later need tables under both
  protocols; nothing in the generated lookup may rule that out.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `cargo xtask codegen` MUST read `dynamixel.ron` and every table it references,
  and write one model type per table plus the `models` module with `ActuatorKind`, into
  `src/models.rs` and `src/models/` only. Output is committed; nothing is generated at build
  time (FR-7.3, constitution V).
- **FR-002**: Output MUST be deterministic and formatted: the same inputs give byte-identical
  files. Each file states that it is generated and names its source table (NFR-8).
- **FR-003**: Codegen MUST validate all input before writing any file. On any error it exits
  non-zero, names the file and the problem, and leaves the committed output unchanged. Error
  classes: missing or unparsable file; `from_fw` not starting at 0 or not strictly increasing;
  duplicate type or register identifiers; unknown family; missing common register (FR-011);
  missing position scale or protocol data (FR-012, FR-013); Model Number location differing
  between tables (FR-008); a P1-capable model with a common register above address 255.
- **FR-004**: A table with no model number in `dynamixel.ron` MUST be skipped with one warning
  per file, and codegen MUST still succeed (FR-7.4).
- **FR-005**: Every model type and `ActuatorKind` variant MUST be gated by its family feature.
  The family names are exactly `ax`, `mx`, `x-series`, `y-series`, `pro` (FR-7.5); any other
  name in `dynamixel.ron` is a codegen error.
- **FR-006**: The lookup MUST map a model number and a firmware version to the `ActuatorKind`
  variant whose table has the highest `from_fw` not above the firmware (FR-2.3). It MUST accept
  the `ModelNumber` newtype that declared setup uses (FR-2.5).
- **FR-007**: The lookup MUST return `UnsupportedModel { model_number, feature }`: with the
  family name when the model is known but its family is off, and with no feature when the model
  number is unknown (FR-2.4).
- **FR-008**: For each known model number, the generated code MUST tell whether its table
  depends on firmware and, if so, where its Firmware Version register is. It MUST also expose
  the one Model Number location shared by all tables, checked at codegen time.
- **FR-009**: Each model type MUST hold the per-actuator cache (the core-defined type: torque
  state, fault state, limits) and give access to it; `ActuatorKind` MUST give the same access
  by `match`, with no `dyn` and no allocation (NFR-4).
- **FR-010**: Each model type MUST implement every method of the `Actuator` trait that the core
  module declares, and `ActuatorKind` MUST implement the trait by delegating with `match`
  (FR-3.2). The scope of those impls follows SRS OQ-2 meanwhile: torque, goal position,
  present position and limits, plus today's `id`, `model_number` and `name`.
  The generated impls only supply per-model data (registers, scale, cache); they perform no
  I/O, which the bus does through `ActuatorRef` (single responsibility).
- **FR-011**: Each model type MUST expose, as compile-time data, the registers that serve the
  common operations: Torque Enable, Goal Position, Present Position; the position limits in
  min/max or CW/CCW form, with the form named; and the hardware-fault register or "none".
  Roles are matched to table rows by their e-Manual register names; codegen never guesses an
  address.
- **FR-012**: Each model type MUST expose the protocols it supports (P1, P2 or both), taken
  only from explicit control-table data, never inferred from registers or family.
  When a mapped table lacks protocol-support data, codegen MUST fail and name the table: the
  gap is for the table maintainers to fix first, so this feature depends on a tables PR that
  records the protocol of every mapped table (not on full e-Manual verification).
- **FR-013**: Each model type MUST expose its position scale (zero, minimum and maximum raw
  values and radians) from the table's `type_info`.
- **FR-014**: Generated code MUST allocate nothing, use no `dyn`, and contain no panic path
  (NFR-1, NFR-3, NFR-4). It MUST build without warnings, std and `no_std`, with each family
  alone, with all families and with none.
- **FR-015**: Model types MUST be public, documented types reachable through `ActuatorKind`
  variants; `ActuatorKind` MUST be `#[non_exhaustive]`; constructors stay crate-internal (SRS
  OQ-1 meanwhile).
  Register constants on model types stay crate-internal: users go through the get/set
  methods, which is easier for them; model types are public only as `match` targets.
- **FR-016**: Every public generated item MUST be documented: model name, model number,
  firmware range and source table (constitution VII).
- **FR-017**: Codegen MUST be tested on fixture tables for each success path and each error
  class of FR-003, and the crate MUST test the generated lookup and data against the real
  tables. Generated files stay excluded from the coverage gate (NFR-6).
- **FR-018**: xtask MUST NOT touch any file outside `src/models.rs` and `src/models/`. Other
  modules' types it relies on (trait, cache, register, newtypes, errors) are consumed, not
  defined, by generated code.

### Key Entities

- **Model map** (`dynamixel.ron`): model number → family, display name, tables by `from_fw`,
  and (FR-012) supported protocols.
- **Control table** (`<model>.ron`): register rows (address, size, name) plus `type_info`
  (position scale) and `unit_info`.
- **Model type**: one generated public struct per table; holds the cache; carries registers,
  common-role data, scale and protocols as compile-time data.
- **`ActuatorKind`**: generated, closed, `#[non_exhaustive]` enum, one variant per model type,
  family-gated; the type the bus stores.
- **Common roles**: Torque Enable, Goal Position, Present Position, position limits (min/max or
  CW/CCW), hardware-fault register (optional).
- **Model lookup**: model number (+ firmware) → variant or `UnsupportedModel`; plus "needs
  firmware" and the Firmware Version location per model number.
- **Family feature**: `ax`, `mx`, `x-series`, `y-series`, `pro`; gates model types and variants.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Running codegen on unchanged tables leaves zero diff, locally and in the CI
  check, on every run.
- **SC-002**: For 100 % of mapped tables (69 today), an automated check confirms every common
  role, the Model Number and Firmware Version locations, the scale and the protocol set match
  the table data.
- **SC-003**: All 7 builds (each of 5 families alone, all, none) pass clippy with warnings
  denied, both std and `no_std` on `thumbv7em-none-eabihf`.
- **SC-004**: For 100 % of model numbers, a test checks the lookup at firmware 0, at each
  `from_fw` boundary and its predecessor, and at 255; and with the family off, that the error
  names that family.
- **SC-005**: Adding a model of an existing family takes a table, a map entry and one codegen
  run, with zero hand-written Rust lines.
- **SC-006**: Each FR-003 error class has a fixture test showing a non-zero exit, a message
  naming the file, and unchanged output files.
- **SC-007**: A one-register table change produces a diff in exactly one generated file.
- **SC-008**: xtask reaches at least 70 % line coverage.

## Assumptions

- **Already in place (PR #4), not re-planned**: `cargo xtask codegen` with the `.cargo` alias;
  parsing of `dynamixel.ron` and brace-style tables; `from_fw` validation; one file per table
  plus `src/models.rs`, formatted by `rustfmt`, with a generated-file header; type, module and
  register-constant naming (leading digit spelled out); duplicate type and register name
  errors; the unmapped-table warning; family `cfg` on modules, re-exports and variants;
  `#[non_exhaustive]` `ActuatorKind`; crate-internal `ActuatorKind::new(model_number,
  firmware, id)` with `from_fw` guards and `UnsupportedModel` naming the feature; `Actuator`
  impls for `id`, `model_number`, `name` on model types and `ActuatorKind`; public model types;
  crate-internal register constants; fixture tests in xtask; CI diff check.
- **Gaps this feature covers**: common-role data (FR-011), scale (FR-013), protocol set
  (FR-012), cache in model types (FR-009), trait impls beyond the three methods (FR-010), the
  firmware-dependence and Firmware/Model Number locations (FR-008), `ModelNumber` input
  (FR-006), the unknown-family check (FR-005), validate-before-write for the new checks
  (FR-003), warning-free builds for every family combination including none (FR-014; today a
  no-family build warns about an unused `Register::new`), documentation of source table
  (FR-016).
- **Dependencies on other modules**: `core` defines the extra `Actuator` trait methods, the
  cache type, `ModelNumber`, the scale and protocol-set types, and fixes the no-family warning
  in `register.rs`. `tables` adds protocol-support data and e-Manual verification metadata
  (FR-7.2, NFR-9) to the map or tables. `ci` runs the per-family builds of SC-003. `bus` does
  all wire I/O, the torque guard and the limit checks. This feature only generates code against
  their types.
- **Meanwhile defaults recorded**: model types public and `ActuatorKind` `#[non_exhaustive]`
  (OQ-1); trait covers torque, goal position, present position and limits only (OQ-2);
  `Position` SI form is radians as `f32`, so the scale is generated in radians (OQ-3); family
  list `ax, mx, x-series, y-series, pro` (FR-7.5); declared setup uses a `ModelNumber`
  newtype (FR-2.5).
- Role-to-register matching uses e-Manual register names already in the tables (e.g.
  "Min Position Limit" / "CW Angle Limit"); a family whose fault register has another name
  (e.g. Y-series "Error Code") is mapped only once the tables module confirms it against the
  e-Manual; until then it is "none".
- Register access and value range are not in the tables yet; this feature does not generate
  them.
- MX on Protocol 1 firmware tables are out of scope (roadmap); the lookup must not prevent one
  model number from mapping to tables under both protocols later.
