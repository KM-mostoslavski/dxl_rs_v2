# Feature Specification: xtask codegen (generated models, dispatch and family features)

**Feature Branch**: `dev` (spec directory `specs/006-xtask-codegen/`; PR branch `006-xtask-x1`,
step X1 of `specs/ideas/module-interfaces.md`)

**Created**: 2026-10-27

**Updated**: 2026-10-28 (aligned with module-interfaces.md §8; limits follow the operating mode;
the mode values live in the tables)

**Status**: Draft

**Input**: User description: "xtask (codegen) module of docs/srs/SRS.md"
→ SRS module `xtask` (SRS §9): paths `xtask/**`, `src/models.rs`, `src/models/**`. Covers
FR-7.3, FR-7.5, NFR-7, NFR-8, and the generated side of FR-2.3 and FR-3.2. Delivery step 3.
No-go: any hand-written file under `src/` other than the `pub mod models;` line in `src/lib.rs`.
This spec also covers the generated side of FR-2.4 (lookup errors) and, since 2026-10-28, of
FR-3.7 (mode source, FR-019). The SRS module text does not list them yet.

## Clarifications

### Session 2026-10-27

- Q: Do the generated `Actuator` impls perform wire operations or only supply per-model data? → A: Only per-model data; I/O is the bus's job (single responsibility).
- Q: A mapped table lacks protocol-support data: fail or generate as "protocols unknown"? → A: Codegen fails and names the table; it is a priority issue for the table maintainers.
- Q: Are the register constants on each model type public API? → A: No, internal; users have get/set methods for that, which makes their lives easier.

### Session 2026-10-28

- Q: Do position limits apply in every operating mode? → A: No. Limits follow the operating mode (architect, 2026-10-28; module-interfaces §2.4b). Codegen emits, per model, where Setup finds the mode: the "Operating Mode" register, XL-320's "Control Mode" register, or the CW/CCW Angle Limits for tables with no mode register (AX, MX on P1). Core owns the types and the decoding; the meaning of each value comes from the tables (next answer, FR-019).
- Q: Where do the mode value lists come from (value 5 is unbounded on X and MX(2.0), bounded on RH-P12-RN, so neither the register nor the family decides)? → A: From each table's `operating_modes: (bounded: [...], unbounded: [...], no_position: [...])` (architect, 2026-10-28; module-interfaces §2.4b, §3.1). Codegen emits the lists with the register in `ModeSource::Register { register, values }`; core holds no per-family list, so a new model or family is a data change only.
- Q: Does the per-actuator cache hold torque and fault state? → A: No. It holds the Id, the limit mode and the limits. The bus keeps torque and fault state as one byte per slot (module-interfaces §2.5, conflict 1).
- Q: Are register access and MX on Protocol 1 in scope? → A: Yes (module-interfaces §8 conflicts 2, 3). Access and area are generated on every register, and the lookup is per protocol, so the MX P1 tables of tables T2 need no xtask change.

## User Scenarios & Testing *(mandatory)*

The "users" of this module are the crate maintainer, who adds and fixes actuator models, and
the other crate modules (`bus` above all) that build on the generated code. Robot application
and embedded developers meet the result as model types, `ActuatorKind` and family features.

