# Plan: bus

Open tasks copied from `specs/003-bus/tasks.md` on 2026-11-05, when the
project left spec-kit. Ids are the spec-kit ones, so the cross-references
(`Blocked on Tnnn`, other modules' ids, PR descriptions) still resolve.
Context for each task (plan, research, contracts, quickstart) stays in
[`specs/003-bus/`](../../specs/003-bus/).

The task texts are copied verbatim. Where they name spec-kit tools that are
gone (`/speckit-ov-srs`, `/speckit-constitution`, `/speckit-ov-pr`, `.aiignore`,
`docs/srs/modules/`, the constitution), read: edit `docs/srs/SRS.md` directly;
the constitution's principles are in `AGENTS.md`.

## Phase 14: Polish & Cross-Cutting Concerns

- [ ] T087 [human] Architect: patch `docs/srs/SRS.md` (human-only path) to this module's decisions and deviations (plan.md "Deviations from the SRS"): decided 2026-10-28: FR-3.7 and FR-2.6 (limits follow the operating mode), FR-4.7 (one result per actuator inside a P2 sync group), FR-8.1, FR-8.2 and FR-6.2 (raw replies run the fault check; raw writes do not update mode or torque state); earlier: FR-1.1 (no `Clock` bound on the struct), FR-1.3 (`DynBus` its own type), FR-1.4 (scan wait), FR-1.5 (`ActuatorRef<'_, P, T, C>`), FR-2.4 (`SetupError::UnsupportedModel` with `id`, `protocol`), FR-2.7 (fault at Setup fails Setup), FR-4.2 (torque Easy tier as unicast), FR-4.5 (plain-value buffer), FR-4.8 (validation all-or-nothing, wire not), FR-6.4 (new variants), FR-6.5 (P1 ping, enable refused while faulted), 2.1 (setup category), 3.3, 4.1, §7, §8 (`Bus<P, T, C, N>`, `new(transport, clock, config)`, `torque_state()`, clock as third parameter); then regenerate `docs/srs/modules/*.md` with `/speckit-ov-srs`
- [ ] T088 [human] Architect: update `docs/ARCHITECTURE.md` (architect-owned) to `Bus<P, T, C = DefaultClock, const N: usize = 32>`, torque state in a bus-owned byte array, the limit mode in each actuator's cache (module-interfaces §2.4b), autoscan in no_std, `DynBus` (module-interfaces §8 conflict 20, §9)
- [ ] T090 Run the manual hardware check of specs/003-bus/quickstart.md §4 (XM430-W350 on P2 in modes 3, 4 and 1, AX-12A on P1, U2D2) if hardware is available; otherwise leave it unchecked in the PR descriptions
