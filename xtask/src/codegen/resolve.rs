//! Resolves the map and the tables into what is rendered, and runs every
//! check of data-model §3 on the way. Problems are collected, never thrown,
//! so one run reports all of them.

use std::collections::{BTreeMap, BTreeSet};

use super::diagnostic::{Check, Diagnostic};
use super::fr9::{self, GenRole};
use super::names::{const_ident, is_reserved_module, module_ident, type_ident};
use super::schema::{
    Access, Area, ControlTable, Instruction, ModelEntry, ModelMap, OperatingModes, ProtocolTag,
    Row, Source, TableRef, TypeInfo,
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
    /// The FR-9 typed registers the table has, in `fr9::ROLES` order.
    pub(crate) typed: Vec<GenRole>,
    /// Whether the table's `instructions` list Bulk Read on Protocol 1 for
    /// every firmware.
    pub(crate) bulk_read_p1: bool,
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
    /// No mode row: the angle limits, overridden by the table's
    /// `torque_control_mode` switch (MX on Protocol 1).
    TorqueControl {
        /// The 1-byte switch row.
        row: Row,
        /// The value that turns torque control on.
        on: u8,
        /// The values the switch takes.
        values: (u8, u8),
    },
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
    /// model or when the tables differ (check 12).
    pub(crate) model_number: Option<(Access, Area)>,
}

