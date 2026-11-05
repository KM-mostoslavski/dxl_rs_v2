---
name: contract
description: Step 4 of the cycle. From one module's section of docs/srs/SRS.md, draft the module's pub stubs and test names with todo!() bodies, then move the row to step 5. No sign-off wait. Use on /contract <module>.
---

# contract

Input: a module name. Its SRS is the `## Module: <module>`
section of `docs/srs/SRS.md` and the requirements its `Covers:` line
names. Ask for the name if not given.

1. Read the SRS section, the module's `//!` doc comment in the skeleton,
   its C4 box in `docs/c4/<module>.dsl`, and `docs/preferences.md`. If
   the first three are not all there, stop and name the missing one.
2. Check the dependency rule in `workflows/cycle/ROADMAP.md`: every
   dependency at step 4 or later. If not, stop and name the blocker.
3. Draft, in the module file:
   - every `pub` item the Interfaces subsection implies: full signature,
     `///` doc comment naming the requirement ids it serves, body `todo!()`
   - hot structs from the Data subsection: `#[repr(C)]`, typed fields,
     and a `size_of` test pinning the size
   - `#[cfg(test)] mod tests` with one `#[test] fn <id>_<slug>()`
     per SRS id the section covers (`fr_4_5_caller_buffer`), body `todo!()`
   On an existing module (adoption or change), start from its current
   pub surface and keep what the SRS still implies.
4. No private items, no implementation, no new dependencies. Anything
   the SRS leaves open: `unimplemented!("SRS <id>: <question>")` in
   place (SRS 5.0) and a row in `docs/deferred.md`.
5. Run `cargo build`: must be green. `cargo test` must compile. Every
   requirement id of the section has a test named for it.
6. Print the pub surface, one line per item. Set the module's ROADMAP
   row to step 5 and say so. Do not wait for sign-off; the owner vetoes
   with `/issue`, which sends the module back here.

Rules:

- If the SRS section is thin, invoke `superpowers:brainstorming` to fill
  it before drafting. Never draft stubs from guesses.
- Never touch another module's file.