Codegen already exists (PR #4). This spec covers what it still lacks; see Assumptions for what
is in place.

### User Story 1 - Common operations work on every model (Priority: P1)

The bus needs the same facts about every actuator, whatever its model: where Torque Enable,
Goal Position and Present Position live, which registers hold the position limits, whether a
hardware-fault register exists, how raw position ticks map to radians, where the operating mode
is found (the goal check follows it), and where the per-actuator cache is. Codegen produces these
facts for every model type, and `ActuatorKind` hands them out by `match`. No model needs
hand-written code.

**Why this priority**: Delivery step 4 (bus, torque guard, limits at Init) cannot start
without it. Today the generated `Actuator` impls only give `id`, `model_number` and `name`.

**Independent Test**: For one model per layout tier (e.g. XM430-W350, AX-12A, XL-320,
PH54-200-S500, YM070-210-M001), build the `ActuatorKind`, ask it for each common register, the
limit form, the fault register, the mode source and the position scale, and compare with the
model's `.ron` table. Write the cache through `ActuatorKind` and read it back.

**Acceptance Scenarios**:

1. **Given** an XM430-W350, **When** the bus asks for its Goal Position register, **Then** it
   gets address 116, size 4, access RW, area RAM, exactly as the table lists it.
2. **Given** an AX-12A, **When** the bus asks for its position limits, **Then** it gets the CW
   and CCW Angle Limit registers, marked as the CW/CCW form, not the min/max form.
3. **Given** a model whose table has no hardware-fault register, **When** the bus asks for it,
   **Then** the answer is "none", not a guessed address.
4. **Given** any model, **When** the bus asks for its position scale, **Then** it gets the
   zero, minimum and maximum raw values and radians from that table's `type_info`.
5. **Given** an `ActuatorKind` holding any model, **When** the bus updates its cache (limit
   mode, limits), **Then** the change is visible through the same value, with no copy and no
   allocation. The cache holds no torque or fault state.
6. **Given** an XM430-W350, an XL-320 and an AX-12A, **When** the bus asks where to find the
   operating mode, **Then** it gets the Operating Mode register (address 11, 1 byte) with the
   value lists of its table's `operating_modes` (bounded 3; unbounded 4, 5; no position 0, 1,
   16), the Control Mode register (address 11, 1 byte) with bounded 2, no position 1, and "from
   the CW/CCW Angle Limits" with no register and no lists, as the tables give them.

---

### User Story 2 - Setup turns any reply into the right model (Priority: P1)

During Setup the bus has a protocol, a model number and, sometimes, a firmware version. It asks
the generated code for the matching model type. The lookup is per protocol: a model number can
have its own table list on P1 and on P2, or tables on one protocol only. When a model has
firmware-dependent tables, the bus also needs to know that it must read Firmware Version, and
where that register is, because a P1 ping carries neither model number nor firmware. It also
needs to know which protocols the model speaks, so a model on the wrong bus fails at Init, not
in the first Cycle.

**Why this priority**: Autoscan and declared setup (FR-2.3, FR-2.5) both depend on it. Errors
must surface at Init (constitution II).

**Independent Test**: For every (protocol, model number) in `dynamixel.ron`, call the lookup
with the firmware values around each `from_fw` boundary and check the variant; query the
firmware flag, the Firmware Version and Model Number locations and the protocol set, and compare
with the tables. Repeat with each family switched off.

**Acceptance Scenarios**:

1. **Given** model number 1190 (XL330-M077) on Protocol 2 and firmware 52, **When** the lookup
   runs, **Then** it returns the `Xl330M077Fw52` variant; with firmware 53 it returns
   `Xl330M077`.
2. **Given** model number 1190 on Protocol 2, **When** the bus asks whether it needs the
   firmware version, **Then** the answer is yes, with the Firmware Version register's address
   and size; for model 1020 (one table) the answer is no.
3. **Given** a build without the `x-series` feature, **When** the lookup runs for 1020 on
   Protocol 2, **Then** it returns `UnsupportedModel` naming `x-series`, the actuator's Id and
   the protocol.
4. **Given** a model number not in `dynamixel.ron`, **When** the lookup runs on either protocol,
   **Then** it returns `UnsupportedModel` with no feature.
5. **Given** a model, **When** the bus asks which protocols it speaks, **Then** it gets the set
   recorded for it in the control-table data (e.g. AX-12A: P1 only); a lookup on a protocol
   where the model has no table (XL-320 on P1) returns `UnsupportedModel` with no feature.
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
failure class of FR-003 (e.g. missing file, bad `from_fw` order, duplicate names, unknown
family, missing `source`/`protocols`/`access`/`area`, missing common register, no mode source,
a mode row without `operating_modes`, bad scale, Model Number at an unexpected address). Check
exit status, message and that existing output files are unchanged.

**Acceptance Scenarios**:

1. **Given** unchanged tables, **When** codegen runs twice, **Then** the second run produces
   byte-identical files.
2. **Given** one register's address changed in one table, **When** codegen runs, **Then**
   the diff is confined to that model's file.
3. **Given** a mapped table that lacks Goal Position, **When** codegen runs, **Then** it fails,
   naming the table and the missing role, and writes nothing.
4. **Given** a table on disk that no model number points to and that `not_mapped` does not
   list, **When** codegen runs, **Then** it succeeds and prints one warning per file. A table
   listed in `not_mapped` (after tables T1: `ax_12_plus`, `ax_18f`, `xw430_t200`) is skipped
   without a warning.
