## What
<!-- One or two sentences. -->

## Why
<!-- Motivation, or "Closes #<issue>". -->

## How
<!-- Only if the approach is non-obvious: design choices, alternatives rejected. -->

## Architecture
<!-- "Follows docs/ARCHITECTURE.md", or list every deviation / issue noticed. -->

## Checklist
- [ ] `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass
- [ ] `cargo test` passes; line coverage ≥ 70 % (`cargo llvm-cov`)
- [ ] `no_std` build passes (`--no-default-features --target thumbv7em-none-eabihf`)
- [ ] Public methods documented
- [ ] Docs updated if behaviour or setup changed
