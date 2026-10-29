# T2 Y group (T070): 12 tables

T1 findings: `t1-y.md` ("Row findings for T2": the alias rows "Indirect Address Write" (256, 2)
and "Indirect Data Write" (634, 1)). Tables stay `source: Unverified`. Pages from `PAGES`, fetched
2026-10-29.

Removed from all 12 tables: 256 2 "Indirect Address Write", 634 1 "Indirect Data Write", 384 2
"Indirect Address Read", 698 1 "Indirect Data Read" (no page lists those names; the `Read` rows
sat in elided blocks, T1 answer 4 extended). Generated effect: `INDIRECT_ADDRESS_WRITE`,
`INDIRECT_DATA_WRITE`, `INDIRECT_ADDRESS_READ`, `INDIRECT_DATA_READ` leave each of the 12 types.

Kept: "Error Code History 3" … "14" (156–167, size 1, R RAM). They are not alias names: each page
lists the series "Error Code History 1, 2, …, 15, 16" with the middle elided, and these rows fill
exactly the elided addresses with the series' own names (T1 answer 4 extended). T1 did not list
them as a T2 correction. They remain "only in table" in `emanual.py diff` (the page prints "…"),
raised again for the architect together with the numbered indirect rows.

Diff after: the 12 Error Code History rows "only in table" and 6 numbered indirect rows "only on
page" (Indirect Address 2, 127, 128; Indirect Data 2, 127, 128). Mode row: `Operating Mode` (33, 1,
RW, EEPROM) on every page, as the tables; `operating_modes` unchanged.

### `ym070_210_a051`

Page https://emanual.robotis.com/docs/en/dxl/y/ym070-210-a051-rh/, fetched 2026-10-29.

- Alias names listed on the page: none.
- Removed 256 2 "Indirect Address Write" (not on the page).
- Removed 634 1 "Indirect Data Write" (not on the page).
- Removed 384 2 "Indirect Address Read" (not on the page).
- Removed 698 1 "Indirect Data Read" (not on the page).
- Diff before: 22 differences; only in table (14): 156 1 "Error Code History 3"; 157 1 "Error Code History 4"; 158 1 "Error Code History 5"; 159 1 "Error Code History 6"; 160 1 "Error Code History 7"; 161 1 "Error Code History 8"; 162 1 "Error Code History 9"; 163 1 "Error Code History 10"; 164 1 "Error Code History 11"; 165 1 "Error Code History 12"; 166 1 "Error Code History 13"; 167 1 "Error Code History 14"; 384 2 "Indirect Address Read"; 698 1 "Indirect Data Read" | only on page (6): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (2): 256 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1" | different access/area (0)
- Diff after: 18 differences; only in table (12): 156 1 "Error Code History 3"; 157 1 "Error Code History 4"; 158 1 "Error Code History 5"; 159 1 "Error Code History 6"; 160 1 "Error Code History 7"; 161 1 "Error Code History 8"; 162 1 "Error Code History 9"; 163 1 "Error Code History 10"; 164 1 "Error Code History 11"; 165 1 "Error Code History 12"; 166 1 "Error Code History 13"; 167 1 "Error Code History 14" | only on page (6): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (0) | different access/area (0)

### `ym070_210_a099`

Page https://emanual.robotis.com/docs/en/dxl/y/ym070-210-a099-rh/, fetched 2026-10-29.

- Alias names listed on the page: none.
- Removed 256 2 "Indirect Address Write" (not on the page).
- Removed 634 1 "Indirect Data Write" (not on the page).
- Removed 384 2 "Indirect Address Read" (not on the page).
- Removed 698 1 "Indirect Data Read" (not on the page).
- Diff before: 22 differences; only in table (14): 156 1 "Error Code History 3"; 157 1 "Error Code History 4"; 158 1 "Error Code History 5"; 159 1 "Error Code History 6"; 160 1 "Error Code History 7"; 161 1 "Error Code History 8"; 162 1 "Error Code History 9"; 163 1 "Error Code History 10"; 164 1 "Error Code History 11"; 165 1 "Error Code History 12"; 166 1 "Error Code History 13"; 167 1 "Error Code History 14"; 384 2 "Indirect Address Read"; 698 1 "Indirect Data Read" | only on page (6): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (2): 256 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1" | different access/area (0)
- Diff after: 18 differences; only in table (12): 156 1 "Error Code History 3"; 157 1 "Error Code History 4"; 158 1 "Error Code History 5"; 159 1 "Error Code History 6"; 160 1 "Error Code History 7"; 161 1 "Error Code History 8"; 162 1 "Error Code History 9"; 163 1 "Error Code History 10"; 164 1 "Error Code History 11"; 165 1 "Error Code History 12"; 166 1 "Error Code History 13"; 167 1 "Error Code History 14" | only on page (6): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (0) | different access/area (0)

