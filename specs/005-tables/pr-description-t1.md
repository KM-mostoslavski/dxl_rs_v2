## What

Tables module, PR **T1** of two (module-interfaces §7). Every control table gets a verification
flag, its protocols, `access` + `area` on every row, a position scale checked on its page, and,
where it has a mode register, `operating_modes` copied from its page. The map gets per-protocol
`p1`/`p2` lists and `not_mapped` records. Two maintainer tools land next to
`analyze_tables.py`: `check_tables.py` (the table check, stdlib Python, no network) and
`emanual.py` (reads a model's e-Manual control table; `rows` / `diff` / `fill`). AX-12A and the
new **RX-28** table are verified; RX-28 is mapped under 28 and compiled in behind a new default
feature `rx` (architect, 2026-10-28: the test robot has 10 RX-28 on Protocol 1).

> **Draft: blocked on three questions for the architect (T055, see "Open questions").** The check
> exits 1 on 37 tables whose page values do not fit the schema (25 tables) or have no page row to
> copy from (12 tables). Everything else passes. The RX-28 part is complete and usable from this
> branch today.

## Why

FR-001, FR-002 (flag), FR-005 (protocols), FR-007 (access/area), FR-008–FR-011 (map, not_mapped),
FR-013–FR-016 (unverified stays mapped, check, analysis, no codegen change from T1 data),
FR-018 (AX-12A, RX-28 verified), FR-020 (mode source and `operating_modes`), FR-017 (this list).
X1 needs `source`, `protocols`, `access`, `area`, `operating_modes` and `p1`/`p2` to exist.

## How

- **`control_tables/check_tables.py`** (contract table-tools §1): own RON-subset parser (rejects
  duplicate keys, FR-012), every rule of ron-schema §1–§2, the mode-source and `operating_modes`
  rules, the map and `not_mapped` rules. Output sections `tables`, `dual-protocol models`,
  `errors`, `summary`; `--markdown` for PR lists; exit 0 / 1 / 2. Runs in 0.2 s on 73 tables.
- **`control_tables/emanual.py`** (contract table-tools §2): finds control tables by header name
  (Address, Size, Data Name, Access), takes the area from the EEPROM/RAM heading, `fill` writes
  `access`/`area` only on an exact (address, size, name) match and keeps the row style; stops on
  any access other than R/RW or a row outside an EEPROM/RAM heading. `PAGES` lists the page of all
  73 tables plus the three MX Protocol 1 pages (each opened on 2026-10-28).
- **Tests**: `control_tables/test_check_tables.py` and `test_emanual.py`, 107 tests, no network.
- **Data**: every value from the model's English e-Manual page fetched on 2026-10-28 (instructions
  from the Protocol 1.0 page). Rows that `fill` left were set by hand only from the page row at
  the same address and size.
- **Analysis**: `analyze_tables.py` reads `protocols` and the map's model numbers; report regenerated.

## Tables (FR-017): `python3 control_tables/check_tables.py --markdown`

<details><summary>73 tables</summary>

