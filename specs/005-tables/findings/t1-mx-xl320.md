# T1 MX(2.0) + XL-320 group (T029): `mx_28`, `mx_64`, `mx_106`, `xl320`

Pages: `mx_28` → https://emanual.robotis.com/docs/en/dxl/mx/mx-28-2/, `mx_64` → …/mx-64-2/, `mx_106` → …/mx-106-2/, `xl320` → https://emanual.robotis.com/docs/en/dxl/x/xl320/. All fetched 2026-10-28.

- MX(2.0): `protocols: [P1, P2]` (Protocol Type(13) values 1 = DYNAMIXEL Protocol 1.0, 2 = DYNAMIXEL Protocol 2.0). `type_info` from the page: Goal Position "0.088 [deg/pulse]", "1[rev] : 0 ~ 4,095 (1 rotation (0 ~ 4,095, total 4,096 counts))"; the Goal Position figure (`/assets/images/dxl/x/dxl_goal_position.jpg`) labels "180° [Goal Position = 2048(0x800)]". Slope 2π/4096, zero 2048: `min_radian` −2048 ticks = `-3.1415927`, `max_radian` +2047 ticks = `3.1400587` (research R-8). Before: ±3.14159265 (slope 2π/4095).
- MX(2.0): Mode row (FR-020): the page lists `Operating Mode` at address 11, size 1, RW, EEPROM, the same row as the table (filled by `fill`); no `Control Mode` and no CW/CCW Angle Limit rows on the page or in the table. No mode-row finding.
- XL-320: `protocols: [P2]`. No Protocol Type register; the page refers only to Protocol 2.0 ("Please refer to DYNAMIXEL Protocol 2.0 for more details about Instruction Packets", "refer to the Status Packet section for DYNAMIXEL Protocol 2.0"); its specification row "Protocol Type" is the serial link ("TTL Half Duplex Asynchronous Serial Communication with 8bit, 1stop, No Parity").
- XL-320 `type_info` (research R-8 finding confirmed): the page gives Goal Position "The available position range is 0 to 1,023 (0x3FF) and the per unit value is 0.29 °", Operating Modes "Joint Mode (0 ~ 300 [°])", and the figure `/assets/images/dxl/dx/dx_series_goal_position.png` (512 → 150°). Changed `2048 / 0 / 4095, ±3.14159265` → `512 / 0 / 1023`, `min_radian: -2.6205530`, `max_radian: 2.6154347`.
- XL-320 mode row: the page lists `Control Mode` at address 11, size 1, RW, EEPROM, as the table; it also lists the CW/CCW Angle Limit pair (6, 8; RW, EEPROM), the table's limits. No `Operating Mode`. No mode-row finding.
- Alias rows (research R-14), not on the pages, filled by hand from the page row at the same address and size; listed per table below. Not removed (T2).

### `mx_28`

Page https://emanual.robotis.com/docs/en/dxl/mx/mx-28-2/, fetched 2026-10-28.

- `fill`: 52 rows filled from the page (address, size and name match).
- Filled by hand from the page row at the same address and size (4; alias rows, not on the page, research R-14; removal is T2):
  - 168 2 "Indirect Address Write" → RW RAM (page: Indirect Address 1)
  - 224 1 "Indirect Data Write" → RW RAM (page: Indirect Data 1)
  - 578 2 "Indirect Address Read" → RW RAM (page: Indirect Address 29)
  - 634 1 "Indirect Data Read" → RW RAM (page: Indirect Data 29)
- Rows only on the page (20, not added in T1): 170 2 "Indirect Address 2" RW RAM; 172 2 "Indirect Address 3" RW RAM; 218 2 "Indirect Address 26" RW RAM; 220 2 "Indirect Address 27" RW RAM; 222 2 "Indirect Address 28" RW RAM; 225 1 "Indirect Data 2" RW RAM; 226 1 "Indirect Data 3" RW RAM; 249 1 "Indirect Data 26" RW RAM; 250 1 "Indirect Data 27" RW RAM; 251 1 "Indirect Data 28" RW RAM; 580 2 "Indirect Address 30" RW RAM; 582 2 "Indirect Address 31" RW RAM; 628 2 "Indirect Address 54" RW RAM; 630 2 "Indirect Address 55" RW RAM; 632 2 "Indirect Address 56" RW RAM; 635 1 "Indirect Data 30" RW RAM; 636 1 "Indirect Data 31" RW RAM; 659 1 "Indirect Data 54" RW RAM; 660 1 "Indirect Data 55" RW RAM; 661 1 "Indirect Data 56" RW RAM.
- Rows only in the table: none.
- Name differences at the same address and size: 168 2 table "Indirect Address Write", page "Indirect Address 1"; 224 1 table "Indirect Data Write", page "Indirect Data 1"; 578 2 table "Indirect Address Read", page "Indirect Address 29"; 634 1 table "Indirect Data Read", page "Indirect Data 29".
- `protocols`/`type_info` change: `[P1, P2] ['min_radian: -3.14159265 -> -3.1415927', 'max_radian: 3.14159265 -> 3.1400587']`.
- Protocol Type values on the page: 1 = DYNAMIXEL Protocol 1.0, 2 = DYNAMIXEL Protocol 2.0.

### `mx_64`

Page https://emanual.robotis.com/docs/en/dxl/mx/mx-64-2/, fetched 2026-10-28.

