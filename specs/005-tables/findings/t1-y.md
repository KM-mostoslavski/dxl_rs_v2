# T1 Y group (T034): STOPPED, raised to the architect

Files: `ym070_210_a051`, `ym070_210_a099`, `ym070_210_b001`, `ym070_210_m001`, `ym070_210_r051`,
`ym070_210_r099`, `ym080_230_a051`, `ym080_230_a099`, `ym080_230_b001`, `ym080_230_m001`,
`ym080_230_r051`, `ym080_230_r099`. Pages from `PAGES` (`https://emanual.robotis.com/docs/en/dxl/y/<model>-rh/`),
all fetched 2026-10-28.

## Why the task stopped

1. **A third area, `Hybrid`.** The Y pages divide the control table into three areas (section "Area
   (EEPROM, RAM, Hybrid)", YM070-210-M001-RH): "The control table is divided into three areas:
   EEPROM, RAM, and Hybrid." … "The Hybrid area retains its values even when power is turned off,
   and changes can be made regardless of the Torque Enable value. When torque is disabled (Torque
   Enable = 0), it behaves like the EEPROM area. Values are immediately stored when changed. When
   torque is enabled (Torque Enable = 1), it behaves like the RAM area. Changes are possible but not
   immediately stored. To save changes made in this state, use Gain Save (170) when torque has been
   disabled." Eleven rows are `Hybrid` on YM070-210-M001-RH: Velocity I Gain (212), Velocity P Gain
   (216), Velocity FF Gain (220), Position D Gain (224), Position I Gain (228), Position P Gain (232),
   Position FF Gain (236), Profile Acceleration (240), Profile Velocity (244), Profile Acceleration
   Time (248), Profile Time (252), all size 4, RW. The schema allows `area: EEPROM | RAM` only
   (ron-schema §1; data-model "Register row"). Writing EEPROM or RAM for these rows would be a guess.
2. **The area sits in a column, not in a heading.** The pages have one control table under the
   heading "Control Table Configuration" with an `Area` column (columns: Address, Size(Byte), Modbus
   Address, Area, Data Name, Access, Initial Value, Range, Unit). `emanual.py` takes the area from an
   EEPROM/RAM heading (contract table-tools §2, research R-7), so `diff` and `fill` exit 1 on every Y
   page, writing nothing:

   ```
   error: row 0 'Model Number': its table is not under an EEPROM or RAM heading (heading above it: 'Control Table Configuration')
   ```

   (The same table also lists three Modbus-only rows with address `N/A` and access `W`: Factory
   Reset, Reboot and Clear Instruction; the helper skips rows without a numeric address.)

## What was written

Nothing beyond T015's `source: Unverified`. The check fails on these 12 tables (missing
`protocols`, rows without `access`/`area`, missing `operating_modes`).

## Question for the architect (blocks T034, T041, and T1's check)

How should `Hybrid` rows be recorded: (a) a third area value `Hybrid` (contract change:
module-interfaces §3.1, core's `Register`), (b) `EEPROM` or `RAM` by a stated rule, or (c) another
rule? With the answer, `emanual.py` also needs to read an `Area` column when a page has one (a tool
change to contract table-tools §2), then T034 and T041 run as written.
