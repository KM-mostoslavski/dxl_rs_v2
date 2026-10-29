//! Resolves the map and the tables into what is rendered, and runs every
//! check of data-model §3 on the way. Problems are collected, never thrown,
//! so one run reports all of them.

use std::collections::{BTreeMap, BTreeSet};

use super::Diagnostic;
use super::names::{const_ident, module_ident, type_ident};
use super::schema::{
    Access, Area, ControlTable, ModelEntry, ModelMap, OperatingModes, ProtocolTag, Row, Source,
    TableRef, TypeInfo,
};

/// The family names, each a Cargo feature (FR-005, plus `rx` since T1).
pub(crate) const FAMILIES: [&str; 6] = ["ax", "mx", "rx", "x-series", "y-series", "pro"];

impl ProtocolTag {
    /// `p1` / `p2`, as the map's list names.
    pub(crate) const fn list(self) -> &'static str {
        match self {
            Self::P1 => "p1",
            Self::P2 => "p2",
        }
    }

    /// `P1` / `P2`, as the generated `ProtocolVersion` variant.
    pub(crate) const fn variant(self) -> &'static str {
        match self {
            Self::P1 => "P1",
            Self::P2 => "P2",
        }
    }
}

/// One generated model type: one mapped table file.
#[derive(Debug)]
#[allow(dead_code, reason = "read as the stories of X1 land")]
pub(crate) struct GenModel {
    /// File stem.
    pub(crate) file: String,
    /// Type name; a leading digit is spelled out (`2xl430_w250` →
    /// `TwoXl430W250`).
    pub(crate) type_ident: String,
    /// Module name (`2xl430_w250` → `two_xl430_w250`).
    pub(crate) module_ident: String,
    /// The model number whose lists name the file.
    pub(crate) number: u16,
    /// Display name.
    pub(crate) name: String,
    /// Other product names sold under the number.
    pub(crate) also: Vec<String>,
    /// Family, the Cargo feature name.
    pub(crate) family: String,
    /// `(protocol, from_fw, until_fw)`, one per list naming the file.
    pub(crate) appearances: Vec<(ProtocolTag, u8, Option<u8>)>,
    /// The table's `protocols`.
    pub(crate) protocols: Vec<ProtocolTag>,
    /// The table's verification state.
    pub(crate) source: Source,
    /// The table's `type_info`.
    pub(crate) scale: TypeInfo,
    /// `(const_ident, row)` in table order.
    pub(crate) registers: Vec<(String, Row)>,
    /// The rows that serve the common operations.
    pub(crate) roles: Roles,
}

/// The rows that serve the common operations, matched by exact e-Manual
/// name (research R-4).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Roles {
    /// `Model Number`.
    pub(crate) model_number: Row,
    /// `Firmware Version`.
    pub(crate) firmware_version: Row,
    /// `Torque Enable`.
    pub(crate) torque_enable: Row,
    /// `Goal Position`.
    pub(crate) goal_position: Row,
    /// `Present Position`.
    pub(crate) present_position: Row,
    /// The position limits.
    pub(crate) limits: Limits,
    /// `Hardware Error Status`, if the table has one.
    pub(crate) hardware_error: Option<Row>,
    /// Where Setup finds the operating mode.
    pub(crate) mode: ModeRole,
}

/// The two position limit rows, in the form the table uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Limits {
    /// `Min Position Limit` and `Max Position Limit`.
    MinMax {
        /// `Min Position Limit`.
        min: Row,
        /// `Max Position Limit`.
        max: Row,
    },
    /// `CW Angle Limit` and `CCW Angle Limit`.
    CwCcw {
        /// `CW Angle Limit`.
        cw: Row,
        /// `CCW Angle Limit`.
        ccw: Row,
    },
}

/// Where Setup finds a model's operating mode (research R-17).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ModeRole {
    /// The `Operating Mode` or `Control Mode` row, with the table's
    /// `operating_modes`.
    Register {
        /// The mode row.
        row: Row,
        /// What each value means.
        values: OperatingModes,
    },
    /// No mode row: the mode follows from the CW/CCW Angle Limits.
    AngleLimits,
}

/// The tables of one model number on one protocol, by firmware.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LookupList {
    /// The protocol.
    pub(crate) protocol: ProtocolTag,
    /// The model number.
    pub(crate) number: u16,
    /// Family, the Cargo feature name.
    pub(crate) family: String,
    /// `(from_fw, type_ident)` in firmware order.
    pub(crate) variants: Vec<(u8, String)>,
}

impl LookupList {
    /// Whether Setup must read the firmware version to pick the table.
    pub(crate) fn firmware_dependent(&self) -> bool {
        self.variants.len() > 1
    }
}

/// Everything the renderers need.
#[derive(Debug, Default)]
pub(crate) struct Resolved {
    /// By model number, then lowest `from_fw`, then file stem.
    pub(crate) models: Vec<GenModel>,
    /// By model number, P1 before P2.
    pub(crate) lookups: Vec<LookupList>,
    /// The access and area every table gives Model Number, `None` with no
    /// model.
    pub(crate) model_number: Option<(Access, Area)>,
}

/// The resolved input and every problem found.
#[derive(Debug, Default)]
pub(crate) struct Outcome {
    /// Valid only when `errors` is empty.
    pub(crate) resolved: Resolved,
    /// Check failures.
    pub(crate) errors: Vec<Diagnostic>,
    /// Unmapped tables.
    pub(crate) warnings: Vec<Diagnostic>,
}

