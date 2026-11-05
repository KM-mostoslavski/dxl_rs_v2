# Procedure: the 10 steps

One paragraph per step: who does it, the exit test, the tool. Steps 5,
6 and 7 end with the automatic step end below, which checks the exit
test before moving the ROADMAP row. Steps 1 to 4 move the row from the
skill that runs them. Step 10 is project-level and has no row.

## Rules that span steps

- Dependency order comes from the skeleton (step 3). A module enters
  step 4 only when every module it depends on is at step 4 or later.
  Modules that satisfy the rule run in parallel, one loop each.
- The pub surface is frozen at the end of step 4. Changing it afterwards
  is a deferral, not a decision.
- One `feature/<module>` branch per module, cut from `dev`. At step 7
  the agent opens a PR into `dev`; I review it and squash-merge it. An
  agent never merges.
- A **change** (bug, small feature, refactor) enters at step 4 and exits
  at step 7. A **new module** runs 1→9. A **walkthrough** is step 10.
- `docs/preferences.md` is read before any code is written and obeyed
  like `QA.md`.

## Feature requests during a build

`/issue feature <text>`: one line in `docs/deferred.md`, a GitHub issue
if there is a remote, keep building. Same for `bug` and `question`.
At the next `/start`, I triage each line next / later / drop:
small (fits existing code) = steps 4 to 7 on its own branch; big (new
module or changes a merged contract) = back to step 3. Only I may
interrupt a running build.

## Step end (agent, automatic)

At the end of steps 5, 6 and 7 the agent does this itself. No question,
one line in the summary saying it was done.

1. Verify the exit test of the step is met. If not, say which part
   fails and stop. Never end a step on a promise. Ending step 7:
   confirm the squash commit is on `dev` first.
2. Update the module's row in `workflows/cycle/ROADMAP.md`:
   - `step` → the next step, or `needs contract rework` after 3 open
     deferrals on this module
   - `last oracle run` → today's date and green/red, if the oracle ran
   - `branch` → the current branch name, if the column is empty
3. Tick the tasks in `docs/plans/<module>.md` whose IDs appear in
   commits since the last step end.
4. Append to `docs/deferred.md` every deferral raised during the step
   that is not already there. One row each, status `open`.
5. Append one line to `docs/learnings.md`: date, module, step, one
   sentence on what to do differently next time. If there is nothing
   to learn, write `nothing new`.

Never edit another module's row. Never remove a deferral: set status
`done` with the date instead.

## 1. Planning (C4 C1/C2)

**Who:** me, with the agent asking questions. **What:** system context
(C1) and containers (C2) in `docs/workspace.dsl`, rendered into
`docs/c4/`. Each container gets a name, one responsibility sentence and
its neighbours. **Exit:** every container in C2 has those three things,
and I can say in one sentence why the project exists. **Tool:**
`superpowers:brainstorming`, Structurizr DSL.

## 2. Defining (POC-ready SRS, holes allowed)

**Who:** me drafting, agent probing for ambiguity. **What:**
`docs/srs/SRS.md`, one `## Module: <module>` section per C2 container,
requirements numbered with SRS ids (`FR-4.5`, `NFR-2`), each one
testable sentence, and a `Covers:` line per module. Holes are allowed
and marked **\[decision\]** or as open questions, and in code
`unimplemented!("SRS <id>: <question>")` (SRS 5.0); the SRS is
POC-ready, not final. This is where my
architectural and data decisions are written down; the agent builds
from here. **Exit:** every C2 container has an SRS section with at
least one testable requirement, and every hole is marked. **Tool:**
`superpowers:brainstorming`, plain markdown.

## 3. Design (HLD, stack, API intent, skeleton)

**Who:** me deciding, agent drafting. **What:** a high-level design in
`docs/plans/<date>-hld.md`: stack, crate layout, module list with
dependency arrows, API intent per module in words. Then the skeleton:
every module as an empty file with a `//!` doc comment stating its
responsibility and dependencies, wired with `mod` lines, compiling.
Each module gets a C3 box in `docs/c4/<module>.dsl`, included from
`docs/workspace.dsl`. **Exit:**
`cargo build` green on the skeleton, dependency order readable from the
`//!` comments, every module has a C3 box. Ends with the skeleton and
the owner column filled (`?` until I set it). **Tool:** run
`superpowers:brainstorming` (skip if the SRS is complete enough), then
`superpowers:writing-plans`. Output: `docs/plans/<module>.md` with
tasks numbered `T<nnn>`, agent tasks only (`[human]` tasks stay in the
plan, owned by me). This file is the task
list for steps 5 to 7. Profile `toy`: skip and say so.

## 4. Contract (pub stubs + test names, todo!() bodies)

**Who:** agent via `/contract`. **Contract** = what the compiler can
check at the module boundary. Lib crate: pub API + test names. Bin
crate: internal signatures + struct layouts. If profile is `toy`, or
the SRS says no tests, say "step 4 folded into 5" and continue.
**What:** for one module, from its SRS section: every `pub` item with
full signature and `///` doc comment, body `todo!()`; hot structs
`#[repr(C)]` with a `size_of` test; one `#[test]` per requirement,
named after it, body `todo!()`. Nothing private, nothing implemented.
**Exit:** `cargo build` green, `cargo test` compiles, every SRS id the
section covers has a test that names it, the pub surface is
printed in the summary. No sign-off: the module goes to step 5 at once.
I veto later with `/issue`, which sends the module back to step 4.
**Tool:** `/contract`.