- `2xc430_w250`: unverified, [P1, P2], 1160 p1 p2
- `2xl430_w250`: unverified, [P1, P2], 1090 p1 p2
- `ax_12_plus`: unverified, [P1], not mapped: same number as 12
- `ax_12a`: verified https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/ 2026-10-28, [P1], 12 p1
- `ax_12w`: unverified, [P1], 300 p1
- `ax_18a`: unverified, [P1], 18 p1
- `ax_18f`: unverified, [P1], not mapped: same number as 18
- `h42_20_s300_ra`: unverified, [?], 51201 tables
- `h54_100_s500_ra`: unverified, [?], 53769 tables
- `h54_200_s500_ra`: unverified, [?], 54025 tables
- `m42_10_s260_ra`: unverified, [?], 43289 tables
- `m54_40_s250_ra`: unverified, [?], 46097 tables
- `m54_60_s250_ra`: unverified, [?], 46353 tables
- `mx_106`: unverified, [P1, P2], 321 p1 p2
- `mx_28`: unverified, [P1, P2], 30 p1 p2
- `mx_64`: unverified, [P1, P2], 311 p1 p2
- `ph42_020_s300`: unverified, [?], 2000 tables
- `ph54_100_s500`: unverified, [?], 2010 tables
- `ph54_200_s500`: unverified, [?], 2020 tables
- `pm42_010_s260`: unverified, [?], 2100 tables
- `pm54_040_s250`: unverified, [?], 2110 tables
- `pm54_060_s250`: unverified, [?], 2120 tables
- `rh_p12_rn`: unverified, [?], 35074 tables
- `rx_28`: verified https://emanual.robotis.com/docs/en/dxl/rx/rx-28/ 2026-10-28, [P1], 28 p1
- `xc330_m181`: unverified, [P2], 1230 p2
- `xc330_m181_fw52`: unverified, [P2], 1230 p2
- `xc330_m288`: unverified, [P2], 1240 p2
- `xc330_m288_fw52`: unverified, [P2], 1240 p2
- `xc330_t181`: unverified, [P2], 1210 p2
- `xc330_t181_fw52`: unverified, [P2], 1210 p2
- `xc330_t288`: unverified, [P2], 1220 p2
- `xc330_t288_fw52`: unverified, [P2], 1220 p2
- `xc430_w150`: unverified, [P1, P2], 1070 p1 p2
- `xc430_w240`: unverified, [P1, P2], 1080 p1 p2
- `xd430_t210`: unverified, [P1, P2], 1011 p1 p2
- `xd430_t350`: unverified, [P1, P2], 1001 p1 p2
- `xd540_t150`: unverified, [P1, P2], 1111 p1 p2
- `xd540_t270`: unverified, [P1, P2], 1101 p1 p2
- `xh430_v210`: unverified, [P1, P2], 1050 p1 p2
- `xh430_v350`: unverified, [P1, P2], 1040 p1 p2
- `xh430_w210`: unverified, [P1, P2], 1010 p1 p2
- `xh430_w350`: unverified, [P1, P2], 1000 p1 p2
- `xh540_v150`: unverified, [P1, P2], 1150 p1 p2
- `xh540_v270`: unverified, [P1, P2], 1140 p1 p2
- `xh540_w150`: unverified, [P1, P2], 1110 p1 p2
- `xh540_w270`: unverified, [P1, P2], 1100 p1 p2
- `xl320`: unverified, [P2], 350 p2
- `xl330_m077`: unverified, [P2], 1190 p2
- `xl330_m077_fw52`: unverified, [P2], 1190 p2
- `xl330_m288`: unverified, [P2], 1200 p2
- `xl330_m288_fw52`: unverified, [P2], 1200 p2
- `xl430_w250`: unverified, [P1, P2], 1060 p1 p2
- `xm430_w210`: unverified, [P1, P2], 1030 p1 p2
- `xm430_w350`: unverified, [P1, P2], 1020 p1 p2
- `xm540_w150`: unverified, [P1, P2], 1130 p1 p2
- `xm540_w270`: unverified, [P1, P2], 1120 p1 p2
- `xw430_t200`: unverified, [P1, P2], not mapped: same number as 1270
- `xw430_t333`: unverified, [P1, P2], 1270 p1 p2
- `xw540_h260`: unverified, [P1, P2], 1310 p1 p2
- `xw540_t140`: unverified, [P1, P2], 1180 p1 p2
- `xw540_t260`: unverified, [P1, P2], 1170 p1 p2
- `ym070_210_a051`: unverified, [?], 4040 tables
- `ym070_210_a099`: unverified, [?], 4050 tables
- `ym070_210_b001`: unverified, [?], 4010 tables
- `ym070_210_m001`: unverified, [?], 4000 tables
- `ym070_210_r051`: unverified, [?], 4020 tables
- `ym070_210_r099`: unverified, [?], 4030 tables
- `ym080_230_a051`: unverified, [?], 4160 tables
- `ym080_230_a099`: unverified, [?], 4170 tables
- `ym080_230_b001`: unverified, [?], 4130 tables
- `ym080_230_m001`: unverified, [?], 4120 tables
- `ym080_230_r051`: unverified, [?], 4140 tables
- `ym080_230_r099`: unverified, [?], 4150 tables

