# Data Model: xtask codegen

**Feature**: [spec.md](spec.md) | **Research**: [research.md](research.md)

There are three groups:

- **Input**: what xtask parses. The schema is owned by the tables module (contract §3).
- **Resolved**: what xtask checks and renders, in memory only.
- **Generated**: what lands in `src/models*`. The Rust types it uses (`Register`, `ModelData`,
  `ModeSource`, `Cache`, …) are owned by core (contract §2). Generated code uses them; it does not
  define them.

## 1. Input (xtask `codegen/schema.rs`)

### ModelMap (`control_tables/dynamixel.ron`)

| Field | Type | Rule |
|---|---|---|
| `models` | map `u16 → ModelEntry` | keys are model numbers |
| `not_mapped` | list of `NotMapped`, default empty | tables on disk that no model number uses, on purpose |
| *(legacy `tables`)* | ignored | read by today's xtask until T2 removes it |

### ModelEntry

| Field | Type | Rule |
|---|---|---|
| `family` | `String` | one of `ax`, `mx`, `x-series`, `y-series`, `pro` |
| `name` | `String` | display name, e.g. `XM430-W350` |
| `also` | list of `String`, default empty | other product names sold under the number; docs only |
| `p1`, `p2` | `Option<Vec<TableRef>>` | at least one present; a present list is non-empty |
| `default` | `Option<ProtocolTag>` | required and `P2` when both lists exist; absent otherwise |

### TableRef

| Field | Type | Rule |
|---|---|---|
| `from_fw` | `u8` | the first is 0, then strictly increasing within one list |
| `file` | `String` | the stem of an existing `control_tables/<file>.ron` |

### NotMapped

| Field | Type | Rule |
|---|---|---|
| `file` | `String` | the file exists and is not in any `p1`/`p2` list |
| `reason` | `NoModelNumber` or `SameNumberAs(u16)` | docs; codegen only checks the shape |

### ControlTable (`control_tables/<file>.ron`, braces rewritten to parentheses)

| Field | Type | Rule |
|---|---|---|
| `source` | `Unverified` or `Verified { url, verified }` | required on a mapped table |
| `protocols` | list of `ProtocolTag` | required on a mapped table, non-empty |
| `type_info` | `TypeInfo` | required |
| `control_table` | list of `Row` | required |
| *(`unit_info`, `instructions`, other fields)* | ignored | |

### TypeInfo (position scale)

| Field | Type | Rule |
|---|---|---|
| `value_of_zero_radian_position` | `i32` | in `min..=max` |
| `value_of_min_radian_position` | `i32` | `< max` |
| `value_of_max_radian_position` | `i32` | |
| `min_radian` | `f32` | `< max_radian` |
| `max_radian` | `f32` | |

### Row

| Field | Type | Rule |
|---|---|---|
| `address` | `u16` | |
| `size` | `u8` | |
| `data_name` | `String` | its constant name is unique within the table |
| `access` | `R` or `RW` | required |
| `area` | `EEPROM` or `RAM` | required |

`ProtocolTag` = `P1 | P2`.

## 2. Resolved (xtask `codegen/resolve.rs`)

### Family

Five values. Each maps to its feature name string, used both in `cfg(feature = "...")` and in
`UnsupportedModel.feature`.

### GenModel: one per mapped table file

| Field | From |
|---|---|
| `file`, `type_ident`, `module_ident` | file stem; a leading digit is spelled out (`2xl430_w250` → `TwoXl430W250` / `two_xl430_w250`) |
| `number`, `name`, `also`, `family` | the one model entry that names the file |
| `appearances` | list of `(ProtocolTag, from_fw, until_fw: Option<u8>)`, one per list naming the file |
| `protocols` | the table's `protocols` field, which must include every protocol in `appearances` |
| `source` | the table's `source` |
| `roles` | `Roles`, below |
| `scale` | `TypeInfo`, checked |
| `registers` | list of `(const_ident, Row)` in table order |

### Roles

| Field | Type | Row name(s) |
|---|---|---|
| `model_number` | row | `Model Number` |
| `firmware_version` | row | `Firmware Version` |
| `torque_enable` | row | `Torque Enable` |
| `goal_position` | row | `Goal Position` |
| `present_position` | row | `Present Position` |
| `limits` | `MinMax { min, max }` or `CwCcw { cw, ccw }` | `Min/Max Position Limit` or `CW/CCW Angle Limit` |
| `hardware_error` | `Option<row>` | `Hardware Error Status` |
| `mode` | `OperatingMode(row)`, `ControlMode(row)` or `AngleLimits` | `Operating Mode`; else `Control Mode`; else none, with the CW/CCW form (research R-17) |

### LookupList: one per (model number, protocol) with tables

| Field | Type |
|---|---|
| `protocol` | `ProtocolTag` |
| `number`, `family` | from the entry |
| `variants` | ordered `(from_fw, type_ident)` |

`firmware_dependent` = `variants.len() > 1`.