/// One mapped file before its table is read.
struct Mapping<'a> {
    number: u16,
    entry: &'a ModelEntry,
    appearances: Vec<(ProtocolTag, u8, Option<u8>)>,
}

/// Checks 3 and 4: the lists present, non-empty, and `default` set exactly
/// when both exist, to P2.
fn check_lists(number: u16, entry: &ModelEntry, errors: &mut Vec<Diagnostic>) {
    let mut fail = |message: String| {
        errors.push(Diagnostic::new(
            "dynamixel",
            format!("model {number}: {message}"),
        ));
    };
    if entry.p1.is_none() && entry.p2.is_none() {
        fail("no p1 or p2 list".to_string());
    }
    for (protocol, list) in [(ProtocolTag::P1, &entry.p1), (ProtocolTag::P2, &entry.p2)] {
        if list.as_ref().is_some_and(Vec::is_empty) {
            fail(format!("{} list is empty", protocol.list()));
        }
    }
    match (entry.p1.is_some() && entry.p2.is_some(), entry.default) {
        (true, None) => fail("default: P2 is required with both p1 and p2".to_string()),
        (true, Some(ProtocolTag::P1)) => fail("default must be P2, found P1".to_string()),
        (false, Some(_)) => fail("default is only allowed with both p1 and p2".to_string()),
        _ => {}
    }
}

/// Resolves `map` against the parsed `tables` (a table that failed to parse
/// is absent and already reported) and the table stems `on_disk`.
pub(crate) fn resolve(
    map: &ModelMap,
    tables: &BTreeMap<String, ControlTable>,
    on_disk: &BTreeSet<String>,
) -> Outcome {
    let mut out = Outcome::default();
    let mut mapped: BTreeMap<&str, Mapping> = BTreeMap::new();

    let mut reported: BTreeSet<(&str, u16)> = BTreeSet::new();
    for (&number, entry) in &map.models {
        check_lists(number, entry, &mut out.errors);
        if !FAMILIES.contains(&entry.family.as_str()) {
            out.errors.push(Diagnostic::new(
                "dynamixel",
                format!(
                    "model {number}: unknown family {:?} (expected {})",
                    entry.family,
                    FAMILIES.join(", ")
                ),
            ));
        }
        for (protocol, list) in [(ProtocolTag::P1, &entry.p1), (ProtocolTag::P2, &entry.p2)] {
            let Some(list) = list.as_ref().filter(|l| !l.is_empty()) else {
                continue;
            };
            check_order(number, protocol, list, &mut out.errors);
            for (i, table) in list.iter().enumerate() {
                let until = list.get(i + 1).map(|next| next.from_fw);
                let mapping = mapped.entry(&table.file).or_insert_with(|| Mapping {
                    number,
                    entry,
                    appearances: Vec::new(),
                });
                if mapping.number == number {
                    mapping.appearances.push((protocol, table.from_fw, until));
                } else if reported.insert((&table.file, number)) {
                    out.errors.push(Diagnostic::new(
                        "dynamixel",
                        format!(
                            "table {} is listed under model {} and model {number}",
                            table.file, mapping.number
                        ),
                    ));
                }
            }
            out.resolved.lookups.push(LookupList {
                protocol,
                number,
                family: entry.family.clone(),
                variants: list
                    .iter()
                    .map(|t| (t.from_fw, type_ident(&t.file)))
                    .collect(),
            });
        }
    }

    let mut types: BTreeMap<String, &str> = BTreeMap::new();
    for (&file, mapping) in &mapped {
        let ty = type_ident(file);
        if let Some(other) = types.insert(ty.clone(), file) {
            out.errors.push(Diagnostic::new(
                file,
                format!("tables {other} and {file} both give the type {ty}"),
            ));
        }
        let Some(table) = tables.get(file) else {
            continue;
        };
        if table.protocols.is_empty() {
            out.errors.push(Diagnostic::new(file, "protocols is empty"));
        }
        for &(protocol, _, _) in &mapping.appearances {
            if !table.protocols.contains(&protocol) {
                out.errors.push(Diagnostic::new(
                    file,
                    format!(
                        "listed under {} but its protocols lack {}",
                        protocol.list(),
                        protocol.variant()
                    ),
                ));
            }
        }
        let registers = registers(file, table, &mut out.errors);
        check_scale(file, &table.type_info, &mut out.errors);
        let Some(roles) = roles(file, table, &mut out.errors) else {
            continue;
        };
        out.resolved.models.push(GenModel {
            file: file.to_string(),
            type_ident: ty,
            module_ident: module_ident(file),
            number: mapping.number,
            name: mapping.entry.name.clone(),
            also: mapping.entry.also.clone(),
            family: mapping.entry.family.clone(),
            appearances: mapping.appearances.clone(),
            protocols: table.protocols.clone(),
            source: table.source.clone(),
            scale: table.type_info,
            registers,
            roles,
        });
    }
    out.resolved.models.sort_by(|a, b| {
        let first = |m: &GenModel| m.appearances.iter().map(|a| a.1).min();
        (a.number, first(a), &a.file).cmp(&(b.number, first(b), &b.file))
    });
    out.resolved.model_number = check_model_number(&out.resolved.models, &mut out.errors);
    check_firmware_version(&out.resolved, &mut out.errors);
    for model in &out.resolved.models {
        check_p1_addresses(model, &mut out.errors);
    }

    let not_mapped: BTreeSet<&str> = map.not_mapped.iter().map(|n| n.file.as_str()).collect();
    for stem in on_disk {
        if !mapped.contains_key(stem.as_str()) && !not_mapped.contains(stem.as_str()) {
            out.warnings.push(Diagnostic::new(
                stem.as_str(),
                "not in dynamixel.ron (models or not_mapped), skipped",
            ));
        }
    }
    out
}

