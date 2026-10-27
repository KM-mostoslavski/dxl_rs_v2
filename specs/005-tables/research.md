# Research: Control tables

**Feature**: [spec.md](spec.md) | **Plan**: [plan.md](plan.md) | **Date**: 2026-10-27

Sources: `control_tables/*.ron` and `xtask/src/codegen.rs` on `dev`; `ron` 0.12.2 (xtask lock
file); module-interfaces sections 3, 4.3 and 7; e-Manual pages fetched 2026-10-27 (raw HTML
parsed, not summaries): AX-12A, AX-18A, MX-28, MX-64, MX-106, MX-28(2.0), MX-64(2.0), XM430-W350,
2XL430-W250, XL330-M077, XL-320, XW430-T200, XW430-T333, PH54-200-S500-R, H54-200-S500-R(A),
RH-P12-RN, RH-P12-RN(A), Protocol 1.0, Protocol 2.0. Prototypes in a scratch copy of the repo
under `/tmp/claude-1000/…/scratchpad/` (never in the repo).

## R-1 Today's codegen ignores the new fields

**Decision**: T1 adds every new field with no xtask change and no regeneration.

**Rationale**: prototype on a scratch copy of `dev`. All 72 tables got `source` (both
`Unverified` and `Verified(url: …, verified: …)`), `protocols`, `instructions`, and `access` +
`area` on every row, in both file styles (multi-line rows and one-line rows in the AX files). The
map got `p1`, `p2`, `default: P2`, `also` and `not_mapped` with `SameNumberAs(12)`. `cargo xtask
codegen` succeeded and `git status src/` showed 0 changed files. serde ignores unknown struct
fields (xtask has no `deny_unknown_fields`), and `ron` skips unit variants, tuple variants and
struct variants as ignored values. Unmapped files still warn, as expected (FR-7.4).

**Alternatives considered**: land the schema with X1 in one step. Rejected: X1 would need data that
does not exist yet, and the contract orders T1 before X1.

## R-2 Shape of the verification flag

**Decision**: top-level `source: Unverified` or
`source: Verified(url: "https://emanual.robotis.com/…", verified: "YYYY-MM-DD")`. One per table.

**Rationale**: contract 3.1 and the SRS "Meanwhile" shape `(url, verified)`, tagged so "unverified"
is a value, not a missing field (FR-001). A missing field is then always an error. The date is an
ISO string, so the check validates it with `datetime.date.fromisoformat` and `<= today` (FR-002).

**Alternatives considered**: `source: None` / `Option`. Rejected: "absent" and "unverified" would
look the same. A per-row flag: rejected by the spec (one flag per table).

## R-3 `protocols` comes from the page's Protocol Type values, not from register names

**Decision**: each table's `protocols` lists the Dynamixel protocols its page offers:
- the page has a Protocol Type register: the DYNAMIXEL protocols in its value table
  (1 → `P1`, 2 → `P2`); other values (Modbus-RTU, S.BUS, iBUS, RC-PWM) are ignored;
- no Protocol Type register: the protocol the page states for the model.

**Rationale**: the register is present in 60 of 72 tables, but its values differ (raw pages):

| Page | Protocol Type values | `protocols` |
|---|---|---|
| XM430-W350, 2XL430-W250, XW430-T200/T333, MX-28(2.0), MX-64(2.0) | 1 ~ 2 | `[P1, P2]` |
| XL330-M077 | 2 ~ 22 (2.0, S.BUS, iBUS, …; no 1) | `[P2]` |
| PH54-200-S500-R, RH-P12-RN(A) | 2, 10 (Modbus-RTU) | `[P2]` |
| MX-64 (Protocol 1 firmware) | no register; Protocol 1.0 | `[P1]` |
| AX-12A, AX-18A | no register; Protocol 1.0 | `[P1]` |
| XL-320 | no register; the page refers to Protocol 2.0 only | `[P2]` |

So "has a Protocol Type register" is not "dual protocol". The analysis script's heuristic (CW/CCW
Angle Limit means P1) puts XL-320 with P1, which the page contradicts (FR-005, FR-015).

