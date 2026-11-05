# AI Agent Instructions

These are the common instructions for any AI agent or assistant working in this
repository, including **Claude** (Claude Code, claude.ai), **ChatGPT / Codex**,
**Cursor** and **Copilot**. Read them before making any change.

This file is the single source of truth for agent rules. `CLAUDE.md` is a
symlink to it: do not duplicate rules, extend them here.

## Spec-driven development

`dxl_rs_v2` is built spec-first with the cycle of
[ov-agentic-workflow](https://github.com/Ovsiankina/ov-agentic-workflow): the
SRS says what, the cycle says who builds it and how, agents write most of the
code, the maintainer decides and reviews.

Read `workflows/cycle/ROADMAP.md` first: which module is at which step, its
owner and its mode. Then `workflows/cycle/PROCEDURE.md` for the step you are
on, and `workflows/cycle/QA.md` before any PR. A module's requirements are its
`## Module:` section in [`docs/srs/SRS.md`](docs/srs/SRS.md) and the ids its
`Covers:` line names; its C3 box is `docs/c4/<module>.dsl`; its tasks are in
`docs/plans/<module>.md`. The modules are `core`, `protocol`, `transport`,
`bus`, `tables`, `xtask` and `ci`. Each `Covers:` line also names the module's
no-go files: do not edit them from that module's branch.

The **oracle** is the set of `[oracle]` checks in `QA.md` plus every command of
`.github/workflows/ci.yml` that runs locally (see [Commands](#commands)).
"Green" means all of them pass.

`specs/` is the frozen spec-kit record: read it for the research, designs and
contracts a plan task points to, never write to it.

## Agent decision policy

**Allowed, never ask:** any internal choice that keeps the oracle green.
Algorithm, private helper, error text, local naming, test data, iteration
order, private struct. Mark each one where it happens:

    // DECISION: <one line: what was chosen and why>

This is the one exception to the rule against comments inside method bodies
(see [Principles](#principles), VII): the reviewer reads the what and the why
in the code.

Bookkeeping is never a question: status rows, plan ticks, doc moves, Cargo
scaffold, copying templates. Do it and mention it in one line.

**Decisions you don't take alone:**

- anything `pub`: signature, visibility, new item, re-export, trait impl on a
  pub type, a new or changed Cargo feature. A `pub` item that a plan task or
  `specs/<feature>/contracts/` spells out is already decided: build it as
  written.
- data layout: `#[repr]`, field order, field types, enum discriminants, the
  generated model layout, anything a `size_of` test pins
- dependencies: adding, removing, bumping, feature flags of a dependency
- `unsafe`, anywhere
- the safety behaviour of the bus: torque state, the `ActuatorIsOff` guard,
  limit checks, fault handling, anything that writes EEPROM
- a control-table value that no e-Manual page confirms (Principle IV)
- any module whose ROADMAP row says owner `mine`

When a task hits one of these:

1. Write it to `docs/deferred.md`: module, step, what was needed, why it is
   not yours to decide, the smallest change that would unblock.
2. Mark the blocked test `#[ignore = "deferred"]`.
3. Continue with the rest of the task.

After 3 open deferrals on one module, stop that module, set its ROADMAP step
to `needs contract rework`, and move to the next module.

**Modules owned `mine`:** do not edit their files unless the task cannot be
done otherwise. If you do, list those files first in your summary, with one
line on why.

**Human-only paths:** `docs/srs/SRS.md` and `docs/ARCHITECTURE.md` are written
by the architect. A conflict between them, or between them and the code, is a
deferral, never resolved by a guess.

## Principles

These come from the spec-kit constitution (v1.4.0, 2026-10-28), which this
file replaces. Authority order: these principles, then `docs/srs/SRS.md`, then
`docs/ARCHITECTURE.md`. The architect amends them by editing this section.

### I. no_std and heap-free (non-negotiable)

- The crate builds with `default-features = false` on `thumbv7em-none-eabihf`;
  CI checks it.
- No code path reachable without the `std` and `alloc` features may allocate.
- No build, `std` or not, allocates per Cycle: multi-actuator reads write into a
  caller-owned `&mut` buffer, and internals use `heapless`.
- `Bus` holds no `Rc`, `RefCell` or stored lifetime; single-actuator access goes
  through the short-lived `ActuatorRef` that borrows `&mut Bus`.

One code base serves desktop and microcontroller users; the embedded case is
the binding constraint.

### II. Absent over failing

- An operation a protocol does not support is absent from that protocol's
  `Bus` impl, so using it is a compile error (`sync_read` exists only on
  `impl Bus<P2, T>`).
- Errors surface at the earliest moment: compile time over Init, Init over
  Cycle. A model compiled out by a feature fails Setup with
  `SetupError::UnsupportedModel { id, model_number, protocol, feature }`.
- Protocol `P`, transport `T`, clock `C` and capacity `N` are fixed at compile
  time; there is no runtime protocol switching.

A missing method is cheaper to debug than a runtime error on a moving robot.

### III. Nothing hidden

- Torque is off at Init and only `enable_torque()` turns it on. A goal write
  while off returns `ActuatorIsOff` and sends nothing; no model-specific side
  effect (no AX-style auto-enable).
- Limits and the operating mode are read from each actuator at Init; a goal is
  checked against the limits only in a mode where the firmware applies them.
  Never fall back to control-table defaults or a default mode; a failed read
  fails Init.
- Writes are never confirmed implicitly; read-back is a separate, explicit
  call.
- Every status packet's error byte is checked, `raw-registers` replies
  included; a fault sets fault state and torque state off and returns a typed
  hardware error.
- No implicit allocation, retry beyond the configured count, or fallback of any
  kind.

A HAL that drives motors must never surprise the caller about what reached the
wire.

### IV. The e-Manual is the only source (non-negotiable)

- Every control-table entry and capability claim (address, size, access, range,
  unit, default, instructions, protocol, firmware-dependent layout, model
  number) comes from the official ROBOTIS e-Manual (`emanual.robotis.com`).
- Memory, third-party SDKs and other models' tables are not a source.
- A model is verified online when it is first needed. A verified `.ron` table
  records the e-Manual URL and verification date; every other table is flagged
  unverified, and stays usable.
- A model that supports both protocols has tables for both; Protocol 2 is the
  default.
- A PR that verifies or changes a control table cites the e-Manual URL and the
  verification date.

A wrong address or size silently corrupts actuator state or EEPROM.

### V. Committed, data-driven codegen

- Model support is extended by adding `.ron` tables, not by hand-writing model
  code.
- `cargo xtask codegen` turns `control_tables/` into plain `.rs` files that are
  committed (`src/models.rs`, `src/models/`). No `build.rs`, no proc macros.
- Generated files are never edited by hand; CI fails when regeneration produces
  a diff.
- Generated code is formatted and readable, so table changes review as source
  diffs.

### VI. Typed, static, panic-free API

- Values cross the API as newtypes (`Id` = `0..=252`, `Position`, other SI
  types) with units and ranges; a raw-register form of each stays reachable.
- Dispatch is static: no `dyn`, no vtables; `ActuatorKind` recovers the model
  by `match`.
- No panic on the happy path or on any I/O or hardware error; those are
  `Result`s with typed `thiserror` (no_std) errors split into transport,
  protocol, status, validation and setup.
- The only allowed panic is the SRS § 5 marker
  `unimplemented!("SRS <id>: <question>")` for a point the spec leaves open;
  where a wrong guess could corrupt actuator state or EEPROM, use
  `compile_error!` instead.

### VII. Developer experience first

- Priority order: developer experience, then performance. Feature velocity is
  not a priority. Extensibility comes from the `.ron` tables.
- Cargo features add API; they do not change an existing method's behaviour.
  Before 1.0 breaking changes are allowed, but each one is called out in its
  PR.
- Every public item is documented. No comments inside method bodies unless they
  resolve a real ambiguity (or record a `// DECISION:`).

### Technical constraints

- Rust edition 2024; workspace of the root crate plus `xtask/`.
- Module layout is Rust 2018+: `foo.rs` + `foo/`, never `mod.rs`
  (`clippy::mod_module_files` is deny).
- Transport is any blocking `embedded_io::{Read, Write, ReadReady}`; the bus
  enforces timeouts by polling `ReadReady` against a `Clock`. No async I/O.
- Status Return Level 2 is assumed; it is never read, tracked or changed.
- One bus = one wire, one protocol, one baud rate.
- The protocol layer is tested against the e-Manual packet examples with a mock
  transport, no hardware required.
- Line coverage stays at or above 70 % (`cargo llvm-cov`, generated models
  excluded; they are tested through `xtask`).

## Git & commit rules

**a) Never push to `main`.** `main` only moves through a reviewed and merged PR.
   Work happens on `dev` and on branches cut from it.

**b) Commit as the maintainer, never credit an AI.** Commits are made under the
   maintainer's own git identity. No `Co-Authored-By:`, `Assisted-by:` or
   "Generated with" line naming an AI, in commits or in PRs.

**c) An agent never approves or merges** a pull request. The maintainer does
   both.

## Commits, branches & pull requests

- **Commits** follow [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/).
  A commit that does a plan task puts its id first in the description, so the
  step end can tick it: `fix(bus): T090 run the XM430 hardware check`.
- **Branches** follow [Conventional Branch](https://conventional-branch.github.io/):
  one `feature/<module>` per module in the cycle (`feature/bus`), `fix/…` for a
  change.
- **Merge** (step 7): a PR from the module branch into `dev`, squash-merged by
  the maintainer. `dev` reaches `main` through its own PR.
- **Explain the change** in the commit body, the PR description, or both.
- **Pull requests** use [`.github/pull_request_template.md`](.github/pull_request_template.md).

## General guidance

- Keep changes scoped to the task. Ask before anything destructive: history
  rewrites, force pushes, deleting branches or data.
- Ambiguity inside the oracle is a `DECISION`; ambiguity outside it is a
  deferral. Never guess silently.
- Match the surrounding code: its naming, idiom and comment density.

---

# Project context

## What this is

`dxl_rs_v2` is the rewrite of a Rust HAL for Dynamixel actuators (Protocol 1
and 2). The repo holds:

- `docs/srs/SRS.md`: the requirements, one `## Module:` section per module.
- `docs/ARCHITECTURE.md`: the design decisions the crate must follow.
- `docs/workspace.dsl`: the Structurizr C4 model (C1/C2), with one C3 file per
  module in `docs/c4/`. `docs/workspace.json` is its layout; never add
  `autolayout` to a view. `docs/.structurizr/` is a git-ignored cache.
- `src/`: the crate. The old std prototype was retired; see git history before
  `refactor!: retire std prototype`.
- `control_tables/*.ron`: per-model Dynamixel control tables, the source data
  for codegen.

## Commands

```sh
cargo build
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --workspace --all-targets --features mock -- -D warnings
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo test --features mock --doc
cargo test --workspace --all-features
cargo build --lib --no-default-features --target thumbv7em-none-eabihf                     # no_std (CI)
cargo build --lib --no-default-features --features alloc --target thumbv7em-none-eabihf    # no_std + alloc (CI)
cargo clippy --no-default-features --features ax,mx,rx,x-series,y-series,pro,mock,partial-reads,raw-registers --target thumbv7em-none-eabihf   # heap-free (CI)
cargo llvm-cov --workspace --ignore-filename-regex 'src/models(\.rs|/)' --fail-under-lines 70   # coverage gate (CI)
cargo xtask codegen                        # regenerate src/models.rs + src/models/ from control_tables/ (commit the result)
python3 control_tables/check_tables.py     # table schema check (CI)
python3 control_tables/analyze_tables.py   # regenerates control_tables/control_table_analysis.md
```

Features: `std` (default) = `alloc` + `serialport`; model families `ax`, `mx`,
`rx`, `x-series`, `y-series`, `pro` (all default); opt-in `mock`,
`partial-reads`, `raw-registers`. CI derives the heap-free and no_std feature
sets from `Cargo.toml` (`feature-sets` job).

## Target architecture (summary; read docs/ARCHITECTURE.md before designing anything)

Layers: **API** (`Bus<P, T, C, const N: usize = 32>` owning a
`heapless::Vec<ActuatorKind, N>`, short-lived `bus.actuator(id)` handles) →
**Models** (generated, one struct per actuator model) → **Protocol** (`P1 | P2`,
compile-time generic on `Bus`) → **Transport** (`T`, generic, mockable).

## Control tables

- One `.ron` per model with `type_info`, `unit_info` and `control_table`
  (`address`, `size`, `data_name`). `dynamixel.ron` is the model-number → file
  map, not a control table.
- `control_table_analysis.md` groups the models into exact layouts and RAM
  layout tiers (keyed by `Goal Position` address): same-named registers sit at
  different addresses across tiers, so addresses are per-model constants.
- ARCHITECTURE.md lags the 2026-10-27/28 decisions in places (`Bus<P, T, N>`,
  "Not implemented yet"); plan tasks bus T088, core T059 and tables T056 bring
  it up to date. Until then the SRS wins, and a gap you notice is reported.