5. **Given** generated files, **When** a reviewer opens one, **Then** it is formatted, says it
   is generated and from which table, and documents the model name, model number, firmware
   range per protocol and whether the table is verified against the e-Manual.

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
  codegen still succeeds (FR-7.4). Listed in `not_mapped`: skipped without a warning.
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
- Position limits in CW/CCW form (AX, XL-320, MX on P1) next to min/max form (X, MX(2.0), PRO,
  Y): both are carried as data, with the form named.
- A model with no hardware-fault register (AX): reported as "none"; fault detection then
  relies on the status packet error byte alone.
- A table with no mode register (AX, MX on P1): its mode source is "from the CW/CCW Angle
  Limits". A table with neither a mode register nor CW/CCW Angle Limits, with an Operating Mode
  register beside CW/CCW Angle Limits, or with both an Operating Mode and a Control Mode
  register: codegen fails, naming the table. XL-320 (Control Mode and CW/CCW Angle Limits) is
  the one table with both a mode register and CW/CCW limits.
- A mode register without `operating_modes`, `operating_modes` in a table with no mode
  register, or one value in two of its lists: codegen fails, naming the table (and the value).
  An empty list is fine (Y and XL-320 have no unbounded mode); a value in no list is decided at
  Setup (`UnknownOperatingMode`), not by codegen.
- A model claiming P1 support whose common registers sit above address 255 (P1 addresses are
  8-bit): codegen fails, naming the model and register.
- A table whose Model Number is not at the address and size every other table uses: codegen
  fails, since a P1 autoscan reads that location before it knows the model.
- No family enabled: `ActuatorKind` has no variants, every lookup fails with
  `UnsupportedModel`, and the build has no warnings.
- `rustfmt` is missing or fails: codegen fails with a clear message; the CI diff check
  catches any unformatted output.
- One model number with tables under both protocols (X-series today), or under one protocol
  only (AX and XL-320 today, MX(1.0) on P1 after tables T2): the lookup is per protocol, so each
  protocol picks from its own table list.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `cargo xtask codegen` MUST read `dynamixel.ron` and every table it references,
  and write one model type per table plus the `models` module with `ActuatorKind`, into
  `src/models.rs` and `src/models/` only. Output is committed; nothing is generated at build
  time (FR-7.3, constitution V).
- **FR-002**: Output MUST be deterministic and formatted: the same inputs give byte-identical
  files. Each file states that it is generated and names its source table (NFR-8).
- **FR-003**: Codegen MUST validate all input before writing any file, and report every
  problem in one run. On any error it exits non-zero, names the file and the problem, and
  leaves the committed output unchanged. Error classes: missing or unparsable file; a malformed
  model map (`from_fw` not starting at 0 or not strictly increasing, no protocol list or an
  empty one, `default` missing or not P2 when both lists exist, a file under two model numbers,
  a file both mapped and in `not_mapped`); unknown family (FR-005); a mapped table without
  `source` or `protocols`, or a row without `access` or `area` (FR-012); a protocol list naming
  a table that lacks that protocol; a missing common register, or one with the wrong access or
  area (FR-011); no mode source, an Operating Mode register beside CW/CCW limits, or two mode
  registers (FR-019); Model Number or Firmware Version location differing between tables
  (FR-008); a P1-listed table with a common register above address 255; an inconsistent
  position scale (FR-013); duplicate type or register identifiers, or a reserved module name;
  a mode row without `operating_modes`, `operating_modes` without a mode row, or overlapping
  value lists (FR-019).
- **FR-004**: A table with no model number in `dynamixel.ron` MUST be skipped. It gives one
  warning per file unless `not_mapped` lists it, and codegen MUST still succeed (FR-7.4).
- **FR-005**: Every model type and `ActuatorKind` variant MUST be gated by its family feature.
  The family names are exactly `ax`, `mx`, `x-series`, `y-series`, `pro` (FR-7.5); any other
  name in `dynamixel.ron` is a codegen error.
- **FR-006**: The lookup MUST map a protocol (P1 or P2), a model number and a firmware version
  to the `ActuatorKind` variant from that protocol's table list whose `from_fw` is the highest
  not above the firmware (FR-2.3). It takes the actuator's `Id` and accepts the `ModelNumber`
  newtype that declared setup uses (FR-2.5). Nothing in it may rule out one model number
  having tables under both protocols.
