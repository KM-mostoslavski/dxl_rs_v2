// C3/C4 for xtask: written by /speckit-ov-c4-code

!element dxl.xtask {
    xtask_cli = component "xtask: CLI and driver" "cargo xtask codegen entry point and the validate-then-write pipeline: parse, check and render everything in memory, format, then write only changed files. Reports Diagnostic lines, a Report summary or a Failure, and sets the exit code." "Rust binary (xtask/src/main.rs, xtask/src/codegen.rs)"
    xtask_schema = component "xtask: Input schema" "serde types for dynamixel.ron (model number to per-protocol table lists, not_mapped) and for each control table (source, protocols, type_info, operating_modes, register rows with access and area). Unknown fields are ignored." "Rust module, serde + ron"
    xtask_resolve = component "xtask: Resolver and checks" "Joins the map and the tables into GenModel and LookupList values and runs every data-model check (lists, from_fw order, default P2, families, roles, attributes, scale, mode source, Model Number and Firmware Version placement, P1 addresses, duplicate names). Collects all problems; unmapped tables become warnings. Builds Rust identifiers from file stems and register names." "Rust module"
    xtask_render = component "xtask: Renderer" "Renders unformatted Rust text: src/models.rs (module root, ActuatorKind, lookup, Describe/Actuator impls), one src/models/<module>.rs per model, and src/models/tests.rs. Every model item is gated by its family feature." "Rust module"
    xtask_output = component "xtask: Output sync" "Formats each file with rustfmt on stdin (edition 2024), writes only files whose bytes changed, removes stale .rs files from src/models/, touches nothing else." "Rust module, rustfmt subprocess"

    xtask_cli -> xtask_schema "Parses dynamixel.ron and every referenced table with"
    xtask_cli -> xtask_resolve "Resolves and checks the input with"
    xtask_cli -> xtask_render "Renders the models files with" "only when no check failed"
    xtask_cli -> xtask_output "Formats and writes the rendered files with"
    xtask_resolve -> xtask_schema "Reads ModelMap and ControlTable values from"
    xtask_render -> xtask_resolve "Reads the Resolved model list and lookups from"

    group "xtask: CLI and driver" {
        xtask_main = component "main" "Accepts exactly the argument codegen, runs codegen::run on the workspace root, prints the summary to stdout and warning:/error: lines to stderr, returns ExitCode." "Rust fn" {
            tags "Code"
        }
        xtask_run = component "codegen::run" "Regenerates src/models.rs and src/models/ under the root from root/control_tables; writes nothing unless every earlier step succeeded (run_with takes the formatter for tests)." "Rust fn" {
            tags "Code"
        }
        xtask_generate = component "codegen::generate" "Reads dynamixel.ron (a broken map stops at once), every referenced table (one diagnostic per bad table), lists the .ron stems on disk, resolves, and renders. Returns Generated { files, warnings }." "Rust fn" {
            tags "Code"
        }
        xtask_diagnostic = component "codegen::Diagnostic" "One problem, naming control_tables/<file>.ron and a message; sorted by file then message." "Rust struct" {
            tags "Code"
        }
        xtask_report = component "codegen::Report" "Successful run: files written, stale files removed, unchanged count, unmapped-table warnings; summary() is the one stdout line." "Rust struct" {
            tags "Code"
        }
        xtask_failure = component "codegen::Failure" "Why a run wrote nothing: Checks { errors, warnings }, Rustfmt(String) or Io(String); lines() gives the stderr lines." "Rust enum" {
            tags "Code"
        }
    }

    group "xtask: Input schema" {
        xtask_model_map = component "schema::ModelMap" "dynamixel.ron: models keyed by model number (ModelEntry: family, name, also, p1/p2 lists of TableRef { from_fw, file }, default) and not_mapped records." "Rust struct, serde Deserialize" {
            tags "Code"
        }
        xtask_not_mapped = component "schema::NotMapped" "A table on disk deliberately left out of the map, with NotMappedReason NoModelNumber or SameNumberAs(n)." "Rust struct, serde Deserialize" {
            tags "Code"
        }
        xtask_control_table = component "schema::ControlTable" "One control table: Source (Unverified or Verified { url, verified }), protocols, TypeInfo (position scale), optional OperatingModes, and Row list (address, size, data_name, Access, Area)." "Rust struct, serde Deserialize" {
            tags "Code"
        }
        xtask_parse = component "schema::parse_map / schema::parse_table" "Read a file and deserialize it with ron (implicit_some), after braces_to_parens turns struct braces into RON parens outside strings." "Rust fns" {
            tags "Code"
        }
    }

    group "xtask: Resolver and checks" {
        xtask_resolve_fn = component "resolve::resolve" "Maps every table to its model number and protocol lists, runs every check, and returns an Outcome { resolved, errors, warnings }." "Rust fn" {
            tags "Code"
        }
        xtask_resolved = component "resolve::Resolved" "What the renderers need: models ordered by model number then from_fw, per-protocol lookups, and the Model Number access/area shared by every table." "Rust struct" {
            tags "Code"
        }
        xtask_gen_model = component "resolve::GenModel" "One generated model type: file, type and module idents, number, name, family, appearances (protocol, from_fw, until_fw), protocols, source, scale, registers, roles." "Rust struct" {
            tags "Code"
        }
        xtask_roles = component "resolve::Roles" "The rows the common operations use, matched by exact e-Manual name: Model Number, Firmware Version, Torque Enable, Goal/Present Position, Limits (MinMax or CwCcw), optional Hardware Error Status, ModeRole (Register with OperatingModes, or AngleLimits)." "Rust struct + enums" {
            tags "Code"
        }
        xtask_lookup_list = component "resolve::LookupList" "The tables of one model number on one protocol by firmware; firmware_dependent() when Setup must read Firmware Version first." "Rust struct" {
            tags "Code"
        }
        xtask_families = component "resolve::FAMILIES" "The accepted family names, each a Cargo feature: ax, mx, rx, x-series, y-series, pro." "Rust const" {
            tags "Code"
        }
        xtask_names = component "names::type_ident / module_ident / const_ident" "Identifiers from file stems and register names (xm430_w350 to Xm430W350, leading digit spelled out, Goal Position to GOAL_POSITION); is_reserved_module rejects keyword module names." "Rust fns" {
            tags "Code"
        }
    }

    group "xtask: Renderer" {
        xtask_render_fn = component "render::render" "Every generated file as (path relative to src/, text): one model file per GenModel, models.rs, and tests.rs when there is a model." "Rust fn" {
            tags "Code"
        }
        xtask_render_model = component "render::render_model" "One model file: struct with Cache, new(id), DATA: ModelData (name, number, protocols, role registers, limits, mode source, PositionScale), one Register const per row, Describe and Actuator impls, docs with source and verification state." "Rust fn" {
            tags "Code"
        }
        xtask_render_module = component "render::render_module / render::render_lookup" "models.rs: feature-gated mod and pub use per model, reg(), MODEL_NUMBER, #[non_exhaustive] ActuatorKind, ActuatorKind::new and firmware_register lookup arms, Describe by match, Actuator impl." "Rust fns" {
            tags "Code"
        }
        xtask_render_tests = component "render_tests::render_tests" "src/models/tests.rs: crate tests of the lookup and model data for every model number, naming registers only by constant." "Rust fn" {
            tags "Code"
        }
    }

    group "xtask: Output sync" {
        xtask_format = component "output::format" "Pipes one file through rustfmt --edition 2024 --emit stdout; a missing rustfmt or a rustfmt error is Failure::Rustfmt naming the file." "Rust fn" {
            tags "Code"
        }
        xtask_sync = component "output::sync" "Creates src/models/, writes only changed files, removes .rs files in src/models/ that were not generated, returns the Report." "Rust fn" {
            tags "Code"
        }
    }

    xtask_main -> xtask_run "Calls with the workspace root"
    xtask_main -> xtask_report "Prints summary and warnings of"
    xtask_main -> xtask_failure "Prints error lines of"
    xtask_run -> xtask_generate "Calls on root/control_tables"
    xtask_run -> xtask_format "Formats every generated file with"
    xtask_run -> xtask_sync "Writes the formatted files with"
    xtask_generate -> xtask_parse "Parses the map and referenced tables with"
    xtask_generate -> xtask_resolve_fn "Resolves and checks with"
    xtask_generate -> xtask_render_fn "Renders with" "only on zero errors"
    xtask_generate -> xtask_failure "Fails with Checks { errors, warnings } as"
    xtask_parse -> xtask_model_map "Produces"
    xtask_parse -> xtask_control_table "Produces"
    xtask_parse -> xtask_diagnostic "Reports unreadable or unparsable files as"
    xtask_model_map -> xtask_not_mapped "Lists"
    xtask_resolve_fn -> xtask_model_map "Reads"
    xtask_resolve_fn -> xtask_control_table "Reads"
    xtask_resolve_fn -> xtask_resolved "Builds"
    xtask_resolve_fn -> xtask_diagnostic "Collects errors and warnings as"
    xtask_resolve_fn -> xtask_names "Names types, modules and consts with"
    xtask_resolve_fn -> xtask_families "Checks each entry's family against"
    xtask_resolved -> xtask_gen_model "Holds"
    xtask_resolved -> xtask_lookup_list "Holds"
    xtask_gen_model -> xtask_roles "Holds"
    xtask_render_fn -> xtask_render_model "Calls per GenModel"
    xtask_render_fn -> xtask_render_module "Calls once"
    xtask_render_fn -> xtask_render_tests "Calls once"
    xtask_render_fn -> xtask_resolved "Reads"
    xtask_render_module -> xtask_lookup_list "Emits lookup arms from"
    xtask_render_model -> xtask_gen_model "Reads"
    xtask_sync -> xtask_report "Returns"
    xtask_format -> xtask_failure "Fails with Rustfmt as"
}

