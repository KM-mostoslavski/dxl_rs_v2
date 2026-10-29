# AX-12A (`ax_12a`): rows, model data, verification (T019–T021)

Page: https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/ (headed "AX-12+, AX-12A"), fetched
2026-10-28 (Python `urllib`). Protocol page: https://emanual.robotis.com/docs/en/dxl/protocol1/,
fetched 2026-10-28.

## T019: rows

- `python3 control_tables/emanual.py diff ax_12a`: 0 differences. The page lists 32 rows (EEPROM
  0–18, RAM 24–48); all 32 table rows match on address, size and name. No name differs, not even
  in case. Nothing for T2.
- `python3 control_tables/emanual.py fill ax_12a`: filled 32 rows, left 0. Every row now has
  `access` (`R` or `RW`) and `area` (`EEPROM` or `RAM`) from the page. Rows filled by hand: none.
  Rows not on the page: none. Alias rows: none.

## T020: model data

(a) `protocols: [P1]`. No Protocol Type register on the page (none of the 32 rows). The page
refers to "the Status Packet section for DYNAMIXEL Protocol 1.0" and notes that the XL430-W250
replacement offers "support for DYNAMIXEL Protocol 2.0" as an additional feature; the Protocol 1.0
page says "DYNAMIXELs using Protocol 1.0 such as AX-12A".

(b) `type_info`: page: "The available position range is 0 to 1,023 (0x3FF) and the per unit value
is 0.29 °", specification "Running Degree 0 ~ 300 [°]", and the Goal Position figure
(`/assets/images/dxl/dx/dx_series_goal_position.png`) labels 0 → 0°, 512 (0x200) → 150°,
1023 (0x3FF) → 300°. Slope 300°/1023 per tick (research R-8 formula
`angle = (raw − zero) × rad_per_tick`), zero 512.
Changed: `min_radian: 0.0` → `-2.6205530` (−512 ticks), `max_radian: 5.2359877` → `2.6154347`
(+511 ticks). Zero / min / max `512 / 0 / 1023` unchanged. Same slope as before (5.2359877 / 1023),
only the angles move to the zero.

(c) `unit_info`:
- `Position 0.29 deg unsigned`: kept. Page: "per unit value is 0.29 °", range 0 ~ 1,023.
- `Speed 0.111 rpm signed`: kept. Page (Moving Speed / Present Speed, Joint Mode): "the unit is
  about 0.111rpm"; Present Speed 0 ~ 2,047 with "the 10th bit becomes the direction bit"
  (CCW 0 ~ 1,023, CW 1,024 ~ 2,047), so the value carries a direction. Reviewer note: the page
  states a direction bit (sign and magnitude), not a two's-complement value; `sign_type` is not
  read by codegen.
- `Torque 0.001465 N·m/bit unsigned`: **removed** (FR-003). The page gives torque only as "about
  0.1%" of the maximum (Max Torque, Torque Limit, Present Load), never in N·m per bit.

(d) `instructions: [(protocol: P1, source: "https://emanual.robotis.com/docs/en/dxl/protocol1/",
list: [Ping, Read, Write, RegWrite, Action, FactoryReset, SyncWrite])]`. The Protocol 1.0 page's
"Instruction Details" sections Ping, Read, Write, Reg Write, Action, Factory Reset and Sync Write
carry no product condition. Two sections do, and neither includes AX-12A:
- Reboot: "Supported products : DYNAMIXEL MX-12W(V41), MX-28/64/106(V40), MX(2.0) and X Series
  (excluding XL-320)".
- Bulk Read: "Supported products : DYNAMIXEL MX, MX(2.0) and X Series (excluding XL-320)".
Matches research R-5.

(e) Mode source: the page lists no `Operating Mode` or `Control Mode` register (32 rows, none of
either name). It lists `CW Angle Limit` (6, 2) and `CCW Angle Limit` (8, 2), and its "CW/CCW
Angle Limit(6, 8)" section says "The following two modes can be set pursuant to the value of CW
and CCW": Wheel Mode "both are 0", Joint Mode "neither are 0". So the table carries neither mode
row and no `operating_modes`; the mode comes from the CW/CCW pair. Nothing to raise.

(f) `source: Verified(url: "https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/", verified:
"2026-10-28")`.

## T021: validation

- `python3 control_tables/check_tables.py`: exit 0; `ax_12a  verified
  https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/ 2026-10-28  [P1]`.
- `python3 control_tables/emanual.py diff ax_12a`: 0 differences, exit 0.
- `cargo xtask codegen && git diff --exit-code src/models.rs src/models/`: empty.
- Audit, 5 random rows (seed 20261028), compared with the raw page HTML (address, size, name,
  access, and the heading above the table):

  | table | page |
  |---|---|
  | 3 1 "ID" RW EEPROM | 3 1 "ID" RW, Control Table of EEPROM Area |
  | 18 1 "Shutdown" RW EEPROM | 18 1 "Shutdown" RW, Control Table of EEPROM Area |
  | 48 2 "Punch" RW RAM | 48 2 "Punch" RW, Control Table of RAM Area |
  | 13 1 "Max Voltage Limit" RW EEPROM | 13 1 "Max Voltage Limit" RW, Control Table of EEPROM Area |
  | 26 1 "CW Compliance Margin" RW RAM | 26 1 "CW Compliance Margin" RW, Control Table of RAM Area |

- `python3 control_tables/check_tables.py --markdown ax_12a` (before the map gets `p1` in T045):

  ```
  - `ax_12a`: verified https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/ 2026-10-28, [P1], 12 tables
  ```
