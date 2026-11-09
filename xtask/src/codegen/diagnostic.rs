//! What a run reports about its input (FR-6.14): one [`Diagnostic`] per
//! problem, naming the file and holding the cause with the values that
//! decided it. `Display` states what was observed; advice is in `hint`.

use std::fmt;
use std::path::PathBuf;

use super::fr9::{Quantity, full_magnitude};
use super::resolve::FAMILIES;
use super::schema::{Access, Area, ProtocolTag};

// DECISION: a struct holding the file beside a cause enum, not a file field
// in each of ~60 variants: every diagnostic names its file the same way.
/// One problem found in the input, naming the file it is in.
#[derive(Debug)]
pub struct Diagnostic {
    /// The table's file stem, or `dynamixel` for the map.
    pub file: String,
    /// What is wrong.
    pub cause: Cause,
}

impl Diagnostic {
    /// A diagnostic for `control_tables/<file>.ron`.
    pub(crate) fn new(file: impl Into<String>, cause: impl Into<Cause>) -> Self {
        Self {
            file: file.into(),
            cause: cause.into(),
        }
    }

    /// What to check, when the facts point at a usual cause.
    pub fn hint(&self) -> Option<&'static str> {
        self.cause.hint()
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "control_tables/{}.ron: {}", self.file, self.cause)
    }
}

/// Sorts `diagnostics` by file, then message, and drops repeats.
pub(crate) fn sort(diagnostics: &mut Vec<Diagnostic>) {
    diagnostics.sort_by_cached_key(|d| (d.file.clone(), d.cause.to_string()));
    diagnostics.dedup_by(|a, b| a.file == b.file && a.cause.to_string() == b.cause.to_string());
}

/// Why a file is reported.
#[derive(Debug)]
pub enum Cause {
    /// The file could not be read.
    Read {
        /// The file.
        path: PathBuf,
        /// The error reading it.
        source: std::io::Error,
    },
    /// The file is not RON of the expected shape. The position is exact
    /// in the file, since rewriting braces keeps every character in place.
    Parse {
        /// The line, from 1.
        line: usize,
        /// The column, from 1.
        column: usize,
        /// The parser's own error, boxed to keep diagnostics small.
        code: Box<ron::Error>,
    },
    /// A `.ron` file whose name is not UTF-8, so no map can name it.
    NonUtf8Name {
        /// The file.
        path: PathBuf,
    },
    /// The file failed a check.
    Check(Check),
}

impl Cause {
    /// What to check, when the facts point at a usual cause.
    pub fn hint(&self) -> Option<&'static str> {
        match self {
            Self::Read { .. } | Self::Parse { .. } | Self::NonUtf8Name { .. } => None,
            Self::Check(check) => check.hint(),
        }
    }
}

// DECISION: keep the start of ron's span only: the spec names one line and
// column, and the start is where the offending token begins.
impl From<ron::error::SpannedError> for Cause {
    fn from(error: ron::error::SpannedError) -> Self {
        Self::Parse {
            line: error.span.start.line,
            column: error.span.start.col,
            code: Box::new(error.code),
        }
    }
}

impl From<Check> for Cause {
    fn from(check: Check) -> Self {
        Self::Check(check)
    }
}

impl fmt::Display for Cause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => write!(f, "cannot read {}: {source}", path.display()),
            Self::Parse { line, column, code } => write!(f, "{line}:{column}: {code}"),
            Self::NonUtf8Name { path } => write!(f, "{}: file name is not UTF-8", path.display()),
            Self::Check(check) => write!(f, "{check}"),
        }
    }
}

