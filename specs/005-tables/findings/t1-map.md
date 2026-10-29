# T1 model map (T045–T047, T050)

## T045: `p1` / `p2` / `default`

Derived from each mapped table's `protocols` (set from the e-Manual in T020, T022, T028–T032):
`[P1]` → `p1`; `[P2]` → `p2`; `[P1, P2]` → `p1` and `p2` equal to the legacy list plus
`default: P2`. Key order `family, name, p1, p2, default, tables`; legacy `tables` unchanged and
last; one entry per line.

- 39 of the 64 entries done: 4 P1-only (12 AX-12A, 18 AX-18A, 300 AX-12W, 28 RX-28), 7 P2-only
  (350 XL-320 and the six XL330/XC330 numbers, each `p2: [(from_fw: 0, file: "…_fw52"),
  (from_fw: 53, file: "…")]`), 28 dual.
- **25 entries not done (blocked)**: 2000, 2010, 2020, 2100, 2110, 2120 (PRO+), 43289, 46097,
  46353, 51201, 53769, 54025 (PRO (A)), 35074 (RH-P12-RN(A)), 4000–4050 and 4120–4170 (Y). Their
  tables have no `protocols` because T033 and T034 stopped (`t1-pro.md`, `t1-y.md`), so the lists
  cannot be derived. They keep only the legacy `tables` list; the check reports
  "model N: no `p1` or `p2` list" for each until T033/T034 run.

## T047: dual-protocol models

`python3 control_tables/check_tables.py`, section `dual-protocol models`: **28** models, each with a
P1 table, a P2 table and `default: P2`, equal to research R-4's expectation (25
X430/X540/2XL/2XC/XW numbers + 3 MX(2.0)). No page changed the count.

```
  30 MX-28(2.0)   p1: mx_28   p2: mx_28   default: P2
  311 MX-64(2.0)   p1: mx_64   p2: mx_64   default: P2
  321 MX-106(2.0)   p1: mx_106   p2: mx_106   default: P2
  1000 XH430-W350   p1: xh430_w350   p2: xh430_w350   default: P2
  1001 XD430-T350   p1: xd430_t350   p2: xd430_t350   default: P2
  1010 XH430-W210   p1: xh430_w210   p2: xh430_w210   default: P2
  1011 XD430-T210   p1: xd430_t210   p2: xd430_t210   default: P2
  1020 XM430-W350   p1: xm430_w350   p2: xm430_w350   default: P2
  1030 XM430-W210   p1: xm430_w210   p2: xm430_w210   default: P2
  1040 XH430-V350   p1: xh430_v350   p2: xh430_v350   default: P2
  1050 XH430-V210   p1: xh430_v210   p2: xh430_v210   default: P2
  1060 XL430-W250   p1: xl430_w250   p2: xl430_w250   default: P2
  1070 XC430-W150   p1: xc430_w150   p2: xc430_w150   default: P2
  1080 XC430-W240   p1: xc430_w240   p2: xc430_w240   default: P2
  1090 2XL430-W250   p1: 2xl430_w250   p2: 2xl430_w250   default: P2
  1100 XH540-W270   p1: xh540_w270   p2: xh540_w270   default: P2
  1101 XD540-T270   p1: xd540_t270   p2: xd540_t270   default: P2
  1110 XH540-W150   p1: xh540_w150   p2: xh540_w150   default: P2
  1111 XD540-T150   p1: xd540_t150   p2: xd540_t150   default: P2
  1120 XM540-W270   p1: xm540_w270   p2: xm540_w270   default: P2
  1130 XM540-W150   p1: xm540_w150   p2: xm540_w150   default: P2
  1140 XH540-V270   p1: xh540_v270   p2: xh540_v270   default: P2
  1150 XH540-V150   p1: xh540_v150   p2: xh540_v150   default: P2
  1160 2XC430-W250   p1: 2xc430_w250   p2: 2xc430_w250   default: P2
  1170 XW540-T260   p1: xw540_t260   p2: xw540_t260   default: P2
  1180 XW540-T140   p1: xw540_t140   p2: xw540_t140   default: P2
  1270 XW430-T333   p1: xw430_t333   p2: xw430_t333   default: P2
  1310 XW540-H260   p1: xw540_h260   p2: xw540_h260   default: P2
```

`cargo xtask codegen && git diff --exit-code src/models.rs src/models/`: empty (codegen still reads
`tables`, US4 AS5, FR-009). US4 AS4 (no table for a protocol → `UnsupportedModel`) is X1's lookup
test; noted for traceability only.

## T050: `not_mapped`

Model numbers re-read on the pages on 2026-10-28 (Model Number row, Initial Value column, "Control
Table of EEPROM Area"):

| File | Page | Model Number | Record |
|---|---|---|---|
| `ax_12_plus` | https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/ (headed "AX-12+, AX-12A") | 12 | `SameNumberAs(12)` |
| `ax_18f` | https://emanual.robotis.com/docs/en/dxl/ax/ax-18a/ (headed "AX-18F, AX-18A") | 18 | `SameNumberAs(18)` |
| `xw430_t200` | https://emanual.robotis.com/docs/en/dxl/x/xw430-t200/ | 1,270 (XW430-T333's page also gives 1,270) | `SameNumberAs(1270)` |

No page gave another number. Added after `models` in `control_tables/dynamixel.ron`.

## T051: validation

`python3 control_tables/check_tables.py`: `summary: 73 tables, 2 verified, 71 unverified, 70 mapped,
3 not mapped, 28 dual-protocol, 5134 errors`, exit 1. The counts match the target; the errors are
all in the blocked tables (`t1-us3-validation.md`) plus the 25 "no `p1` or `p2` list" entries
above. No map or record error: 0 unrecorded files, 0 files under two numbers, 0 duplicate numbers
(the parser rejects them), 0 files unmapped for being unverified. `cargo xtask codegen` prints the
three skip warnings (`ax_12_plus`, `ax_18f`, `xw430_t200`; expected until X1 reads `not_mapped`) and
`git diff --exit-code src/models.rs src/models/` is empty.
