Stacked on #8, #9, #25 and #10; its diff shrinks once they merge.

## What

Tables module, PR **T2** of two (module-interfaces §7), on top of xtask X1. Three priority tables
are verified against their own e-Manual pages (**MX-64(2.0)**, **AX-12+**, and the new **MX-64 on
Protocol 1**); MX-28 and MX-106 on Protocol 1 are added (unverified) and mapped under 29, 310 and
320 (`p1` only). The four e-Manual-less alias rows ("Indirect Address/Data Write/Read") leave all
66 tables that carried them, and the other T1 row findings are applied from the pages. The map
drops the legacy `tables` list (now rejected by the check), names the products sold under a
shared number in `also`, and renames 35074 to RH-P12-RN(A). `src/models*` are regenerated and
committed; they are the only change outside `control_tables/`.

`python3 control_tables/check_tables.py`: `summary: 76 tables, 5 verified, 71 unverified, 73 mapped, 3 not mapped, 28 dual-protocol, 0 errors`.

## Why

FR-003 / US2 AS2–AS3 (rows the e-Manual does not list are removed, corrections from the page),
FR-004 / FR-018 / US2 AS1 (priority models verified, each with its own record), FR-008 / FR-019 /
US5 AS1–AS2 / AS6 (MX on Protocol 1), FR-012 / US5 AS4 (`also`), contract ron-schema §2 ("`tables`
absent after T2"), FR-015 (analysis), FR-017 (this list), FR-020 (mode source).

## How

- **Verified** (every row, `type_info`, `unit_info`, `protocols`, `instructions` from the Protocol
  1.0/2.0 pages, mode source), each with its own record:

  | Table | Record | Notes |
  |---|---|---|
  | `ax_12a` (T1) | https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/ 2026-10-28 | unchanged |
  | `rx_28` (T1) | https://emanual.robotis.com/docs/en/dxl/rx/rx-28/ 2026-10-28 | unchanged |
  | `mx_64` MX-64(2.0), 311 | https://emanual.robotis.com/docs/en/dxl/mx/mx-64-2/ 2026-10-29 | 4 alias rows removed; `unit_info` corrected to the page (velocity 0.229 rpm, current 3.36 mA; the old `0.0239691227 rad/s` and `1.0 raw` are no page values); `instructions` P1 `Ping, Read, Write, RegWrite, Action, FactoryReset, Reboot, SyncWrite, BulkRead`, P2 `Ping, Read, Write, RegWrite, Action, FactoryReset, Reboot, Clear, SyncRead, SyncWrite, BulkRead, BulkWrite`; `operating_modes` `[3]` / `[4, 5]` / `[0, 1, 16]` confirmed on the page |
  | `ax_12_plus` AX-12+ (not mapped, `SameNumberAs(12)`) | https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/ 2026-10-29 | the page is headed "AX-12+, AX-12A" (AX-12+ has no page of its own; T055 approval); `unit_info` `Torque 0.001465 N·m/bit` removed (not on the page, as T1 did for `ax_12a`); `instructions` P1 `Ping, Read, Write, RegWrite, Action, FactoryReset, SyncWrite` |
  | `mx_64_p1` MX-64, 310 (new) | https://emanual.robotis.com/docs/en/dxl/mx/mx-64/ 2026-10-29 | 38 rows from `emanual.py rows`; `instructions` P1 `Ping, Read, Write, RegWrite, Action, FactoryReset, SyncWrite, BulkRead` plus `Reboot` `from_fw: 40` ("MX-28/64/106(V40)") |

- **SC-002 audit**: 5 random rows per verified table, read on the page by eye, 15 of 15 match:
  `mx_64` 98 BUS Watchdog, 78 Velocity P Gain, 34 Min Voltage Limit, 116 Goal Position, 64 Torque
  Enable; `ax_12_plus` 46 Moving, 28 CW Compliance Slope, 16 Status Return Level, 43 Present
  Temperature, 6 CW Angle Limit; `mx_64_p1` 25 LED, 24 Torque Enable, 13 Max Voltage Limit, 42
  Present Voltage, 20 Multi Turn Offset. Every row was also compared by a second, regex-based reader
  of the raw page (0 mismatches).
- **MX on Protocol 1** (`mx_28_p1` 29, `mx_64_p1` 310, `mx_106_p1` 320; `protocols: [P1]`, family
  `mx`): rows from each model's own page only. **Mode source: the CW/CCW Angle Limit pair, no
  `Operating Mode` row, no `operating_modes`** (none of the three pages has an Operating Mode
  register; their "Operating Modes" specification line is Wheel / Joint / Multi-turn, set by
  CW/CCW 0/0, other, 4095/4095). `type_info` 2048 / 0 / 4095, ±π·(2048|2047)/2048 from "0 to 4,095,
  0.088 [°]" and the figure "180° [Goal Position = 2048(0x800)]". MX-106's "Drive Mode (10)" is
  master/slave + reverse, not a mode register.
- **Alias rows** (research R-14): "Indirect Address Write", "Indirect Data Write", "Indirect
  Address Read", "Indirect Data Read" removed from every table whose page does not list the name —
  that is all 66 that had them (MX(2.0) 3, X430 15, X540 11, X330 12, PRO/PRO+/RH 13, Y 12). For
  X330 the T1 open question ("rename or remove" the `Read` rows in elided blocks) is answered:
  removed, since renaming would add numbered indirect rows, which T2 does not add.
- **Other row corrections** (all tables stay `Unverified`):
  - `ph42_020_s300`, `ph54_100_s500`, `ph54_200_s500`: (12, 1) "Secondary ID" → "Secondary(Shadow)
    ID", the pages' name.
  - `rh_p12_rn`: added External Port Data 1–4 (600, 602, 604, 606; size 2; RAM; page access
    `R/RW` recorded `RW`), listed on its page and missing from the table (T1 row-presence finding).
  - **Mode rows**: no correction needed. Every page's `Operating Mode` / `Control Mode` row matched
    its table in T1 and again in every T2 diff; no `operating_modes` list changed.
- **Map** (`dynamixel.ron`): `29: MX-28`, `310: MX-64`, `320: MX-106` with `p1` only;
  `also: ["AX-12+"]` on 12, `["AX-18F"]` on 18, `["XW430-T200"]` on 1270 (the three files stay
  `not_mapped`, `SameNumberAs`); 35074 renamed "RH-P12-RN(A)" (page title, model number 35,074);
  legacy `tables` deleted from all 65 entries and from the header comment.
- **Check** (`check_tables.py`): rejects a `tables` key ("removed in T2"); validates `also` (a
  non-empty list of distinct non-empty names); counts `Hardware Error Status` in the Protocol 1
  address rule, as X1's check 14 does (`resolve.rs` `check_p1_addresses`; found comparing X1 with
  the contract, T058). Tests first for each rule; 115 tests, no network.