**Alternatives considered**: infer from the register (wrong for XL330 and PRO+), or from the family
(XL-320 is in `x-series` but has no Protocol 1). Both rejected: constitution IV.

**Known limit**: on Protocol 1, addresses above 255 are unreachable (Protocol 1.0 page: 1-byte
address; MX-64(2.0) and XW pages: "Protocol 1.0 does not support addresses greater than 256").
Tables keep those rows; the check fails only when a role register sits above 255 in a table
mapped under `p1` (contract 4.3).

## R-4 Per-protocol map, and the transition

**Decision**: per model number, `p1` and/or `p2` firmware-ordered lists. When the page gives one
table for both protocols, the same file is in both lists. `default: P2` only when both lists exist.
T1 keeps the legacy `tables:` list, equal to the default protocol's list (or the only list). T2
deletes it, after X1 reads only `p1`/`p2`.

**Rationale**: contract 3.2 and conflict 11. FR-009 holds by construction: `tables` is unchanged in
T1, so today's codegen selects the same file. Expected count from the pages checked and the family
pattern, to be confirmed page by page in T1: 28 dual entries (25 X430/X540/2XL/2XC/XW numbers,
3 MX(2.0)), 32 P2-only (6 XL330/XC330 with two variants each, XL-320, 13 PRO/PRO+/RH-P12-RN(A),
12 Y), 3 P1-only (AX). T2 adds 3 P1-only (MX P1).

**Alternatives considered**: a `protocol` key per `(from_fw, file)` item in one list. Rejected:
the contract fixes the two-list shape, and it makes "default" and "missing protocol" harder to
read.

## R-5 Where instruction support comes from

**Decision**: `instructions` is a list of entries
`(protocol, source, list, from_fw?)`. `source` is the protocol page URL. Entries for the same
protocol add up; an entry with `from_fw: n` applies from firmware `n`. Verified tables must
cover each protocol in `protocols`; unverified tables carry none (contract 3.1: verified only).

**Rationale**: no model page lists instructions (AX-12A, MX-64 checked). The Protocol 1.0 and 2.0
pages list them with product and firmware conditions, verbatim:
- P1 Reboot: "MX-12W(V41), MX-28/64/106(V40), MX(2.0) and X Series (excluding XL-320)".
- P1 Bulk Read: "MX, MX(2.0) and X Series (excluding XL-320)".
- P2 Fast Sync Read / Fast Bulk Read: "X430/540 Series (Fw v45 or above, 2XL/2XC Not Supported),
  X330 (Fw v46 or above), P Series (Fw v12 or above), RH-P12-RN(A) (Fw v13 or above)".
- P2 Control Table Backup: "X430/540 Series(Fw v45 or above), X330 Series(Fw v46 or above),
  P Series(Fw v12 or above)".

So support depends on product and firmware, and the URL differs from the model page. Example for
AX-12A: `(protocol: P1, source: "https://emanual.robotis.com/docs/en/dxl/protocol1/", list:
[Ping, Read, Write, RegWrite, Action, FactoryReset, SyncWrite])` (no Reboot, no Bulk Read).

