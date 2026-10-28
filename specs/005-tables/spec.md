# Feature Specification: Control tables (verification flags, capabilities, per-protocol model map)

**Feature Branch**: `005-tables` (spec written on `dev`; no branch created yet)

**Created**: 2026-10-27

**Status**: Draft

**Input**: User description: "tables module of docs/srs/SRS.md"
→ SRS module `tables` (SRS §9): paths `control_tables/**`; covers FR-7.1, FR-7.2, FR-7.4,
NFR-9; delivery steps 3 and 7. Rule: every row comes from the e-Manual; online verification is
deferred to the first use of a model, and every table not yet verified says so (FR-7.2, amended
2026-10-27). No-go: `src/**`. Updated 2026-10-28 for the architect's decision "limits follow the
operating mode" (`specs/ideas/module-interfaces.md` §2.4b): the tables give the mode source.

## Clarifications

### Session 2026-10-27

- Q: Are the MX Protocol 1 tables (MX-28, MX-64, MX-106 on Protocol 1 firmware) written in this
  feature, or does this feature only make the map able to hold them? → A: Written in this
  feature. The same rule holds for every actuator that supports both protocols: it gets a table
  for each protocol, and Protocol 2 is the default.
- Q: Y-series and PRO (25 tables) are unverified; verify them in this feature, or unmap them
  until a later feature does? → A: Neither. Bulk e-Manual verification is deferred: the
  registers are normally correct, the tables are usable, and a mistake is easy to fix when
  found. A model is verified when it is first needed, one at a time. Every table not yet
  double-checked is flagged explicitly as unverified (`.ron` metadata, generated docs, table
  check report). Nothing is unmapped for being unverified; Y-series and PRO stay mapped and
  flagged.
- Q: Which models are verified first? → A: AX-12 (AX-12A, and AX-12+, which also needs its
  model number mapped) and MX-64 (MX-64(2.0) and MX-64 on Protocol 1).
- Q: Which per-register attributes does this feature add to the ~4,900 rows? → A: (a) access (R/RW) and memory area (EEPROM/RAM) on every table: simple fields for development, possibly enough for production; range, default and unit later, per model on verification.

### Session 2026-10-28

- Q: The firmware applies position limits only in position mode, so the bus must know each
  actuator's operating mode. What do the tables give for it? → A: Limits follow the operating
  mode (architect decision). Codegen takes each model's mode source from its table: the
  "Operating Mode" register (X, MX(2.0), PRO, PRO+, RH-P12-RN, Y), the "Control Mode" register
  (XL-320), or, on models with neither (AX, MX on Protocol 1), the CW/CCW Angle Limit pair. Every
  table whose e-Manual page lists one of those registers carries its row with the page's address,
  size, access and area; the MX Protocol 1 tables carry none. The table check requires exactly
  one mode source per table.
- Q: XW430-T200 and XW430-T333 give the same Model Number (1,270) on the e-Manual. Correct it or
  keep it? → A: Both pages re-checked on 2026-10-28 give 1,270. XW430-T200 is recorded under
  1270, like any product that shares a number (User Story 5, AS4).

## User Scenarios & Testing *(mandatory)*

The "users" of this module are the crate maintainer who adds or fixes actuator models, the
reviewer who must trust a table change, and, through codegen, every application developer whose
actuator is driven by the addresses and sizes in these files.

### User Story 1 - See at a glance whether a table was checked (Priority: P1)

A maintainer or application developer opens any table and sees, at its top, whether it was
verified against the e-Manual. A verified table names the e-Manual page and the date. An
unverified table says "unverified" explicitly. The table check lists every unverified table
but does not fail CI because of it.

**Why this priority**: Under the amended FR-7.2, unverified tables stay in use. That is only
safe if nobody mistakes one for a checked table. No table carries any record today, so the flag
must exist before any table can be marked verified.

**Independent Test**: Add the flag to every table; run the table check and see each table
listed as verified (URL and date) or unverified, with a success exit; remove the flag from one
table and see the check fail; run codegen and see the generated model types unchanged.

**Acceptance Scenarios**:

1. **Given** any table, **When** the maintainer reads it, **Then** its top level states either
   verified (e-Manual URL and date) or unverified, never neither.
2. **Given** a table with no flag at all, **When** the table check runs, **Then** it fails and
   names that table.
