# T081: gates (2026-10-29, T2 worktree, regenerated files staged)

## quickstart §1

| Gate | Result |
|---|---|
| `python3 -m unittest discover -s control_tables -p 'test_*.py'` | OK (115 tests) |
| `python3 control_tables/check_tables.py` | exit 0; `summary: 76 tables, 5 verified, 71 unverified, 73 mapped, 3 not mapped, 28 dual-protocol, 0 errors` |
| `cargo xtask codegen && git diff --exit-code src/models.rs src/models/` | `0 written, 0 removed, 75 unchanged`; diff empty; no skip warning |
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo test --workspace` | pass (125 + 426 + 8 + 9 tests) |
| `cargo clippy --no-default-features --features ax,mx,rx,x-series,y-series,pro --target thumbv7em-none-eabihf` | pass (`rx` added to the quickstart list since RX-28) |
| `cargo llvm-cov --workspace --ignore-filename-regex 'src/models(\.rs|/)' --fail-under-lines 70` | pass, lines 98.20 % |

## quickstart §2, T2 rows

| Item | Result |
|---|---|
| US1 AS1 / SC-001 | `head -3` of all 76 tables: each has `source: Unverified` or `source: Verified(url: …)` |
| US2, SC-002 | check lists `mx_64`, `ax_12_plus`, `mx_64_p1` verified (2026-10-29) with their `emanual.robotis.com` URLs, plus `ax_12a`, `rx_28` from T1 |
| SC-002 audit, SC-008 | `emanual.py diff`: `ax_12_plus` 0, `mx_64_p1` 0, `mx_64` only the 22 numbered indirect rows (question in `t2-mx_64.md`); mode row and `operating_modes` of `mx_64` match the page; 5 random rows each by eye, 15/15 (`t2-us2-validation.md`) |
| US2 AS5, FR-013 | Y-series and PRO tables mapped and `unverified` |
| US3 AS6, US5 AS6 | `grep -L '"Operating Mode"'`: 5 AX, `rx_28`, `xl320`, 3 MX P1, `dynamixel.ron`; `grep -L operating_modes`: the same minus `xl320` |
| US4 AS4 | generated test `no_tables_on_this_protocol` includes `(P2, 310)` (and 29, 320): passes |
| US5, SC-003 | summary above; 29, 310, 320 `p1` only; 0 unrecorded files, 0 duplicate numbers |
| US2 AS4, US5 AS3–AS4 | codegen prints no skip warning; `ax_12_plus`, `ax_18f`, `xw430_t200` not generated, named in `also` docs |
| contract §2 `tables` | scratch copy with `tables: [(from_fw: 0, file: "ax_12a")]` on 12: exit 1, "dynamixel.ron: model 12: the legacy `tables` key was removed in T2" |