impl Resolved {
    /// Whether Setup must read the firmware version of `list`'s model: its
    /// table depends on firmware, or one of its tables has an FR-9 register
    /// only from some firmware on (MX-64 Protocol 1 Realtime Tick).
    pub(crate) fn reads_firmware(&self, list: &LookupList) -> bool {
        list.firmware_dependent()
            || list.variants.iter().any(|(_, ty)| {
                self.models
                    .iter()
                    .filter(|model| &model.type_ident == ty)
                    .any(|model| model.typed.iter().any(|role| role.from_fw > 0))
            })
    }
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
    let mut fail = |check: Check| errors.push(Diagnostic::new("dynamixel", check));
    let model = number;
    if entry.p1.is_none() && entry.p2.is_none() {
        fail(Check::NoLists { model });
    }
    for (protocol, list) in [(ProtocolTag::P1, &entry.p1), (ProtocolTag::P2, &entry.p2)] {
        if list.as_ref().is_some_and(Vec::is_empty) {
            fail(Check::EmptyList { model, protocol });
        }
    }
    match (entry.p1.is_some() && entry.p2.is_some(), entry.default) {
        (true, None) => fail(Check::DefaultMissing { model }),
        (true, Some(found @ ProtocolTag::P1)) => fail(Check::DefaultNotP2 { model, found }),
        (false, Some(default)) => fail(Check::DefaultWithOneList {
            model,
            default,
            present: match (&entry.p1, &entry.p2) {
                (Some(_), _) => Some(ProtocolTag::P1),
                (None, Some(_)) => Some(ProtocolTag::P2),
                (None, None) => None,
            },
        }),
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
                Check::UnknownFamily {
                    model: number,
                    family: entry.family.clone(),
                },
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
                        Check::TableUnderTwoModels {
                            table: table.file.clone(),
                            first: mapping.number,
                            second: number,
                        },
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
                Check::DuplicateType {
                    first: other.to_string(),
                    second: file.to_string(),
                    ty: ty.clone(),
                },
            ));
        }
        let module = module_ident(file);
        if is_reserved_module(&module) {
            out.errors.push(Diagnostic::new(
                file,
                Check::ReservedModule {
                    module: module.clone(),
                },
            ));
        }
        let Some(table) = tables.get(file) else {
            continue;
        };
        if table.protocols.is_empty() {
            out.errors
                .push(Diagnostic::new(file, Check::EmptyProtocols));
        }
        for &(protocol, _, _) in &mapping.appearances {
            if !table.protocols.contains(&protocol) {
                out.errors
                    .push(Diagnostic::new(file, Check::ProtocolMissing { protocol }));
            }
        }
        let registers = registers(file, table, &mut out.errors);
        check_scale(file, &table.type_info, &mut out.errors);
        let before = out.errors.len();
        fr9::check_unit_source(file, table, &mut out.errors);
        fr9::check_unit_info(file, table, &mut out.errors);
        fr9::check_per_mode(file, table, &mut out.errors);
        let Some(mut roles) = roles(file, table, &mut out.errors) else {
            continue;
        };
        let angle_limits = roles.mode == ModeRole::AngleLimits;
        let switch = fr9::torque_switch(file, table, angle_limits, &mut out.errors);
        if out.errors.len() > before {
            continue;
        }
        if let Some(switch) = &switch {
            roles.mode = ModeRole::TorqueControl {
                row: switch.row.clone(),
                on: switch.on,
                values: switch.values,
            };
        }
        let typed = fr9::roles(file, table, switch.as_ref(), &mut out.errors);
        out.resolved.models.push(GenModel {
            file: file.to_string(),
            type_ident: ty,
            module_ident: module,
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
            typed,
            bulk_read_p1: table
                .instructions
                .iter()
                .any(|list| list.always_has(ProtocolTag::P1, Instruction::BulkRead)),
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
    for &file in &not_mapped {
        if mapped.contains_key(file) {
            out.errors.push(Diagnostic::new(
                "dynamixel",
                Check::MappedAndNotMapped {
                    table: file.to_string(),
                },
            ));
        }
        if !on_disk.contains(file) {
            out.errors.push(Diagnostic::new(
                "dynamixel",
                Check::NotMappedMissing {
                    table: file.to_string(),
                },
            ));
        }
    }
    for stem in on_disk {
        if !mapped.contains_key(stem.as_str()) && !not_mapped.contains(stem.as_str()) {
            out.warnings
                .push(Diagnostic::new(stem.as_str(), Check::Unmapped));
        }
    }
    out
}

/// Check 2: each list starts at firmware 0 and strictly increases; one
/// diagnostic per pair out of order.
fn check_order(
    number: u16,
    protocol: ProtocolTag,
    list: &[TableRef],
    errors: &mut Vec<Diagnostic>,
) {
    if let Some(first) = list.first()
        && first.from_fw != 0
    {
        errors.push(Diagnostic::new(
            "dynamixel",
            Check::FirstFromFw {
                model: number,
                protocol,
                table: first.file.clone(),
                from_fw: first.from_fw,
            },
        ));
    }
    let observed: Vec<u8> = list.iter().map(|t| t.from_fw).collect();
    for pair in list.windows(2).filter(|w| w[0].from_fw >= w[1].from_fw) {
        errors.push(Diagnostic::new(
            "dynamixel",
            Check::FromFwOrder {
                model: number,
                protocol,
                observed: observed.clone(),
                first: pair[0].file.clone(),
                first_fw: pair[0].from_fw,
                second: pair[1].file.clone(),
                second_fw: pair[1].from_fw,
            },
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
                Check::DuplicateConstant {
                    first: other.to_string(),
                    second: row.data_name.clone(),
                    ident: ident.clone(),
                },
            ));
        }
        out.push((ident, row.clone()));
    }
    out
}