- **Analysis**: `analyze_tables.py` reads only `p1`/`p2`; report and JSON regenerated (76 models).

## Tables (FR-017): `check_tables.py --markdown` for the 70 changed tables

<details><summary>70 tables</summary>

- `2xc430_w250`: unverified, [P1, P2], 1160 p1 p2
- `2xl430_w250`: unverified, [P1, P2], 1090 p1 p2
- `ax_12_plus`: verified https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/ 2026-10-29, [P1], not mapped: same number as 12
- `h42_20_s300_ra`: unverified, [P2], 51201 p2
- `h54_100_s500_ra`: unverified, [P2], 53769 p2
- `h54_200_s500_ra`: unverified, [P2], 54025 p2
- `m42_10_s260_ra`: unverified, [P2], 43289 p2
- `m54_40_s250_ra`: unverified, [P2], 46097 p2
- `m54_60_s250_ra`: unverified, [P2], 46353 p2
- `mx_106`: unverified, [P1, P2], 321 p1 p2
- `mx_106_p1`: unverified, [P1], 320 p1
- `mx_28`: unverified, [P1, P2], 30 p1 p2
- `mx_28_p1`: unverified, [P1], 29 p1
- `mx_64`: verified https://emanual.robotis.com/docs/en/dxl/mx/mx-64-2/ 2026-10-29, [P1, P2], 311 p1 p2
- `mx_64_p1`: verified https://emanual.robotis.com/docs/en/dxl/mx/mx-64/ 2026-10-29, [P1], 310 p1
- `ph42_020_s300`: unverified, [P2], 2000 p2
- `ph54_100_s500`: unverified, [P2], 2010 p2
- `ph54_200_s500`: unverified, [P2], 2020 p2
- `pm42_010_s260`: unverified, [P2], 2100 p2
- `pm54_040_s250`: unverified, [P2], 2110 p2
- `pm54_060_s250`: unverified, [P2], 2120 p2
- `rh_p12_rn`: unverified, [P2], 35074 p2
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
- `ym070_210_a051`: unverified, [P2], 4040 p2
- `ym070_210_a099`: unverified, [P2], 4050 p2
- `ym070_210_b001`: unverified, [P2], 4010 p2
- `ym070_210_m001`: unverified, [P2], 4000 p2
- `ym070_210_r051`: unverified, [P2], 4020 p2
- `ym070_210_r099`: unverified, [P2], 4030 p2
- `ym080_230_a051`: unverified, [P2], 4160 p2
- `ym080_230_a099`: unverified, [P2], 4170 p2
- `ym080_230_b001`: unverified, [P2], 4130 p2
- `ym080_230_m001`: unverified, [P2], 4120 p2
- `ym080_230_r051`: unverified, [P2], 4140 p2
- `ym080_230_r099`: unverified, [P2], 4150 p2

</details>

Removed or corrected rows per table (address size name):
- `mx_28`, `mx_64`, `mx_106`, X430 (15), X540 (11): removed 168 2 Indirect Address Write, 224 1
  Indirect Data Write, 578 2 Indirect Address Read, 634 1 Indirect Data Read.
- X330 current (6): removed 168 2 Indirect Address Write, 224 1 Indirect Data Write, 180 2
  Indirect Address Read, 230 1 Indirect Data Read.
