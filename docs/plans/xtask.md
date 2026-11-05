# Plan: xtask

Open tasks copied from `specs/006-xtask-codegen/tasks.md` on 2026-11-05, when the
project left spec-kit. Ids are the spec-kit ones, so the cross-references
(`Blocked on Tnnn`, other modules' ids, PR descriptions) still resolve.
Context for each task (plan, research, contracts, quickstart) stays in
[`specs/006-xtask-codegen/`](../../specs/006-xtask-codegen/).

The task texts are copied verbatim. Where they name spec-kit tools that are
gone (`/speckit-ov-srs`, `/speckit-constitution`, `/speckit-ov-pr`, `.aiignore`,
`docs/srs/modules/`, the constitution), read: edit `docs/srs/SRS.md` directly;
the constitution's principles are in `AGENTS.md`.

## Phase 8: Polish & Cross-Cutting Concerns (PR X1)

- [ ] T061 [human] Architect: confirm contract §8 conflict 19 (`UnsupportedModel` moves to `SetupError` and gains `id` and `protocol`) and the per-protocol lookup, then update `docs/srs/SRS.md`: FR-2.3/FR-2.4 (FR-2.4 still reads `Error::UnsupportedModel { model_number, feature }`), the items the 2026-10-28 mode decisions touch (FR-2.6 mode read at Setup, FR-3.7 limits follow the operating mode, the mode values in each table's `operating_modes`), FR-7.1/FR-7.4 (per-protocol map, `not_mapped`) and the xtask module "Covers" line. `docs/srs/SRS.md` is in `.aiignore`: agents must not edit it. Not blocking X1's merge
- [ ] T062 [human] Architect: approve deviation D1 (generated `reg()` helper) and clarifications C1–C5 in the PR's Architecture section (plan "Deviations from module-interfaces.md"), as constitution Governance requires for any deviation. Nothing to add to the contract for the mode source: §2.4b and §4.3 already carry check 17, `operating_modes` included
