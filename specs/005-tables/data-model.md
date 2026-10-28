# Data Model: Control tables

**Feature**: [spec.md](spec.md) | **Research**: [research.md](research.md) | **Format**:
[contracts/ron-schema.md](contracts/ron-schema.md)

All entities are RON data in `control_tables/`. Names in `code` are the RON spellings fixed by
module-interfaces section 3.

## Control table (`control_tables/<stem>.ron`)

One file per model and firmware variant. The file stem names the generated type (`mx_64_p1` →
`Mx64P1`).

| Field | Type | Required | Source | Read by |
|---|---|---|---|---|
| `source` | Verification flag | always | this feature | check, X1 (docs) |
| `protocols` | non-empty set of `P1`, `P2` | always | model page, Protocol Type values (research R-3) | check, X1 (`ModelData::protocols`) |
| `instructions` | list of Instruction entry | verified tables only, one or more per protocol in `protocols` | protocol pages (R-5) | check; not codegen yet |
| `type_info` | Position scale | always (unchanged shape) | model page | X1 (`PositionScale`) |
| `unit_info` | list of `{data_name, value, unit, sign_type}` | optional (unchanged shape) | model page | nobody yet |
| `control_table` | list of Register row, non-empty | always | model page | today's codegen, X1 |

Validation:
- Every field above comes from the e-Manual; a value the page does not give is left out.
- Brace style stays (`{ key: value }`); no real map appears in a table.
- A table mapped under `p1` must list `P1` in `protocols`; under `p2`, `P2`.
- Exactly one mode source, from the rows (see "Mode source" below).

## Verification flag (`source`)

| Variant | Fields | Meaning |
|---|---|---|
| `Unverified` | none | Not yet checked as a whole. Usable. |
| `Verified(url, verified)` | `url`: string, `verified`: string `YYYY-MM-DD` | Every value in the file was checked against `url` on that date. |

Validation: `url` starts with `https://emanual.robotis.com/`. `verified` is a real calendar date
not after the day the check runs.

State transitions:

```
Unverified ──(whole table checked against its page)──► Verified(url, date)
Unverified ──(one row fixed from the page)───────────► Unverified        (PR lists the fix)
Verified(url, d1) ──(table changed and re-checked)───► Verified(url, d2) (d2 = date of the new check)
Verified ──(page moved or rewritten later)───────────► Verified (unchanged; nothing redone)
```

There is no transition back to `Unverified`. A verified table that changes without a re-check is
a review error, not a state.

## Register row (`control_table[i]`)

| Field | Type | Required | Notes |
|---|---|---|---|
| `address` | u16 | yes | unchanged |
| `size` | u8 | yes | unchanged |
| `data_name` | string | yes | the name the model's own page uses |
| `access` | `R` or `RW` | yes (new) | from the page's Access column |
| `area` | `EEPROM` or `RAM` | yes (new) | from the page heading the row sits under |

Validation:
- Any other `access` value (e.g. `W`) stops the work and is raised (contract 3.1).
- Range, default and unit are out of scope (FR-007).
- Role rows, for mapped tables (contract 4.3): `Torque Enable` RW/RAM, `Goal Position` RW/RAM,
  `Present Position` R/RAM, and exactly one complete limit pair, `Min/Max Position Limit` or
  `CW/CCW Angle Limit`, RW/EEPROM. A `Firmware Version` row. `Model Number` at address 0, size 2,
  R/EEPROM.
- In a mapped table, no two rows give the same register constant (ron-schema.md section 1).
- In a table mapped under `p1`, no role row and no mode row above address 255.

## Mode source (derived from the rows, not a field)

How the bus learns an actuator's operating mode at Setup, so a goal is checked only where the
firmware applies the limits (module-interfaces §2.4b, architect 2026-10-28). X1 emits it as
`ModelData::mode: ModeSource`. The tables give the rows; the values (which mode means what) are
core's.

| `ModeSource` | Rows in the table | Models |
|---|---|---|
| `OperatingMode { register, values }` | one `Operating Mode` row, no CW/CCW Angle Limit row | X, MX(2.0), PRO, PRO+, RH-P12-RN, Y |
| `ControlMode(Register)` | one `Control Mode` row (the CW/CCW pair is also there, as the limits) | XL-320 only |
| `AngleLimits` | neither mode row; `CW Angle Limit` + `CCW Angle Limit` | AX, MX on Protocol 1 |

`register` comes from the row. `values` (which Operating Mode value list applies) is open in
module-interfaces §2.4b; the tables carry nothing for it until core, xtask and tables agree
(research R-15).

Validation (every table, mapped or not; codegen checks 11 and 17, plus the `xl320` rule;
research R-15):
- Exactly one source: at most one row of each mode name, never both names in one table, no
  `Operating Mode` row beside CW/CCW Angle Limit rows; with neither mode row, the CW/CCW pair
  must be present.
- `Control Mode` only in `xl320`.
- The mode row is 1 byte, `RW`, `EEPROM`. Its address, size, access and area are the page's; a
  page that lists a mode register means the table carries that row, and a page that lists none
  means the table carries none (FR-020).

## Instruction entry (`instructions[i]`)

| Field | Type | Required | Notes |
|---|---|---|---|
| `protocol` | `P1` or `P2` | yes | must be in the table's `protocols` |
| `source` | string URL on `emanual.robotis.com` | yes | the protocol page |
| `list` | non-empty list of instruction names | yes | no duplicates within one entry |
| `from_fw` | u8 | no | the entry applies from this firmware version |