/// Check 2: each list starts at firmware 0 and strictly increases.
fn check_order(
    number: u16,
    protocol: ProtocolTag,
    list: &[TableRef],
    errors: &mut Vec<Diagnostic>,
) {
    if list.first().is_some_and(|first| first.from_fw != 0) {
        errors.push(Diagnostic::new(
            "dynamixel",
            format!(
                "model {number}: {} from_fw must start at 0",
                protocol.list()
            ),
        ));
    }
    if list.windows(2).any(|w| w[0].from_fw >= w[1].from_fw) {
        errors.push(Diagnostic::new(
            "dynamixel",
            format!(
                "model {number}: {} from_fw must strictly increase",
                protocol.list()
            ),
        ));
    }
}

/// The table's rows with their constant names; check 16 for two rows giving
/// one constant.
fn registers(file: &str, table: &ControlTable, errors: &mut Vec<Diagnostic>) -> Vec<(String, Row)> {
    let mut seen: BTreeMap<String, &str> = BTreeMap::new();
    let mut out = Vec::with_capacity(table.control_table.len());
    for row in &table.control_table {
        let ident = const_ident(&row.data_name);
        if let Some(other) = seen.insert(ident.clone(), &row.data_name) {
            errors.push(Diagnostic::new(
                file,
                format!(
                    "rows {other:?} and {:?} both give the constant {ident}",
                    row.data_name
                ),
            ));
        }
        out.push((ident, row.clone()));
    }
    out
}

const OPERATING_MODE: &str = "Operating Mode";
const CONTROL_MODE: &str = "Control Mode";

fn access_name(access: Access) -> &'static str {
    match access {
        Access::R => "R",
        Access::RW => "RW",
    }
}

fn area_name(area: Area) -> &'static str {
    match area {
        Area::Eeprom => "EEPROM",
        Area::Ram => "RAM",
        Area::Hybrid => "Hybrid",
    }
}

/// The common-role rows of `table`, or `None` after reporting checks 10, 11
/// and 17.
fn roles(file: &str, table: &ControlTable, errors: &mut Vec<Diagnostic>) -> Option<Roles> {
    let before = errors.len();
    let find = |name: &str| table.control_table.iter().find(|r| r.data_name == name);
    let mut role = |name: &str| {
        let row = find(name);
        if row.is_none() {
            errors.push(Diagnostic::new(
                file,
                format!("missing role {name} (row {name:?})"),
            ));
        }
        row.cloned()
    };
    let model_number = role("Model Number");
    let firmware_version = role("Firmware Version");
    let torque_enable = role("Torque Enable");
    let goal_position = role("Goal Position");
    let present_position = role("Present Position");
    let limits = limits(file, &find, errors);
    let hardware_error = find("Hardware Error Status").cloned();
    let mode = mode(file, table, &find, errors);

    for (name, row, access, area) in [
        ("Torque Enable", &torque_enable, Access::RW, Area::Ram),
        ("Goal Position", &goal_position, Access::RW, Area::Ram),
        ("Present Position", &present_position, Access::R, Area::Ram),
    ] {
        if let Some(row) = row {
            check_attrs(file, name, row, access, area, errors);
        }
    }
    if let Some(Limits::MinMax { min: a, max: b } | Limits::CwCcw { cw: a, ccw: b }) = &limits {
        for row in [a, b] {
            check_attrs(file, &row.data_name, row, Access::RW, Area::Eeprom, errors);
        }
    }
    if let Some(ModeRole::Register { row, .. }) = &mode
        && (row.size != 1 || row.access != Access::RW || row.area != Area::Eeprom)
    {
        errors.push(Diagnostic::new(
            file,
            format!(
                "role {} must be 1 byte RW/EEPROM, found {} byte{} {}/{}",
                row.data_name,
                row.size,
                if row.size == 1 { "" } else { "s" },
                access_name(row.access),
                area_name(row.area)
            ),
        ));
    }
    if errors.len() > before {
        return None;
    }
    Some(Roles {
        model_number: model_number?,
        firmware_version: firmware_version?,
        torque_enable: torque_enable?,
        goal_position: goal_position?,
        present_position: present_position?,
        limits: limits?,
        hardware_error,
        mode: mode?,
    })
}

/// Check 11 for one role row.
fn check_attrs(
    file: &str,
    role: &str,
    row: &Row,
    access: Access,
    area: Area,
    errors: &mut Vec<Diagnostic>,
) {
    if row.access != access || row.area != area {
        errors.push(Diagnostic::new(
            file,
            format!(
                "role {role} must be {}/{}, found {}/{}",
                access_name(access),
                area_name(area),
                access_name(row.access),
                area_name(row.area)
            ),
        ));
    }
}

