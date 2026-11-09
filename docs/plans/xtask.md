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

## FR-6.14: precise tooling errors (added 2026-11-09)

From the read-only audit of 2026-11-09 against FR-6.14 (line numbers at
`dev` 49bdc7a). A change at step 5: xtask is a binary, so its error
types are internal and step 4 is a no-op.

- [x] T201 [contract] Replace `Diagnostic { file, message: String }` (`codegen.rs:30-36`) with a typed diagnostic: one variant per check cause, each holding `file`, `model`, `register`, `field` and the deciding values as fields; `Display` keeps today's text, built from the fields; a separate `hint()` carries advice (FR-6.12), and no message carries advice any more (e.g. `output.rs:29` "run rustup component add rustfmt" moves to the hint). FR-6.14
- [x] T202 Silent skip, `fr9.rs:665-669`: a torque switch range outside `u8` (`u8::try_from(..).ok()?`) returns `None` with no diagnostic, so codegen exits 0 and MX-64 P1 falls back to `ModeSource::AngleLimits`, losing `HasGoalCurrent` and `HasTorqueCtrlModeEnable`. Report a "switch range outside u8" diagnostic with the observed min and max; test with `(-1, 1)`. FR-6.14, Principle III
- [x] T203 Panic, `fr9.rs:353` (also 356, `render_tests.rs:127`): `1i64 << bits` overflows for a row of size 0 or ≥ 8 that has a `range`. Check the row size before `raw_span` and report it as a diagnostic; test both sizes. FR-6.14 (no input panics)
- [x] T204 A range with `min > max` (`fr9.rs:472` → 486, 532) is reported as "does not fit N byte(s)": give it its own cause with min and max. FR-6.6, FR-6.8
- [x] T205 Split `Failure::Io(String)` (`output.rs:54`, 56, 64, 71, 72, 86) into one variant per operation (create dir, write, list dir, read entry, remove), each with the path and the `io::Error` kept typed. Attribute a failed `read_dir` of `control_tables/` to the directory, not `dynamixel.ron` (`codegen.rs:169-171`). Report the read errors and non-UTF-8 names ignored today (`output.rs:61`, 73-75; `codegen.rs:176`). FR-6.14
- [x] T206 Split `Failure::Rustfmt(String)` (`output.rs:18-45`): not found, spawn, stdin, wait, write, writer panicked, rejected (exit status and stderr: rustfmt refused the generated code), non-UTF-8 output; "not found" gets the install hint. FR-6.14
- [x] T207 RON parse errors: one `Parse` variant holding the file, line, column and the `ron` error code typed, not stringified (positions are exact, `schema.rs:379-403`). FR-6.14
- [x] T208 Add the missing deciding values: `from_fw` order (`resolve.rs:414`, 423: the observed values, the offending pair, the file); switch and goal rows with wrong attributes (`fr9.rs:634-638`, 654-656: the found size, access and area, as `resolve.rs:513` does); per-mode lists, `default`, both limit forms (`fr9.rs:454`, 588; `resolve.rs:224`, 573: name the items present); a range without `unit_source` (`fr9.rs:459`: one diagnostic per row, not the first only); Model Number access/area (`resolve.rs:713-726`: list each table's value instead of a majority vote); bad arguments (`main.rs:14-16`: echo what was received). FR-6.7, FR-6.8 as adapted by FR-6.14
- [x] T209 NFR-12 for xtask: replace the substring assertions (`resolve.rs:825-834`, `codegen.rs:808-829`) with variant and field assertions; one message test per variant checking each deciding value; cover the paths with no test today (T202 to T207, the usage path), and the tests that skip deciding values (max_radian, check 13 locations, "cannot read", RON `line:col`, check 22 row names). NFR-12