- X330 `_fw52` (6): removed 168 2 Indirect Address Write, 208 1 Indirect Data Write, 180 2
  Indirect Address Read, 214 1 Indirect Data Read (kept 208 1 Indirect Data 1, from the page note
  "Indirect Data is 208 ~ 227" before V53).
- PRO (A) (6), PRO+ (6), `rh_p12_rn`: removed 168 2 Indirect Address Write, 634 1 Indirect Data
  Write, 296 2 Indirect Address Read, 698 1 Indirect Data Read; PH (3) renamed 12 1 Secondary ID →
  Secondary(Shadow) ID; `rh_p12_rn` added 600/602/604/606 2 External Port Data 1–4.
- Y (12): removed 256 2 Indirect Address Write, 634 1 Indirect Data Write, 384 2 Indirect Address
  Read, 698 1 Indirect Data Read.
- `ax_12_plus`, `mx_64`: verified, see above. New: `mx_28_p1`, `mx_64_p1`, `mx_106_p1`.

## Generated code (the only change outside `control_tables/`)

Regenerated with `cargo xtask codegen`, never edited by hand; each change traces to a table change:

| Files | Change |
|---|---|
| `src/models/mx_28_p1.rs`, `mx_64_p1.rs`, `mx_106_p1.rs` (new) | `Mx28P1`, `Mx64P1` (verified doc), `Mx106P1`: P1 only, `ModeSource::AngleLimits` |
| `src/models.rs`, `src/models/tests.rs` | modules, `ActuatorKind` variants, `(P1, 29/310/320)` lookup arms and generated tests; "also sold as" docs; "RH-P12-RN(A)" |
| 65 mapped model files | the 4 alias constants (`INDIRECT_ADDRESS_WRITE`, `INDIRECT_DATA_WRITE`, `INDIRECT_ADDRESS_READ`, `INDIRECT_DATA_READ`) removed: 260 constants |
| `mx_64.rs` | plus "Verified against …/mx-64-2/ on 2026-10-29" |
| `ph42_020_s300.rs`, `ph54_100_s500.rs`, `ph54_200_s500.rs` | plus `SECONDARY_ID` → `SECONDARY_SHADOW_ID` |
| `rh_p12_rn.rs` | plus `EXTERNAL_PORT_DATA_1..4`; name "RH-P12-RN(A)" |
| `ax_12a.rs`, `ax_18a.rs`, `xw430_t333.rs` | "also sold as AX-12+ / AX-18F / XW430-T200" |

No `ModeValues` changed. Codegen prints no skip warning; `ax_12_plus`, `ax_18f`, `xw430_t200` stay
skipped by `not_mapped`. No non-generated code used a removed constant.

## Architecture

Follows docs/ARCHITECTURE.md and module-interfaces §3 / §7, with these points for the architect:

- Legacy `tables` removed from the map and now **rejected** by the check (contract ron-schema §2).
- The AX-12+ record cites the AX-12A page ("AX-12+, AX-12A"); AX-12+ has no page of its own (T055).
- `emanual.py diff mx_64` still lists 22 numbered indirect rows "only on page": T060 / research
  R-9 say to add none beyond Indirect Address/Data 1, while T063 asks the diff to exit 0. Kept
  T060; confirm or ask for the rows.
- `Clear` is listed for MX(2.0) on P2: the Protocol 2.0 page sets no product condition on option
  0x01 (only 0x02 is "Y series only"). Confirm.
- MX-64 / MX-106 on Protocol 1 have **Torque Ctrl Mode Enable (70)**: with it set, the actuator
  ignores Goal Position, but the `AngleLimits` mode source does not see it, so a goal write passes
  the bounded check and is ignored by the firmware. Needs a decision (third mode input, or accept).
- Rows the pages list with a firmware condition are not modelled: X "Startup Configuration" (60)
  and "Backup Ready" (147) "available from firmware V45" were **not added**; MX P1 "Realtime Tick"
  ("from Firmware v40") is kept as listed. Decide how per-row firmware conditions are expressed.
- Y "Error Code History 3–14" (156–167) are kept: they fill the page's elided "1, 2, …, 15, 16"
  series with its own names (T1 answer 4 extended); `diff` shows them "only in table".
- The MX P1 pages' Multi Turn Offset note says multi-turn is "both CW and CCW are 0", against the
  CW/CCW table on the same page (4095); the table (and §2.4b) is followed.
- Contract table-tools §1, row "14": should also name `Hardware Error Status` (X1 checks it).

Details and evidence: `specs/005-tables/findings/t2-*.md` (not part of this PR).

## Checklist
- [x] `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass
- [x] `cargo test` passes; line coverage ≥ 70 % (`cargo llvm-cov`: 98.20 %)
- [x] `no_std` build passes (`--no-default-features --features ax,mx,rx,x-series,y-series,pro --target thumbv7em-none-eabihf`)
- [x] `cargo xtask codegen` leaves `src/models.rs` and `src/models/` unchanged (regenerated output committed)
- [x] Public methods documented (no Rust source changed by hand)
- [x] Docs updated if behaviour or setup changed (`dynamixel.ron` header; analysis regenerated)
