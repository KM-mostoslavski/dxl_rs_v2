# T1 US3 validation (T042), re-run 2026-10-29: PASSING

(The 2026-10-28 run was blocked on three questions; provisional answers recorded in spec
Clarifications, Session 2026-10-29.)

- `python3 control_tables/check_tables.py`: exit 0, `summary: 73 tables, 2 verified, 71 unverified,
  70 mapped, 3 not mapped, 28 dual-protocol, 0 errors`; protocols column on all 73 (6 `[P1]`, 38
  `[P2]`, 29 `[P1, P2]`); no row-attribute, mode-source or `operating_modes` error.
- Mode-row findings (T028–T034): none; every mode row matches its page. `operating_modes` findings
  (T036–T041): page lists that differ from the family lists: `mx_28`, `xc430_w150`, `xc430_w240`
  (`t1-modes-mx-xl320.md`, `t1-modes-x430.md`); none in PRO, RH, Y.
- `grep -L '"Operating Mode"' control_tables/*.ron`: the 5 `ax_*` tables, `xl320`, `rx_28` (new in
  T1, CW/CCW pair) and `dynamixel.ron`.
- `grep -L 'operating_modes' control_tables/*.ron`: the 5 `ax_*` tables, `rx_28` and
  `dynamixel.ron` (67 tables carry it).
- `python3 control_tables/analyze_tables.py`: regenerated; XL-320 under "P2 only"; per stem the JSON
  `protocols` equals the table's (0 mismatches, 0 tables without protocols).
- `cargo xtask codegen && git diff --exit-code src/models.rs src/models/`: empty.
- No role rule failed on real data.
