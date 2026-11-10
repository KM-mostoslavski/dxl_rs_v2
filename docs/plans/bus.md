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

## D-01: Setup holds hardware-faulted actuators

Plan, decisions and tests in
[`2026-11-05-setup-holds-faults.md`](2026-11-05-setup-holds-faults.md).
Changes a merged contract: step 3 → 4 → 5.

- [ ] T091 [human] Architect: decide D1 (hold by default), D2 (`SetupReport { actuators, faulted: IdSet }`), D3 (hardware-only flags); amend FR-2.7, FR-6.2 and the § 8 "Typed EEPROM writes" sentence (supersedes T087's FR-2.7 item); triage D-01 as bus step 4
- [x] T092 Contract: `SetupReport<'a>`, `declare`/`autoscan` on `Bus` and `DynBus` return it, callers and examples updated, the four old fault tests renamed to `todo!()`, new test stubs
- [x] T093 Declared Setup holds an actuator whose replies carry only hardware fault flags as `Faulted`; mixed flags and P2 error codes still fail
- [x] T094 Autoscan holds a hardware-faulted pong the same way
- [x] T095 Recovery path (`write_raw` → `clear_fault` → `enable_torque`), whole-bus `enable_torque` refused, `DynBus` parity, rustdoc, D-01 done

## Typed limit setters

Plan: [`2026-11-09-typed-limits.md`](2026-11-09-typed-limits.md).

- [ ] T096 [human] Architect: SRS text for typed limit writes (plan D1 to D10)
- [x] T097 tables: `unit_info` for the D1 registers in the six verified tables
- [x] T098 xtask: limit roles and the `Pwm` quantity; regenerate models
- [x] T099 core: `units::Pwm`, `ValidationError::ActuatorIsOn`, the new roles
- [x] T100 bus: limit fields, `LimitField`, `write_limit`, setters and getters, checks, cache updates
- [x] T101 bus: docs and step end
- [x] T102 bus: D8 torque confirmed off, D9 no wheel switch with a live speed
- [x] T103 tables, xtask, core, bus: D10 typed Acceleration Limit
- [ ] T104 tables: unit data of the 70 tables without `unit_source`, from their e-Manual pages (`2026-11-10-settings.md` S4)
- [ ] T105 bus, core, xtask: `LimitField` → `SettingField`, `write_limit` → `write_setting` (S1)
- [ ] T106 tables: Homing Offset, Moving Threshold, Return Delay Time `unit_info` in the six verified tables
- [ ] T107 xtask, core, bus: typed Homing Offset, Moving Threshold, Return Delay Time (S2)
- [ ] T108 core, bus: `OperatingMode` and `set_operating_mode` (S3)
- [ ] T109 bus: docs, codegen over all tables, oracle, step end
- [ ] T110 SRS: FR-10 reconciled with D1 to D10 and S1 to S5
