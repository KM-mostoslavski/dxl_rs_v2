# QA checklist

`[oracle]` items are mechanical: run the command, pass or fail. They are
the merge gate; the agent runs them at step 6 and opens the PR at
step 7 when all pass; I merge. `[reviewer]` items need judgment; the reviewer agent
applies them during the walkthrough (step 10), function by function.
The profile tag (`[serious]`, `[poc, serious]`) says which ROADMAP
profiles run the item; `toy` runs none.

The step-6 report fits one screen: every `[oracle]` item with pass/fail,
every `// DECISION:`, every deferral, the pub diff. A longer report
fails this checklist.

## Oracle

- [ ] `[oracle]` `[poc, serious]` Tests green. `cargo test --workspace`,
      zero failures. Tests marked `#[ignore = "deferred"]` are listed in
      the report, not hidden.
- [ ] `[oracle]` `[serious]` Clippy clean.
      `cargo clippy --workspace --all-targets -- -D warnings`, zero
      warnings, also with `--features mock` and `--all-features`. This
      also catches leftover `todo!()` and `dbg!` (`Cargo.toml`
      `[workspace.lints]`). `unimplemented!("SRS <id>: …")` is the SRS
      5.0 marker and stays allowed; each one is listed in the report.
- [ ] `[oracle]` `[poc, serious]` Pub diff empty against the step-4
      contract, unless the task text says otherwise. Compare `pub`
      items, signatures, visibility, re-exports, trait impls on pub
      types.
- [ ] `[oracle]` `[poc, serious]` Requirement coverage. Every
      SRS id the module's `Covers:` line names has a test that names it
      (in its name or its doc comment). `grep` the ids against the
      tests; a missing one fails.
- [ ] `[oracle]` `[poc, serious]` The CI commands that run locally
      pass: `cargo fmt --check`, the doc tests with `--features mock`,
      the thumbv7em no_std builds, `cargo xtask codegen` without a diff,
      `python3 control_tables/check_tables.py`.
- [ ] `[oracle]` `[serious]` No `mod.rs` anywhere. `clippy::mod_module_files`
      is deny; a module is `foo.rs` with children in `foo/`.
- [ ] `[oracle]` `[poc, serious]` No new dependency, no `unsafe`, no
      `#[repr]` or field-order change, unless the task text asked for
      it or `docs/deferred.md` records it. `cargo tree` diff and `grep`.
- [ ] `[oracle]` `[poc, serious]` No file of a module whose ROADMAP
      owner is `mine` appears in the branch diff.
- [ ] `[oracle]` `[poc, serious]` A C4 box for the module exists in
      `docs/c4/<module>.dsl`.
- [ ] `[oracle]` `[poc, serious]` `docs/plans/<module>.md` exists and
      every task is ticked or in `docs/deferred.md`.
- [ ] `[oracle]` `[poc, serious]` Every `// DECISION:` added on the
      branch and every deferral raised on it is in the report.

## Reviewer (walkthrough, step 10)

- [ ] `[reviewer]` `[poc, serious]` The function does what its SRS
      requirement says, no more, no less. Nothing in it contradicts a
      requirement.
- [ ] `[reviewer]` `[poc, serious]` Every pub item on the path is
      implied by a requirement.
- [ ] `[reviewer]` `[poc, serious]` The dependencies drawn in
      `docs/c4/<module>.dsl` match the module's `use` lines.
- [ ] `[reviewer]` `[serious]` Hot structs are `#[repr(C)]` and each
      has a `size_of` test pinning its size. Hot = named as hot path in
      the SRS or HLD, or crossing an FFI or serialization boundary.
- [ ] `[reviewer]` `[poc, serious]` Every `avoid` line of
      `docs/preferences.md` is respected and every `like` line is
      followed where it applies.
- [ ] `[reviewer]` `[poc, serious]` Each `// DECISION:` on the path is
      one the owner would have taken. If not, it is an issue.