- `fill`: 54 rows filled from the page (address, size and name match).
- Filled by hand from the page row at the same address and size (4; alias rows, not on the page, research R-14; removal is T2):
  - 168 2 "Indirect Address Write" → RW RAM (page: Indirect Address 1)
  - 224 1 "Indirect Data Write" → RW RAM (page: Indirect Data 1)
  - 578 2 "Indirect Address Read" → RW RAM (page: Indirect Address 29)
  - 634 1 "Indirect Data Read" → RW RAM (page: Indirect Data 29)
- Rows only on the page (20, not added in T1): 170 2 "Indirect Address 2" RW RAM; 172 2 "Indirect Address 3" RW RAM; 218 2 "Indirect Address 26" RW RAM; 220 2 "Indirect Address 27" RW RAM; 222 2 "Indirect Address 28" RW RAM; 225 1 "Indirect Data 2" RW RAM; 226 1 "Indirect Data 3" RW RAM; 249 1 "Indirect Data 26" RW RAM; 250 1 "Indirect Data 27" RW RAM; 251 1 "Indirect Data 28" RW RAM; 580 2 "Indirect Address 30" RW RAM; 582 2 "Indirect Address 31" RW RAM; 628 2 "Indirect Address 54" RW RAM; 630 2 "Indirect Address 55" RW RAM; 632 2 "Indirect Address 56" RW RAM; 635 1 "Indirect Data 30" RW RAM; 636 1 "Indirect Data 31" RW RAM; 659 1 "Indirect Data 54" RW RAM; 660 1 "Indirect Data 55" RW RAM; 661 1 "Indirect Data 56" RW RAM.
- Rows only in the table: none.
- Name differences at the same address and size: 168 2 table "Indirect Address Write", page "Indirect Address 1"; 224 1 table "Indirect Data Write", page "Indirect Data 1"; 578 2 table "Indirect Address Read", page "Indirect Address 29"; 634 1 table "Indirect Data Read", page "Indirect Data 29".
- `protocols`/`type_info` change: `[P1, P2] ['min_radian: -3.14159265 -> -3.1415927', 'max_radian: 3.14159265 -> 3.1400587']`.
- Protocol Type values on the page: 1 = DYNAMIXEL Protocol 1.0, 2 = DYNAMIXEL Protocol 2.0.

### `mx_106`

Page https://emanual.robotis.com/docs/en/dxl/mx/mx-106-2/, fetched 2026-10-28.

- `fill`: 54 rows filled from the page (address, size and name match).
- Filled by hand from the page row at the same address and size (4; alias rows, not on the page, research R-14; removal is T2):
  - 168 2 "Indirect Address Write" → RW RAM (page: Indirect Address 1)
  - 224 1 "Indirect Data Write" → RW RAM (page: Indirect Data 1)
  - 578 2 "Indirect Address Read" → RW RAM (page: Indirect Address 29)
  - 634 1 "Indirect Data Read" → RW RAM (page: Indirect Data 29)
- Rows only on the page (20, not added in T1): 170 2 "Indirect Address 2" RW RAM; 172 2 "Indirect Address 3" RW RAM; 218 2 "Indirect Address 26" RW RAM; 220 2 "Indirect Address 27" RW RAM; 222 2 "Indirect Address 28" RW RAM; 225 1 "Indirect Data 2" RW RAM; 226 1 "Indirect Data 3" RW RAM; 249 1 "Indirect Data 26" RW RAM; 250 1 "Indirect Data 27" RW RAM; 251 1 "Indirect Data 28" RW RAM; 580 2 "Indirect Address 30" RW RAM; 582 2 "Indirect Address 31" RW RAM; 628 2 "Indirect Address 54" RW RAM; 630 2 "Indirect Address 55" RW RAM; 632 2 "Indirect Address 56" RW RAM; 635 1 "Indirect Data 30" RW RAM; 636 1 "Indirect Data 31" RW RAM; 659 1 "Indirect Data 54" RW RAM; 660 1 "Indirect Data 55" RW RAM; 661 1 "Indirect Data 56" RW RAM.
- Rows only in the table: none.
- Name differences at the same address and size: 168 2 table "Indirect Address Write", page "Indirect Address 1"; 224 1 table "Indirect Data Write", page "Indirect Data 1"; 578 2 table "Indirect Address Read", page "Indirect Address 29"; 634 1 table "Indirect Data Read", page "Indirect Data 29".
- `protocols`/`type_info` change: `[P1, P2] ['min_radian: -3.14159265 -> -3.1415927', 'max_radian: 3.14159265 -> 3.1400587']`.
- Protocol Type values on the page: 1 = DYNAMIXEL Protocol 1.0, 2 = DYNAMIXEL Protocol 2.0.

### `xl320`

Page https://emanual.robotis.com/docs/en/dxl/x/xl320/, fetched 2026-10-28.

- `fill`: 31 rows filled from the page (address, size and name match).
- Rows only on the page (0, not added in T1): none.
- Rows only in the table: none.
- Name differences at the same address and size: none.
- `protocols`/`type_info` change: `[P2] ['value_of_zero_radian_position: 2048 -> 512', 'value_of_max_radian_position: 4095 -> 1023', 'min_radian: -3.14159265 -> -2.6205530', 'max_radian: 3.14159265 -> 2.6154347']`.
