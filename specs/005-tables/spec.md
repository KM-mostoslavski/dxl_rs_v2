# Feature Specification: Control tables (e-Manual verification, capabilities, model map)

**Feature Branch**: `005-tables` (spec written on `dev`; no branch created yet)

**Created**: 2026-10-27

**Status**: Draft

**Input**: User description: "tables module of docs/srs/SRS.md"
→ SRS module `tables` (SRS §9): paths `control_tables/**`; covers FR-7.1, FR-7.2, FR-7.4,
NFR-9; delivery steps 3 and 7. Rule: no row without an online e-Manual lookup (FR-7.2).
No-go: `src/**`.

## User Scenarios & Testing *(mandatory)*

The "users" of this module are the crate maintainer who adds or fixes actuator models, the
reviewer who must trust a table change, and, through codegen, every application developer whose
actuator is driven by the addresses and sizes in these files.

### User Story 1 - Trust one table because it says where it was checked (Priority: P1)

A maintainer opens the XM430-W350 table and sees which official e-Manual page it was checked
against and on which date. Every register in it (name, address, size, and any other attribute
the table carries) matches that page. A reviewer can open the same page and repeat the check.

**Why this priority**: FR-7.2 and constitution IV are non-negotiable, and no table carries a
verification record today. Delivery step 3 asks for one X-series model verified end to end
first; it fixes the record format every other table then follows.

**Independent Test**: Open the recorded e-Manual URL and compare every row of the XM430-W350
table against it; run the table check and see the table reported as verified with its URL and
date; run codegen and see it still produces the same model type.

**Acceptance Scenarios**:

1. **Given** the XM430-W350 table, **When** the maintainer reads it, **Then** it names the
   official e-Manual page (on `emanual.robotis.com`) and the date it was checked.
2. **Given** that page, **When** a reviewer compares it row by row, **Then** every address,
   size and name in the table matches it, and no row exists that the page does not list.
3. **Given** a table with no verification record, **When** the table check runs, **Then** it
   names that table as unverified.
4. **Given** a table whose record cites a page outside `emanual.robotis.com`, **When** the
   table check runs, **Then** it rejects the record.
5. **Given** the verification record is added, **When** codegen runs, **Then** the generated
   model type is unchanged (the record is metadata, not a register).

---

### User Story 2 - Every shipped table is verified, family by family (Priority: P1)

The maintainer verifies the remaining tables against their own e-Manual pages, one model
family at a time (X-series, then AX, then MX on Protocol 2, then the rest in scope). A table
the e-Manual contradicts is corrected; a table that cannot be verified is not shipped.

**Why this priority**: Delivery step 7. Until a table is verified, the crate drives real
motors with addresses nobody checked; a wrong address or size can corrupt EEPROM (constitution
IV rationale).

**Independent Test**: Run the table check; it lists every table as verified, or as unmapped
with a stated reason. Pick any table and any row at random and confirm it against the cited
page.

**Acceptance Scenarios**:

1. **Given** a family's tables, **When** they are verified, **Then** each one carries its own
   record (its own model's page, not a sibling's), even where layouts are identical.
2. **Given** a row the e-Manual lists at a different address or size, **When** the table is
   verified, **Then** the row is corrected and the correction is listed in the PR.
3. **Given** a row the e-Manual does not list, **When** the table is verified, **Then** the row
   is removed, not kept "just in case".
4. **Given** a table with firmware variants (e.g. XL330 before and after firmware 53),
   **When** it is verified, **Then** each variant table is checked against the firmware
   history the e-Manual gives and carries its own record.
5. **Given** a table that still has no record when the family is done, **When** codegen runs,
   **Then** that table is not compiled in, and the map says why.

---

### User Story 3 - Know what a model can do, not only where its registers are (Priority: P2)

A maintainer (and, later, the bus and protocol modules through codegen) can read, for each
model and table, which protocols it speaks, which instructions it supports, and the per
register attributes the e-Manual gives. The data is recorded, not guessed from register names.

**Why this priority**: FR-7.2 lists capability claims (instructions, protocol,
firmware-dependent layout, model number) as verified data. Today the tables hold only
address, size and name per row, plus scaling for a few registers; the protocol of a table is
inferred by the analysis script from register names, which groups XL-320 with the Protocol 1
tables.

