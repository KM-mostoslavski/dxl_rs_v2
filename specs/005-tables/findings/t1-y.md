# T1 Y group (T034), completed 2026-10-29

First run (2026-10-28) stopped: the pages define a third area `Hybrid` and give the area in an `Area` column under "Control Table Configuration".

**Answer 2 (architect, provisional, 2026-10-29):** `area` gains `Hybrid`, read from the page's Area column. `emanual.py` reads an `Area` column when a table has one (EEPROM, RAM or Hybrid; anything else stops it); `check_tables.py` accepts `Hybrid` (tests added; role rows keep their own required areas). Page: "The Hybrid area retains its values even when power is turned off, and changes can be made regardless of the Torque Enable value." Hybrid rows: Velocity I/P/FF Gain (212, 216, 220), Position D/I/P/FF Gain (224–236), Profile Acceleration (240), Profile Velocity (244), Profile Acceleration Time (248), Profile Time (252), size 4, RW. The three Modbus-only rows with address `N/A` (access `W`) are skipped.

**Answer 4 extended (flag for the architect):** rows inside an elided "…" block take the block's access/area: "Error Code History 3" … "14" (156–167, size 1; the page lists 1, 2, …, 15, 16, all R RAM) and the alias rows "Indirect Address Read" (384, 2) and "Indirect Data Read" (698, 1).

Group results:
- `protocols: [P2]`: Protocol Type values 2 = DYNAMIXEL Protocol 2.0, 10(TBD) = Modbus-RTU ("will be available for DYNAMIXEL-Y servos in the future").
- **`type_info` fixed on 8 geared tables** (page difference): specification row Resolution 26,738,688 [pulse/rev] (A051, R051) and 51,904,512 [pulse/rev] (A099, R099), i.e. 524,288 × 51 / × 99; the tables had ±262,144 = ±π. Now min/max ±13,369,344 (A051, R051) and ±25,952,256 (A099, R099), zero 0, ±3.14159265 unchanged. B001/M001 pages give 524,288 [pulse/rev]: unchanged. Note: every Y page's Goal Position says "1[pulse] (Approx. 0.006866 deg)", which fits none of the resolution rows (524,288 pulse/rev is 0.000687°); the model-specific Resolution row was used. Flag for the architect.
- Mode rows: `Operating Mode` (33, 1, RW, EEPROM) on every page, as the table. No mode-row finding.
- Row findings for T2: alias rows "Indirect Address Write" (256, 2) and "Indirect Data Write" (634, 1) from Indirect Address/Data 1 (RW EEPROM / RW RAM).

### `ym070_210_a051`

Page https://emanual.robotis.com/docs/en/dxl/y/ym070-210-a051-rh/, fetched 2026-10-29.

- `fill`: 82 rows (exact address, size, name).
- By hand, page row at the same address and size: 256 2 "Indirect Address Write" → RW EEPROM (page: Indirect Address 1).
- By hand, page row at the same address and size: 634 1 "Indirect Data Write" → RW RAM (page: Indirect Data 1).
- Elided block (answer 4): 156 1 "Error Code History 3" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 157 1 "Error Code History 4" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 158 1 "Error Code History 5" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 159 1 "Error Code History 6" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 160 1 "Error Code History 7" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 161 1 "Error Code History 8" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 162 1 "Error Code History 9" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 163 1 "Error Code History 10" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 164 1 "Error Code History 11" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 165 1 "Error Code History 12" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 166 1 "Error Code History 13" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 167 1 "Error Code History 14" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 384 2 "Indirect Address Read" → RW EEPROM (between "Indirect Address 2" (258) and "Indirect Address 127" (508), both RW EEPROM).
- Elided block (answer 4): 698 1 "Indirect Data Read" → RW RAM (between "Indirect Data 2" (635) and "Indirect Data 127" (760), both RW RAM).
- Only on the page (not added in T1): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM.
- Name differences at the same address and size: 256 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1".

