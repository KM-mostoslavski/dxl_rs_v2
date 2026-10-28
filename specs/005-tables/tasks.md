---

description: "Task list for the tables module (verification flags, capabilities, per-protocol model map)"
---

# Tasks: Control tables (verification flags, capabilities, per-protocol model map)

**Input**: Design documents from `specs/005-tables/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/ron-schema.md,
contracts/table-tools.md, quickstart.md; shared contract `specs/ideas/module-interfaces.md`
(sections 1, 2.4b, 3, 4.3, 7). Updated 2026-10-28 for the mode source (architect decision
"limits follow the operating mode", FR-020) and for `operating_modes` (architect decision "the
mode values live in the tables", module-interfaces §3.1).

**Tests**: Required (constitution: every acceptance scenario is checked). For this data module
the tests are `control_tables/test_check_tables.py` and `control_tables/test_emanual.py`
(`python3 -m unittest discover -s control_tables -p 'test_*.py'`), the table check
`python3 control_tables/check_tables.py` on the real data, and the codegen no-diff gate. In every
story, the test/check tasks come before the data-change tasks and name the acceptance scenarios
they cover.

**Organization**: The module ships as **two PRs** (plan "PR split", module-interfaces §7):
**T1** (no dependency, no generated change) and **T2** (after xtask X1, commits regenerated
`src/models*`). Task IDs run in execution order, so the document is split into Part A (PR T1)
and Part B (PR T2). A user story that has work in both PRs has one phase in each part, labelled
with its slice; every phase header names its PR.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1 = verification flag, US2 = priority models verified, US3 = capability data
  (protocols, access, area, mode source, `operating_modes`, analysis), US4 = per-protocol model map,
  US5 = missing tables and model numbers
- **[human]**: a task only the architect or maintainer may do (files in `.aiignore`, approvals);
  an agent never does it

## Rules for every task

- **Worktrees.** T1 work happens in a git worktree `../dxl_rs_v2-005-t1` (branch
  `005-tables-t1` off `origin/main`); T2 work in `../dxl_rs_v2-005-t2` (branch `005-tables-t2`
  off `origin/main`). Paths `control_tables/…` and `src/…` below are relative to that worktree.
  The design docs and working notes (`specs/005-tables/…`, including `specs/005-tables/findings/`
  and `specs/005-tables/pr-description-t1.md` / `pr-description-t2.md`; plan "Working files")
  live in the main checkout on `dev` and are **never staged on the T1/T2 branches**.
- **Paths.** Agents change only `control_tables/**`. T1 additionally changes, for RX-28 only (T023),
  `Cargo.toml` (the `rx` feature) and the regenerated `src/models.rs` and `src/models/rx_28.rs`.
  T2 additionally commits `src/models.rs` and
  `src/models/**`, produced **only** by `cargo xtask codegen`, never hand-edited (constitution V).
  No `Cargo.toml`, `xtask/**`, `src/**` source or `.github/**` change.
- **Human-only files** (`.aiignore`): `.aiignore`, `.claude/settings.json`,
  `.github/workflows/ov-*.yml`, `.github/ov-ruleset-main.json`, `.specify/extensions/ov/**`,
  `docs/srs/SRS.md`. Any change needed there is a `[human]` task. Agents also do not edit
  `docs/ARCHITECTURE.md`, `docs/srs/modules/*.md`, the constitution or `CLAUDE.md` (architect's).
- **Data source (constitution IV, FR-003).** Every value written into a table or the map (name,
  address, size, `access`, `area`, `protocols`, `instructions`, `operating_modes`, `type_info`,
  `unit_info`, model number, firmware boundary) comes from the model's English page on
  `emanual.robotis.com` (instructions from the Protocol 1.0 / 2.0 pages), fetched at the time of the
  task. Never from memory, a third-party SDK, the ROBOTIS GitHub sources or another model's table. A
  value the page does not give is left out. If the e-Manual is unreachable, the task stops: nothing
  is written. Every task that writes register data cites its page URL in the findings file it
  writes.
- **Verification is deferred.** Only AX-12A and RX-28 (T1), MX-64(2.0), AX-12+ and MX-64 on Protocol 1 (T2)
  are verified. Every other table is and stays `source: Unverified`, even when rows are fixed.
- **Mode source (FR-020, research R-15).** Every table keeps exactly one: its `Operating Mode`
  row, `xl320`'s `Control Mode` row, or, with neither, the CW/CCW Angle Limit pair. Each task
  that reads a page confirms the mode row (or its absence) there. T1 adds, moves or removes no
  mode row; a mismatch is a finding, fixed in T2 Phase 11. A table has `operating_modes` exactly
  when it has a mode row, its values copied from its own page (research R-15); a row added in T2
  comes with its `operating_modes`.
- **RON style.** Brace style stays (`{ key: value }`, contract 3.1). Rows keep their file's style
  (one-line or multi-line). A multi-line row keeps the field order
  `address, size, data_name, access, area`, one per line.
- **Python.** 3.10 or later, standard library only (`re`, `datetime`, `html.parser`,
  `urllib.request`, `argparse`, `unittest`, `tempfile`). No `pip`. No network in tests.
- **Findings.** Every task that diffs a table against its page writes what it found (rows filled
  by hand, alias rows, rows not on the page, `type_info` changes, the page URL and fetch date) to
  its own file under `specs/005-tables/findings/` (one file per task, so parallel tasks never
  share a file). The PR description tasks merge them (FR-017).
- **Commits.** No `Co-Authored-By`, `Assisted-by` or "Generated with" lines (constitution
  workflow). PRs authored by Ovsiankina, into `main`, from `.github/pull_request_template.md`.

---

# Part A: PR T1 (`005-tables-t1`, no dependency)

## Phase 1: Setup (PR T1)

**Purpose**: Worktree, baseline and tool skeletons

- [ ] T001 Check the main checkout for a dirty tree (`git status`) and unmerged work, then `git fetch origin` and create the T1 worktree: `git worktree add ../dxl_rs_v2-005-t1 -b 005-tables-t1 origin/main`. Confirm `control_tables/` and `xtask/` on `origin/main` equal `dev` (`git diff --stat origin/main dev -- control_tables xtask` is empty); report any difference before going on
- [ ] T002 Record the baseline in the T1 worktree: `python3 --version` is ≥ 3.10; `cargo xtask codegen && git diff --exit-code src/models.rs src/models/` is empty and prints exactly three skip warnings (`ax_12_plus`, `ax_18f`, `xw430_t200`); `control_tables/` holds 72 tables plus `dynamixel.ron`, the map has 63 model numbers and 69 mapped files; mode rows (research R-15): 66 tables with one `Operating Mode` row (address 11 in 54, 33 in the 12 `ym*` tables, size 1), `xl320` with one `Control Mode` row, the 5 `ax_*` tables with neither and the CW/CCW Angle Limit pair (`grep -c '"Operating Mode"\|"Control Mode"' control_tables/*.ron`); no table has `operating_modes` (`grep -l operating_modes control_tables/*.ron` is empty). Write the counts and warnings to `specs/005-tables/findings/t1-baseline.md`
- [ ] T003 [P] Create `control_tables/check_tables.py` (executable, `#!/usr/bin/env python3`): module docstring pointing to `specs/005-tables/contracts/table-tools.md` §1; `argparse` CLI exactly `check_tables.py [--dir DIR] [--today YYYY-MM-DD] [--markdown] [STEM ...]` (`--dir` defaults to the script's directory, `--today` to `datetime.date.today()`); `main(argv=None) -> int`; `if __name__ == "__main__": sys.exit(main())`. A bad argument (including a `--today` that is not ISO `YYYY-MM-DD`) exits 2
- [ ] T004 [P] Create `control_tables/emanual.py` (executable, `#!/usr/bin/env python3`): module docstring pointing to contracts/table-tools.md §2; `argparse` subcommands `rows URL`, `diff STEM [URL]`, `fill STEM [URL]`; an empty module-level `PAGES: dict[str, str] = {}` (stem → page URL); any URL not starting with `https://emanual.robotis.com/` is refused with exit 2; `main(argv=None) -> int`
- [ ] T005 Create `control_tables/test_check_tables.py` and `control_tables/test_emanual.py` with `unittest` scaffolding: a helper that writes a fixture directory (a `dynamixel.ron` plus named `.ron` tables) into `tempfile.TemporaryDirectory`, and a helper that calls `check_tables.main([...])` capturing stdout, stderr and the exit code. `python3 -m unittest discover -s control_tables -p 'test_*.py'` runs (zero tests is fine). Depends on T003, T004

---

## Phase 2: Foundational (PR T1, blocking prerequisites)

**Purpose**: The RON-subset parser, the report frame of the check, and the e-Manual helper with
its page list. US2 and US3 both need `emanual.py`; every story needs the parser.

**⚠️ CRITICAL**: No user-story work starts until this phase is complete

- [ ] T006 [P] Parser tests in `control_tables/test_check_tables.py` (class `ParserTests`), research R-6: parse `( … )` and `{ key: value }` records as dicts; `{ INT: value }` maps with integer keys; lists with trailing commas; strings with escapes and non-ASCII (`"N·m/bit"`); ints and negative floats (`-3.1415927`); `//` comments anywhere; bare identifiers (`Unverified`, `P1`, `RW`, `EEPROM`) as an `Ident` value; tagged values `Verified(url: "…", verified: "2026-10-28")` and `SameNumberAs(12)` as a `Tagged(name, args)` value; one-line rows (as in `ax_18a.ron`) and multi-line rows (as in `mx_64.ron`). A **duplicate map key** (`{ 1020: …, 1020: … }`) raises a parse error naming the file and the key (FR-012); an unterminated string raises a parse error naming file and line. Through the CLI both give exit 2 (quickstart §2 FR-012 row)
- [ ] T007 [P] e-Manual helper tests in `control_tables/test_emanual.py`, no network (patch the fetch function): three hand-written HTML snippets shaped like the AX page (columns Address, Size, Data Name, Description, Access, Initial Value, Min, Max), the X page (Address, Size(Byte), Data Name, Access, Initial Value, Range, Unit) and the PRO+ page (an extra Modbus address column), each with tables under headings "Control Table of EEPROM Area" and "Control Table of RAM Area". Assert: `parse_page(html)` returns `(address, size, name, access, area)` in page order; an Access value other than `R` or `RW` (e.g. `W`) is an error (exit 1, nothing written); a row not under an EEPROM or RAM heading is an error (exit 1, nothing written); `diff` reports the four categories of the contract (only in table, only on page, different name at the same address and size, different existing `access`/`area`) and exits 1 when any exist, 0 otherwise; `fill` on a one-line fixture and on a multi-line fixture adds `access` and `area` only where address, size and name all match one page row and the row has neither field yet, keeps the row style and field order `address, size, data_name, access, area`, never changes any other field, lists every row it left (e.g. an alias row "Indirect Address Write" at the address of "Indirect Address 1"), and the written file re-parses with the check's parser; `rows https://example.com/` exits 2
- [ ] T008 Implement the parser in `control_tables/check_tables.py` until T006 passes: a tokenizer plus recursive parser for the RON subset of research R-6, returning plain Python values (`dict`, `list`, `str`, `int`, `float`, `Ident`, `Tagged`), rejecting duplicate keys in any map or record, raising `RonError(file, line, col, message)`. Brace records must parse as records (xtask reads braces as structs; no real map appears in a table). Depends on T006
- [ ] T009 Report-frame tests in `control_tables/test_check_tables.py` (class `CliTests`): a clean fixture exits 0; a fixture without `dynamixel.ron` exits 2; a file that does not parse exits 2 and stderr names it; stdout has the sections `tables`, `dual-protocol models`, `errors`, `summary` in that order (contract table-tools §1 sample); every error line is printed under `errors` and also one per line on stderr; `STEM ...` limits only the per-table section (an error in another file still gives exit 1); `--markdown` prints the per-table lines as a Markdown list. Depends on T008
- [ ] T010 Implement the report frame in `control_tables/check_tables.py` until T009 passes: load every `*.ron` in `--dir` except `dynamixel.ron` and the map; collect errors as `(file, location, message)`; print the per-table line `stem  flag  protocols  mapping`, the dual-protocol section, errors and the summary line `summary: N tables, N verified, N unverified, N mapped, N not mapped, N dual-protocol, N errors`; exit 0 when no error, 1 on any error, 2 on bad arguments, a parse error or a missing `dynamixel.ron` (contract exit-code table). Network: none. Depends on T009
- [ ] T011 [P] Implement `control_tables/emanual.py` until T007 passes (contract table-tools §2, research R-7): fetch with `urllib.request` (timeout, a `User-Agent`, exit 2 on network error or 404); `html.parser` finds every table whose header row has Address, Size, Data Name and Access (match header names, not column positions), takes the area from the nearest preceding heading containing "EEPROM" or "RAM"; `rows` prints the rows in RON row style; `diff` and `fill` read the table with the parser from `check_tables.py` (import it) and edit the `.ron` text in place, re-parsing it before writing. Rules quoted from the contract: "It adds `access` and `area` only to a row whose address, size and name all match one page row, and only when the row has neither field yet. It never changes an address, size, name or any other field." "It stops with exit code 1, writing nothing, when the page has an access value other than `R` or `RW`, or a row it cannot place under an EEPROM or RAM heading." Depends on T008 (parallel with T009–T010)
- [ ] T012 Fill `PAGES` in `control_tables/emanual.py`: one `"stem": "https://emanual.robotis.com/…"` line for each of the 72 tables and for `mx_28_p1`, `mx_64_p1`, `mx_106_p1`, found by navigating the e-Manual DYNAMIXEL index (`https://emanual.robotis.com/docs/en/dxl/`), each opened once to confirm it loads and names the model. Known from research R-9/R-10/R-11: `ax_12a` and `ax_12_plus` → `https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/` (headed "AX-12+, AX-12A"; `…/ax-12+/` is a 404); `ax_18f` → the AX-18A page (covers "AX-18F, AX-18A"); `mx_64` → `https://emanual.robotis.com/docs/en/dxl/mx/mx-64-2/`; `mx_64_p1` → `https://emanual.robotis.com/docs/en/dxl/mx/mx-64/`; `xw430_t200` → `https://emanual.robotis.com/docs/en/dxl/x/xw430-t200/`; each `_fw52` stem → its model's page. Write the list with any page not found to `specs/005-tables/findings/t1-pages.md`. Depends on T011

**Checkpoint**: `python3 -m unittest discover -s control_tables -p 'test_*.py'` passes;
`python3 control_tables/check_tables.py` parses all 73 files and exits 0 (no rule yet);
`python3 control_tables/emanual.py rows https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/`
prints 32 rows.

---

## Phase 3: User Story 1 - See at a glance whether a table was checked (Priority: P1) — PR T1 🎯 MVP

**Goal**: Every table carries `source: Unverified` or `source: Verified(url, verified)` at its
top; the check lists unverified tables, fails on a missing or invalid flag, and codegen output is
unchanged.

**Independent Test**: `python3 control_tables/check_tables.py` lists 72 tables with their flag
and exits 0; a scratch copy with one `source` line removed exits 1 naming that file;
`cargo xtask codegen && git diff --exit-code src/models.rs src/models/` is empty.

### Tests for User Story 1 ⚠️

- [ ] T013 [US1] Flag tests in `control_tables/test_check_tables.py` (class `SourceTests`), covering US1 AS1–AS4, FR-001, FR-002 and the data-model rule "`url` starts with `https://emanual.robotis.com/`. `verified` is a real calendar date not after the day the check runs.": no `source` key → exit 1, the error names the file (AS2); `source: Unverified` → listed as `unverified`, exit 0 (AS3); `Verified(url: "https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/", verified: "2026-10-28")` with `--today 2026-10-28` → listed `verified <url> 2026-10-28`, exit 0 (AS1); URL on another host, and `http://emanual.robotis.com/…` → exit 1 (AS4); `verified: "2026-02-30"` → exit 1; `verified: "2026-10-29"` with `--today 2026-10-28` → exit 1; `verified: "28.10.2026"` → exit 1; any other tag (`source: Checked`) → exit 1; `--markdown` prints one list item per table with the flag, URL and date

### Implementation for User Story 1

- [ ] T014 [US1] Implement the `source` rules and the flag column of the per-table line (and the `--markdown` form) in `control_tables/check_tables.py` until T013 passes. Use `datetime.date.fromisoformat` plus a strict `^\d{4}-\d{2}-\d{2}$` match, and compare with `--today`
- [ ] T015 [US1] Insert `source: Unverified,` as the first field of the top-level tuple of all 72 tables `control_tables/*.ron` (every file except `dynamixel.ron`), using each file's own indentation (2 spaces in X/MX/PRO/Y/RH files, 4 in AX files). A short throwaway script in the scratchpad is fine; it must not reformat anything else (`git diff --stat` shows +1 line per file). No table is marked `Verified` here (FR-013: unverified tables stay mapped)
- [ ] T016 [US1] Validate US1 (quickstart §2 US1 rows): `python3 control_tables/check_tables.py` exits 0 and lists 72 tables as `unverified` (AS3, SC-001); `head -3 control_tables/*.ron` shows the flag at the top of every table (AS1, SC-005); in a scratch copy run with `--dir`, delete one `source` line → exit 1 naming it (AS2), then set a URL on another host, a date `2026-02-30`, a future date → exit 1 each (AS4); `cargo xtask codegen && git diff --exit-code src/models.rs src/models/` is empty (AS5, FR-016). Commit "tables: verification flag" (data) separately from the tool commit

**Checkpoint**: US1 is complete on its own: every table is flagged and the check enforces it.

---

## Phase 4: User Story 2, T1 slice - AX-12A and RX-28 verified (Priority: P1) — PR T1

**Goal**: AX-12A carries `Verified(url, date)`, `protocols`, `instructions` and `access`/`area`
on all 32 rows, every value checked on its page. RX-28 is written new from its page, verified,
mapped under 28 and compiled in behind a new `rx` feature (architect 2026-10-28). (AX-12+,
MX-64(2.0) and MX-64 P1 follow in PR T2, Phase 10.)

**Independent Test**: The check reports `ax_12a  verified https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/ <date>  [P1]`;
`python3 control_tables/emanual.py diff ax_12a` reports no difference; 5 random rows match the page by eye.

### Tests for User Story 2 (T1 slice) ⚠️

- [ ] T017 [US2] Instruction-entry tests in `control_tables/test_check_tables.py` (class `InstructionTests`), covering US2 AS1 (each verified table carries its own record) and FR-006, rules quoted from data-model "Instruction entry": `instructions` on an `Unverified` table → exit 1 ("verified tables only"); a `Verified` table without at least one entry per protocol in `protocols` → exit 1; `protocol` "must be in the table's `protocols`"; entry `source` not on `https://emanual.robotis.com/` → exit 1; `list` "non-empty list of instruction names", "no duplicates within one entry"; a name outside `Ping, Read, Write, RegWrite, Action, FactoryReset, Reboot, Clear, ControlTableBackup, SyncRead, SyncWrite, FastSyncRead, BulkRead, BulkWrite, FastBulkRead` → exit 1; "`P1` entries may use only `Ping, Read, Write, RegWrite, Action, FactoryReset, Reboot, SyncWrite, BulkRead`" (a P1 entry with `SyncRead` → exit 1); "Two entries of one protocol must differ in `from_fw`" (two P1 entries both without `from_fw` → exit 1; `from_fw: 40` beside one without → exit 0)

### Implementation for User Story 2 (T1 slice)

- [ ] T018 [US2] Implement the `instructions` rules in `control_tables/check_tables.py` until T017 passes
- [ ] T019 [US2] Rows of `control_tables/ax_12a.ron` from `https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/`: run `python3 control_tables/emanual.py diff ax_12a`. Research R-9 expects all 32 rows to match (address, size, name). If any address, size or name differs in a way that changes a generated register constant, stop: AX-12A cannot be verified in T1 (plan R-9 rule "a verification lands in T1 only if it changes nothing codegen reads today"); leave it `Unverified`, fill only `access`/`area`, and record that its verification moves to T2. A name differing only in case may be fixed here. Then `python3 control_tables/emanual.py fill ax_12a`: every row gets `access` (`R` or `RW`) and `area` (`EEPROM` or `RAM`) from the page. Write the result to `specs/005-tables/findings/t1-ax_12a.md`
- [ ] T020 [US2] Model data of `control_tables/ax_12a.ron`, each value checked by hand on `https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/`: (a) `protocols: [P1]` after `source` (no Protocol Type register; the page states Protocol 1.0, research R-3); (b) `type_info`: zero / min / max `512 / 0 / 1023`, `min_radian: -2.6205530`, `max_radian: 2.6154347` when the page gives 0 ~ 1023 over 0 ~ 300° (research R-8: `angle = (raw − zero) × rad_per_tick`, −512 and +511 ticks at 300°/1023); if the page gives another slope, follow the page and record it; (c) each `unit_info` entry (`Position 0.29 deg`, `Speed 0.111 rpm`, `Torque 0.001465 N·m/bit`): keep it only if the page gives that value, else remove it and record the removal (FR-003); (d) `instructions: [(protocol: P1, source: "https://emanual.robotis.com/docs/en/dxl/protocol1/", list: [...])]` with every instruction whose product condition on the Protocol 1.0 page includes AX-12A (research R-5 expects `[Ping, Read, Write, RegWrite, Action, FactoryReset, SyncWrite]`, no `Reboot`, no `BulkRead`; the page decides); (e) mode source (FR-020, research R-15): confirm the page lists no `Operating Mode` or `Control Mode` register, so the table carries neither row and no `operating_modes`, and its mode comes from the `CW Angle Limit` / `CCW Angle Limit` rows (checked with all rows in T019); if the page lists one, stop and raise it (verification moves to T2, where the row can be added); (f) only then `source: Verified(url: "https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/", verified: "<YYYY-MM-DD of this check>")`. Append every change to `specs/005-tables/findings/t1-ax_12a.md`. Skip (f) if T019 or (e) stopped
- [ ] T021 [US2] Validate the T1 slice: `python3 control_tables/check_tables.py` exits 0 and lists `ax_12a` as verified with the URL and date (SC-002); `python3 control_tables/emanual.py diff ax_12a` exits 0; audit 5 random rows by eye on the page (address, size, name, access, area; SC-002) and note them in `specs/005-tables/findings/t1-ax_12a.md`; `python3 control_tables/check_tables.py --markdown ax_12a` output saved there for the PR (FR-017); `cargo xtask codegen && git diff --exit-code src/models.rs src/models/` is empty (`type_info` is not read by today's codegen)

### RX-28 (architect 2026-10-28: the test robot has 10 RX-28 on Protocol 1)

- [ ] T022 [US2] Create `control_tables/rx_28.ron` (RX-28, Protocol 1 only) from `https://emanual.robotis.com/docs/en/dxl/rx/rx-28/` only: add `"rx_28": "https://emanual.robotis.com/docs/en/dxl/rx/rx-28/"` to `PAGES` in `control_tables/emanual.py`; take the rows from `python3 control_tables/emanual.py rows https://emanual.robotis.com/docs/en/dxl/rx/rx-28/` with `address`, `size`, `data_name`, `access` and `area`, in `ax_12a.ron`'s one-line row style, and check every row by hand on the page. Model data as T020 (a)–(f), on the RX-28 page: (a) `protocols: [P1]`; (b) `type_info` from the page's position range and resolution (research R-8 formula; record the slope); (c) `unit_info` only with values the page gives; (d) `instructions` with every instruction whose product condition on `https://emanual.robotis.com/docs/en/dxl/protocol1/` includes RX-28 (the page decides); (e) mode source: confirm the page lists no `Operating Mode` or `Control Mode` register and does list `CW Angle Limit` / `CCW Angle Limit`, so the table carries neither mode row and no `operating_modes`; (f) only then `source: Verified(url: "https://emanual.robotis.com/docs/en/dxl/rx/rx-28/", verified: "<YYYY-MM-DD of this check>")`. The table keeps the top-level fields of `ax_12a.ron` so the existing xtask reads it. Write the page URL, fetch date and every value to `specs/005-tables/findings/t1-rx_28.md`. If the page is unreachable, stop: nothing is written
- [ ] T023 [US2] Map RX-28 and compile it in (architect 2026-10-28; the only generated change in T1, flagged in the PR's Architecture section): add `28: (family: "rx", name: "RX-28", tables: [(from_fw: 0, file: "rx_28")])` to `control_tables/dynamixel.ron` after the `ax` entries, with the model number and name read on the RX-28 page (Model Number default; the robot reports 28 too); add `rx = []` to the model families in `Cargo.toml` and `"rx"` to `default` after `"mx"`; run `cargo xtask codegen` and stage the regenerated `src/models.rs` and the new `src/models/rx_28.rs` (written only by codegen, constitution V). Any generated change other than the RX-28 model → stop and report
- [ ] T024 [US2] Validate RX-28: `python3 control_tables/check_tables.py` exits 0 and lists `rx_28` as verified with its URL and date; `python3 control_tables/emanual.py diff rx_28` exits 0; `cargo xtask codegen && git diff --exit-code src/models.rs src/models/` is empty once T023's files are staged; `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build --no-default-features --features std` (RX off) and `cargo clippy --no-default-features --features ax,mx,rx,x-series,y-series,pro --target thumbv7em-none-eabihf -- -D warnings` pass; 5 random rows match the page by eye

**Checkpoint**: one verified table proves the whole verify workflow (quickstart §5).

---

## Phase 5: User Story 3 - Know what a model can do (Priority: P2) — PR T1

**Goal**: Every table records `protocols` from its page and `access` + `area` on every row;
`type_info` matches each page; every table gives exactly one mode source, its mode row (or its
absence) confirmed on the page and its `operating_modes` copied from the page; the analysis takes
protocols from the data.

**Independent Test**: The check passes (it fails on a row without `access` or `area`, on a table
without exactly one mode source, and on a mode row without `operating_modes`) and shows a protocols
column on every table; `python3 control_tables/analyze_tables.py` puts XL-320 under P2 and its
grouping equals the `protocols` fields.

### Tests for User Story 3 ⚠️

- [ ] T025 [US3] Tests in `control_tables/test_check_tables.py` (classes `ProtocolsTests`, `RowTests`, `RoleTests`), covering US3 AS1–AS3, FR-005, FR-007 and xtask checks 9–12, 15, 16 (contract table-tools mapping): `protocols` missing, empty, duplicated (`[P1, P1]`) or holding `P3` → exit 1; a row without `access` or without `area` → exit 1 with the contract's error form (`xl320.ron: row 30 "Goal Position": missing \`area\``); `access: W` → exit 1 ("Any other `access` value (e.g. `W`) stops the work and is raised"); `area: ROM` → exit 1. For a mapped table (mapped = named in any `p1`, `p2` or legacy `tables` list): a missing role (`Torque Enable`, `Goal Position`, `Present Position`, `Firmware Version`) → exit 1; "exactly one complete limit pair, `Min/Max Position Limit` or `CW/CCW Angle Limit`" (both forms → 1; `CW Angle Limit` alone → 1); role attributes "`Torque Enable` RW/RAM, `Goal Position` RW/RAM, `Present Position` R/RAM … limit pair … RW/EEPROM" (each wrong one → 1); "`Model Number` at address 0, size 2, R/EEPROM" (`access: RW` → 1, `area: RAM` → 1, address 4 → 1); scale sanity "`max_raw > min_raw`, `max_radian > min_radian`, zero within `min_raw..=max_raw`" (each broken → 1); two rows whose names give one register constant (`Goal Position` and `Goal-Position`; constant = name split on non-alphanumerics, upper-cased, joined with `_`) → 1, while two rows on one address with different constants → 0; an unmapped table is not role-checked
- [ ] T026 [US3] Mode-source tests in `control_tables/test_check_tables.py` (class `ModeSourceTests`), covering US3 AS5, AS7, US5 AS6, FR-020, SC-008, xtask checks 11, 14 and 17, and the rules quoted from contracts/ron-schema.md §1 "Mode source" (for every table, mapped or not): "Exactly one source. Two rows of one mode name, both names in one table, an `Operating Mode` row in a table with CW/CCW Angle Limit rows, or neither name without the CW/CCW pair is an error." and "A mode row without `operating_modes`, `operating_modes` without a mode row, or a value in two of its lists is an error. A value outside 0–255 is an error." First make the fixture helper of T005 write `operating_modes` on every fixture table with a mode row unless a test opts out, so the fixtures of T009, T013, T017, T025 and later tasks stay valid. Fixtures: an X-shaped table (Min/Max form, one `Operating Mode` at 11, size 1, `RW`, `EEPROM`, `operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16])`) → 0; a Y-shaped one (`Operating Mode` at 33, `(bounded: [3], unbounded: [], no_position: [0, 1])`) → 0; an XL-320-shaped table named `xl320` (`Control Mode` plus the CW/CCW pair, `(bounded: [2], unbounded: [], no_position: [1])`) → 0; an AX- or MX-P1-shaped table (CW/CCW pair, no mode row, no `operating_modes`) → 0; a Min/Max table without `Operating Mode` → 1 ("no mode source", naming the file); `Operating Mode` and `Control Mode` in one table → 1; an AX-shaped table plus an `Operating Mode` row → 1; two `Operating Mode` rows in an unmapped table → 1; "`Control Mode` is accepted only in `xl320.ron`" (the same rows in a table named `xl330_x`) → 1; "The mode row has `size: 1`, `access: RW` and `area: EEPROM`" (size 2 → 1; `access: R` → 1; `area: RAM` → 1); a table under `p1` with `Operating Mode` at address 300 → 1 (the mode register counts as a role). `operating_modes` (US3 AS7): the X-shaped table without it → 1 naming the file; the AX-shaped table with `(bounded: [3], unbounded: [], no_position: [])` → 1; 4 in both `bounded` and `unbounded` → 1, the error naming the value and both lists; a missing `no_position` key → 1; a value `256` → 1; a value `-1` → 1; `bounded: 3` (not a list) → 1; an unmapped table with an `Operating Mode` row and no `operating_modes` → 1; all three lists empty on an X-shaped table → 0 (no rule asks for a value; the page decides)

### Implementation for User Story 3

- [ ] T027 [US3] Implement the `protocols`, row, role, mode-source and `operating_modes` rules in `control_tables/check_tables.py` until T025 and T026 pass, and add the protocols column (`[P1]`, `[P2]`, `[P1, P2]`) to the per-table line. The mode-source and `operating_modes` rules run on every table, mapped or not; the role rules only on mapped tables. On the real data the `operating_modes` rule fails on the 67 tables with a mode row until T036–T041 write the field; T042 checks it
- [ ] T028 [P] [US3] AX group, files `control_tables/ax_12_plus.ron`, `control_tables/ax_12w.ron`, `control_tables/ax_18a.ron`, `control_tables/ax_18f.ron` (one-line rows; `ax_12a` is done in T019–T020). For each stem: `python3 control_tables/emanual.py diff <stem>`, then `fill <stem>`; resolve every row `fill` left by hand from the page row at the same address and size (data-model: `access` "`R` or `RW`", `area` "`EEPROM` or `RAM`"; any other access value stops the task and is raised); add `protocols: [P1]` after `source` only if the page states Protocol 1.0 and has no Protocol Type register (research R-3); set `type_info` from the page (research R-8 expects `512 / 0 / 1023`, `min_radian: -2.6205530`, `max_radian: 2.6154347` for 0 ~ 1023 over 0 ~ 300°); mode row (FR-020, research R-15): where the table has an `Operating Mode` or `Control Mode` row, confirm the page lists that register at the same address and size (its `access`/`area` come from the page like any row); a page that lists a mode register the table lacks or places elsewhere, or a table mode row the page does not list, is a mode-row finding for T2 Phase 11, not fixed here. AX expected: no mode register on these pages; the mode comes from the CW/CCW Angle Limit pair, and the tables get no `operating_modes`. Do not add `instructions`; keep `source: Unverified`; change no address, size or name (T1 changes nothing codegen reads). Pages from `PAGES` (`ax_12_plus` → AX-12A page, `ax_18f` → AX-18A page). Write every manual fill, row not on the page, name mismatch, mode-row finding and `type_info` change with URL and date to `specs/005-tables/findings/t1-ax.md`
- [ ] T029 [P] [US3] MX(2.0) + XL-320 group, files `control_tables/mx_28.ron`, `control_tables/mx_64.ron`, `control_tables/mx_106.ron`, `control_tables/xl320.ron`. Same procedure as T028. Expected from research (confirm per page): MX(2.0) Protocol Type values 1 ~ 2 → `protocols: [P1, P2]`; XL-320 has no Protocol Type register and refers to Protocol 2.0 only → `protocols: [P2]`. The four MX alias rows "Indirect Address Write", "Indirect Data Write" (same address and size as "Indirect Address 1" / "Indirect Data 1") and "Indirect Address Read", "Indirect Data Read" (at the address of "Indirect Address 29" / "Indirect Data 29") are not on the pages (research R-14): give them `access`/`area` of the page row at the same address and size and list them; do not remove them (removal is T2). `type_info`: MX(2.0) pages give 4096 pulses per turn → `2048 / 0 / 4095`, `min_radian: -3.1415927`, `max_radian: 3.1400587`; XL-320 page gives Goal Position 0 ~ 1023 over 0 ~ 300° → `512 / 0 / 1023`, `-2.6205530` / `2.6154347` (research R-8; follow the page if it differs). Mode rows as in T028: MX(2.0) `Operating Mode`, XL-320 `Control Mode` (both at address 11, size 1 in today's tables), each confirmed on its page (their `operating_modes` follow in T036). Findings to `specs/005-tables/findings/t1-mx-xl320.md`
- [ ] T030 [P] [US3] X430 group, files `control_tables/2xc430_w250.ron`, `control_tables/2xl430_w250.ron`, `control_tables/xc430_w150.ron`, `control_tables/xc430_w240.ron`, `control_tables/xd430_t210.ron`, `control_tables/xd430_t350.ron`, `control_tables/xh430_v210.ron`, `control_tables/xh430_v350.ron`, `control_tables/xh430_w210.ron`, `control_tables/xh430_w350.ron`, `control_tables/xl430_w250.ron`, `control_tables/xm430_w210.ron`, `control_tables/xm430_w350.ron`, `control_tables/xw430_t200.ron`, `control_tables/xw430_t333.ron`. Same procedure as T028, alias rows as in T029. Expected (confirm per page's Protocol Type value table; 1 → `P1`, 2 → `P2`, other values such as Modbus-RTU ignored): `protocols: [P1, P2]`. `type_info` `2048 / 0 / 4095`, `min_radian: -3.1415927`, `max_radian: 3.1400587` when the page gives 4096 pulses per turn (0.088°) with center 2048. Mode row as in T028: `Operating Mode` (address 11, size 1 in today's tables) confirmed on each page (`operating_modes` follow in T037). Findings to `specs/005-tables/findings/t1-x430.md`
- [ ] T031 [P] [US3] X540 group, files `control_tables/xd540_t150.ron`, `control_tables/xd540_t270.ron`, `control_tables/xh540_v150.ron`, `control_tables/xh540_v270.ron`, `control_tables/xh540_w150.ron`, `control_tables/xh540_w270.ron`, `control_tables/xm540_w150.ron`, `control_tables/xm540_w270.ron`, `control_tables/xw540_h260.ron`, `control_tables/xw540_t140.ron`, `control_tables/xw540_t260.ron`. Same procedure and expectations as T030 (`operating_modes` follow in T038). Findings to `specs/005-tables/findings/t1-x540.md`
- [ ] T032 [P] [US3] X330 group, files `control_tables/xc330_m181.ron`, `control_tables/xc330_m181_fw52.ron`, `control_tables/xc330_m288.ron`, `control_tables/xc330_m288_fw52.ron`, `control_tables/xc330_t181.ron`, `control_tables/xc330_t181_fw52.ron`, `control_tables/xc330_t288.ron`, `control_tables/xc330_t288_fw52.ron`, `control_tables/xl330_m077.ron`, `control_tables/xl330_m077_fw52.ron`, `control_tables/xl330_m288.ron`, `control_tables/xl330_m288_fw52.ron`. Same procedure as T028, alias rows as in T029. For a `_fw52` table use the firmware-52-and-below control table the model's page gives; if the page gives none, fill nothing in that file by hand and list every row as a finding (no sibling copy). Expected: Protocol Type values 2 ~ 22 with no 1 → `protocols: [P2]` (research R-3, XL330-M077). `type_info` as T030 when the page gives 4096 pulses per turn. Mode row as T030 (for a `_fw52` table, against the firmware-52 control table of the page; `operating_modes` follow in T039). Findings to `specs/005-tables/findings/t1-x330.md`
- [ ] T033 [P] [US3] PRO / PRO+ / RH group, files `control_tables/h42_20_s300_ra.ron`, `control_tables/h54_100_s500_ra.ron`, `control_tables/h54_200_s500_ra.ron`, `control_tables/m42_10_s260_ra.ron`, `control_tables/m54_40_s250_ra.ron`, `control_tables/m54_60_s250_ra.ron`, `control_tables/ph42_020_s300.ron`, `control_tables/ph54_100_s500.ron`, `control_tables/ph54_200_s500.ron`, `control_tables/pm42_010_s260.ron`, `control_tables/pm54_040_s250.ron`, `control_tables/pm54_060_s250.ron`, `control_tables/rh_p12_rn.ron`. Same procedure as T028, alias rows as in T029 (pages have an extra Modbus column; `emanual.py` matches by header name). Expected: the PRO+ and RH-P12-RN(A) pages give Protocol Type 2, 10 (Modbus-RTU) → `protocols: [P2]`; the six PRO `*_ra` tables have no Protocol Type row, so use the protocol their page states (research R-3). `type_info`: research R-8 says these are already consistent (zero 0, symmetric); check against the page, change only on a page difference. Mode row as T030 (`operating_modes` follow in T040). Findings to `specs/005-tables/findings/t1-pro.md`
- [ ] T034 [P] [US3] Y group, files `control_tables/ym070_210_a051.ron`, `control_tables/ym070_210_a099.ron`, `control_tables/ym070_210_b001.ron`, `control_tables/ym070_210_m001.ron`, `control_tables/ym070_210_r051.ron`, `control_tables/ym070_210_r099.ron`, `control_tables/ym080_230_a051.ron`, `control_tables/ym080_230_a099.ron`, `control_tables/ym080_230_b001.ron`, `control_tables/ym080_230_m001.ron`, `control_tables/ym080_230_r051.ron`, `control_tables/ym080_230_r099.ron`. Same procedure as T028, alias rows as in T029. Expected `protocols: [P2]` (confirm on each page). `type_info` checked as in T033. Mode row as in T028: `Operating Mode` at address 33 (size 1 in today's tables) confirmed on each page (`operating_modes` follow in T041). Tables stay mapped and `Unverified` (FR-013, US2 AS5). Findings to `specs/005-tables/findings/t1-y.md`
- [ ] T035 [P] [US3] Update `control_tables/analyze_tables.py` (research R-13, FR-015): read each table's `protocols` (import the parser from `check_tables.py`) instead of the CW/CCW Angle Limit heuristic; group P1-only, P2-only and `[P1, P2]` in the protocol section; take model numbers from `dynamixel.ron` `p1`/`p2` lists and, while present, the legacy `tables` list (the `"<file>.model"` loader matches nothing today). Keep the rest of the report unchanged

Rules for T036–T041 (FR-020, US3 AS7, research R-15, contracts/ron-schema.md §1). Per stem, open
the page `PAGES` gives (the one its group task used) and read the mode register's value table
(`Operating Mode`, or XL-320's `Control Mode`) and the text of the position limits (Min/Max
Position Limit, or CW/CCW Angle Limit) that says in which modes they apply. Write one line
`operating_modes: (bounded: [...], unbounded: [...], no_position: [...]),` after `protocols` and
before `type_info`, with the file's indentation: all three keys, values ascending, `[]` when
empty. `bounded`: position modes in which the page says the limits apply; `unbounded`: position
modes in which it says they are not used; `no_position`: modes in which the actuator does not
follow Goal Position (current, velocity, PWM, wheel). A value the page marks reserved goes in no
list. If the page does not say whether the limits apply in one of its position modes, write
nothing in that table and raise it: the value is never placed by guess, and T1 waits for the
answer (the check requires the field). Only a table with a mode row gets the field; a table whose
page lists a mode register the table lacks (a T1 finding of its group task) gets the row and the
field together in T2 Phase 11. Each table copies its own page, never a sibling table or the
family list; the page wins over the expected lists below, and a difference is recorded and
flagged to the architect. Keep `source: Unverified`; change nothing else. Per stem, write the
page URL, the fetch date, the page's value table (value and mode name, quoted), the limit
sentence (quoted) and the line written to the named findings file.

- [ ] T036 [P] [US3] `operating_modes` for the MX(2.0) + XL-320 group, files `control_tables/mx_28.ron`, `control_tables/mx_64.ron`, `control_tables/mx_106.ron`, `control_tables/xl320.ron` (after T029). Cite each table's own page from `PAGES`; for MX-64(2.0) that is `https://emanual.robotis.com/docs/en/dxl/mx/mx-64-2/`, read by core research R-17 (Operating Mode(11) 0, 1, 3, 4, 5, 16; Min/Max Position Limit "not used in Extended Position Control Mode and Current-based Position Control Mode"), and for XL-320 `https://emanual.robotis.com/docs/en/dxl/x/xl320/` (Control Mode(11): 1 Wheel Mode, 2 Joint Mode; "If it is set to Wheel Mode, Goal Position value is not used."). Expected: MX(2.0) `(bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16])`; XL-320 `(bounded: [2], unbounded: [], no_position: [1])`. Findings to `specs/005-tables/findings/t1-modes-mx-xl320.md`
- [ ] T037 [P] [US3] `operating_modes` for the X430 group, the 15 files of T030: `control_tables/2xc430_w250.ron`, `control_tables/2xl430_w250.ron`, `control_tables/xc430_w150.ron`, `control_tables/xc430_w240.ron`, `control_tables/xd430_t210.ron`, `control_tables/xd430_t350.ron`, `control_tables/xh430_v210.ron`, `control_tables/xh430_v350.ron`, `control_tables/xh430_w210.ron`, `control_tables/xh430_w350.ron`, `control_tables/xl430_w250.ron`, `control_tables/xm430_w210.ron`, `control_tables/xm430_w350.ron`, `control_tables/xw430_t200.ron` (not mapped; the rule applies to it too), `control_tables/xw430_t333.ron` (after T030). Cite each table's own page from `PAGES`; core research R-17 read `https://emanual.robotis.com/docs/en/dxl/x/xm430-w350/` (Operating Mode(11) 0, 1, 3, 4, 5, 16; "These values are not used in Extended Position Control Mode and Current-based Position Control Mode"), `https://emanual.robotis.com/docs/en/dxl/x/xl430-w250/` and `https://emanual.robotis.com/docs/en/dxl/x/2xl430-w250/` (1, 3, 4, 16 only; "The angle limits set through this control table item do not apply in Extended Position Control Mode"). Expected: `(bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16])`; for `xl430_w250` and `2xl430_w250` `(bounded: [3], unbounded: [4], no_position: [1, 16])`; `2xc430_w250` and the rest as their pages give. Findings to `specs/005-tables/findings/t1-modes-x430.md`
- [ ] T038 [P] [US3] `operating_modes` for the X540 group, the 11 files of T031: `control_tables/xd540_t150.ron`, `control_tables/xd540_t270.ron`, `control_tables/xh540_v150.ron`, `control_tables/xh540_v270.ron`, `control_tables/xh540_w150.ron`, `control_tables/xh540_w270.ron`, `control_tables/xm540_w150.ron`, `control_tables/xm540_w270.ron`, `control_tables/xw540_h260.ron`, `control_tables/xw540_t140.ron`, `control_tables/xw540_t260.ron` (after T031). Cite each table's own page from `PAGES`; core research R-17 read `https://emanual.robotis.com/docs/en/dxl/x/xw540-t260/` (the same list and sentence as XM430-W350). Expected: `(bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16])`. Findings to `specs/005-tables/findings/t1-modes-x540.md`
- [ ] T039 [P] [US3] `operating_modes` for the X330 group, the 12 files of T032: `control_tables/xc330_m181.ron`, `control_tables/xc330_m181_fw52.ron`, `control_tables/xc330_m288.ron`, `control_tables/xc330_m288_fw52.ron`, `control_tables/xc330_t181.ron`, `control_tables/xc330_t181_fw52.ron`, `control_tables/xc330_t288.ron`, `control_tables/xc330_t288_fw52.ron`, `control_tables/xl330_m077.ron`, `control_tables/xl330_m077_fw52.ron`, `control_tables/xl330_m288.ron`, `control_tables/xl330_m288_fw52.ron` (after T032). Cite each table's own page from `PAGES`; core research R-17 read `https://emanual.robotis.com/docs/en/dxl/x/xl330-m288/` (the same list and sentence as XM430-W350). For a `_fw52` table use what the page gives for firmware 52 and below; if the page gives one Operating Mode description for all firmware, it applies to both tables; if it gives a list for the newer firmware only, write nothing in the `_fw52` table and raise it. Expected: `(bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16])`. Findings to `specs/005-tables/findings/t1-modes-x330.md`
- [ ] T040 [P] [US3] `operating_modes` for the PRO / PRO+ / RH group, the 13 files of T033: `control_tables/h42_20_s300_ra.ron`, `control_tables/h54_100_s500_ra.ron`, `control_tables/h54_200_s500_ra.ron`, `control_tables/m42_10_s260_ra.ron`, `control_tables/m54_40_s250_ra.ron`, `control_tables/m54_60_s250_ra.ron`, `control_tables/ph42_020_s300.ron`, `control_tables/ph54_100_s500.ron`, `control_tables/ph54_200_s500.ron`, `control_tables/pm42_010_s260.ron`, `control_tables/pm54_040_s250.ron`, `control_tables/pm54_060_s250.ron`, `control_tables/rh_p12_rn.ron` (after T033). Cite each table's own page from `PAGES`; core research R-17 read `https://emanual.robotis.com/docs/en/dxl/pro/h54-200-s500-ra/` (0, 1, 3, 4, 16; "Max Position Limit(48) and Min Position Limit(52) are NOT used in Extended Position Control Mode."), `https://emanual.robotis.com/docs/en/dxl/p/ph54-200-s500-r/` and `https://emanual.robotis.com/docs/en/dxl/p/pm54-060-s250-r/` (0, 1, 3, 4, 16; limits "only used when the Operating Mode is the Position Control Mode (Joint Mode)"), and the older RH-P12-RN page `https://emanual.robotis.com/docs/en/platform/rh_p12_rn/` (0 Current Control Mode, "1 ~ 4 Reserved", 5 Current based Position Control Mode; "The Goal Position(596) can't exceed these values"). `rh_p12_rn` is RH-P12-RN(A) (35074): read its own (A) page from `PAGES`, not the older one. Expected: PRO (A) and PRO+ `(bounded: [3], unbounded: [4], no_position: [0, 1, 16])`; RH-P12-RN(A) `(bounded: [5], unbounded: [], no_position: [0])`. Findings to `specs/005-tables/findings/t1-modes-pro.md`
- [ ] T041 [P] [US3] `operating_modes` for the Y group, the 12 files of T034: `control_tables/ym070_210_a051.ron`, `control_tables/ym070_210_a099.ron`, `control_tables/ym070_210_b001.ron`, `control_tables/ym070_210_m001.ron`, `control_tables/ym070_210_r051.ron`, `control_tables/ym070_210_r099.ron`, `control_tables/ym080_230_a051.ron`, `control_tables/ym080_230_a099.ron`, `control_tables/ym080_230_b001.ron`, `control_tables/ym080_230_m001.ron`, `control_tables/ym080_230_r051.ron`, `control_tables/ym080_230_r099.ron` (after T034). Cite each table's own page from `PAGES`; core research R-17 read `https://emanual.robotis.com/docs/en/dxl/y/ym070-210-m001-rh/` and `https://emanual.robotis.com/docs/en/dxl/y/ym080-230-m001-rh/` (Operating Mode(33): "DYNAMIXEL-Y actuators support three control modes", 0, 1, 3; Position Control Mode: "Movement is restricted to within the range defined by Max and Min [Position Limit(84)]"). Expected: `(bounded: [3], unbounded: [], no_position: [0, 1])`. Findings to `specs/005-tables/findings/t1-modes-y.md`
- [ ] T042 [US3] Validate US3 after T027–T041: `python3 control_tables/check_tables.py` exits 0 with a protocols column on all 73 tables (US3 AS1, FR-005, SC-006), no row-attribute error (AS2, FR-007) and no mode-source or `operating_modes` error (AS5, AS7, FR-020, SC-008); every mode-row finding of T028–T034 and every `operating_modes` finding of T036–T041 is in its findings file (AS6, AS7); `grep -L '"Operating Mode"' control_tables/*.ron` lists only the 5 `ax_*` tables, `xl320` and `dynamixel.ron`; `grep -L 'operating_modes' control_tables/*.ron` lists only the 5 `ax_*` tables and `dynamixel.ron` (67 tables carry it; any other gap is a raised table of T036–T041); `python3 control_tables/analyze_tables.py` regenerates `control_tables/control_table_analysis.md` and `control_tables/control_table_analysis.json`; XL-320 sits under P2 and, per stem, the JSON protocol equals the table's `protocols` (AS4, FR-015; check with a one-off script); `cargo xtask codegen && git diff --exit-code src/models.rs src/models/` is empty. If a role rule fails on real data and the fix would change an address, size or register constant, stop and raise it (T1 cannot regenerate). Commit per family group ("tables: access and area, <group>", then "tables: operating modes, <group>")

**Checkpoint**: every row carries `access` and `area`, every table its protocols and exactly one
mode source, and every table with a mode row its `operating_modes`.

---

## Phase 6: User Story 4, T1 slice - Per-protocol model map (Priority: P2) — PR T1

**Goal**: Every model number has `p1` and/or `p2` lists and `default: P2` when both exist, beside
the unchanged legacy `tables` list.

**Independent Test**: The check's `dual-protocol models` section lists each dual model with a P1
table, a P2 table and `default: P2`; codegen output is unchanged.

### Tests for User Story 4 (T1 slice) ⚠️

- [ ] T043 [US4] Map tests in `control_tables/test_check_tables.py` (class `MapTests`), covering US4 AS1–AS3, AS5, FR-008–FR-010, FR-012, FR-019, SC-007 and xtask checks 2–6, 8, 13, 14, 16. Rules quoted from data-model "Model entry" / "Table ref" / "Map validation": `from_fw` "first in a list is 0; strictly increasing" (first 1 → exit 1; swapped values → exit 1, quickstart FR-010 row); `file` "stem of an existing `.ron`" (missing → 1); neither `p1` nor `p2` → 1; an empty list → 1; `default` "exactly when both lists exist" and "must be `P2`" (missing with both → 1; `default: P1` → 1; present with one list → 1); `family` one of `"ax"`, `"mx"`, `"x-series"`, `"y-series"`, `"pro"` (`"dxl"` → 1); "A file appears under at most one model number. It may appear under both `p1` and `p2` of that number." (two numbers → 1; both lists of one number → 0); a `p1` list naming a table whose `protocols` lacks `P1` → 1 (and `p2`/`P2`); "Dual-protocol rule: if any table of a model lists both protocols in `protocols`, the model has both lists (and `default: P2`)" → 1 when one list is missing; legacy `tables` that differs from the default (or only) list → 1; "`Firmware Version` sits at one (address, size) across the files of one number's `p1` list, and across its `p2` list" → 1 when it differs; a role row above address 255 in a table under `p1` → 1; two mapped stems giving one type identifier (`ax_12a`, `ax12a` → `Ax12a`) → 1; a mapped stem whose module identifier is `tests` or a Rust 2024 keyword (`type`) → 1; a leading digit is spelled out (`2xl430_w250` → `TwoXl430W250`, not an error); the `dual-protocol models` section prints `<number> <name>   p1: <files>   p2: <files>   default: P2` (AS1–AS3)

### Implementation for User Story 4 (T1 slice)

- [ ] T044 [US4] Implement the map rules and the `dual-protocol models` section in `control_tables/check_tables.py` until T043 passes; mapping column `<number> p1 p2` in the per-table line
- [ ] T045 [US4] Add `p1`/`p2`/`default` to all 64 entries of `control_tables/dynamixel.ron`, keeping the legacy `tables` list unchanged and last. Derive the lists from each mapped table's `protocols` (set from the e-Manual in T020, T028–T034): table(s) `[P1]` → `p1` only; `[P2]` → `p2` only; `[P1, P2]` → `p1` and `p2` both equal to the existing `tables` list, plus `default: P2`. XL330/XC330 entries keep their two firmware variants in order (`p2: [(from_fw: 0, file: "…_fw52"), (from_fw: 53, file: "…")]`). Key order per entry: `family, name, p1, p2, default, tables`; one entry per line as today. Do not add `also`, new numbers or `not_mapped` here
- [ ] T046 [US4] Rewrite the header comment of `control_tables/dynamixel.ron` to document, in plain words, `family`, `name`, `also`, `p1`, `p2` (firmware-ordered, `from_fw` from 0, strictly increasing), `default` (must be `P2`, exactly when both lists exist), the legacy `tables` (T1 only, equal to the default or only list; removed in T2) and `not_mapped` (`NoModelNumber` or `SameNumberAs(n)`; unverified is never a reason) (contract ron-schema §2 last line)
- [ ] T047 [US4] Validate the T1 slice: `python3 control_tables/check_tables.py` exits 0 (the unrecorded-file rules come in Phase 7); the `dual-protocol models` section lists every `[P1, P2]` model with `default: P2` (research R-4 expects 28: 25 X430/X540/2XL/2XC/XW numbers + 3 MX(2.0); record the actual count and any page that changed it in `specs/005-tables/findings/t1-map.md`) (US4 AS1–AS3, SC-007); `cargo xtask codegen && git diff --exit-code src/models.rs src/models/` is empty (US4 AS5, FR-009). US4 AS4 (no table for a protocol → `UnsupportedModel`) is X1's lookup test; note it for traceability only

**Checkpoint**: the map speaks both protocols; today's codegen still reads `tables`.

---

## Phase 7: User Story 5, T1 slice - Record the unmapped tables (Priority: P2) — PR T1

**Goal**: The three files no number points to are recorded with their reason, so the check has
zero unrecorded files.

**Independent Test**: The check summary shows `3 not mapped` and 0 errors; deleting one record in
a scratch copy fails the check naming the file.

### Tests for User Story 5 (T1 slice) ⚠️

- [ ] T048 [US5] Not-mapped tests in `control_tables/test_check_tables.py` (class `NotMappedTests`), covering US5 AS4–AS5, FR-011, SC-003 and xtask checks 6–7: a table neither mapped nor in `not_mapped` → exit 1 naming the file ("A file is mapped or in `not_mapped`, never both, never neither"); a file both mapped and in `not_mapped` → 1; a record naming a missing file → 1; `SameNumberAs(n)` where `n` is not a key of `models` → 1 ("`SameNumberAs` names a number in `models`"); a reason other than `NoModelNumber` or `SameNumberAs(u16)` (e.g. `Unverified`) → 1 ("Unverified" is never a not-mapped reason); the per-table line shows `not mapped: same number as 12` / `not mapped: no model number`

### Implementation for User Story 5 (T1 slice)

- [ ] T049 [US5] Implement the `not_mapped` rules and mapping column in `control_tables/check_tables.py` until T048 passes
- [ ] T050 [US5] Add to `control_tables/dynamixel.ron`, after `models`, `not_mapped: [(file: "ax_12_plus", reason: SameNumberAs(12)), (file: "ax_18f", reason: SameNumberAs(18)), (file: "xw430_t200", reason: SameNumberAs(1270))]`, after re-reading each model number on its page the same day: AX-12A page `https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/` (headed "AX-12+, AX-12A", Model Number 12); the AX-18A page (covers "AX-18F, AX-18A", Model Number 18); `https://emanual.robotis.com/docs/en/dxl/x/xw430-t200/` (Model Number 1,270, same as XW430-T333; re-confirmed by the architect on 2026-10-28, research R-10). If a page now gives another number, stop and raise it. Write URLs, numbers and dates to `specs/005-tables/findings/t1-map.md`
- [ ] T051 [US5] Validate the T1 slice: `python3 control_tables/check_tables.py` exits 0 with `summary: 73 tables, 2 verified, 71 unverified, 70 mapped, 3 not mapped, <n> dual-protocol, 0 errors` (SC-003: 0 unrecorded files, 0 duplicate numbers, 0 files unmapped for being unverified); `cargo xtask codegen` still prints the three skip warnings (expected until X1 reads `not_mapped`) and `git diff --exit-code src/models.rs src/models/` is empty

**Checkpoint**: PR T1 content is complete.

---

## Phase 8: Polish and PR (PR T1)

- [ ] T052 Run every gate of quickstart §1 in the T1 worktree (`python3 -m unittest discover -s control_tables -p 'test_*.py'`; `python3 control_tables/check_tables.py`; `cargo xtask codegen && git diff --exit-code src/models.rs src/models/`; `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`; `cargo test --workspace`; `cargo clippy --no-default-features --features ax,mx,rx,x-series,y-series,pro --target thumbv7em-none-eabihf`) and every T1 row of the quickstart §2 table (scratch copies with `--dir` for US1 AS2, AS4, FR-010, FR-012). Time the check (plan: under 2 s). Fix fallout only in `control_tables/**`
- [ ] T053 Write `specs/005-tables/pr-description-t1.md` (main checkout, not staged) from `.github/pull_request_template.md`: What/Why (tables T1: FR-001, FR-002, FR-005, FR-007, FR-008–FR-011, FR-013–FR-016, FR-018 for AX-12A and RX-28, FR-020); per changed table its flag from `python3 control_tables/check_tables.py --markdown` (FR-017); every `type_info` change and every finding merged from `specs/005-tables/findings/t1-*.md` with page URL and date, mode-row findings in their own list (fixed in T2); the `operating_modes` written per table from `specs/005-tables/findings/t1-modes-*.md`, with page URL and date, and every page whose list differs from its family's expected list (research R-15) in its own list; the AX-12A audit rows; Architecture section listing the plan's deviations (two PRs for one module, approval asked in module-interfaces §9; XW430-T200 under 1270, now as the contract shows (both pages re-checked 2026-10-28); AX-18F as `SameNumberAs(18)`, not in the contract example; `instructions` entries carry `source` and optional `from_fw`; the check has its own rules beside codegen's (`Control Mode` only in `xl320`, unmapped tables checked, unrecorded files are errors); the check duplicates xtask's data checks 2–17, mode source and `operating_modes` included; `operating_modes` placed after `protocols` (contract 3.1 fixes no position); AX-12+ cites the AX-12A page); notes for X1 (`type_info` values after T1, one model type per file even when under `p1` and `p2`, every table gives exactly one mode source, `Operating Mode` at 11 or 33, `operating_modes` on the 67 tables with a mode row, tables read with `IMPLICIT_SOME`, research R-1), core (slope 2π/4096 fixed; the mode value lists are now in the tables, so core holds none) and ci (remove the `if [ -f control_tables/check_tables.py ]` guard in its next change); "no generated file changed". No AI attribution
- [ ] T054 Commit on `005-tables-t1` in the plan's order (tools + tests; flags; access/area/protocols/type_info and mode-row checks per family; `operating_modes` per family; map; AX-12A verification; RX-28 table, then its map entry with the `rx` feature and the regenerated `src/models.rs` and `src/models/rx_28.rs` in one commit; analysis), staging only `control_tables/**` plus T023's `Cargo.toml` and generated files; push and open the PR into `main` with the body from T053, authored by Ovsiankina
- [ ] T055 [human] Architect: approve the T1/T2 split (module-interfaces §9) and the AX-12+ record citing the AX-12A page (research R-9, US2 AS1 "its own model's page") before T1 merges; answer any `operating_modes` question T036–T041 raised (a position mode whose page does not say whether the limits apply), since T1 cannot pass the check without the field
- [ ] T056 [human] Architect (not blocking T1): amend `docs/srs/SRS.md` (in `.aiignore`) for the 2026-10-28 decisions (FR-3.7: limits follow the operating mode; the tables give the mode source and, in `operating_modes`, the meaning of each mode value, FR-020) and the stale roadmap line "AX-12+, AX-18F, XW430-T200: … Adding the number is enough" (their numbers are shared, research R-10), then regenerate `docs/srs/modules/*.md` with `/speckit-ov-srs` (plan Constitution Check, Governance row); update `docs/ARCHITECTURE.md` "Not implemented yet" (MX P1 tables, unmapped models); optionally add `python3 control_tables/check_tables.py` to the command list in `CLAUDE.md`

---

# Part B: PR T2 (`005-tables-t2`, after xtask X1 is merged)

## Phase 9: Setup (PR T2)

**Purpose**: Start from a `main` that has X1, and keep the check in step with codegen

- [ ] T057 Confirm xtask X1 is merged into `main` (and with it T1, core-1 and protocol PR #7, module-interfaces §7). Check the main checkout for a dirty tree, then `git fetch origin && git worktree add ../dxl_rs_v2-005-t2 -b 005-tables-t2 origin/main`. Baseline: `cargo xtask codegen && git diff --exit-code src/models.rs src/models/` is empty and prints no skip warning (X1 reads `not_mapped`); `python3 control_tables/check_tables.py` exits 0; unit tests pass. Write the baseline to `specs/005-tables/findings/t2-baseline.md`
- [ ] T058 Compare X1's data-only checks (`specs/006-xtask-codegen/data-model.md` §3 and `xtask/src/codegen.rs` as merged) and its mode-source derivation (`ModelData::mode`, module-interfaces §2.4b) with the mapping table in `specs/005-tables/contracts/table-tools.md` §1. If X1 added or changed a data-only check (checks 11 and 17 for the mode source and `operating_modes` included, and how X1 parses `operating_modes`: confirm the merged code matches the mapping rows), add a failing test to `control_tables/test_check_tables.py` and the rule to `control_tables/check_tables.py` ("Codegen stays the authority. If xtask adds a data check, this table and the check follow."); otherwise record "no change" in `specs/005-tables/findings/t2-baseline.md`

---

## Phase 10: User Story 2, T2 slice - AX-12+, MX-64(2.0), MX-64 P1 verified (Priority: P1) — PR T2

**Goal**: The three remaining priority tables are verified, each against its own page, with
their own record (US2 AS1); MX-64 P1 is written new and mapped under 310.

**Independent Test**: The check reports `mx_64`, `ax_12_plus` and `mx_64_p1` as verified with an
`emanual.robotis.com` URL and a valid date; `python3 control_tables/emanual.py diff <stem>`
exits 0 for each; 5 random rows of each match the page by eye (SC-002).

### Tests for User Story 2 (T2 slice) ⚠️

The rules these tasks need are already tested (T013, T017, T025, T026, T043, T048). The checks for
this slice are `check_tables.py` and `emanual.py diff` in T063; quickstart §2 rows "US2, SC-002"
and "SC-002 audit".

### Implementation for User Story 2 (T2 slice)

- [ ] T059 [P] [US2] Create `control_tables/mx_64_p1.ron` (MX-64 on Protocol 1 firmware) from `https://emanual.robotis.com/docs/en/dxl/mx/mx-64/` only (research R-11): rows from `python3 control_tables/emanual.py rows https://emanual.robotis.com/docs/en/dxl/mx/mx-64/`, reviewed row by row against the page, each with `access` (`R` or `RW`) and `area` (`EEPROM` or `RAM`); layout and 2-space multi-line style of `control_tables/mx_64.ron`; `protocols: [P1]` (no Protocol Type register, the page points to MX(2.0) for Protocol 2); `type_info` from the page's position range and resolution (research R-8 formula `angle = (raw − zero) × rad_per_tick`); `unit_info` only for values the page gives; mode source (FR-020, US5 AS6): no `Operating Mode` row, no `operating_modes`, and the `CW Angle Limit` / `CCW Angle Limit` pair from the page (the mode comes from them, module-interfaces §2.4b); if the page lists an Operating Mode register, stop and raise it; `instructions`: one P1 entry (`source: "https://emanual.robotis.com/docs/en/dxl/protocol1/"`) with every instruction whose Protocol 1.0 condition includes MX-64, plus a second P1 entry with `from_fw` for any instruction the page conditions on a firmware version (research R-5: Reboot for "MX-28/64/106(V40)" → `from_fw: 40`); every role row at address ≤ 255; then `source: Verified(url: "https://emanual.robotis.com/docs/en/dxl/mx/mx-64/", verified: "<date>")`. A firmware boundary not stated on the page is not mapped (spec edge case). Findings (every row, URL, date) to `specs/005-tables/findings/t2-mx_64_p1.md`
- [ ] T060 [P] [US2] Verify `control_tables/mx_64.ron` (MX-64(2.0)) against `https://emanual.robotis.com/docs/en/dxl/mx/mx-64-2/` (research R-9): `python3 control_tables/emanual.py diff mx_64`; remove the four alias rows "Indirect Address Write", "Indirect Data Write", "Indirect Address Read", "Indirect Data Read" (not on the page, US2 AS3); keep the existing "Indirect Address 1" / "Indirect Data 1" rows and add no other indirect rows; correct any row the page lists at another address or size (US2 AS2); check `access`/`area` of every row, the `Operating Mode` row (address, size, access, area on the page; FR-020), every value of `operating_modes` against the page's Operating Mode(11) values and the Min/Max Position Limit text (research R-15; written in T036, expected `(bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16])`), `protocols: [P1, P2]` (Protocol Type 1 ~ 2), `type_info` (`2048 / 0 / 4095`, `-3.1415927` / `3.1400587` from T1, against the page), every `unit_info` entry (remove what the page does not give); `instructions`: a P1 entry from `https://emanual.robotis.com/docs/en/dxl/protocol1/` and a P2 entry from `https://emanual.robotis.com/docs/en/dxl/protocol2/`, each with the instructions whose product condition includes MX(2.0), firmware conditions as `from_fw` entries (research R-5 lists Fast Sync/Bulk Read and Control Table Backup only for X430/540, X330, P Series and RH-P12-RN(A); the page decides); then `source: Verified(url: "https://emanual.robotis.com/docs/en/dxl/mx/mx-64-2/", verified: "<date>")`. Every corrected or removed row to `specs/005-tables/findings/t2-mx_64.md`
- [ ] T061 [P] [US2] Verify `control_tables/ax_12_plus.ron` against `https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/` (the page headed "AX-12+, AX-12A"; AX-12+ has no page of its own, research R-9, architect approval in T055): `python3 control_tables/emanual.py diff ax_12_plus https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/`; resolve every difference from the page; check `protocols: [P1]`, `type_info`, `unit_info`, `access`/`area`, and the mode source (no `Operating Mode` or `Control Mode` register on the page, CW/CCW pair present, no `operating_modes`; FR-020); add `instructions` (P1 entry from the Protocol 1.0 page, conditions that include AX-12+); then `source: Verified(url: "https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/", verified: "<date>")`. Its own record even though the layout equals `ax_12a` (US2 AS1). Findings to `specs/005-tables/findings/t2-ax_12_plus.md`
- [ ] T062 [US2] Add `310: (family: "mx", name: "MX-64", p1: [(from_fw: 0, file: "mx_64_p1")])` to `control_tables/dynamixel.ron`, with the model number and name read on `https://emanual.robotis.com/docs/en/dxl/mx/mx-64/` (research R-11: 310). No `default` (one list only). Depends on T059
- [ ] T063 [US2] Validate the T2 slice: `python3 control_tables/check_tables.py` exits 0 and lists `ax_12a`, `rx_28`, `mx_64`, `ax_12_plus`, `mx_64_p1` as verified (US2 AS1, SC-002, FR-018), with no mode-source or `operating_modes` error; `python3 control_tables/emanual.py diff` exits 0 for `mx_64`, `ax_12_plus` and `mx_64_p1`, mode rows included, and `mx_64`'s `operating_modes` matches its page (SC-008); audit 5 random rows of each by eye (SC-002) into its findings file; `python3 control_tables/check_tables.py --markdown mx_64 ax_12_plus mx_64_p1` saved for the PR; Y-series and PRO tables still mapped and `unverified` (US2 AS5, FR-013); run `cargo xtask codegen` and review the diff: only the new `Mx64P1` type, the removed MX-64(2.0) alias constants, the verification docs of the verified models and, if T060 corrected a list, the `ModeValues` in `Mx64::DATA` change

**Checkpoint**: all four priority tables are verified.

---

## Phase 11: User Story 3, T2 slice - Remove rows the e-Manual does not list (Priority: P2) — PR T2

**Goal**: The alias rows not on the e-Manual leave the other 65 tables, and the other T1 row
findings, mode-row findings included, are corrected from the pages (research R-14, R-15, spec
edge cases "a mistake found in an unverified table" and "a table misses the Operating Mode row").
Tables stay `Unverified`.

**Independent Test**: `python3 control_tables/emanual.py diff <stem>` no longer shows the alias rows
for any table whose page does not list them; the check exits 0.

### Tests for User Story 3 (T2 slice) ⚠️

The row, role and mode-source rules are tested (T025, T026). The check for each group is the
per-stem `emanual.py diff` before and after, recorded in its findings file, and
`check_tables.py` in T071.

### Implementation for User Story 3 (T2 slice)

For each task below: for every stem, re-run `python3 control_tables/emanual.py diff <stem>`;
remove "Indirect Address Write", "Indirect Data Write", "Indirect Address Read" and "Indirect Data
Read" only where the page does not list that name; apply every other row correction listed in the
group's T1 findings file, from the page (address, size or name the page gives), including each
mode-row finding: add the `Operating Mode` / `Control Mode` row the page lists, or move it to the
page's address and size, with the page's `access` and `area` (FR-020); a row added this way comes
with its `operating_modes` from the same page (rules of T036–T041); never add a row the page
does not show; the table keeps exactly one mode source; keep `source: Unverified`; write each
removal and correction with URL and date to the named T2 findings file (FR-017).

- [ ] T064 [P] [US3] MX(2.0) rest, files `control_tables/mx_28.ron`, `control_tables/mx_106.ron` (`mx_64` done in T060); T1 findings `specs/005-tables/findings/t1-mx-xl320.md`; write `specs/005-tables/findings/t2-mx.md`
- [ ] T065 [P] [US3] AX + XL-320 corrections, files `control_tables/ax_12w.ron`, `control_tables/ax_18a.ron`, `control_tables/ax_18f.ron`, `control_tables/xl320.ron` (no alias rows in these; only T1 findings from `specs/005-tables/findings/t1-ax.md` and `t1-mx-xl320.md`); write `specs/005-tables/findings/t2-ax-xl320.md`
- [ ] T066 [P] [US3] X430 group, the 15 files of T030; T1 findings `specs/005-tables/findings/t1-x430.md`; write `specs/005-tables/findings/t2-x430.md`
- [ ] T067 [P] [US3] X540 group, the 11 files of T031; T1 findings `specs/005-tables/findings/t1-x540.md`; write `specs/005-tables/findings/t2-x540.md`
- [ ] T068 [P] [US3] X330 group, the 12 files of T032; T1 findings `specs/005-tables/findings/t1-x330.md`; write `specs/005-tables/findings/t2-x330.md`
- [ ] T069 [P] [US3] PRO / PRO+ / RH group, the 13 files of T033; T1 findings `specs/005-tables/findings/t1-pro.md`; write `specs/005-tables/findings/t2-pro.md`
- [ ] T070 [P] [US3] Y group, the 12 files of T034; T1 findings `specs/005-tables/findings/t1-y.md`; write `specs/005-tables/findings/t2-y.md`
- [ ] T071 [US3] Validate: `python3 control_tables/check_tables.py` exits 0 (no role or mode row removed by mistake, every mode row with its `operating_modes`); `grep -l 'Indirect Address Write' control_tables/*.ron` lists only tables whose page lists that name; `cargo xtask codegen` and review the diff: only removed alias constants, corrected registers and the `ModeValues` of a mode row added in this phase change, each one listed in a T2 findings file (SC-004)

**Checkpoint**: no table carries a row its page does not list (as far as the diffs show).

---

## Phase 12: User Story 4, T2 slice - Drop the legacy `tables` list (Priority: P2) — PR T2

**Goal**: The map has only `p1`/`p2`/`default` (contract ron-schema §2: `tables` "Absent after T2").

**Independent Test**: `grep -c 'tables:' control_tables/dynamixel.ron` is 0; the check exits 0;
codegen output does not change because of the removal.

### Tests for User Story 4 (T2 slice) ⚠️

- [ ] T072 [US4] In `control_tables/test_check_tables.py`, replace the T1 test "legacy `tables` differs from the default list" with "any `tables` key in a model entry → exit 1, naming the model number" (contract ron-schema §2 "Absent after T2: from T2 on, the check rejects a `tables` key in any model entry"; plan PR split T2); keep every other map test

### Implementation for User Story 4 (T2 slice)

- [ ] T073 [US4] Implement the rule in `control_tables/check_tables.py` until T072 passes, then delete `tables: [...]` from every entry of `control_tables/dynamixel.ron` and remove the legacy paragraph from its header comment. Update `control_tables/analyze_tables.py` to stop reading `tables` (T035)
- [ ] T074 [US4] Validate: check exits 0; `cargo xtask codegen && git diff --exit-code src/models.rs src/models/` shows no change caused by this phase (X1 ignores `tables`; US4 AS5, FR-009)

---

## Phase 13: User Story 5, T2 slice - MX Protocol 1 tables and other product names (Priority: P2) — PR T2

**Goal**: MX-28 and MX-106 on Protocol 1 firmware exist (unverified) and are mapped under their
own numbers; the products sold under an existing number are named in `also`.

**Independent Test**: The check reports a P1 table for each MX model (29, 310, 320), 3 not-mapped
records, 0 unrecorded files, 0 duplicate numbers; codegen prints no skip warning.

### Tests for User Story 5 (T2 slice) ⚠️

- [ ] T075 [US5] In `control_tables/test_check_tables.py`: `also` per data-model "`also` | list of string | no | other products sold under this number (docs only)": `also: "AX-12+"` (not a list) → exit 1; `also: []` or an empty string → exit 1; `also: ["AX-12+"]` → 0; a P1-only entry without `default` → 0 (US5 AS1). Implement the rule in `control_tables/check_tables.py` until it passes

### Implementation for User Story 5 (T2 slice)

- [ ] T076 [P] [US5] Create `control_tables/mx_28_p1.ron` from `https://emanual.robotis.com/docs/en/dxl/mx/mx-28/` only: rows from `python3 control_tables/emanual.py rows https://emanual.robotis.com/docs/en/dxl/mx/mx-28/` reviewed against the page, with `access` and `area`; `source: Unverified` (US5 AS2); `protocols: [P1]`; `type_info` and `unit_info` only as the page gives them; no `instructions` (verified tables only); no `Operating Mode` row and no `operating_modes`, the CW/CCW Angle Limit pair from the page (mode source, FR-020, US5 AS6; if the page lists an Operating Mode register, stop and raise it); every role row at address ≤ 255; layout of `control_tables/mx_64_p1.ron`. Read the model number on the page (research R-11: 29). Findings to `specs/005-tables/findings/t2-mx_28_p1.md`
- [ ] T077 [P] [US5] Create `control_tables/mx_106_p1.ron` from `https://emanual.robotis.com/docs/en/dxl/mx/mx-106/` only, same rules as T076 (research R-11: model number 320). Findings to `specs/005-tables/findings/t2-mx_106_p1.md`
- [ ] T078 [US5] Edit `control_tables/dynamixel.ron` (after T076, T077), each value read on the cited page: add `29: (family: "mx", name: "MX-28", p1: [(from_fw: 0, file: "mx_28_p1")])` and `320: (family: "mx", name: "MX-106", p1: [(from_fw: 0, file: "mx_106_p1")])` (US5 AS1, FR-019); add `also: ["AX-12+"]` to 12 (AX-12A page), `also: ["AX-18F"]` to 18 (AX-18A page), `also: ["XW430-T200"]` to 1270 (XW430-T200 page, Model Number 1,270, confirmed 2026-10-28) — the three files stay in `not_mapped` with `SameNumberAs` (US5 AS4, FR-012); rename 35074 to the name its page gives (research: "RH-P12-RN(A)"; 35073 is the older RH-P12-RN, not in this repo). Record each change with URL and date in `specs/005-tables/findings/t2-map.md`
- [ ] T079 [US5] Validate: `python3 control_tables/check_tables.py` exits 0 with `summary: 76 tables, 5 verified, 71 unverified, 73 mapped, 3 not mapped, <n> dual-protocol, 0 errors` (data-model "Data after each PR"; SC-003); entries 29, 310, 320 show `p1` only; `grep -L '"Operating Mode"' control_tables/*.ron` lists only the 5 `ax_*` tables, `xl320`, `mx_28_p1`, `mx_64_p1`, `mx_106_p1` and `dynamixel.ron`, and `grep -L 'operating_modes' control_tables/*.ron` the same files but `xl320` (US3 AS7, US5 AS6, FR-020); `cargo xtask codegen` prints no skip warning (US2 AS4, US5 AS3–AS4: `ax_12_plus`, `ax_18f`, `xw430_t200` stay skipped, named in `also`) and generates `Mx28P1`, `Mx64P1`, `Mx106P1`

**Checkpoint**: PR T2 content is complete.

---

## Phase 14: Polish and PR (PR T2)

- [ ] T080 Regenerate and audit: `cargo xtask codegen`; list every changed file under `src/models.rs` and `src/models/` with its cause (new MX P1 types, removed alias constants, corrected registers and mode rows with their `ModeValues`, name `RH-P12-RN(A)`, verification docs, `also`) in `specs/005-tables/findings/t2-codegen.md`; any change without a cause is a bug to fix in `control_tables/**`, never in `src/` (SC-004, constitution V). Regenerate `control_tables/control_table_analysis.md` and `.json` with `python3 control_tables/analyze_tables.py` (FR-015)
- [ ] T081 Run every gate of quickstart §1 in the T2 worktree (the T052 list; the codegen diff is empty once the regenerated files are staged) plus `cargo llvm-cov --workspace --ignore-filename-regex 'src/models(\.rs|/)' --fail-under-lines 70`, and every T2 row of quickstart §2 (US2/SC-002 for `mx_64`, `ax_12_plus`, `mx_64_p1`; SC-002/SC-008 audit with `emanual.py diff`; US3 AS6/US5 AS6 mode rows; US5/SC-003 summary; codegen skip warnings; the `tables` key rejection)
- [ ] T082 Write `specs/005-tables/pr-description-t2.md` (main checkout, not staged) from `.github/pull_request_template.md`: per changed table its flag from `check_tables.py --markdown` and every corrected or removed row (FR-017, merged from `specs/005-tables/findings/t2-*.md`); the four verified records with URL and date; the SC-002 audit rows; every mode-row correction with its `operating_modes` (FR-020) and the MX P1 tables' mode source (CW/CCW pair, no Operating Mode, no `operating_modes`); the generated diff explained from `t2-codegen.md`, flagged as the only change outside `control_tables/` (contract §1); Architecture: legacy `tables` removed and now rejected by the check, AX-12+ record cites the AX-12A page. No AI attribution
- [ ] T083 Commit on `005-tables-t2` (verification; alias removals and corrections per group; map; new MX P1 tables; regenerated `src/models*` with the table change that caused it; analysis), staging only `control_tables/**`, `src/models.rs` and `src/models/**`; push and open the PR into `main` with the body from T082, authored by Ovsiankina

---

## PR map

| PR | Tasks | Branch (off `main`) | Merged first | Gates before opening |
|---|---|---|---|---|
| **T1** | T001–T056: 56 tasks (T055, T056 human) | `005-tables-t1` | nothing (parallel with core-1; module-interfaces §7) | quickstart §1: `python3 -m unittest discover -s control_tables -p 'test_*.py'`; `python3 control_tables/check_tables.py`; `cargo xtask codegen && git diff --exit-code src/models.rs src/models/` (must be empty); `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`; `cargo test --workspace`; `cargo clippy --no-default-features --features ax,mx,rx,x-series,y-series,pro --target thumbv7em-none-eabihf` |
| **T2** | T057–T083: 27 tasks | `005-tables-t2` | xtask **X1** (which itself needs protocol PR #7, core-1 and T1) | the same quickstart §1 list, with the regenerated `src/models*` committed so the codegen diff is empty; plus the coverage gate `cargo llvm-cov --workspace --ignore-filename-regex 'src/models(\.rs|/)' --fail-under-lines 70` |

T2 may run in parallel with core-2 and the bus PRs (module-interfaces §7). Do not open T2
before X1 is in `main`: today's codegen cannot map one number to two protocols, and the legacy
`tables` removal would break it.

---

## Dependencies & Execution Order

### Phase dependencies

- **Phase 1 (Setup T1)**: none. T003, T004 parallel; T005 after both.
- **Phase 2 (Foundational)**: after Phase 1. Blocks every story. T006 ∥ T007; T008 after T006;
  T009 → T010 after T008; T011 after T007 and T008 (parallel with T009–T010); T012 after T011.
- **Phase 3 (US1)**: after Phase 2. T013 → T014 → T015 → T016.
- **Phase 4 (US2 T1)**: after US1 (needs `source` and the flag rules). T017 → T018; T019 → T020
  → T021 (T019 can start once T018 is done; it needs only `emanual.py`); T022 → T023 → T024
  after T021.
- **Phase 5 (US3)**: after Phase 4 (AX-12A is filled in T019 so T028 skips it). T025 → T026 →
  T027 (one test file); T028–T035 all parallel after T027 (T028–T034 also need T012's page
  list); T036–T041 parallel with each other, each after its group's task on the same files (T036
  after T029, T037 after T030, T038 after T031, T039 after T032, T040 after T033, T041 after
  T034); T042 after all.
- **Phase 6 (US4 T1)**: after Phase 5 (the lists are derived from `protocols`). T043 → T044 →
  T045 → T046 → T047.
- **Phase 7 (US5 T1)**: after Phase 6 (same file `dynamixel.ron`). T048 → T049 → T050 → T051.
- **Phase 8 (Polish T1)**: after Phase 7. T052 → T053 → T054; T055 before merge; T056 any time.
- **— T1 merges; X1 merges —**
- **Phase 9 (Setup T2)**: after X1 is in `main`. T057 → T058.
- **Phase 10 (US2 T2)**: after Phase 9. T059 ∥ T060 ∥ T061; T062 after T059; T063 after all.
- **Phase 11 (US3 T2)**: after Phase 10 (T060 handles `mx_64`). T064–T070 parallel; T071 after.
- **Phase 12 (US4 T2)**: after Phase 11 (kept sequential so each phase's codegen review shows
  only its own changes). T072 → T073 → T074.
- **Phase 13 (US5 T2)**: after Phase 12 (same file `dynamixel.ron`). T075 first; T076 ∥ T077;
  T078 after both; T079 last.
- **Phase 14 (Polish T2)**: after Phase 13. T080 → T081 → T082 → T083.

### User story dependencies

- **US1**: none beyond Foundational. Every other story needs its `source` field.
- **US2**: T1 slice needs US1 and `emanual.py`; T2 slice needs X1 and US4's map format.
- **US3**: needs US1 (row rules run on flagged tables). Independent of US2 except that AX-12A is
  filled by US2.
- **US4**: needs US3's `protocols` to derive the lists.
- **US5**: T1 slice needs US4's map rules; T2 slice needs US2's `mx_64_p1` layout and US4's
  legacy removal.

### Within each story

- Test tasks first; they fail before the rule exists.
- Rules in `check_tables.py` before the data that they validate.
- Data changes before the validate task that ends the phase.
- `test_check_tables.py`, `check_tables.py` and `dynamixel.ron` are single files: tasks on them
  never run in parallel. A table file is written by its group task (T028–T034) before its
  `operating_modes` task (T036–T041), never by both at once.

---

## Parallel Examples

### Foundational

```bash
Task: "T006 Parser tests in control_tables/test_check_tables.py"
Task: "T007 e-Manual helper tests in control_tables/test_emanual.py"
# after T008:
Task: "T009–T010 report frame in control_tables/check_tables.py"
Task: "T011 emanual.py implementation in control_tables/emanual.py"
```

### User Story 1

No parallel tasks: one test file, one tool file, one scripted edit over the 72 tables.

### User Story 2

```bash
# T1: sequential (one table).
# T2, after T057–T058:
Task: "T059 create control_tables/mx_64_p1.ron from …/dxl/mx/mx-64/"
Task: "T060 verify control_tables/mx_64.ron against …/dxl/mx/mx-64-2/"
Task: "T061 verify control_tables/ax_12_plus.ron against …/dxl/ax/ax-12a/"
```

### User Story 3

```bash
# T1, after T027 (seven agents plus the analysis script):
Task: "T028 AX group (4 files)"
Task: "T029 MX(2.0) + XL-320 group (4 files)"
Task: "T030 X430 group (15 files)"
Task: "T031 X540 group (11 files)"
Task: "T032 X330 group (12 files)"
Task: "T033 PRO / PRO+ / RH group (13 files)"
Task: "T034 Y group (12 files)"
Task: "T035 analyze_tables.py reads protocols and the new map"
# then, each after its group's task above (six agents):
Task: "T036 operating_modes, MX(2.0) + XL-320 group (4 files)"
Task: "T037 operating_modes, X430 group (15 files)"
Task: "T038 operating_modes, X540 group (11 files)"
Task: "T039 operating_modes, X330 group (12 files)"
Task: "T040 operating_modes, PRO / PRO+ / RH group (13 files)"
Task: "T041 operating_modes, Y group (12 files)"
# T2, after T063:
Task: "T064–T070 alias removal and corrections, one agent per group"
```

### User Story 4

No parallel tasks: every task touches `check_tables.py`, its test file or `dynamixel.ron`.

### User Story 5

```bash
# T2, after T075:
Task: "T076 create control_tables/mx_28_p1.ron from …/dxl/mx/mx-28/"
Task: "T077 create control_tables/mx_106_p1.ron from …/dxl/mx/mx-106/"
```

---

## Implementation Strategy

### MVP first (PR T1)

1. Phases 1–2: tools and parser.
2. Phase 3 (US1): every table flagged. **Stop and validate**: the check runs, CI's guarded
   `control tables: schema check` job could already pass.
3. Phase 4 (US2 T1): AX-12A verified, proving the verify workflow.
4. Phases 5–7: protocols, access/area and mode rows (seven parallel agents), the mode-source
   and `operating_modes` rules, `operating_modes` on the 67 tables with a mode row (six parallel
   agents), map lists, not-mapped records.
5. Phase 8: gates, PR T1. This unblocks X1, which needs `source`, `protocols`, `access`, `area`,
   `operating_modes`, `p1`/`p2` and exactly one mode source per table to exist.

### Incremental delivery

- T1 changes no generated file, so it can merge any time, independent of core and xtask.
- T2 waits for X1, then adds the remaining verified models, the MX P1 tables, the alias removals,
  the mode-row corrections (each with its `operating_modes`) and the legacy-key removal, and commits
  the regenerated `src/models*`.
- Later models are verified one at a time, each in its own PR, with quickstart §5 (US2 AS6).
  This tasks file does not schedule any.

## Notes

- [P] = different files and no dependency on an unfinished task.
- `src/models.rs` and `src/models/**` change only through `cargo xtask codegen`, and only in T2.
- Stop at any checkpoint to validate a story on its own.
- If any e-Manual value contradicts the plan's expectation (protocol set, model number, slope, mode
  register, mode value list), the page wins; record it in the findings and in the PR, and flag it to
  the architect.