3. **Given** tables flagged unverified, **When** the table check runs, **Then** it lists them by
   name and still exits with success.
4. **Given** a verified record that cites a page outside `emanual.robotis.com` or an invalid
   date, **When** the table check runs, **Then** it rejects the record.
5. **Given** the flag is added, **When** codegen runs, **Then** the generated model types are
   unchanged (the flag is metadata, not a register), and the flag is in a form codegen can carry
   into each model type's documentation.

---

### User Story 2 - AX-12 and MX-64 verified first (Priority: P1)

The maintainer verifies the priority models against their own e-Manual pages, one model at a
time: AX-12A, AX-12+, MX-64(2.0) and MX-64 on Protocol 1 (a new table, written from its own
page). AX-12+ gets its model number recorded in the map.
Each verified table records its URL and date. Any other model is verified the same way later,
when it is first needed, never in bulk.

**Why this priority**: These are the actuators in use now. A wrong address or size in them can
corrupt EEPROM on real hardware (constitution IV rationale). The other tables wait until a
model is needed.

**Independent Test**: Run the table check and see the four priority tables reported as verified
with their URL and date, and AX-12+ recorded under its model number. Pick any row of one of them
and confirm it against the cited page.

**Acceptance Scenarios**:

1. **Given** the AX-12A, AX-12+, MX-64(2.0) and MX-64 Protocol 1 tables, **When** they are
   verified, **Then** each one carries its own record (its own model's page, not a sibling's),
   even where layouts are identical.
2. **Given** a row the e-Manual lists at a different address or size, **When** the table is
   verified, **Then** the row is corrected and the correction is listed in the PR.
3. **Given** a row the e-Manual does not list, **When** the table is verified, **Then** the row
   is removed, not kept "just in case".
4. **Given** the AX-12+ table, **When** the e-Manual gives its model number, **Then** the map
   records it under that number. If the number is already used (AX-12A, 12), the product name
   goes under the existing entry and the file stays in the not-mapped record with the shared
   number as its reason (User Story 5, AS4): codegen still skips it, now without a warning.
5. **Given** a model that is not a priority model and not yet needed, **When** this feature
   ends, **Then** its table is still mapped, still compiled in, and flagged unverified.
6. **Given** a model that becomes needed later, **When** it is verified, **Then** that one
   model is verified in its own PR, and its flag changes from unverified to verified.

---

### User Story 3 - Know what a model can do, not only where its registers are (Priority: P2)

A maintainer (and, later, the bus and protocol modules through codegen) can read, for each
model and table, which protocols it speaks, which instructions it supports, and the per
register attributes in scope. The data comes from the e-Manual, not from register names.

**Why this priority**: FR-7.2 lists capability claims (instructions, protocol,
firmware-dependent layout, model number) as e-Manual data. Today the tables hold only address,
size and name per row, plus scaling for a few registers; the protocol of a table is inferred by
the analysis script from register names, which groups XL-320 with the Protocol 1 tables. Since
2026-10-28 position limits follow the operating mode, so each table must also tell codegen where
the actuator's mode comes from.

**Independent Test**: For a verified table, read its capability data and compare each claim
with the e-Manual page; regenerate the analysis and see the protocol grouping come from the
recorded data, not from a heuristic; remove the "Operating Mode" row from a copy of an X-series
table and see the check fail.

**Acceptance Scenarios**:

1. **Given** any table, **When** a maintainer reads it, **Then** it states the protocol its
   layout belongs to; a verified table also states the instructions the e-Manual lists for that
   model.
2. **Given** a table, **When** a maintainer reads a register, **Then** it shows the attributes
   in scope for this feature (see FR-007), each one taken from the e-Manual.
3. **Given** the e-Manual gives no value for an attribute, **When** the table is written,
   **Then** the attribute is left absent, never filled from another model or a default.
4. **Given** the analysis is regenerated, **When** a maintainer reads it, **Then** each table's
   protocol comes from its recorded data.
5. **Given** any table, **When** the table check runs, **Then** it finds exactly one mode source:
   an "Operating Mode" row, a "Control Mode" row (XL-320 only), or, with neither, the CW/CCW
   Angle Limit pair. A table with no source, with both registers, or with an "Operating Mode" row
   beside the CW/CCW pair fails the check.
