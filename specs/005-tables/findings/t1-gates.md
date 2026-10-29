# T1 gates (T052), re-run 2026-10-29, worktree `../dxl_rs_v2-005-t1`

| Gate | Result |
|---|---|
| `python3 -m unittest discover -s control_tables -p 'test_*.py'` | OK, 113 tests |
| `python3 control_tables/check_tables.py` | exit 0, `summary: 73 tables, 2 verified, 71 unverified, 70 mapped, 3 not mapped, 28 dual-protocol, 0 errors`; 0.21 s |
| `cargo xtask codegen && git diff --exit-code src/models.rs src/models/` | empty; three skip warnings (`ax_12_plus`, `ax_18f`, `xw430_t200`) |
| `cargo fmt --check` | OK |
| `cargo clippy --all-targets -- -D warnings` | OK |
| `cargo test --workspace` | OK (4 test binaries, all pass) |
| `cargo clippy --no-default-features --features ax,mx,rx,x-series,y-series,pro --target thumbv7em-none-eabihf` | OK |

Quickstart §2, T1 rows (scratch copies, `--dir`, `--today 2026-10-29`; first error line shown):

```
US1 AS2 delete source: exit 1 (want 1); xm430_w350.ron: missing `source` (Unverified or Verified(url, verified))
US1 AS4 other host: exit 1 (want 1); xm430_w350.ron: `source` url: URL 'https://example.com/x/' is not on https://emanual.robotis.com/
US1 AS4 2026-02-30: exit 1 (want 1); ax_12a.ron: `source` verified: '2026-02-30' is not a real date
US1 AS4 future date: exit 1 (want 1); ax_12a.ron: `source` verified: 2026-10-30 is in the future (today is 2026-10-29)
US3 AS5 delete Operating Mode: exit 1 (want 1); xm430_w350.ron: no mode source: no `Operating Mode` or `Control Mode` row and no CW/CCW Angle Limit pair
US3 AS5 add Control Mode: exit 1 (want 1); xm430_w350.ron: both an `Operating Mode` and a `Control Mode` row (two mode sources)
US3 AS7 delete operating_modes: exit 1 (want 1); xm430_w350.ron: `operating_modes`: missing; a table with a mode row lists what each mode value means
US3 AS7 operating_modes on ax_12a: exit 1 (want 1); ax_12a.ron: `operating_modes`: present without an `Operating Mode` or `Control Mode` row
US3 AS7 4 in two lists: exit 1 (want 1); xm430_w350.ron: `operating_modes`: value 4 in both `bounded` and `unbounded`
US5 delete a not_mapped record: exit 1 (want 1); ax_18f.ron: not mapped and not in `not_mapped` of dynamixel.ron
FR-010 swap from_fw: exit 1 (want 1); dynamixel.ron: model 1190: `p2`: the first `from_fw` must be 0, found 53
FR-012 second 1020 key: exit 2 (want 2); error: dynamixel.ron:39:5: duplicate key 1020
```
