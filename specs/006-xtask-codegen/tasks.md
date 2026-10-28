---

description: "Task list for the xtask codegen module (generated models, dispatch and family features), PR X1"
---

# Tasks: xtask codegen (generated models, dispatch and family features)

**Input**: Design documents from `specs/006-xtask-codegen/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/codegen-cli.md,
contracts/generated-api.md, quickstart.md, `specs/ideas/module-interfaces.md` (cited as
"contract §n"), constitution v1.3.0

**Tests**: Required (FR-017, SC-002, SC-004, SC-006, SC-008; constitution: generated code is
excluded from coverage and tested through xtask). Test tasks come first in each story and must fail
(or not compile) before the implementation tasks after them. Every acceptance scenario of
spec.md is named by at least one test task, including the codegen failure on a mapped table
without protocol data (FR-012, clarification 2).

**Organization**: One PR (**X1**, contract §7). Phases follow the spec's story priorities. Every
story touches the same xtask files (`resolve.rs`, `render.rs`, `render_tests.rs`), so stories run
one after the other inside the PR; each ends at a checkpoint that regenerates `src/models*` and
passes its tests.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1 = common-role data, US2 = per-protocol lookup, US3 = family features,
  US4 = validated, readable codegen, US5 = concrete model types
- **[human]**: only a human may do it (file listed in `.aiignore`, or an architect decision)

## Rules for every task

- Paths an agent may change in X1: `xtask/**`, and `src/models.rs` + `src/models/**` **only by
  running `cargo xtask codegen`** (never by hand). No other file under `src/`, no `Cargo.toml`,
  no `control_tables/**` change is committed (FR-018, contract §1). The one exception is the C2
  fallback in T031, flagged as a cross-module touch.
- Rust 2024. Layout `xtask/src/codegen.rs` + `xtask/src/codegen/*.rs`, never `mod.rs`
  (`clippy::mod_module_files` is on in `xtask/src/main.rs`).
- Document every public item, in xtask and in generated code (constitution VII). No comments
  inside fn bodies unless they resolve a real ambiguity; this also applies to generated code.
- No new dependency or dev-dependency in `xtask/Cargo.toml` (plan, research R-13): `ron` 0.12.2 and
  `serde` only; the existing `tempdir` helper replaces `tempfile`.
- Generated non-test code: no `dyn`, no allocation, no `unwrap`/`expect`/`panic!`, no indexing,
  no arithmetic outside `const` initialisers (contracts/generated-api.md "Visibility summary").
- `src/models/tests.rs` must hold no literal register address or size: it refers to constants by
  name (`Type::FIRMWARE_VERSION`, `Type::DATA`), so a one-row table change still changes one file
  (SC-007).
- Never stage `.aiignore`, `.claude/**`, `.specify/**`, `.github/workflows/ov-*.yml`,
  `.github/ov-ruleset-main.json` or `docs/srs/SRS.md` (human-only, `.aiignore`). Never stage
  `specs/**` on the PR branch (specs live on `dev`).
- Commits and the PR carry no AI co-authorship or attribution lines (constitution, user memory).
  The PR is opened as Ovsiankina.

---

## Phase 1: Setup (PR X1)

**Purpose**: Confirm the prerequisites are merged, branch, and split the xtask module.

- [ ] T001 Verify prerequisites on `origin/main` and branch. Check, and stop with a report naming what is missing if any check fails (never work around it): (a) protocol PR #7 merged: `src/protocol.rs` has `pub enum ProtocolVersion { P1, P2 }` (contract §5); (b) core-1 merged: `src/actuator.rs` has `ModelNumber` with `get()`, `Cache` with `new`/`id`/`limits`/`record_limits`, `ModelData` with fields `name, model_number, protocols, torque_enable, goal_position, present_position, limits, hardware_error, scale`, `LimitRegisters::{MinMax { min, max }, CwCcw { cw, ccw }}`, `Protocols { p1, p2 }` with `supports`, and `Describe` nameable as `crate::actuator::Describe` (plan C1); `src/register.rs` has `Register { addr, size, access, area }` with `pub(crate)` fields and `Access::{R, RW}`, `Area::{Eeprom, Ram}`; `src/error.rs` has `SetupError::UnsupportedModel { id, model_number, protocol, feature }` and still the `#[doc(hidden)]` `Error::UnsupportedModel` shim; `PositionScale::new` is a `pub(crate) const fn`; (c) tables T1 merged: every `control_tables/*.ron` has `source`, `protocols`, and `access`/`area` on every row; `control_tables/dynamixel.ron` has `p1`/`p2`/`default`/`not_mapped` beside legacy `tables`. Also record (for T061) whether `src/lib.rs` re-exports `ModelNumber`, `PositionScale` and `SetupError` at the crate root, and whether core-1 removed the `ActuatorKind::new(u16, u8, Id)` tests from `src/actuator.rs` (plan C2). Then `git switch -c 006-xtask-x1 origin/main`
- [ ] T002 Split the xtask module (research R-13): create `xtask/src/codegen/{schema,names,resolve,render,render_tests,output,fixtures}.rs`, each with a `//!` module doc stating its one job; declare them in `xtask/src/codegen.rs` (`mod schema; mod names; mod resolve; mod render; mod render_tests; mod output; #[cfg(test)] mod fixtures;`). Move, unchanged: `type_ident`, `module_ident`, `spell_leading_digit`, `const_ident` and the tests `type_names`, `module_names`, `const_names` to `xtask/src/codegen/names.rs`; `braces_to_parens` and `braces_become_parens_outside_strings` to `xtask/src/codegen/schema.rs`; the `tempdir` test module to `xtask/src/codegen/fixtures.rs` as `pub(crate) struct Dir` with `new()` and `path()`. Leave `generate`, rendering and the other tests in `xtask/src/codegen.rs` working as today
- [ ] T003 Baseline gates in the split state: `cargo test -p xtask`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo xtask codegen && git diff --exit-code -- src/models.rs src/models` (today's xtask reads T1's legacy `tables:` field, so output must be unchanged). Fix fallout only in `xtask/**`

---

## Phase 2: Foundational (PR X1, blocking prerequisites)

**Purpose**: New input schema, diagnostics, the validate-then-write pipeline (research R-1, R-2,
R-3), and test fixtures. At the end, xtask reads the T1 schema and still produces **byte-identical**
output to today, through the new pipeline.

**⚠️ CRITICAL**: No user-story work starts until this phase is complete.

- [ ] T004 [P] Schema tests in `#[cfg(test)] mod tests` of xtask/src/codegen/schema.rs (must not compile before T006): `parses_unverified_table` (T1-shaped table with `source: Unverified`, `protocols: [P2]`, `type_info`, rows with `access`/`area`); `parses_verified_source` (`source: Verified(url: "https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/", verified: "2026-10-28")` gives `Source::Verified { url, verified }`); `row_without_access_fails` and `row_without_area_fails` (error text contains `access` / `area`); `table_without_source_fails`; `table_without_protocols_fails`; `ignores_unit_info_and_instructions`; `area_names` (`EEPROM` → `Area::Eeprom`, `RAM` → `Area::Ram`); `parses_map_with_implicit_some` (`p1`, `p2`, `default: P2`, `also: ["AX-12+"]`, `not_mapped` with `NoModelNumber` and `SameNumberAs(1270)`); `legacy_tables_field_is_ignored`; `empty_list_differs_from_absent` (`p1: []` → `Some(vec![])`, no `p1` → `None`)
- [ ] T005 [P] Names tests in `#[cfg(test)] mod tests` of xtask/src/codegen/names.rs: keep the moved tests; add `two_xc430_w250` (`type_ident("2xc430_w250") == "TwoXc430W250"`, `module_ident` → `two_xc430_w250`) and `reserved_module_idents` (`is_reserved_module` is true for `tests`, `type`, `mod`, `async`, `gen`, false for `xm430_w350`)
- [ ] T006 Implement the input types in xtask/src/codegen/schema.rs (data-model §1, contract §3), all `#[derive(Debug, Deserialize)]`, no `deny_unknown_fields` (research R-1). `ModelMap { models: BTreeMap<u16, ModelEntry>, #[serde(default)] not_mapped: Vec<NotMapped> }` — `models`: "keys are model numbers"; `not_mapped`: "tables on disk that no model number uses, on purpose"; legacy `tables`: "ignored". `ModelEntry { family: String, name: String, #[serde(default)] also: Vec<String>, p1: Option<Vec<TableRef>>, p2: Option<Vec<TableRef>>, default: Option<ProtocolTag> }` — `family`: "one of `ax`, `mx`, `x-series`, `y-series`, `pro`"; `p1`, `p2`: "at least one present; a present list is non-empty"; `default`: "required and `P2` when both lists exist; absent otherwise". `TableRef { from_fw: u8, file: String }` — `from_fw`: "the first is 0, then strictly increasing within one list"; `file`: "the stem of an existing `control_tables/<file>.ron`". `NotMapped { file: String, reason: NotMappedReason }`, `enum NotMappedReason { NoModelNumber, SameNumberAs(u16) }` — `file`: "the file exists and is not in any `p1`/`p2` list". `ControlTable { source: Source, protocols: Vec<ProtocolTag>, type_info: TypeInfo, control_table: Vec<Row> }`; `enum Source { Unverified, Verified { url: String, verified: String } }`. `TypeInfo { value_of_zero_radian_position: i32, value_of_min_radian_position: i32, value_of_max_radian_position: i32, min_radian: f32, max_radian: f32 }`. `Row { address: u16, size: u8, data_name: String, access: Access, area: Area }` with `enum Access { R, RW }` and `enum Area { #[serde(rename = "EEPROM")] Eeprom, #[serde(rename = "RAM")] Ram }`; `enum ProtocolTag { P1, P2 }` (also `Clone, Copy, PartialEq, Eq, PartialOrd, Ord`). Functions `parse_map(path) -> Result<ModelMap, Diagnostic>` with `ron::Options::default().with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)` and `parse_table(path) -> Result<ControlTable, Diagnostic>` after `braces_to_parens`; the `Diagnostic.file` is the file stem (`dynamixel` for the map), the message the I/O or ron error
- [ ] T007 Implement `pub(crate) fn is_reserved_module(ident: &str) -> bool` in xtask/src/codegen/names.rs: true for `tests` (reserved for the generated test module, research R-16) and for every Rust 2024 strict and reserved keyword (`as break const continue crate else enum extern false fn for if impl in let loop match mod move mut pub ref return self Self static struct super trait true type unsafe use where while async await dyn abstract become box do final macro override priv typeof unsized virtual yield try gen`)
- [ ] T008 In xtask/src/codegen.rs define, with unit tests in its `mod tests`: `pub struct Diagnostic { pub file: String, pub message: String }` (data-model §2: "`file: String` (the table or `dynamixel`), `message: String`"), deriving `Ord` by `(file, message)`, `Display` = `control_tables/{file}.ron: {message}`; `pub struct Report { pub written: Vec<PathBuf>, pub removed: Vec<PathBuf>, pub unchanged: usize, pub warnings: Vec<Diagnostic> }`; `pub enum Failure { Checks { errors: Vec<Diagnostic>, warnings: Vec<Diagnostic> }, Rustfmt(String), Io(String) }`. Tests: `diagnostic_display`, `diagnostics_sort_by_file_then_message`
- [ ] T009 [P] Fixture builders in xtask/src/codegen/fixtures.rs (`#[cfg(test)]`, research R-11 layer 1): `Table` builder rendering T1-shaped RON with `Table::xm()` = `source: Unverified`, `protocols: [P1, P2]`, `type_info` (2048, 0, 4095, -3.14159265, 3.14159265), rows Model Number (0, 2, R, EEPROM), Firmware Version (6, 1, R, EEPROM), Max Position Limit (48, 4, RW, EEPROM), Min Position Limit (52, 4, RW, EEPROM), Torque Enable (64, 1, RW, RAM), Hardware Error Status (70, 1, R, RAM), Goal Position (116, 4, RW, RAM), Present Position (132, 4, R, RAM); `Table::ax()` = `protocols: [P1]`, `type_info` (512, 0, 1023, -2.6205530, 2.6154347), rows Model Number (0, 2, R, EEPROM), Firmware Version (2, 1, R, EEPROM), CW Angle Limit (6, 2, RW, EEPROM), CCW Angle Limit (8, 2, RW, EEPROM), Torque Enable (24, 1, RW, RAM), Goal Position (30, 2, RW, RAM), Present Position (36, 2, R, RAM), no Hardware Error Status; modifiers `without(name)`, `with_row(address, size, name, access, area)`, `protocols(&str)`, `source(&str)`, `type_info(zero, min, max, min_rad, max_rad)`, `omit_field("source" | "protocols")`. `Map` builder: `model(number, family, name)`, `.p1(&[(u8, &str)])`, `.p2(..)`, `.default_p2()`, `.also(&[..])`, `.not_mapped(file, reason)`, `.legacy_tables(..)`. `Root::new(&Map, &[(&str, Table)])` writing `control_tables/` and `src/` under a `Dir`; `Root::sentinels()` writing `src/models.rs` and `src/models/old.rs` with known bytes and returning a snapshot of (path, bytes, mtime); `Root::assert_unchanged(&snapshot)`. `pub(crate) struct Failing { pub check: u8, pub root: Root, pub file: &'static str, pub keywords: &'static [&'static str] }` and `pub(crate) fn failing() -> Vec<Failing>` (empty here; stories add one entry per check of data-model §3)
- [ ] T010 [P] Output tests in `#[cfg(test)] mod tests` of xtask/src/codegen/output.rs: `formats_on_stdin` (`"fn  a(){}"` → `"fn a() {}\n"`); `rustfmt_error_names_file` (invalid Rust → `Failure::Rustfmt` whose text contains `src/models/x.rs`); `missing_rustfmt` (`format_with("rustfmt-does-not-exist", ..)` → message ``rustfmt not found: run `rustup component add rustfmt` ``); `sync_writes_only_changed` (second sync: `written` empty, `unchanged == n`, mtimes equal); `sync_removes_stale_rs_only` (a stale `src/models/gone.rs` is removed and listed in `removed`; `src/models/notes.txt` and `src/lib.rs` are untouched); `sync_creates_models_dir`
- [ ] T011 Implement xtask/src/codegen/output.rs (research R-2, R-14, contracts/codegen-cli.md "Writes"): `pub(crate) fn format(rel: &str, text: &str) -> Result<String, Failure>` = `format_with("rustfmt", rel, text)`, which runs `<program> --edition 2024 --emit stdout` with the text on stdin; `ErrorKind::NotFound` → `Failure::Rustfmt("rustfmt not found: run `rustup component add rustfmt`")`; non-zero exit → `Failure::Rustfmt("rustfmt failed on src/<rel>: <stderr>")`. `pub(crate) fn sync(src: &Path, files: &[(String, String)]) -> Result<Report, Failure>`: create `src/models/` if absent; write a file only when its bytes differ; delete every `src/models/*.rs` not in `files`; never touch anything outside `src/models.rs` and `src/models/` (FR-018); I/O errors → `Failure::Io("<path>: <error>")`
- [ ] T012 Implement the resolved model in xtask/src/codegen/resolve.rs (data-model §2) except roles and checks: `pub(crate) struct GenModel { file, type_ident, module_ident, number: u16, name, also: Vec<String>, family: String, appearances: Vec<(ProtocolTag, u8, Option<u8>)>, protocols: Vec<ProtocolTag>, source: Source, scale: TypeInfo, registers: Vec<(String, Row)> }` — `file, type_ident, module_ident`: "file stem; a leading digit is spelled out (`2xl430_w250` → `TwoXl430W250` / `two_xl430_w250`)"; `appearances`: "list of `(ProtocolTag, from_fw, until_fw: Option<u8>)`, one per list naming the file"; `registers`: "list of `(const_ident, Row)` in table order". `pub(crate) struct LookupList { protocol: ProtocolTag, number: u16, family: String, variants: Vec<(u8, String)> }` with `fn firmware_dependent(&self) -> bool { variants.len() > 1 }`. `pub(crate) struct Resolved { models: Vec<GenModel>, lookups: Vec<LookupList> }`, `models` ordered by model number, then lowest `from_fw`, then file stem; `lookups` ordered by model number, P1 before P2 (research R-8). `pub(crate) fn resolve(map, tables: &BTreeMap<String, ControlTable>, on_disk: &BTreeSet<String>) -> Outcome` with `Outcome { resolved, errors: Vec<Diagnostic>, warnings: Vec<Diagnostic> }`; a file under both `p1` and `p2` of one number is one `GenModel` with two appearances
- [ ] T013 Rewire the pipeline in xtask/src/codegen.rs and xtask/src/main.rs: `pub fn generate(dir: &Path) -> Result<Generated, Failure>` parses the map (a failure is one `dynamixel` diagnostic and stops), then each referenced table once (a missing or unparsable table is one diagnostic for that table; the others are still checked, research R-3), calls `resolve`, and returns `Failure::Checks` (errors sorted) if any error, else `Generated { files: Vec<(String, String)>, warnings }`; `pub fn run(root: &Path) -> Result<Report, Failure>` = `generate(root/control_tables)` → `output::format` every file (all before any write) → `output::sync(root/src, ..)`. Move today's `render_model`/`render_module`/`firmware_doc`/`cfg_attr` into xtask/src/codegen/render.rs, reading `GenModel`/`Resolved` and keeping today's output text (`Register::new(addr, size)`, three-method `Actuator` impls, `new(model_number: u16, firmware: u8, id: Id) -> Result<Self, Error>` built from each number's default list: `p2` when present, else `p1`). `render_tests.rs` gets `pub(crate) fn render_tests(&Resolved) -> Option<String>` returning `None` for now (no `tests.rs`, no `mod tests;` line yet). Rewrite today's tests `generates_model_and_dispatch`, `rejects_duplicate_register_names`, `rejects_duplicate_type_names`, `reports_missing_files` on the new schema with `fixtures.rs`. `main.rs` calls `codegen::run(&workspace_root())`; delete `run_codegen`
- [ ] T014 Checkpoint: `cargo test -p xtask`, `cargo clippy --workspace --all-targets -- -D warnings`, then `cargo xtask codegen && git diff --exit-code -- src/models.rs src/models` must show **no diff** (same output, new pipeline). If T1 maps a file only under `p1` that the legacy `tables` lacks, the only allowed diff is that model's new file and its lines in `src/models.rs`; record it for T061. Commit `xtask/**`

**Checkpoint**: Foundation ready. The pipeline validates before writing and reads the T1 schema.

---

## Phase 3: User Story 1 - Common operations work on every model (Priority: P1, PR X1) 🎯 MVP

**Goal**: Every model type holds a `Cache`, carries `DATA: ModelData` (roles, limit form, fault
register or `None`, scale, protocols), builds registers through `reg()` with access and area, and
implements `Describe`; `ActuatorKind` delegates `Describe` by `match`.

**Independent Test**: `cargo test -p xtask resolve render real_tables` (role resolution, rendered
`DATA`, real-tables oracle for XM430-W350, AX-12A, XL-320, PH54-200-S500, YM070-210-M001) and
`cargo test --lib models::tests` (cache round trip through `ActuatorKind`).

### Tests for User Story 1 ⚠️

> Write first; they must fail or not compile before T020–T023.

- [ ] T015 [P] [US1] Role and check tests in `mod tests` of xtask/src/codegen/resolve.rs: `roles_min_max` (`Table::xm()`: Torque Enable 64/1, Goal Position 116/4, Present Position 132/4, `MinMax { min: 52, max: 48 }`, hardware error `Some(70)`; US1 AS1); `roles_cw_ccw` (`Table::ax()`: `CwCcw { cw: 6, ccw: 8 }`; US1 AS2); `no_fault_register_is_none` (AX fixture → `None`; and a table with only `Error Code` at 153 → `None`, research R-4; US1 AS3); `scale_from_type_info` (US1 AS4). Data-model §3 checks, each asserting the diagnostic's `file` and key words: check 10 "a missing role, half a limit form, or both limit forms" → `check10_missing_goal_position` (`missing role Goal Position`), `check10_half_limit_form`, `check10_both_limit_forms`; check 11 "wrong role attributes (Torque Enable and Goal Position not `RW`/`RAM`; Present Position not `R`/`RAM`; a limit not `RW`/`EEPROM`)" → `check11_torque_enable_attrs` (`must be RW/RAM, found R/RAM`), `check11_goal_position_attrs`, `check11_present_position_attrs`, `check11_limit_attrs`; check 15 "bad scale: `max_raw ≤ min_raw`, `max_rad ≤ min_rad`, or zero outside `min..=max`" → `check15_raw_range`, `check15_radian_range`, `check15_zero_outside`. Add one `Failing` entry each for checks 10, 11 and 15 to `failing()` in xtask/src/codegen/fixtures.rs
- [ ] T016 [P] [US1] Render tests in `mod tests` of xtask/src/codegen/render.rs: the `Table::xm()` model file contains `pub struct Xm {` with field `cache: Cache`, `Self { cache: Cache::new(id) }`, `pub(crate) const GOAL_POSITION: Register = super::reg(116, 4, Access::RW, Area::Ram);`, `pub(crate) const DATA: ModelData = ModelData {`, `limits: LimitRegisters::MinMax { min: Self::MIN_POSITION_LIMIT, max: Self::MAX_POSITION_LIMIT }`, `hardware_error: Some(Self::HARDWARE_ERROR_STATUS)`, `protocols: Protocols { p1: true, p2: true }`, `scale: PositionScale::new(2048, 0, 4095, <{:?} of -3.14159265_f32>, <{:?} of 3.14159265_f32>)` (expected text built with `format!("{:?}", v)`, research R-6), `impl Describe for Xm` returning `&Self::DATA`, `&self.cache`, `&mut self.cache`, `fn id(&self) -> Id` returning `self.cache.id()`; the `Table::ax()` file contains `LimitRegisters::CwCcw { cw: Self::CW_ANGLE_LIMIT, ccw: Self::CCW_ANGLE_LIMIT }` and `hardware_error: None`. `models.rs` contains `const fn reg(addr: u16, size: u8, access: Access, area: Area) -> Register {`, `Register { addr, size, access, area }`, the `#![allow(dead_code, unused_variables, clippy::approx_constant, reason = ...)]` attribute, `impl Describe for ActuatorKind` with `match *self` and `Self::Xm(ref m) => m.data()` / `Self::Xm(ref mut m) => m.cache_mut()`. No output contains `Register::new` (plan D1)
- [ ] T017 [P] [US1] Real-tables oracle test `real_tables_roles_match_oracle` in `mod tests` of xtask/src/codegen.rs (SC-002, research R-11 layer 2): run `generate` on `<workspace>/control_tables` (workspace = parent of `CARGO_MANIFEST_DIR`); for every mapped table, find each role row by exact `data_name` with a lookup written in the test (`Torque Enable`, `Goal Position`, `Present Position`, `Min/Max Position Limit` or `CW/CCW Angle Limit`, `Hardware Error Status`), then check the resolver's `Roles`, the rendered `reg(addr, size, Access::.., Area::..)` line of each, the `LimitRegisters` form, `hardware_error`, and that the rendered `PositionScale::new(..)` equals the table's `type_info` (`{:?}` for radians). Assert the number of checked tables equals the number of distinct mapped files (no hard-coded 69). Spot checks: XM430-W350 `GOAL_POSITION` = `super::reg(116, 4, Access::RW, Area::Ram)` (US1 AS1); AX-12A `CwCcw` with CW 6 and CCW 8 (US1 AS2) and `hardware_error: None` (US1 AS3); XL-320 `CwCcw` 6/8 and Hardware Error Status 50; PH54-200-S500 Goal Position 564 and Hardware Error Status 518; YM070-210-M001 `hardware_error: None` although it has `Error Code` at 153 (spec Assumptions); every model's scale from its `type_info` (US1 AS4)
- [ ] T018 [P] [US1] Render-tests tests in `mod tests` of xtask/src/codegen/render_tests.rs: for a two-model fixture, `render_tests` returns `Some(text)` starting with the `@generated` header, with one `#[test]` per model type named `<module>_describe_round_trip`, each under `#[cfg(feature = "<family>")]`, that builds `ActuatorKind::<Type>(<Type>::new(id))`, asserts `kind.data() == &<Type>::DATA`, `kind.cache().id() == id`, then `kind.cache_mut().record_limits(10, 20)` is `Ok` and `kind.cache().limits()` has `min_raw() == 10`, `max_raw() == 20` (US1 AS5, FR-009). The text contains no `reg(` and no address digits from the fixture rows (SC-007)

### Implementation for User Story 1

- [ ] T019 [US1] Implement roles in xtask/src/codegen/resolve.rs (data-model §2 "Roles", research R-4): `pub(crate) struct Roles { model_number, firmware_version, torque_enable, goal_position, present_position: Row, limits: Limits, hardware_error: Option<Row> }` with `enum Limits { MinMax { min: Row, max: Row }, CwCcw { cw: Row, ccw: Row } }`, matched by exact `data_name` (`Model Number`, `Firmware Version`, `Torque Enable`, `Goal Position`, `Present Position`, `Min Position Limit` + `Max Position Limit` or `CW Angle Limit` + `CCW Angle Limit`, `Hardware Error Status`); add `roles: Roles` to `GenModel`. Add check 10 (names "table, role"), check 11 ("table, role, found"), check 15 ("table"). Message forms from contracts/codegen-cli.md: `missing role Goal Position (row "Goal Position")`, `role Torque Enable must be RW/RAM, found R/RAM`
- [ ] T020 [US1] Render model files in xtask/src/codegen/render.rs per contracts/generated-api.md "`src/models/<module>.rs`": imports `use crate::{Actuator, Id, PositionScale};`, `use crate::actuator::{Cache, Describe, LimitRegisters, ModelData, Protocols};`, `use crate::register::{Access, Area, Register};` (use the owning-file paths from specs/004-core/contracts/core-api.md "Paths for xtask and bus" for any root re-export T001 found missing); `#[derive(Debug, Clone, PartialEq, Eq)] pub struct <Type> { cache: Cache }`; `pub(crate) const fn new(id: Id) -> Self`; `pub(crate) const DATA: ModelData` with `name`, `model_number`, `protocols: Protocols { p1, p2 }` from the table's `protocols`, the role fields by constant name, `limits`, `hardware_error`, `scale: PositionScale::new(zero, min, max, min_rad, max_rad)` with radians printed by `{:?}` (research R-6, R-7); one `pub(crate) const <NAME>: Register = super::reg(addr, size, Access::R|RW, Area::Eeprom|Ram);` per row in table order; `impl Describe` (`&Self::DATA`, `&self.cache`, `&mut self.cache`); `impl Actuator` with `id` → `self.cache.id()`, `model_number` → literal, `name` → literal (kept until core-2, research R-12). Keep today's doc lines (US4 replaces them)
- [ ] T021 [US1] Render `src/models.rs` changes in xtask/src/codegen/render.rs: the module-level `#![allow(dead_code, unused_variables, clippy::approx_constant, reason = "every register is generated but only some are used; with no family enabled some parameters are unused; type_info angles are table data, not uses of π")]`; private `const fn reg(addr: u16, size: u8, access: Access, area: Area) -> Register { Register { addr, size, access, area } }` documented "Builds one register row. Private to `models`; model files call `super::reg`." (plan D1); `impl Describe for ActuatorKind` with one `match *self` per method (`ref` / `ref mut`, research R-8: `match self {}` fails on the empty enum); `impl Actuator for ActuatorKind` delegating `id`, `model_number`, `name` by `match *self`; `#[cfg(test)] mod tests;` when `render_tests` returns `Some`. Keep the old `new(u16, u8, Id) -> Result<Self, Error>` lookup so the crate still builds against core-1's `Error::UnsupportedModel` shim (US2 replaces it)
- [ ] T022 [US1] Implement the round-trip part of `render_tests` in xtask/src/codegen/render_tests.rs (research R-11 layer 3): file header `// @generated by `cargo xtask codegen`. Do not edit: change `control_tables/` and regenerate.`, a `//!` doc, imports used in every family combination only (`use crate::Id; use crate::actuator::Describe;`), family-specific names written as full paths (`crate::models::<Type>`); one `<module>_describe_round_trip` test per model type as in T018; emit it as `src/models/tests.rs`
- [ ] T023 [US1] Regenerate with `cargo xtask codegen`; run `cargo test -p xtask`, `cargo test --lib models::tests`, `cargo clippy --workspace --all-targets -- -D warnings`; commit `xtask/**` with the regenerated `src/models.rs` and `src/models/**`

**Checkpoint**: The bus can read every common role, limit form, fault register and scale through
`ActuatorKind` (US1 done, MVP of X1).

---

## Phase 4: User Story 2 - Setup turns any reply into the right model (Priority: P1, PR X1)

**Goal**: `ActuatorKind::new(ProtocolVersion, ModelNumber, u8, Id) -> Result<Self, SetupError>`,
`ActuatorKind::firmware_register`, `MODEL_NUMBER`, and the map checks that make the lookup safe.

**Independent Test**: `cargo test --lib models::tests` (every (protocol, number) at firmware 0,
each boundary and its predecessor, 255) and `cargo test -p xtask resolve render real_tables`.

### Tests for User Story 2 ⚠️

> Write first; they must fail or not compile before T029–T032.

- [ ] T024 [P] [US2] Map and lookup checks in `mod tests` of xtask/src/codegen/resolve.rs (data-model §3), each asserting `file` and key words: check 2 "`from_fw` not starting at 0, or not strictly increasing, per list" (`check02_first_not_zero`, `check02_not_increasing`: message names `model 1190` and `p2`, e.g. `model 1190: p2 from_fw must strictly increase`); check 3 "no `p1` and no `p2`, or an empty list" (`check03_no_lists`, `check03_empty_list`); check 4 "`default` missing or not `P2` with both lists; present with one list" (`check04_missing_default`, `check04_default_p1`, `check04_default_with_one_list`); check 6 "a file under two model numbers" (`check06_file_under_two_numbers`, names both numbers); check 8 "a mapped table without `source` or `protocols` (parse error), or with empty `protocols`" (`check08_missing_protocols`, `check08_missing_source`, `check08_empty_protocols`: each fails and names the table, FR-012 clarification); check 9 "a list `pX` names a table whose `protocols` lacks `PX`" (`check09_p1_list_without_p1`, names table and `P1`); check 12 "Model Number not at (0, 2), or its access/area differ between tables" (`check12_model_number_address`, `check12_model_number_access`); check 13 "Firmware Version location differs between variants of one (number, protocol)" (`check13_firmware_version_moves`, names the number and both tables); check 14 "a role above address 255 in a table listed under `p1`" (`check14_p1_role_above_255`, `listed under p1 but Goal Position is at address 564 (> 255)`). Plus `lookup_lists_per_protocol` (1190 with `p2: [(0, "fw52"), (53, "new")]` → one P2 `LookupList`, `firmware_dependent()`), `same_file_under_p1_and_p2` (one `GenModel`, two appearances, two `LookupList`s). Add one `Failing` entry per check 2, 3, 4, 6, 8, 9, 12, 13, 14 to `failing()` in xtask/src/codegen/fixtures.rs
- [ ] T025 [P] [US2] Lookup render tests in `mod tests` of xtask/src/codegen/render.rs (contracts/generated-api.md "Lookup semantics"): `models.rs` contains `pub(crate) fn new(protocol: ProtocolVersion, model_number: ModelNumber, firmware: u8, id: Id) -> Result<Self, SetupError>` and `match (protocol, model_number.get())`; for the 1190 fixture `(ProtocolVersion::P2, 1190) if firmware < 53 => Ok(Self::Fw52(Fw52::new(id)))` before `(ProtocolVersion::P2, 1190) => Ok(Self::New(New::new(id)))`, a `#[cfg(not(feature = "x-series"))]` arm returning `SetupError::UnsupportedModel { id, model_number: 1190, protocol, feature: Some("x-series") }`, and the final `_` arm with `feature: None`; P1 arms come before P2 arms of the same number; `firmware_register` has `(ProtocolVersion::P2, 1190) => Some(Fw52::FIRMWARE_VERSION)` and no arm for a one-table number; `pub(crate) const MODEL_NUMBER: Register = reg(0, 2, Access::R, Area::Eeprom);`; the AX fixture's `DATA` has `protocols: Protocols { p1: true, p2: false }` (US2 AS5)
- [ ] T026 [P] [US2] Lookup render-tests tests in `mod tests` of xtask/src/codegen/render_tests.rs: for each (protocol, number) the generated tests call `ActuatorKind::new(ProtocolVersion::PX, ModelNumber(n), fw, id)` with `fw` in {0, each `from_fw` b > 0 and b − 1, 255}, deduplicated and sorted, and expect the variant of the highest `from_fw ≤ fw` (1190 fixture: 0 → `Fw52`, 52 → `Fw52`, 53 → `New`, 255 → `New`; US2 AS1, SC-004); `firmware_register(P2, ModelNumber(1190)) == Some(<Fw52>::FIRMWARE_VERSION)` and `None` for a one-table number (US2 AS2); `MODEL_NUMBER == <Type>::MODEL_NUMBER` for every type (FR-008); the largest `u16` not in the map gives `feature: None` on both protocols (US2 AS4); a number with tables only on P2, asked on P1, gives `feature: None`; every `Ok` lookup on protocol P has `kind.data().protocols.supports(P)`; the argument is a `ModelNumber` value (US2 AS6)
- [ ] T027 [P] [US2] Real-tables lookup test `real_tables_lookup_and_protocols` in `mod tests` of xtask/src/codegen.rs (SC-002 rest): for every mapped table, the Model Number row is (0, 2) with the access/area rendered in `MODEL_NUMBER`; the Firmware Version row matches the oracle; the rendered `Protocols { p1, p2 }` equals the table's `protocols`; for every map list the rendered arms and `if firmware < N` guards follow `from_fw`. Spot checks: 1190 on P2 has `if firmware < 53` → `Xl330M077Fw52` and a `firmware_register` arm; 1020 has no `firmware_register` arm (US2 AS2); AX-12A renders `Protocols { p1: true, p2: false }` (US2 AS5); XL-320 (350) has no P1 arm

### Implementation for User Story 2

- [ ] T028 [US2] Implement checks 2, 3, 4, 6, 8 (empty `protocols`; missing fields already fail in `parse_table`), 9, 12, 13 and 14 in xtask/src/codegen/resolve.rs, each naming what data-model §3 "Names" lists (e.g. "map, model number, protocol"; "table, role, address"). Model Number access/area agreement and Firmware Version agreement per (number, protocol) use `Roles` from T019. Expose the agreed Model Number access/area on `Resolved` for `MODEL_NUMBER` (plan C4)
- [ ] T029 [US2] Render the lookup in xtask/src/codegen/render.rs per contracts/generated-api.md `src/models.rs` and research R-8, R-9: replace the old `new`; imports `use crate::{Actuator, Id, ModelNumber, SetupError};`, `use crate::actuator::{Cache, Describe, ModelData};`, `use crate::protocol::ProtocolVersion;`, `use crate::register::{Access, Area, Register};` (no `Error`, no `LimitRegisters`/`Protocols`/`PositionScale`, research R-10); one arm per (protocol, table) under the family `cfg`, guard `if firmware < <next from_fw>` on every table but the last; one `#[cfg(not(feature = "<family>"))]` arm per (protocol, number) with tables returning `Err(SetupError::UnsupportedModel { id, model_number: <n>, protocol, feature: Some("<family>") })`; final `_ => Err(SetupError::UnsupportedModel { id, model_number: model_number.get(), protocol, feature: None })`; `firmware_register` with one `Some(<FirstType>::FIRMWARE_VERSION)` arm per firmware-dependent (protocol, number) under the family `cfg`, else `None` (plan C3); `pub(crate) const MODEL_NUMBER: Register = reg(0, 2, Access::<agreed>, Area::<agreed>);` documented "Model Number location, the same in every table (codegen checks it). Read on P1 Setup."; doc comments of `new` and `firmware_register` copied from contracts/generated-api.md
- [ ] T030 [US2] Implement the lookup tests in xtask/src/codegen/render_tests.rs as in T026, the family-on ones under `#[cfg(feature = "<family>")]`, comparing with `assert_eq!` against `Ok(ActuatorKind::<Type>(<Type>::new(id)))`; add `use crate::{ModelNumber, SetupError}; use crate::protocol::ProtocolVersion;` only if every family combination uses them (the unknown-number test always does)
- [ ] T031 [US2] C2 fallback, only if T001 found core-1 kept tests calling `ActuatorKind::new(u16, u8, Id)` in `src/actuator.rs`: delete exactly those tests in its `#[cfg(test)]` block and nothing else; record it for T061 as a cross-module touch (plan C2). Otherwise mark this task done with "not needed"
- [ ] T032 [US2] Regenerate with `cargo xtask codegen`; run `cargo test -p xtask`, `cargo test --lib models::tests`, `cargo clippy --workspace --all-targets -- -D warnings`; commit `xtask/**` with the regenerated `src/models*`

**Checkpoint**: Setup can map any (protocol, model number, firmware) to a variant or a typed
`SetupError` (US2 done).

---

## Phase 5: User Story 3 - Compile only the families I use (Priority: P2, PR X1)

**Goal**: The family list is checked, every generated item is family-gated, and all 14 builds
(5 families alone, all, none; std and no_std) are warning-free, tests included.

**Independent Test**: quickstart.md §3 (the three `cargo test --lib ... models::tests` lines) and
§4 (the family matrix) under `-D warnings`.

### Tests for User Story 3 ⚠️

> Write first; they must fail or not compile before T036–T038.

- [ ] T033 [P] [US3] Check 5 test `check05_unknown_family` in `mod tests` of xtask/src/codegen/resolve.rs (data-model §3 check 5 "unknown family", names "map, model number, family"): family `x_series` on 1020 fails with a `dynamixel` diagnostic containing `model 1020`, `"x_series"` and `expected ax, mx, x-series, y-series, pro` (US3 AS3, FR-005). Add its `Failing` entry to `failing()` in xtask/src/codegen/fixtures.rs
- [ ] T034 [P] [US3] Gating render tests in `mod tests` of xtask/src/codegen/render.rs for a fixture with two families: every `mod`, `pub use`, `ActuatorKind` variant, `Describe`/`Actuator` match arm, lookup arm and `firmware_register` arm of a model is preceded by `#[cfg(feature = "<family>")]`; the family-off lookup arm by `#[cfg(not(feature = "<family>"))]`; the `use` lines of `models.rs` name only items used with no family enabled (no `LimitRegisters`, `Protocols`, `PositionScale`)
- [ ] T035 [P] [US3] Family-off render-tests tests in `mod tests` of xtask/src/codegen/render_tests.rs: per (protocol, number) with tables, a `#[cfg(not(feature = "<family>"))]` test asserting `ActuatorKind::new(PX, ModelNumber(n), 0, id) == Err(SetupError::UnsupportedModel { id, model_number: n, protocol: PX, feature: Some("<family>") })` (US3 AS2, US2 AS3, SC-004) and `firmware_register(PX, ModelNumber(n)) == None` (plan C3); the imports of `tests.rs` are the same in every combination

### Implementation for User Story 3

- [ ] T036 [US3] Implement check 5 in xtask/src/codegen/resolve.rs with `const FAMILIES: [&str; 5] = ["ax", "mx", "x-series", "y-series", "pro"]` (FR-005; family stays a `String` in serde, research R-1), message `model <n>: unknown family "<f>" (expected ax, mx, x-series, y-series, pro)`
- [ ] T037 [US3] Finish gating in xtask/src/codegen/render.rs and xtask/src/codegen/render_tests.rs: family `cfg` on every per-model item listed in T034, `models.rs` imports limited per research R-10, and the family-off tests of T035
- [ ] T038 [US3] Regenerate; run quickstart.md §3 (all three lines) and every line of quickstart.md §4 plus the all-families no_std line of §1, all under `-D warnings`, and the matching `cargo test --lib --no-default-features --features std,<family> models::tests` leg for each of the five families (ci feature-test legs, plan "Note to ci"). All pass with zero warnings (US3 AS1, AS2, SC-003). US3 AS4: the same generated x-series lookup tests pass in the all-families and the `std,x-series` builds, so enabling more families changes no existing result. Fix only in `xtask/**`, regenerate, commit

**Checkpoint**: Any family subset builds clean and fails Setup with the family to enable (US3 done).

---

## Phase 6: User Story 4 - Change a table and review a readable diff (Priority: P2, PR X1)

**Goal**: All 16 checks run before any write, failures leave `src/models*` untouched, unmapped
tables warn, reruns are byte-identical, one row change touches one file, and generated docs name
model, number, firmware range per protocol, source table and verification state.

**Independent Test**: `cargo test -p xtask` end-to-end tests on a temp root (quickstart.md §2,
US4 rows) and the manual failure in quickstart.md §7.

### Tests for User Story 4 ⚠️

> Write first; they must fail or not compile before T043–T046.

- [ ] T039 [P] [US4] End-to-end tests in `mod tests` of xtask/src/codegen.rs, each on a `fixtures::Root` with `run`: `second_run_is_byte_identical` (US4 AS1, SC-001: second `Report` has `written` and `removed` empty, `unchanged` = file count, bytes and mtimes equal); `one_row_change_rewrites_one_file` (US4 AS2, SC-007: two models, change Goal Position 116 → 120 in one table, `written == [that model's file]`); `every_check_leaves_output_untouched` (US4 AS3, SC-006: for each entry of `fixtures::failing()`, `run` returns `Failure::Checks` with an error whose `file` and message contain the entry's `file` and `keywords`, and `Root::assert_unchanged` holds; also assert `failing()` covers checks 1 to 16); `missing_goal_position_writes_nothing` (US4 AS3 explicitly); `unmapped_tables_warn_and_succeed` (US4 AS4, FR-004: stray tables `ax_12_plus`, `ax_18f`, `xw430_t200` neither mapped nor in `not_mapped` give one warning each, text `not in dynamixel.ron (models or not_mapped), skipped`, and `run` is `Ok`; listing them in `not_mapped` removes the warnings); `rustfmt_failure_writes_nothing` (via `output::format_with` with a missing program: `Failure::Rustfmt`, sentinels unchanged)
- [ ] T040 [P] [US4] Input and identifier checks in `mod tests` of xtask/src/codegen/resolve.rs (data-model §3): check 1 "map or table missing or unparsable" → `check01_map_missing`, `check01_map_unparsable`, `check01_table_missing`, `check01_table_unparsable`, `check01_row_without_access` (contract §4.3 "Missing `access`/`area` on any row"), and `two_broken_tables_two_diagnostics` (R-3: all problems in one run, sorted by file then message); check 7 "a file both mapped and in `not_mapped`; a `not_mapped` file that does not exist" → `check07_mapped_and_not_mapped`, `check07_not_mapped_missing`; check 16 "duplicate type identifier or register constant; module identifier `tests` or a keyword" → `check16_duplicate_type` (`xl_320` and `xl320` → `Xl320`, names both tables), `check16_duplicate_register` (`LED` and `Led`), `check16_module_tests`, `check16_module_keyword` (`type.ron`); `aliases_on_one_address_allowed` (Indirect Address 1 and Indirect Address Write both at 168). Add `Failing` entries for checks 1, 7 and 16 to `failing()` in xtask/src/codegen/fixtures.rs (check 1 is implemented by T013; these tests lock it in)
- [ ] T041 [P] [US4] Doc render tests in `mod tests` of xtask/src/codegen/render.rs (US4 AS5, FR-016, research R-15): a model file starts with the `@generated` header then `// Source: control_tables/<file>.ron`; the struct doc has `/// XM430-W350 (model number 1020).`, `/// Protocol 1, all firmware. Protocol 2, all firmware.`, ``/// Source table: `control_tables/xm430_w350.ron`.`` and `/// Unverified: not yet checked against the ROBOTIS e-Manual.`; a verified fixture has `/// Verified against <url> on <date>.` instead; `also: ["AX-12+"]` gives `(model number 12, also sold as AX-12+)`; firmware variants give `Protocol 2, firmware up to v52.` and `Protocol 2, firmware v53 and later.`; each `ActuatorKind` variant doc has the name line, the protocols and the firmware range (e.g. `XM430-W350 (model number 1020). Protocol 1 and 2, all firmware.`); `models.rs` and `tests.rs` start with the `@generated` header
- [ ] T042 [US4] Output-line tests in `mod tests` of xtask/src/codegen.rs (after T039, same file): `Report::summary()` is `generated 70 files under src/ (2 written, 0 removed, 68 unchanged)`; `Failure::lines()` for `Checks` gives every `error: control_tables/<file>.ron: <message>` line sorted, then every `warning: ...` line; `Rustfmt`/`Io` give one `error: ...` line (contracts/codegen-cli.md "Output streams and exit status")

### Implementation for User Story 4

- [ ] T043 [US4] Implement check 7, check 16 (on top of today's duplicate checks, plus `names::is_reserved_module`) and the unmapped warning in xtask/src/codegen/resolve.rs: `on_disk` = stems of `control_tables/*.ron` except `dynamixel`; one warning per stem that is neither in a `p1`/`p2` list nor in `not_mapped` (data-model §3 "Warnings"); two rows on one address with different names stay allowed (research R-16)
- [ ] T044 [US4] Implement the docs of research R-15 in xtask/src/codegen/render.rs: model struct doc (name line with `also`, one `Protocol 1|2, firmware <range>.` line per protocol from `appearances` with the ranges "all firmware", "up to v<n>", "v<n> and later", "v<a> to v<b>", source table line, Unverified/Verified line); per-variant `ActuatorKind` doc; model file header `// Source: control_tables/<file>.ron`
- [ ] T045 [US4] Implement `Report::summary()` and `Failure::lines()` in xtask/src/codegen.rs, and finish xtask/src/main.rs per contracts/codegen-cli.md: only `codegen` is accepted, anything else prints `usage: cargo xtask codegen` to stderr and exits 1; success prints the summary to stdout and the warnings to stderr, exit 0; failure prints nothing to stdout, every line of `Failure::lines()` to stderr, exit 1. `main.rs` keeps only argument parsing, printing and the exit code (research R-13)
- [ ] T046 [US4] Regenerate; run `cargo test -p xtask` and `cargo xtask codegen` twice (second run reports 0 written, 0 removed); commit `xtask/**` with the regenerated `src/models*`

**Checkpoint**: Bad input never reaches `src/models*`, and every generated file reviews as a
documented source diff (US4 done).

---

## Phase 7: User Story 5 - Reach a concrete model for model-specific work (Priority: P3, PR X1)

**Goal**: Model types are public, documented `match` targets; `ActuatorKind` is
`#[non_exhaustive]`; constructors, `DATA` and register constants stay crate-internal (FR-015).

**Independent Test**: `cargo test --lib models::tests` (match to the concrete type) and
`cargo test --doc` (the `compile_fail,E0004` example on `ActuatorKind`).

### Tests for User Story 5 ⚠️

> Write first; they must fail before T049–T050.

- [ ] T047 [P] [US5] Visibility render tests in `mod tests` of xtask/src/codegen/render.rs: `ActuatorKind` is `#[non_exhaustive] pub enum` and its doc holds a ```` ```compile_fail,E0004 ```` example `fn f(kind: dxl_rs_v2::ActuatorKind) { match kind {} }` (feature-independent; US5 AS3); `models.rs` has `pub use <module>::<Type>;` per model; every model file has a documented `pub struct`, and `new`, `DATA` and every register constant are `pub(crate)`: no model file contains `pub const` or `pub fn` (US5 AS1, AS2, FR-015)
- [ ] T048 [P] [US5] Concrete-match render-tests test in `mod tests` of xtask/src/codegen/render_tests.rs: one generated test per model type, under its family `cfg`, building the type through its first lookup `(protocol, number, from_fw)` and asserting `matches!(&kind, Ok(ActuatorKind::<Type>(m)) if m.id() == id && m.name() == <Type>::DATA.name && m.model_number() == <Type>::DATA.model_number)` (US5 AS1; spec US5 Independent Test with 1020 → `Xm430W350`)

### Implementation for User Story 5

- [ ] T049 [US5] Add the `compile_fail,E0004` example to the `ActuatorKind` doc in xtask/src/codegen/render.rs (contract §8 conflict 13: compile-fail checks are rustdoc examples with error codes), keeping the doc text of contracts/generated-api.md ("Any supported actuator, one variant per model type. Match on it to reach a model's own features; use the [`Actuator`] trait for what every model has in common.")
- [ ] T050 [US5] Implement the concrete-match tests of T048 in xtask/src/codegen/render_tests.rs
- [ ] T051 [US5] Regenerate; run `cargo test --lib models::tests` and `cargo test --doc`; then the two manual checks of quickstart.md §5 in a scratch crate under the session scratchpad (never in the repo): naming `dxl_rs_v2::models::Ax12a` with `--no-default-features --features std,x-series` fails to resolve, and `dxl_rs_v2::models::Xm430W350::GOAL_POSITION` fails with E0624. Record both results for T061; commit `xtask/**` with the regenerated `src/models*`

**Checkpoint**: All five stories work; the generated API matches contracts/generated-api.md.

---

## Phase 8: Polish & Cross-Cutting Concerns (PR X1)

- [ ] T052 Coverage (SC-008): `cargo llvm-cov -p xtask` reports ≥ 70 % lines (`main.rs` is the only untested file), and `cargo llvm-cov --workspace --ignore-filename-regex 'src/models(\.rs|/)' --fail-under-lines 70` passes; add xtask tests only where a branch of `resolve.rs`/`render.rs`/`output.rs` is uncovered
- [ ] T053 [P] Generated-code audit with `grep` over `src/models.rs` and `src/models/*.rs` (excluding `tests.rs`): no `dyn`, `unwrap`, `expect`, `panic!`, `Box`, `Vec`, `Register::new`, no `[` indexing in fn bodies, no `+`/`-` outside `const` initialisers (FR-014, contracts/generated-api.md "Visibility summary"); `src/models/tests.rs` holds no literal register address (SC-007). Fix only in `xtask/**`, regenerate
- [ ] T054 [P] Doc check: `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --no-default-features --features std,x-series` pass (FR-016, missing and broken intra-doc links in generated docs)
- [ ] T055 Run every gate of quickstart.md §1 in order, ending with `cargo xtask codegen && git add --intent-to-add src/models.rs src/models && git diff --exit-code -- src/models.rs src/models` (no diff, SC-001)
- [ ] T056 Quickstart.md §6 dry run (SC-005): copy `control_tables/xm430_w350.ron` to `control_tables/xm430_w999.ron`, add an unused model number entry for it to `control_tables/dynamixel.ron`, run `cargo xtask codegen`: exactly one new `src/models/xm430_w999.rs`, and `src/models.rs` and `src/models/tests.rs` gain its lines; `cargo test --lib models::tests` passes; zero hand-written Rust. Then revert every change with `git checkout -- control_tables src/models.rs src/models` and `git clean -f control_tables src/models`; commit nothing
- [ ] T057 Quickstart.md §7 failure demo: rename `"Goal Position"` to `"Goal Pos"` in `control_tables/xm430_w350.ron`, run `cargo xtask codegen`: exit status 1, stderr `error: control_tables/xm430_w350.ron: missing role Goal Position ...`, and `git status` shows only the `.ron` change. Revert it
- [ ] T058 Re-run quickstart.md §4 once more on the final commit (all lines, zero warnings), since US4/US5 changed generated docs and tests after T038
- [ ] T059 [human] Architect: confirm contract §8 conflict 19 (`UnsupportedModel` moves to `SetupError` and gains `id` and `protocol`) and the per-protocol lookup, then update `docs/srs/SRS.md` FR-2.3/FR-2.4 (FR-2.4 still reads `Error::UnsupportedModel { model_number, feature }`). `docs/srs/SRS.md` is in `.aiignore`: agents must not edit it. Not blocking X1's merge
- [ ] T060 [human] Architect: approve deviation D1 (generated `reg()` helper) and clarifications C1–C4 in the PR's Architecture section (plan "Deviations from module-interfaces.md"), as constitution Governance requires for any deviation
- [ ] T061 Write the PR description to `specs/006-xtask-codegen/pr-description.md` on `dev` (not staged on the PR branch) from `.github/pull_request_template.md`: **What** (X1: codegen emits `Describe`, `DATA`, `Cache`, per-protocol lookup, `firmware_register`, `MODEL_NUMBER`, the 16 checks, verification docs; `src/models*` regenerated from T1 tables); **Why** (spec 006, delivery step 3, unblocks core-2 and bus); **How** (research R-2 validate-then-write, R-5/D1 `reg()`, R-8 lookup, R-11 three test layers); **Architecture**: D1, C1–C4, the Complexity Tracking rows of plan.md, the spec items superseded by contract §8 (plan table), any root re-export fallback (T001/T020), the T031 C2 fallback if used, any extra model file from T014, the quickstart.md §5 results (T051), the ci note (family test legs, spec 007), follow-up X2 (drop `id`/`model_number`/`name` overrides after core-2), and that `xtask/Cargo.lock` was left alone; tick the template checklist. No AI attribution
- [ ] T062 Push `006-xtask-x1` and open the PR into `main` as Ovsiankina with `gh pr create --base main --head 006-xtask-x1 --body-file specs/006-xtask-codegen/pr-description.md`, staging only `xtask/**`, `src/models.rs`, `src/models/**` (and `src/actuator.rs` only if T031 applied)

---

## PR map

| PR | Tasks | Branch (off `main`) | Merged first | Gates (quickstart.md) |
|---|---|---|---|---|
| **X1** (only PR of this feature) | T001–T062 (T059, T060 human) | `006-xtask-x1` | protocol PR #7 with `ProtocolVersion` (contract §5, branch `001-protocol-packets`); core-1 (spec 004, suggested branch `004-core-1`; incl. plan C1 `crate::actuator::Describe` and C2 test removal); tables T1 (spec 005, suggested branch `005-tables-t1`). Contract §7: core-1 and T1 before X1 | §1: `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace`; `cargo clippy --no-default-features --features ax,mx,x-series,y-series,pro --target thumbv7em-none-eabihf -- -D warnings`; `cargo llvm-cov --workspace --ignore-filename-regex 'src/models(\.rs|/)' --fail-under-lines 70`; `cargo xtask codegen && git add --intent-to-add src/models.rs src/models && git diff --exit-code -- src/models.rs src/models`. Plus §2 `cargo test -p xtask`, §3 three `models::tests` lines, §4 family matrix |

Out of scope here: X2 (optional, after core-2: regenerate without the `id`/`model_number`/`name`
overrides). After X1: core-2, tables T2 (commits its own regenerated output, no xtask change),
bus-1…6.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: needs PR #7, core-1 and T1 on `main` (T001 stops otherwise).
- **Foundational (Phase 2)**: after Setup; blocks every story.
- **US1 (Phase 3)**: after Phase 2.
- **US2 (Phase 4)**: after US1 (uses `Roles` for checks 12–14; edits the same `resolve.rs`,
  `render.rs`, `render_tests.rs`).
- **US3 (Phase 5)**: after US2 (gates the lookup arms US2 renders).
- **US4 (Phase 6)**: after US3 (its end-to-end test iterates the `Failing` entries of checks
  2–15 added by US1–US3; docs need US2's appearances).
- **US5 (Phase 7)**: after US4 (adds to the `ActuatorKind` doc US4 writes).
- **Polish (Phase 8)**: after all stories. T059 and T060 are human and do not block T061–T062.

### Within Each Story

- Test tasks first; they fail or do not compile until the story's implementation lands.
- Implementation order inside a story: `resolve.rs` → `render.rs` → `render_tests.rs` →
  regenerate and commit.
- Foundational: T004 → T006; T005 → T007; T010 → T011; T008, T009 before T012 → T013 → T014.
- US1: T015–T018 → T019 → T020 → T021 → T022 → T023.
- US2: T024–T027 → T028 → T029 → T030 → T031 → T032.
- US3: T033–T035 → T036 → T037 → T038.
- US4: T039–T041, then T042 (same file as T039) → T043 → T044 → T045 → T046.
- US5: T047–T048 → T049 → T050 → T051.

### Parallel Opportunities

- Foundational: T004 (`schema.rs`), T005 (`names.rs`), T009 (`fixtures.rs`), T010 (`output.rs`)
  in parallel.
- Each story's test tasks touch different files (`resolve.rs` + `fixtures.rs`, `render.rs`,
  `render_tests.rs`, `codegen.rs`) and run in parallel. Only one test task per story edits
  `fixtures.rs`, so there is no conflict on it.
- Polish: T053 and T054 in parallel.
- Stories cannot run in parallel with each other: they edit the same three xtask files and each
  regenerates `src/models*`.

---

## Parallel Example: User Story 1

```bash
# Tests in parallel (different files):
Task: "T015 role and check tests in xtask/src/codegen/resolve.rs (+ Failing entries in fixtures.rs)"
Task: "T016 render tests in xtask/src/codegen/render.rs"
Task: "T017 real-tables oracle test in xtask/src/codegen.rs"
Task: "T018 round-trip render-tests tests in xtask/src/codegen/render_tests.rs"
```

## Parallel Example: User Story 2

```bash
Task: "T024 map and lookup checks in xtask/src/codegen/resolve.rs (+ fixtures.rs)"
Task: "T025 lookup render tests in xtask/src/codegen/render.rs"
Task: "T026 lookup render-tests tests in xtask/src/codegen/render_tests.rs"
Task: "T027 real-tables lookup test in xtask/src/codegen.rs"
```

## Parallel Example: User Story 3

```bash
Task: "T033 check 5 test in xtask/src/codegen/resolve.rs (+ fixtures.rs)"
Task: "T034 gating render tests in xtask/src/codegen/render.rs"
Task: "T035 family-off render-tests tests in xtask/src/codegen/render_tests.rs"
```

## Parallel Example: User Story 4

```bash
Task: "T039 end-to-end tests in xtask/src/codegen.rs"
Task: "T040 input and identifier checks in xtask/src/codegen/resolve.rs (+ fixtures.rs)"
Task: "T041 doc render tests in xtask/src/codegen/render.rs"
# then T042 in xtask/src/codegen.rs (same file as T039)
```

## Parallel Example: User Story 5

```bash
Task: "T047 visibility render tests in xtask/src/codegen/render.rs"
Task: "T048 concrete-match render-tests test in xtask/src/codegen/render_tests.rs"
```

---

## Implementation Strategy

### MVP First

The MVP is **PR X1** itself, the only PR of this feature. Inside it, the first useful slice is
Phases 1–3 (Setup, Foundational, US1): after T023 the bus has every common role, limit form, fault
register, scale and the cache through `ActuatorKind`, while the old lookup still builds against
core-1's shim. Stop and validate at T014 (byte-identical output through the new pipeline) and at
T023.

### Incremental Delivery

1. Phases 1–2 → pipeline parity (no generated diff).
2. US1 → `Describe`, `DATA`, `reg()` regenerated and committed.
3. US2 → per-protocol lookup with `SetupError`; the crate no longer uses core-1's
   `Error::UnsupportedModel` shim.
4. US3 → family matrix clean.
5. US4 → full checks, docs, CLI output.
6. US5 → visibility and compile-fail proof.
7. Polish → gates, PR.

Each checkpoint is a local commit on `006-xtask-x1` that passes `cargo test -p xtask`,
`cargo test --lib models::tests` and clippy. The PR is opened once, after T061.

## Notes

- [P] = different files, no dependency on an unfinished task.
- Generated files change only through `cargo xtask codegen`; a hand edit fails the CI diff check.
- If a prerequisite turns out to differ from contract §2 (a name, a path, a field), stop and raise
  it with core; do not adapt core files from X1.
