# T1 PRO / PRO+ / RH group (T033), completed 2026-10-29

First run (2026-10-28) stopped: every page gives access **`R/RW`** for External Port Data 1–4 (600, 602, 604, 606; size 2; RAM). The page's function table gives the access per External Port Mode (Analogue Input `R`, Digital Output `W`, …).

**Answer 1 (architect, provisional, 2026-10-29; spec Clarifications):** record `RW`. `emanual.py` maps a page `R/RW` to `RW` (`ACCESS_RECORDED_AS`, with a test); any other non-R/RW access still stops it. Page value: `R/RW` on all 13 pages; the 12 PRO tables with those rows record `RW`. `rh_p12_rn` has no External Port Data rows although its page lists them (row-presence finding for T2).

**Answer 4 extended (flag for the architect):** "Indirect Address Read" (296, 2) and "Indirect Data Read" (698, 1) sit in the elided "…" parts of the Indirect Address 1–128 / Indirect Data 1–128 blocks and take the elided block's access/area (rows on both sides agree). Answer 4 named the X330 rows; the same rule is applied here.

Group results:
- `protocols: [P2]`. PRO+ and RH-P12-RN(A): Protocol Type(13) values 2 = DYNAMIXEL Protocol 2.0, 10 = Modbus-RTU. PRO (A): no Protocol Type register; the page refers only to Protocol 2.0 ("Please refer to DYNAMIXEL Protocol 2.0 for more details about Instruction Packets", "refer to the Status Packet section of Protocol 2.0").
- `type_info` unchanged on the 12 PRO tables: each page gives "-180 ~ 180 [°]" ↔ ±N with the limit range ±N equal to the table (H42/PH42 ±303,750; H54/PH54 ±501,923; M42/PM42 ±263,187; M54/PM54 ±251,417). Note: the M42-10-S260-R(A) page gives 526,375 pulse/rev against ±263,187 (PM42: 526,374).
- **`rh_p12_rn` `type_info` left unchanged and raised:** table `0 / -740 / 740`, ±1.099 rad; the RH-P12-RN(A) page gives stroke 0 ~ 106 mm, Goal Position figure 0 (open) / 740 (closed), limit range "0 ~ 1,150", "Degree of Precision 0.088°", no radian scale. Neither the negative range nor ±1.099 rad is on the page; a replacement would be a guess. Question for the architect.
- Mode rows: `Operating Mode` (11, 1, RW, EEPROM) on every page, as the table. No mode-row finding.
- Row findings for T2: alias rows "Indirect Address Write" (168, 2; page Indirect Address 1, RW **EEPROM**) and "Indirect Data Write" (634, 1; Indirect Data 1, RW RAM); PH tables name (12, 1) "Secondary ID", the pages "Secondary(Shadow) ID" (different register constant).

### `h42_20_s300_ra`

Page https://emanual.robotis.com/docs/en/dxl/pro/h42-20-s300-ra/, fetched 2026-10-29.

- `fill`: 63 rows (exact address, size, name).
- By hand, page row at the same address and size: 168 2 "Indirect Address Write" → RW EEPROM (page: Indirect Address 1).
- By hand, page row at the same address and size: 634 1 "Indirect Data Write" → RW RAM (page: Indirect Data 1).
- Elided block (answer 4): 296 2 "Indirect Address Read" → RW EEPROM (between "Indirect Address 3" (172) and "Indirect Address 128" (422), both RW EEPROM).
- Elided block (answer 4): 698 1 "Indirect Data Read" → RW RAM (between "Indirect Data 3" (636) and "Indirect Data 128" (761), both RW RAM).
- Only on the page (not added in T1): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM.
- Name differences at the same address and size: 168 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1".

### `h54_100_s500_ra`

Page https://emanual.robotis.com/docs/en/dxl/pro/h54-100-s500-ra/, fetched 2026-10-29.

- `fill`: 63 rows (exact address, size, name).
- By hand, page row at the same address and size: 168 2 "Indirect Address Write" → RW EEPROM (page: Indirect Address 1).
- By hand, page row at the same address and size: 634 1 "Indirect Data Write" → RW RAM (page: Indirect Data 1).
- Elided block (answer 4): 296 2 "Indirect Address Read" → RW EEPROM (between "Indirect Address 3" (172) and "Indirect Address 128" (422), both RW EEPROM).
- Elided block (answer 4): 698 1 "Indirect Data Read" → RW RAM (between "Indirect Data 3" (636) and "Indirect Data 128" (761), both RW RAM).
- Only on the page (not added in T1): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM.
- Name differences at the same address and size: 168 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1".

### `h54_200_s500_ra`

Page https://emanual.robotis.com/docs/en/dxl/pro/h54-200-s500-ra/, fetched 2026-10-29.

