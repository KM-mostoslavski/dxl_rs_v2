# Implementation Plan: Control tables (verification flags, capabilities, per-protocol model map)

**Branch**: `005-tables` (planned on `dev`; the PR branches are created from `main` at
implementation) | **Date**: 2026-10-27, updated 2026-10-28 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/005-tables/spec.md`, the shared contract
`specs/ideas/module-interfaces.md` (sections 1, 2.4b, 3 and 7), constitution v1.3.0 and
`docs/srs/modules/tables.md`. Updated 2026-10-28 for the architect's decisions "limits follow the
operating mode" (module-interfaces §2.4b) and "the mode values live in the tables"
(`operating_modes`, module-interfaces §3.1).

## Summary

This feature changes data, not Rust code. It extends the 72 `control_tables/*.ron` files and
`dynamixel.ron` to the RON schema of module-interfaces section 3, and adds two maintainer tools
next to `analyze_tables.py`.

- **Every table** gets `source` (`Unverified`, or `Verified(url, verified)`), `protocols`, and
  `access` + `area` on every row. Verified tables also get `instructions`. All values come from
  the model's e-Manual page (`instructions` from the Protocol 1.0 / 2.0 pages).
- **Every table gives exactly one mode source** (architect, 2026-10-28; module-interfaces §2.4b):
  its "Operating Mode" row (X, MX(2.0), PRO, PRO+, RH-P12-RN, Y), XL-320's "Control Mode" row,
  or, with neither (AX, MX on Protocol 1), the CW/CCW Angle Limit pair. The rows exist today;
  T1 confirms each against its page and the check enforces the rule.
- **Every table with a mode row gets `operating_modes`** (architect, 2026-10-28;
  module-interfaces §3.1): `(bounded: [...], unbounded: [...], no_position: [...])`, the mode
  values copied from that model's page, three disjoint lists. Absent on the AX and MX Protocol 1
  tables. X1 derives `ModelData::mode` from the mode row and this field (codegen check 17);
  core holds no per-family value list (research R-15).
- **The map** gets `p1` / `p2` lists per model number, `default: P2` on dual-protocol models,
  `also` for other products sold under the same number, and a `not_mapped` record.
- **A table check** (`check_tables.py`, Python stdlib only) validates the schema, the map and the
  mode source with its `operating_modes`, lists unverified tables, and fails only on real
  errors. CI can run it.
- **An e-Manual helper** (`emanual.py`) reads a model page's control table and fills `access` and
  `area` only where address, size and name all match the page. It lists every mismatch for a
  human.
- **Priority models** are verified: AX-12A, AX-12+, MX-64(2.0), MX-64 on Protocol 1.
- **New tables**: MX-28, MX-64 and MX-106 on Protocol 1 firmware (models 29, 310, 320), with no
  "Operating Mode" row and no `operating_modes`: their mode comes from the CW/CCW Angle Limit
  pair.

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
- Every table keeps exactly one mode source at every commit, so X1 never meets a table it cannot
  derive `ModelData::mode` from. T1 adds or moves no row: a missing or misplaced mode row found
  on a page is a T2 correction (it changes a generated const), and that table's
  `operating_modes` comes with the row in T2. `operating_modes` changes no generated file in T1
  (today's codegen ignores it; research R-1).

**Scale/Scope**: 72 tables, 4,635 register rows (plus 269 `unit_info` entries), 63 model numbers
and 69 mapped files today. After T2: 75 tables, 66 model numbers, 72 mapped files, 3 not mapped.
About 28 dual-protocol model numbers (research R-4). Mode sources today: 66 `Operating Mode`
tables (address 11 in 54, address 33 in the 12 Y tables; size 1), 1 `Control Mode` (`xl320`,
address 11), 5 CW/CCW only (AX). After T2: + 3 CW/CCW only (MX P1). `operating_modes`: 0 tables
today, 67 after T1 (66 `Operating Mode` + `xl320`), still 67 after T2.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Check | Status |
|---|---|---|
| I. no_std and Heap-Free | No Rust code. Generated code is unchanged in T1; T2 only adds model types of the same shape. | N/A |
| II. Absent Over Failing | The per-protocol map lets X1 fail Setup with `UnsupportedModel` when a model has no table on the bus protocol (US4 AS4). Data only here. | PASS |
| III. Nothing Hidden | Unverified tables say so at their top, in the check report and (through X1) in the docs. No range or default is added, so nothing can become a limit fallback. Exactly one mode source per table, the rule of codegen's check 17, so X1 never chooses between two. A mode value in no `operating_modes` list fails Setup (`UnknownOperatingMode`); a value the page leaves unclear is raised, never put in a list by guess. | PASS |
| IV. e-Manual Is the Only Source | Every new value comes from the model's e-Manual page (instructions from the protocol pages). The helper fills a field only when address, size and name match the page; it never copies a sibling table. The mode row (or its absence) is confirmed on each page; the mode source is never chosen by family. `operating_modes` is copied from each model's own page, not from a family list. Unverified tables stay usable and flagged. Dual-protocol models get both lists with `default: P2`. | PASS, with one note: AX-12+ has no page of its own (research R-9) |
| V. Committed, Data-Driven Codegen | T1: no generated change, proven by prototype. T2: regenerated output committed in the same PR, CI diff check stays green. No hand edits. | PASS |
| VI. Typed, Static, Panic-Free API | No API change. `access`/`area` feed core's `Register` (contract 2.2). | N/A |
| VII. Developer Experience First | One flag per table, at the top. One command shows what is verified. Mismatches are listed, not hidden. | PASS |
| Workflow: one SRS module per PR | Two PRs (T1, T2), as contract section 7 schedules. | **DEVIATION**, see Complexity Tracking |
| Workflow: PR that verifies or changes a table cites URL and date | PR description lists, per changed table, its flag and every corrected or removed row (FR-017). The check prints that list. | PASS |
| Workflow: gates (fmt, clippy, test, no_std, codegen no-diff, coverage ≥ 70 %) | No Rust source changes; coverage is unchanged. Python tests are extra. | PASS |
| Governance: SRS and module copy agree | `docs/srs/modules/tables.md` was regenerated on 2026-10-27 and carries the FR-7.2 / NFR-9 amendment, as does constitution 1.3.0. Not yet in the SRS: the 2026-10-28 decisions (FR-3.7, limits follow the operating mode; FR-7.2, `operating_modes` in each table) and the stale roadmap line "AX-12+, AX-18F, XW430-T200 … Adding the number is enough" (the numbers are shared, research R-10). The architect amends both. Not edited here. | FLAGGED |

Post-design re-check (after Phase 1, repeated 2026-10-28 for the mode source and
`operating_modes`): unchanged. The
schema, the tools and the PR split keep every principle. The only exceptions are the two-PR split
and the SRS lines not yet amended, both listed in module-interfaces section 9.

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
├── tasks.md                 # /speckit-tasks
├── findings/                # implementation notes: one file per task that compares tables
│                            # with pages (t1-*.md, t2-*.md); never staged on T1/T2
└── pr-description-t1.md, pr-description-t2.md   # PR bodies, drafted here; never staged
```

**Working files.** The design docs, `findings/` and the two PR descriptions live in the main
checkout on `dev`. The T1 and T2 branches are cut from `main` in their own worktrees and stage
only `control_tables/**` (T2 also `src/models.rs`, `src/models/**`). Every task that diffs a table
against its page writes its own findings file (page URL, fetch date, rows filled by hand, rows not
on the page, `type_info` and mode-row findings), so parallel tasks never share a file. Each PR
description merges its PR's findings (FR-017).

### Source (repository root)

```text
control_tables/
├── *.ron                    # T1: + source, protocols; rows + access, area; type_info fixes;
│                            #     mode rows confirmed on each page; + operating_modes on the
│                            #     67 tables with a mode row (research R-15)
│                            # T1: AX-12A Verified + instructions; T2: MX-64(2.0) Verified,
│                            #     alias rows removed (research R-14), mode-row corrections
├── mx_28_p1.ron             # T2: new, Protocol 1 firmware, model 29, Unverified, no Operating Mode
├── mx_64_p1.ron             # T2: new, model 310, Verified + instructions, no Operating Mode
├── mx_106_p1.ron            # T2: new, model 320, Unverified, no Operating Mode
├── ax_12_plus.ron           # T1: not_mapped SameNumberAs(12); T2: Verified
├── dynamixel.ron            # T1: p1/p2/default beside legacy tables, not_mapped, header doc
│                            # T2: also, MX P1 entries, legacy tables removed
├── check_tables.py          # T1: new, the table check (FR-014, FR-020); T2: rejects `tables`
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
| **T1** | nothing (parallel with core-1) | Tools and tests, the mode-source and `operating_modes` rules included. `source: Unverified` + `protocols` on all 72 tables. `access` + `area` on all 4,635 rows from each model's page via `emanual.py`. Each table's `Operating Mode` / `Control Mode` row (or its absence) confirmed on its page, and `operating_modes` copied from that page into the 67 tables with a mode row (research R-15). `type_info` checked against each page; wrong values fixed (XL-320 range, X/MX(2.0)/2XL slope 2π/4096, AX angles moved to the zero; research R-8). Map: `p1`/`p2`/`default` beside legacy `tables`, `not_mapped` for `ax_12_plus`, `ax_18f`, `xw430_t200`. AX-12A verified (all 32 rows already match its page). Findings list (rows whose name or presence differs from the page). Analysis regenerated. | none (CI proves it) |
| **T2** | X1 merged | `mx_28_p1`, `mx_64_p1`, `mx_106_p1` (no `Operating Mode` row, no `operating_modes`) + map entries 29, 310, 320 (`p1` only). `also` on 12, 18, 1270. MX-64(2.0), AX-12+ and MX-64 P1 verified, mode rows and `operating_modes` included. Alias rows not on the e-Manual removed from 66 tables (research R-14). Other row corrections from the T1 findings applied, mode rows (each with its `operating_modes`) included. Name fix `RH-P12-RN(A)` (35074). Legacy `tables` removed, and from then on the check rejects any `tables` key in a model entry (contracts/ron-schema.md §2). Regenerated `src/models*` committed. Analysis regenerated. | yes, all explained (SC-004) |

T1 is large but mechanical. Its commits are split so review stays possible: tools, then flags and
protocols, then `access`/`area` per family, then `operating_modes` per family, then the map, then
verification, then the analysis.

## Deviations from module-interfaces.md

1. **XW430-T200 is `SameNumberAs(1270)`.** No longer a deviation: contract 3.2 now shows
   `(file: "xw430_t200", reason: SameNumberAs(1270))`. Both e-Manual pages give Model Number
   1,270 (XW430-T200 and XW430-T333, re-checked 2026-10-28) and both tables are identical, so US5
   AS4 records it under the existing entry (research R-10). Kept here so the numbers below, which
   other plans cite, stay stable.
2. **AX-18F is `SameNumberAs(18)`.** The contract's example does not list it. The AX-18A page
   covers "AX-18F, AX-18A" and gives Model Number 18. Same layout. Not a contract change, data
   only.
3. **`instructions` entries carry `source` and an optional `from_fw`.** Contract 3.1 says the shape
   is free until xtask reads it. Model pages do not list instructions; the protocol pages do,
   with firmware conditions (research R-5). Not a contract change.
4. **The check repeats every data-only codegen check** of contract 4.3 and spec 006
   (data-model §3, checks 2–17): roles and their access/area, `Firmware Version` present and at
   one location per (number, protocol), exactly one complete limit form, Model Number at (0, 2)
   R/EEPROM, P1 role address ≤ 255 (the mode register counts as a role), scale sanity,
   `default`, known `family`, file under two numbers, duplicate register constants and type
   identifiers, module identifier `tests` or a keyword, and the mode source (check 17: no
   source, an `Operating Mode` row beside the CW/CCW pair, both mode rows, a mode row without
   `operating_modes`, `operating_modes` without a mode row, or a value in two of its lists
   fail; check 11: the mode register is 1 byte, `RW`, `EEPROM`). The mapping is in
   contracts/table-tools.md. They are duplicated on purpose so T1 cannot land data that X1 will
   reject. Codegen stays the authority.
5. **The check has rules codegen does not.** A `Control Mode` row outside `xl320` fails
   (contract 4.3 gives this rule to the tables check; XL-320 is the only table with that
   register, so a second one is raised before it lands. Its values need no core change any more:
   they are in `operating_modes`). The mode source, `operating_modes` and the mode-row
   attributes are checked on every table, unmapped ones included (codegen reads only mapped
   tables). A table neither mapped nor in `not_mapped` is an error,
   not a warning (contract 4.3). In T1 the legacy `tables` must equal the default list; from T2
   on, a `tables` key is an error. None of these changes what X1 reads (research R-15).

No other deviation. Names, RON spellings (`R | RW`, `EEPROM | RAM`, `P1 | P2`, `Unverified`,
`Verified(url, verified)`, `NoModelNumber`, `SameNumberAs(u16)`,
`operating_modes: (bounded, unbounded, no_position)`), file ownership and order follow the
contract.

## Findings for other modules and the architect

- **xtask (X1)**: XL-320 `type_info` is wrong today (2048 / 4095 / ±π, the page says 0–1023 over
  0–300°). X/MX(2.0)/2XL tables give a slope of 2π/4095; the pages say 2π/4096. T1 fixes both
  before X1 starts reading `type_info`. After T1 the 4096-pulse tables read
  `PositionScale::new(2048, 0, 4095, -3.1415927, 3.1400587)`, and AX / XL-320
  `(512, 0, 1023, -2.6205530, 2.6154347)` (research R-8). X1's examples and tests use these
  values (xtask research R-6, contracts/generated-api.md). X1 must emit one model type per file
  even when the same file is under `p1` and `p2`. Mode source: `Operating Mode` sits at address 11
  in 54 tables and at 33 in the 12 Y tables, so X1 takes the register from the row, never a
  per-family constant, and the value lists from the table's `operating_modes`. After T1 every
  table passes check 17 (with `operating_modes`) and the mode-register attributes of check 11.
  Read tables with the `IMPLICIT_SOME` extension, as the map: `operating_modes` is an optional
  field, and without the extension `ron` 0.12.2 fails with "Expected option" (research R-1).
- **core**: the 2π/4095 slope flagged in core research R-7 is fixed in T1. Core's own test
  vectors that build the XM430 scale with ±π can stay (they test the type, not the table) or move
  to `3.1400587`. Decided 2026-10-28: core holds no per-family value list; it decodes a mode value
  with the `ModeValues { bounded, unbounded, no_position }` that X1 emits from each table's
  `operating_modes` (module-interfaces §2.4b). A core document that still shows the per-family
  `ModeValues { X, Pro, Y, RhP12Rn }` enum or a `ControlMode` variant predates the decision; that
  update is core's. Some pages give a shorter list than their family (core research R-17:
  XL430-W250 and 2XL430-W250 list 1, 3, 4, 16 only); T1 copies each page's own list.
- **ci**: resolved. ci (specs/007-ci contracts) has a `control tables: schema check` job that runs
  `python3 control_tables/check_tables.py` and
  `python3 -m unittest discover -s control_tables -p 'test_*.py'`. It is guarded by
  `if [ -f control_tables/check_tables.py ]` until T1 merges (research R-12).
- **Architect**: XW430-T200 / T333 share number 1270 on the e-Manual (confirmed 2026-10-28). AX-12+
  has no page of its own; its record cites the AX-12A page, which names AX-12+ (research R-9).
  RH-P12-RN entry 35074 is "RH-P12-RN(A)" on the e-Manual (35073 is the older RH-P12-RN, not in this
  repo). SRS FR-3.7 (limits follow the operating mode), FR-7.2 (`operating_modes` in each table) and
  the roadmap line on AX-12+, AX-18F, XW430-T200 need the 2026-10-28 amendment (Constitution Check).
  CLAUDE.md's command list could add the check (not edited here).

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| Two PRs for one module (T1, T2) | T2's new tables and the removal of legacy `tables` need X1's per-protocol codegen. T1 must land before X1 so X1 has data to read. | One PR after X1: X1 would have no `source`, `protocols`, `access`, `area`, `operating_modes` to read and would fail its own checks. One PR before X1: today's codegen cannot map one number to two protocols. Approval asked in module-interfaces section 9. |
| T2 commits `src/models.rs` and `src/models/**` (no-go `src/**`) | CI fails on a stale regeneration diff; new tables must be generated. | Leaving them out breaks CI. The files are tool output only and are listed in the PR as the only change outside `control_tables/` (contract section 1). |
| Python tools in a Rust workspace | The spec puts the check next to `analyze_tables.py`. An `xtask` subcommand would be xtask's file (no-go). | A Rust binary in `control_tables/` needs a workspace member and a `Cargo.toml` change (ci's file). |
