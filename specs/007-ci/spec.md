# Feature Specification: CI (hardened quality gates)

**Feature Branch**: `007-ci` (spec directory only; work stays on `dev`)

**Created**: 2026-10-27

**Status**: Draft

**Input**: User description: "ci module of docs/srs/modules/ci.md"
→ SRS module `ci` (SRS §9): NFR-1 (`no_std` + heap-free), NFR-6 (coverage), the FR-7.3 codegen
diff check; delivery step 5. Paths: `.github/workflows/**`, `Cargo.toml`. No-go: `src/**`,
`control_tables/**`, and every path in `.aiignore` (including `.github/workflows/ov-*.yml`).

## User Scenarios & Testing *(mandatory)*

CI already runs most of the constitution's gates (see Assumptions). This feature closes the gaps
between what the SRS and constitution promise and what CI proves today.

The "users" of this module are contributors and agents opening PRs (they want a clear red or
green, fast), the architect reviewing and merging (they want each promise proved by a machine,
not by reading the diff), and downstream users of the crate (the configurations they pick are
the ones CI built).

### User Story 1 - Prove the no_std, heap-free promise as the SRS states it (Priority: P1)

The architect merges a PR knowing the crate really builds for a microcontroller with default
features off, and that no code reachable without `std` allocates. Today CI only lints that
target (a lint pass does not build the crate), and only with every model family and `mock`
switched on.

**Why this priority**: NFR-1 and constitution I are NON-NEGOTIABLE, and the embedded user class
is the binding constraint. A lint-only check is weaker than the stated promise.

**Independent Test**: On a throwaway branch, add a heap allocation to code reachable without
`std`; the no_std build check turns red. Revert; it turns green.

**Acceptance Scenarios**:

1. **Given** a PR, **When** CI runs, **Then** the library is built (not only linted) for
   `thumbv7em-none-eabihf` with default features off and nothing else enabled, and again with
   every model family and `mock` enabled.
2. **Given** a change that allocates, or uses `std`, in code reachable without the `std` and
   `alloc` features, **When** CI runs, **Then** the no_std build fails and names the item.
3. **Given** the `alloc` feature on and `std` off, **When** CI runs, **Then** the build for the
   microcontroller target passes.
4. **Given** FR-003 is in scope and a minimal firmware program calls the crate's public API,
   **When** CI runs, **Then** that program links for the microcontroller target.

---

### User Story 2 - Every feature set a user can pick builds (Priority: P1)

A downstream user compiles only the model families they need (NFR-7). Whatever single feature
they pick, the crate builds. Today CI builds only the default set and one hand-written no_std
set, so a family that silently depends on another family's code would pass CI.

**Why this priority**: Features are a public promise (constitution VII: features add API only;
constitution II: a compiled-out family fails Setup with `UnsupportedModel`, not a build error).
A broken single-family build reaches users unseen.

**Independent Test**: On a throwaway branch, make one family's code use an item gated behind
another family; the check for that family alone turns red and names the feature set.

**Acceptance Scenarios**:

1. **Given** a PR, **When** CI runs, **Then** the crate builds with no optional feature, with
   each declared feature alone, with the defaults, and with all features.
2. **Given** a change that breaks one feature set only, **When** CI runs, **Then** the failure
   names that feature set.
3. **Given** a new feature declared in `Cargo.toml` (e.g. `partial-reads`), **When** CI runs,
   **Then** it is built alone without any workflow edit.
4. **Given** a PR, **When** CI runs, **Then** the test suite also runs with every feature
   enabled at once.

---

### User Story 3 - See the coverage figure on every PR (Priority: P2)

A reviewer sees the line coverage of the PR on the PR page, without opening job logs. Today the
70 % gate fails or passes, but the figure is only in the raw log.

**Why this priority**: NFR-6 says every PR *reports* its coverage. The gate already exists, so
this is a visibility fix, not a new safety net.