!element dxl.lib {
    xtask_gen_module = component "xtask: Generated models module" "src/models.rs, generated and committed: declares and re-exports each model behind its family feature, ActuatorKind with its protocol + model number + firmware lookup, MODEL_NUMBER register, and the Describe / Actuator impls by match." "Rust, generated by cargo xtask codegen"
    xtask_gen_models = component "xtask: Generated model types" "src/models/<module>.rs, one per mapped control table (73 today), each behind its family feature (ax, mx, rx, x-series, y-series, pro), plus the generated src/models/tests.rs." "Rust, generated by cargo xtask codegen"

    xtask_gen_module -> xtask_gen_models "Declares, re-exports and wraps one variant per"

    group "xtask: Generated models module" {
        xtask_models_mod = component "models" "Module root: #![allow(dead_code, ...)], #[cfg(feature = family)] mod + pub use per model, private const fn reg(addr, size, access, area) used by every model file, and pub(crate) MODEL_NUMBER (address 0, size 2) read during P1 Setup." "Rust module" {
            tags "Code"
        }
        xtask_actuator_kind = component "models::ActuatorKind" "Public, #[non_exhaustive] closed enum, one variant per model type, each gated by its family feature; derives Debug, Clone, PartialEq, Eq; re-exported at the crate root. Implements Describe (data, cache, cache_mut) and Actuator by match." "Rust enum" {
            tags "Code"
        }
        xtask_kind_new = component "models::ActuatorKind::new" "pub(crate): picks the variant for (ProtocolVersion, ModelNumber) using the table whose from_fw is the highest not above firmware; otherwise SetupError::UnsupportedModel { id, model_number, protocol, feature }, with feature naming the disabled family." "Rust fn" {
            tags "Code"
        }
        xtask_kind_fw = component "models::ActuatorKind::firmware_register" "pub(crate): Some(Firmware Version register) when the model's table depends on firmware on that protocol (e.g. XL330 / XC330 before and after fw 52), None otherwise or when the family is off." "Rust fn" {
            tags "Code"
        }
    }

    group "xtask: Generated model types" {
        xtask_model_pattern = component "models::<module>::<Model>" "Pattern of every generated model: pub struct holding a Cache, pub(crate) new(id), pub(crate) DATA: ModelData (name, model number, Protocols, torque/goal/present-position registers, LimitRegisters MinMax or CwCcw, optional hardware error register, ModeSource, PositionScale), one pub(crate) Register const per table row, Describe + Actuator impls; doc states the source table and Verified (URL, date) or Unverified." "Rust struct, generated" {
            tags "Code"
        }
        xtask_model_ax12a = component "models::ax_12a::Ax12a" "Example: AX-12A, model number 12, Protocol 1 only, CW/CCW Angle Limits as limits and mode source, no Hardware Error Status, verified against the e-Manual." "Rust struct, generated" {
            tags "Code"
        }
        xtask_model_xm430 = component "models::xm430_w350::Xm430W350" "Example: XM430-W350 on Protocol 2, Min/Max Position Limit and Operating Mode register with its mode values, Hardware Error Status present." "Rust struct, generated" {
            tags "Code"
        }
        xtask_models_tests = component "models::tests" "src/models/tests.rs: generated crate tests of ActuatorKind::new, firmware_register and model data for every model number, naming registers only by constant." "Rust test module, generated" {
            tags "Code"
        }
    }

    xtask_models_mod -> xtask_actuator_kind "Defines"
    xtask_models_mod -> xtask_model_pattern "Declares and re-exports each"
    xtask_actuator_kind -> xtask_model_pattern "Wraps one variant per"
    xtask_kind_new -> xtask_actuator_kind "Constructs a variant of"
    xtask_kind_fw -> xtask_kind_new "Tells Setup which register to read before"
    xtask_model_ax12a -> xtask_model_pattern "Follows"
    xtask_model_xm430 -> xtask_model_pattern "Follows"
    xtask_models_tests -> xtask_kind_new "Tests"
    xtask_models_tests -> xtask_model_pattern "Tests the DATA of"

    xtask_gen_module -> dxl.lib.kind "Generates ActuatorKind and its impls for (the Actuator trait itself is hand-written in crate::actuator)"
    xtask_gen_models -> dxl.lib.models "Is the generated body of"
    xtask_gen_models -> dxl.lib.kind "Implements the sealed Describe and the Actuator trait of"
    xtask_gen_models -> dxl.lib.units "Builds PositionScale from type_info with"
    xtask_kind_new -> dxl.lib.errors "Fails with SetupError::UnsupportedModel"
    dxl.lib.scan -> xtask_kind_new "Instantiates actuators by protocol, model number and firmware with"
    dxl.lib.scan -> xtask_kind_fw "Asks which Firmware Version register to read with"
}

