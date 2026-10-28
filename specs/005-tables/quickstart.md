# Quickstart: validating the control tables

**Feature**: [spec.md](spec.md) | **Format**: [contracts/ron-schema.md](contracts/ron-schema.md) |
**Tools**: [contracts/table-tools.md](contracts/table-tools.md)

## Prerequisites

- Python 3.10 or later (no packages to install).
- Rust latest stable with `rustfmt` (for `cargo xtask codegen`).
- Network access to `emanual.robotis.com` for sections 4 and 5 only.

## 1. Gates every tables PR runs

```sh
python3 -m unittest discover -s control_tables -p 'test_*.py'
python3 control_tables/check_tables.py
cargo xtask codegen && git diff --exit-code src/models.rs src/models/
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
cargo clippy --no-default-features --features ax,mx,x-series,y-series,pro --target thumbv7em-none-eabihf
```

T2 also runs the coverage gate, since it commits regenerated `src/models*` (CI runs it on every
PR; T1 touches no Rust, so its coverage is unchanged):

```sh
cargo llvm-cov --workspace --ignore-filename-regex 'src/models(\.rs|/)' --fail-under-lines 70
```

Expected: all pass. The check exits 0 and lists every unverified table. In T1 `cargo xtask codegen`
changes nothing (FR-016, SC-004); it still warns about the three `not_mapped` files until X1 reads
`not_mapped`. In T2 the regenerated files are already committed, so the diff is empty, and
codegen skips the three `not_mapped` files without a warning.

## 2. Scenario checks

| Spec item | How | Expected |
|---|---|---|
| US1 AS1, SC-001, SC-005 | `head -3 control_tables/*.ron` | every table starts with `source: Unverified` or `source: Verified(url: …, verified: …)` |
| US1 AS2 | delete the `source` line of one table in a scratch copy, run the check with `--dir` | exit 1, error names that file |
| US1 AS3 | run the check | unverified tables listed by name, exit 0 |
| US1 AS4, FR-002 | in a scratch copy, set a URL on another host, then a date `2026-02-30`, then a future date | exit 1 each time, error names the file |
| US1 AS5, FR-016 | T1: `cargo xtask codegen`, `git diff src/` | empty |
| US2, SC-002 | check output for `ax_12a` (T1) and `mx_64`, `ax_12_plus`, `mx_64_p1` (T2) | `verified` with an `emanual.robotis.com` URL and a valid date |
| SC-002 audit, SC-008 | `python3 control_tables/emanual.py diff ax_12a` (and the other three) | no difference, mode row (or its absence) included; then 5 random rows checked by eye on the cited page |
| US2 AS5, FR-013 | check output | Y-series and PRO tables mapped and `unverified` |
| US3 AS1, FR-005, SC-006 | check output, protocols column | every table shows `[P1]`, `[P2]` or `[P1, P2]` |
| US3 AS2, FR-007 | the check passes (it fails on a row without `access` or `area`) | every row has both |
| US3 AS4, FR-015 | `python3 control_tables/analyze_tables.py`, then read the protocol section | XL-320 under P2; grouping equals the `protocols` fields |
| US3 AS5, FR-020, SC-008 | the check passes; scratch copy: delete the `Operating Mode` row of `xm430_w350`, then add a `Control Mode` row to it | exit 1 each time, the error names the file (no mode source; two sources) |
| US3 AS6, US5 AS6, FR-020 | `grep -L '"Operating Mode"' control_tables/*.ron` | only the 5 AX tables, `xl320`, `dynamixel.ron` and (T2) the 3 MX P1 tables |
| US4 AS1–3, SC-007 | `dual-protocol models` section of the check | each lists a P1 and a P2 table and `default: P2` |
| US4 AS4 | after T2: X1's lookup test asks for model 310 on P2 | `UnsupportedModel` (X1's test; listed here for traceability) |
| US4 AS5, FR-009 | T1: legacy `tables` equals the default list (check rule); codegen diff empty | same table per number as before |
| US5, SC-003 | check summary | 0 unmapped files without a record, 0 duplicate numbers; T2: entries 29, 310, 320 with `p1` |
| US2 AS4, US5 AS3–AS4 | T2: `cargo xtask codegen` | no skip warning; `ax_12_plus`, `ax_18f`, `xw430_t200` still not generated, their names in `also` |
| contract §2 `tables` | T2: scratch copy, add `tables: [(from_fw: 0, file: "ax_12a")]` to entry 12 | exit 1 naming model 12 |
| FR-010 | scratch copy: swap two `from_fw` values | exit 1 |
| FR-012 | scratch copy: add a second `1020:` key | exit 2 (parse error names the duplicate key) |

## 3. Negative cases in the unit tests

`test_check_tables.py` holds one small fixture per rule in ron-schema.md and asserts the exit code
and the error text: missing flag, bad URL host, bad date, future date, empty `protocols`,
`instructions` on an unverified table, a P1 entry with `SyncRead`, a row without `access`, an
`access: W`, a missing role, a P1-mapped role above 255, `default` missing or `P1`, a file under two
numbers, a file both mapped and in `not_mapped`, an unrecorded file, `SameNumberAs` to an unknown
number, bad `from_fw` order, a missing file, a legacy `tables` that differs from the default list,
a mapped table without `Firmware Version`, two files of one `p1` (or `p2`) list with `Firmware
Version` at different (address, size), both limit forms, half a limit form (`CW Angle Limit`
alone), `Model Number` with `access: RW` or `area: RAM`, `family: "dxl"`, two rows of one table
whose names give one constant (`Goal Position` and `Goal-Position`), two mapped stems that give one
type identifier (`ax_12a` and `ax12a`), a mapped stem `tests`, a mapped stem `type`. Mode source
(FR-020): a Min/Max table without `Operating Mode`, an `Operating Mode` and a `Control Mode` row
in one table, an `Operating Mode` row beside the CW/CCW pair, two `Operating Mode` rows in an
unmapped table, a `Control Mode` row in a table other than `xl320`, an `Operating Mode` row of
size 2, `access: R` or in `RAM`, a `p1`-mapped table with `Operating Mode` at 300; passing cases:
an X-shaped, a Y-shaped, an XL-320-shaped and an AX/MX-P1-shaped table. In T2,
the legacy test becomes "any `tables` key is an error".

`test_emanual.py` parses three hand-written HTML snippets shaped like the AX, X and PRO+ pages
(different columns, EEPROM/RAM headings) and checks `fill` on a fixture table in both row styles.

## 4. Fill `access` and `area` for one table (T1 workflow)

```sh
python3 control_tables/emanual.py diff xm430_w350
python3 control_tables/emanual.py fill xm430_w350
python3 control_tables/check_tables.py xm430_w350
```

Expected: `fill` reports the rows it filled and the alias rows it left (research R-14). The human
sets those from the page row at the same address and size, adds them to the PR's findings list, and
the check passes.

## 5. Verify one model (any later PR, one model at a time)

1. `python3 control_tables/emanual.py diff <stem>`: resolve every difference from the page.
2. Check `type_info`, every `unit_info` entry, `protocols`, the mode row (`Operating Mode` or
   `Control Mode`, or that the page lists neither) and the protocol page instructions by hand on
   the page. Remove what the page does not give.
3. Set `source: Verified(url: "<page>", verified: "<today>")` and add `instructions`.
4. `python3 control_tables/check_tables.py --markdown <stem>` and paste the line into the PR,
   with every corrected or removed row (FR-017).
5. If a row changed: `cargo xtask codegen` and commit `src/models*` in the same PR.