### `ym070_210_a099`

Page https://emanual.robotis.com/docs/en/dxl/y/ym070-210-a099-rh/, fetched 2026-10-29.

- `fill`: 82 rows (exact address, size, name).
- By hand, page row at the same address and size: 256 2 "Indirect Address Write" → RW EEPROM (page: Indirect Address 1).
- By hand, page row at the same address and size: 634 1 "Indirect Data Write" → RW RAM (page: Indirect Data 1).
- Elided block (answer 4): 156 1 "Error Code History 3" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 157 1 "Error Code History 4" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 158 1 "Error Code History 5" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 159 1 "Error Code History 6" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 160 1 "Error Code History 7" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 161 1 "Error Code History 8" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 162 1 "Error Code History 9" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 163 1 "Error Code History 10" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 164 1 "Error Code History 11" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 165 1 "Error Code History 12" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 166 1 "Error Code History 13" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 167 1 "Error Code History 14" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 384 2 "Indirect Address Read" → RW EEPROM (between "Indirect Address 2" (258) and "Indirect Address 127" (508), both RW EEPROM).
- Elided block (answer 4): 698 1 "Indirect Data Read" → RW RAM (between "Indirect Data 2" (635) and "Indirect Data 127" (760), both RW RAM).
- Only on the page (not added in T1): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM.
- Name differences at the same address and size: 256 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1".

### `ym070_210_b001`

Page https://emanual.robotis.com/docs/en/dxl/y/ym070-210-b001-rh/, fetched 2026-10-29.

- `fill`: 82 rows (exact address, size, name).
- By hand, page row at the same address and size: 256 2 "Indirect Address Write" → RW EEPROM (page: Indirect Address 1).
- By hand, page row at the same address and size: 634 1 "Indirect Data Write" → RW RAM (page: Indirect Data 1).
- Elided block (answer 4): 156 1 "Error Code History 3" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 157 1 "Error Code History 4" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 158 1 "Error Code History 5" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 159 1 "Error Code History 6" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 160 1 "Error Code History 7" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 161 1 "Error Code History 8" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 162 1 "Error Code History 9" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 163 1 "Error Code History 10" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 164 1 "Error Code History 11" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 165 1 "Error Code History 12" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 166 1 "Error Code History 13" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 167 1 "Error Code History 14" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 384 2 "Indirect Address Read" → RW EEPROM (between "Indirect Address 2" (258) and "Indirect Address 127" (508), both RW EEPROM).
- Elided block (answer 4): 698 1 "Indirect Data Read" → RW RAM (between "Indirect Data 2" (635) and "Indirect Data 127" (760), both RW RAM).
- Only on the page (not added in T1): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM.
- Name differences at the same address and size: 256 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1".

### `ym070_210_m001`

Page https://emanual.robotis.com/docs/en/dxl/y/ym070-210-m001-rh/, fetched 2026-10-29.

- `fill`: 82 rows (exact address, size, name).
- By hand, page row at the same address and size: 256 2 "Indirect Address Write" → RW EEPROM (page: Indirect Address 1).
- By hand, page row at the same address and size: 634 1 "Indirect Data Write" → RW RAM (page: Indirect Data 1).
- Elided block (answer 4): 156 1 "Error Code History 3" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 157 1 "Error Code History 4" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 158 1 "Error Code History 5" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 159 1 "Error Code History 6" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 160 1 "Error Code History 7" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 161 1 "Error Code History 8" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 162 1 "Error Code History 9" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 163 1 "Error Code History 10" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 164 1 "Error Code History 11" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 165 1 "Error Code History 12" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 166 1 "Error Code History 13" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 167 1 "Error Code History 14" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 384 2 "Indirect Address Read" → RW EEPROM (between "Indirect Address 2" (258) and "Indirect Address 127" (508), both RW EEPROM).
- Elided block (answer 4): 698 1 "Indirect Data Read" → RW RAM (between "Indirect Data 2" (635) and "Indirect Data 127" (760), both RW RAM).
- Only on the page (not added in T1): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM.
- Name differences at the same address and size: 256 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1".