### Report and Diagnostic

| Type | Fields |
|---|---|
| `Diagnostic` | `file: String` (the table or `dynamixel`), `message: String` |
| `Report` | `written: Vec<path>`, `removed: Vec<path>`, `unchanged: usize`, `warnings: Vec<Diagnostic>` |

## 3. Checks (all before any write)

Each failing check adds one `Diagnostic`. The run fails if there is at least one.

| # | Check | Names | Source |
|---|---|---|---|
| 1 | map or table missing or unparsable | the file | FR-003 |
| 2 | `from_fw` not starting at 0, or not strictly increasing, per list | map, model number, protocol | FR-003 |
| 3 | no `p1` and no `p2`, or an empty list | map, model number | contract §3.2 |
| 4 | `default` missing or not `P2` with both lists; present with one list | map, model number | contract §3.2, constitution IV |
| 5 | unknown family | map, model number, family | FR-005 |
| 6 | a file under two model numbers | map, file, both numbers | contract §3.2 |
| 7 | a file both mapped and in `not_mapped`; a `not_mapped` file that does not exist | map, file | contract §4.3 |
| 8 | a mapped table without `source` or `protocols` (parse error), or with empty `protocols` | table | FR-012 |
| 9 | a list `pX` names a table whose `protocols` lacks `PX` | table, protocol | contract §4.3 |
| 10 | a missing role, half a limit form, or both limit forms | table, role | FR-011 |
| 11 | wrong role attributes (Torque Enable and Goal Position not `RW`/`RAM`; Present Position not `R`/`RAM`; a limit not `RW`/`EEPROM`; a mode register not 1 byte or not `EEPROM`) | table, role, found | contract §4.3, §2.4b, plan C5 |
| 12 | Model Number not at (0, 2), or its access/area differ between tables | table | FR-008 |
| 13 | Firmware Version location differs between variants of one (number, protocol) | map, model number, both tables | FR-008 |
| 14 | a role above address 255 in a table listed under `p1` | table, role, address | FR-003 |
| 15 | bad scale: `max_raw ≤ min_raw`, `max_rad ≤ min_rad`, or zero outside `min..=max` | table | contract §4.3 |
| 16 | duplicate type identifier or register constant; module identifier `tests` or a keyword | table(s), identifier | FR-003, R-16 |
| 17 | no mode source (neither an `Operating Mode` nor a `Control Mode` row, and no CW/CCW Angle Limit rows); an `Operating Mode` row beside CW/CCW Angle Limits; both an `Operating Mode` and a `Control Mode` row | table, the rows found | FR-019, contract §2.4b, plan C5 |

Warnings, which do not fail the run: a `.ron` file other than `dynamixel.ron` that is neither
mapped nor in `not_mapped`. One warning per file (FR-004, FR-7.4).

## 4. Pipeline states

```text
Parse ──► Resolve + check ──► Render ──► Format (rustfmt stdin) ──► Sync to disk
  │             │                │              │
  └─────────────┴──── any error ─┴──────────────┴──► exit 1, no file touched
```

Sync writes only changed files and deletes stale `src/models/*.rs` files. It never touches
anything outside `src/models.rs` and `src/models/` (FR-018).

## 5. Generated items (shape; signatures in [contracts/generated-api.md](contracts/generated-api.md))

| Item | Where | Visibility | Content |
|---|---|---|---|
| model type `<Type>` | `src/models/<module>.rs` | `pub`, documented, `cfg(family)` | one field `cache: Cache` |
| `<Type>::new(id)` | same | crate | `Cache::new(id)` |
| `<Type>::DATA` | same | crate | `ModelData`: name, number, protocols, roles by constant name, mode source, scale |
| `<Type>::<REGISTER>` | same | crate | one `Register` per row, via `reg(...)` |
| `impl Describe`, `impl Actuator` | same | (trait) | `&Self::DATA`, `&self.cache`, `&mut self.cache`; `id`/`model_number`/`name` until core-2 |
| `reg` | `src/models.rs` | private to `models` | `const fn` building `Register` |
| `MODEL_NUMBER` | `src/models.rs` | crate | `reg(0, 2, R, EEPROM)` |
| `ActuatorKind` | `src/models.rs` | `pub`, `#[non_exhaustive]` | one variant per model type, `cfg(family)` |
| `ActuatorKind::new`, `::firmware_register` | `src/models.rs` | crate | R-8, R-9 |
| `impl Describe`, `impl Actuator` for `ActuatorKind` | `src/models.rs` | (trait) | `match *self` |
| tests | `src/models/tests.rs` | `cfg(test)` | R-11 layer 3 |

Per-actuator runtime state (generated side): only `Cache` (an `Id`, the limit mode and the limits,
set at Setup). Torque and fault state are not in model types; the bus keeps them in
`[TorqueState; N]` (contract §2.5, §6). The limit mode is decided by core from the mode source
and the value the bus reads (contract §2.4b); codegen emits only the source.