**Independent Test**: For a verified table, read its capability data and compare each claim
with the e-Manual page; regenerate the analysis and see the protocol grouping come from the
recorded data, not from a heuristic.

**Acceptance Scenarios**:

1. **Given** a verified table, **When** a maintainer reads it, **Then** it states the protocol
   its layout belongs to and the instructions the e-Manual lists for that model.
2. **Given** a verified table, **When** a maintainer reads a register, **Then** it shows the
   attributes in scope for this feature (see FR-007), each one taken from the e-Manual.
3. **Given** the e-Manual gives no value for an attribute, **When** the table is written,
   **Then** the attribute is left absent, never filled from another model or a default.
4. **Given** the analysis is regenerated, **When** a maintainer reads it, **Then** each table's
   protocol comes from its recorded capability data.

---

### User Story 4 - One model number, a table per protocol (Priority: P2)

Some actuators can be switched between Protocol 1 and Protocol 2 (62 of the 72 tables have a
`Protocol Type` register), and MX actuators exist with Protocol 1 firmware. The model map can
say, for one model number, which table applies on a Protocol 1 bus and which on a Protocol 2
bus, with firmware variants under each.

**Why this priority**: The SRS roadmap requires the map not to block MX on Protocol 1. Today
the map holds one list of tables per model number, ordered by firmware only, so it cannot
express a second protocol.

**Independent Test**: Write a map entry for one model number with a table for each protocol;
the table check accepts it and reports, for each protocol, which table a given firmware
version selects.

**Acceptance Scenarios**:

1. **Given** a model whose e-Manual lists one control table for both protocols, **When** the
   map is read, **Then** the same table is selected on either bus.
2. **Given** a model whose e-Manual lists a different table per protocol, **When** the map is
   read for one protocol, **Then** only that protocol's table is selected.
3. **Given** a model with no table for a protocol, **When** the map is read for that protocol,
   **Then** no table is selected (the bus reports `UnsupportedModel`).
4. **Given** the existing map entries, **When** the new format is adopted, **Then** each one
   keeps selecting the same table under the protocol it already serves.

---

### User Story 5 - Ship the models that are still missing (Priority: P3)

The maintainer adds the models the architecture lists as missing: AX-12+, AX-18F and XW430-T200
get their model numbers (or are folded into an existing entry), and MX on Protocol 1 firmware
(e.g. MX-64, model 310) gets verified tables. [NEEDS CLARIFICATION: Are the MX Protocol 1
tables (MX-28, MX-64, MX-106 on Protocol 1 firmware) written in this feature, or does this
feature only make the map able to hold them (User Story 4), leaving the tables on the roadmap
as SRS §8 says?]

**Why this priority**: ARCHITECTURE.md calls MX on Protocol 1 "required" but the SRS puts it on
the roadmap; the three unmapped tables only need a model number. Both are additions on top of
verified, protocol-aware tables.

**Independent Test**: The table check reports zero unmapped tables without a reason; codegen
no longer warns about the three skipped files.

**Acceptance Scenarios**:

1. **Given** AX-12+, AX-18F or XW430-T200, **When** the e-Manual gives its model number,
   **Then** the map lists it with that number and codegen no longer skips it.
2. **Given** an unmapped table whose e-Manual model number is already used by a mapped model,
   **When** it is mapped, **Then** it does not create a second entry for that number; it is
   recorded against the existing entry, or removed if the layouts are identical, and the
   choice is stated in the PR.
3. **Given** the e-Manual gives no model number for one of them, **When** the work ends,
   **Then** the table stays unmapped and the map records why.

---

### Edge Cases

- The e-Manual page moves or is rewritten after a table was verified: the record keeps the URL
  and date it was checked on; the table is re-verified only when it is changed, and the new
  record replaces the old.
- The e-Manual is unreachable during the work: the table is not written or changed. A row is
  never written from memory, a third-party SDK or a sibling table (constitution IV).
- Two models share one byte-identical layout (14 distinct layouts cover all 72 tables): each
  table is still checked against its own model's page, since ranges, defaults and units can
  differ even when addresses do not.
- A register is renamed between pages (e.g. "Bus Watchdog" vs "BUS Watchdog" both appear
  today): the table uses the name the model's own page gives; any cross-model name mismatch is
  reported by the analysis, not silently unified.
