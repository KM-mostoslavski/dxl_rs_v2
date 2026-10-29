# T2 PRO / PRO+ / RH group (T069): 13 tables

T1 findings: `t1-pro.md` ("Row findings for T2": the alias rows; the PH tables' name "Secondary
ID" at (12, 1) where the pages say "Secondary(Shadow) ID"; `rh_p12_rn` lacks External Port Data
1–4 although its page lists them). Tables stay `source: Unverified`. Pages from `PAGES`, fetched
2026-10-29.

## Corrections other than alias rows

- **Renamed** (12, 1) "Secondary ID" → "Secondary(Shadow) ID" in `ph42_020_s300`, `ph54_100_s500`,
  `ph54_200_s500` (each page's row: `12 1 "Secondary(Shadow) ID" RW EEPROM`; T2-start diff:
  "different name at the same address and size: 12 1 table "Secondary ID", page "Secondary(Shadow)
  ID""). Generated effect: constant `SECONDARY_ID` → `SECONDARY_SHADOW_ID` in `Ph42020S300`,
  `Ph54100S500`, `Ph54200S500`. The PRO (A) tables keep "Secondary ID": their pages use that name
  (no diff).
- **Added** to `rh_p12_rn` (page https://emanual.robotis.com/docs/en/platform/rh_p12_rna/, rows
  `600 2 "External Port Data 1"`, `602 2 "… 2"`, `604 2 "… 3"`, `606 2 "… 4"`, RAM, access `R/RW`
  on the page, recorded `RW` per the 2026-10-29 provisional answer): four rows after "Present
  Temperature" (594), as in the PRO tables. Page text: "data of external port is controlled by the
  External Port Data(600 ~ 607)". Generated effect: `EXTERNAL_PORT_DATA_1` … `_4` added to
  `RhP12Rn`.

## Alias rows

"Indirect Address Write" (168, 2), "Indirect Data Write" (634, 1), "Indirect Address Read" (296,
2), "Indirect Data Read" (698, 1) removed from all 13 tables: no page lists those names (the
`Read` rows sat in elided blocks, T1 answer 4 extended). Generated effect:
`INDIRECT_ADDRESS_WRITE`, `INDIRECT_DATA_WRITE`, `INDIRECT_ADDRESS_READ`, `INDIRECT_DATA_READ`
leave each of the 13 types.

Diff after: only the 6 numbered indirect rows "only on page" (Indirect Address 2, 3, 128;
Indirect Data 2, 3, 128). Mode row: `Operating Mode` (11, 1, RW, EEPROM) on every page, as the
tables; `operating_modes` unchanged. Note: in the per-table sections below, "Diff before" for the
three PH tables and `rh_p12_rn` was taken after the rename / addition above (the T2-start diffs
had the name difference and the four External Port Data rows "only on page" in addition).

### `h42_20_s300_ra`

Page https://emanual.robotis.com/docs/en/dxl/pro/h42-20-s300-ra/, fetched 2026-10-29.

- Alias names listed on the page: none.
- Removed 168 2 "Indirect Address Write" (not on the page).
- Removed 634 1 "Indirect Data Write" (not on the page).
- Removed 296 2 "Indirect Address Read" (not on the page).
- Removed 698 1 "Indirect Data Read" (not on the page).
- Diff before: 10 differences; only in table (2): 296 2 "Indirect Address Read"; 698 1 "Indirect Data Read" | only on page (6): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (2): 168 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1" | different access/area (0)
- Diff after: 6 differences; only in table (0) | only on page (6): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (0) | different access/area (0)

### `h54_100_s500_ra`

Page https://emanual.robotis.com/docs/en/dxl/pro/h54-100-s500-ra/, fetched 2026-10-29.

- Alias names listed on the page: none.
- Removed 168 2 "Indirect Address Write" (not on the page).
- Removed 634 1 "Indirect Data Write" (not on the page).
- Removed 296 2 "Indirect Address Read" (not on the page).
- Removed 698 1 "Indirect Data Read" (not on the page).
- Diff before: 10 differences; only in table (2): 296 2 "Indirect Address Read"; 698 1 "Indirect Data Read" | only on page (6): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (2): 168 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1" | different access/area (0)
- Diff after: 6 differences; only in table (0) | only on page (6): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (0) | different access/area (0)

