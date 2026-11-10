# Roadmap: cycle

## Tooling

- Claude Code with Superpowers: `brainstorming`, `writing-plans`,
  `test-driven-development`.
- Claude Code `/goal` for every build in mode `loop`: the agent runs
  against the oracle until green, no questions, under `AGENTS.md`
  (`CLAUDE.md` is a symlink to it).
- Project skills in `.claude/skills/`, all user commands: `/start`,
  `/issue`, `/mode`, `/contract`. Step end is automatic: the agent runs
  it at the end of steps 5, 6 and 7 (see `PROCEDURE.md`).
- `/walkthrough` and the reviewer agent of the cycle are not adopted
  here; the maintainer reviews each module PR at step 7.
- Oracle = the `[oracle]` items in `QA.md` and the local commands of
  `.github/workflows/ci.yml` (`AGENTS.md` § Commands). An agent never
  merges: the maintainer squash-merges the PR into `dev`.

## Entry points

- **New module**: steps 1→9. Starts from a C4 box that has no code yet.
- **Change** (bug, small feature, refactor): steps 4→7. Starts from an
  existing module. Step 4 is a no-op when the pub surface is untouched;
  it is a real contract when it is not.
- **Walkthrough**: step 10, project-level, any time the app mostly
  runs. Optional, repeatable, the only code review in the cycle.
- **Existing project**: see "Adopting an existing project" in
  `PROCEDURE.md`. dxl_rs_v2 did not follow it: see the Status note.

## Dependency rule

Order comes from the skeleton (step 3): the `//!` doc comments and `use`
lines of the empty modules. A module enters step 4 only when every
module it depends on is at step 4 or later.

Modules that satisfy the rule are built in parallel, one loop and one
`feature/<module>` branch each.

## Status

`profile: toy | poc | serious` — serious = full `QA.md`; poc = oracle
items only; toy = build + run only. `docs/srs/SRS.md` non-functional section
overrides `QA.md`. Start a project, or an adoption, on `poc`; move to
`serious` once the oracle is green on the base branch.

`profile: serious`

Every module entered the cycle on 2026-11-05 from spec-kit, already
built and merged into `dev`, so each one starts at the step its code is
really at, without a step-4 contract: the open spec-kit tasks were
carried into `docs/plans/<module>.md` instead of being dropped. The
pub surface on `dev` at that date stands for the step-4 contract.
protocol has no open task left, so its step 5 is done: it is at step 6.
The others are at step 5 until their plan is ticked or deferred.

| module | owner | mode | step | branch | last oracle run |
|---|---|---|---|---|---|
| core | delegated | loop | 5 | — | `2026-11-05 green`: all 20 local CI commands |
| protocol | delegated | loop | 6 | — | `2026-11-05 green`: all 20 local CI commands |
| transport | delegated | loop | 5 | — | `2026-11-05 green`: all 20 local CI commands |
| bus | delegated | loop | 5 | `feature/bus` | `2026-11-10 green` (typed settings T104 to T110): the 13 commands of `AGENTS.md` § Commands, coverage 98.29 %, codegen and table analysis clean, 76 tables 0 errors; CI feature-test legs red, as on `dev` (D-02) |
| tables | delegated | loop | 5 | — | `2026-11-05 green`: all 20 local CI commands |
| xtask | delegated | loop | 5 | fix/xtask-errors | `2026-11-05 green`: all 20 local CI commands |
| ci | delegated | loop | 5 | — | `2026-11-05 green`: all 20 local CI commands |

- **owner** `delegated`: an agent builds it under the step-4 contract.
  `mine`: I write it by hand; agents do not touch its files without
  listing them first. `?`: not set yet.
- **mode**: set with `/mode`. `draft`, `pair` or `loop` (`/goal`). `?`:
  not set yet; nothing is built on a row without a mode.
- **step**: the step the module is *in*, updated at step end.
- **branch**: one `feature/<module>` per module, squash-merged into
  `dev` by the maintainer at step 7.
- **last oracle run**: date and green/red, written at step end.