### `ym070_210_r051`

Page https://emanual.robotis.com/docs/en/dxl/y/ym070-210-r051-rh/, fetched 2026-10-29.

- `fill`: 82 rows (exact address, size, name).
- By hand, page row at the same address and size: 256 2 "Indirect Address Write" → RW EEPROM (page: Indirect Address 1).
- By hand, page row at the same address and size: 634 1 "Indirect Data Write" → RW RAM (page: Indirect Data 1).
- Elided block (answer 4): 156 1 "Error Code History 3" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 157 1 "Error Code History 4" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 158 1 "Error Code History 5" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 159 1 "Error Code History 6" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 160 1 "Error Code History 7" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 161 1 "Error Code History 8" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 162 1 "Error Code History 9" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 163 1 "Error Code History 10" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 164 1 "Error Code History 11" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 165 1 "Error Code History 12" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 166 1 "Error Code History 13" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 167 1 "Error Code History 14" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 384 2 "Indirect Address Read" → RW EEPROM (between "Indirect Address 2" (258) and "Indirect Address 127" (508), both RW EEPROM).
- Elided block (answer 4): 698 1 "Indirect Data Read" → RW RAM (between "Indirect Data 2" (635) and "Indirect Data 127" (760), both RW RAM).
- Only on the page (not added in T1): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM.
- Name differences at the same address and size: 256 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1".

### `ym070_210_r099`

Page https://emanual.robotis.com/docs/en/dxl/y/ym070-210-r099-rh/, fetched 2026-10-29.

- `fill`: 82 rows (exact address, size, name).
- By hand, page row at the same address and size: 256 2 "Indirect Address Write" → RW EEPROM (page: Indirect Address 1).
- By hand, page row at the same address and size: 634 1 "Indirect Data Write" → RW RAM (page: Indirect Data 1).
- Elided block (answer 4): 156 1 "Error Code History 3" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 157 1 "Error Code History 4" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 158 1 "Error Code History 5" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 159 1 "Error Code History 6" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 160 1 "Error Code History 7" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 161 1 "Error Code History 8" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 162 1 "Error Code History 9" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 163 1 "Error Code History 10" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 164 1 "Error Code History 11" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 165 1 "Error Code History 12" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 166 1 "Error Code History 13" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 167 1 "Error Code History 14" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 384 2 "Indirect Address Read" → RW EEPROM (between "Indirect Address 2" (258) and "Indirect Address 127" (508), both RW EEPROM).
- Elided block (answer 4): 698 1 "Indirect Data Read" → RW RAM (between "Indirect Data 2" (635) and "Indirect Data 127" (760), both RW RAM).
- Only on the page (not added in T1): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM.
- Name differences at the same address and size: 256 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1".

### `ym080_230_a051`

Page https://emanual.robotis.com/docs/en/dxl/y/ym080-230-a051-rh/, fetched 2026-10-29.

- `fill`: 82 rows (exact address, size, name).
- By hand, page row at the same address and size: 256 2 "Indirect Address Write" → RW EEPROM (page: Indirect Address 1).
- By hand, page row at the same address and size: 634 1 "Indirect Data Write" → RW RAM (page: Indirect Data 1).
- Elided block (answer 4): 156 1 "Error Code History 3" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 157 1 "Error Code History 4" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 158 1 "Error Code History 5" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 159 1 "Error Code History 6" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 160 1 "Error Code History 7" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 161 1 "Error Code History 8" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 162 1 "Error Code History 9" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 163 1 "Error Code History 10" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 164 1 "Error Code History 11" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 165 1 "Error Code History 12" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 166 1 "Error Code History 13" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 167 1 "Error Code History 14" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 384 2 "Indirect Address Read" → RW EEPROM (between "Indirect Address 2" (258) and "Indirect Address 127" (508), both RW EEPROM).
- Elided block (answer 4): 698 1 "Indirect Data Read" → RW RAM (between "Indirect Data 2" (635) and "Indirect Data 127" (760), both RW RAM).
- Only on the page (not added in T1): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM.
- Name differences at the same address and size: 256 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1".

