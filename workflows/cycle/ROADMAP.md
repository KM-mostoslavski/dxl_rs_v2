# Roadmap: cycle

## Tooling

- Claude Code with Superpowers: `brainstorming`, `writing-plans`,
  `test-driven-development`.
- Claude Code `/goal` for every delegated build: the agent runs against
  the oracle until green, no questions, under `CLAUDE.md`.
- Project skills in `.claude/skills/`, all user commands: `/start`,
  `/issue`, `/contract`, `/walkthrough`. Step end is automatic: the
  agent runs it at the end of steps 5, 6 and 7 (see `PROCEDURE.md`).
- Project agent `.claude/agents/reviewer.md`: reads the SRS, `QA.md`
  and `docs/preferences.md`; used by `/walkthrough` only.
- Oracle = the `[oracle]` items in `QA.md`: `cargo test`, `cargo clippy`,
  pub diff, requirement coverage. It is the only merge gate. Reviewer =
  the `[reviewer]` items, read by the reviewer agent at step 10.

## Entry points

- **New module**: steps 1→9. Starts from a C4 box that has no code yet.
- **Change** (bug, small feature, refactor): steps 4→7. Starts from an
  existing module. Step 4 is a no-op when the pub surface is untouched;
  it is a real contract when it is not.
- **Walkthrough**: step 10, project-level, any time the app mostly
  runs. Optional, repeatable, the only code review in the cycle.
- **Existing project**: see "Adopting an existing project" in
  `PROCEDURE.md`. Every module enters at step 4.

## Dependency rule

Order comes from the skeleton (step 3): the `//!` doc comments and `use`
lines of the empty modules. A module enters step 4 only when every
module it depends on is at step 4 or later.

Modules that satisfy the rule are built in parallel, one loop and one
branch each. Nothing waits for me.

## Status

`profile: toy | poc | serious` — serious = full `QA.md`; poc = oracle
items only; toy = build + run only. `docs/srs/SRS.md` non-functional section
overrides `QA.md`. Start a project, or an adoption, on `poc`; move to
`serious` once the oracle is green on the base branch.

| module | owner | step | branch | last oracle run |
|---|---|---|---|---|
| _example_ | mine · delegated | 1–10 · `needs contract rework` | `mod/<name>` | `2026-10-29 green` |

- **owner** `delegated`: an agent builds it in a `/goal` loop under the
  step-4 contract. `mine`: I write it by hand, no agent touches its
  files. Default `delegated`.
- **step**: the step the module is *in*, updated at step end.
- **branch**: one branch per module, squashed onto the base branch by
  the agent at step 7 when the oracle is green.
- **last oracle run**: date and green/red, written at step end.