### `h54_200_s500_ra`

Page https://emanual.robotis.com/docs/en/dxl/pro/h54-200-s500-ra/, fetched 2026-10-29.

- Alias names listed on the page: none.
- Removed 168 2 "Indirect Address Write" (not on the page).
- Removed 634 1 "Indirect Data Write" (not on the page).
- Removed 296 2 "Indirect Address Read" (not on the page).
- Removed 698 1 "Indirect Data Read" (not on the page).
- Diff before: 10 differences; only in table (2): 296 2 "Indirect Address Read"; 698 1 "Indirect Data Read" | only on page (6): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (2): 168 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1" | different access/area (0)
- Diff after: 6 differences; only in table (0) | only on page (6): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (0) | different access/area (0)

### `m42_10_s260_ra`

Page https://emanual.robotis.com/docs/en/dxl/pro/m42-10-s260-ra/, fetched 2026-10-29.

- Alias names listed on the page: none.
- Removed 168 2 "Indirect Address Write" (not on the page).
- Removed 634 1 "Indirect Data Write" (not on the page).
- Removed 296 2 "Indirect Address Read" (not on the page).
- Removed 698 1 "Indirect Data Read" (not on the page).
- Diff before: 10 differences; only in table (2): 296 2 "Indirect Address Read"; 698 1 "Indirect Data Read" | only on page (6): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (2): 168 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1" | different access/area (0)
- Diff after: 6 differences; only in table (0) | only on page (6): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (0) | different access/area (0)

### `m54_40_s250_ra`

Page https://emanual.robotis.com/docs/en/dxl/pro/m54-40-s250-ra/, fetched 2026-10-29.

- Alias names listed on the page: none.
- Removed 168 2 "Indirect Address Write" (not on the page).
- Removed 634 1 "Indirect Data Write" (not on the page).
- Removed 296 2 "Indirect Address Read" (not on the page).
- Removed 698 1 "Indirect Data Read" (not on the page).
- Diff before: 10 differences; only in table (2): 296 2 "Indirect Address Read"; 698 1 "Indirect Data Read" | only on page (6): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (2): 168 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1" | different access/area (0)
- Diff after: 6 differences; only in table (0) | only on page (6): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (0) | different access/area (0)

### `m54_60_s250_ra`

Page https://emanual.robotis.com/docs/en/dxl/pro/m54-60-s250-ra/, fetched 2026-10-29.

- Alias names listed on the page: none.
- Removed 168 2 "Indirect Address Write" (not on the page).
- Removed 634 1 "Indirect Data Write" (not on the page).
- Removed 296 2 "Indirect Address Read" (not on the page).
- Removed 698 1 "Indirect Data Read" (not on the page).
- Diff before: 10 differences; only in table (2): 296 2 "Indirect Address Read"; 698 1 "Indirect Data Read" | only on page (6): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (2): 168 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1" | different access/area (0)
- Diff after: 6 differences; only in table (0) | only on page (6): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (0) | different access/area (0)

### `ph42_020_s300`

Page https://emanual.robotis.com/docs/en/dxl/p/ph42-020-s300-r/, fetched 2026-10-29.

- Alias names listed on the page: none.
- Removed 168 2 "Indirect Address Write" (not on the page).
- Removed 634 1 "Indirect Data Write" (not on the page).
- Removed 296 2 "Indirect Address Read" (not on the page).
- Removed 698 1 "Indirect Data Read" (not on the page).
- Diff before: 10 differences; only in table (2): 296 2 "Indirect Address Read"; 698 1 "Indirect Data Read" | only on page (6): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (2): 168 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1" | different access/area (0)
- Diff after: 6 differences; only in table (0) | only on page (6): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (0) | different access/area (0)

### `ph54_100_s500`

Page https://emanual.robotis.com/docs/en/dxl/p/ph54-100-s500-r/, fetched 2026-10-29.

