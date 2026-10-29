# T1 US3 validation (T042), 2026-10-28: NOT PASSING (blocked)

`python3 control_tables/check_tables.py`: exit 1, `summary: 73 tables, 2 verified, 71 unverified,
70 mapped, 3 not mapped, 0 dual-protocol, 5109 errors` (before Phase 6). Every error sits in a table
blocked on an architect question; every other table passes the US3 rules (protocols, row
access/area, roles, mode source, `operating_modes`):

| Tables | Errors | Cause | Findings |
|---|---|---|---|
| 13 PRO / PRO+ / RH | 135–149 each | T033 stopped: page access `R/RW` (External Port Data 1–4) | `t1-pro.md` |
| 12 Y | 205 each | T034 stopped: page area `Hybrid` and an `Area` column | `t1-y.md` |
| 6 X330 `_fw52` | 123 each | T032: the pages give no firmware-52 control table, nothing filled | `t1-x330.md` |
| 6 X330 (current firmware) | 4 each | T032: alias rows "Indirect Address Read" (180, 2) and "Indirect Data Read" (230, 1) sit in the rows the page elides ("…"); no page row at that address and size | `t1-x330.md` |

T040 and T041 (`operating_modes` for the PRO and Y groups) were not run: they follow T033 and
T034, which stopped.

Other T042 checks:
- `grep -L '"Operating Mode"' control_tables/*.ron`: the 5 `ax_*` tables, `xl320`, `rx_28` (new in
  T1, CW/CCW pair, as AX) and `dynamixel.ron`.
- `grep -L 'operating_modes' control_tables/*.ron`: the 5 `ax_*` tables, `rx_28`, `dynamixel.ron`,
  plus the 25 PRO/PRO+/RH and Y tables of the stopped tasks.
- `python3 control_tables/analyze_tables.py`: regenerated; XL-320 under "P2 only"; per stem, the
  JSON `protocols` equals the table's `protocols` (0 mismatches; the 25 blocked tables have none
  yet and show `?`).
- `cargo xtask codegen && git diff --exit-code src/models.rs src/models/`: empty.
- No role rule failed on real data in the filled tables (no address, size or constant change needed).
