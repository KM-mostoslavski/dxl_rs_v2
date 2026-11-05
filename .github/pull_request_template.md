## What
<!-- One or two sentences. -->

## Why
<!-- Motivation, or "Closes #<issue>". -->

## How
<!-- Only if the approach is non-obvious: design choices, alternatives rejected. -->

## Architecture
<!-- "Follows docs/ARCHITECTURE.md", or list every deviation / issue noticed. -->

## Cycle
<!-- Module and step (workflows/cycle/ROADMAP.md), plan task ids, and the SRS ids covered. -->
- [ ] Every `// DECISION:` added here is listed below
- [ ] Every deferral raised here is in `docs/deferred.md`

## Checklist
- [ ] `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass, also with `--features mock` and `--all-features`
- [ ] `cargo test --workspace` passes, also with `--all-features`; line coverage ≥ 70 % (`cargo llvm-cov`)
- [ ] `no_std` builds pass on `thumbv7em-none-eabihf`: no features, `alloc`, and the heap-free set (see `AGENTS.md` § Commands)
- [ ] `cargo xtask codegen` leaves `src/models.rs` and `src/models/` unchanged
- [ ] Public items documented
- [ ] A changed control table cites its e-Manual URL and verification date
- [ ] Docs updated if behaviour or setup changed
- [ ] The maintainer can explain every line of this diff