dxl.xtask.xtask_schema -> dxl.tables "Reads control_tables/*.ron and dynamixel.ron" "serde + ron"
dxl.xtask.xtask_output -> dxl.lib.xtask_gen_module "Writes src/models.rs" "rustfmt-formatted .rs, committed"
dxl.xtask.xtask_output -> dxl.lib.xtask_gen_models "Writes src/models/*.rs and removes stale ones" "rustfmt-formatted .rs, committed"
dxl.xtask.xtask_render -> dxl.lib.xtask_gen_module "Renders the text of"
dxl.xtask.xtask_render -> dxl.lib.xtask_gen_models "Renders the text of"
dxl.xtask.xtask_parse -> dxl.tables "Deserializes" "ron 0.12"
dxl.xtask.xtask_render_module -> dxl.lib.xtask_actuator_kind "Generates"
dxl.xtask.xtask_render_module -> dxl.lib.xtask_kind_new "Generates"
dxl.xtask.xtask_render_module -> dxl.lib.xtask_kind_fw "Generates"
dxl.xtask.xtask_render_model -> dxl.lib.xtask_model_pattern "Generates one per mapped table"
dxl.xtask.xtask_render_tests -> dxl.lib.xtask_models_tests "Generates"
dxl.xtask.xtask_sync -> dxl.lib.xtask_models_mod "Writes"
maintainer -> dxl.xtask.xtask_cli "Runs" "cargo xtask codegen"
ci -> dxl.xtask.xtask_cli "Reruns codegen and fails on git diff of src/models.rs and src/models" "GitHub Actions"