- **FR-007**: The lookup MUST fail with `SetupError::UnsupportedModel { id, model_number,
  protocol, feature }`. `feature` is the family name when the model has tables on that
  protocol but its family is off, and none when the model number is unknown or has no tables on
  that protocol (FR-2.4; the error moved from `Error` into `SetupError` and gained `id` and
  `protocol`, module-interfaces §8 conflicts 6 and 19).
- **FR-008**: For each model number and protocol, the generated code MUST tell, inside the
  crate, whether its table depends on firmware and, if so, where its Firmware Version register
  is. It MUST also expose, inside the crate, the one Model Number location shared by all
  tables, checked at codegen time. None of these is on the public trait (module-interfaces §8
  conflict 17).
- **FR-009**: Each model type MUST hold the per-actuator cache (the core-defined type: the
  Id, the limit mode and the limits, set at Setup) and give crate-internal access to it;
  `ActuatorKind` MUST give the same access by `match`, with no `dyn` and no allocation (NFR-4).
  The cache holds no torque or fault state; the bus keeps those, one byte per slot.
- **FR-010**: Each model type MUST implement the core-defined crate-internal description trait
  (model data, cache, mutable cache) and the public `Actuator` trait, and `ActuatorKind` MUST
  implement both by delegating with `match` (FR-3.2). The generated impls only supply
  per-model data (registers, scale, mode source, cache); they perform no I/O, which the bus does
  through `ActuatorRef` (single responsibility). The operations of SRS OQ-2 meanwhile (torque,
  goal position, present position and limits) reach the bus through that data. While core's
  `Actuator` trait still requires `id`, `model_number` and `name`, the generated impls spell
  them out.
- **FR-011**: Each model type MUST expose, as compile-time data, the registers that serve the
  common operations: Torque Enable, Goal Position, Present Position; the position limits in
  min/max or CW/CCW form, with the form named; and the hardware-fault register or "none".
  Roles are matched to table rows by their e-Manual register names; codegen never guesses an
  address. Every register carries the access (R or RW) and area (EEPROM or RAM) its row gives,
  and codegen fails when a role register has another access or area than its role needs.
- **FR-012**: Each model type MUST expose the protocols it supports (P1, P2 or both), taken
  only from explicit control-table data, never inferred from registers or family. Every
  protocol list in `dynamixel.ron` that names a table must be one of that table's protocols.
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
  firmware range per protocol, source table, and whether the table is verified against the
  e-Manual (URL and date) or unverified (FR-7.2, constitution IV and VII).
- **FR-017**: Codegen MUST be tested on fixture tables for each success path and each error
  class of FR-003, and the crate MUST test the generated lookup and data against the real
  tables. Generated files stay excluded from the coverage gate (NFR-6).
- **FR-018**: xtask MUST NOT touch any file outside `src/models.rs` and `src/models/`. Other
  modules' types it relies on (traits, model data, cache, register, mode source, newtypes,
  errors) are consumed, not defined, by generated code.
- **FR-019**: Each model type MUST expose, as compile-time data, where Setup finds its
  operating mode and what each mode value means, because limits follow the operating mode
  (architect, 2026-10-28): the "Operating Mode" register when the table has one (X, MX(2.0),
  PRO, PRO+, RH-P12-RN, Y) or XL-320's "Control Mode" register, each with the three value lists
  (`bounded`, `unbounded`, `no_position`) of the table's `operating_modes`; or "from the CW/CCW
  Angle Limits" when the table has no mode register (AX, MX on P1). Codegen MUST fail, naming
  the table, when a table has neither a mode register nor CW/CCW Angle Limit rows, has an
  Operating Mode row beside CW/CCW Angle Limits, has both an Operating Mode and a Control Mode
  row, has a mode row without `operating_modes` or `operating_modes` without a mode row, or has
  one value in two of its lists. A mode register MUST be one byte, RW, in EEPROM, as the bus
  reads it once at Setup (contract §4.3). The value lists are table data, copied by codegen; the
  mode types and the decoding of a value with the lists belong to core.

### Key Entities

- **Model map** (`dynamixel.ron`): model number → family, display name, `also` names, and one
  table list per protocol (`p1`, `p2`) ordered by `from_fw`, with `default: P2` when both
  exist; plus `not_mapped` tables, each with a reason.