- Alias names listed on the page: none.
- Removed 168 2 "Indirect Address Write" (not on the page).
- Removed 634 1 "Indirect Data Write" (not on the page).
- Removed 296 2 "Indirect Address Read" (not on the page).
- Removed 698 1 "Indirect Data Read" (not on the page).
- Diff before: 10 differences; only in table (2): 296 2 "Indirect Address Read"; 698 1 "Indirect Data Read" | only on page (6): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (2): 168 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1" | different access/area (0)
- Diff after: 6 differences; only in table (0) | only on page (6): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (0) | different access/area (0)

### `ph54_200_s500`

Page https://emanual.robotis.com/docs/en/dxl/p/ph54-200-s500-r/, fetched 2026-10-29.

- Alias names listed on the page: none.
- Removed 168 2 "Indirect Address Write" (not on the page).
- Removed 634 1 "Indirect Data Write" (not on the page).
- Removed 296 2 "Indirect Address Read" (not on the page).
- Removed 698 1 "Indirect Data Read" (not on the page).
- Diff before: 10 differences; only in table (2): 296 2 "Indirect Address Read"; 698 1 "Indirect Data Read" | only on page (6): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (2): 168 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1" | different access/area (0)
- Diff after: 6 differences; only in table (0) | only on page (6): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (0) | different access/area (0)

### `pm42_010_s260`

Page https://emanual.robotis.com/docs/en/dxl/p/pm42-010-s260-r/, fetched 2026-10-29.

- Alias names listed on the page: none.
- Removed 168 2 "Indirect Address Write" (not on the page).
- Removed 634 1 "Indirect Data Write" (not on the page).
- Removed 296 2 "Indirect Address Read" (not on the page).
- Removed 698 1 "Indirect Data Read" (not on the page).
- Diff before: 10 differences; only in table (2): 296 2 "Indirect Address Read"; 698 1 "Indirect Data Read" | only on page (6): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (2): 168 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1" | different access/area (0)
- Diff after: 6 differences; only in table (0) | only on page (6): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (0) | different access/area (0)

### `pm54_040_s250`

Page https://emanual.robotis.com/docs/en/dxl/p/pm54-040-s250-r/, fetched 2026-10-29.

- Alias names listed on the page: none.
- Removed 168 2 "Indirect Address Write" (not on the page).
- Removed 634 1 "Indirect Data Write" (not on the page).
- Removed 296 2 "Indirect Address Read" (not on the page).
- Removed 698 1 "Indirect Data Read" (not on the page).
- Diff before: 10 differences; only in table (2): 296 2 "Indirect Address Read"; 698 1 "Indirect Data Read" | only on page (6): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (2): 168 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1" | different access/area (0)
- Diff after: 6 differences; only in table (0) | only on page (6): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (0) | different access/area (0)

### `pm54_060_s250`

Page https://emanual.robotis.com/docs/en/dxl/p/pm54-060-s250-r/, fetched 2026-10-29.

- Alias names listed on the page: none.
- Removed 168 2 "Indirect Address Write" (not on the page).
- Removed 634 1 "Indirect Data Write" (not on the page).
- Removed 296 2 "Indirect Address Read" (not on the page).
- Removed 698 1 "Indirect Data Read" (not on the page).
- Diff before: 10 differences; only in table (2): 296 2 "Indirect Address Read"; 698 1 "Indirect Data Read" | only on page (6): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (2): 168 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1" | different access/area (0)
- Diff after: 6 differences; only in table (0) | only on page (6): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (0) | different access/area (0)

### `rh_p12_rn`

Page https://emanual.robotis.com/docs/en/platform/rh_p12_rna/, fetched 2026-10-29.

- Alias names listed on the page: none.
- Removed 168 2 "Indirect Address Write" (not on the page).
- Removed 634 1 "Indirect Data Write" (not on the page).
- Removed 296 2 "Indirect Address Read" (not on the page).
- Removed 698 1 "Indirect Data Read" (not on the page).
- Diff before: 10 differences; only in table (2): 296 2 "Indirect Address Read"; 698 1 "Indirect Data Read" | only on page (6): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (2): 168 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1" | different access/area (0)
- Diff after: 6 differences; only in table (0) | only on page (6): 170 2 "Indirect Address 2" RW EEPROM; 172 2 "Indirect Address 3" RW EEPROM; 422 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 636 1 "Indirect Data 3" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (0) | different access/area (0)