- `fill`: 63 rows (exact address, size, name).
- By hand, page row at the same address and size: 168 2 "Indirect Address Write" → RW EEPROM (page: Indirect Address 1).
- By hand, page row at the same address and size: 634 1 "Indirect Data Write" → RW RAM (page: Indirect Data 1).
- Elided block (answer 4): 296 2 "Indirect Address Read" → RW EEPROM (between "Indirect Address 3" (172) and "Indirect Address 128" (422), both RW EEPROM).
- Elided block (answer 4): 698 1 "Indirect Data Read" → RW RAM (between "Indirect Data 3" (636) and "Indirect Data 128" (761), both RW RAM).
- Only on the page (not added in T1): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM.
- Name differences at the same address and size: 168 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1".

### `m42_10_s260_ra`

Page https://emanual.robotis.com/docs/en/dxl/pro/m42-10-s260-ra/, fetched 2026-10-29.

- `fill`: 63 rows (exact address, size, name).
- By hand, page row at the same address and size: 168 2 "Indirect Address Write" → RW EEPROM (page: Indirect Address 1).
- By hand, page row at the same address and size: 634 1 "Indirect Data Write" → RW RAM (page: Indirect Data 1).
- Elided block (answer 4): 296 2 "Indirect Address Read" → RW EEPROM (between "Indirect Address 3" (172) and "Indirect Address 128" (422), both RW EEPROM).
- Elided block (answer 4): 698 1 "Indirect Data Read" → RW RAM (between "Indirect Data 3" (636) and "Indirect Data 128" (761), both RW RAM).
- Only on the page (not added in T1): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM.
- Name differences at the same address and size: 168 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1".

### `m54_40_s250_ra`

Page https://emanual.robotis.com/docs/en/dxl/pro/m54-40-s250-ra/, fetched 2026-10-29.

- `fill`: 63 rows (exact address, size, name).
- By hand, page row at the same address and size: 168 2 "Indirect Address Write" → RW EEPROM (page: Indirect Address 1).
- By hand, page row at the same address and size: 634 1 "Indirect Data Write" → RW RAM (page: Indirect Data 1).
- Elided block (answer 4): 296 2 "Indirect Address Read" → RW EEPROM (between "Indirect Address 3" (172) and "Indirect Address 128" (422), both RW EEPROM).
- Elided block (answer 4): 698 1 "Indirect Data Read" → RW RAM (between "Indirect Data 3" (636) and "Indirect Data 128" (761), both RW RAM).
- Only on the page (not added in T1): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM.
- Name differences at the same address and size: 168 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1".

### `m54_60_s250_ra`

Page https://emanual.robotis.com/docs/en/dxl/pro/m54-60-s250-ra/, fetched 2026-10-29.

- `fill`: 63 rows (exact address, size, name).
- By hand, page row at the same address and size: 168 2 "Indirect Address Write" → RW EEPROM (page: Indirect Address 1).
- By hand, page row at the same address and size: 634 1 "Indirect Data Write" → RW RAM (page: Indirect Data 1).
- Elided block (answer 4): 296 2 "Indirect Address Read" → RW EEPROM (between "Indirect Address 3" (172) and "Indirect Address 128" (422), both RW EEPROM).
- Elided block (answer 4): 698 1 "Indirect Data Read" → RW RAM (between "Indirect Data 3" (636) and "Indirect Data 128" (761), both RW RAM).
- Only on the page (not added in T1): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM.
- Name differences at the same address and size: 168 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1".

### `ph42_020_s300`

Page https://emanual.robotis.com/docs/en/dxl/p/ph42-020-s300-r/, fetched 2026-10-29.

- `fill`: 65 rows (exact address, size, name).
- By hand, page row at the same address and size: 12 1 "Secondary ID" → RW EEPROM (page: Secondary(Shadow) ID).
- By hand, page row at the same address and size: 168 2 "Indirect Address Write" → RW EEPROM (page: Indirect Address 1).
- By hand, page row at the same address and size: 634 1 "Indirect Data Write" → RW RAM (page: Indirect Data 1).
- Elided block (answer 4): 296 2 "Indirect Address Read" → RW EEPROM (between "Indirect Address 3" (172) and "Indirect Address 128" (422), both RW EEPROM).
- Elided block (answer 4): 698 1 "Indirect Data Read" → RW RAM (between "Indirect Data 3" (636) and "Indirect Data 128" (761), both RW RAM).
- Only on the page (not added in T1): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM.
- Name differences at the same address and size: 12 1 table "Secondary ID", page "Secondary(Shadow) ID"; 168 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1".

### `ph54_100_s500`

Page https://emanual.robotis.com/docs/en/dxl/p/ph54-100-s500-r/, fetched 2026-10-29.

- `fill`: 65 rows (exact address, size, name).
- By hand, page row at the same address and size: 12 1 "Secondary ID" → RW EEPROM (page: Secondary(Shadow) ID).
- By hand, page row at the same address and size: 168 2 "Indirect Address Write" → RW EEPROM (page: Indirect Address 1).
- By hand, page row at the same address and size: 634 1 "Indirect Data Write" → RW RAM (page: Indirect Data 1).
- Elided block (answer 4): 296 2 "Indirect Address Read" → RW EEPROM (between "Indirect Address 3" (172) and "Indirect Address 128" (422), both RW EEPROM).
- Elided block (answer 4): 698 1 "Indirect Data Read" → RW RAM (between "Indirect Data 3" (636) and "Indirect Data 128" (761), both RW RAM).
- Only on the page (not added in T1): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM.
- Name differences at the same address and size: 12 1 table "Secondary ID", page "Secondary(Shadow) ID"; 168 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1".