### `ym080_230_a099`

Page https://emanual.robotis.com/docs/en/dxl/y/ym080-230-a099-rh/, fetched 2026-10-29.

- `fill`: 82 rows (exact address, size, name).
- By hand, page row at the same address and size: 256 2 "Indirect Address Write" → RW EEPROM (page: Indirect Address 1).
- By hand, page row at the same address and size: 634 1 "Indirect Data Write" → RW RAM (page: Indirect Data 1).
- Elided block (answer 4): 156 1 "Error Code History 3" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 157 1 "Error Code History 4" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 158 1 "Error Code History 5" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 159 1 "Error Code History 6" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 160 1 "Error Code History 7" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 161 1 "Error Code History 8" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 162 1 "Error Code History 9" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 163 1 "Error Code History 10" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 164 1 "Error Code History 11" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 165 1 "Error Code History 12" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 166 1 "Error Code History 13" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 167 1 "Error Code History 14" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 384 2 "Indirect Address Read" → RW EEPROM (between "Indirect Address 2" (258) and "Indirect Address 127" (508), both RW EEPROM).
- Elided block (answer 4): 698 1 "Indirect Data Read" → RW RAM (between "Indirect Data 2" (635) and "Indirect Data 127" (760), both RW RAM).
- Only on the page (not added in T1): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM.
- Name differences at the same address and size: 256 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1".

### `ym080_230_b001`

Page https://emanual.robotis.com/docs/en/dxl/y/ym080-230-b001-rh/, fetched 2026-10-29.

- `fill`: 82 rows (exact address, size, name).
- By hand, page row at the same address and size: 256 2 "Indirect Address Write" → RW EEPROM (page: Indirect Address 1).
- By hand, page row at the same address and size: 634 1 "Indirect Data Write" → RW RAM (page: Indirect Data 1).
- Elided block (answer 4): 156 1 "Error Code History 3" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 157 1 "Error Code History 4" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 158 1 "Error Code History 5" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 159 1 "Error Code History 6" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 160 1 "Error Code History 7" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 161 1 "Error Code History 8" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 162 1 "Error Code History 9" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 163 1 "Error Code History 10" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 164 1 "Error Code History 11" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 165 1 "Error Code History 12" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 166 1 "Error Code History 13" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 167 1 "Error Code History 14" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 384 2 "Indirect Address Read" → RW EEPROM (between "Indirect Address 2" (258) and "Indirect Address 127" (508), both RW EEPROM).
- Elided block (answer 4): 698 1 "Indirect Data Read" → RW RAM (between "Indirect Data 2" (635) and "Indirect Data 127" (760), both RW RAM).
- Only on the page (not added in T1): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM.
- Name differences at the same address and size: 256 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1".

### `ym080_230_m001`

Page https://emanual.robotis.com/docs/en/dxl/y/ym080-230-m001-rh/, fetched 2026-10-29.

- `fill`: 82 rows (exact address, size, name).
- By hand, page row at the same address and size: 256 2 "Indirect Address Write" → RW EEPROM (page: Indirect Address 1).
- By hand, page row at the same address and size: 634 1 "Indirect Data Write" → RW RAM (page: Indirect Data 1).
- Elided block (answer 4): 156 1 "Error Code History 3" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 157 1 "Error Code History 4" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 158 1 "Error Code History 5" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 159 1 "Error Code History 6" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 160 1 "Error Code History 7" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 161 1 "Error Code History 8" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 162 1 "Error Code History 9" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 163 1 "Error Code History 10" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 164 1 "Error Code History 11" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 165 1 "Error Code History 12" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 166 1 "Error Code History 13" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 167 1 "Error Code History 14" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 384 2 "Indirect Address Read" → RW EEPROM (between "Indirect Address 2" (258) and "Indirect Address 127" (508), both RW EEPROM).
- Elided block (answer 4): 698 1 "Indirect Data Read" → RW RAM (between "Indirect Data 2" (635) and "Indirect Data 127" (760), both RW RAM).
- Only on the page (not added in T1): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM.
- Name differences at the same address and size: 256 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1".

