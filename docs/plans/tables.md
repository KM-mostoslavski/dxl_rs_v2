# Plan: tables

Open tasks copied from `specs/005-tables/tasks.md` on 2026-11-05, when the
project left spec-kit. Ids are the spec-kit ones, so the cross-references
(`Blocked on Tnnn`, other modules' ids, PR descriptions) still resolve.
Context for each task (plan, research, contracts, quickstart) stays in
[`specs/005-tables/`](../../specs/005-tables/).

The task texts are copied verbatim. Where they name spec-kit tools that are
gone (`/speckit-ov-srs`, `/speckit-constitution`, `/speckit-ov-pr`, `.aiignore`,
`docs/srs/modules/`, the constitution), read: edit `docs/srs/SRS.md` directly;
the constitution's principles are in `AGENTS.md`.

## Phase 8: Polish and PR (PR T1)

- [ ] T055 [human] Architect: approve the T1/T2 split (module-interfaces §9) and the AX-12+ record citing the AX-12A page (research R-9, US2 AS1 "its own model's page") before T1 merges; answer any `operating_modes` question T036–T041 raised (a position mode whose page does not say whether the limits apply), since T1 cannot pass the check without the field
- [ ] T056 [human] Architect (not blocking T1): amend `docs/srs/SRS.md` (in `.aiignore`) for the 2026-10-28 decisions (FR-3.7: limits follow the operating mode; the tables give the mode source and, in `operating_modes`, the meaning of each mode value, FR-020) and the stale roadmap line "AX-12+, AX-18F, XW430-T200: … Adding the number is enough" (their numbers are shared, research R-10), then regenerate `docs/srs/modules/*.md` with `/speckit-ov-srs` (plan Constitution Check, Governance row); update `docs/ARCHITECTURE.md` "Not implemented yet" (MX P1 tables, unmapped models); optionally add `python3 control_tables/check_tables.py` to the command list in `CLAUDE.md`

## FR-6.14: precise table-check errors (added 2026-11-09)

From the read-only audit of 2026-11-09 (line numbers at `dev` 49bdc7a).

- [ ] T210 `check_tables.py`: give every check a stable cause code and its fields as data (file, location, model, register, field, deciding values) next to today's `(file, location, message)` tuples (`:372-382`); add the found value where it is missing (`:434` size, `:781` Model Number); one exit code per failure class (exit 2 covers three causes today, `:11-12`); advice apart from the facts. Tests in `test_check_tables.py` check the cause code and fields, not only the message. FR-6.14, NFR-12
