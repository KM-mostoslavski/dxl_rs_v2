# Contract: Control-table and model-map RON format

**Feature**: [../spec.md](../spec.md) | **Model**: [../data-model.md](../data-model.md) |
**Shared contract**: `specs/ideas/module-interfaces.md` section 3 (and 2.4b for the mode source)

This is the format `xtask` (X1) reads. It makes module-interfaces section 3 exact. Where that
section leaves a choice to this module (`instructions`, the place of `operating_modes`), the
choice is stated here. The table check enforces every rule below.

## 1. Control table: `control_tables/<stem>.ron`

```ron
(
    source: Verified(url: "https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/", verified: "2026-10-28"),
    // or: source: Unverified,
    protocols: [P1],
    instructions: [
        (protocol: P1, source: "https://emanual.robotis.com/docs/en/dxl/protocol1/",
         list: [Ping, Read, Write, RegWrite, Action, FactoryReset, SyncWrite]),
    ],
    type_info: {
        value_of_zero_radian_position: 512,
        value_of_max_radian_position: 1023,
        value_of_min_radian_position: 0,
        min_radian: -2.6205530,
        max_radian: 2.6154347,
    },
    unit_info: [
        { data_name: "Position", value: 0.29, unit: "deg", sign_type: "unsigned" },
    ],
    control_table: [
        { address: 0, size: 2, data_name: "Model Number", access: R, area: EEPROM },
        { address: 24, size: 1, data_name: "Torque Enable", access: RW, area: RAM },
    ],
)
```

The values are an illustration of the shape, not the AX-12A data. A multi-line row keeps its
fields in the order `address, size, data_name, access, area`, one per line.

A table with an `Operating Mode` or `Control Mode` row (here an X-series table, unverified) also
carries `operating_modes`, on one line after `protocols` (after `instructions` when present) and
before `type_info`:

```ron
(
  source: Unverified,
  protocols: [P1, P2],
  operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),
  // type_info, unit_info and control_table as above; control_table holds the mode row
  // { address: 11, size: 1, data_name: "Operating Mode", access: RW, area: EEPROM }
)
```

Key order: `source`, `protocols`, `instructions`, `operating_modes`, `type_info`, `unit_info`,
`control_table`; an absent key is skipped.

| Key | Grammar | Rule |
|---|---|---|
| `source` | `Unverified` \| `Verified(url: STRING, verified: STRING)` | required; URL prefix `https://emanual.robotis.com/`; date ISO `YYYY-MM-DD`, real, not in the future |
| `protocols` | `[PROTO (, PROTO)?]`, `PROTO = P1 \| P2` | required, non-empty, no duplicates; written `[P1]`, `[P2]` or `[P1, P2]` |
| `instructions` | `[ENTRY, …]`, `ENTRY = (protocol: PROTO, source: STRING, list: [INSTR, …] (, from_fw: INT)?)` | only when `source` is `Verified`, then at least one entry per protocol in `protocols`; `protocol` in `protocols`; `source` URL as above; `list` non-empty, no duplicates; two entries of one protocol differ in `from_fw` |
| `INSTR` | `Ping \| Read \| Write \| RegWrite \| Action \| FactoryReset \| Reboot \| Clear \| ControlTableBackup \| SyncRead \| SyncWrite \| FastSyncRead \| BulkRead \| BulkWrite \| FastBulkRead` | P1 entries: only `Ping, Read, Write, RegWrite, Action, FactoryReset, Reboot, SyncWrite, BulkRead` |
| `operating_modes` | `(bounded: [U8, …], unbounded: [U8, …], no_position: [U8, …])`, `U8` an integer 0–255 | present exactly when the table has an `Operating Mode` or `Control Mode` row; all three keys; a list may be empty (`[]`); no value in two lists; values from the model's page (research R-15) [17] |
| `type_info` | unchanged | `max > min` for values and radians; zero value within `min..=max`. Meaning: `min_radian` and `max_radian` are `(raw − zero) × rad_per_tick` at the min and max values (research R-8) |
| `unit_info` | unchanged | optional; not read by codegen |
| `control_table` | `[ROW, …]`, `ROW = { address: INT, size: INT, data_name: STRING, access: ACCESS, area: AREA }` | required, non-empty |
| `ACCESS` | `R \| RW` | required on every row; anything else is a contract change |
| `AREA` | `EEPROM \| RAM` | required on every row |

Extra rules for a table that the map lists (contract 4.3, data part; xtask checks in brackets):
- Roles present with these attributes: `Torque Enable` RW RAM; `Goal Position` RW RAM; `Present
  Position` R RAM; limits RW EEPROM. [10, 11]
- Exactly one complete limit form: `Min Position Limit` + `Max Position Limit`, or `CW Angle
  Limit` + `CCW Angle Limit`. Both forms, or half of one, is an error. [10]
- A `Firmware Version` row is present (any access, area). [10, xtask R-4]
- `Model Number` at address 0, size 2, `R`, `EEPROM`. [12]
- No two rows map to the same register constant. The constant is the `data_name` split on
  non-alphanumeric characters, upper-cased and joined with `_` (`Secondary(Shadow) ID` →
  `SECONDARY_SHADOW_ID`). Two rows on one address with different constants are fine. [16]
