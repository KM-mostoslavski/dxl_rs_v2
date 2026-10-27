# Contract: table tools (`check_tables.py`, `emanual.py`)

**Feature**: [../spec.md](../spec.md) | **Format**: [ron-schema.md](ron-schema.md)

Both are Python 3.10+ scripts with no third-party package, in `control_tables/`. The command
lines, exit codes and output below are the contract. Internals and tests are not.

## 1. `check_tables.py` (the table check, FR-014)

```text
python3 control_tables/check_tables.py [--dir DIR] [--today YYYY-MM-DD] [--markdown] [STEM ...]
```

| Option | Default | Meaning |
|---|---|---|
| `--dir` | the script's directory | where the tables and `dynamixel.ron` are |
| `--today` | local date | the "not in the future" bound for `verified` dates (tests pin it) |
| `--markdown` | off | print the per-table lines as a Markdown list, for the PR description (FR-017) |
| `STEM ...` | all | limit the per-table section to these tables; checks still run on all |

Network: none.

Output, plain mode (stdout):

```text
tables
  ax_12a        verified https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/ 2026-10-28  [P1]      12 p1
  ax_12_plus    unverified                                                              [P1]      not mapped: same number as 12
  xm430_w350    unverified                                                              [P1, P2]  1020 p1 p2
dual-protocol models
  1020 XM430-W350   p1: xm430_w350   p2: xm430_w350   default: P2
errors
  xl320.ron: row 30 "Goal Position": missing `area`
summary: 72 tables, 1 verified, 71 unverified, 69 mapped, 3 not mapped, 28 dual-protocol, 1 error
```

Errors go to stdout under `errors` and are also printed one per line to stderr. Every error names
the file, and the row, key or model number when there is one.

Exit codes:

| Code | When |
|---|---|
| 0 | no error. Unverified tables are listed, never errors. |
| 1 | at least one rule of [ron-schema.md](ron-schema.md) is broken |
| 2 | bad arguments, a file that does not parse, a missing `dynamixel.ron` |

Rules checked: every rule of ron-schema.md sections 1 and 2. Nothing else (no network, no
comparison with the e-Manual). These include every data-only check of xtask (spec 006,
data-model §3), so a table the check passes is not rejected by codegen for its data:

| xtask check | Check rule |
|---|---|
| 2, 3, 4 | `from_fw` order; at least one non-empty list; `default: P2` exactly with both lists |
| 5 | `family` is one of `ax`, `mx`, `x-series`, `y-series`, `pro` |
| 6, 7 | a file under one number only; mapped or `not_mapped`, never both; files exist |
| 8, 9 | `source` and non-empty `protocols`; a `pX` list names only tables with `PX` |
| 10 | every role present, `Firmware Version` included; exactly one complete limit form |
| 11 | role access and area |
| 12 | `Model Number` at (0, 2), `R`, `EEPROM`, in every mapped table |
| 13 | `Firmware Version` at one (address, size) within each `p1` list and within each `p2` list |
| 14 | no role above 255 in a table under `p1` |
| 15 | scale sanity |
| 16 | no duplicate register constant in a table; no duplicate type identifier among mapped files; no module identifier `tests` or a keyword |

Codegen stays the authority. If xtask adds a data check, this table and the check follow.

## 2. `emanual.py` (page reader, maintainer only)

```text
python3 control_tables/emanual.py rows URL
python3 control_tables/emanual.py diff STEM [URL]
python3 control_tables/emanual.py fill STEM [URL]
```

`URL` defaults to the table's entry in the script's page list (one `STEM: URL` line per table, all
on `https://emanual.robotis.com/`). A URL elsewhere is refused.

| Command | Reads | Writes | Prints |
|---|---|---|---|
| `rows` | the page | nothing | every control-table row: address, size, name, access, area, in page order, in the RON row style |
| `diff` | the page, the table | nothing | rows only in the table, rows only on the page, rows with a different name at the same address and size, rows whose existing `access`/`area` differ |
| `fill` | the page, the table | `control_tables/STEM.ron` | the rows it filled and the rows it left for a human |

`fill` rules:
- It adds `access` and `area` only to a row whose address, size and name all match one page row,
  and only when the row has neither field yet. It never changes an address, size, name or any
  other field.
- It keeps the file's row style (multi-line or one-line).
- It stops with exit code 1, writing nothing, when the page has an access value other than `R` or
  `RW`, or a row it cannot place under an EEPROM or RAM heading.

Exit codes: 0 done; 1 the page broke a rule above or the diff found differences (`diff` only);
2 network error, page not found, or bad arguments.

Not in CI: it needs the network. Its tests (`test_emanual.py`) run on small HTML snippets in the
test file and need no network.