- A correction changes an address or size that generated code already uses: the regenerated
  model files change with it in the same PR, and the PR says so (CI fails on a stale diff).
- A firmware variant's boundary (`from_fw`) is not stated on the e-Manual page: the variant is
  not mapped until the boundary is found there.
- The verification record names a page for a different model than the table: the check
  cannot catch this alone; the reviewer confirms the model name on the cited page.
- A date in the record is in the future or not a real calendar date: the check rejects it.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Every control table MUST carry one verification record at its top level: the
  e-Manual URL it was checked against and the date of the check, as
  `source: (url: "...", verified: "YYYY-MM-DD")` (SRS §8 open question, "Meanwhile" default).
- **FR-002**: The URL in a verification record MUST be on `emanual.robotis.com`, and the date
  MUST be a real calendar date not later than the day it was written.
- **FR-003**: Every value in a table (register name, address, size, any register attribute,
  every capability claim) and every model number and firmware boundary in the map MUST match
  the official e-Manual, looked up online when it is written or changed. A value the e-Manual
  does not give MUST be left out (constitution IV, FR-7.2).
- **FR-004**: A table that changes MUST get a new verification record with the date of the new
  lookup; a record MUST NOT be updated without the lookup.
- **FR-005**: Each table MUST record the protocol its layout belongs to, taken from the
  e-Manual, so no tool has to infer it from register names.
- **FR-006**: Each model MUST record the instructions the e-Manual lists as supported (ping,
  read, write, sync and bulk instructions and the others the page lists), per protocol.
- **FR-007**: Each register row MUST carry these attributes from the e-Manual, in addition to
  name, address and size: [NEEDS CLARIFICATION: Which per-register attributes does this feature
  add to all ~4,400 rows: (a) access and memory area (EEPROM/RAM) only, (b) access, area,
  range, default and unit (the full FR-7.2 list), or (c) none, verifying only what is there
  now?]
- **FR-008**: The model map MUST be able to give, for one model number, a separate list of
  firmware-ordered tables per protocol, and MUST allow the same table to serve both protocols.
- **FR-009**: Adopting the per-protocol map format MUST leave every existing model number
  selecting the same table under the protocol it serves today, so generated model types do not
  change because of the format alone.
- **FR-010**: Within one model number and one protocol, firmware boundaries MUST be strictly
  increasing, start at 0, and each MUST name a table file that exists.
- **FR-011**: A table not referenced by the map MUST be listed in the map's "not mapped"
  record with its reason (no model number found, duplicate of an entry, not yet verified), so
  the codegen warning (FR-7.4) is never the only trace.
- **FR-012**: A model number MUST appear at most once in the map; a second product with the
  same number (e.g. a predecessor with an identical layout) MUST be recorded under the
  existing entry, never as a second entry.
- **FR-013**: Tables of the Y-series and PRO families MUST be either verified in this feature
  or taken out of the map (and listed as not mapped, FR-011) until they are; the family feature
  names stay reserved either way (FR-7.5). [NEEDS CLARIFICATION: Y-series and PRO (25 tables,
  generated today) are on the SRS roadmap, yet FR-7.2 forbids unverified tables. Verify them in
  this feature, or unmap them (removing their generated model types) until a later feature
  verifies them?]
- **FR-014**: A repeatable table check MUST report, for every table: verified (URL and date) or
  not, mapped or not mapped with reason, protocol recorded or not; and for the map: duplicate
  numbers, missing files, bad firmware ordering. It MUST exit with a failure when anything
  required here is missing, so CI can run it.
- **FR-015**: The analysis report MUST be regenerated from the recorded data after this
  feature and MUST take each table's protocol from FR-005.
- **FR-016**: Each table MUST stay readable by the existing codegen: new metadata MUST NOT
  change any generated model type except through a verified correction of a register.
- **FR-017**: Every PR that touches `control_tables/` MUST list, per changed table, the
  e-Manual URL and verification date, and every corrected or removed row (NFR-9).

### Key Entities

- **Control table**: one file per model (and per firmware variant): register rows plus model
  data (scaling, verification record, protocol, capabilities).
- **Register row**: one register of a table: name, address, size, and the attributes FR-007
  settles.
