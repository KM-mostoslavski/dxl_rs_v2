# Plan: core

Open tasks copied from `specs/004-core/tasks.md` on 2026-11-05, when the
project left spec-kit. Ids are the spec-kit ones, so the cross-references
(`Blocked on Tnnn`, other modules' ids, PR descriptions) still resolve.
Context for each task (plan, research, contracts, quickstart) stays in
[`specs/004-core/`](../../specs/004-core/).

The task texts are copied verbatim. Where they name spec-kit tools that are
gone (`/speckit-ov-srs`, `/speckit-constitution`, `/speckit-ov-pr`, `.aiignore`,
`docs/srs/modules/`, the constitution), read: edit `docs/srs/SRS.md` directly;
the constitution's principles are in `AGENTS.md`.

## Phase 7: Polish & PR core-1

- [ ] T044 [human] Architect approvals needed before core-1 merges (plan Complexity Tracking, Deviations, module-interfaces §9): (a) core in two PRs against "one module per PR" (approved 2026-10-28); (b) open risk research R-12 (`cache_mut()` callable through `A: Actuator` once core-2 lands; a token argument would change X1's emitted impl). No longer open, decided 2026-10-28: conflict 19 (`id` and `protocol` on `UnsupportedModel`); deviation D3 (the value lists live in the tables, `ModeSource::Register { register, values: ModeValues }`, nothing left for X1 to pick); `position_limits*()` return the EEPROM limits in every mode (research R-17); core-1 deletes `P2::broadcast_ping` and PR #7's `expect(dead_code)` (research R-19)

## Phase 10: Polish & PR core-2

- [ ] T059 [human] Architect follow-ups, not blocking either PR: amend constitution Principle II (`UnsupportedModel { model_number, feature }` → the `SetupError::UnsupportedModel { id, model_number, protocol, feature }` shape; conflict 19 accepted 2026-10-28) through `/speckit-constitution`; update docs/ARCHITECTURE.md to the 2026-10-27 and 2026-10-28 decisions (cache = `Id` + limit mode + limits, limits follow the operating mode, one-byte `TorqueState` in the bus, five error categories, a fault on a `raw-registers` reply sets `Faulted`; module-interfaces conflict 20); update docs/srs/SRS.md (FR-2.4, FR-2.6, FR-3.2, FR-3.7, FR-6.4, §2.1 and the §3.3 rows `Actuator` (trait), Cache, Limits, Register) to the same decisions (research R-17, R-18)
