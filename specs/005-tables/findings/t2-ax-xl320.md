# T2 AX + XL-320 (T065): `ax_12w`, `ax_18a`, `ax_18f`, `xl320`

T1 findings: `t1-ax.md` and `t1-mx-xl320.md`. Neither lists a row finding for these four tables
(no alias rows, no row only in the table or only on the page, no name difference, no mode-row
finding). Nothing to correct; no file changed. All four stay `source: Unverified`.

`python3 control_tables/emanual.py diff <stem>` on 2026-10-29 (pages from `PAGES`):

| stem | page | result |
|---|---|---|
| `ax_12w` | https://emanual.robotis.com/docs/en/dxl/ax/ax-12w/ | 0 differences |
| `ax_18a` | https://emanual.robotis.com/docs/en/dxl/ax/ax-18a/ | 0 differences |
| `ax_18f` | https://emanual.robotis.com/docs/en/dxl/ax/ax-18a/ (headed "AX-18F, AX-18A") | 0 differences |
| `xl320` | https://emanual.robotis.com/docs/en/dxl/x/xl320/ | 0 differences |

Mode source: the three AX tables keep the CW/CCW Angle Limit pair and no mode row (their pages list
no `Operating Mode`/`Control Mode`); `xl320` keeps `Control Mode` (11, 1, RW, EEPROM) as its page,
with `operating_modes` unchanged.
