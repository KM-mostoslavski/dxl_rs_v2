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
    /// The file is not RON of the expected shape.
    Parse(Box<ron::error::SpannedError>),
    /// The directory of the tables could not be listed.
    ListDir {
        /// The directory.
        path: PathBuf,
        /// The error listing it.
        source: std::io::Error,
    },
    /// The file failed a check.
    Check(Check),
}

impl Cause {
    /// What to check, when the facts point at a usual cause.
    pub fn hint(&self) -> Option<&'static str> {
        match self {
            Self::Read { .. } | Self::Parse(_) | Self::ListDir { .. } => None,
            Self::Check(check) => check.hint(),
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
            Self::Parse(error) => write!(f, "{error}"),
            Self::ListDir { path, source } => {
                write!(f, "cannot list {}: {source}", path.display())
            }
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
    },
    /// Check 2: a list's firmware versions do not strictly increase.
    FromFwOrder {
        /// The model number.
        model: u16,
        /// The list.
        protocol: ProtocolTag,
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
    /// Check 4: a `default` beside one list.
    DefaultWithOneList {
        /// The model number.
        model: u16,
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
    BothLimitForms,
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
    /// Check 12: Model Number's access and area differ from the other
    /// tables'.
    ModelNumberAttrs {
        /// This table's access and area.
        found: (Access, Area),
        /// The other tables'.
        others: (Access, Area),
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
    PerModeWithoutUnitSource,
    /// Check 18: `torque_control_mode` without `unit_source`.
    SwitchWithoutUnitSource,
    /// Check 18: a row range without `unit_source`.
    RangeWithoutUnitSource {
        /// The row.
        register: String,
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
    PerModeIncomplete,
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

const OPERATING_MODE: &str = "Operating Mode";
const CONTROL_MODE: &str = "Control Mode";

impl fmt::Display for Check {
    #[allow(
        clippy::too_many_lines,
        reason = "one arm per check, each a single message"
    )]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FirstFromFw { model, protocol } => write!(
                f,
                "model {model}: {} from_fw must start at 0",
                protocol.list()
            ),
            Self::FromFwOrder { model, protocol } => write!(
                f,
                "model {model}: {} from_fw must strictly increase",
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
            Self::DefaultWithOneList { model } => write!(
                f,
                "model {model}: default is only allowed with both p1 and p2"
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
            Self::BothLimitForms => f.write_str(
                "both limit forms: Min/Max Position Limit and CW/CCW Angle Limit rows",
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
            Self::ModelNumberAttrs { found, others } => write!(
                f,
                "Model Number is {}/{} here but {}/{} in the other tables",
                found.0, found.1, others.0, others.1
            ),
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
            Self::PerModeWithoutUnitSource => {
                f.write_str("per-mode operating_modes lists without unit_source")
            }
            Self::SwitchWithoutUnitSource => {
                f.write_str("torque_control_mode without unit_source")
            }
            Self::RangeWithoutUnitSource { register } => {
                write!(f, "row {register:?} has a range without unit_source")
            }
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
            Self::PerModeIncomplete => f.write_str(
                "operating_modes: current, velocity, pwm and current_position come all together or not at all",
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
            Self::SwitchRegisterAttrs { register } => write!(
                f,
                "torque_control_mode: register {register:?} must be 1 byte RW/RAM"
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
            Self::SwitchGoalAttrs { goal } => {
                write!(f, "torque_control_mode: goal {goal:?} must be RW/RAM")
            }
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