### `ym070_210_b001`

Page https://emanual.robotis.com/docs/en/dxl/y/ym070-210-b001-rh/, fetched 2026-10-29.

- Alias names listed on the page: none.
- Removed 256 2 "Indirect Address Write" (not on the page).
- Removed 634 1 "Indirect Data Write" (not on the page).
- Removed 384 2 "Indirect Address Read" (not on the page).
- Removed 698 1 "Indirect Data Read" (not on the page).
- Diff before: 22 differences; only in table (14): 156 1 "Error Code History 3"; 157 1 "Error Code History 4"; 158 1 "Error Code History 5"; 159 1 "Error Code History 6"; 160 1 "Error Code History 7"; 161 1 "Error Code History 8"; 162 1 "Error Code History 9"; 163 1 "Error Code History 10"; 164 1 "Error Code History 11"; 165 1 "Error Code History 12"; 166 1 "Error Code History 13"; 167 1 "Error Code History 14"; 384 2 "Indirect Address Read"; 698 1 "Indirect Data Read" | only on page (6): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (2): 256 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1" | different access/area (0)
- Diff after: 18 differences; only in table (12): 156 1 "Error Code History 3"; 157 1 "Error Code History 4"; 158 1 "Error Code History 5"; 159 1 "Error Code History 6"; 160 1 "Error Code History 7"; 161 1 "Error Code History 8"; 162 1 "Error Code History 9"; 163 1 "Error Code History 10"; 164 1 "Error Code History 11"; 165 1 "Error Code History 12"; 166 1 "Error Code History 13"; 167 1 "Error Code History 14" | only on page (6): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (0) | different access/area (0)

### `ym070_210_m001`

Page https://emanual.robotis.com/docs/en/dxl/y/ym070-210-m001-rh/, fetched 2026-10-29.

- Alias names listed on the page: none.
- Removed 256 2 "Indirect Address Write" (not on the page).
- Removed 634 1 "Indirect Data Write" (not on the page).
- Removed 384 2 "Indirect Address Read" (not on the page).
- Removed 698 1 "Indirect Data Read" (not on the page).
- Diff before: 22 differences; only in table (14): 156 1 "Error Code History 3"; 157 1 "Error Code History 4"; 158 1 "Error Code History 5"; 159 1 "Error Code History 6"; 160 1 "Error Code History 7"; 161 1 "Error Code History 8"; 162 1 "Error Code History 9"; 163 1 "Error Code History 10"; 164 1 "Error Code History 11"; 165 1 "Error Code History 12"; 166 1 "Error Code History 13"; 167 1 "Error Code History 14"; 384 2 "Indirect Address Read"; 698 1 "Indirect Data Read" | only on page (6): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (2): 256 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1" | different access/area (0)
- Diff after: 18 differences; only in table (12): 156 1 "Error Code History 3"; 157 1 "Error Code History 4"; 158 1 "Error Code History 5"; 159 1 "Error Code History 6"; 160 1 "Error Code History 7"; 161 1 "Error Code History 8"; 162 1 "Error Code History 9"; 163 1 "Error Code History 10"; 164 1 "Error Code History 11"; 165 1 "Error Code History 12"; 166 1 "Error Code History 13"; 167 1 "Error Code History 14" | only on page (6): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (0) | different access/area (0)

### `ym070_210_r051`

Page https://emanual.robotis.com/docs/en/dxl/y/ym070-210-r051-rh/, fetched 2026-10-29.

