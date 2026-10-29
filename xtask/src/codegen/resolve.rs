//! Resolves the map and the tables into what is rendered, and runs every
//! check of data-model §3 on the way. Problems are collected, never thrown,
//! so one run reports all of them.

use std::collections::{BTreeMap, BTreeSet};

use super::Diagnostic;
use super::names::{const_ident, module_ident, type_ident};
use super::schema::{ControlTable, ModelMap, ProtocolTag, Row, Source, TypeInfo};

impl ProtocolTag {
    /// `p1` / `p2`, as the map's list names.
    pub(crate) const fn list(self) -> &'static str {
        match self {
            Self::P1 => "p1",
            Self::P2 => "p2",
        }
    }

    /// `P1` / `P2`, as the generated `ProtocolVersion` variant.
    #[allow(dead_code, reason = "the lookup uses it from US2")]
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
    #[allow(dead_code, reason = "firmware_register uses it from US2")]
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
    entry: &'a super::schema::ModelEntry,
    appearances: Vec<(ProtocolTag, u8, Option<u8>)>,
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

    for (&number, entry) in &map.models {
        for (protocol, list) in [(ProtocolTag::P1, &entry.p1), (ProtocolTag::P2, &entry.p2)] {
            let Some(list) = list else { continue };
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
            registers: registers(file, table, &mut out.errors),
        });
    }
    out.resolved.models.sort_by(|a, b| {
        let first = |m: &GenModel| m.appearances.iter().map(|a| a.1).min();
        (a.number, first(a), &a.file).cmp(&(b.number, first(b), &b.file))
    });

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
    list: &[super::schema::TableRef],
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