**Independent Test**: Open any PR after the change; the coverage figure and the 70 % threshold
appear in the run summary.

**Acceptance Scenarios**:

1. **Given** a PR, **When** the coverage check finishes, **Then** the line coverage figure
   (generated models excluded) and the threshold appear in the run summary linked from the PR.
2. **Given** coverage below 70 %, **When** the check finishes, **Then** it fails and the
   summary still shows the figure.
3. **Given** a PR from a fork, **When** the check runs, **Then** the figure appears without any
   write permission or secret.

---

### User Story 4 - Docs are complete and their examples fail for the right reason (Priority: P2)

The architect relies on CI, not review, to catch an undocumented public item, a broken doc link,
or a compile-fail example that fails for the wrong reason. Today none of the three is checked:
stable rustdoc accepts a `compile_fail,E0277` example that fails with any error at all.

**Why this priority**: Constitution VII requires every public item documented. Constitution II
proves "absent over failing" with compile-fail examples; an example that fails for the wrong
reason proves nothing. The `bus` module will add many such examples.

**Independent Test**: On a throwaway branch, remove a doc comment from a public item, break an
intra-doc link, and change a compile-fail example so it fails with another error code; each
turns its check red.

**Acceptance Scenarios**:

1. **Given** a public item without documentation, with any feature set, **When** CI runs,
   **Then** a check fails and names the item.
2. **Given** a doc warning (broken intra-doc link, bad code block), **When** CI runs, **Then**
   the docs check fails.
3. **Given** a compile-fail example that names an error code, **When** the example fails with a
   different code, **Then** CI fails.
4. **Given** a toolchain update unrelated to the PR, **When** CI runs, **Then** the
   compile-fail check gives the same result as before (its toolchain changes only by a
   deliberate commit).

---

### User Story 5 - The std build works on every desktop OS the SRS names (Priority: P2)

A robot application developer on Windows or macOS (SRS 4.3) builds the crate with defaults and
opens a serial port. Today CI builds only on Linux, so the Windows-only serial-port code path
has never been compiled by CI.

**Why this priority**: SRS 4.3 lists Linux, macOS and Windows as targets of the `std` build.
The Windows branch of the serial adapter is real code with no CI coverage.

**Independent Test**: On a throwaway branch, introduce a type error in the Windows-only serial
code; only the Windows check turns red.

**Acceptance Scenarios**:

1. **Given** a PR, **When** CI runs, **Then** the default build and every test that needs no
   serial device pass on Linux, Windows and macOS.
2. **Given** a change that breaks OS-specific code, **When** CI runs, **Then** the check for
   that OS fails and names the OS.

---

### User Story 6 - The oldest supported Rust version is stated and checked (Priority: P3)

A user on an older stable toolchain reads the crate's minimum Rust version from its metadata and
can trust that CI built the crate with it. Today the minimum (1.85, the edition 2024 floor)
appears only in spec notes; the crate does not declare it and CI uses latest stable only.

**Why this priority**: No SRS item or principle requires a minimum version; it is a DX nicety,
and its scope is open (FR-010).

**Independent Test**: On a throwaway branch, use a language or library feature newer than the
declared minimum; the minimum-version check turns red.

**Acceptance Scenarios**:

1. **Given** the crate metadata, **When** a user reads it, **Then** it states the minimum Rust
   version.
2. **Given** a change that needs a newer Rust than declared, **When** CI runs, **Then** the
   minimum-version check fails.

---

### Edge Cases

- A new gate lands while `main` fails it. Today two public fields of
  `Error::UnsupportedModel` (`src/error.rs`) lack docs, so the missing-docs check of FR-007
  would be red on day one. This module may not touch `src/**`; the fix lands first in its own
  PR (owned by the error module), then the gate.
- The nightly or pinned toolchain used by FR-008 breaks or is withdrawn: the check fails with a
  message naming the toolchain, and is fixed by a deliberate toolchain bump, not by the PR
  author.