- Listed under `p1`: no role row and no mode row above address 255 (the mode register counts as
  a role, contract 4.3) [14]; `P1` in `protocols`. Listed under `p2`: `P2` in `protocols`.

Mode source, for every table, mapped or not (module-interfaces 2.4b, 3.1; research R-15). X1
derives `ModelData::mode` from the mode row and `operating_modes`:
- `Register`: one `Operating Mode` row, or one `Control Mode` row, plus `operating_modes`.
  `AngleLimits`: neither row, no `operating_modes`, and the `CW Angle Limit` + `CCW Angle Limit`
  pair.
- Exactly one source. Two rows of one mode name, both names in one table, an `Operating Mode` row
  in a table with CW/CCW Angle Limit rows, or neither name without the CW/CCW pair is an error.
  [17]
- A mode row without `operating_modes`, `operating_modes` without a mode row, or a value in two
  of its lists is an error. A value outside 0–255 is an error. [17; for X1 a value outside u8 or
  a missing key is a parse error, 1]
- `Control Mode` is accepted only in `xl320.ron` (tables check only, contract 4.3). XL-320 is the
  only table with that register; a second one is raised before it lands.
- The mode row has `size: 1`, `access: RW` and `area: EEPROM`. [11]
- A table whose page lists `Operating Mode` or `Control Mode` carries that row and the page's
  value lists in `operating_modes`; a table whose page lists neither carries neither (the MX
  Protocol 1 tables, AX). The check cannot see the page; `emanual.py diff` (for the row) and the
  reviewer (for the row and the lists) do.

Compatibility: today's xtask (`ron` 0.12.2, no `deny_unknown_fields`) reads this format and
produces identical output, `operating_modes` included (research R-1). X1 reads tables with the
`IMPLICIT_SOME` extension, since `operating_modes` is optional (research R-1). X1 may add
`deny_unknown_fields` only for the keys above.

## 2. Model map: `control_tables/dynamixel.ron`

T1 shape (legacy `tables` kept):

```ron
(
  models: {
    12: (family: "ax", name: "AX-12A",
         p1: [(from_fw: 0, file: "ax_12a")],
         tables: [(from_fw: 0, file: "ax_12a")]),
    311: (family: "mx", name: "MX-64(2.0)",
          p1: [(from_fw: 0, file: "mx_64")],
          p2: [(from_fw: 0, file: "mx_64")],
          default: P2,
          tables: [(from_fw: 0, file: "mx_64")]),
    1190: (family: "x-series", name: "XL330-M077",
           p2: [(from_fw: 0, file: "xl330_m077_fw52"), (from_fw: 53, file: "xl330_m077")],
           tables: [(from_fw: 0, file: "xl330_m077_fw52"), (from_fw: 53, file: "xl330_m077")]),
  },
  not_mapped: [
    (file: "ax_12_plus", reason: SameNumberAs(12)),
    (file: "ax_18f", reason: SameNumberAs(18)),
    (file: "xw430_t200", reason: SameNumberAs(1270)),
  ],
)
```

T2 shape: no `tables`; `also` where the page names more products; MX Protocol 1 entries.

```ron
    12: (family: "ax", name: "AX-12A", also: ["AX-12+"], p1: [(from_fw: 0, file: "ax_12a")]),
    310: (family: "mx", name: "MX-64", p1: [(from_fw: 0, file: "mx_64_p1")]),
```

| Key | Grammar | Rule |
|---|---|---|
| `models` | `{ INT: ENTRY, … }` | keys are model numbers (u16), unique |
| `family` | STRING | one of `ax`, `mx`, `x-series`, `y-series`, `pro`; anything else is an error [5] |
| `name` | STRING | the page's product name |
| `also` | `[STRING, …]` | optional; other products under this number |
| `p1`, `p2` | `[(from_fw: INT, file: STRING), …]` | at least one present; non-empty; first `from_fw` 0; strictly increasing; `file` exists |
| `default` | `P2` | present exactly when `p1` and `p2` both exist |
| `tables` | as `p1` | T1 only: equal to `p2` when both exist, else to the only list. Absent after T2: from T2 on, the check rejects a `tables` key in any model entry |
| `not_mapped` | `[(file: STRING, reason: NoModelNumber \| SameNumberAs(INT)), …]` | file exists; not mapped; `SameNumberAs(n)` names a key of `models` |

Whole-map rules:
- Every `<stem>.ron` except `dynamixel.ron` is mapped or in `not_mapped`.
- A file is under at most one model number (both lists of that number are fine).
- If a mapped table lists `[P1, P2]`, its model has both lists.
- Within one model number's `p1` list, every file has its `Firmware Version` row at the same
  (address, size). The same holds within its `p2` list. [13]
- No two mapped files give the same type identifier. The type identifier is the stem with a
  leading digit spelled out (`2xl430_w250` → `two_xl430_w250`), each `_` part capitalised and the
  parts joined (`TwoXl430W250`). [16]
- A mapped file whose module identifier (the stem with a leading digit spelled out) is `tests` or
  a Rust keyword (strict or reserved, 2024 edition) is an error. [16, xtask R-16]

The header comment of `dynamixel.ron` documents these keys in plain words.