/// The limit form; check 10 for a missing, half or doubled form.
fn limits<'a>(
    file: &str,
    find: &impl Fn(&str) -> Option<&'a Row>,
    errors: &mut Vec<Diagnostic>,
) -> Option<Limits> {
    let pair = |a: &str, b: &str| (find(a), find(b));
    let min_max = pair("Min Position Limit", "Max Position Limit");
    let cw_ccw = pair("CW Angle Limit", "CCW Angle Limit");
    let present = |p: (Option<&Row>, Option<&Row>)| p.0.is_some() || p.1.is_some();
    if present(min_max) && present(cw_ccw) {
        errors.push(Diagnostic::new(
            file,
            "both limit forms: Min/Max Position Limit and CW/CCW Angle Limit rows",
        ));
        return None;
    }
    match (min_max, cw_ccw) {
        ((Some(min), Some(max)), _) => Some(Limits::MinMax {
            min: min.clone(),
            max: max.clone(),
        }),
        (_, (Some(cw), Some(ccw))) => Some(Limits::CwCcw {
            cw: cw.clone(),
            ccw: ccw.clone(),
        }),
        ((None, None), (None, None)) => {
            errors.push(Diagnostic::new(
                file,
                "missing role position limits (rows \"Min Position Limit\" and \"Max Position Limit\", or \"CW Angle Limit\" and \"CCW Angle Limit\")",
            ));
            None
        }
        ((Some(row), None) | (None, Some(row)), _) | (_, (Some(row), None) | (None, Some(row))) => {
            errors.push(Diagnostic::new(
                file,
                format!("half a limit form: {:?} without its pair", row.data_name),
            ));
            None
        }
    }
}

/// The mode source; check 17.
fn mode<'a>(
    file: &str,
    table: &ControlTable,
    find: &impl Fn(&str) -> Option<&'a Row>,
    errors: &mut Vec<Diagnostic>,
) -> Option<ModeRole> {
    let before = errors.len();
    let mut fail = |message: String| errors.push(Diagnostic::new(file, message));
    let operating = find(OPERATING_MODE);
    let control = find(CONTROL_MODE);
    let angle_limits = find("CW Angle Limit").is_some() || find("CCW Angle Limit").is_some();
    if operating.is_some() && control.is_some() {
        fail(format!(
            "two mode registers: {OPERATING_MODE:?} and {CONTROL_MODE:?} rows"
        ));
    } else if operating.is_some() && angle_limits {
        fail(format!(
            "{OPERATING_MODE:?} row beside CW/CCW Angle Limit rows (only {CONTROL_MODE:?} may sit beside them)"
        ));
    }
    let row = operating.or(control);
    match (row, &table.operating_modes) {
        (Some(row), None) => fail(format!("{:?} row without operating_modes", row.data_name)),
        (None, Some(_)) => fail(format!(
            "operating_modes without an {OPERATING_MODE:?} or {CONTROL_MODE:?} row"
        )),
        (None, None) if !angle_limits => fail(format!(
            "no mode source: needs an {OPERATING_MODE:?} or {CONTROL_MODE:?} row, or CW/CCW Angle Limit rows"
        )),
        _ => {}
    }
    if let Some(values) = &table.operating_modes {
        let lists = [
            ("bounded", &values.bounded),
            ("unbounded", &values.unbounded),
            ("no_position", &values.no_position),
        ];
        for (i, (a, list_a)) in lists.iter().enumerate() {
            for (b, list_b) in lists.iter().skip(i + 1) {
                for value in list_a.iter().filter(|v| list_b.contains(v)) {
                    fail(format!(
                        "operating_modes: value {value} in both {a} and {b}"
                    ));
                }
            }
        }
    }
    if errors.len() > before {
        return None;
    }
    match (row, &table.operating_modes) {
        (Some(row), Some(values)) => Some(ModeRole::Register {
            row: row.clone(),
            values: values.clone(),
        }),
        _ => Some(ModeRole::AngleLimits),
    }
}

/// Check 15: the scale is a proper range with zero inside it.
fn check_scale(file: &str, scale: &TypeInfo, errors: &mut Vec<Diagnostic>) {
    let (zero, min, max) = (
        scale.value_of_zero_radian_position,
        scale.value_of_min_radian_position,
        scale.value_of_max_radian_position,
    );
    if max <= min {
        errors.push(Diagnostic::new(
            file,
            format!(
                "type_info: value_of_max_radian_position ({max}) must be above value_of_min_radian_position ({min})"
            ),
        ));
    }
    let finite = scale.min_radian.is_finite() && scale.max_radian.is_finite();
    if !finite || scale.max_radian <= scale.min_radian {
        errors.push(Diagnostic::new(
            file,
            format!(
                "type_info: max_radian ({:?}) must be finite and above min_radian ({:?})",
                scale.max_radian, scale.min_radian
            ),
        ));
    }
    if !(min..=max).contains(&zero) {
        errors.push(Diagnostic::new(
            file,
            format!("type_info: value_of_zero_radian_position ({zero}) outside {min}..={max}"),
        ));
    }
}