- A PR from a fork: no secrets, no write token. Every blocking check must still run and report.
- Several pushes to one PR in a row: older runs are cancelled so the PR shows the newest result.
- A docs-only or spec-only PR: every blocking check still runs, so required checks never stay
  "pending" and block the merge.
- A runner image drops a system package the build needs (e.g. `libudev` for the serial port on
  Linux): the setup step fails visibly, not the build step with a confusing linker error.
- A Linux-only test (pseudo-terminal pair) on Windows: it is skipped by its own platform gate,
  not by the workflow.
- The feature matrix grows with every new feature: the cost grows linearly (each feature alone),
  never with every combination.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Every existing gate MUST keep running with the same strength: format check; lint
  with warnings denied, with and without `mock`; workspace tests; `mock` doc examples; no_std
  lint on `thumbv7em-none-eabihf`; coverage ≥ 70 % with generated models excluded; codegen diff
  check (FR-7.3). No gate may be removed or weakened.
- **FR-002**: CI MUST build (not only lint) the library for `thumbv7em-none-eabihf` with default
  features off and nothing else enabled, with `alloc` alone, and with every model family plus
  `mock`. A build without `std` and `alloc` MUST fail if any reachable code allocates (NFR-1,
  constitution I).
- **FR-003**: The no_std check MUST [NEEDS CLARIFICATION: also link a minimal firmware binary
  that calls the public API on `thumbv7em-none-eabihf`? This needs a small test program outside
  this module's paths (`.github/workflows/**`, `Cargo.toml`). Default if unanswered: library
  build only (FR-002).]
- **FR-004**: CI MUST build the crate with no optional feature, with each feature declared in
  `Cargo.toml` alone, with the defaults, and with all features. Features that need `std` are
  built on the host; all others also on the microcontroller target. A newly declared feature
  MUST be picked up with no workflow edit (NFR-7, constitution II and VII).
- **FR-005**: CI MUST run the workspace tests with every feature enabled at once.
- **FR-006**: The coverage check MUST publish the line coverage figure and the 70 % threshold
  in the run summary reachable from the PR, pass or fail, with no write permission (NFR-6).
- **FR-007**: CI MUST fail when a public item lacks documentation under any checked feature set,
  and when the docs build reports any warning (constitution VII).
- **FR-008**: CI MUST fail when a compile-fail doc example that names an error code fails with a
  different code (constitution II). The toolchain this check uses MUST change only by a
  deliberate commit.
- **FR-009**: CI MUST build the crate with default features and run every test that needs no
  serial device on Linux, Windows and macOS (SRS 4.3).
- **FR-010**: The crate MUST [NEEDS CLARIFICATION: declare and enforce a minimum supported Rust
  version (1.85, the edition 2024 floor) in `Cargo.toml` and a CI job, or support latest stable
  only until 1.0? Default if unanswered: latest stable only; User Story 6 is dropped.]
- **FR-011**: Workflows MUST run with read-only repository permissions and no secrets, so fork
  PRs get the same checks. No PR-controlled code may run on a self-hosted runner.
- **FR-012**: A newer push to the same PR MUST cancel the older, still-running CI run.
- **FR-013**: Blocking checks MUST NOT be skipped by path filters; every PR runs every blocking
  check.
- **FR-014**: Every check MUST have a stable, descriptive name that says what it proves (e.g.
  "no_std build", "feature: ax alone"), so the architect can mark it required and a reader
  understands a failure from the PR page.
- **FR-015**: This module MUST change only `.github/workflows/**` (never `ov-*.yml`) and
  `Cargo.toml`. In `Cargo.toml` it MAY change package metadata only; it MUST NOT add, remove or
  rename features or dependencies.

### Key Entities