- **Verification record**: the e-Manual URL and the date a table was last checked against it.
- **Capability data**: per model or table: protocol of the layout, supported instructions per
  protocol, firmware-dependent layout (variants).
- **Model map entry**: one model number → family, display name, and per protocol a
  firmware-ordered list of tables.
- **Not-mapped record**: a table file the map leaves out, with the reason.
- **Table check**: a repeatable report of the above that CI can run.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100 % of the tables the map references carry a verification record with an
  `emanual.robotis.com` URL and a valid date; the table check reports zero unverified mapped
  tables.
- **SC-002**: In an audit of 5 random rows from each verified table against its cited page,
  100 % match (address, size, name and every recorded attribute).
- **SC-003**: 0 table files are unmapped without a recorded reason, and 0 model numbers appear
  twice in the map.
- **SC-004**: After the per-protocol map format is adopted, codegen output differs from the
  previous output only where a verified correction changed a register (0 unexplained diffs).
- **SC-005**: A maintainer can tell, for any table, in under 1 minute, which page it was checked
  against, on which date, and which protocol it belongs to.
- **SC-006**: 100 % of tables have their protocol recorded; the analysis report's protocol
  grouping matches the recorded data for every table.

## Assumptions

What already exists, so the plan only covers the gaps:

- 72 control tables exist in `control_tables/*.ron`, each with `type_info` (position scaling),
  `unit_info` (scaling for a few registers) and `control_table` rows of `address`, `size`,
  `data_name` only. None has a verification record, a recorded protocol, instructions, access,
  memory area, range or default.
- `dynamixel.ron` maps 69 tables under 64 model numbers, each with `family`, `name` and a
  firmware-ordered `tables` list (`from_fw`, `file`); 6 XL330/XC330 models have a
  firmware-53 variant. It has no protocol dimension.
- 3 tables are not mapped: `ax_12_plus`, `ax_18f`, `xw430_t200`; codegen skips them with a
  warning (ARCHITECTURE.md "Not implemented yet").
- MX tables exist only for Protocol 2 firmware (MX-28/64/106 (2.0), model numbers 30, 311,
  321). MX Protocol 1 tables (e.g. MX-64, model 310) are missing (ARCHITECTURE.md).
- `cargo xtask codegen` already exists, reads `dynamixel.ron` and the `control_table` rows, and
  ignores fields it does not know, so adding `source` and capability fields does not break it.
  Generated model types for all 69 mapped tables are committed under `src/models*`.
- Family features `ax`, `mx`, `x-series`, `y-series`, `pro` already exist; this feature does
  not change them.
- `control_table_analysis.md` / `.json` and `analyze_tables.py` exist; the script infers
  protocol from the presence of CW/CCW Angle Limit, which places XL-320 with the Protocol 1
  tables. Whether that is right is for the e-Manual lookup to settle.

Defaults and boundaries:

- The verification record format follows the SRS "Meanwhile" default (FR-001) until the
  architect answers the SRS open question; renaming the fields later is a mechanical change.
- One record per table, not per row; the record covers every value in the file.
- The English e-Manual page for the exact model is the cited source.
- Teaching codegen to read the per-protocol map and the capability data belongs to the
  `xtask` module; this feature defines the data and keeps it readable by today's codegen
  (FR-016). Until `xtask` reads the per-protocol map, no table is added under a second
  protocol in the map.
- A verified correction regenerates `src/models*`. That output is tool-generated, committed in
  the same PR so CI's diff check passes, and flagged in the PR as the only change outside
  `control_tables/**` (no-go `src/**`; constitution Governance).
- Y-series and PRO scope is open (FR-013). The effort figures below assume the 44 X-series, AX
  and MX tables, with the all-72 figure for comparison.
- The table check lives with the tables (next to `analyze_tables.py`); wiring it into CI is the
  `ci` module's job.

Effort (verification is human or agent work with an online lookup per table; estimates, not
measurements):

- Address, size and name only: about 20–40 minutes per table including review, so about
  15–30 hours for the 44 X-series, AX and MX tables, and about 25–50 hours
  if all 72 are verified.
- Adding the full per-register attribute list (FR-007 option b) roughly doubles that: about
  40–90 minutes per table.
- Identical layouts (14 groups) speed up address checks but do not remove the per-model
  lookup, since each table needs its own page and record.