</details>

## Verified tables

| Table | Page | Verified | `type_info` | `instructions` (P1, Protocol 1.0 page) | Mode source |
|---|---|---|---|---|---|
| `ax_12a` | https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/ | 2026-10-28 | 512 / 0 / 1023, −2.6205530 / 2.6154347 (300°/1023, figure 512 → 150°) | Ping, Read, Write, RegWrite, Action, FactoryReset, SyncWrite (Reboot and Bulk Read are conditioned on MX/X products) | CW/CCW Angle Limit pair, no mode register on the page |
| `rx_28` (new) | https://emanual.robotis.com/docs/en/dxl/rx/rx-28/ | 2026-10-28 | same as AX-12A (page: 0 ~ 1,023, 0.29°, 0° ~ 300°, same figure) | same as AX-12A | CW/CCW Angle Limit pair, no mode register |

- AX-12A: `emanual.py diff` 0 differences (32 rows). `unit_info` "Torque 0.001465 N·m/bit" **removed**
  (the page gives torque only as "about 0.1%"); Position 0.29 deg and Speed 0.111 rpm kept (page
  values). Audit, 5 random rows against the page: 3 1 "ID" RW EEPROM; 18 1 "Shutdown" RW EEPROM;
  48 2 "Punch" RW RAM; 13 1 "Max Voltage Limit" RW EEPROM; 26 1 "CW Compliance Margin" RW RAM; all
  match.
- RX-28: 32 rows from `emanual.py rows`, all 32 compared again with the raw page HTML (0
  mismatches), `diff` 0 differences; Model Number initial value 28 (the robot reports 28). No
  Protocol Type register; the page cites only Protocol 1.0. One-line rows (the style of
  `ax_18a.ron`; the task named `ax_12a.ron`, which uses multi-line rows).

## `type_info` changes (each from the model's page, research R-8)

- **41 tables** (MX(2.0) ×3, X430 ×15, X540 ×11, X330 ×12): `min_radian -3.14159265 → -3.1415927`,
  `max_radian 3.14159265 → 3.1400587`. Pages: Goal Position "0.088 [deg/pulse]", "0 ~ 4,095, total
  4,096 counts", figure "180° [Goal Position = 2048(0x800)]" → slope 2π/4096, zero 2048.
- **`xl320`**: `2048 / 0 / 4095, ±3.14159265` → `512 / 0 / 1023, -2.6205530 / 2.6154347` (page: 0 ~ 1,023
  at 0.29°, Joint Mode 0 ~ 300°, figure 512 → 150°).
- **AX ×5** (`ax_12a`, `ax_12_plus`, `ax_12w`, `ax_18a`, `ax_18f`): `min_radian 0.0 → -2.6205530`,
  `max_radian 5.2359877 → 2.6154347` (same slope, angles moved to zero 512). The stale comment
  `// 300° = 5.2359 rad` in `ax_18a`/`ax_18f` went with the old value.
- PRO / PRO+ / RH / Y: not checked (tasks stopped, see below).

## Row findings (not fixed in T1; corrections are T2)

- **Alias rows not on the pages** (research R-14), `access`/`area` taken from the page row at the
  same address and size: "Indirect Address Write" (168, 2) and "Indirect Data Write" (224, 1) from
  Indirect Address/Data 1, "Indirect Address Read" (578, 2) and "Indirect Data Read" (634, 1) from
  Indirect Address/Data 29, all RW RAM, in the 26 X430/X540 tables; Write rows in MX(2.0) ×3 and the
  6 X330 tables likewise; MX(2.0) Read rows from Indirect Address/Data 29.
- **X330 alias rows with no page row**: "Indirect Address Read" (180, 2) and "Indirect Data Read"
  (230, 1) in the 6 current-firmware X330 tables sit in the rows the page elides ("…" between
  Indirect Address 3 and 26); left without `access`/`area` (question 3).
