# Specification Quality Checklist: Control tables (verification flags, capabilities, per-protocol model map)

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-10-27
**Updated**: 2026-10-27 (clarification session); 2026-10-28 (mode source, stale text)
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

- Clarification session 2026-10-27 resolved 2 of 3 markers: User Story 5 (MX Protocol 1 tables
  are written in this feature; every dual-protocol model gets tables for both protocols,
  Protocol 2 by default) and FR-013 (no bulk verification; unverified tables stay mapped and
  flagged, Y-series and PRO included; AX-12 and MX-64 verified first).
- The last marker, FR-007, was resolved in the same session: access and area on every row (a).
- Rewritten: User Stories 1, 2, 4, 5; FR-001, FR-003, FR-004, FR-006, FR-008, FR-011, FR-013,
  FR-014, FR-016, FR-017; new FR-018, FR-019; SC-001 to SC-005, new SC-007; edge cases,
  assumptions and effort.
- The product is a data set for a library, so file names (`dynamixel.ron`, `.ron`), the
  `source: (url, verified)` record shape and `emanual.robotis.com` are user-facing constraints
  from the SRS and constitution IV, not implementation choices. Same convention as specs 001
  and 002.
- SRS FR-7.2, NFR-9, delivery step 7 and the roadmap carry the 2026-10-27 amendment, and so
  does constitution IV (v1.3.0). Still to flag: delivery step 3 ("one X-series model verified")
  is not covered unless that model is needed; the SRS roadmap line "AX-12+, AX-18F, XW430-T200
  … Adding the number is enough" is stale (their numbers are shared, so they stay not mapped).
- 2026-10-28: architect decision "limits follow the operating mode" (module-interfaces §2.4b).
  Added US3 AS5–AS6, US5 AS6, FR-020, SC-008, the Mode source entity, an edge case and an
  assumption; FR-014 fails on a missing mode source. Not yet in SRS FR-3.7.
- 2026-10-28 cleanup: 60 (not 62) tables have Protocol Type; 63 model numbers and 4,635 register
  rows (plus 269 `unit_info` entries); US2 AS4 and US5 AS3–AS4 match the plan (shared-number
  files stay not mapped, skipped without a warning); FR-019 separates MX(2.0) (dual) from MX on
  Protocol 1 (own numbers, Protocol 1 only).
- Cross-module dependency: reading the per-protocol map, the default protocol, the capability
  data and the verification flag (for generated docs) in codegen is the `xtask` module's job;
  CI wiring of the table check is the `ci` module's job. A correction or new table regenerates
  `src/models*` (no-go `src/**`): flagged as a deviation to state in the PR.
- Effort is rescoped in the spec's Assumptions: the old 15–30 h bulk-verification estimate is
  dropped; the 4 priority tables take about 1.5–3 h, other models are paid for one at a time.
