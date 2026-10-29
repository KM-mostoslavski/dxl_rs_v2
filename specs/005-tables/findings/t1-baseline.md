# T1 baseline (T002)

Worktree `../dxl_rs_v2-005-t1`, branch `005-tables-t1` at `origin/main` f338e2a, 2026-10-28.

- T001: `git diff --stat origin/main dev -- control_tables xtask` is empty. The main checkout is
  dirty only with the architect's uncommitted SRS / constitution / ov files (left untouched).
- `python3 --version`: Python 3.14.8 (>= 3.10).
- `cargo xtask codegen`: "generated 70 files under src/"; `git diff --exit-code src/models.rs
  src/models/` empty. Exactly three skip warnings:
  - `warning: control_tables/ax_12_plus.ron is not in dynamixel.ron, skipped`
  - `warning: control_tables/ax_18f.ron is not in dynamixel.ron, skipped`
  - `warning: control_tables/xw430_t200.ron is not in dynamixel.ron, skipped`
- `control_tables/`: 72 tables plus `dynamixel.ron`.
- Map: 63 model numbers, 69 mapped files (69 `file:` references, 69 distinct).
- Mode rows (research R-15), from `grep -c '"Operating Mode"\|"Control Mode"' control_tables/*.ron`:
  - 66 tables with one `Operating Mode` row: address 11 in 54, address 33 in the 12 `ym*` tables,
    size 1 in all.
  - `xl320` with one `Control Mode` row (address 11, size 1).
  - 5 `ax_*` tables (`ax_12a`, `ax_12_plus`, `ax_12w`, `ax_18a`, `ax_18f`) with neither.
- `grep -l operating_modes control_tables/*.ron`: empty (no table has the field).