/// One failed check, with the model, table, register or field involved and
/// the values that decided it. Checks are numbered as in data-model §3.
#[derive(Debug, Clone, PartialEq)]
pub enum Check {
    /// Check 2: a list's first table is not at firmware 0.
    FirstFromFw {
        /// The model number.
        model: u16,
        /// The list.
        protocol: ProtocolTag,
        /// The first table.
        table: String,
        /// Its `from_fw`.
        from_fw: u8,
    },
    /// Check 2: two neighbours of a list whose firmware versions do not
    /// strictly increase.
    FromFwOrder {
        /// The model number.
        model: u16,
        /// The list.
        protocol: ProtocolTag,
        /// Every `from_fw` of the list, in order.
        observed: Vec<u8>,
        /// The earlier table of the pair.
        first: String,
        /// Its `from_fw`.
        first_fw: u8,
        /// The later table of the pair.
        second: String,
        /// Its `from_fw`.
        second_fw: u8,
    },
    /// Check 3: a model number with neither list.
    NoLists {
        /// The model number.
        model: u16,
    },
    /// Check 3: an empty list.
    EmptyList {
        /// The model number.
        model: u16,
        /// The empty list.
        protocol: ProtocolTag,
    },
    /// Check 4: both lists and no `default`.
    DefaultMissing {
        /// The model number.
        model: u16,
    },
    /// Check 4: both lists and a `default` other than P2.
    DefaultNotP2 {
        /// The model number.
        model: u16,
        /// The `default` found.
        found: ProtocolTag,
    },
    /// Check 4: a `default` without both lists.
    DefaultWithOneList {
        /// The model number.
        model: u16,
        /// The `default` found.
        default: ProtocolTag,
        /// The one list present, if any.
        present: Option<ProtocolTag>,
    },
    /// Check 5: a family that is not a Cargo feature.
    UnknownFamily {
        /// The model number.
        model: u16,
        /// The family found.
        family: String,
    },
    /// Check 6: one table under two model numbers.
    TableUnderTwoModels {
        /// The table.
        table: String,
        /// The first model number listing it.
        first: u16,
        /// The other model number listing it.
        second: u16,
    },
    /// Check 7: a table both mapped and in `not_mapped`.
    MappedAndNotMapped {
        /// The table.
        table: String,
    },
    /// Check 7: `not_mapped` names a file that is not on disk.
    NotMappedMissing {
        /// The table.
        table: String,
    },
    /// Check 8: an empty `protocols`.
    EmptyProtocols,
    /// Check 9: listed under a protocol the table's `protocols` lacks.
    ProtocolMissing {
        /// The list naming the table.
        protocol: ProtocolTag,
    },
    /// Check 10: a common role's row is missing.
    MissingRole {
        /// The row's name.
        register: String,
    },
    /// Check 10: both position limit forms.
    BothLimitForms {
        /// The limit rows present.
        rows: Vec<String>,
    },
    /// Check 10: neither position limit form.
    MissingLimits,
    /// Check 10: one row of a limit form without the other.
    HalfLimitForm {
        /// The row present.
        register: String,
    },
    /// Check 11: a role row with the wrong access or area.
    RoleAttrs {
        /// The row's name.
        register: String,
        /// The access and area the role needs.
        expected: (Access, Area),
        /// The row's access and area.
        found: (Access, Area),
    },
    /// Check 11: a mode row that is not 1 byte RW/EEPROM.
    ModeRowAttrs {
        /// The row's name.
        register: String,
        /// The row's size.
        size: u8,
        /// The row's access.
        access: Access,
        /// The row's area.
        area: Area,
    },
    /// Check 12: Model Number not at address 0, size 2.
    ModelNumberLocation {
        /// The row's address.
        address: u16,
        /// The row's size.
        size: u8,
    },
    /// Check 12: the tables give Model Number different access and area.
    ModelNumberAttrsDiffer {
        /// Each access and area found, with the tables giving it.
        groups: Vec<((Access, Area), Vec<String>)>,
    },
    /// Check 13: Firmware Version moves between the tables of one model
    /// number and protocol.
    FirmwareVersionMoves {
        /// The model number.
        model: u16,
        /// The list.
        protocol: ProtocolTag,
        /// The first table.
        first: String,
        /// Its Firmware Version address and size.
        first_at: (u16, u8),
        /// The other table.
        other: String,
        /// Its Firmware Version address and size.
        other_at: (u16, u8),
    },
    /// Check 14: a role row above P1's 8-bit addresses.
    P1Address {
        /// The row's name.
        register: String,
        /// The row's address.
        address: u16,
    },
    /// Check 15: the raw scale is not a proper range.
    RawScaleOrder {
        /// `value_of_min_radian_position`.
        min: i32,
        /// `value_of_max_radian_position`.
        max: i32,
    },
    /// Check 15: the radian scale is not a finite proper range.
    RadianScale {
        /// `min_radian`.
        min: f32,
        /// `max_radian`.
        max: f32,
    },
    /// Check 15: the zero tick outside the raw scale.
    ZeroOutside {
        /// `value_of_zero_radian_position`.
        zero: i32,
        /// `value_of_min_radian_position`.
        min: i32,
        /// `value_of_max_radian_position`.
        max: i32,
    },
    /// Check 16: two tables give one type name.
    DuplicateType {
        /// The first table.
        first: String,
        /// The other table.
        second: String,
        /// The type name.
        ty: String,
    },
    /// Check 16: a module name that is a keyword or `tests`.
    ReservedModule {
        /// The module name.
        module: String,
    },
    /// Check 16: two rows give one constant name.
    DuplicateConstant {
        /// The first row.
        first: String,
        /// The other row.
        second: String,
        /// The constant name.
        ident: String,
    },
    /// Check 17: both an Operating Mode and a Control Mode row.
    TwoModeRegisters,
    /// Check 17: an Operating Mode row beside CW/CCW Angle Limit rows.
    OperatingModeBesideAngleLimits,
    /// Check 17: a mode row without `operating_modes`.
    ModeRowWithoutValues {
        /// The mode row.
        register: String,
    },
    /// Check 17: `operating_modes` without a mode row.
    ValuesWithoutModeRow,
    /// Check 17: neither a mode row nor angle limits.
    NoModeSource,
    /// Check 17: one mode value in two lists.
    ModeValueInTwoLists {
        /// The value.
        value: u8,
        /// The first list.
        first: &'static str,
        /// The other list.
        second: &'static str,
    },
    /// Check 18: `unit_source` is `Unverified`.
    UnitSourceUnverified,
    /// Check 18: per-mode lists without `unit_source`.
    PerModeWithoutUnitSource {
        /// The per-mode lists present.
        present: Vec<&'static str>,
    },
    /// Check 18: `torque_control_mode` without `unit_source`.
    SwitchWithoutUnitSource,
    /// Check 18: a row range without `unit_source`.
    RangeWithoutUnitSource {
        /// The row.
        register: String,
        /// The range's low end.
        min: i64,
        /// The range's high end.
        max: i64,
    },
    /// Check 19: a ranged row whose size has no raw span.
    RowRangeSize {
        /// The row.
        register: String,
        /// Its size.
        size: u8,
        /// The range's low end.
        min: i64,
        /// The range's high end.
        max: i64,
    },
    /// Check 19: a row range whose low end is above its high end.
    RowRangeReversed {
        /// The row.
        register: String,
        /// The range's low end.
        min: i64,
        /// The range's high end.
        max: i64,
    },
    /// Check 19: a row range that fits neither sign in its size.
    RowRangeDoesNotFit {
        /// The row.
        register: String,
        /// The range's low end.
        min: i64,
        /// The range's high end.
        max: i64,
        /// The row's size.
        size: u8,
    },
    /// Check 19: a `unit_info` entry naming no row.
    UnitNamesNoRow {
        /// The entry's `data_name`.
        register: String,
    },
    /// Check 19: a step that is not finite and positive.
    UnitValue {
        /// The entry's `data_name`.
        register: String,
        /// The step.
        value: f64,
    },
    /// Check 19: a unit not in the known list.
    UnknownUnit {
        /// The entry's `data_name`.
        register: String,
        /// The unit.
        unit: String,
    },
    /// Check 19: a `sign_type` not in the known forms.
    UnknownSign {
        /// The entry's `data_name`.
        register: String,
        /// The `sign_type`.
        sign_type: String,
    },
    /// Check 19: a `unit_info` row whose size has no raw span.
    UnitRowSize {
        /// The entry's `data_name`.
        register: String,
        /// The row's size.
        size: u8,
        /// The `sign_type`.
        sign_type: String,
    },
    /// Check 19: a direction bit outside the register.
    SignDoesNotFit {
        /// The entry's `data_name`.
        register: String,
        /// The `sign_type`.
        sign_type: String,
        /// The row's size.
        size: u8,
    },
    /// Check 19: an entry range whose low end is above its high end.
    UnitRangeReversed {
        /// The entry's `data_name`.
        register: String,
        /// The range's low end.
        min: i64,
        /// The range's high end.
        max: i64,
    },
    /// Check 19: an entry range that does not fit its register and sign.
    UnitRangeDoesNotFit {
        /// The entry's `data_name`.
        register: String,
        /// The range's low end.
        min: i64,
        /// The range's high end.
        max: i64,
        /// The row's size.
        size: u8,
        /// The `sign_type`.
        sign_type: String,
    },
    /// Check 19: `modes` in a table with a mode register.
    ModesInModeRegisterTable {
        /// The entry's `data_name`.
        register: String,
    },
    /// Check 19: a mode name not in the known list.
    UnknownMode {
        /// The entry's `data_name`.
        register: String,
        /// The mode.
        mode: String,
    },
    /// Check 19: a profile not in the known list.
    UnknownProfile {
        /// The entry's `data_name`.
        register: String,
        /// The profile.
        profile: String,
    },
    /// Check 19: `bounded_by` naming no row.
    BoundedByNoRow {
        /// The entry's `data_name`.
        register: String,
        /// The `bounded_by` name.
        bounded_by: String,
    },
    /// Check 20: some of the four per-mode lists without the others.
    PerModeIncomplete {
        /// The lists present.
        present: Vec<&'static str>,
        /// The lists missing.
        missing: Vec<&'static str>,
    },
    /// Check 20: `current`, `velocity` and `pwm` do not split
    /// `no_position`.
    PerModeSplit {
        /// The three lists' values, sorted.
        split: Vec<u8>,
        /// `no_position`, sorted.
        no_position: Vec<u8>,
    },
    /// Check 20: a `current_position` value outside `unbounded`.
    CurrentPositionNotUnbounded {
        /// The value.
        value: u8,
    },
    /// Check 21: a switch in a table with a mode register.
    SwitchInModeRegisterTable,
    /// Check 21: the switch names no row.
    SwitchRegisterNoRow {
        /// The switch row's name.
        register: String,
    },
    /// Check 21: the switch row is not 1 byte RW/RAM.
    SwitchRegisterAttrs {
        /// The switch row's name.
        register: String,
        /// Its size.
        size: u8,
        /// Its access.
        access: Access,
        /// Its area.
        area: Area,
    },
    /// Check 21: the switch row has no range.
    SwitchRegisterNoRange {
        /// The switch row's name.
        register: String,
    },
    /// Check 21: `on` outside the switch row's range.
    SwitchOnOutsideRange {
        /// The `on` value.
        on: u8,
        /// The switch row's name.
        register: String,
        /// The range's low end.
        min: i64,
        /// The range's high end.
        max: i64,
    },
    /// Check 21: the switch row's range outside `u8`.
    SwitchRangeOutsideU8 {
        /// The switch row's name.
        register: String,
        /// The range's low end.
        min: i64,
        /// The range's high end.
        max: i64,
    },
    /// Check 21: the goal names no row.
    SwitchGoalNoRow {
        /// The goal row's name.
        goal: String,
    },
    /// Check 21: the goal row is not RW/RAM.
    SwitchGoalAttrs {
        /// The goal row's name.
        goal: String,
        /// Its access.
        access: Access,
        /// Its area.
        area: Area,
    },
    /// Check 21: the goal is the switch row.
    SwitchGoalIsRegister {
        /// The row's name.
        register: String,
    },
    /// Check 22: two rows serve one role.
    RoleTwoRows {
        /// The role's field.
        field: &'static str,
        /// The rows, in role order.
        rows: Vec<String>,
    },
    /// Check 22: a unit that is not the role's quantity.
    UnitNotQuantity {
        /// The entry's `data_name`.
        register: String,
        /// The unit.
        unit: String,
        /// The role's field.
        field: &'static str,
        /// The role's quantity.
        quantity: Quantity,
    },
    /// Check 22: two scales of one role in one mode.
    TwoScales {
        /// The entry's `data_name`.
        register: String,
        /// The role's field.
        field: &'static str,
        /// The modes both scales cover.
        modes: Vec<&'static str>,
    },
    /// Check 22: a direction-bit load step more than 5 % off 1 / full
    /// magnitude.
    LoadStep {
        /// The entry's `data_name`.
        register: String,
        /// The step.
        value: f64,
        /// The unit.
        unit: String,
        /// The direction bit.
        bit: u8,
    },
    /// A table on disk that nothing maps (a warning).
    Unmapped,
}