const OPERATING_MODE: &str = "Operating Mode";
const CONTROL_MODE: &str = "Control Mode";

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
                Check::MissingRole {
                    register: name.to_string(),
                },
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
            Check::ModeRowAttrs {
                register: row.data_name.clone(),
                size: row.size,
                access: row.access,
                area: row.area,
            },
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
            Check::RoleAttrs {
                register: role.to_string(),
                expected: (access, area),
                found: (row.access, row.area),
            },
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
        let rows = [min_max.0, min_max.1, cw_ccw.0, cw_ccw.1]
            .into_iter()
            .flatten()
            .map(|row| row.data_name.clone())
            .collect();
        errors.push(Diagnostic::new(file, Check::BothLimitForms { rows }));
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
            errors.push(Diagnostic::new(file, Check::MissingLimits));
            None
        }
        ((Some(row), None) | (None, Some(row)), _) | (_, (Some(row), None) | (None, Some(row))) => {
            errors.push(Diagnostic::new(
                file,
                Check::HalfLimitForm {
                    register: row.data_name.clone(),
                },
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
    let mut fail = |check: Check| errors.push(Diagnostic::new(file, check));
    let operating = find(OPERATING_MODE);
    let control = find(CONTROL_MODE);
    let angle_limits = find("CW Angle Limit").is_some() || find("CCW Angle Limit").is_some();
    if operating.is_some() && control.is_some() {
        fail(Check::TwoModeRegisters);
    } else if operating.is_some() && angle_limits {
        fail(Check::OperatingModeBesideAngleLimits);
    }
    let row = operating.or(control);
    match (row, &table.operating_modes) {
        (Some(row), None) => fail(Check::ModeRowWithoutValues {
            register: row.data_name.clone(),
        }),
        (None, Some(_)) => fail(Check::ValuesWithoutModeRow),
        (None, None) if !angle_limits => fail(Check::NoModeSource),
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
                for &value in list_a.iter().filter(|v| list_b.contains(v)) {
                    fail(Check::ModeValueInTwoLists {
                        value,
                        first: a,
                        second: b,
                    });
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
        errors.push(Diagnostic::new(file, Check::RawScaleOrder { min, max }));
    }
    let finite = scale.min_radian.is_finite() && scale.max_radian.is_finite();
    if !finite || scale.max_radian <= scale.min_radian {
        errors.push(Diagnostic::new(
            file,
            Check::RadianScale {
                min: scale.min_radian,
                max: scale.max_radian,
            },
        ));
    }
    if !(min..=max).contains(&zero) {
        errors.push(Diagnostic::new(file, Check::ZeroOutside { zero, min, max }));
    }
}

/// Check 12: Model Number at (0, 2) in every table, with one access and
/// area. Returns that access and area, or `None` with no model or when the
/// tables differ.
fn check_model_number(models: &[GenModel], errors: &mut Vec<Diagnostic>) -> Option<(Access, Area)> {
    let mut groups: BTreeMap<(Access, Area), Vec<String>> = BTreeMap::new();
    for model in models {
        let row = &model.roles.model_number;
        if (row.address, row.size) != (0, 2) {
            errors.push(Diagnostic::new(
                model.file.as_str(),
                Check::ModelNumberLocation {
                    address: row.address,
                    size: row.size,
                },
            ));
        }
        groups
            .entry((row.access, row.area))
            .or_default()
            .push(model.file.clone());
    }
    if groups.len() > 1 {
        // DECISION: one diagnostic on the map listing every table's value,
        // not a majority vote blaming the fewest: no table is known right.
        errors.push(Diagnostic::new(
            "dynamixel",
            Check::ModelNumberAttrsDiffer {
                groups: groups.into_iter().collect(),
            },
        ));
        return None;
    }
    groups.into_keys().next()
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
                    Check::FirmwareVersionMoves {
                        model: list.number,
                        protocol: list.protocol,
                        first: first.to_string(),
                        first_at: (row.address, row.size),
                        other: other.to_string(),
                        other_at: (other_row.address, other_row.size),
                    },
                ));
            }
        }
    }
}