- **Control table** (`<model>.ron`): `source` (unverified, or e-Manual URL and date),
  `protocols`, `operating_modes` (with a mode row only), register rows (address, size, name,
  access, area), `type_info` (position scale) and `unit_info`.
- **Model type**: one generated public struct per table; holds the cache; carries registers,
  common-role data, mode source, scale and protocols as compile-time data.
- **`ActuatorKind`**: generated, closed, `#[non_exhaustive]` enum, one variant per model type,
  family-gated; the type the bus stores.
- **Common roles**: Torque Enable, Goal Position, Present Position, position limits (min/max or
  CW/CCW), hardware-fault register (optional), mode source (the Operating Mode or Control Mode
  register with its table's value lists, or the CW/CCW Angle Limits).
- **Model lookup**: protocol + model number (+ firmware) → variant or `UnsupportedModel`; plus
  "needs firmware" and the Firmware Version location per (protocol, model number).
- **Family feature**: `ax`, `mx`, `x-series`, `y-series`, `pro`; gates model types and variants.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Running codegen on unchanged tables leaves zero diff, locally and in the CI
  check, on every run.
- **SC-002**: For 100 % of mapped tables (69 today), an automated check confirms every common
  role, the mode source, the Model Number and Firmware Version locations, the scale and the
  protocol set match the table data.
- **SC-003**: All 14 builds pass clippy with warnings denied: 7 family combinations (each of
  the 5 families alone, all, none), each built std and `no_std` on `thumbv7em-none-eabihf`.
- **SC-004**: For 100 % of (protocol, model number) pairs, a test checks the lookup at
  firmware 0, at each `from_fw` boundary and its predecessor, and at 255; and with the family
  off, that the error names that family.
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
- **Gaps this feature covers**: the new table and map schema (`source`, `protocols`, `access`,
  `area`, `operating_modes`, per-protocol lists, `not_mapped`); common-role data with access and
  area (FR-011); mode source (FR-019); scale (FR-013); protocol set (FR-012); cache in model types
  (FR-009); the description trait and the trait impls (FR-010); the per-protocol lookup returning
  `SetupError` (FR-006, FR-007); the firmware-dependence and Firmware/Model Number locations
  (FR-008); `ModelNumber` input (FR-006); the unknown-family check (FR-005); validate-before-write
  for every check (FR-003); warning-free builds for every family combination including none (FR-014;
  today a no-family build warns about an unused `Register::new`); documentation of source table and
  verification state (FR-016).
- **Dependencies on other modules**: `core` defines `Actuator`, the description trait, the
  model-data, cache, register, scale, protocol-set and mode-source types, `ModelNumber` and
  `SetupError`, decodes a mode value with the lists codegen emits (core holds no per-family list),
  and fixes the no-family warning in `register.rs` (core-1). `protocol` defines `ProtocolVersion`
  (PR #7, merged). `tables` adds `source`, `protocols`, `access`/`area`, `operating_modes` and the
  per-protocol map (T1), and the MX P1 tables later (T2). `ci` runs the per-family builds of SC-003.
  `bus` does all wire I/O, the torque guard, the mode read and the limit checks. This feature only
  generates code against their types.
- **Meanwhile defaults recorded**: model types public and `ActuatorKind` `#[non_exhaustive]`
  (OQ-1); the operations served are torque, goal position, present position and limits only
  (OQ-2); `Position` SI form is radians as `f32`, so the scale is generated in radians (OQ-3);
  family list `ax, mx, x-series, y-series, pro` (FR-7.5); declared setup uses a `ModelNumber`
  newtype (FR-2.5).
- Role-to-register matching uses e-Manual register names already in the tables (e.g.
  "Min Position Limit" / "CW Angle Limit", "Operating Mode" / "Control Mode"); a family whose
  fault register has another name (e.g. Y-series "Error Code") is mapped only once the tables
  module confirms it against the e-Manual; until then it is "none".
- Access and area are in every row after tables T1 and are generated on every register. Value
  range, default and unit come per model with e-Manual verification; this feature does not
  generate them yet.
- MX on Protocol 1 is in scope: the lookup is per protocol now, and tables T2 adds the MX(1.0)
  P1 tables with no xtask change. They have no mode register and no `operating_modes`, so
  their mode source is the CW/CCW Angle Limits.