- Alias names listed on the page: none.
- Removed 256 2 "Indirect Address Write" (not on the page).
- Removed 634 1 "Indirect Data Write" (not on the page).
- Removed 384 2 "Indirect Address Read" (not on the page).
- Removed 698 1 "Indirect Data Read" (not on the page).
- Diff before: 22 differences; only in table (14): 156 1 "Error Code History 3"; 157 1 "Error Code History 4"; 158 1 "Error Code History 5"; 159 1 "Error Code History 6"; 160 1 "Error Code History 7"; 161 1 "Error Code History 8"; 162 1 "Error Code History 9"; 163 1 "Error Code History 10"; 164 1 "Error Code History 11"; 165 1 "Error Code History 12"; 166 1 "Error Code History 13"; 167 1 "Error Code History 14"; 384 2 "Indirect Address Read"; 698 1 "Indirect Data Read" | only on page (6): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (2): 256 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1" | different access/area (0)
- Diff after: 18 differences; only in table (12): 156 1 "Error Code History 3"; 157 1 "Error Code History 4"; 158 1 "Error Code History 5"; 159 1 "Error Code History 6"; 160 1 "Error Code History 7"; 161 1 "Error Code History 8"; 162 1 "Error Code History 9"; 163 1 "Error Code History 10"; 164 1 "Error Code History 11"; 165 1 "Error Code History 12"; 166 1 "Error Code History 13"; 167 1 "Error Code History 14" | only on page (6): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (0) | different access/area (0)

### `ym070_210_r099`

Page https://emanual.robotis.com/docs/en/dxl/y/ym070-210-r099-rh/, fetched 2026-10-29.

- Alias names listed on the page: none.
- Removed 256 2 "Indirect Address Write" (not on the page).
- Removed 634 1 "Indirect Data Write" (not on the page).
- Removed 384 2 "Indirect Address Read" (not on the page).
- Removed 698 1 "Indirect Data Read" (not on the page).
- Diff before: 22 differences; only in table (14): 156 1 "Error Code History 3"; 157 1 "Error Code History 4"; 158 1 "Error Code History 5"; 159 1 "Error Code History 6"; 160 1 "Error Code History 7"; 161 1 "Error Code History 8"; 162 1 "Error Code History 9"; 163 1 "Error Code History 10"; 164 1 "Error Code History 11"; 165 1 "Error Code History 12"; 166 1 "Error Code History 13"; 167 1 "Error Code History 14"; 384 2 "Indirect Address Read"; 698 1 "Indirect Data Read" | only on page (6): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (2): 256 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1" | different access/area (0)
- Diff after: 18 differences; only in table (12): 156 1 "Error Code History 3"; 157 1 "Error Code History 4"; 158 1 "Error Code History 5"; 159 1 "Error Code History 6"; 160 1 "Error Code History 7"; 161 1 "Error Code History 8"; 162 1 "Error Code History 9"; 163 1 "Error Code History 10"; 164 1 "Error Code History 11"; 165 1 "Error Code History 12"; 166 1 "Error Code History 13"; 167 1 "Error Code History 14" | only on page (6): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (0) | different access/area (0)

### `ym080_230_a051`

Page https://emanual.robotis.com/docs/en/dxl/y/ym080-230-a051-rh/, fetched 2026-10-29.

- Alias names listed on the page: none.
- Removed 256 2 "Indirect Address Write" (not on the page).
- Removed 634 1 "Indirect Data Write" (not on the page).
- Removed 384 2 "Indirect Address Read" (not on the page).
- Removed 698 1 "Indirect Data Read" (not on the page).
- Diff before: 22 differences; only in table (14): 156 1 "Error Code History 3"; 157 1 "Error Code History 4"; 158 1 "Error Code History 5"; 159 1 "Error Code History 6"; 160 1 "Error Code History 7"; 161 1 "Error Code History 8"; 162 1 "Error Code History 9"; 163 1 "Error Code History 10"; 164 1 "Error Code History 11"; 165 1 "Error Code History 12"; 166 1 "Error Code History 13"; 167 1 "Error Code History 14"; 384 2 "Indirect Address Read"; 698 1 "Indirect Data Read" | only on page (6): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (2): 256 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1" | different access/area (0)
- Diff after: 18 differences; only in table (12): 156 1 "Error Code History 3"; 157 1 "Error Code History 4"; 158 1 "Error Code History 5"; 159 1 "Error Code History 6"; 160 1 "Error Code History 7"; 161 1 "Error Code History 8"; 162 1 "Error Code History 9"; 163 1 "Error Code History 10"; 164 1 "Error Code History 11"; 165 1 "Error Code History 12"; 166 1 "Error Code History 13"; 167 1 "Error Code History 14" | only on page (6): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (0) | different access/area (0)