/// Check 12: Model Number at (0, 2) in every table, with one access and
/// area. Returns the agreed access and area.
fn check_model_number(models: &[GenModel], errors: &mut Vec<Diagnostic>) -> Option<(Access, Area)> {
    let mut counts: BTreeMap<(Access, Area), usize> = BTreeMap::new();
    for model in models {
        let row = &model.roles.model_number;
        if (row.address, row.size) != (0, 2) {
            errors.push(Diagnostic::new(
                model.file.as_str(),
                format!(
                    "Model Number must be at address 0, size 2, found address {}, size {}",
                    row.address, row.size
                ),
            ));
        }
        *counts.entry((row.access, row.area)).or_default() += 1;
    }
    let (&(access, area), _) = counts.iter().max_by_key(|&(_, &n)| n)?;
    for model in models {
        let row = &model.roles.model_number;
        if (row.access, row.area) != (access, area) {
            errors.push(Diagnostic::new(
                model.file.as_str(),
                format!(
                    "Model Number is {}/{} here but {}/{} in the other tables",
                    access_name(row.access),
                    area_name(row.area),
                    access_name(access),
                    area_name(area)
                ),
            ));
        }
    }
    Some((access, area))
}

/// Check 13: one Firmware Version location across the tables of one
/// (model number, protocol).
fn check_firmware_version(resolved: &Resolved, errors: &mut Vec<Diagnostic>) {
    for list in resolved.lookups.iter().filter(|l| l.firmware_dependent()) {
        let rows: Vec<(&str, &Row)> = list
            .variants
            .iter()
            .filter_map(|(_, ty)| resolved.models.iter().find(|m| m.type_ident == *ty))
            .map(|m| (m.file.as_str(), &m.roles.firmware_version))
            .collect();
        let Some(&(first, row)) = rows.first() else {
            continue;
        };
        for &(other, other_row) in &rows[1..] {
            if (other_row.address, other_row.size) != (row.address, row.size) {
                errors.push(Diagnostic::new(
                    "dynamixel",
                    format!(
                        "model {}: {} tables {first} and {other} have Firmware Version at different locations ({}/{} and {}/{})",
                        list.number,
                        list.protocol.list(),
                        row.address,
                        row.size,
                        other_row.address,
                        other_row.size
                    ),
                ));
            }
        }
    }
}

