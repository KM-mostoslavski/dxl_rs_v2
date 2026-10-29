# AX-12+ (`ax_12_plus`): verified (T061)

Page: https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/, fetched 2026-10-29. The page is headed
"AX-12+, AX-12A"; "NOTE : AX-12A is a new version of the AX-12+ with the same performance but
more advanced external design"; weights "AX-12 (53.5 [g]), AX-12+ (53.5 [g]), AX-12A (54.6 [g])".
AX-12+ has no page of its own (research R-9; architect approval of the citation is T055).
Protocol page: https://emanual.robotis.com/docs/en/dxl/protocol1/, fetched 2026-10-29.

## Rows

`python3 control_tables/emanual.py diff ax_12_plus https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/`:
0 differences (before and after). Independent reader (regex over the raw `<tr>` cells): 32 table
rows, 32 page rows, 0 mismatches in address, size, name and access; areas from the "Control Table
of EEPROM Area" (0–18) and "Control Table of RAM Area" (24–48) headings. No row corrected or
removed. Rows (all on the page): 0 2 Model Number R EEPROM; 2 1 Firmware Version R EEPROM; 3 1
ID RW EEPROM; 4 1 Baud Rate RW EEPROM; 5 1 Return Delay Time RW EEPROM; 6 2 CW Angle Limit RW
EEPROM; 8 2 CCW Angle Limit RW EEPROM; 11 1 Temperature Limit RW EEPROM; 12 1 Min Voltage Limit
RW EEPROM; 13 1 Max Voltage Limit RW EEPROM; 14 2 Max Torque RW EEPROM; 16 1 Status Return Level
RW EEPROM; 17 1 Alarm LED RW EEPROM; 18 1 Shutdown RW EEPROM; 24 1 Torque Enable RW RAM; 25 1 LED
RW RAM; 26 1 CW Compliance Margin RW RAM; 27 1 CCW Compliance Margin RW RAM; 28 1 CW Compliance
Slope RW RAM; 29 1 CCW Compliance Slope RW RAM; 30 2 Goal Position RW RAM; 32 2 Moving Speed RW
RAM; 34 2 Torque Limit RW RAM; 36 2 Present Position R RAM; 38 2 Present Speed R RAM; 40 2
Present Load R RAM; 42 1 Present Voltage R RAM; 43 1 Present Temperature R RAM; 44 1 Registered R
RAM; 46 1 Moving R RAM; 47 1 Lock RW RAM; 48 2 Punch RW RAM.

## Model data, each checked on the page

- Model Number row Initial Value 12 (stays `not_mapped: SameNumberAs(12)`; `also: ["AX-12+"]`
  on 12 in T078).
- `protocols: [P1]`: no Protocol Type register; "Please refer to DYNAMIXEL Protocol 1.0 for more
  details about Instruction Packets".
- Mode source (FR-020): no `Operating Mode` or `Control Mode` register on the page; CW/CCW Angle
  Limit (6, 8) present ("Wheel Mode: both are 0; Joint Mode: neither are 0"); no
  `operating_modes`. Unchanged.
- `type_info` `512 / 0 / 1023`, `-2.6205530` / `2.6154347`, unchanged: Goal Position "The
  available position range is 0 to 1,023 (0x3FF) and the per unit value is 0.29 °", Running
  Degree "0 ~ 300 [°]", figure `/assets/images/dxl/dx/dx_series_goal_position.png` (the one T1
  read for AX-12A: 512 → 150°).
- `unit_info`:
  - kept `Position 0.29 deg unsigned` (Goal/Present Position "0.29 °").
  - kept `Speed 0.111 rpm signed` (Moving Speed / Present Speed, Joint Mode "about 0.111rpm";
    Present Speed 10th bit is the direction bit).
  - **removed** `Torque 0.001465 N·m/bit unsigned`: the page gives no such value (Max Torque /
    Torque Limit / Present Load are in "about 0.1%"). Same removal T1 made on `ax_12a`.
- `instructions` added (Protocol 1.0 page): P1 `Ping, Read, Write, RegWrite, Action,
  FactoryReset, SyncWrite`. Not listed: Reboot ("Supported products : DYNAMIXEL MX-12W(V41),
  MX-28/64/106(V40), MX(2.0) and X Series (excluding XL-320)") and Bulk Read ("DYNAMIXEL MX, MX(2.0)
  and X Series (excluding XL-320)"): AX-12+ is in neither.
- `source: Verified(url: "https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/", verified: "2026-10-29")`.
  Its own record (US2 AS1), even though the layout equals `ax_12a`.

## SC-002 audit (T063, 2026-10-29)

5 rows drawn at random (Python `random.seed(20261029)`), each read on the page by eye: 46 1
"Moving" R RAM; 28 1 "CW Compliance Slope" RW RAM; 16 1 "Status Return Level" RW EEPROM; 43 1
"Present Temperature" R RAM; 6 2 "CW Angle Limit" RW EEPROM. 5 of 5 match.