### `ym080_230_a099`

Page https://emanual.robotis.com/docs/en/dxl/y/ym080-230-a099-rh/, fetched 2026-10-29.

- Alias names listed on the page: none.
- Removed 256 2 "Indirect Address Write" (not on the page).
- Removed 634 1 "Indirect Data Write" (not on the page).
- Removed 384 2 "Indirect Address Read" (not on the page).
- Removed 698 1 "Indirect Data Read" (not on the page).
- Diff before: 22 differences; only in table (14): 156 1 "Error Code History 3"; 157 1 "Error Code History 4"; 158 1 "Error Code History 5"; 159 1 "Error Code History 6"; 160 1 "Error Code History 7"; 161 1 "Error Code History 8"; 162 1 "Error Code History 9"; 163 1 "Error Code History 10"; 164 1 "Error Code History 11"; 165 1 "Error Code History 12"; 166 1 "Error Code History 13"; 167 1 "Error Code History 14"; 384 2 "Indirect Address Read"; 698 1 "Indirect Data Read" | only on page (6): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (2): 256 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1" | different access/area (0)
- Diff after: 18 differences; only in table (12): 156 1 "Error Code History 3"; 157 1 "Error Code History 4"; 158 1 "Error Code History 5"; 159 1 "Error Code History 6"; 160 1 "Error Code History 7"; 161 1 "Error Code History 8"; 162 1 "Error Code History 9"; 163 1 "Error Code History 10"; 164 1 "Error Code History 11"; 165 1 "Error Code History 12"; 166 1 "Error Code History 13"; 167 1 "Error Code History 14" | only on page (6): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (0) | different access/area (0)

### `ym080_230_b001`

Page https://emanual.robotis.com/docs/en/dxl/y/ym080-230-b001-rh/, fetched 2026-10-29.

- Alias names listed on the page: none.
- Removed 256 2 "Indirect Address Write" (not on the page).
- Removed 634 1 "Indirect Data Write" (not on the page).
- Removed 384 2 "Indirect Address Read" (not on the page).
- Removed 698 1 "Indirect Data Read" (not on the page).
- Diff before: 22 differences; only in table (14): 156 1 "Error Code History 3"; 157 1 "Error Code History 4"; 158 1 "Error Code History 5"; 159 1 "Error Code History 6"; 160 1 "Error Code History 7"; 161 1 "Error Code History 8"; 162 1 "Error Code History 9"; 163 1 "Error Code History 10"; 164 1 "Error Code History 11"; 165 1 "Error Code History 12"; 166 1 "Error Code History 13"; 167 1 "Error Code History 14"; 384 2 "Indirect Address Read"; 698 1 "Indirect Data Read" | only on page (6): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (2): 256 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1" | different access/area (0)
- Diff after: 18 differences; only in table (12): 156 1 "Error Code History 3"; 157 1 "Error Code History 4"; 158 1 "Error Code History 5"; 159 1 "Error Code History 6"; 160 1 "Error Code History 7"; 161 1 "Error Code History 8"; 162 1 "Error Code History 9"; 163 1 "Error Code History 10"; 164 1 "Error Code History 11"; 165 1 "Error Code History 12"; 166 1 "Error Code History 13"; 167 1 "Error Code History 14" | only on page (6): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (0) | different access/area (0)

### `ym080_230_m001`

Page https://emanual.robotis.com/docs/en/dxl/y/ym080-230-m001-rh/, fetched 2026-10-29.