Instruction names: `Ping, Read, Write, RegWrite, Action, FactoryReset, Reboot, Clear,
ControlTableBackup, SyncRead, SyncWrite, FastSyncRead, BulkRead, BulkWrite, FastBulkRead` (both
pages' tables; Status 0x55 is a reply, not an instruction).

**Alternatives considered**: one flat list per protocol. Rejected: it cannot say "Reboot from
firmware 40" without losing the condition. Free text notes: rejected, the check cannot validate
them.

## R-6 The check is Python, standard library only, with its own RON-subset parser

**Decision**: `control_tables/check_tables.py`, Python ≥ 3.10, no third-party package. A small
tokenizer + recursive parser reads the RON subset these files use: `( … )`, `{ … }` (struct
style, as xtask reads it), `[ … ]`, `key: value`, strings, ints, floats, bare identifiers and
tagged values such as `Verified(…)` and `SameNumberAs(12)`, `//` comments. It rejects duplicate
keys, which catches a model number listed twice (FR-012); serde would keep the last one silently.

**Rationale**: prototype (~70 lines) parsed all 73 files of the scratch copy, both row styles,
in 0.12 s, and detected duplicate keys. Python is already the table tooling language
(`analyze_tables.py`), and runners have it. No `pip install` keeps CI and maintainer setup at zero.

**Alternatives considered**:
- An `xtask check-tables` subcommand using the real `ron` crate. Rejected: `xtask/**` is xtask's
  (no-go), and the spec places the check with the tables.
- The `pyron` / `ron` PyPI packages. Rejected: extra dependency, unmaintained, and they do not
  report duplicate map keys.
- Regex only, like `analyze_tables.py`. Rejected: cannot validate structure or catch a missing
  field reliably.

## R-7 Filling `access` and `area` from the pages

**Decision**: `control_tables/emanual.py` (stdlib `urllib`, `html.parser`) reads a model page,
finds its control tables by header names (`Address`, `Size`, `Data Name`, `Access`), and takes the
area from the heading above each table ("Control Table of EEPROM Area" / "… RAM Area"). Commands:
`rows URL` prints the page rows; `diff STEM URL` compares them with a table; `fill STEM URL` writes
`access` and `area` into a row only when address, size and name all match a page row. Every other
row is listed and left for a human:
- same address and size, other name (e.g. the alias rows of R-14): the human takes access and area
  from that page row and the name goes on the findings list;
- no page row at that address and size: nothing is written; the row is raised in the PR.

A URL list per table lives in the script (one line each), so reruns are reproducible.

**Rationale**: 4,635 rows on 72 pages cannot be typed by hand without errors. The pages differ in
columns: AX/MX/XL-320 have `Description` and `Min/Max`; X has `Initial Value / Range / Unit`; PRO+
adds a Modbus address column (seen on raw pages). Header-name lookup handles all four. Only `R` and
`RW` appear in every page checked; a `W` or anything else stops the fill and is raised (contract
3.1). Matching on all three of address, size and name means the tool never guesses.

**Alternatives considered**: copy access/area from one table per layout group (14 groups).
Rejected: constitution IV forbids sibling tables as a source. Read the ROBOTIS GitHub sources of the
e-Manual: rejected, the source must be `emanual.robotis.com`. Not committing the helper: rejected,
reviewers and later per-model verifications need to rerun it.

## R-8 `type_info` is checked against each page in T1

**Decision**: while filling `access`/`area`, T1 compares each table's `type_info` with its page
(position range and resolution) and fixes a wrong value from the page. The table stays
`Unverified`; the PR lists the fix (spec edge case "mistake found in an unverified table").

The fields mean: `angle = (raw − zero) × rad_per_tick`, and `min_radian` / `max_radian` are that
angle at the min and max values. A symmetric pair (±π, ∓150°) only fits when the zero sits
exactly between min and max, which is not the case for 0..=4095 or 0..=1023.

**Rationale**: today's codegen ignores `type_info`, so a fix changes no generated file. X1 starts
reading it for `PositionScale` (contract 2.3), where a wrong value moves an actuator to a wrong
angle. Core uses only zero and the slope `(max_radian − min_radian) / (max − min)` (core R-7).
Three findings:

- **XL-320** (confirmed): `xl320.ron` has 2048 / 0 / 4095 and ±π, but the page gives Goal
  Position 0 ~ 1023, 0.29°, joint mode 0 ~ 300°. It gets the AX values below.
- **X-series, MX(2.0), 2XL/2XC** (41 tables, flagged by core R-7): 2048 / 0 / 4095 with ±π, a
  slope of 2π/4095. The pages give 4096 pulses per turn (0.088°), center 2048, so the slope is
  2π/4096. T1 sets `min_radian: -3.1415927` (−2048 ticks), `max_radian: 3.1400587` (π·2047/2048,
  +2047 ticks), each after checking the model's page. This changes the slope by 0.024 %, half a
  tick at ±π.
- **AX** (5 tables): zero 512 with `min_radian: 0.0`, `max_radian: 5.2359877`, so 0..=1023 over
  0..300°, a slope of 300°/1023. T1 keeps that slope and moves the angles to the zero:
  `min_radian: -2.6205530` (−512 ticks), `max_radian: 2.6154347` (+511 ticks). ∓150° would need
  the zero at 511.5. No behaviour change.

If a page gives a slope that differs from the table (e.g. 300°/1024 for an AX page), T1 follows
the page and lists the change in the PR. The PR description lists every `type_info` change.

**Alternatives considered**: wait for per-model verification. Rejected: X1 would generate a wrong
XL-320 scale. Fixing it in X1: rejected, `control_tables/**` is ours.

## R-9 What verifying a table means, and the four priority tables

**Decision**: a table is marked `Verified` only after every value in the file is checked on the
cited page on that date: every row (address, size, name, access, area), `type_info`, every
`unit_info` entry, `protocols`, and `instructions` (on the protocol pages). A row or `unit_info`
entry the page does not give is removed. Pages:

| Table | Page | Model Number on page | PR |
|---|---|---|---|
| `ax_12a` | `https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/` | 12 | T1 |
| `mx_64` (MX-64(2.0)) | `https://emanual.robotis.com/docs/en/dxl/mx/mx-64-2/` | 311 | T2 |
| `ax_12_plus` | `https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/` | 12 | T2 |
| `mx_64_p1` | `https://emanual.robotis.com/docs/en/dxl/mx/mx-64/` | 310 | T2 |

Rule: a verification lands in T1 only if it changes nothing codegen reads today (an address, a
size, or a name that changes the const ident). A name that differs only in case (e.g. "BUS
Watchdog") gives the same const ident and can land in T1.

**Rationale**: FR-004, FR-018. Pre-check on the raw pages, 2026-10-27:
- `ax_12a`: all 32 rows match the page exactly (address, size, name). Access is R/RW, areas
  EEPROM/RAM. It can land in T1.
- `mx_64`: the 54 real rows match, but 4 alias rows ("Indirect Address Write/Read", "Indirect Data
  Write/Read", R-14) are not on the page. Removing them changes generated consts, so MX-64(2.0)
  is verified in T2. The page shows the indirect blocks as "Indirect Address 1, 2, 3, …, 26, 27,
  28" (and 29 … 56 at 578); the verified table keeps the rows the existing table already has
  (Indirect Address 1, Indirect Data 1) and adds no others in this feature. `unit_info` holds values
such as AX "Torque 0.001465 N·m/bit" that the page may not give; FR-003 says such a value is left
out.

**AX-12+ has no page of its own.** `…/dxl/ax/ax-12+/` returns 404. The AX-12A page is headed
"AX-12+, AX-12A", lists weights for AX-12, AX-12+ and AX-12A, and says "AX-12A is a new version of
the AX-12+ with the same performance". So it is AX-12+'s own page. Its record cites that URL.
Flagged because US2 AS1 says "its own model's page, not a sibling's".

**Alternatives considered**: verify `mx_106` alongside `mx_64` (same layout). Rejected: FR-018, no
bulk verification.

## R-10 The three unmapped tables

**Decision**: all three get a `not_mapped` record in T1, with the reason the page gives, and their
product name goes into `also` on the existing entry in T2:

| File | Page | Model Number | Layout | Record |
|---|---|---|---|---|
| `ax_12_plus` | AX-12A page ("AX-12+, AX-12A") | 12 | identical to `ax_12a` | `SameNumberAs(12)`, `also: ["AX-12+"]` on 12 |
| `ax_18f` | AX-18A page ("AX-18F, AX-18A") | 18 | identical to `ax_18a` | `SameNumberAs(18)`, `also: ["AX-18F"]` on 18 |
| `xw430_t200` | `…/dxl/x/xw430-t200/` | 1,270 | identical to `xw430_t333` (rows and `type_info`) | `SameNumberAs(1270)`, `also: ["XW430-T200"]` on 1270 |

**Rationale**: FR-011, FR-012, US5 AS4. Keeping the files (rather than deleting them) follows the
contract example and keeps a table to verify for AX-12+ (FR-018). The XW430-T200 number is what the
page says; it equals XW430-T333's, which looks like an e-Manual error. It is raised to the
architect, not corrected from memory.

**Alternatives considered**: delete identical files. Allowed by US5 AS4, but the contract keeps
`ax_12_plus`; left to the architect as a later cleanup. `NoModelNumber` for XW430-T200 (contract
example): rejected, the page gives a number.

## R-11 MX tables for Protocol 1 firmware

**Decision**: T2 adds `mx_28_p1.ron`, `mx_64_p1.ron`, `mx_106_p1.ron`, family `mx`, map entries
29 "MX-28", 310 "MX-64", 320 "MX-106", `p1` only, from the pages `…/dxl/mx/mx-28/`,
`…/mx-64/`, `…/mx-106/`. Rows come from `emanual.py rows`, reviewed against the page. `mx_64_p1`
is verified; the other two stay `Unverified` (US5 AS2).

**Rationale**: model numbers read from the raw pages (29, 310, 320). The pages have no Protocol
Type register and point to MX(2.0) for Protocol 2, so `protocols: [P1]`. Separate numbers mean no
clash with 30, 311, 321. File stems `*_p1` match the contract example and give the types `Mx64P1`
etc. A firmware boundary inside one page is mapped only when the page states it (spec edge case).

**Alternatives considered**: put the P1 tables under 30/311/321 `p1`. Rejected: the pages give
different numbers; those entries already get `p1` = the MX(2.0) table (Protocol Type 1).

## R-12 CI and PR description

**Decision**: the check is a script with exit codes (0 clean, 1 errors, 2 usage or parse error). It
has a `--markdown` mode that prints, per table named on the command line, the flag and URL/date,
for the PR description (FR-017). CI wiring is the `ci` module's, not done here. ci runs it in its
`control tables: schema check` job, guarded until T1 merges (specs/007-ci contracts).

**Rationale**: FR-014 asks for a repeatable check CI can run, and `.github/workflows/**` is
ci's. Corrected and removed rows are listed from `git diff` plus
`emanual.py diff`.

**Alternatives considered**: a GitHub Action inside this PR. Rejected: file ownership.

## R-13 Analysis report

**Decision**: `analyze_tables.py` reads `protocols` from each table and the model numbers from the
new map (`p1`/`p2`/legacy `tables`). Its protocol section groups P1-only, P2-only and both. The
report is regenerated in T1 and T2.

**Rationale**: FR-015, SC-006. Its model-number loader looks for `"<file>.model"` keys that no
longer exist, so the report's "Model #s" column is empty today ("—" in every row).

**Alternatives considered**: drop the analysis. Rejected: FR-015 requires it.

## R-14 Alias rows that are not on the e-Manual

**Decision**: 66 tables carry four alias rows, "Indirect Address Write" and "Indirect Data Write"
(same address and size as "Indirect Address 1" / "Indirect Data 1") and "Indirect Address Read" /
"Indirect Data Read" (at the address of "Indirect Address 29" / "Indirect Data 29"). The pages do
not list these names. In T1 their `access` and `area` come from the page row at the same address
and size (R-7), and they go on the findings list. T2 removes them from every table where
`emanual.py diff` showed the page does not list the name. The tables stay `Unverified`; the PR
lists each removal.

**Rationale**: FR-003 and US2 AS3: a row the e-Manual does not list is removed. Removal changes
generated consts, so it waits for T2, which regenerates anyway. The evidence is the T1 diff output,
so no extra lookup is needed. Not removing them would keep non-e-Manual names in 66 generated types.

**Alternatives considered**: remove them only when each model is verified. Rejected: they are known
mistakes now, and the spec's edge case says a found mistake is fixed and listed. Remove them in T1:
rejected, it changes generated output before X1.

## Summary of unknowns

Every Technical Context item is resolved. Open items are for the architect, not blockers: the shared
number 1270 (R-10), AX-12+ page citation (R-9), approval of the T1/T2 split (plan), and CI wiring of
the check (R-12).