### `ym080_230_r051`

Page https://emanual.robotis.com/docs/en/dxl/y/ym080-230-r051-rh/, fetched 2026-10-29.

- `fill`: 82 rows (exact address, size, name).
- By hand, page row at the same address and size: 256 2 "Indirect Address Write" → RW EEPROM (page: Indirect Address 1).
- By hand, page row at the same address and size: 634 1 "Indirect Data Write" → RW RAM (page: Indirect Data 1).
- Elided block (answer 4): 156 1 "Error Code History 3" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 157 1 "Error Code History 4" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 158 1 "Error Code History 5" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 159 1 "Error Code History 6" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 160 1 "Error Code History 7" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 161 1 "Error Code History 8" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 162 1 "Error Code History 9" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 163 1 "Error Code History 10" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 164 1 "Error Code History 11" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 165 1 "Error Code History 12" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 166 1 "Error Code History 13" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 167 1 "Error Code History 14" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 384 2 "Indirect Address Read" → RW EEPROM (between "Indirect Address 2" (258) and "Indirect Address 127" (508), both RW EEPROM).
- Elided block (answer 4): 698 1 "Indirect Data Read" → RW RAM (between "Indirect Data 2" (635) and "Indirect Data 127" (760), both RW RAM).
- Only on the page (not added in T1): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM.
- Name differences at the same address and size: 256 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1".

### `ym080_230_r099`

Page https://emanual.robotis.com/docs/en/dxl/y/ym080-230-r099-rh/, fetched 2026-10-29.

- `fill`: 82 rows (exact address, size, name).
- By hand, page row at the same address and size: 256 2 "Indirect Address Write" → RW EEPROM (page: Indirect Address 1).
- By hand, page row at the same address and size: 634 1 "Indirect Data Write" → RW RAM (page: Indirect Data 1).
- Elided block (answer 4): 156 1 "Error Code History 3" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 157 1 "Error Code History 4" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 158 1 "Error Code History 5" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 159 1 "Error Code History 6" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 160 1 "Error Code History 7" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 161 1 "Error Code History 8" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 162 1 "Error Code History 9" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 163 1 "Error Code History 10" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 164 1 "Error Code History 11" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 165 1 "Error Code History 12" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 166 1 "Error Code History 13" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 167 1 "Error Code History 14" → R RAM (between "Error Code History 2" (155) and "Error Code History 15" (168), both R RAM).
- Elided block (answer 4): 384 2 "Indirect Address Read" → RW EEPROM (between "Indirect Address 2" (258) and "Indirect Address 127" (508), both RW EEPROM).
- Elided block (answer 4): 698 1 "Indirect Data Read" → RW RAM (between "Indirect Data 2" (635) and "Indirect Data 127" (760), both RW RAM).
- Only on the page (not added in T1): 258 2 "Indirect Address 2" RW EEPROM; 508 2 "Indirect Address 127" RW EEPROM; 510 2 "Indirect Address 128" RW EEPROM; 635 1 "Indirect Data 2" RW RAM; 760 1 "Indirect Data 127" RW RAM; 761 1 "Indirect Data 128" RW RAM.
- Name differences at the same address and size: 256 2 table "Indirect Address Write", page "Indirect Address 1"; 634 1 table "Indirect Data Write", page "Indirect Data 1".