6. **Given** a model whose e-Manual page lists an "Operating Mode" or "Control Mode" register,
   **When** its table is written or checked against the page, **Then** that row is present with
   the page's address, size, access and area.

---

### User Story 4 - One model, a table per protocol, Protocol 2 by default (Priority: P2)

Some actuators can be switched between Protocol 1 and Protocol 2 (60 of the 72 tables have a
`Protocol Type` register; its values on the model's page say which protocols it offers), and MX
actuators exist with Protocol 1 firmware. The model map says,
for each such model, which table applies on a Protocol 1 bus and which on a Protocol 2 bus,
with firmware variants under each, and marks Protocol 2 as the default.

**Why this priority**: The amended SRS requires tables for both protocols on every
dual-protocol model. Today the map holds one list of tables per model number, ordered by
firmware only, so it cannot express a second protocol.

**Independent Test**: Write a map entry for one model with a table for each protocol; the table
check accepts it and reports, for each protocol, which table a given firmware version selects,
and which protocol is the default.

**Acceptance Scenarios**:

1. **Given** a model whose e-Manual lists one control table for both protocols, **When** the
   map is read, **Then** the same table is selected on either bus.
2. **Given** a model whose e-Manual lists a different table per protocol, **When** the map is
   read for one protocol, **Then** only that protocol's table is selected.
3. **Given** a model that supports both protocols, **When** a consumer reads the map without
   naming a protocol (generated docs, the table check summary), **Then** the Protocol 2 table
   is the one used.
4. **Given** a model with no table for a protocol, **When** the map is read for that protocol,
   **Then** no table is selected (the bus reports `UnsupportedModel`).
5. **Given** the existing map entries, **When** the new format is adopted, **Then** each one
   keeps selecting the same table under the protocol it already serves.

---

### User Story 5 - Ship the tables and model numbers that are missing (Priority: P2)

The maintainer writes the MX tables for Protocol 1 firmware (MX-28, MX-64, MX-106), maps every
dual-protocol model under both protocols, and records the remaining tables without a model
number (AX-18F, XW430-T200) under the number their page gives. MX-64 on Protocol 1 is verified
(User Story 2); the others are written from the e-Manual and flagged unverified until they are
needed.

**Why this priority**: The architect requires MX on Protocol 1 in this feature. The two
unmapped tables only need their model number recorded. All of it builds on the flag (User
Story 1) and the per-protocol map (User Story 4).

**Independent Test**: The table check reports zero unmapped tables without a reason, a
Protocol 1 table for each MX model, and both protocols for every dual-protocol model; codegen no
longer warns about skipped files.

**Acceptance Scenarios**:

1. **Given** MX-28, MX-64 or MX-106, **When** the map is read for Protocol 1, **Then** it
   selects that model's Protocol 1 table, under the model number the e-Manual gives for the
   Protocol 1 firmware (e.g. MX-64, model 310).
2. **Given** an MX Protocol 1 table other than MX-64, **When** it is written, **Then** every
   row comes from that model's e-Manual page and the table is flagged unverified.
3. **Given** AX-18F or XW430-T200, **When** the e-Manual gives its model number, **Then** the
   map records it with that number (AS4 when the number is already used) and codegen no longer
   warns about it.
4. **Given** an unmapped table whose e-Manual model number is already used by a mapped model,
   **When** it is recorded, **Then** it does not create a second entry for that number: its
   product name goes under the existing entry, and the file stays in the not-mapped record with
   the shared number as its reason. Codegen skips it without a warning; the PR states it.
5. **Given** the e-Manual gives no model number for one of them, **When** the work ends,
   **Then** the table stays unmapped and the map records why.
6. **Given** an MX Protocol 1 table, **When** it is written, **Then** it has no "Operating Mode"
   row and carries the CW/CCW Angle Limit pair, from which the mode is derived. If the page lists
   an Operating Mode register, the work stops and the architect is told.

---

### Edge Cases

- A mistake is found in an unverified table (a user reports a wrong address): the row is fixed
  from the model's e-Manual page and the PR lists it. The table stays flagged unverified unless
  the whole table is checked in that PR.
- A verified table is changed later: it is re-checked against the page, and the new record (URL
  and date) replaces the old one.
- The e-Manual page moves or is rewritten after a table was verified: the record keeps the URL
  and date it was checked on; nothing is redone until the table is changed.
- The e-Manual is unreachable during the work: no table is written, corrected or marked
  verified. A row is never written from memory, a third-party SDK or a sibling table
  (constitution IV).
- Two models share one byte-identical layout (14 distinct layouts cover all 72 tables):
  verifying one does not verify the other; each keeps its own flag until checked against its
  own page, since ranges, defaults and units can differ even when addresses do not.
- A register is renamed between pages (e.g. "Bus Watchdog" vs "BUS Watchdog" both appear
  today): the table uses the name the model's own page gives; any cross-model name mismatch is
  reported by the analysis, not silently unified.
- A correction changes an address or size that generated code already uses: the regenerated
  model files change with it in the same PR, and the PR says so (CI fails on a stale diff).
- A firmware variant's boundary (`from_fw`) is not stated on the e-Manual page: the variant is
  not mapped until the boundary is found there.
- The verified record names a page for a different model than the table: the check cannot
  catch this alone; the reviewer confirms the model name on the cited page.
- A date in the record is in the future or not a real calendar date: the check rejects it.
- A table misses the "Operating Mode" row its page lists, or has it at another address or size:
  the row is fixed from the page like any other correction, and the table still gives exactly
  one mode source.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Every control table MUST carry one verification flag at its top level: either
  verified, with the e-Manual URL and the date of the check (SRS §8 "Meanwhile" default
  `source: (url: "...", verified: "YYYY-MM-DD")`), or explicitly unverified. A table with
  neither is an error.
- **FR-002**: The URL in a verified record MUST be on `emanual.robotis.com`, and the date MUST
  be a real calendar date not later than the day it was written.
- **FR-003**: Every value in a table (register name, address, size, any register attribute,
  every capability claim) and every model number and firmware boundary in the map MUST come
  from the official e-Manual, never from memory, a third-party SDK or another model's table. A
  value the e-Manual does not give MUST be left out (constitution IV, FR-7.2 amended).
- **FR-004**: A table MUST be marked verified only after every value in it was checked against
  the model's online e-Manual page on the recorded date. A verified table that changes MUST get
  a new record with the date of the new check.
- **FR-005**: Each table MUST record the protocol its layout belongs to, taken from the
  e-Manual, so no tool has to infer it from register names.
- **FR-006**: Each verified model MUST record the instructions the e-Manual lists as supported
  (ping, read, write, sync and bulk instructions and the others the page lists), per protocol.
- **FR-007**: Each register row in every table MUST carry, in addition to name, address and
  size, two simple fields from the e-Manual: access (R or RW) and memory area (EEPROM or RAM).
  Codegen and the bus need them for the torque guard and the EEPROM limit reads. Range, default
  and unit (the rest of the SRS FR-7.2 list) are added per model when it is verified, and are
  out of scope here.
- **FR-008**: The model map MUST be able to give, per model, a separate list of firmware-ordered
  tables per protocol, MUST allow the same table to serve both protocols, and MUST mark
  Protocol 2 as the default for every model that supports both.
- **FR-009**: Adopting the per-protocol map format MUST leave every existing model number
  selecting the same table under the protocol it serves today, so generated model types do not
  change because of the format alone.
- **FR-010**: Within one model number and one protocol, firmware boundaries MUST be strictly
  increasing, start at 0, and each MUST name a table file that exists.
- **FR-011**: A table not referenced by the map MUST be listed in the map's "not mapped"
  record with its reason (no model number found, or the same model number as a mapped entry),
  so the codegen warning (FR-7.4) is never the only trace. Being unverified MUST NOT be a reason
  to unmap a table.
- **FR-012**: A model number MUST appear at most once in the map; a second product with the
  same number (e.g. a predecessor with an identical layout) MUST be recorded under the
  existing entry, never as a second entry.
- **FR-013**: Every table mapped today, including the 25 Y-series and PRO tables, MUST stay
  mapped and flagged unverified until its model is verified; the family feature names stay as
  they are (FR-7.5).
- **FR-014**: A repeatable table check MUST report, for every table: verified (URL and date) or
  unverified, mapped or not mapped with reason, protocol recorded or not; for every
  dual-protocol model, its table per protocol and its default; and for the map: duplicate
  numbers, missing files, bad firmware ordering. It MUST fail on a missing flag, an invalid
  record, a map error or a table without exactly one mode source (FR-020), and MUST NOT fail
  because a table is unverified, so CI can run it.
- **FR-015**: The analysis report MUST be regenerated from the recorded data after this
  feature and MUST take each table's protocol from FR-005.
- **FR-016**: Each table MUST stay readable by the existing codegen: new metadata MUST NOT
  change any generated model type except through a correction of a register. The verification
  flag MUST be readable by codegen so it can be shown in each generated model type's
  documentation (rendering it there is the `xtask` module's job).
- **FR-017**: Every PR that touches `control_tables/` MUST list, per changed table, its flag
  (verified with URL and date, or unverified) and every corrected or removed row (NFR-9
  amended).
- **FR-018**: AX-12A, AX-12+, MX-64(2.0) and MX-64 on Protocol 1 MUST be verified in this
  feature, each against its own e-Manual page. No other table is verified in bulk; any other
  model is verified one at a time, in its own PR, when it is first needed.
- **FR-019**: Every model that supports both protocols MUST have a table mapped for each
  protocol: the same table where the e-Manual gives one control table for both, a separate
  table where it gives one per protocol. The MX(2.0) models are among them. MX-28, MX-64 and
  MX-106 on Protocol 1 firmware are not: the e-Manual gives them their own model numbers, so
  they are Protocol 1 only models. This feature writes their tables and maps them under those
  numbers (User Story 5).
- **FR-020**: Every table MUST give exactly one source for the actuator's operating mode, from
  which the bus decides whether position limits apply (architect, 2026-10-28): the "Operating
  Mode" row where the model's page lists that register, the "Control Mode" row on XL-320, or,
  on models with neither (AX, MX on Protocol 1), the CW/CCW Angle Limit pair. A mode register
  the page lists MUST be in the table with the page's address, size, access and area; a table
  whose page lists none MUST NOT carry one. Both mode registers in one table, or an "Operating
  Mode" row beside the CW/CCW pair, is an error (codegen rejects them too).

### Key Entities

- **Control table**: one file per model (and per firmware variant): register rows plus model
  data (scaling, verification flag, protocol, capabilities).
- **Register row**: one register of a table: name, address, size, and the attributes FR-007
  settles.
- **Verification flag**: either verified (the e-Manual URL and the date a table was checked
  against it) or explicitly unverified.
- **Capability data**: per model or table: protocol of the layout, supported instructions per
  protocol, firmware-dependent layout (variants).
- **Model map entry**: one model number → family, display name, per protocol a
  firmware-ordered list of tables, and the default protocol.
- **Not-mapped record**: a table file the map leaves out, with the reason.
- **Mode source**: where an actuator's operating mode comes from: its "Operating Mode" row,
  XL-320's "Control Mode" row, or the CW/CCW Angle Limit pair. Derived by codegen from the rows;
  not a field of its own.
- **Table check**: a repeatable report of the above that CI can run.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100 % of tables carry a verification flag; 0 tables have neither verified nor
  unverified. The table check lists every unverified table and exits with success when nothing
  else is wrong.
- **SC-002**: The 4 priority tables (AX-12A, AX-12+, MX-64(2.0), MX-64 Protocol 1) are
  reported verified with an `emanual.robotis.com` URL and a valid date; in an audit of 5 random
  rows from each against its cited page, 100 % match (address, size, name and every recorded
  attribute).
- **SC-003**: 0 table files are unmapped without a recorded reason, 0 tables are unmapped for
  being unverified, and 0 model numbers appear twice in the map.
- **SC-004**: After the per-protocol map format is adopted, codegen output differs from the
  previous output only where a correction changed a register or a new table was added (0
  unexplained diffs).
- **SC-005**: A maintainer can tell, for any table, in under 1 minute, whether it is verified
  (and if so against which page and on which date) and which protocol it belongs to.
- **SC-006**: 100 % of tables have their protocol recorded; the analysis report's protocol
  grouping matches the recorded data for every table.
- **SC-007**: 100 % of dual-protocol models select a table on both a Protocol 1 and a Protocol 2
  bus, and 100 % of them have Protocol 2 marked as the default.
- **SC-008**: 100 % of tables give exactly one mode source in the table check; for the 4
  priority tables the mode row (or its absence) matches the cited page.

## Assumptions

What already exists, so the plan only covers the gaps:

- 72 control tables exist in `control_tables/*.ron`, each with `type_info` (position scaling),
  `unit_info` (scaling for a few registers) and `control_table` rows of `address`, `size`,
  `data_name` only (4,635 register rows in all, plus 269 `unit_info` entries). None has a
  verification record or flag, a recorded protocol, instructions, access, memory area, range or
  default.
- `dynamixel.ron` maps 69 tables under 63 model numbers, each with `family`, `name` and a
  firmware-ordered `tables` list (`from_fw`, `file`); 6 XL330/XC330 models have a
  firmware-53 variant. It has no protocol dimension and no default protocol.
- Mode sources today: 66 tables have an "Operating Mode" row (65 mapped, plus `xw430_t200`), the
  XL-320 table has a "Control Mode" row, and the 5 AX tables have neither, only the CW/CCW Angle
  Limit pair. Each table already gives exactly one source; this feature checks the rows against
  the pages and keeps the rule. Reading the mode at Setup and checking goals against it belong
  to core and the bus; deriving the source from the rows belongs to codegen.
- 3 tables are not mapped: `ax_12_plus`, `ax_18f`, `xw430_t200`; codegen skips them with a
  warning (ARCHITECTURE.md "Not implemented yet").
- MX tables exist only for Protocol 2 firmware (MX-28/64/106 (2.0), model numbers 30, 311,
  321). MX Protocol 1 tables (e.g. MX-64, model 310) are missing (ARCHITECTURE.md).
- `cargo xtask codegen` already exists, reads `dynamixel.ron` and the `control_table` rows, and
  ignores fields it does not know, so adding the flag and capability fields does not break it.
  Generated model types for all 69 mapped tables are committed under `src/models*`.
- Family features `ax`, `mx`, `x-series`, `y-series`, `pro` already exist; this feature does
  not change them.
- `control_table_analysis.md` / `.json` and `analyze_tables.py` exist; the script infers
  protocol from the presence of CW/CCW Angle Limit, which places XL-320 with the Protocol 1
  tables. Whether that is right is for the e-Manual to settle.

Defaults and boundaries:

- SRS FR-7.2, NFR-9, the decision log, delivery step 7 and the roadmap carry the 2026-10-27
  amendment this spec follows: online verification deferred to first use per model, an
  explicit unverified flag in the `.ron` metadata and the generated docs, priority AX-12 and
  MX-64, and tables for both protocols with Protocol 2 as the default on dual-protocol models
  (MX on Protocol 1 left the roadmap). Constitution IV carries the same amendment (v1.3.0). The
  2026-10-28 decision (limits follow the operating mode) is not in the SRS yet; the architect
  amends it.
- Delivery step 3's "one X-series model verified" is not done by this feature unless that
  model is needed; the first verified models are AX-12 and MX-64.
- The verified record follows the SRS §8 "Meanwhile" default (FR-001); the exact shape of the
  unverified value is set in the plan. Renaming fields later is a mechanical change.
- One flag per table, not per row; a verified record covers every value in the file.
- "First needed" means a module, test, example or user first drives that model on hardware, or
  a bug is reported against it.
- The English e-Manual page for the exact model is the source.
- Teaching codegen to read the per-protocol map, the default protocol, the capability data, the
  mode source and the flag (for the generated docs) belongs to the `xtask` module. This feature
  defines the data and keeps it readable by today's codegen (FR-016); the plan orders the PRs so
  the map never gets ahead of codegen and CI's regeneration diff check stays green.
- A correction or a new table regenerates `src/models*`. That output is tool-generated,
  committed in the same PR so CI's diff check passes, and flagged in the PR as the only change
  outside `control_tables/**` (no-go `src/**`; constitution Governance).
- The table check lives with the tables (next to `analyze_tables.py`); wiring it into CI is the
  `ci` module's job.

Effort (verification and new tables are human or agent work with an online lookup per table;
estimates, not measurements):

- Adding the flag (all unverified) and the recorded protocol to 72 tables is mechanical: a few
  hours, protocol looked up per model family.
- Verifying one table (address, size, name): about 20–40 minutes including review. The 4
  priority tables take about 1.5–3 hours.
- Writing the 2 other MX Protocol 1 tables (MX-28, MX-106) from the e-Manual: about 30–60
  minutes each, left unverified.
- Each per-register attribute added (FR-007) adds time per table; option (b) roughly doubles
  it. With deferred verification, that cost is paid per model, not up front.