/// Check 14: a table listed under `p1` keeps every role, FR-9 roles and
/// their bounding rows included, within P1's 8-bit addresses.
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
        ModeRole::Register { row, .. } | ModeRole::TorqueControl { row, .. } => Some(row),
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
    let typed = model
        .typed
        .iter()
        .flat_map(|role| [Some(&role.row), role.bounded_by.as_ref()]);
    for row in rows.into_iter().chain(typed).flatten() {
        if row.address > 255 {
            errors.push(Diagnostic::new(
                model.file.as_str(),
                Check::P1Address {
                    register: row.data_name.clone(),
                    address: row.address,
                },
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::diagnostic::Cause;
    use crate::codegen::fixtures::{Map, Table, assert_check, outcome};
    use crate::codegen::schema::{OperatingModes, parse_table_str};

    #[track_caller]
    fn fails_with(table: Table, check: Check) {
        assert_check(&single(table).errors, "t", &check);
    }

    #[track_caller]
    fn map_fails_with(map: &Map, tables: &[(&str, Table)], file: &str, check: Check) {
        assert_check(&outcome(map, tables).errors, file, &check);
    }

    fn single(table: Table) -> Outcome {
        let map = Map::single(1020, "x-series", "T", "t", &table);
        outcome(&map, &[("t", table)])
    }

    fn roles_of(table: Table) -> Roles {
        let out = single(table);
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        out.resolved.models[0].roles.clone()
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
        fails_with(
            Table::xm().without("Goal Position"),
            Check::MissingRole {
                register: "Goal Position".into(),
            },
        );
    }

    #[test]
    fn check10_missing_model_number() {
        fails_with(
            Table::xm().without("Model Number"),
            Check::MissingRole {
                register: "Model Number".into(),
            },
        );
    }

    #[test]
    fn check10_half_limit_form() {
        fails_with(
            Table::xm().without("Max Position Limit"),
            Check::HalfLimitForm {
                register: "Min Position Limit".into(),
            },
        );
        fails_with(
            Table::ax().without("CW Angle Limit"),
            Check::HalfLimitForm {
                register: "CCW Angle Limit".into(),
            },
        );
    }

    #[test]
    fn check10_no_limits() {
        fails_with(
            Table::xm()
                .without("Min Position Limit")
                .without("Max Position Limit"),
            Check::MissingLimits,
        );
    }

    fn replaced(table: Table, row: (u16, u8, &str, &'static str, &'static str)) -> Table {
        table
            .without(row.2)
            .with_row(row.0, row.1, row.2, row.3, row.4)
    }

    #[test]
    fn check11_torque_enable_attrs() {
        fails_with(
            replaced(Table::xm(), (64, 1, "Torque Enable", "R", "RAM")),
            Check::RoleAttrs {
                register: "Torque Enable".into(),
                expected: (Access::RW, Area::Ram),
                found: (Access::R, Area::Ram),
            },
        );
    }

    #[test]
    fn check11_goal_position_attrs() {
        fails_with(
            replaced(Table::xm(), (116, 4, "Goal Position", "RW", "EEPROM")),
            Check::RoleAttrs {
                register: "Goal Position".into(),
                expected: (Access::RW, Area::Ram),
                found: (Access::RW, Area::Eeprom),
            },
        );
    }

    #[test]
    fn check11_present_position_attrs() {
        fails_with(
            replaced(Table::xm(), (132, 4, "Present Position", "RW", "RAM")),
            Check::RoleAttrs {
                register: "Present Position".into(),
                expected: (Access::R, Area::Ram),
                found: (Access::RW, Area::Ram),
            },
        );
    }

    #[test]
    fn check11_limit_attrs() {
        fails_with(
            replaced(Table::xm(), (48, 4, "Max Position Limit", "RW", "Hybrid")),
            Check::RoleAttrs {
                register: "Max Position Limit".into(),
                expected: (Access::RW, Area::Eeprom),
                found: (Access::RW, Area::Hybrid),
            },
        );
    }

    #[test]
    fn check15_raw_range() {
        fails_with(
            Table::xm().type_info(0, 10, 10, -1.0, 1.0),
            Check::RawScaleOrder { min: 10, max: 10 },
        );
    }

    #[test]
    fn check15_radian_range() {
        fails_with(
            Table::xm().type_info(2048, 0, 4095, 1.0, -1.0),
            Check::RadianScale {
                min: 1.0,
                max: -1.0,
            },
        );
        fails_with(
            Table::xm().type_info(2048, 0, 4095, -1.0, f32::INFINITY),
            Check::RadianScale {
                min: -1.0,
                max: f32::INFINITY,
            },
        );
    }

    #[test]
    fn check15_zero_outside() {
        fails_with(
            Table::xm().type_info(5000, 0, 4095, -1.0, 1.0),
            Check::ZeroOutside {
                zero: 5000,
                min: 0,
                max: 4095,
            },
        );
    }

    fn values(bounded: &[u8], unbounded: &[u8], no_position: &[u8]) -> OperatingModes {
        OperatingModes {
            bounded: bounded.to_vec(),
            unbounded: unbounded.to_vec(),
            no_position: no_position.to_vec(),
            current: None,
            velocity: None,
            pwm: None,
            current_position: None,
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
        fails_with(
            Table::xm()
                .without("Operating Mode")
                .omit_field("operating_modes"),
            Check::NoModeSource,
        );
    }

    #[test]
    fn check17_operating_mode_with_cw_ccw() {
        fails_with(
            Table::ax()
                .with_row(11, 1, "Operating Mode", "RW", "EEPROM")
                .operating_modes("(bounded: [3], unbounded: [], no_position: [])"),
            Check::OperatingModeBesideAngleLimits,
        );
    }

    #[test]
    fn check17_two_mode_registers() {
        fails_with(
            Table::xm().with_row(12, 1, "Control Mode", "RW", "EEPROM"),
            Check::TwoModeRegisters,
        );
    }

    #[test]
    fn check17_mode_row_without_operating_modes() {
        fails_with(
            Table::xm().omit_field("operating_modes"),
            Check::ModeRowWithoutValues {
                register: "Operating Mode".into(),
            },
        );
        fails_with(
            Table::xl320().omit_field("operating_modes"),
            Check::ModeRowWithoutValues {
                register: "Control Mode".into(),
            },
        );
    }

    #[test]
    fn check17_operating_modes_without_mode_row() {
        fails_with(
            Table::ax().operating_modes("(bounded: [3], unbounded: [], no_position: [])"),
            Check::ValuesWithoutModeRow,
        );
    }

    #[test]
    fn check17_overlapping_lists() {
        fails_with(
            Table::xm().operating_modes("(bounded: [3, 4], unbounded: [4, 5], no_position: [0])"),
            Check::ModeValueInTwoLists {
                value: 4,
                first: "bounded",
                second: "unbounded",
            },
        );
        fails_with(
            Table::xm().operating_modes("(bounded: [3], unbounded: [4, 5], no_position: [5])"),
            Check::ModeValueInTwoLists {
                value: 5,
                first: "unbounded",
                second: "no_position",
            },
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
        for (row, size, access, area) in [
            (
                (11, 2, "Operating Mode", "RW", "EEPROM"),
                2,
                Access::RW,
                Area::Eeprom,
            ),
            (
                (11, 1, "Operating Mode", "R", "EEPROM"),
                1,
                Access::R,
                Area::Eeprom,
            ),
            (
                (11, 1, "Operating Mode", "RW", "RAM"),
                1,
                Access::RW,
                Area::Ram,
            ),
        ] {
            fails_with(
                replaced(Table::xm(), row),
                Check::ModeRowAttrs {
                    register: "Operating Mode".into(),
                    size,
                    access,
                    area,
                },
            );
        }
    }

    fn p2(table: Table) -> Table {
        table.protocols("[P2]")
    }

    #[test]
    fn check03_no_lists() {
        map_fails_with(
            &Map::new().model(1020, "x-series", "X"),
            &[],
            "dynamixel",
            Check::NoLists { model: 1020 },
        );
    }

    #[test]
    fn check03_empty_list() {
        map_fails_with(
            &Map::new().model(1020, "x-series", "X").p1(&[]),
            &[],
            "dynamixel",
            Check::EmptyList {
                model: 1020,
                protocol: ProtocolTag::P1,
            },
        );
    }

    fn both(map: Map) -> Map {
        map.p1(&[(0, "xm")]).p2(&[(0, "xm")])
    }

    #[test]
    fn check04_missing_default() {
        map_fails_with(
            &both(Map::new().model(1020, "x-series", "X")),
            &[("xm", Table::xm())],
            "dynamixel",
            Check::DefaultMissing { model: 1020 },
        );
    }

    #[test]
    fn check04_default_p1() {
        map_fails_with(
            &both(Map::new().model(1020, "x-series", "X")).default_p1(),
            &[("xm", Table::xm())],
            "dynamixel",
            Check::DefaultNotP2 {
                model: 1020,
                found: ProtocolTag::P1,
            },
        );
    }

    #[test]
    fn check06_file_under_two_numbers() {
        map_fails_with(
            &Map::new()
                .model(1020, "x-series", "A")
                .p2(&[(0, "xm")])
                .model(1030, "x-series", "B")
                .p2(&[(0, "xm")]),
            &[("xm", p2(Table::xm()))],
            "dynamixel",
            Check::TableUnderTwoModels {
                table: "xm".into(),
                first: 1020,
                second: 1030,
            },
        );
    }

    #[test]
    fn check08_missing_protocols() {
        let text = Table::xm().omit_field("protocols").render();
        let d = parse_table_str(&text, "xm").unwrap_err();
        assert_eq!(d.file, "xm");
        assert!(
            matches!(&d.cause, Cause::Parse { code, .. }
                if matches!(**code, ron::Error::MissingStructField { field: "protocols", .. })),
            "{d:?}"
        );
    }

    #[test]
    fn check08_missing_source() {
        let text = Table::xm().omit_field("source").render();
        let d = parse_table_str(&text, "xm").unwrap_err();
        assert_eq!(d.file, "xm");
        assert!(
            matches!(&d.cause, Cause::Parse { code, .. }
                if matches!(**code, ron::Error::MissingStructField { field: "source", .. })),
            "{d:?}"
        );
    }

    #[test]
    fn check08_empty_protocols() {
        map_fails_with(
            &Map::new().model(1020, "x-series", "X").p2(&[(0, "xm")]),
            &[("xm", Table::xm().protocols("[]"))],
            "xm",
            Check::EmptyProtocols,
        );
    }

    #[test]
    fn check09_p1_list_without_p1() {
        map_fails_with(
            &Map::new().model(1020, "x-series", "X").p1(&[(0, "xm")]),
            &[("xm", p2(Table::xm()))],
            "xm",
            Check::ProtocolMissing {
                protocol: ProtocolTag::P1,
            },
        );
    }

    #[test]
    fn check12_model_number_address() {
        fails_with(
            replaced(Table::xm(), (0, 4, "Model Number", "R", "EEPROM")),
            Check::ModelNumberLocation {
                address: 0,
                size: 4,
            },
        );
    }

    #[test]
    fn check13_firmware_version_moves() {
        map_fails_with(
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
            Check::FirmwareVersionMoves {
                model: 1190,
                protocol: ProtocolTag::P2,
                first: "old".into(),
                first_at: (6, 1),
                other: "new".into(),
                other_at: (2, 1),
            },
        );
    }

    #[test]
    fn check14_p1_role_above_255() {
        fails_with(
            replaced(Table::xm(), (564, 4, "Goal Position", "RW", "RAM")),
            Check::P1Address {
                register: "Goal Position".into(),
                address: 564,
            },
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
        map_fails_with(
            &Map::new().model(1020, "x_series", "X").p2(&[(0, "xm")]),
            &[("xm", p2(Table::xm()))],
            "dynamixel",
            Check::UnknownFamily {
                model: 1020,
                family: "x_series".into(),
            },
        );
    }

    #[test]
    fn check02_names_the_offending_values() {
        map_fails_with(
            &Map::new().model(1190, "x-series", "X").p2(&[(1, "a")]),
            &[("a", p2(Table::xm()))],
            "dynamixel",
            Check::FirstFromFw {
                model: 1190,
                protocol: ProtocolTag::P2,
                table: "a".into(),
                from_fw: 1,
            },
        );
        let errors = outcome(
            &Map::new().model(1190, "x-series", "X").p2(&[
                (0, "a"),
                (60, "b"),
                (53, "c"),
                (53, "d"),
            ]),
            &[
                ("a", p2(Table::xm())),
                ("b", p2(Table::xm())),
                ("c", p2(Table::xm())),
                ("d", p2(Table::xm())),
            ],
        )
        .errors;
        for (first, first_fw, second, second_fw) in [("b", 60, "c", 53), ("c", 53, "d", 53)] {
            assert_check(
                &errors,
                "dynamixel",
                &Check::FromFwOrder {
                    model: 1190,
                    protocol: ProtocolTag::P2,
                    observed: vec![0, 60, 53, 53],
                    first: first.into(),
                    first_fw,
                    second: second.into(),
                    second_fw,
                },
            );
        }
    }

    #[test]
    fn check04_default_with_one_list_names_it() {
        map_fails_with(
            &Map::new()
                .model(1020, "x-series", "X")
                .p2(&[(0, "xm")])
                .default_p2(),
            &[("xm", p2(Table::xm()))],
            "dynamixel",
            Check::DefaultWithOneList {
                model: 1020,
                default: ProtocolTag::P2,
                present: Some(ProtocolTag::P2),
            },
        );
        map_fails_with(
            &Map::new().model(1020, "x-series", "X").default_p1(),
            &[],
            "dynamixel",
            Check::DefaultWithOneList {
                model: 1020,
                default: ProtocolTag::P1,
                present: None,
            },
        );
    }

    #[test]
    fn check10_both_limit_forms_names_the_rows() {
        fails_with(
            Table::xm().with_row(6, 2, "CW Angle Limit", "RW", "EEPROM"),
            Check::BothLimitForms {
                rows: vec![
                    "Min Position Limit".into(),
                    "Max Position Limit".into(),
                    "CW Angle Limit".into(),
                ],
            },
        );
    }

    #[test]
    fn check12_lists_each_tables_model_number_attrs() {
        let rw = || replaced(Table::ax(), (0, 2, "Model Number", "RW", "EEPROM"));
        map_fails_with(
            &Map::new()
                .model(12, "ax", "A")
                .p1(&[(0, "a")])
                .model(18, "ax", "B")
                .p1(&[(0, "b")])
                .model(300, "ax", "C")
                .p1(&[(0, "c")]),
            &[("a", Table::ax()), ("b", Table::ax()), ("c", rw())],
            "dynamixel",
            Check::ModelNumberAttrsDiffer {
                groups: vec![
                    ((Access::R, Area::Eeprom), vec!["a".into(), "b".into()]),
                    ((Access::RW, Area::Eeprom), vec!["c".into()]),
                ],
            },
        );
        let out = outcome(
            &Map::new()
                .model(12, "ax", "A")
                .p1(&[(0, "a")])
                .model(18, "ax", "B")
                .p1(&[(0, "b")]),
            &[("a", rw()), ("b", Table::ax())],
        );
        assert_eq!(out.errors.len(), 1, "{:?}", out.errors);
        assert_check(
            &out.errors,
            "dynamixel",
            &Check::ModelNumberAttrsDiffer {
                groups: vec![
                    ((Access::R, Area::Eeprom), vec!["b".into()]),
                    ((Access::RW, Area::Eeprom), vec!["a".into()]),
                ],
            },
        );
        assert_eq!(out.resolved.model_number, None);
    }

    #[test]
    fn rx_is_a_family() {
        let out = outcome(
            &Map::new().model(28, "rx", "RX-28").p1(&[(0, "rx_28")]),
            &[("rx_28", Table::ax())],
        );
        assert!(out.errors.is_empty(), "{:?}", out.errors);
    }

    use crate::codegen::fixtures::{Root, xm_map};
    use crate::codegen::{Failure, generate};

    fn generate_errors(root: &Root) -> Vec<Diagnostic> {
        match generate(&root.control_tables()) {
            Err(Failure::Checks { errors, .. }) => errors,
            other => panic!("expected check failures, got {other:?}"),
        }
    }

    #[test]
    fn check01_map_missing() {
        let root = Root::new(&xm_map(), &[("xm", Table::xm())]);
        let path = root.control_tables().join("dynamixel.ron");
        std::fs::remove_file(&path).unwrap();
        let errors = generate_errors(&root);
        assert!(
            errors.iter().any(|d| d.file == "dynamixel"
                && matches!(&d.cause, Cause::Read { path: p, source }
                    if *p == path && source.kind() == std::io::ErrorKind::NotFound)),
            "{errors:?}"
        );
    }

    #[test]
    fn check01_map_unparsable() {
        let root = Root::new(&xm_map(), &[("xm", Table::xm())]);
        root.write("dynamixel", "(models: {");
        let errors = generate_errors(&root);
        assert!(
            errors.iter().any(|d| d.file == "dynamixel"
                && matches!(&d.cause, Cause::Parse { line: 1, column: 10, code }
                    if **code == ron::Error::Eof)),
            "{errors:?}"
        );
    }

    #[test]
    fn check01_table_missing() {
        let root = Root::new(&xm_map(), &[]);
        let path = root.control_tables().join("xm.ron");
        let errors = generate_errors(&root);
        assert!(
            errors.iter().any(|d| d.file == "xm"
                && matches!(&d.cause, Cause::Read { path: p, source }
                    if *p == path && source.kind() == std::io::ErrorKind::NotFound)),
            "{errors:?}"
        );
    }

    #[test]
    fn check01_table_unparsable() {
        let root = Root::new(&xm_map(), &[("xm", Table::xm())]);
        root.write("xm", "( source: Unverified, protocols: [P1, P2], ");
        let errors = generate_errors(&root);
        assert!(
            errors.iter().any(|d| d.file == "xm"
                && matches!(&d.cause, Cause::Parse { line: 1, code, .. }
                    if **code == ron::Error::Eof)),
            "{errors:?}"
        );
    }

    #[test]
    fn check01_row_without_access() {
        let root = Root::new(&xm_map(), &[("xm", Table::xm())]);
        let text = Table::xm()
            .render()
            .replace("access: RW, area: RAM", "area: RAM");
        root.write("xm", &text);
        let errors = generate_errors(&root);
        assert!(
            errors.iter().any(|d| d.file == "xm"
                && matches!(&d.cause, Cause::Parse { code, .. }
                    if matches!(**code, ron::Error::MissingStructField { field: "access", .. }))),
            "{errors:?}"
        );
    }

    #[test]
    fn two_broken_tables_two_diagnostics() {
        let root = Root::new(
            &Map::new()
                .model(12, "ax", "A")
                .p1(&[(0, "b")])
                .model(18, "ax", "B")
                .p1(&[(0, "a")]),
            &[
                ("a", Table::ax().without("Goal Position")),
                ("b", Table::ax().omit_field("source")),
            ],
        );
        let errors = generate_errors(&root);
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert_eq!(
            (errors[0].file.as_str(), errors[1].file.as_str()),
            ("a", "b")
        );
    }

    #[test]
    fn check07_mapped_and_not_mapped() {
        map_fails_with(
            &xm_map().not_mapped("xm", "NoModelNumber"),
            &[("xm", Table::xm())],
            "dynamixel",
            Check::MappedAndNotMapped { table: "xm".into() },
        );
    }

    #[test]
    fn check07_not_mapped_missing() {
        map_fails_with(
            &xm_map().not_mapped("gone", "SameNumberAs(1020)"),
            &[("xm", Table::xm())],
            "dynamixel",
            Check::NotMappedMissing {
                table: "gone".into(),
            },
        );
    }

    #[test]
    fn check16_duplicate_type() {
        map_fails_with(
            &Map::new()
                .model(1, "ax", "A")
                .p1(&[(0, "xl_320")])
                .model(2, "ax", "B")
                .p1(&[(0, "xl320")]),
            &[("xl_320", Table::ax()), ("xl320", Table::ax())],
            "xl_320",
            Check::DuplicateType {
                first: "xl320".into(),
                second: "xl_320".into(),
                ty: "Xl320".into(),
            },
        );
    }

    #[test]
    fn check16_duplicate_register() {
        fails_with(
            Table::xm()
                .with_row(65, 1, "LED", "RW", "RAM")
                .with_row(66, 1, "Led", "RW", "RAM"),
            Check::DuplicateConstant {
                first: "LED".into(),
                second: "Led".into(),
                ident: "LED".into(),
            },
        );
    }

    #[test]
    fn check16_module_tests() {
        let table = Table::xm();
        map_fails_with(
            &Map::single(1020, "x-series", "X", "tests", &table),
            &[("tests", table)],
            "tests",
            Check::ReservedModule {
                module: "tests".into(),
            },
        );
    }

    #[test]
    fn check16_module_keyword() {
        let table = Table::xm();
        map_fails_with(
            &Map::single(1020, "x-series", "X", "type", &table),
            &[("type", table)],
            "type",
            Check::ReservedModule {
                module: "type".into(),
            },
        );
    }

    #[test]
    fn aliases_on_one_address_allowed() {
        let out = single(
            Table::xm()
                .with_row(168, 2, "Indirect Address 1", "RW", "RAM")
                .with_row(168, 2, "Indirect Address Write", "RW", "RAM"),
        );
        assert!(out.errors.is_empty(), "{:?}", out.errors);
    }

    #[test]
    fn unmapped_tables_warn() {
        let table = Table::xm();
        let out = outcome(
            &Map::single(1020, "x-series", "X", "xm", &table)
                .not_mapped("ax_18f", "SameNumberAs(18)"),
            &[
                ("xm", table),
                ("ax_18f", Table::ax()),
                ("stray", Table::ax()),
            ],
        );
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        let [warning] = out.warnings.as_slice() else {
            panic!("{:?}", out.warnings);
        };
        assert_eq!(warning.file, "stray");
        assert!(matches!(warning.cause, Cause::Check(Check::Unmapped)));
    }
}
