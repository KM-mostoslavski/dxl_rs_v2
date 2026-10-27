<!-- ov-policy -->
<!--
Sync Impact Report
- Version change: 1.0.0 → 1.1.0
- Modified principles: none
- Modified sections: Technical Constraints (transport contract is `Read + Write + ReadReady`
  with bus-held `Clock`; opt-in `mock` feature)
- Added sections: none
- Removed sections: none
- Templates: none need updates
- Deferred TODOs: none
-->

# dxl_rs_v2 Constitution

## Core Principles

### I. no_std and Heap-Free (NON-NEGOTIABLE)

- The crate MUST build with `default-features = false` on `thumbv7em-none-eabihf`; CI checks it.
- No code path reachable without the `std` feature may allocate.
- No build, `std` or not, may allocate per Cycle: multi-actuator reads write into a
  caller-owned `&mut` buffer, and internals use `heapless`.
- `Bus` holds no `Rc`, `RefCell` or stored lifetime; single-actuator access goes through the
  short-lived `ActuatorRef` that borrows `&mut Bus`.

Rationale: one code base serves desktop and microcontroller users; the embedded case is the
binding constraint.

### II. Absent Over Failing

- An operation a protocol does not support MUST be absent from that protocol's `Bus` impl, so
  using it is a compile error (e.g. `sync_read` exists only on `impl Bus<P2, T>`).
- Errors surface at the earliest moment: compile time over Init, Init over Cycle. A model
  compiled out by a feature fails Setup with `UnsupportedModel { model_number, feature }`.
- Protocol `P`, transport `T` and capacity `N` are fixed at compile time; there is no runtime
  protocol switching.

Rationale: a missing method is cheaper to debug than a runtime error on a moving robot.

### III. Nothing Hidden

- Torque is off at Init and only `enable_torque()` turns it on. A goal write while off returns
  `ActuatorIsOff` and sends nothing; no model-specific side effect (no AX-style auto-enable).
- Limits are read from each actuator's EEPROM at Init. The crate MUST NOT fall back to
  control-table defaults; a failed limit read fails Init.
- Writes are never confirmed implicitly; read-back is a separate, explicit call.
- Every status packet's error byte is checked; a fault sets fault state and torque state off
  and returns a typed hardware error.
- No implicit allocation, retry beyond the configured count, or fallback of any kind.

Rationale: a HAL that drives motors must never surprise the caller about what reached the wire.

### IV. e-Manual Is the Only Source (NON-NEGOTIABLE)

- Every control-table entry and every capability claim (register address, size, access, range,
  unit, default, supported instructions, protocol, firmware-dependent layout, model number)
  MUST be verified against the official ROBOTIS e-Manual (`emanual.robotis.com`), looked up
  online when it is written or changed.
- Memory, third-party SDKs and other models' tables are not a source.
- Each `.ron` table records the e-Manual URL and verification date it was checked against.

Rationale: a wrong address or size silently corrupts actuator state or EEPROM.

### V. Committed, Data-Driven Codegen

- Model support is extended by adding `.ron` tables, not by hand-writing model code.
- `cargo xtask codegen` turns `control_tables/` into plain `.rs` files that are committed
  (`src/models.rs`, `src/models/`). No `build.rs`, no proc macros.
- Generated files MUST NOT be edited by hand; CI fails when regeneration produces a diff.
- Generated code is formatted and readable so table changes review as source diffs.

Rationale: zero build-time codegen, auditable output, and new models without new logic.

### VI. Typed, Static, Panic-Free API

- Values cross the API as newtypes (`Id` = `0..=252`, `Position`, other SI types) with units
  and ranges; a raw-register form of each stays reachable.
- Dispatch is static: no `dyn`, no vtables; `ActuatorKind` recovers the model by `match`.
- No panic on the happy path or on any I/O or hardware error; those are `Result`s with typed
  `thiserror` (no_std) errors split into transport, protocol, status and validation.
- The only allowed panic is the SRS 5.0 marker `unimplemented!("SRS <id>: <question>")` for a
  point the spec leaves open; where a wrong guess could corrupt actuator state or EEPROM,
  `compile_error!` is used instead.

Rationale: misuse is caught by the type checker, and failures are values the caller handles.

### VII. Developer Experience First

- Priority order: developer experience, then performance. Feature velocity is not a priority.
- Cargo features add API; they do not change an existing method's behaviour. During alpha
  (before 1.0) breaking changes are allowed, but each one is called out in its PR.
- Every public item is documented. No comments inside method bodies unless they resolve a
  real ambiguity.

Rationale: an awkward API is not worth shipping sooner.

## Technical Constraints

- Rust edition 2024; workspace of the root crate plus `xtask/`.
- Module layout is Rust 2018+: `foo.rs` + `foo/`, never `mod.rs` (`clippy::mod_module_files`).
- Transport is any blocking `embedded_io::{Read, Write, ReadReady}`; timeouts are enforced by
  the bus, which polls `ReadReady` against a `Clock`. Async I/O is out of scope.
- Features: `std` (default: `alloc` + `serialport`); model families `ax`, `mx`, `x-series`,
  `y-series`, `pro` (all default); opt-in `partial-reads`, `raw-registers` and `mock`.
- Assumed factory Status Return Level 2; Status Return Level is never read, tracked or changed.
- One bus = one wire, one protocol, one baud rate.

## Development Workflow and Quality Gates

- One SRS module per PR, branched from `main` and merged into `main` only, using
  `.github/pull_request_template.md`.
- Every PR MUST pass: `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`;
  `cargo test` for the workspace; the no_std clippy check on `thumbv7em-none-eabihf`;
  `cargo xtask codegen` leaving the generated files unchanged; at least 70 % line coverage
  (`cargo llvm-cov`, generated models excluded and tested through `xtask`).
- Commits and PRs carry no AI co-authorship or attribution (`Co-Authored-By`, `Assisted-by`,
  "Generated with"). Agent work is identified by the dedicated agent account that makes it.
- A PR touching `control_tables/` cites the e-Manual URL and verification date (Principle IV).
- The protocol layer is tested against the e-Manual packet examples with a mock transport, no
  hardware required.

## Governance

- Authority order: this constitution, then `docs/srs/SRS.md`, then `docs/ARCHITECTURE.md`.
  The SRS and ARCHITECTURE.md MUST agree; a conflict between them is a defect raised with the
  architect, never resolved by an agent's guess. Deviations from either are flagged explicitly
  in the PR's Architecture section.
- `docs/srs/SRS.md` is written by the architect. Module specs in `docs/srs/modules/` are
  generated by `/speckit-ov-srs` and never edited by hand.
- Amendments are made by the architect through `/speckit-constitution`, with the version bumped
  by semantic versioning: MAJOR for removing or redefining a principle, MINOR for a new
  principle or section or materially expanded guidance, PATCH for wording and clarifications.
- Every plan (`/speckit-plan` Constitution Check) and every PR review verifies compliance with
  these principles; any exception is justified in writing in the plan or PR.

**Version**: 1.1.0 | **Ratified**: 2026-10-26 | **Last Amended**: 2026-10-26
