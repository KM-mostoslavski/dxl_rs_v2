# T2 baseline (T057, T058)

Date: 2026-10-29. Worktree `../dxl_rs_v2-005-t2`, branch `005-tables-t2` off `origin/integration`
at `7dd7469` (main + core-1 #8, tables T1 #9, xtask X1 #25, ci #10; user decision 2026-10-29:
"merged into `main`" reads "merged into `integration`").

## T057 baseline

- Main checkout (`dev`): dirty with the architect's SRS/constitution edits and untracked ov files;
  nothing there is staged or committed by T2. No unmerged T2 work existed (`005-tables-t2` new).
- `python3 --version`: 3.14.8.
- `cargo xtask codegen`: `generated 72 files under src/ (0 written, 0 removed, 72 unchanged)`, no
  skip warning (X1 reads `not_mapped`). `git diff --exit-code src/models.rs src/models/`: empty.
- `python3 control_tables/check_tables.py`: exit 0,
  `summary: 73 tables, 2 verified, 71 unverified, 70 mapped, 3 not mapped, 28 dual-protocol, 0 errors`
  (T1 + RX-28: the task list's "72 tables" predates RX-28).
- `python3 -m unittest discover -s control_tables -p 'test_*.py'`: 113 tests, OK.

## T058 X1 checks against contracts/table-tools.md §1

Compared `specs/006-xtask-codegen/data-model.md` §3 (checks 1–17) and `xtask/src/codegen/resolve.rs`
/ `schema.rs` as merged in X1 with the mapping table.

| xtask check | X1 as merged | check_tables.py | Result |
|---|---|---|---|
| 1 parse | `OperatingModes { bounded, unbounded, no_position: Vec<u8> }`, all three required, no `deny_unknown_fields`; `also: Vec<String>` defaulting to empty; `from_fw: u8`; `address: u16`, `size: u8`; `Area` = `EEPROM` / `RAM` / `Hybrid` | three keys required, values 0–255, unknown keys rejected (stricter) | matches |
| 2–9, 13, 15, 16 | as in the mapping table | same rules | no change |
| 5 | `FAMILIES = ax, mx, rx, x-series, y-series, pro` | same tuple | no change |
| 10, 11 | roles + limit forms; mode register 1 byte RW EEPROM | same | no change |
| 12 | Model Number at (0, 2); access/area equal across mapped tables (majority vote) | (0, 2), `R`, `EEPROM` in each mapped table | check is stricter; no change |
| **14** | `check_p1_addresses` also checks **`Hardware Error Status`** (an optional role, when present) besides the roles and the mode register | roles, limit rows and mode rows only | **changed**: added |
| 17 | mode source as in the mapping row; `Operating Mode` beside **either** CW or CCW row fails | `any_cw` (either row) | matches |

Change made (T058): `test_check_tables.py` `MapTests.test_p1_hardware_error_status_above_255`
(failed first), then `check_tables.py` counts `Hardware Error Status` (`FAULT_ROLE`) in the
`p1` > 255 rule. 114 tests OK; the real data still passes.

For the architect: the contract table row for check 14 (`contracts/table-tools.md` §1, "no role
above 255 in a table under `p1`, the mode register included") should also name `Hardware Error
Status`. Agents do not edit the contract; the check follows codegen as the contract says.
