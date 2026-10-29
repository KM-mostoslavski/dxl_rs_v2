# T1 AX group (T028): `ax_12_plus`, `ax_12w`, `ax_18a`, `ax_18f`

Pages from `PAGES`: `ax_12_plus` → AX-12A page (headed "AX-12+, AX-12A"), `ax_18f` → AX-18A page (headed "AX-18F, AX-18A"). All fetched 2026-10-28.

- `protocols: [P1]` on all four: the pages have no Protocol Type register; they refer only to Protocol 1.0 ("Please refer to DYNAMIXEL Protocol 1.0 for more details about Instruction Packets", "refer to the Status Packet section for DYNAMIXEL Protocol 1.0"; the only Protocol 2.0 mention is that the XL430-W250 replacement adds "support for DYNAMIXEL Protocol 2.0").
- `type_info` on all four: pages give "The available position range is 0 to 1,023 (0x3FF) and the per unit value is 0.29 °", Running Degree "0 ~ 300 [°]", and the Goal Position figure `/assets/images/dxl/dx/dx_series_goal_position.png` (0 → 0°, 512 → 150°, 1023 → 300°). Slope 300°/1023, zero 512 (research R-8): `min_radian: 0.0` → `-2.6205530`, `max_radian: 5.2359877` → `2.6154347`; zero/min/max `512 / 0 / 1023` unchanged. In `ax_18a` and `ax_18f` the stale comment `// 300° = 5.2359 rad` on the old `max_radian` line was removed with the value.
- Mode source (FR-020): none of the pages lists `Operating Mode` or `Control Mode` (the diffs show no page-only row); all list `CW Angle Limit` (6, 2) and `CCW Angle Limit` (8, 2), RW, EEPROM, and describe Wheel Mode ("both are 0") and Joint Mode ("neither are 0"). The tables carry neither mode row and get no `operating_modes`. No mode-row finding.
- `unit_info` and `instructions` untouched (unverified tables); `source: Unverified` kept; no address, size or name changed.

### `ax_12_plus`

Page https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/, fetched 2026-10-28.

- `fill`: 32 rows filled from the page (address, size and name match).
- Rows only on the page (0, not added in T1): none.
- Rows only in the table: none.
- Name differences at the same address and size: none.
- `protocols`/`type_info` change: `[P1] ['min_radian: 0.0 -> -2.6205530', 'max_radian: 5.2359877 -> 2.6154347']`.

### `ax_12w`

Page https://emanual.robotis.com/docs/en/dxl/ax/ax-12w/, fetched 2026-10-28.

- `fill`: 32 rows filled from the page (address, size and name match).
- Rows only on the page (0, not added in T1): none.
- Rows only in the table: none.
- Name differences at the same address and size: none.
- `protocols`/`type_info` change: `[P1] ['min_radian: 0.0 -> -2.6205530', 'max_radian: 5.2359877 -> 2.6154347']`.

### `ax_18a`

Page https://emanual.robotis.com/docs/en/dxl/ax/ax-18a/, fetched 2026-10-28.

- `fill`: 32 rows filled from the page (address, size and name match).
- Rows only on the page (0, not added in T1): none.
- Rows only in the table: none.
- Name differences at the same address and size: none.
- `protocols`/`type_info` change: `[P1] ['min_radian: 0.0 -> -2.6205530', 'max_radian: 5.2359877 -> 2.6154347 (removed comment `// 300° = 5.2359 rad`)']`.

### `ax_18f`

Page https://emanual.robotis.com/docs/en/dxl/ax/ax-18a/, fetched 2026-10-28.

- `fill`: 32 rows filled from the page (address, size and name match).
- Rows only on the page (0, not added in T1): none.
- Rows only in the table: none.
- Name differences at the same address and size: none.
- `protocols`/`type_info` change: `[P1] ['min_radian: 0.0 -> -2.6205530', 'max_radian: 5.2359877 -> 2.6154347 (removed comment `// 300° = 5.2359 rad`)']`.
