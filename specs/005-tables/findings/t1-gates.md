# T1 gates (T052), 2026-10-28, worktree `../dxl_rs_v2-005-t1`

| Gate | Result |
|---|---|
| `python3 -m unittest discover -s control_tables -p 'test_*.py'` | OK, 107 tests |
| `python3 control_tables/check_tables.py` | **exit 1**: `summary: 73 tables, 2 verified, 71 unverified, 70 mapped, 3 not mapped, 28 dual-protocol, 5134 errors`, all in the blocked tables (`t1-us3-validation.md`, `t1-map.md`); 0.19 s |
| `cargo xtask codegen && git diff --exit-code src/models.rs src/models/` | empty (RX-28 files staged); three skip warnings |
| `cargo fmt --check` | OK |
| `cargo clippy --all-targets -- -D warnings` | OK |
| `cargo test --workspace` | OK (127 + 9 + 5 + 1) |
| `cargo clippy --no-default-features --features ax,mx,rx,x-series,y-series,pro --target thumbv7em-none-eabihf` | OK |

Quickstart §2, T1 rows (scratch copies of `control_tables/`, `--dir`; "new errors" = errors not in
the current run, so the blocked tables do not hide a result):

```
US1 AS2 delete source: exit 1; new errors: ['xm430_w350.ron: missing `source` (Unverified or Verified(url, verified))']
US1 AS4 other host: exit 1; new errors: ['xm430_w350.ron: `instructions`: missing; a verified table lists its instructions per protocol', "xm430_w350.ron: `source` url: URL 'https://example.com/x/' is not on https://emanual.robotis.com/"]
US1 AS4 date 2026-02-30: exit 1; new errors: ['xm430_w350.ron: `instructions`: missing; a verified table lists its instructions per protocol', "xm430_w350.ron: `source` verified: '2026-02-30' is not a real date"]
US1 AS4 future date: exit 1; new errors: ['xm430_w350.ron: `instructions`: missing; a verified table lists its instructions per protocol', 'xm430_w350.ron: `source` verified: 2026-10-29 is in the future (today is 2026-10-28)']
US3 AS5 delete Operating Mode row: exit 1; new errors: ['xm430_w350.ron: `operating_modes`: present without an `Operating Mode` or `Control Mode` row', 'xm430_w350.ron: no mode source: no `Operating Mode` or `Control Mode` row and no CW/CCW Angle Limit pair']
US3 AS5 add Control Mode row: exit 1; new errors: ['xm430_w350.ron: `Control Mode` is accepted only in xl320.ron (contract 4.3); raise a second one first', 'xm430_w350.ron: both an `Operating Mode` and a `Control Mode` row (two mode sources)']
US3 AS7 delete operating_modes: exit 1; new errors: ['xm430_w350.ron: `operating_modes`: missing; a table with a mode row lists what each mode value means']
US3 AS7 operating_modes on ax_12a: exit 1; new errors: ['ax_12a.ron: `operating_modes`: present without an `Operating Mode` or `Control Mode` row']
US3 AS7 value 4 in two lists: exit 1; new errors: ['xm430_w350.ron: `operating_modes`: value 4 in both `bounded` and `unbounded`']
FR-010 swap from_fw: exit 1; new errors: ['dynamixel.ron: model 1190: `p2`: `from_fw` must strictly increase', 'dynamixel.ron: model 1190: `p2`: the first `from_fw` must be 0, found 53', 'dynamixel.ron: model 1190: legacy `tables` differs from the default (or only) list']
FR-012 second 1020 key: exit 2; error: dynamixel.ron:39:5: duplicate key 1020
```

Also: `head -3 control_tables/*.ron` shows `source` first in every table (US1 AS1, SC-005); the
`tables` section lists all 73 with flag and protocols (`[?]` for the 25 blocked tables; US3 AS1);
Y-series and PRO tables mapped and `unverified` (US2 AS5); XL-320 under P2 in the analysis (US3 AS4).