- **Rows on the pages that the tables lack**: "Startup Configuration" (60, 1, RW, EEPROM) and "Backup
  Ready" (147, 1, R, RAM) on every X430/X540/X330 page; the numbered indirect rows the tables never
  carried (MX(2.0), X).
- No address, size or name of an existing row differs from its page (AX, MX(2.0), XL-320, X430,
  X540, X330 current firmware).

### Mode rows (FR-020)

Every mode row checked matches its page: `Operating Mode` (11, 1, RW, EEPROM) on MX(2.0), X430,
X540 and X330; `Control Mode` (11, 1, RW, EEPROM) on XL-320; no mode register on the AX and RX-28
pages (CW/CCW pair). **No mode-row finding.** The `_fw52` X330 rows could only be compared with
the current-firmware table (the pages give no firmware-52 table). PRO / PRO+ / RH / Y mode rows
were not checked (tasks stopped).

## `operating_modes` (each from its own page, 2026-10-28)

| Tables | Written |
|---|---|
| `mx_64`, `mx_106`, 10 X430 (`xd430_*`, `xh430_*`, `xm430_*`, `xw430_*`), 11 X540, 12 X330 (incl. `_fw52`) | `(bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16])` |
| `mx_28`, `xl430_w250`, `2xl430_w250`, `2xc430_w250`, `xc430_w150`, `xc430_w240` | `(bounded: [3], unbounded: [4], no_position: [1, 16])` |
| `xl320` (`Control Mode`) | `(bounded: [2], unbounded: [], no_position: [1])` |

Pages: Operating Mode(11) value table; Min/Max Position Limit "These values are not used in
Extended Position Control Mode and Current-based Position Control Mode" (or "…not used in Extended
Position Control Mode" on pages without mode 5); Position Control Mode "Operating position range is
limited by the Max Position Limit(48) and the Min Position Limit(52)"; XL-320 "CW Angle Limit: the
minimum value of Goal Position(30)", "If it is set to Wheel Mode, Goal Position value is not used."
X330 pages give one mode description for all firmware, so it applies to the `_fw52` tables too.