- Alias names listed on the page: none.
- Removed 256 2 "Indirect Address Write" (not on the page).
- Removed 634 1 "Indirect Data Write" (not on the page).
- Removed 384 2 "Indirect Address Read" (not on the page).
- Removed 698 1 "Indirect Data Read" (not on the page).
- Diff before: 22 differences; only in table (14): 156 1 "Error Code History 3"; 157 1 "Error Code History 4"; 158 1 "Error Code History 5"; 159 1 "Error Code History 6"; 160 1 "Error Code History 7"; 161 1 "Error Code History 8"; 162 1 "Error Code History 9"; 163 1 "Error Code History 10"; 164 1 "Error Code History 11"; 165 1 "Error Code History 12"; 166 1 "Error Code History 13"; 167 1 "Error Code History 14"; 384 2 "Indirect Address Read"; 698 1 "Indirect Data Read" | only on page (6): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (2): 256 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1" | different access/area (0)
- Diff after: 18 differences; only in table (12): 156 1 "Error Code History 3"; 157 1 "Error Code History 4"; 158 1 "Error Code History 5"; 159 1 "Error Code History 6"; 160 1 "Error Code History 7"; 161 1 "Error Code History 8"; 162 1 "Error Code History 9"; 163 1 "Error Code History 10"; 164 1 "Error Code History 11"; 165 1 "Error Code History 12"; 166 1 "Error Code History 13"; 167 1 "Error Code History 14" | only on page (6): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (0) | different access/area (0)

### `ym080_230_r051`

Page https://emanual.robotis.com/docs/en/dxl/y/ym080-230-r051-rh/, fetched 2026-10-29.

- Alias names listed on the page: none.
- Removed 256 2 "Indirect Address Write" (not on the page).
- Removed 634 1 "Indirect Data Write" (not on the page).
- Removed 384 2 "Indirect Address Read" (not on the page).
- Removed 698 1 "Indirect Data Read" (not on the page).
- Diff before: 22 differences; only in table (14): 156 1 "Error Code History 3"; 157 1 "Error Code History 4"; 158 1 "Error Code History 5"; 159 1 "Error Code History 6"; 160 1 "Error Code History 7"; 161 1 "Error Code History 8"; 162 1 "Error Code History 9"; 163 1 "Error Code History 10"; 164 1 "Error Code History 11"; 165 1 "Error Code History 12"; 166 1 "Error Code History 13"; 167 1 "Error Code History 14"; 384 2 "Indirect Address Read"; 698 1 "Indirect Data Read" | only on page (6): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (2): 256 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1" | different access/area (0)
- Diff after: 18 differences; only in table (12): 156 1 "Error Code History 3"; 157 1 "Error Code History 4"; 158 1 "Error Code History 5"; 159 1 "Error Code History 6"; 160 1 "Error Code History 7"; 161 1 "Error Code History 8"; 162 1 "Error Code History 9"; 163 1 "Error Code History 10"; 164 1 "Error Code History 11"; 165 1 "Error Code History 12"; 166 1 "Error Code History 13"; 167 1 "Error Code History 14" | only on page (6): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (0) | different access/area (0)

### `ym080_230_r099`

Page https://emanual.robotis.com/docs/en/dxl/y/ym080-230-r099-rh/, fetched 2026-10-29.

- Alias names listed on the page: none.
- Removed 256 2 "Indirect Address Write" (not on the page).
- Removed 634 1 "Indirect Data Write" (not on the page).
- Removed 384 2 "Indirect Address Read" (not on the page).
- Removed 698 1 "Indirect Data Read" (not on the page).
- Diff before: 22 differences; only in table (14): 156 1 "Error Code History 3"; 157 1 "Error Code History 4"; 158 1 "Error Code History 5"; 159 1 "Error Code History 6"; 160 1 "Error Code History 7"; 161 1 "Error Code History 8"; 162 1 "Error Code History 9"; 163 1 "Error Code History 10"; 164 1 "Error Code History 11"; 165 1 "Error Code History 12"; 166 1 "Error Code History 13"; 167 1 "Error Code History 14"; 384 2 "Indirect Address Read"; 698 1 "Indirect Data Read" | only on page (6): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (2): 256 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1" | different access/area (0)
- Diff after: 18 differences; only in table (12): 156 1 "Error Code History 3"; 157 1 "Error Code History 4"; 158 1 "Error Code History 5"; 159 1 "Error Code History 6"; 160 1 "Error Code History 7"; 161 1 "Error Code History 8"; 162 1 "Error Code History 9"; 163 1 "Error Code History 10"; 164 1 "Error Code History 11"; 165 1 "Error Code History 12"; 166 1 "Error Code History 13"; 167 1 "Error Code History 14" | only on page (6): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM | different name (0) | different access/area (0)

