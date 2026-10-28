# Specification Quality Checklist: xtask codegen (generated models, dispatch and family features)

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-10-27
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [x] Written for non-technical stakeholders
- [x] All mandatory sections completed

## Requirement Completeness

- [x] No [NEEDS CLARIFICATION] markers remain
- [x] Requirements are testable and unambiguous
- [x] Success criteria are measurable
- [x] Success criteria are technology-agnostic (no implementation details)
- [x] All acceptance scenarios are defined
- [x] Edge cases are identified
- [x] Scope is clearly bounded
- [x] Dependencies and assumptions identified

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria
- [x] User scenarios cover primary flows
- [x] Feature meets measurable outcomes defined in Success Criteria
- [x] No implementation details leak into specification

## Notes

- The product is a library and this module is its code generator, so `ActuatorKind`, model
  types, Cargo family features, `std` / `no_std` and the `thumbv7em-none-eabihf` target are
  domain terms from the SRS and constitution, not implementation choices. Same convention as
  specs 001 and 002.
- The 3 [NEEDS CLARIFICATION] markers (FR-010, FR-012, FR-015) were resolved in the
  2026-10-27 clarify session (spec Clarifications).
- 2026-10-28: spec aligned with `specs/ideas/module-interfaces.md` §8 (cache without torque
  state, per-protocol lookup with `SetupError`, access and MX on P1 in scope) and with the
  decision that limits follow the operating mode (FR-019).
- 2026-10-28: decision "the mode values live in the tables" (module-interfaces §2.4b, §3.1).
  FR-019 and FR-003 now carry `operating_modes` (`ModeSource::Register { register, values }` or
  `AngleLimits`); a clarification, an edge case and US1 AS6 values were added. The open question
  "how X1 picks the value list" is closed; plan C6 is dropped as moot.
- Scope is the gap on top of PR #4; the Assumptions section lists what already exists so the
  plan does not redo it.
- Cross-module dependencies (core types, tables protocol data, ci per-family builds) are
  recorded in Assumptions; the plan must order PRs accordingly.
