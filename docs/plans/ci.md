# Plan: ci

Open tasks copied from `specs/007-ci/tasks.md` on 2026-11-05, when the
project left spec-kit. Ids are the spec-kit ones, so the cross-references
(`Blocked on Tnnn`, other modules' ids, PR descriptions) still resolve.
Context for each task (plan, research, contracts, quickstart) stays in
[`specs/007-ci/`](../../specs/007-ci/).

The task texts are copied verbatim. Where they name spec-kit tools that are
gone (`/speckit-ov-srs`, `/speckit-constitution`, `/speckit-ov-pr`, `.aiignore`,
`docs/srs/modules/`, the constitution), read: edit `docs/srs/SRS.md` directly;
the constitution's principles are in `AGENTS.md`.

## Phase 8: Polish & cross-cutting

- [ ] T041 Validate quickstart.md § 3–6 on GitHub and record the result in the PR description: coverage table reached in 2 clicks (SC-002); a PR from a fork (`gh repo fork`) with a one-line docs change shows the same checks, all run, `Contents: read` in "Set up job" (FR-011, SC-004); two pushes within a minute cancel the first run (FR-012); a PR that changes only `specs/` runs every check (FR-013); the compile-fail job installs 1.97.1 (US4 AS4). Close the probe PRs afterwards
- [ ] T043 [human] After the ci PR merges: mark the required checks of contract ci-checks.md § Required-check guidance in `.github/ov-ruleset-main.json` (`.aiignore` path; not in the repository yet, create it first, e.g. with `/speckit-ov-pr`): `fmt, clippy, test (Linux)`, `no_std build (thumbv7em-none-eabihf)`, `coverage (>= 70 % lines)`, `generated models up to date`, `feature sets`, `docs: complete, no warnings`, `doc examples: compile-fail error codes`, `control tables: schema check`, `std build and tests (Windows)`, `std build and tests (macOS)`. Not the individual `feature set (…)` / `feature tests (…)` legs
- [ ] T044 [human] Architect: add the new gates to the constitution's "Development Workflow and Quality Gates" list (`/speckit-constitution`), and update the checklist of `.github/pull_request_template.md` and the Commands section of `CLAUDE.md` (both outside `ci`'s paths, plan.md § Follow-ups)
- [ ] T045 [human] Architect, after merge: track SC-003 over the first 5 PR runs (≤ 15 min warm) and SC-006 over the first 20 PRs (≤ 1 unrelated blocking failure); decide on pinning actions by commit SHA (R-11, optional)