- **Check**: one named pass/fail result shown on a PR, proving one promise (e.g. "no_std
  build"). Blocking or advisory.
- **Feature set**: one combination of Cargo features CI builds: none, one feature alone, the
  defaults, or all.
- **Target**: where a build is checked: Linux, Windows or macOS host (`std`), or the
  `thumbv7em-none-eabihf` microcontroller (`no_std`).
- **Run summary**: the page linked from each check, where the coverage figure is published.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For each promise in User Stories 1, 2, 4 and 5, a seeded fault on a throwaway
  branch turns the matching check red: 100 % of seeded faults caught, and the check name points
  to the promise broken.
- **SC-002**: On every PR, a reviewer reaches the coverage figure in at most 2 clicks from the
  PR page, without opening a log.
- **SC-003**: A full CI run on a PR, with a warm cache, finishes in at most 15 minutes wall
  clock.
- **SC-004**: Zero checks need a secret or a write permission; a fork PR shows the same set of
  checks as a branch PR.
- **SC-005**: The number of feature sets built equals the number of features declared in
  `Cargo.toml` plus 3 (none, defaults, all), with no manual list to keep in sync.
- **SC-006**: Over the first 20 PRs after this lands, at most one blocking check fails for a
  reason unrelated to the PR's change (flaky test, toolchain drift).

## Assumptions

- What exists today (`.github/workflows/ci.yml`), so the plan covers only gaps: job `check`
  (format, lint with and without `mock`, workspace tests, `mock` doc examples; Linux, latest
  stable, `-D warnings` for all jobs); job `no-std` (lint only, families + `mock`, on
  `thumbv7em-none-eabihf`); job `coverage` (line coverage ≥ 70 %, generated models excluded,
  gate only); job `codegen` (regenerate and fail on any diff, new files included; `xtask`
  removes stale outputs itself). Triggers: every PR, pushes to `main`.
- `ov-aiignore.yml` (agent commits vs `.aiignore`) exists, is read-only for agents, and is not
  changed or duplicated by this feature.
- The constitution's gate list (Development Workflow) is already fully covered by `ci.yml`; this
  feature adds gates on top. Adding them to the constitution's list is the architect's call
  (`/speckit-constitution`), not part of this module.
- Probe on 2026-10-27 (local, stable 1.97.1): the library already builds for the target with no
  features, with `alloc` alone, with each family alone and with `mock` alone; docs build with no
  warning; the missing-docs check fails only on the two `UnsupportedModel` fields (see Edge
  Cases). So FR-002, FR-004 and the docs half of FR-007 are expected green on day one.
- "Every PR reports coverage" (NFR-6) means the whole-workspace figure, not coverage of the
  changed lines only.
- The feature matrix builds each feature alone, not every combination: linear cost, and it
  catches cross-family leaks. Not-yet-declared features (`partial-reads`, `raw-registers`) join
  automatically when `Cargo.toml` declares them.
- The compile-fail check covers today's single example (`src/transport.rs`, `E0277`) and any the
  `bus` module adds. Default: a dated nightly toolchain for doc examples only, blocking; the
  plan may choose another tool that checks error codes on stable.
- The repository is public, so hosted Windows and macOS runners add no cost.
- Marking checks as required on `main` lives in the ruleset (`.github/ov-ruleset-main.json`,
  read-only for agents); the architect does it after merge.
- Updating `.github/pull_request_template.md` and `CLAUDE.md` with the new gates is outside this
  module's paths; it is a follow-up for the architect.
- Optional, not required by the SRS or constitution (recorded, not built here):
  - Dependency audit and licence check (security advisories, licence allow-list).
  - Manual hardware-in-the-loop job on a self-hosted runner with a U2D2 and an XM430-W350,
    started by hand only and never on fork PRs; it would serve the NFR-5 reference bench once
    the architect decides it.
  - API-compatibility (semver) check: breaking changes are allowed before 1.0.
  - Scheduled weekly run to catch toolchain and dependency drift between PRs.
  - Undefined-behaviour interpreter run (Miri) on the protocol layer.
  - Automated dependency-update PRs.