Instruction names: `Ping, Read, Write, RegWrite, Action, FactoryReset, Reboot, Clear,
ControlTableBackup, SyncRead, SyncWrite, FastSyncRead, BulkRead, BulkWrite, FastBulkRead`.
`P1` entries may use only `Ping, Read, Write, RegWrite, Action, FactoryReset, Reboot, SyncWrite,
BulkRead` (Protocol 1.0 page).

Entries of one protocol add up. Two entries of one protocol must differ in `from_fw`.

## Position scale (`type_info`), unchanged shape

| Field | Type |
|---|---|
| `value_of_zero_radian_position` | i32 |
| `value_of_min_radian_position` | i32 |
| `value_of_max_radian_position` | i32 |
| `min_radian` | f32 |
| `max_radian` | f32 |

Validation (same as contract 4.3): `max_raw > min_raw`, `max_radian > min_radian`, zero within
`min_raw..=max_raw`.

Meaning: `angle = (raw − zero) × rad_per_tick`. `min_radian` and `max_radian` are that angle at
the min and max values, so `max_radian − min_radian = (max − min) × rad_per_tick`. `rad_per_tick`
comes from the page (one turn over the pulse count, or the stated span over the stated range).
The check cannot test this against the page; T1 sets the values from it (research R-8):

| Tables | zero / min / max | `min_radian` / `max_radian` | `rad_per_tick` |
|---|---|---|---|
| X-series, MX(2.0), 2XL/2XC (4096 pulses per turn) | 2048 / 0 / 4095 | `-3.1415927` / `3.1400587` (−π, π·2047/2048) | 2π/4096 |
| AX, XL-320 (0 ~ 1023 over 0 ~ 300°) | 512 / 0 / 1023 | `-2.6205530` / `2.6154347` (−512 and +511 ticks) | 300°/1023 |
| PRO, Y, RH-P12-RN (zero 0, symmetric) | unchanged | unchanged | already consistent |

## Model map (`control_tables/dynamixel.ron`)

```
(models: { <u16>: Model entry, … }, not_mapped: [ Not-mapped record, … ])
```

### Model entry

| Field | Type | Required | Notes |
|---|---|---|---|
| `family` | `"ax"`, `"mx"`, `"x-series"`, `"y-series"`, `"pro"` | yes | Cargo feature (FR-7.5); any other value is an error |
| `name` | string | yes | display name from the page |
| `also` | list of string | no | other products sold under this number (docs only) |
| `p1` | list of Table ref | at least one of `p1`, `p2` | tables on a Protocol 1 bus |
| `p2` | list of Table ref | at least one of `p1`, `p2` | tables on a Protocol 2 bus |
| `default` | `P2` | exactly when both lists exist | must be `P2` |
| `tables` | list of Table ref | T1 only | legacy, equals the default (or only) list; removed in T2 |

### Table ref

| Field | Type | Notes |
|---|---|---|
| `from_fw` | u8 | first in a list is 0; strictly increasing |
| `file` | string | stem of an existing `.ron` in `control_tables/` |

### Not-mapped record

| Field | Type | Notes |
|---|---|---|
| `file` | string | stem of an existing table |
| `reason` | `NoModelNumber` or `SameNumberAs(u16)` | `SameNumberAs` names a number in `models` |

Map validation (FR-008 to FR-012, FR-019, contract 3.2):
- A model number appears once (the parser rejects a duplicate key).
- A file appears under at most one model number. It may appear under both `p1` and `p2` of that
  number.
- A file is mapped or in `not_mapped`, never both, never neither.
- Dual-protocol rule: if any table of a model lists both protocols in `protocols`, the model has
  both lists (and `default: P2`).
- "Unverified" is never a not-mapped reason.
- `Firmware Version` sits at one (address, size) across the files of one number's `p1` list, and
  across its `p2` list.
- No two mapped files give the same type identifier; no mapped file gives the module identifier
  `tests` or a Rust keyword (ron-schema.md section 2).
- T1: legacy `tables` equals the default (or only) list. From T2 on, any `tables` key in a model
  entry is an error.

Relationships:

```
Model entry 1 ──p1/p2── * Table ref ──file── 1 Control table
Not-mapped record ──file── 1 Control table ;  SameNumberAs(n) ── 1 Model entry n
Control table 1 ── * Register row ; 0..* Instruction entry ; 1 Verification flag
Control table 1 ── 1 Mode source (derived from 1 mode row, or from the CW/CCW pair)
```

Data after each PR:

| | Today | After T1 | After T2 |
|---|---|---|---|
| Table files | 72 | 72 | 75 |
| Model numbers | 63 | 63 | 66 |
| Mapped files | 69 | 69 | 72 |
| `not_mapped` | none (3 warnings) | 3 records | 3 records |
| `Verified` tables | 0 | 1 (AX-12A) | 4 (+ MX-64(2.0), AX-12+, MX-64 P1) |
| Mode source `OperatingMode` / `ControlMode` / `AngleLimits` (all tables) | 66 / 1 / 5 | 66 / 1 / 5 | 66 / 1 / 8 |
| Model entry key `tables` | every entry | every entry (legacy) | none (an error) |

## Table check report

Produced by `check_tables.py` (see [contracts/table-tools.md](contracts/table-tools.md)).

| Part | Content |
|---|---|
| Per table | stem, flag (`verified <url> <date>` or `unverified`), protocols, mapping (`<number> p1/p2` or `not mapped: <reason>`) |
| Per dual-protocol model | number, name, P1 table(s), P2 table(s), default |
| Errors | one line each, naming the file (and row or key) |
| Summary | counts: tables, verified, unverified, mapped, not mapped, dual-protocol models, errors |

Exit status: 0 when no error (unverified tables are listed, not errors), 1 on any error.
