# Contract: Control-table and model-map RON format

**Feature**: [../spec.md](../spec.md) | **Model**: [../data-model.md](../data-model.md) |
**Shared contract**: `specs/ideas/module-interfaces.md` section 3

This is the format `xtask` (X1) reads. It makes module-interfaces section 3 exact. Where that
section leaves a choice to this module (`instructions`), the choice is stated here. The table
check enforces every rule below.

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

| Key | Grammar | Rule |
|---|---|---|
| `source` | `Unverified` \| `Verified(url: STRING, verified: STRING)` | required; URL prefix `https://emanual.robotis.com/`; date ISO `YYYY-MM-DD`, real, not in the future |
| `protocols` | `[PROTO (, PROTO)?]`, `PROTO = P1 \| P2` | required, non-empty, no duplicates; written `[P1]`, `[P2]` or `[P1, P2]` |
| `instructions` | `[ENTRY, …]`, `ENTRY = (protocol: PROTO, source: STRING, list: [INSTR, …] (, from_fw: INT)?)` | only when `source` is `Verified`, then at least one entry per protocol in `protocols`; `protocol` in `protocols`; `source` URL as above; `list` non-empty, no duplicates; two entries of one protocol differ in `from_fw` |
| `INSTR` | `Ping \| Read \| Write \| RegWrite \| Action \| FactoryReset \| Reboot \| Clear \| ControlTableBackup \| SyncRead \| SyncWrite \| FastSyncRead \| BulkRead \| BulkWrite \| FastBulkRead` | P1 entries: only `Ping, Read, Write, RegWrite, Action, FactoryReset, Reboot, SyncWrite, BulkRead` |
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
- Listed under `p1`: no role row above address 255; `P1` in `protocols`. Listed under `p2`: `P2`
  in `protocols`.

Compatibility: today's xtask (`ron` 0.12.2, no `deny_unknown_fields`) reads this format and
produces identical output (research R-1). X1 may add `deny_unknown_fields` only for the keys
above.

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
| `tables` | as `p1` | T1 only: equal to `p2` when both exist, else to the only list. Absent after T2 |
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