/// Check 14: a table listed under `p1` keeps every role within P1's 8-bit
/// addresses.
fn check_p1_addresses(model: &GenModel, errors: &mut Vec<Diagnostic>) {
    if !model.appearances.iter().any(|a| a.0 == ProtocolTag::P1) {
        return;
    }
    let roles = &model.roles;
    let (limit_a, limit_b) = match &roles.limits {
        Limits::MinMax { min, max } => (min, max),
        Limits::CwCcw { cw, ccw } => (cw, ccw),
    };
    let mode = match &roles.mode {
        ModeRole::Register { row, .. } => Some(row),
        ModeRole::AngleLimits => None,
    };
    let rows = [
        Some(&roles.model_number),
        Some(&roles.firmware_version),
        Some(&roles.torque_enable),
        Some(&roles.goal_position),
        Some(&roles.present_position),
        Some(limit_a),
        Some(limit_b),
        roles.hardware_error.as_ref(),
        mode,
    ];
    for row in rows.into_iter().flatten() {
        if row.address > 255 {
            errors.push(Diagnostic::new(
                model.file.as_str(),
                format!(
                    "listed under p1 but {} is at address {} (> 255)",
                    row.data_name, row.address
                ),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::fixtures::{Map, Table, outcome};
    use crate::codegen::schema::{OperatingModes, parse_table_str};

    fn single(table: Table) -> Outcome {
        let map = Map::single(1020, "x-series", "T", "t", &table);
        outcome(&map, &[("t", table)])
    }

    fn roles_of(table: Table) -> Roles {
        let out = single(table);
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        out.resolved.models[0].roles.clone()
    }

    #[track_caller]
    fn assert_fails(table: Table, keywords: &[&str]) {
        let out = single(table);
        assert!(
            out.errors
                .iter()
                .any(|d| d.file == "t" && keywords.iter().all(|k| d.message.contains(k))),
            "no error with {keywords:?} in {:?}",
            out.errors
        );
    }

    fn at(row: &Row) -> (u16, u8) {
        (row.address, row.size)
    }

    #[test]
    fn roles_min_max() {
        let roles = roles_of(Table::xm());
        assert_eq!(at(&roles.torque_enable), (64, 1));
        assert_eq!(at(&roles.goal_position), (116, 4));
        assert_eq!(at(&roles.present_position), (132, 4));
        let Limits::MinMax { min, max } = &roles.limits else {
            panic!("expected min/max limits");
        };
        assert_eq!((min.address, max.address), (52, 48));
        assert_eq!(roles.hardware_error.as_ref().map(at), Some((70, 1)));
        assert_eq!(at(&roles.model_number), (0, 2));
        assert_eq!(at(&roles.firmware_version), (6, 1));
    }

    #[test]
    fn roles_cw_ccw() {
        let roles = roles_of(Table::ax());
        let Limits::CwCcw { cw, ccw } = &roles.limits else {
            panic!("expected CW/CCW limits");
        };
        assert_eq!((cw.address, ccw.address), (6, 8));
    }

    #[test]
    fn no_fault_register_is_none() {
        assert_eq!(roles_of(Table::ax()).hardware_error, None);
        let y =
            Table::xm()
                .without("Hardware Error Status")
                .with_row(153, 1, "Error Code", "R", "RAM");
        assert_eq!(roles_of(y).hardware_error, None);
    }

    #[test]
    fn scale_from_type_info() {
        let out = single(Table::ax());
        let scale = out.resolved.models[0].scale;
        assert_eq!(
            (
                scale.value_of_zero_radian_position,
                scale.value_of_min_radian_position,
                scale.value_of_max_radian_position
            ),
            (512, 0, 1023)
        );
        assert_eq!((scale.min_radian, scale.max_radian), Table::ax().radians());
    }

    #[test]
    fn check10_missing_goal_position() {
        assert_fails(
            Table::xm().without("Goal Position"),
            &["missing role Goal Position"],
        );
    }

    #[test]
    fn check10_missing_model_number() {
        assert_fails(
            Table::xm().without("Model Number"),
            &["missing role Model Number"],
        );
    }

    #[test]
    fn check10_half_limit_form() {
        assert_fails(
            Table::xm().without("Max Position Limit"),
            &["half a limit form", "Min Position Limit"],
        );
        assert_fails(
            Table::ax().without("CW Angle Limit"),
            &["half a limit form", "CCW Angle Limit"],
        );
    }

    #[test]
    fn check10_both_limit_forms() {
        assert_fails(
            Table::xm().with_row(6, 2, "CW Angle Limit", "RW", "EEPROM"),
            &["both limit forms"],
        );
    }

    #[test]
    fn check10_no_limits() {
        assert_fails(
            Table::xm()
                .without("Min Position Limit")
                .without("Max Position Limit"),
            &["missing role position limits"],
        );
    }

    fn replaced(table: Table, row: (u16, u8, &str, &'static str, &'static str)) -> Table {
        table
            .without(row.2)
            .with_row(row.0, row.1, row.2, row.3, row.4)
    }

    #[test]
    fn check11_torque_enable_attrs() {
        assert_fails(
            replaced(Table::xm(), (64, 1, "Torque Enable", "R", "RAM")),
            &["role Torque Enable must be RW/RAM, found R/RAM"],
        );
    }

    #[test]
    fn check11_goal_position_attrs() {
        assert_fails(
            replaced(Table::xm(), (116, 4, "Goal Position", "RW", "EEPROM")),
            &["role Goal Position must be RW/RAM, found RW/EEPROM"],
        );
    }

    #[test]
    fn check11_present_position_attrs() {
        assert_fails(
            replaced(Table::xm(), (132, 4, "Present Position", "RW", "RAM")),
            &["role Present Position must be R/RAM, found RW/RAM"],
        );
    }

    #[test]
    fn check11_limit_attrs() {
        assert_fails(
            replaced(Table::xm(), (48, 4, "Max Position Limit", "RW", "Hybrid")),
            &["role Max Position Limit must be RW/EEPROM, found RW/Hybrid"],
        );
    }

    #[test]
    fn check15_raw_range() {
        assert_fails(
            Table::xm().type_info(0, 10, 10, -1.0, 1.0),
            &["value_of_max_radian_position (10) must be above"],
        );
    }

    #[test]
    fn check15_radian_range() {
        assert_fails(
            Table::xm().type_info(2048, 0, 4095, 1.0, -1.0),
            &["max_radian", "above min_radian"],
        );
        assert_fails(
            Table::xm().type_info(2048, 0, 4095, -1.0, f32::INFINITY),
            &["max_radian", "finite"],
        );
    }

    #[test]
    fn check15_zero_outside() {
        assert_fails(
            Table::xm().type_info(5000, 0, 4095, -1.0, 1.0),
            &["value_of_zero_radian_position (5000) outside 0..=4095"],
        );
    }

    fn values(bounded: &[u8], unbounded: &[u8], no_position: &[u8]) -> OperatingModes {
        OperatingModes {
            bounded: bounded.to_vec(),
            unbounded: unbounded.to_vec(),
            no_position: no_position.to_vec(),
        }
    }

    #[test]
    fn mode_operating_mode() {
        let ModeRole::Register { row, values: v } = roles_of(Table::xm()).mode else {
            panic!("expected a mode register");
        };
        assert_eq!(
            (row.data_name.as_str(), at(&row)),
            ("Operating Mode", (11, 1))
        );
        assert_eq!(v, values(&[3], &[4, 5], &[0, 1, 16]));
    }

    #[test]
    fn mode_control_mode() {
        let roles = roles_of(Table::xl320());
        let ModeRole::Register { row, values: v } = &roles.mode else {
            panic!("expected a mode register");
        };
        assert_eq!((row.data_name.as_str(), at(row)), ("Control Mode", (11, 1)));
        assert_eq!(*v, values(&[2], &[], &[1]));
        let Limits::CwCcw { cw, ccw } = &roles.limits else {
            panic!("expected CW/CCW limits");
        };
        assert_eq!((cw.address, ccw.address), (6, 8));
        assert_eq!(roles.hardware_error.as_ref().map(at), Some((50, 1)));
    }

    #[test]
    fn mode_angle_limits() {
        assert_eq!(roles_of(Table::ax()).mode, ModeRole::AngleLimits);
    }

    #[test]
    fn check17_no_mode_source() {
        assert_fails(
            Table::xm()
                .without("Operating Mode")
                .omit_field("operating_modes"),
            &["no mode source"],
        );
    }

    #[test]
    fn check17_operating_mode_with_cw_ccw() {
        assert_fails(
            Table::ax()
                .with_row(11, 1, "Operating Mode", "RW", "EEPROM")
                .operating_modes("(bounded: [3], unbounded: [], no_position: [])"),
            &["\"Operating Mode\" row beside CW/CCW Angle Limit rows"],
        );
    }

    #[test]
    fn check17_two_mode_registers() {
        assert_fails(
            Table::xm().with_row(12, 1, "Control Mode", "RW", "EEPROM"),
            &["\"Operating Mode\"", "\"Control Mode\""],
        );
    }

    #[test]
    fn check17_mode_row_without_operating_modes() {
        assert_fails(
            Table::xm().omit_field("operating_modes"),
            &["\"Operating Mode\" row without operating_modes"],
        );
        assert_fails(
            Table::xl320().omit_field("operating_modes"),
            &["\"Control Mode\" row without operating_modes"],
        );
    }

    #[test]
    fn check17_operating_modes_without_mode_row() {
        assert_fails(
            Table::ax().operating_modes("(bounded: [3], unbounded: [], no_position: [])"),
            &["operating_modes without an \"Operating Mode\" or \"Control Mode\" row"],
        );
    }

    #[test]
    fn check17_overlapping_lists() {
        assert_fails(
            Table::xm().operating_modes("(bounded: [3, 4], unbounded: [4, 5], no_position: [0])"),
            &["value 4 in both bounded and unbounded"],
        );
        assert_fails(
            Table::xm().operating_modes("(bounded: [3], unbounded: [4, 5], no_position: [5])"),
            &["value 5 in both unbounded and no_position"],
        );
    }

    #[test]
    fn empty_lists_allowed() {
        let ModeRole::Register { values: v, .. } =
            roles_of(Table::xm().operating_modes("(bounded: [3], unbounded: [], no_position: [])"))
                .mode
        else {
            panic!("expected a mode register");
        };
        assert_eq!(v, values(&[3], &[], &[]));
    }

    #[test]
    fn check11_mode_register_attrs() {
        for row in [
            (11, 2, "Operating Mode", "RW", "EEPROM"),
            (11, 1, "Operating Mode", "R", "EEPROM"),
            (11, 1, "Operating Mode", "RW", "RAM"),
        ] {
            assert_fails(
                replaced(Table::xm(), row),
                &["role Operating Mode must be 1 byte RW/EEPROM"],
            );
        }
    }

    #[track_caller]
    fn assert_map_fails(map: &Map, tables: &[(&str, Table)], file: &str, keywords: &[&str]) {
        let out = outcome(map, tables);
        assert!(
            out.errors
                .iter()
                .any(|d| d.file == file && keywords.iter().all(|k| d.message.contains(k))),
            "no {file} error with {keywords:?} in {:?}",
            out.errors
        );
    }

    fn p2(table: Table) -> Table {
        table.protocols("[P2]")
    }

    #[test]
    fn check02_first_not_zero() {
        assert_map_fails(
            &Map::new().model(1190, "x-series", "X").p2(&[(1, "a")]),
            &[("a", p2(Table::xm()))],
            "dynamixel",
            &["model 1190: p2 from_fw must start at 0"],
        );
    }

    #[test]
    fn check02_not_increasing() {
        assert_map_fails(
            &Map::new()
                .model(1190, "x-series", "X")
                .p2(&[(0, "a"), (60, "b"), (53, "c")]),
            &[
                ("a", p2(Table::xm())),
                ("b", p2(Table::xm())),
                ("c", p2(Table::xm())),
            ],
            "dynamixel",
            &["model 1190: p2 from_fw must strictly increase"],
        );
    }

    #[test]
    fn check03_no_lists() {
        assert_map_fails(
            &Map::new().model(1020, "x-series", "X"),
            &[],
            "dynamixel",
            &["model 1020: no p1 or p2 list"],
        );
    }

    #[test]
    fn check03_empty_list() {
        assert_map_fails(
            &Map::new().model(1020, "x-series", "X").p1(&[]),
            &[],
            "dynamixel",
            &["model 1020: p1 list is empty"],
        );
    }

    fn both(map: Map) -> Map {
        map.p1(&[(0, "xm")]).p2(&[(0, "xm")])
    }

    #[test]
    fn check04_missing_default() {
        assert_map_fails(
            &both(Map::new().model(1020, "x-series", "X")),
            &[("xm", Table::xm())],
            "dynamixel",
            &["model 1020: default: P2 is required with both p1 and p2"],
        );
    }

    #[test]
    fn check04_default_p1() {
        assert_map_fails(
            &both(Map::new().model(1020, "x-series", "X")).default_p1(),
            &[("xm", Table::xm())],
            "dynamixel",
            &["model 1020: default must be P2, found P1"],
        );
    }

    #[test]
    fn check04_default_with_one_list() {
        assert_map_fails(
            &Map::new()
                .model(1020, "x-series", "X")
                .p2(&[(0, "xm")])
                .default_p2(),
            &[("xm", p2(Table::xm()))],
            "dynamixel",
            &["model 1020: default is only allowed with both p1 and p2"],
        );
    }

    #[test]
    fn check06_file_under_two_numbers() {
        assert_map_fails(
            &Map::new()
                .model(1020, "x-series", "A")
                .p2(&[(0, "xm")])
                .model(1030, "x-series", "B")
                .p2(&[(0, "xm")]),
            &[("xm", p2(Table::xm()))],
            "dynamixel",
            &["table xm is listed under model 1020 and model 1030"],
        );
    }

    #[test]
    fn check08_missing_protocols() {
        let text = Table::xm().omit_field("protocols").render();
        let d = parse_table_str(&text, "xm").unwrap_err();
        assert_eq!(d.file, "xm");
        assert!(d.message.contains("protocols"), "{}", d.message);
    }

    #[test]
    fn check08_missing_source() {
        let text = Table::xm().omit_field("source").render();
        let d = parse_table_str(&text, "xm").unwrap_err();
        assert_eq!(d.file, "xm");
        assert!(d.message.contains("source"), "{}", d.message);
    }

    #[test]
    fn check08_empty_protocols() {
        assert_map_fails(
            &Map::new().model(1020, "x-series", "X").p2(&[(0, "xm")]),
            &[("xm", Table::xm().protocols("[]"))],
            "xm",
            &["protocols is empty"],
        );
    }

    #[test]
    fn check09_p1_list_without_p1() {
        assert_map_fails(
            &Map::new().model(1020, "x-series", "X").p1(&[(0, "xm")]),
            &[("xm", p2(Table::xm()))],
            "xm",
            &["listed under p1 but its protocols lack P1"],
        );
    }

    #[test]
    fn check12_model_number_address() {
        assert_fails(
            replaced(Table::xm(), (0, 4, "Model Number", "R", "EEPROM")),
            &["Model Number must be at address 0, size 2, found address 0, size 4"],
        );
    }

    #[test]
    fn check12_model_number_access() {
        assert_map_fails(
            &Map::new()
                .model(12, "ax", "A")
                .p1(&[(0, "a")])
                .model(18, "ax", "B")
                .p1(&[(0, "b")])
                .model(300, "ax", "C")
                .p1(&[(0, "c")]),
            &[
                ("a", Table::ax()),
                ("b", Table::ax()),
                (
                    "c",
                    replaced(Table::ax(), (0, 2, "Model Number", "RW", "EEPROM")),
                ),
            ],
            "c",
            &["Model Number is RW/EEPROM here but R/EEPROM in the other tables"],
        );
    }

    #[test]
    fn check13_firmware_version_moves() {
        assert_map_fails(
            &Map::new()
                .model(1190, "x-series", "X")
                .p2(&[(0, "old"), (53, "new")]),
            &[
                ("old", p2(Table::xm())),
                (
                    "new",
                    p2(replaced(
                        Table::xm(),
                        (2, 1, "Firmware Version", "R", "EEPROM"),
                    )),
                ),
            ],
            "dynamixel",
            &[
                "model 1190",
                "old",
                "new",
                "Firmware Version at different locations",
            ],
        );
    }

    #[test]
    fn check14_p1_role_above_255() {
        assert_fails(
            replaced(Table::xm(), (564, 4, "Goal Position", "RW", "RAM")),
            &["listed under p1 but Goal Position is at address 564 (> 255)"],
        );
        let out = single(p2(replaced(
            Table::xm(),
            (564, 4, "Goal Position", "RW", "RAM"),
        )));
        assert!(out.errors.is_empty(), "{:?}", out.errors);
    }

    #[test]
    fn lookup_lists_per_protocol() {
        let out = outcome(
            &Map::new()
                .model(1190, "x-series", "X")
                .p2(&[(0, "fw52"), (53, "new")]),
            &[("fw52", p2(Table::xm())), ("new", p2(Table::xm()))],
        );
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        let lookups = &out.resolved.lookups;
        assert_eq!(lookups.len(), 1);
        assert_eq!(lookups[0].protocol, ProtocolTag::P2);
        assert_eq!(
            lookups[0].variants,
            [(0, "Fw52".to_string()), (53, "New".to_string())]
        );
        assert!(lookups[0].firmware_dependent());
        assert_eq!(
            out.resolved.models[0].appearances,
            [(ProtocolTag::P2, 0, Some(53))]
        );
    }

    #[test]
    fn same_file_under_p1_and_p2() {
        let out = single(Table::xm());
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        assert_eq!(out.resolved.models.len(), 1);
        assert_eq!(
            out.resolved.models[0].appearances,
            [(ProtocolTag::P1, 0, None), (ProtocolTag::P2, 0, None)]
        );
        let lookups = &out.resolved.lookups;
        assert_eq!(lookups.len(), 2);
        assert_eq!(
            (lookups[0].protocol, lookups[1].protocol),
            (ProtocolTag::P1, ProtocolTag::P2)
        );
        assert!(!lookups[0].firmware_dependent());
        assert_eq!(out.resolved.model_number, Some((Access::R, Area::Eeprom)));
    }

    #[test]
    fn check05_unknown_family() {
        assert_map_fails(
            &Map::new().model(1020, "x_series", "X").p2(&[(0, "xm")]),
            &[("xm", p2(Table::xm()))],
            "dynamixel",
            &[
                "model 1020",
                "\"x_series\"",
                "expected ax, mx, rx, x-series, y-series, pro",
            ],
        );
    }

    #[test]
    fn rx_is_a_family() {
        let out = outcome(
            &Map::new().model(28, "rx", "RX-28").p1(&[(0, "rx_28")]),
            &[("rx_28", Table::ax())],
        );
        assert!(out.errors.is_empty(), "{:?}", out.errors);
    }
}