**Pages that differ from the expected family list** (research R-15; the page wins):
- `mx_28` (MX-28(2.0), https://emanual.robotis.com/docs/en/dxl/mx/mx-28-2/): lists only 1, 3, 4, 16
  (no 0, no 5), unlike MX-64/106(2.0).
- `xc430_w150`, `xc430_w240` (…/x/xc430-w150/, …/x/xc430-w240/): list only 1, 3, 4, 16.

No value was left unplaced; no reserved value appeared.

## Map

28 dual-protocol models, each with `default: P2` (research R-4 expected 28):

<details><summary>dual-protocol models</summary>

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

</details>

`not_mapped` (model numbers re-read on the pages 2026-10-28): `ax_12_plus` → `SameNumberAs(12)`
(AX-12A page "AX-12+, AX-12A", 12), `ax_18f` → `SameNumberAs(18)` (AX-18A page "AX-18F, AX-18A",
18), `xw430_t200` → `SameNumberAs(1270)` (XW430-T200 page 1,270; XW430-T333 also 1,270).
25 entries (PRO / PRO+ / RH / Y) keep only the legacy `tables` list until their tables have
`protocols` (question 1, 2).

## Open questions for the architect (T055; block the check)

1. **PRO / PRO+ / RH (13 tables, T033 stopped):** all 13 pages give access **`R/RW`** for External
   Port Data 1–4 (600–606, size 2); the access depends on External Port Mode (input R, output W).
   The schema allows R | RW only. Record as a new access value, as RW, or another rule? T033 and
   T040 (`operating_modes`) wait for it.
2. **Y (12 tables, T034 stopped):** the pages define a third area **`Hybrid`** ("behaves like the
   EEPROM area" with torque off, "like the RAM area" with torque on) for 11 rows (gains 212–236,
   profile 240–252), and give the area in a column, not a heading. The schema allows EEPROM | RAM.
   Record as a new area value, map it to EEPROM or RAM, or another rule? `emanual.py` then needs
   to read an `Area` column. T034 and T041 wait for it.
3. **X330 (12 tables, T032):** (a) the six `_fw52` tables: the pages give no firmware-52 control
   table, only "Firmware versions prior to V53 support up to 20 Indirect Address/Data items. The
   address range for Indirect Address is 168 ~ 207, and for Indirect Data is 208 ~ 227." Per T032
   nothing was filled. May the current table plus this note serve as the firmware-52 source?
   (b) the six current-firmware tables: alias rows at 180 and 230 have no page row (elided "…"):
   give them the elided block's access/area, or wait for their removal in T2?

Also for T055: approve the T1/T2 split (module-interfaces §9) and the AX-12+ record citing the
AX-12A page (research R-9).

## Architecture

Follows module-interfaces §1, §2.4b, §3, §4.3, §7 and the specs in `specs/005-tables/`, with these
deviations:
- **Two PRs for one module** (T1 now, T2 after X1); approval asked in module-interfaces §9.
- **Generated change in T1: RX-28 only.** `Cargo.toml` gains the default feature `rx`;
  `cargo xtask codegen` adds `src/models/rx_28.rs` (type `Rx28`) and 20 lines in `src/models.rs`.
  No other generated file changed. CI's no_std job lists the families explicitly and does not
  include `rx` yet (ci's file; noted for ci).
- **Family `rx`**: ron-schema §2 lists five families; the check also accepts `rx` (architect,
  2026-10-28: "X1 accepts family `rx`").
- XW430-T200 under 1270 as `SameNumberAs(1270)`, now as the contract shows (both pages re-checked
  2026-10-28). AX-18F as `SameNumberAs(18)`, not in the contract example.
- `instructions` entries carry `source` and an optional `from_fw` (contract 3.1 leaves the shape
  open).
- The check has rules codegen does not: `Control Mode` only in `xl320`, mode-source and
  `operating_modes` rules on unmapped tables too, unrecorded files are errors, and in T1 a model
  entry must carry the legacy `tables` equal to its default list (today's codegen reads it).
- The check duplicates xtask's data checks 2–17 (mode source and `operating_modes` included) so T1
  cannot land data X1 rejects.
- `operating_modes` sits after `protocols` (after `instructions` when present), before
  `type_info` (contract 3.1 fixes no position).
- AX-12+ cites the AX-12A page (no page of its own).
- `emanual.py` stops on page values outside the schema (as the contract says); that is what
  blocked the PRO and Y groups.

Notes for other modules:
- **X1**: after T1 the 4096-pulse tables read `PositionScale::new(2048, 0, 4095, -3.1415927,
  3.1400587)` and AX / XL-320 / RX-28 `(512, 0, 1023, -2.6205530, 2.6154347)`. One model type per
  file even when it is under `p1` and `p2`. Every table gives exactly one mode source;
  `Operating Mode` at 11 (33 on Y); `operating_modes` on 42 tables now, 67 once the PRO and Y
  groups land. Read tables with `IMPLICIT_SOME` (research R-1). Accept family `rx`.
- **core**: slope 2π/4096 now in the tables; the mode value lists live in each table's
  `operating_modes`, so core holds none. Some pages give fewer modes than their family (above).
- **ci**: remove the `if [ -f control_tables/check_tables.py ]` guard in its next change; add `rx`
  to the no_std feature list.

## Checklist
- [x] `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass
- [x] `cargo test` passes; line coverage ≥ 70 % (`cargo llvm-cov`) (no hand-written Rust changed)
- [x] `no_std` build passes (`--no-default-features --features ax,mx,rx,x-series,y-series,pro --target thumbv7em-none-eabihf`)
- [x] `cargo xtask codegen` leaves `src/models.rs` and `src/models/` unchanged (the RX-28 output is committed)
- [x] Public methods documented (Python tools: module and function docstrings)
- [x] Docs updated if behaviour or setup changed (`dynamixel.ron` header documents the new keys)
- [ ] `python3 control_tables/check_tables.py` exits 0: **no**, 5134 errors in the 37 blocked tables (open questions); `python3 -m unittest discover -s control_tables -p 'test_*.py'`: 107 tests pass