### `ph54_200_s500`

Page https://emanual.robotis.com/docs/en/dxl/p/ph54-200-s500-r/, fetched 2026-10-29.

- `fill`: 65 rows (exact address, size, name).
- By hand, page row at the same address and size: 12 1 "Secondary ID" → RW EEPROM (page: Secondary(Shadow) ID).
- By hand, page row at the same address and size: 168 2 "Indirect Address Write" → RW EEPROM (page: Indirect Address 1).
- By hand, page row at the same address and size: 634 1 "Indirect Data Write" → RW RAM (page: Indirect Data 1).
- Elided block (answer 4): 296 2 "Indirect Address Read" → RW EEPROM (between "Indirect Address 3" (172) and "Indirect Address 128" (422), both RW EEPROM).
- Elided block (answer 4): 698 1 "Indirect Data Read" → RW RAM (between "Indirect Data 3" (636) and "Indirect Data 128" (761), both RW RAM).
- Only on the page (not added in T1): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM.
- Name differences at the same address and size: 12 1 table "Secondary ID", page "Secondary(Shadow) ID"; 168 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1".

### `pm42_010_s260`

Page https://emanual.robotis.com/docs/en/dxl/p/pm42-010-s260-r/, fetched 2026-10-29.

- `fill`: 66 rows (exact address, size, name).
- By hand, page row at the same address and size: 168 2 "Indirect Address Write" → RW EEPROM (page: Indirect Address 1).
- By hand, page row at the same address and size: 634 1 "Indirect Data Write" → RW RAM (page: Indirect Data 1).
- Elided block (answer 4): 296 2 "Indirect Address Read" → RW EEPROM (between "Indirect Address 3" (172) and "Indirect Address 128" (422), both RW EEPROM).
- Elided block (answer 4): 698 1 "Indirect Data Read" → RW RAM (between "Indirect Data 3" (636) and "Indirect Data 128" (761), both RW RAM).
- Only on the page (not added in T1): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM.
- Name differences at the same address and size: 168 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1".

### `pm54_040_s250`

Page https://emanual.robotis.com/docs/en/dxl/p/pm54-040-s250-r/, fetched 2026-10-29.

- `fill`: 66 rows (exact address, size, name).
- By hand, page row at the same address and size: 168 2 "Indirect Address Write" → RW EEPROM (page: Indirect Address 1).
- By hand, page row at the same address and size: 634 1 "Indirect Data Write" → RW RAM (page: Indirect Data 1).
- Elided block (answer 4): 296 2 "Indirect Address Read" → RW EEPROM (between "Indirect Address 3" (172) and "Indirect Address 128" (422), both RW EEPROM).
- Elided block (answer 4): 698 1 "Indirect Data Read" → RW RAM (between "Indirect Data 3" (636) and "Indirect Data 128" (761), both RW RAM).
- Only on the page (not added in T1): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM.
- Name differences at the same address and size: 168 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1".

### `pm54_060_s250`

Page https://emanual.robotis.com/docs/en/dxl/p/pm54-060-s250-r/, fetched 2026-10-29.

- `fill`: 66 rows (exact address, size, name).
- By hand, page row at the same address and size: 168 2 "Indirect Address Write" → RW EEPROM (page: Indirect Address 1).
- By hand, page row at the same address and size: 634 1 "Indirect Data Write" → RW RAM (page: Indirect Data 1).
- Elided block (answer 4): 296 2 "Indirect Address Read" → RW EEPROM (between "Indirect Address 3" (172) and "Indirect Address 128" (422), both RW EEPROM).
- Elided block (answer 4): 698 1 "Indirect Data Read" → RW RAM (between "Indirect Data 3" (636) and "Indirect Data 128" (761), both RW RAM).
- Only on the page (not added in T1): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM.
- Name differences at the same address and size: 168 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1".

### `rh_p12_rn`

Page https://emanual.robotis.com/docs/en/platform/rh_p12_rna/, fetched 2026-10-29.

- `fill`: 59 rows (exact address, size, name).
- By hand, page row at the same address and size: 168 2 "Indirect Address Write" → RW EEPROM (page: Indirect Address 1).
- By hand, page row at the same address and size: 634 1 "Indirect Data Write" → RW RAM (page: Indirect Data 1).
- Elided block (answer 4): 296 2 "Indirect Address Read" → RW EEPROM (between "Indirect Address 3" (172) and "Indirect Address 128" (422), both RW EEPROM).
- Elided block (answer 4): 698 1 "Indirect Data Read" → RW RAM (between "Indirect Data 3" (636) and "Indirect Data 128" (761), both RW RAM).
- Only on the page (not added in T1): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 600 2 "External Port Data 1" RW RAM; 602 2 "External Port Data 2" RW RAM; 604 2 "External Port Data 3" RW RAM; 606 2 "External Port Data 4" RW RAM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM.
- Name differences at the same address and size: 168 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1".

