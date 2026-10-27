# Data Model: CI (hardened quality gates)

**Feature**: [spec.md](spec.md) | **Research**: [research.md](research.md)

CI has no Rust types. Its "data" is the set of checks, the feature sets they build, where they
build, and the figures they publish. Full catalogue: [contracts/ci-checks.md](contracts/ci-checks.md).

## Check

One named pass/fail result on a PR (one GitHub Actions job, or one matrix leg).

| Field | Type | Rules |
|---|---|---|
| `job_id` | YAML key | Stable. Used by `needs:`. |
| `name` | string | Says what it proves (FR-014). No toolchain version, no date. A matrix name includes its feature set and target, or its OS. |
| `kind` | `blocking` | Every check in this feature is blocking. No `continue-on-error`. |
| `runs_on` | `ubuntu-latest` \| `windows-latest` \| `macos-latest` | GitHub-hosted only (FR-011). |
| `toolchain` | `stable` \| `COMPILE_FAIL_TOOLCHAIN` | Only `compile-fail` is pinned (R-7, R-15). |
| `needs` | list of `job_id` | `no-std`, `feature-set`, `feature-test`, `compile-fail` → `feature-sets`; `features` → `feature-sets`, `feature-set`, `feature-test`. |
| `proves` | list of FR / SRS ids | At least one. |

States, per run: `queued → running → success | failure | cancelled | skipped`.

- `cancelled`: a newer push to the same PR started a run (FR-012).
- `skipped`: only a `feature-set` or `feature-test` leg, or `no-std` or `compile-fail`, whose
  `feature-sets` job failed. The `features` aggregate
  then reports `failure`, never `success`.

Invariants:

- No `paths` or `paths-ignore` filter: every PR runs every check (FR-013).
- No check needs a secret or a write permission (FR-011, SC-004).
- Removing or weakening a kept command is a spec violation (FR-001).

## Feature set

One combination of Cargo features passed to one build. Derived from `Cargo.toml`, never typed by
hand. Rule: [contracts/feature-sets.md](contracts/feature-sets.md).

| Field | Type | Rules |
|---|---|---|
| `name` | string | `no features`, `<feature> alone`, `defaults`, `all features` (build matrix); `std`, `<feature>` (test matrix); `heap-free, all`, `no_std, all` (no_std job) |
| `args` | cargo flags | e.g. `--no-default-features --features ax` |
| `needs_std` | bool | `std` in the transitive closure of the set's features |
| `needs_alloc` | bool | `alloc` in that closure |

Relationships:

- Host target: every set in the host list (count = declared features + 3, SC-005).
- Microcontroller target: only sets with `needs_std = false`.
- `heap-free, all` = every declared feature with `needs_std = false` and `needs_alloc = false`.
  This is the set FR-002 requires to stay heap-free.
- Host test sets (`cargo test --lib`): `std` alone, and `std` plus each `heap-free` feature.

## Target

Where a build runs.

| Target | Builds | `std` | Notes |
|---|---|---|---|
| Linux host (`x86_64-unknown-linux-gnu`) | all host sets, test sets, tests, coverage, docs, compile-fail (incl. a std-off doc pass), codegen, table check | yes | `libudev-dev` installed in a named step |
| Windows host | defaults: build + crate tests | yes | pty tests skip by their own `cfg(unix)` |
| macOS host | defaults: build + crate tests | yes | pty tests run |
| `thumbv7em-none-eabihf` | no_std sets, target sets | no | library only; firmware link is the FR-003 TODO |

## Toolchain pin

| Field | Value | Rules |
|---|---|---|
| `COMPILE_FAIL_TOOLCHAIN` | `"1.97.1"` | Workflow-level `env`. Changed only by a commit that edits it (FR-008). |
| `RUSTC_BOOTSTRAP` | `1` | Set only on the three compile-fail doc-test steps. Never workflow-wide. |

## Coverage report

Published by the `coverage` check in its run summary, pass or fail (FR-006).

| Field | Source | Rules |
|---|---|---|
| `percent` | `cargo llvm-cov report --json --summary-only` → `.data[0].totals.lines.percent` | Generated models excluded by the same regex as the gate |
| `covered`, `count` | same object, `.covered`, `.count` | shown next to the percentage |
| `threshold` | constant `70` | same number as `--fail-under-lines` |
| `result` | `percent >= threshold` | `pass` or `fail`; the gate step decides the check |

## Cargo.toml lint table

| Key | Value | Effect |
|---|---|---|
| `[lints.rust] missing_docs` | `"warn"` | Root package only. Warns locally; fails CI through `RUSTFLAGS: -D warnings` in every leg (FR-007). |
