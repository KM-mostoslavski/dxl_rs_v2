# Implementation Plan: Control tables (verification flags, capabilities, per-protocol model map)

**Branch**: `005-tables` (planned on `dev`; the PR branches are created from `main` at
implementation) | **Date**: 2026-10-27 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/005-tables/spec.md`, the shared contract
`specs/ideas/module-interfaces.md` (sections 1, 3 and 7), constitution v1.2.0 and
`docs/srs/modules/tables.md`.

## Summary

This feature changes data, not Rust code. It extends the 72 `control_tables/*.ron` files and
`dynamixel.ron` to the RON schema of module-interfaces section 3, and adds two maintainer tools
next to `analyze_tables.py`.

- **Every table** gets `source` (`Unverified`, or `Verified(url, verified)`), `protocols`, and
  `access` + `area` on every row. Verified tables also get `instructions`. All values come from
  the model's e-Manual page (`instructions` from the Protocol 1.0 / 2.0 pages).
- **The map** gets `p1` / `p2` lists per model number, `default: P2` on dual-protocol models,
  `also` for other products sold under the same number, and a `not_mapped` record.
- **A table check** (`check_tables.py`, Python stdlib only) validates the schema and the map,
  lists unverified tables, and fails only on real errors. CI can run it.
- **An e-Manual helper** (`emanual.py`) reads a model page's control table and fills `access` and
  `area` only where address, size and name all match the page. It lists every mismatch for a
  human.
- **Priority models** are verified: AX-12A, AX-12+, MX-64(2.0), MX-64 on Protocol 1.
- **New tables**: MX-28, MX-64 and MX-106 on Protocol 1 firmware (models 29, 310, 320).

It ships as two PRs (module-interfaces section 7): **T1** has no dependency and changes no
generated file. **T2** follows xtask X1, adds the new tables and commits the regenerated
`src/models*`.

## Technical Context

**Language/Version**: RON data files, read by `ron` 0.12.2 in `xtask`. Tools in Python 3.10 or
later, standard library only (development uses 3.14.8; GitHub `ubuntu-latest` ships 3.12). No Rust
code changes. Rust latest stable (rustc 1.97.1) only for `cargo xtask codegen`.

**Primary Dependencies**: none new. The tools use `re`, `datetime`, `html.parser`,
`urllib.request`, `argparse`, `unittest`. No `pip` package (research R-6).

**Storage**: files in `control_tables/`: 72 tables today (75 after T2), the map, the analysis
report (`.md` + `.json`).

**Testing**:
- `python3 -m unittest discover -s control_tables -p 'test_*.py'`: parser, every check rule,
  e-Manual HTML parsing on small hand-written fixtures. No network.
- `python3 control_tables/check_tables.py` on the real data: exit 0.
- `cargo xtask codegen` then `git diff --exit-code src/`: no diff in T1 (proven by prototype,
  research R-1); the committed output in T2.
- The usual Rust gates (fmt, clippy, test, no_std, coverage) pass unchanged: T1 touches no Rust,
  T2 touches only generated files.

**Target Platform**: maintainer machines (Linux, macOS) and the GitHub Linux runner. The helper
needs network access to `emanual.robotis.com`; the check does not.

**Project Type**: data set + maintainer tools inside a library crate workspace.

**Performance Goals**: the check runs in under 2 s on 75 tables (prototype parser: 0.12 s for 73
files).

**Constraints**:
- Every value from the official e-Manual (constitution IV, FR-003). Unverified tables stay mapped.
- T1 must leave today's codegen output byte-identical (FR-016, contract T1 "no regeneration").
- Brace style stays valid for xtask's brace-to-paren rewrite (contract 3.1).
- Paths: `control_tables/**` only. T2 also commits the regenerated `src/models.rs` and
  `src/models/**` (contract section 1).

**Scale/Scope**: 72 tables, 4,635 register rows (the spec's 4,904 also counts 269 `unit_info`
entries), 63 model numbers and 69 mapped files today (the spec says 64 numbers; the map header
comment mentions `from_fw` once more). After T2: 75 tables, 66 model numbers, 72 mapped files,
3 not mapped. About 28 dual-protocol model numbers (research R-4).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Check | Status |
|---|---|---|
| I. no_std and Heap-Free | No Rust code. Generated code is unchanged in T1; T2 only adds model types of the same shape. | N/A |
| II. Absent Over Failing | The per-protocol map lets X1 fail Setup with `UnsupportedModel` when a model has no table on the bus protocol (US4 AS4). Data only here. | PASS |
| III. Nothing Hidden | Unverified tables say so at their top, in the check report and (through X1) in the docs. No range or default is added, so nothing can become a limit fallback. | PASS |
| IV. e-Manual Is the Only Source | Every new value comes from the model's e-Manual page (instructions from the protocol pages). The helper fills a field only when address, size and name match the page; it never copies a sibling table. Unverified tables stay usable and flagged. Dual-protocol models get both lists with `default: P2`. | PASS, with one note: AX-12+ has no page of its own (research R-9) |
| V. Committed, Data-Driven Codegen | T1: no generated change, proven by prototype. T2: regenerated output committed in the same PR, CI diff check stays green. No hand edits. | PASS |
| VI. Typed, Static, Panic-Free API | No API change. `access`/`area` feed core's `Register` (contract 2.2). | N/A |
| VII. Developer Experience First | One flag per table, at the top. One command shows what is verified. Mismatches are listed, not hidden. | PASS |
| Workflow: one SRS module per PR | Two PRs (T1, T2), as contract section 7 schedules. | **DEVIATION**, see Complexity Tracking |
| Workflow: PR that verifies or changes a table cites URL and date | PR description lists, per changed table, its flag and every corrected or removed row (FR-017). The check prints that list. | PASS |
| Workflow: gates (fmt, clippy, test, no_std, codegen no-diff, coverage ≥ 70 %) | No Rust source changes; coverage is unchanged. Python tests are extra. | PASS |
| Governance: SRS and module copy agree | `docs/srs/modules/tables.md` is the pre-amendment copy (FR-7.2 "every entry verified online", roadmap "MX P1 later"). Constitution 1.2.0 and the spec carry the 2026-10-27 amendment. Architect regenerates the copy. Not edited here. | FLAGGED |

Post-design re-check (after Phase 1): unchanged. The schema, the tools and the PR split keep every
principle. The only exceptions are the two-PR split and the stale module copy, both already
listed in module-interfaces section 9.

## Project Structure

### Documentation (this feature)

```text
specs/005-tables/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── ron-schema.md        # table and map format (module-interfaces §3, made exact)
│   └── table-tools.md       # check_tables.py and emanual.py: CLI, exit codes, rules
├── checklists/
│   └── requirements.md
└── tasks.md                 # /speckit-tasks
```

### Source (repository root)

```text
control_tables/
├── *.ron                    # T1: + source, protocols; rows + access, area; type_info fixes
│                            # T1: AX-12A Verified + instructions; T2: MX-64(2.0) Verified,
│                            #     alias rows removed (research R-14)
├── mx_28_p1.ron             # T2: new, Protocol 1 firmware, model 29, Unverified
├── mx_64_p1.ron             # T2: new, model 310, Verified + instructions
├── mx_106_p1.ron            # T2: new, model 320, Unverified
├── ax_12_plus.ron           # T1: not_mapped SameNumberAs(12); T2: Verified
├── dynamixel.ron            # T1: p1/p2/default beside legacy tables, not_mapped, header doc
│                            # T2: also, MX P1 entries, legacy tables removed
├── check_tables.py          # T1: new, the table check (FR-014)
├── test_check_tables.py     # T1: new, unittest
├── emanual.py               # T1: new, page reader / diff / fill helper
├── test_emanual.py          # T1: new, unittest on hand-written HTML snippets
├── analyze_tables.py        # T1: protocol from `protocols`, model numbers from the new map
├── control_table_analysis.md / .json   # T1 and T2: regenerated
src/models.rs, src/models/** # T2 only: regenerated by `cargo xtask codegen`, never hand-edited
```

**Structure Decision**: everything lives in `control_tables/`, the module's SRS path. The tools sit
next to `analyze_tables.py` (spec Assumptions). Tests use the `test_*.py` names so
`unittest discover` finds them. No Rust module and no `Cargo.toml` change. Wiring the check into
CI is the `ci` module's job (research R-12).

### PR split

| PR | Needs | Content | Generated diff |
|---|---|---|---|
| **T1** | nothing (parallel with core-1) | Tools and tests. `source: Unverified` + `protocols` on all 72 tables. `access` + `area` on all 4,635 rows from each model's page via `emanual.py`. `type_info` checked against each page; wrong values fixed (XL-320 range, X/MX(2.0)/2XL slope 2π/4096, AX angles moved to the zero; research R-8). Map: `p1`/`p2`/`default` beside legacy `tables`, `not_mapped` for `ax_12_plus`, `ax_18f`, `xw430_t200`. AX-12A verified (all 32 rows already match its page). Findings list (rows whose name or presence differs from the page). Analysis regenerated. | none (CI proves it) |
| **T2** | X1 merged | `mx_28_p1`, `mx_64_p1`, `mx_106_p1` + map entries 29, 310, 320 (`p1` only). `also` on 12, 18, 1270. MX-64(2.0), AX-12+ and MX-64 P1 verified. Alias rows not on the e-Manual removed from 66 tables (research R-14). Other row corrections from the T1 findings applied. Name fix `RH-P12-RN(A)` (35074). Legacy `tables` removed. Regenerated `src/models*` committed. Analysis regenerated. | yes, all explained (SC-004) |

T1 is large but mechanical. Its commits are split so review stays possible: tools, then flags and
protocols, then `access`/`area` per family, then the map, then verification, then the analysis.

## Deviations from module-interfaces.md

1. **XW430-T200 is `SameNumberAs(1270)`, not `NoModelNumber`.** The contract's example in 3.2 shows
   `(file: "xw430_t200", reason: NoModelNumber)`. The e-Manual page for XW430-T200 gives Model
   Number 1,270, the same as XW430-T333 (raw page checked 2026-10-27), and both tables are
   identical. US5 AS4 then requires recording it under the existing entry. The shared number looks
   like an e-Manual copy error; raised to the architect (research R-10). If a real XW430-T200
   reports another number, the bus fails Setup with `UnsupportedModel`, which is safe.
2. **AX-18F is `SameNumberAs(18)`.** The contract's example does not list it. The AX-18A page
   covers "AX-18F, AX-18A" and gives Model Number 18. Same layout. Not a contract change, data
   only.
3. **`instructions` entries carry `source` and an optional `from_fw`.** Contract 3.1 says the shape
   is free until xtask reads it. Model pages do not list instructions; the protocol pages do,
   with firmware conditions (research R-5). Not a contract change.
4. **The check repeats every data-only codegen check** of contract 4.3 and spec 006
   (data-model §3, checks 2–16): roles and their access/area, `Firmware Version` present and at
   one location per (number, protocol), exactly one complete limit form, Model Number at (0, 2)
   R/EEPROM, P1 role address ≤ 255, scale sanity, `default`, known `family`, file under two
   numbers, duplicate register constants and type identifiers, module identifier `tests` or a
   keyword. The mapping is in contracts/table-tools.md. They are duplicated on purpose so T1
   cannot land data that X1 will reject. Codegen stays the authority.

No other deviation. Names, RON spellings (`R | RW`, `EEPROM | RAM`, `P1 | P2`, `Unverified`,
`Verified(url, verified)`, `NoModelNumber`, `SameNumberAs(u16)`), file ownership and order follow
the contract.

## Findings for other modules and the architect

- **xtask (X1)**: XL-320 `type_info` is wrong today (2048 / 4095 / ±π, the page says 0–1023 over
  0–300°). X/MX(2.0)/2XL tables give a slope of 2π/4095; the pages say 2π/4096. T1 fixes both
  before X1 starts reading `type_info`. After T1 the 4096-pulse tables read
  `PositionScale::new(2048, 0, 4095, -3.1415927, 3.1400587)`, and AX / XL-320
  `(512, 0, 1023, -2.6205530, 2.6154347)` (research R-8). X1's examples and tests use these
  values (xtask research R-6, contracts/generated-api.md). X1 must emit one model type per file even when the same file is under
  `p1` and `p2`.
- **core**: the 2π/4095 slope flagged in core research R-7 is fixed in T1. Core's own test
  vectors that build the XM430 scale with ±π can stay (they test the type, not the table) or move
  to `3.1400587`.
- **ci**: resolved. ci (specs/007-ci contracts) has a `control tables: schema check` job that runs
  `python3 control_tables/check_tables.py` and
  `python3 -m unittest discover -s control_tables -p 'test_*.py'`. It is guarded by
  `if [ -f control_tables/check_tables.py ]` until T1 merges (research R-12).
- **Architect**: XW430-T200 / T333 share number 1270 on the e-Manual. AX-12+ has no page of its
  own; its record cites the AX-12A page, which names AX-12+ (research R-9). RH-P12-RN entry 35074 is
  "RH-P12-RN(A)" on the e-Manual (35073 is the older RH-P12-RN, not in this repo).
  `docs/srs/modules/tables.md` is stale. CLAUDE.md's command list could add the check (not
  edited here).

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| Two PRs for one module (T1, T2) | T2's new tables and the removal of legacy `tables` need X1's per-protocol codegen. T1 must land before X1 so X1 has data to read. | One PR after X1: X1 would have no `source`, `protocols`, `access`, `area` to read and would fail its own checks. One PR before X1: today's codegen cannot map one number to two protocols. Approval asked in module-interfaces section 9. |
| T2 commits `src/models.rs` and `src/models/**` (no-go `src/**`) | CI fails on a stale regeneration diff; new tables must be generated. | Leaving them out breaks CI. The files are tool output only and are listed in the PR as the only change outside `control_tables/` (contract section 1). |
| Python tools in a Rust workspace | The spec puts the check next to `analyze_tables.py`. An `xtask` subcommand would be xtask's file (no-go). | A Rust binary in `control_tables/` needs a workspace member and a `Cargo.toml` change (ci's file). |