## 5. Build

**Entry:** run `/mode` if the row has no mode; otherwise use the row.
**Who:** owner `delegated`: an agent, in the row's mode (`loop` = `/goal`,
I am not in the room). Owner `mine`: me, by hand. **What:** fill the `todo!()`
bodies under the step-4 contract, the `CLAUDE.md` decision policy and
`docs/preferences.md`. Commit per task, Conventional Commits with the
task ID first in the description (`fix(bus): T090 ...`); step 9 changes
use the issue number (`fix(bus): #42 ...`).
Contract breaks go to `docs/deferred.md` and the test is
`#[ignore = "deferred"]`; the loop continues. **Exit:** oracle green,
no `todo!()` left, every deferral names its smallest unblocking
change, at most 3 open deferrals. After 3 the loop stops the module
and sets `needs contract rework`. **Tool:** `/goal` with the oracle,
`superpowers:test-driven-development`.

## 6. Verify

**Who:** agent, alone. **What:** the agent walks `QA.md`: runs every
`[oracle]` item and writes a report of one screen: pass/fail per item,
every `// DECISION:` listed, every deferral listed, pub diff shown.
The report goes into the PR description at step 7. An unmet requirement (a failing test named for an SRS id) sends
the module back to step 5, fix, verify again, no question. Round cap
per profile: `toy` 1, `poc` 2, `serious` 3. After the cap, remaining
unmet requirements go to `docs/deferred.md` tagged `unmet`, their tests
are marked `#[ignore = "deferred"]`, and the module continues to
step 7. A time box in the SRS overrides the cap. **Exit:** oracle
green, report written. **Tool:** `QA.md`.

## 7. Merge

**Who:** agent opens the PR, I merge. **What:** a PR from
`feature/<module>` into `dev` with `.github/pull_request_template.md`
and the step-6 report, opened when the oracle is green on the branch.
I review and squash-merge it. Message: Conventional Commits, module,
SRS section, one line on what it does. Branch deleted. `dev` reaches
`main` through its own PR. **Exit:** `dev` builds, oracle green on
`dev`, ROADMAP row says step 8 (new module) or is closed (change). **Tool:** git, then the automatic step end.

## 8. Deploy (augmented)

**Who:** me, agent assisting. **What:** whatever deploy means for this
project, as written in the HLD: publish a crate, cut a release tag,
build a binary, flash a target. The agent drafts release notes from the
squash messages and checks that the SRS version matches. **Exit:** the
artifact exists where the HLD says, the tag exists, the ROADMAP row says
8. **Tool:** project-specific, named in `docs/plans/<date>-hld.md`.

## 9. Maintenance (issues → deferred.md → step 4)

**Who:** me triaging, agent executing. **What:** every issue found in
use, in a bench, or in a walkthrough goes into `docs/deferred.md` with
a module name and a step. Each one re-enters the cycle as a change at
step 4: contract if the pub surface moves, straight to 5 if not.
**Exit:** `docs/deferred.md` has no untriaged entry; every open entry
has a module and a step. **Tool:** `/issue`, `/start`, `/contract`.

## 10. Walkthrough (the code review)

Not adopted in dxl_rs_v2: `/walkthrough` and the reviewer agent are
not copied; the step-7 PR review stands in for it. Kept for when they
are.

**Who:** me, in depth, with the reviewer agent. **When:** any time the
app mostly runs; optional, repeatable, the only code review in the
cycle. **What:** on the first run the agent writes
`docs/plans/walkthrough.md`: every entry point (binaries, pub macros,
API functions, examples, test harnesses), the modules each one
reaches, in the order to walk them. I reorder or cut it. Then, entry
point by entry point, function by function down the call graph: the
agent says what the function does and which requirement it serves, the
reviewer agent checks it against the SRS as it is now and against
`docs/preferences.md`, and answers in one line. A mismatch is an issue
on the spot. Each thing I dislike is an issue; the second time I
dislike the same shape, the pattern goes into `docs/preferences.md`.
One function at a time, at my pace, never ahead of me. **Exit:** every
row of `docs/plans/walkthrough.md` ticked, or I say stop; the plan
carries the position across sessions. Outputs are issues, preference
lines and ticks; no report. **Tool:** `/walkthrough`,
`.claude/agents/reviewer.md`.

## Adopting an existing project

1. Delete the old rulebook: any AGENTS.md policy, agent sections of
   CONTRIBUTING, spec-kit folders, task lists. They are not migrated.
   Design documents (SRS, C4, decision records) stay as input.
2. Copy the workflow as `README.md` says. Profile `poc`.
3. Every module enters at step 4 with a contract drafted from its
   current pub surface and its SRS section. Owner `delegated` unless I
   say `mine`. No module enters at step 5 without a contract.
4. Make the oracle green on the base branch before the first loop, or
   shrink the oracle (profile) until it is.
5. `/start`.