impl Check {
    /// What to check, when the facts point at a usual cause.
    pub fn hint(&self) -> Option<&'static str> {
        match self {
            Self::UnitSourceUnverified => Some(
                "check the FR-9 data against the e-Manual page, then record it as Verified(url, verified)",
            ),
            Self::SwitchRegisterNoRange { .. } => Some(
                "give the switch row its e-Manual range, so the values other than on are known",
            ),
            Self::Unmapped => Some(
                "list it under a model number's p1 or p2 in dynamixel.ron, or in not_mapped with a reason",
            ),
            _ => None,
        }
    }
}

impl fmt::Display for Access {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::R => "R",
            Self::RW => "RW",
        })
    }
}

impl fmt::Display for Area {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Eeprom => "EEPROM",
            Self::Ram => "RAM",
            Self::Hybrid => "Hybrid",
        })
    }
}

/// `"a", "b", "c"`.
fn quoted(names: &[String]) -> String {
    let names: Vec<String> = names.iter().map(|n| format!("{n:?}")).collect();
    names.join(", ")
}

const OPERATING_MODE: &str = "Operating Mode";
const CONTROL_MODE: &str = "Control Mode";

impl fmt::Display for Check {
    #[allow(
        clippy::too_many_lines,
        reason = "one arm per check, each a single message"
    )]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FirstFromFw {
                model,
                protocol,
                table,
                from_fw,
            } => write!(
                f,
                "model {model}: {} from_fw must start at 0, found {from_fw} (table {table})",
                protocol.list()
            ),
            Self::FromFwOrder {
                model,
                protocol,
                observed,
                first,
                first_fw,
                second,
                second_fw,
            } => write!(
                f,
                "model {model}: {} from_fw must strictly increase, found {observed:?}: {first} at {first_fw}, then {second} at {second_fw}",
                protocol.list()
            ),
            Self::NoLists { model } => write!(f, "model {model}: no p1 or p2 list"),
            Self::EmptyList { model, protocol } => {
                write!(f, "model {model}: {} list is empty", protocol.list())
            }
            Self::DefaultMissing { model } => write!(
                f,
                "model {model}: default: P2 is required with both p1 and p2"
            ),
            Self::DefaultNotP2 { model, found } => write!(
                f,
                "model {model}: default must be P2, found {}",
                found.variant()
            ),
            Self::DefaultWithOneList {
                model,
                default,
                present,
            } => write!(
                f,
                "model {model}: default is only allowed with both p1 and p2, found default {} with {}",
                default.variant(),
                present.map_or_else(|| "no list".to_string(), |p| format!("only {}", p.list()))
            ),
            Self::UnknownFamily { model, family } => write!(
                f,
                "model {model}: unknown family {family:?} (expected {})",
                FAMILIES.join(", ")
            ),
            Self::TableUnderTwoModels {
                table,
                first,
                second,
            } => write!(
                f,
                "table {table} is listed under model {first} and model {second}"
            ),
            Self::MappedAndNotMapped { table } => {
                write!(f, "table {table} is both mapped and in not_mapped")
            }
            Self::NotMappedMissing { table } => {
                write!(f, "not_mapped names {table}, which does not exist")
            }
            Self::EmptyProtocols => f.write_str("protocols is empty"),
            Self::ProtocolMissing { protocol } => write!(
                f,
                "listed under {} but its protocols lack {}",
                protocol.list(),
                protocol.variant()
            ),
            Self::MissingRole { register } => {
                write!(f, "missing role {register} (row {register:?})")
            }
            Self::BothLimitForms { rows } => write!(
                f,
                "both limit forms: Min/Max Position Limit and CW/CCW Angle Limit rows, found {}",
                quoted(rows)
            ),
            Self::MissingLimits => f.write_str(
                "missing role position limits (rows \"Min Position Limit\" and \"Max Position Limit\", or \"CW Angle Limit\" and \"CCW Angle Limit\")",
            ),
            Self::HalfLimitForm { register } => {
                write!(f, "half a limit form: {register:?} without its pair")
            }
            Self::RoleAttrs {
                register,
                expected,
                found,
            } => write!(
                f,
                "role {register} must be {}/{}, found {}/{}",
                expected.0, expected.1, found.0, found.1
            ),
            Self::ModeRowAttrs {
                register,
                size,
                access,
                area,
            } => write!(
                f,
                "role {register} must be 1 byte RW/EEPROM, found {size} byte{} {access}/{area}",
                if *size == 1 { "" } else { "s" }
            ),
            Self::ModelNumberLocation { address, size } => write!(
                f,
                "Model Number must be at address 0, size 2, found address {address}, size {size}"
            ),
            Self::ModelNumberAttrsDiffer { groups } => {
                let groups: Vec<String> = groups
                    .iter()
                    .map(|((access, area), tables)| {
                        format!("{access}/{area} in {}", tables.join(", "))
                    })
                    .collect();
                write!(
                    f,
                    "Model Number access/area differ across tables: {}",
                    groups.join("; ")
                )
            }
            Self::FirmwareVersionMoves {
                model,
                protocol,
                first,
                first_at,
                other,
                other_at,
            } => write!(
                f,
                "model {model}: {} tables {first} and {other} have Firmware Version at different locations ({}/{} and {}/{})",
                protocol.list(),
                first_at.0,
                first_at.1,
                other_at.0,
                other_at.1
            ),
            Self::P1Address { register, address } => write!(
                f,
                "listed under p1 but {register} is at address {address} (> 255)"
            ),
            Self::RawScaleOrder { min, max } => write!(
                f,
                "type_info: value_of_max_radian_position ({max}) must be above value_of_min_radian_position ({min})"
            ),
            Self::RadianScale { min, max } => write!(
                f,
                "type_info: max_radian ({max:?}) must be finite and above min_radian ({min:?})"
            ),
            Self::ZeroOutside { zero, min, max } => write!(
                f,
                "type_info: value_of_zero_radian_position ({zero}) outside {min}..={max}"
            ),
            Self::DuplicateType { first, second, ty } => {
                write!(f, "tables {first} and {second} both give the type {ty}")
            }
            Self::ReservedModule { module } => write!(
                f,
                "module name {module} is reserved (a Rust keyword, or the generated tests module)"
            ),
            Self::DuplicateConstant {
                first,
                second,
                ident,
            } => write!(
                f,
                "rows {first:?} and {second:?} both give the constant {ident}"
            ),
            Self::TwoModeRegisters => write!(
                f,
                "two mode registers: {OPERATING_MODE:?} and {CONTROL_MODE:?} rows"
            ),
            Self::OperatingModeBesideAngleLimits => write!(
                f,
                "{OPERATING_MODE:?} row beside CW/CCW Angle Limit rows (only {CONTROL_MODE:?} may sit beside them)"
            ),
            Self::ModeRowWithoutValues { register } => {
                write!(f, "{register:?} row without operating_modes")
            }
            Self::ValuesWithoutModeRow => write!(
                f,
                "operating_modes without an {OPERATING_MODE:?} or {CONTROL_MODE:?} row"
            ),
            Self::NoModeSource => write!(
                f,
                "no mode source: needs an {OPERATING_MODE:?} or {CONTROL_MODE:?} row, or CW/CCW Angle Limit rows"
            ),
            Self::ModeValueInTwoLists {
                value,
                first,
                second,
            } => write!(
                f,
                "operating_modes: value {value} in both {first} and {second}"
            ),
            Self::UnitSourceUnverified => {
                f.write_str("unit_source must be Verified(url, verified), found Unverified")
            }
            Self::PerModeWithoutUnitSource { present } => write!(
                f,
                "per-mode operating_modes lists without unit_source: {}",
                present.join(", ")
            ),
            Self::SwitchWithoutUnitSource => {
                f.write_str("torque_control_mode without unit_source")
            }
            Self::RangeWithoutUnitSource { register, min, max } => write!(
                f,
                "row {register:?} has a range ({min}..={max}) without unit_source"
            ),
            Self::RowRangeSize {
                register,
                size,
                min,
                max,
            } => write!(
                f,
                "row {register:?}: size {size} is outside 1..=7 bytes, so range {min}..={max} cannot be checked"
            ),
            Self::RowRangeReversed { register, min, max } => {
                write!(f, "row {register:?}: range {min}..={max} has min above max")
            }
            Self::RowRangeDoesNotFit {
                register,
                min,
                max,
                size,
            } => write!(
                f,
                "row {register:?}: range {min}..={max} does not fit {size} byte(s)"
            ),
            Self::UnitNamesNoRow { register } => {
                write!(f, "unit_info {register:?}: names no row")
            }
            Self::UnitValue { register, value } => write!(
                f,
                "unit_info {register:?}: value {value} must be finite and above 0"
            ),
            Self::UnknownUnit { register, unit } => write!(
                f,
                "unit_info {register:?}: unknown unit {unit:?} (expected {})",
                super::fr9::UNITS.join(", ")
            ),
            Self::UnknownSign {
                register,
                sign_type,
            } => write!(
                f,
                "unit_info {register:?}: unknown sign_type {sign_type:?} (expected unsigned, twos_complement or direction_bit_N)"
            ),
            Self::UnitRowSize {
                register,
                size,
                sign_type,
            } => write!(
                f,
                "unit_info {register:?}: row size {size} is outside 1..=7 bytes, so sign_type {sign_type} cannot be checked"
            ),
            Self::SignDoesNotFit {
                register,
                sign_type,
                size,
            } => write!(
                f,
                "unit_info {register:?}: sign_type {sign_type} does not fit a {size}-byte register"
            ),
            Self::UnitRangeReversed { register, min, max } => write!(
                f,
                "unit_info {register:?}: range {min}..={max} has min above max"
            ),
            Self::UnitRangeDoesNotFit {
                register,
                min,
                max,
                size,
                sign_type,
            } => write!(
                f,
                "unit_info {register:?}: range {min}..={max} does not fit a {size}-byte {sign_type} register"
            ),
            Self::ModesInModeRegisterTable { register } => write!(
                f,
                "unit_info {register:?}: modes is only allowed in an angle-limit table"
            ),
            Self::UnknownMode { register, mode } => write!(
                f,
                "unit_info {register:?}: unknown mode {mode:?} (expected joint, wheel, multi_turn or torque_control)"
            ),
            Self::UnknownProfile { register, profile } => write!(
                f,
                "unit_info {register:?}: unknown profile {profile:?} (expected velocity_based or time_based)"
            ),
            Self::BoundedByNoRow {
                register,
                bounded_by,
            } => write!(
                f,
                "unit_info {register:?}: bounded_by {bounded_by:?} names no row"
            ),
            Self::PerModeIncomplete { present, missing } => write!(
                f,
                "operating_modes: current, velocity, pwm and current_position come all together or not at all, found {} without {}",
                present.join(", "),
                missing.join(", ")
            ),
            Self::PerModeSplit { split, no_position } => write!(
                f,
                "operating_modes: current, velocity and pwm {split:?} must split no_position {no_position:?} exactly"
            ),
            Self::CurrentPositionNotUnbounded { value } => write!(
                f,
                "operating_modes: current_position value {value} is not in unbounded"
            ),
            Self::SwitchInModeRegisterTable => f.write_str(
                "torque_control_mode: only an angle-limit table (no mode register) may have one",
            ),
            Self::SwitchRegisterNoRow { register } => {
                write!(f, "torque_control_mode: register {register:?} names no row")
            }
            Self::SwitchRegisterAttrs {
                register,
                size,
                access,
                area,
            } => write!(
                f,
                "torque_control_mode: register {register:?} must be 1 byte RW/RAM, found {size} byte{} {access}/{area}",
                if *size == 1 { "" } else { "s" }
            ),
            Self::SwitchRegisterNoRange { register } => {
                write!(f, "torque_control_mode: register {register:?} has no range")
            }
            Self::SwitchOnOutsideRange {
                on,
                register,
                min,
                max,
            } => write!(
                f,
                "torque_control_mode: on {on} is outside {register:?}'s range {min}..={max}"
            ),
            Self::SwitchRangeOutsideU8 { register, min, max } => write!(
                f,
                "torque_control_mode: register {register:?}'s range {min}..={max} is outside u8 (0..=255)"
            ),
            Self::SwitchGoalNoRow { goal } => {
                write!(f, "torque_control_mode: goal {goal:?} names no row")
            }
            Self::SwitchGoalAttrs { goal, access, area } => write!(
                f,
                "torque_control_mode: goal {goal:?} must be RW/RAM, found {access}/{area}"
            ),
            Self::SwitchGoalIsRegister { register } => write!(
                f,
                "torque_control_mode: goal and register are the same row {register:?}"
            ),
            Self::RoleTwoRows { field, rows } => {
                let names: Vec<String> = rows.iter().map(|r| format!("{r:?}")).collect();
                write!(f, "role {field} has two rows: {}", names.join(" and "))
            }
            Self::UnitNotQuantity {
                register,
                unit,
                field,
                quantity,
            } => write!(
                f,
                "unit_info {register:?}: unit {unit:?} is not the {field} role's quantity ({quantity:?})"
            ),
            Self::TwoScales {
                register,
                field,
                modes,
            } => write!(
                f,
                "unit_info {register:?}: two {field} scales in mode {}",
                modes.join(", ")
            ),
            Self::LoadStep {
                register,
                value,
                unit,
                bit,
            } => write!(
                f,
                "unit_info {register:?}: step {value} {unit} is not about 100 % / {} (the magnitude below direction bit {bit})",
                full_magnitude(*bit)
            ),
            Self::Unmapped => f.write_str("not in dynamixel.ron (models or not_mapped), skipped"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ProtocolTag::{P1, P2};

    fn strings(names: &[&str]) -> Vec<String> {
        names.iter().map(ToString::to_string).collect()
    }

    /// Every check, built with distinct values, and its whole message.
    #[allow(
        clippy::too_many_lines,
        reason = "one case per check, each a value and its message"
    )]
    fn cases() -> Vec<(Check, &'static str)> {
        vec![
            (
                Check::FirstFromFw {
                    model: 1190,
                    protocol: P2,
                    table: "ta".into(),
                    from_fw: 7,
                },
                "model 1190: p2 from_fw must start at 0, found 7 (table ta)",
            ),
            (
                Check::FromFwOrder {
                    model: 1191,
                    protocol: P1,
                    observed: vec![0, 60, 53],
                    first: "tb".into(),
                    first_fw: 60,
                    second: "tc".into(),
                    second_fw: 53,
                },
                "model 1191: p1 from_fw must strictly increase, found [0, 60, 53]: tb at 60, then tc at 53",
            ),
            (
                Check::NoLists { model: 1192 },
                "model 1192: no p1 or p2 list",
            ),
            (
                Check::EmptyList {
                    model: 1193,
                    protocol: P2,
                },
                "model 1193: p2 list is empty",
            ),
            (
                Check::DefaultMissing { model: 1194 },
                "model 1194: default: P2 is required with both p1 and p2",
            ),
            (
                Check::DefaultNotP2 {
                    model: 1195,
                    found: P1,
                },
                "model 1195: default must be P2, found P1",
            ),
            (
                Check::DefaultWithOneList {
                    model: 1196,
                    default: P1,
                    present: Some(P2),
                },
                "model 1196: default is only allowed with both p1 and p2, found default P1 with only p2",
            ),
            (
                Check::DefaultWithOneList {
                    model: 1197,
                    default: P2,
                    present: None,
                },
                "model 1197: default is only allowed with both p1 and p2, found default P2 with no list",
            ),
            (
                Check::UnknownFamily {
                    model: 1198,
                    family: "x_series".into(),
                },
                "model 1198: unknown family \"x_series\" (expected ax, mx, rx, x-series, y-series, pro)",
            ),
            (
                Check::TableUnderTwoModels {
                    table: "td".into(),
                    first: 12,
                    second: 18,
                },
                "table td is listed under model 12 and model 18",
            ),
            (
                Check::MappedAndNotMapped { table: "te".into() },
                "table te is both mapped and in not_mapped",
            ),
            (
                Check::NotMappedMissing { table: "tf".into() },
                "not_mapped names tf, which does not exist",
            ),
            (Check::EmptyProtocols, "protocols is empty"),
            (
                Check::ProtocolMissing { protocol: P2 },
                "listed under p2 but its protocols lack P2",
            ),
            (
                Check::MissingRole {
                    register: "Torque Enable".into(),
                },
                "missing role Torque Enable (row \"Torque Enable\")",
            ),
            (
                Check::BothLimitForms {
                    rows: strings(&["Min Position Limit", "CCW Angle Limit"]),
                },
                "both limit forms: Min/Max Position Limit and CW/CCW Angle Limit rows, found \"Min Position Limit\", \"CCW Angle Limit\"",
            ),
            (
                Check::MissingLimits,
                "missing role position limits (rows \"Min Position Limit\" and \"Max Position Limit\", or \"CW Angle Limit\" and \"CCW Angle Limit\")",
            ),
            (
                Check::HalfLimitForm {
                    register: "CW Angle Limit".into(),
                },
                "half a limit form: \"CW Angle Limit\" without its pair",
            ),
            (
                Check::RoleAttrs {
                    register: "Goal Position".into(),
                    expected: (Access::RW, Area::Ram),
                    found: (Access::R, Area::Hybrid),
                },
                "role Goal Position must be RW/RAM, found R/Hybrid",
            ),
            (
                Check::ModeRowAttrs {
                    register: "Control Mode".into(),
                    size: 2,
                    access: Access::R,
                    area: Area::Ram,
                },
                "role Control Mode must be 1 byte RW/EEPROM, found 2 bytes R/RAM",
            ),
            (
                Check::ModeRowAttrs {
                    register: "Operating Mode".into(),
                    size: 1,
                    access: Access::RW,
                    area: Area::Ram,
                },
                "role Operating Mode must be 1 byte RW/EEPROM, found 1 byte RW/RAM",
            ),
            (
                Check::ModelNumberLocation {
                    address: 3,
                    size: 4,
                },
                "Model Number must be at address 0, size 2, found address 3, size 4",
            ),
            (
                Check::ModelNumberAttrsDiffer {
                    groups: vec![
                        ((Access::R, Area::Eeprom), strings(&["ta", "tb"])),
                        ((Access::RW, Area::Hybrid), strings(&["tc"])),
                    ],
                },
                "Model Number access/area differ across tables: R/EEPROM in ta, tb; RW/Hybrid in tc",
            ),
            (
                Check::FirmwareVersionMoves {
                    model: 1190,
                    protocol: P2,
                    first: "old".into(),
                    first_at: (6, 1),
                    other: "new".into(),
                    other_at: (2, 4),
                },
                "model 1190: p2 tables old and new have Firmware Version at different locations (6/1 and 2/4)",
            ),
            (
                Check::P1Address {
                    register: "Goal Position".into(),
                    address: 564,
                },
                "listed under p1 but Goal Position is at address 564 (> 255)",
            ),
            (
                Check::RawScaleOrder { min: 4095, max: 10 },
                "type_info: value_of_max_radian_position (10) must be above value_of_min_radian_position (4095)",
            ),
            (
                Check::RadianScale {
                    min: 1.5,
                    max: -2.25,
                },
                "type_info: max_radian (-2.25) must be finite and above min_radian (1.5)",
            ),
            (
                Check::ZeroOutside {
                    zero: 5000,
                    min: 1,
                    max: 4095,
                },
                "type_info: value_of_zero_radian_position (5000) outside 1..=4095",
            ),
            (
                Check::DuplicateType {
                    first: "xl320".into(),
                    second: "xl_320".into(),
                    ty: "Xl320".into(),
                },
                "tables xl320 and xl_320 both give the type Xl320",
            ),
            (
                Check::ReservedModule {
                    module: "type".into(),
                },
                "module name type is reserved (a Rust keyword, or the generated tests module)",
            ),
            (
                Check::DuplicateConstant {
                    first: "LED".into(),
                    second: "Led".into(),
                    ident: "LED".into(),
                },
                "rows \"LED\" and \"Led\" both give the constant LED",
            ),
            (
                Check::TwoModeRegisters,
                "two mode registers: \"Operating Mode\" and \"Control Mode\" rows",
            ),
            (
                Check::OperatingModeBesideAngleLimits,
                "\"Operating Mode\" row beside CW/CCW Angle Limit rows (only \"Control Mode\" may sit beside them)",
            ),
            (
                Check::ModeRowWithoutValues {
                    register: "Control Mode".into(),
                },
                "\"Control Mode\" row without operating_modes",
            ),
            (
                Check::ValuesWithoutModeRow,
                "operating_modes without an \"Operating Mode\" or \"Control Mode\" row",
            ),
            (
                Check::NoModeSource,
                "no mode source: needs an \"Operating Mode\" or \"Control Mode\" row, or CW/CCW Angle Limit rows",
            ),
            (
                Check::ModeValueInTwoLists {
                    value: 4,
                    first: "bounded",
                    second: "no_position",
                },
                "operating_modes: value 4 in both bounded and no_position",
            ),
            (
                Check::UnitSourceUnverified,
                "unit_source must be Verified(url, verified), found Unverified",
            ),
            (
                Check::PerModeWithoutUnitSource {
                    present: vec!["velocity", "current_position"],
                },
                "per-mode operating_modes lists without unit_source: velocity, current_position",
            ),
            (
                Check::SwitchWithoutUnitSource,
                "torque_control_mode without unit_source",
            ),
            (
                Check::RangeWithoutUnitSource {
                    register: "P Gain".into(),
                    min: 3,
                    max: 254,
                },
                "row \"P Gain\" has a range (3..=254) without unit_source",
            ),
            (
                Check::RowRangeSize {
                    register: "LED".into(),
                    size: 8,
                    min: -2,
                    max: 9,
                },
                "row \"LED\": size 8 is outside 1..=7 bytes, so range -2..=9 cannot be checked",
            ),
            (
                Check::RowRangeReversed {
                    register: "LED Red".into(),
                    min: 9,
                    max: -2,
                },
                "row \"LED Red\": range 9..=-2 has min above max",
            ),
            (
                Check::RowRangeDoesNotFit {
                    register: "LED Blue".into(),
                    min: 0,
                    max: 300,
                    size: 1,
                },
                "row \"LED Blue\": range 0..=300 does not fit 1 byte(s)",
            ),
            (
                Check::UnitNamesNoRow {
                    register: "Goal PWM".into(),
                },
                "unit_info \"Goal PWM\": names no row",
            ),
            (
                Check::UnitValue {
                    register: "LED".into(),
                    value: -0.5,
                },
                "unit_info \"LED\": value -0.5 must be finite and above 0",
            ),
            (
                Check::UnknownUnit {
                    register: "LED".into(),
                    unit: "lux".into(),
                },
                "unit_info \"LED\": unknown unit \"lux\" (expected deg, rpm, mA, V, °C, %, ms, deg/s^2)",
            ),
            (
                Check::UnknownSign {
                    register: "LED".into(),
                    sign_type: "signed".into(),
                },
                "unit_info \"LED\": unknown sign_type \"signed\" (expected unsigned, twos_complement or direction_bit_N)",
            ),
            (
                Check::UnitRowSize {
                    register: "Present Load".into(),
                    size: 0,
                    sign_type: "twos_complement".into(),
                },
                "unit_info \"Present Load\": row size 0 is outside 1..=7 bytes, so sign_type twos_complement cannot be checked",
            ),
            (
                Check::SignDoesNotFit {
                    register: "LED".into(),
                    sign_type: "direction_bit_10".into(),
                    size: 1,
                },
                "unit_info \"LED\": sign_type direction_bit_10 does not fit a 1-byte register",
            ),
            (
                Check::UnitRangeReversed {
                    register: "LED".into(),
                    min: 10,
                    max: 2,
                },
                "unit_info \"LED\": range 10..=2 has min above max",
            ),
            (
                Check::UnitRangeDoesNotFit {
                    register: "LED".into(),
                    min: 0,
                    max: 256,
                    size: 1,
                    sign_type: "unsigned".into(),
                },
                "unit_info \"LED\": range 0..=256 does not fit a 1-byte unsigned register",
            ),
            (
                Check::ModesInModeRegisterTable {
                    register: "Goal Velocity".into(),
                },
                "unit_info \"Goal Velocity\": modes is only allowed in an angle-limit table",
            ),
            (
                Check::UnknownMode {
                    register: "Moving Speed".into(),
                    mode: "spin".into(),
                },
                "unit_info \"Moving Speed\": unknown mode \"spin\" (expected joint, wheel, multi_turn or torque_control)",
            ),
            (
                Check::UnknownProfile {
                    register: "Profile Velocity".into(),
                    profile: "fast".into(),
                },
                "unit_info \"Profile Velocity\": unknown profile \"fast\" (expected velocity_based or time_based)",
            ),
            (
                Check::BoundedByNoRow {
                    register: "Goal Current".into(),
                    bounded_by: "Current Cap".into(),
                },
                "unit_info \"Goal Current\": bounded_by \"Current Cap\" names no row",
            ),
            (
                Check::PerModeIncomplete {
                    present: vec!["pwm"],
                    missing: vec!["current", "velocity", "current_position"],
                },
                "operating_modes: current, velocity, pwm and current_position come all together or not at all, found pwm without current, velocity, current_position",
            ),
            (
                Check::PerModeSplit {
                    split: vec![0, 1],
                    no_position: vec![0, 1, 16],
                },
                "operating_modes: current, velocity and pwm [0, 1] must split no_position [0, 1, 16] exactly",
            ),
            (
                Check::CurrentPositionNotUnbounded { value: 5 },
                "operating_modes: current_position value 5 is not in unbounded",
            ),
            (
                Check::SwitchInModeRegisterTable,
                "torque_control_mode: only an angle-limit table (no mode register) may have one",
            ),
            (
                Check::SwitchRegisterNoRow {
                    register: "Torque Mode".into(),
                },
                "torque_control_mode: register \"Torque Mode\" names no row",
            ),
            (
                Check::SwitchRegisterAttrs {
                    register: "Moving Speed".into(),
                    size: 2,
                    access: Access::R,
                    area: Area::Eeprom,
                },
                "torque_control_mode: register \"Moving Speed\" must be 1 byte RW/RAM, found 2 bytes R/EEPROM",
            ),
            (
                Check::SwitchRegisterNoRange {
                    register: "Torque Ctrl Mode Enable".into(),
                },
                "torque_control_mode: register \"Torque Ctrl Mode Enable\" has no range",
            ),
            (
                Check::SwitchOnOutsideRange {
                    on: 2,
                    register: "Torque Ctrl Mode Enable".into(),
                    min: 0,
                    max: 1,
                },
                "torque_control_mode: on 2 is outside \"Torque Ctrl Mode Enable\"'s range 0..=1",
            ),
            (
                Check::SwitchRangeOutsideU8 {
                    register: "Torque Ctrl Mode Enable".into(),
                    min: -1,
                    max: 3,
                },
                "torque_control_mode: register \"Torque Ctrl Mode Enable\"'s range -1..=3 is outside u8 (0..=255)",
            ),
            (
                Check::SwitchGoalNoRow {
                    goal: "Goal Thrust".into(),
                },
                "torque_control_mode: goal \"Goal Thrust\" names no row",
            ),
            (
                Check::SwitchGoalAttrs {
                    goal: "Present Position".into(),
                    access: Access::R,
                    area: Area::Ram,
                },
                "torque_control_mode: goal \"Present Position\" must be RW/RAM, found R/RAM",
            ),
            (
                Check::SwitchGoalIsRegister {
                    register: "Torque Ctrl Mode Enable".into(),
                },
                "torque_control_mode: goal and register are the same row \"Torque Ctrl Mode Enable\"",
            ),
            (
                Check::RoleTwoRows {
                    field: "present_velocity",
                    rows: strings(&["Present Velocity", "Present Speed"]),
                },
                "role present_velocity has two rows: \"Present Velocity\" and \"Present Speed\"",
            ),
            (
                Check::UnitNotQuantity {
                    register: "Present Temperature".into(),
                    unit: "V".into(),
                    field: "present_temperature",
                    quantity: Quantity::Temperature,
                },
                "unit_info \"Present Temperature\": unit \"V\" is not the present_temperature role's quantity (Temperature)",
            ),
            (
                Check::TwoScales {
                    register: "Present Load".into(),
                    field: "present_load",
                    modes: vec!["Bounded", "Wheel"],
                },
                "unit_info \"Present Load\": two present_load scales in mode Bounded, Wheel",
            ),
            (
                Check::LoadStep {
                    register: "Present Load".into(),
                    value: 0.2,
                    unit: "%".into(),
                    bit: 10,
                },
                "unit_info \"Present Load\": step 0.2 % is not about 100 % / 1023 (the magnitude below direction bit 10)",
            ),
            (
                Check::Unmapped,
                "not in dynamixel.ron (models or not_mapped), skipped",
            ),
        ]
    }

    /// The position of `check`'s variant in the enum. Exhaustive, so a new
    /// variant fails to compile until it is given a case.
    fn index(check: &Check) -> usize {
        match check {
            Check::FirstFromFw { .. } => 0,
            Check::FromFwOrder { .. } => 1,
            Check::NoLists { .. } => 2,
            Check::EmptyList { .. } => 3,
            Check::DefaultMissing { .. } => 4,
            Check::DefaultNotP2 { .. } => 5,
            Check::DefaultWithOneList { .. } => 6,
            Check::UnknownFamily { .. } => 7,
            Check::TableUnderTwoModels { .. } => 8,
            Check::MappedAndNotMapped { .. } => 9,
            Check::NotMappedMissing { .. } => 10,
            Check::EmptyProtocols => 11,
            Check::ProtocolMissing { .. } => 12,
            Check::MissingRole { .. } => 13,
            Check::BothLimitForms { .. } => 14,
            Check::MissingLimits => 15,
            Check::HalfLimitForm { .. } => 16,
            Check::RoleAttrs { .. } => 17,
            Check::ModeRowAttrs { .. } => 18,
            Check::ModelNumberLocation { .. } => 19,
            Check::ModelNumberAttrsDiffer { .. } => 20,
            Check::FirmwareVersionMoves { .. } => 21,
            Check::P1Address { .. } => 22,
            Check::RawScaleOrder { .. } => 23,
            Check::RadianScale { .. } => 24,
            Check::ZeroOutside { .. } => 25,
            Check::DuplicateType { .. } => 26,
            Check::ReservedModule { .. } => 27,
            Check::DuplicateConstant { .. } => 28,
            Check::TwoModeRegisters => 29,
            Check::OperatingModeBesideAngleLimits => 30,
            Check::ModeRowWithoutValues { .. } => 31,
            Check::ValuesWithoutModeRow => 32,
            Check::NoModeSource => 33,
            Check::ModeValueInTwoLists { .. } => 34,
            Check::UnitSourceUnverified => 35,
            Check::PerModeWithoutUnitSource { .. } => 36,
            Check::SwitchWithoutUnitSource => 37,
            Check::RangeWithoutUnitSource { .. } => 38,
            Check::RowRangeSize { .. } => 39,
            Check::RowRangeReversed { .. } => 40,
            Check::RowRangeDoesNotFit { .. } => 41,
            Check::UnitNamesNoRow { .. } => 42,
            Check::UnitValue { .. } => 43,
            Check::UnknownUnit { .. } => 44,
            Check::UnknownSign { .. } => 45,
            Check::UnitRowSize { .. } => 46,
            Check::SignDoesNotFit { .. } => 47,
            Check::UnitRangeReversed { .. } => 48,
            Check::UnitRangeDoesNotFit { .. } => 49,
            Check::ModesInModeRegisterTable { .. } => 50,
            Check::UnknownMode { .. } => 51,
            Check::UnknownProfile { .. } => 52,
            Check::BoundedByNoRow { .. } => 53,
            Check::PerModeIncomplete { .. } => 54,
            Check::PerModeSplit { .. } => 55,
            Check::CurrentPositionNotUnbounded { .. } => 56,
            Check::SwitchInModeRegisterTable => 57,
            Check::SwitchRegisterNoRow { .. } => 58,
            Check::SwitchRegisterAttrs { .. } => 59,
            Check::SwitchRegisterNoRange { .. } => 60,
            Check::SwitchOnOutsideRange { .. } => 61,
            Check::SwitchRangeOutsideU8 { .. } => 62,
            Check::SwitchGoalNoRow { .. } => 63,
            Check::SwitchGoalAttrs { .. } => 64,
            Check::SwitchGoalIsRegister { .. } => 65,
            Check::RoleTwoRows { .. } => 66,
            Check::UnitNotQuantity { .. } => 67,
            Check::TwoScales { .. } => 68,
            Check::LoadStep { .. } => 69,
            Check::Unmapped => 70,
        }
    }

    #[test]
    fn every_check_message_states_its_values() {
        let mut covered = std::collections::BTreeSet::new();
        for (check, message) in cases() {
            assert_eq!(check.to_string(), message, "{check:?}");
            covered.insert(index(&check));
        }
        assert_eq!(covered, (0..=70).collect());
    }

    #[test]
    fn hints_only_where_a_usual_cause_is_known() {
        for (check, message) in cases() {
            let hint = match check {
                Check::UnitSourceUnverified => Some(
                    "check the FR-9 data against the e-Manual page, then record it as Verified(url, verified)",
                ),
                Check::SwitchRegisterNoRange { .. } => Some(
                    "give the switch row its e-Manual range, so the values other than on are known",
                ),
                Check::Unmapped => Some(
                    "list it under a model number's p1 or p2 in dynamixel.ron, or in not_mapped with a reason",
                ),
                _ => None,
            };
            assert_eq!(check.hint(), hint, "{message}");
            let diagnostic = Diagnostic::new("t", check);
            assert_eq!(diagnostic.hint(), hint);
            assert!(!message.contains(" hint"), "{message}");
        }
    }

    #[test]
    fn foreign_causes_state_their_values() {
        let path = PathBuf::from("/tables/t.ron");
        let missing = || std::io::Error::from(std::io::ErrorKind::NotFound);
        for (cause, message) in [
            (
                Cause::Read {
                    path: path.clone(),
                    source: missing(),
                },
                format!("cannot read /tables/t.ron: {}", missing()),
            ),
            (
                Cause::Parse {
                    line: 3,
                    column: 7,
                    code: Box::new(ron::Error::Eof),
                },
                "3:7: Unexpected end of RON".to_string(),
            ),
            (
                Cause::NonUtf8Name { path: path.clone() },
                "/tables/t.ron: file name is not UTF-8".to_string(),
            ),
        ] {
            assert_eq!(cause.to_string(), message);
            assert_eq!(cause.hint(), None);
        }
        let d = Diagnostic::new("t", Check::EmptyProtocols);
        assert_eq!(d.to_string(), "control_tables/t.ron: protocols is empty");
    }
}
