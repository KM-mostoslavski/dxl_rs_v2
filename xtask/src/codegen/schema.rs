//! The input schema (module-interfaces §3): `dynamixel.ron` and the control
//! tables, as serde types, and how to read them.
//!
//! Unknown fields are ignored, so metadata the tables module adds (legacy
//! `tables`, ...) needs no codegen change. `instructions` is read for the
//! Protocol 1 Bulk Read capability (specs/011-bus-fr9). The FR-9 fields
//! (`unit_source`, `unit_info`, `torque_control_mode`, per-mode
//! `operating_modes` lists, row `range`) are read; `unit_info` counts only
//! in a table with `unit_source` (specs/009-tables-fr9).

use std::collections::BTreeMap;
use std::path::Path;

use ron::extensions::Extensions;
use serde::Deserialize;

use super::diagnostic::{Cause, Diagnostic};

/// `dynamixel.ron`: model numbers and the tables each uses per protocol.
#[derive(Debug, Deserialize)]
pub(crate) struct ModelMap {
    /// Keys are model numbers.
    pub(crate) models: BTreeMap<u16, ModelEntry>,
    /// Tables on disk that no model number uses, on purpose. The legacy
    /// `tables` field of each entry is ignored.
    #[serde(default)]
    pub(crate) not_mapped: Vec<NotMapped>,
}

/// One model number in `dynamixel.ron`.
#[derive(Debug, Deserialize)]
pub(crate) struct ModelEntry {
    /// One of `ax`, `mx`, `rx`, `x-series`, `y-series`, `pro`.
    pub(crate) family: String,
    /// Display name, e.g. `XM430-W350`.
    pub(crate) name: String,
    /// Other product names sold under the number; docs only.
    #[serde(default)]
    pub(crate) also: Vec<String>,
    /// Tables on Protocol 1. At least one of `p1`, `p2` is present; a
    /// present list is non-empty.
    #[serde(default)]
    pub(crate) p1: Option<Vec<TableRef>>,
    /// Tables on Protocol 2.
    #[serde(default)]
    pub(crate) p2: Option<Vec<TableRef>>,
    /// Required and `P2` when both lists exist; absent otherwise.
    #[serde(default)]
    pub(crate) default: Option<ProtocolTag>,
}

/// A control table file and the first firmware version it applies to.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct TableRef {
    /// The first is 0, then strictly increasing within one list.
    pub(crate) from_fw: u8,
    /// The stem of an existing `control_tables/<file>.ron`.
    pub(crate) file: String,
}

/// A table file that no model number uses, on purpose.
#[derive(Debug, Deserialize)]
pub(crate) struct NotMapped {
    /// The file exists and is not in any `p1`/`p2` list.
    pub(crate) file: String,
    /// Why; parsed to check its shape, otherwise unused.
    #[allow(
        dead_code,
        reason = "parsed to check its shape; codegen does not use it"
    )]
    pub(crate) reason: NotMappedReason,
}

/// Why a table is in `not_mapped`.
#[derive(Debug, Deserialize)]
pub(crate) enum NotMappedReason {
    /// Its e-Manual page gives no model number.
    NoModelNumber,
    /// Its e-Manual page gives a number already in `models`.
    SameNumberAs(#[allow(dead_code, reason = "docs only")] u16),
}

/// A protocol in the map or a table's `protocols`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub(crate) enum ProtocolTag {
    /// Protocol 1.0.
    P1,
    /// Protocol 2.0.
    P2,
}

/// The part of a model's `.ron` file codegen reads.
#[derive(Debug, Deserialize)]
pub(crate) struct ControlTable {
    /// Whether the table was checked against the e-Manual.
    pub(crate) source: Source,
    /// The protocols the e-Manual serves this layout on.
    pub(crate) protocols: Vec<ProtocolTag>,
    /// The instructions the e-Manual lists for the model, per protocol;
    /// empty in a table that does not record them.
    #[serde(default)]
    pub(crate) instructions: Vec<InstructionList>,
    /// Whether the FR-9 data (`unit_info`, `torque_control_mode`, per-mode
    /// lists, row ranges) was checked against the e-Manual; absent in a
    /// table that has none of it.
    #[serde(default)]
    pub(crate) unit_source: Option<Source>,
    /// Present exactly when the table has an `Operating Mode` or `Control
    /// Mode` row (check 17).
    #[serde(default)]
    pub(crate) operating_modes: Option<OperatingModes>,
    /// MX Protocol 1 torque control switch (check 21).
    #[serde(default)]
    pub(crate) torque_control_mode: Option<TorqueControlMode>,
    /// Position scale.
    pub(crate) type_info: TypeInfo,
    /// Register units. Read only under `unit_source`; legacy entries of
    /// the other tables are parsed and ignored.
    #[serde(default)]
    pub(crate) unit_info: Vec<UnitEntry>,
    /// Register rows, in table order.
    pub(crate) control_table: Vec<Row>,
}

/// One `unit_info` entry (specs/009-tables-fr9/contracts/fr9-schema.md).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct UnitEntry {
    /// A row's exact name.
    pub(crate) data_name: String,
    /// The raw step, in `unit`.
    pub(crate) value: f64,
    /// `deg`, `rpm`, `mA`, `V`, `°C`, `%`, `ms` or `deg/s^2`.
    pub(crate) unit: String,
    /// `unsigned`, `twos_complement` or `direction_bit_N`.
    pub(crate) sign_type: String,
    /// The Protocol 1 angle-limit modes it applies in; absent = all.
    #[serde(default)]
    pub(crate) modes: Option<Vec<String>>,
    /// The Drive Mode profile it applies in; absent = both.
    #[serde(default)]
    pub(crate) profile: Option<String>,
    /// Raw 0 means no limit.
    #[serde(default)]
    pub(crate) zero_is_unlimited: bool,
    /// The raw range in these modes.
    #[serde(default)]
    pub(crate) range: Option<RawRange>,
    /// The row bounding the magnitude.
    #[serde(default)]
    pub(crate) bounded_by: Option<String>,
    /// The first firmware with the register.
    #[serde(default)]
    pub(crate) from_fw: Option<u8>,
}

/// One `instructions` entry: the instructions a model supports on one
/// protocol, from `from_fw` on.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct InstructionList {
    /// The protocol the list is for.
    pub(crate) protocol: ProtocolTag,
    /// The e-Manual page the list comes from; docs only.
    #[allow(
        dead_code,
        reason = "parsed to check its shape; codegen does not use it"
    )]
    pub(crate) source: String,
    /// The instructions.
    pub(crate) list: Vec<Instruction>,
    /// The first firmware the list applies to; absent = all.
    #[serde(default)]
    pub(crate) from_fw: Option<u8>,
}

impl InstructionList {
    /// Whether the list names `instruction` on `protocol` for every
    /// firmware.
    pub(crate) fn always_has(&self, protocol: ProtocolTag, instruction: Instruction) -> bool {
        self.protocol == protocol
            && self.from_fw.unwrap_or(0) == 0
            && self.list.contains(&instruction)
    }
}

/// An instruction name, as `control_tables/check_tables.py` lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[allow(
    dead_code,
    reason = "parsed to check the table; codegen reads only BulkRead"
)]
pub(crate) enum Instruction {
    /// `Ping`.
    Ping,
    /// `Read`.
    Read,
    /// `Write`.
    Write,
    /// `RegWrite`.
    RegWrite,
    /// `Action`.
    Action,
    /// `FactoryReset`.
    FactoryReset,
    /// `Reboot`.
    Reboot,
    /// `Clear`.
    Clear,
    /// `ControlTableBackup`.
    ControlTableBackup,
    /// `SyncRead`.
    SyncRead,
    /// `SyncWrite`.
    SyncWrite,
    /// `FastSyncRead`.
    FastSyncRead,
    /// `BulkRead`.
    BulkRead,
    /// `BulkWrite`.
    BulkWrite,
    /// `FastBulkRead`.
    FastBulkRead,
}

/// A raw range, both ends included.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) struct RawRange {
    /// The lowest raw value.
    pub(crate) min: i64,
    /// The highest raw value.
    pub(crate) max: i64,
}

/// `torque_control_mode`: a 1-byte switch that puts an angle-limit model in
/// torque (current) control, and the goal register it then follows.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct TorqueControlMode {
    /// The switch row's name.
    pub(crate) register: String,
    /// The value that turns torque control on.
    pub(crate) on: u8,
    /// The goal row's name.
    pub(crate) goal: String,
}

/// What each value of the mode register means (module-interfaces §3.1).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct OperatingModes {
    /// Values in which the EEPROM limits apply. Required, may be empty.
    pub(crate) bounded: Vec<u8>,
    /// Position values in which they do not. Required, may be empty.
    pub(crate) unbounded: Vec<u8>,
    /// Values in which Goal Position is ignored. Required, may be empty.
    pub(crate) no_position: Vec<u8>,
    /// Current control values; with `velocity` and `pwm` a split of
    /// `no_position`. The four per-mode lists come all together or not at
    /// all (check 20).
    #[serde(default)]
    pub(crate) current: Option<Vec<u8>>,
    /// Velocity control values.
    #[serde(default)]
    pub(crate) velocity: Option<Vec<u8>>,
    /// PWM control values.
    #[serde(default)]
    pub(crate) pwm: Option<Vec<u8>>,
    /// Current-based position values, a subset of `unbounded`.
    #[serde(default)]
    pub(crate) current_position: Option<Vec<u8>>,
}

/// A table's verification state.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) enum Source {
    /// Not yet checked against the e-Manual.
    Unverified,
    /// Checked against the page at `url` on `verified` (`YYYY-MM-DD`).
    Verified {
        /// The e-Manual page.
        url: String,
        /// The date of the check.
        verified: String,
    },
}

/// The position scale of a table: raw ticks to radians.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub(crate) struct TypeInfo {
    /// The tick at 0 rad; in `min..=max`.
    pub(crate) value_of_zero_radian_position: i32,
    /// The lowest tick; below `max`.
    pub(crate) value_of_min_radian_position: i32,
    /// The highest tick.
    pub(crate) value_of_max_radian_position: i32,
    /// The angle of the lowest tick; below `max_radian`.
    pub(crate) min_radian: f32,
    /// The angle of the highest tick.
    pub(crate) max_radian: f32,
}

/// One row of a control table.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct Row {
    /// Control-table address.
    pub(crate) address: u16,
    /// Width in bytes.
    pub(crate) size: u8,
    /// The e-Manual register name.
    pub(crate) data_name: String,
    /// Read only or read-write.
    pub(crate) access: Access,
    /// Memory area.
    pub(crate) area: Area,
    /// The raw range the e-Manual gives, under `unit_source`.
    #[serde(default)]
    pub(crate) range: Option<RawRange>,
}

/// Register access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub(crate) enum Access {
    /// Read only.
    R,
    /// Read and write.
    RW,
}

/// Register memory area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub(crate) enum Area {
    /// Kept across power cycles.
    #[serde(rename = "EEPROM")]
    Eeprom,
    /// Lost at power off.
    #[serde(rename = "RAM")]
    Ram,
    /// Kept across power cycles and writable with torque on (Y series).
    Hybrid,
}

/// Reads `dynamixel.ron`. The diagnostic names the file `dynamixel`.
pub(crate) fn parse_map(path: &Path) -> Result<ModelMap, Diagnostic> {
    parse_map_str(&read(path, "dynamixel")?)
}

/// Parses the text of `dynamixel.ron`.
pub(crate) fn parse_map_str(text: &str) -> Result<ModelMap, Diagnostic> {
    options()
        .from_str(text)
        .map_err(|e| Diagnostic::new("dynamixel", e))
}

/// Reads one control table, named `stem` in the diagnostic.
pub(crate) fn parse_table(path: &Path, stem: &str) -> Result<ControlTable, Diagnostic> {
    parse_table_str(&read(path, stem)?, stem)
}

/// Parses the text of one control table. Records written with braces are
/// read as structs; see [`braces_to_parens`].
pub(crate) fn parse_table_str(text: &str, stem: &str) -> Result<ControlTable, Diagnostic> {
    options()
        .from_str(&braces_to_parens(text))
        .map_err(|e| Diagnostic::new(stem, e))
}

fn read(path: &Path, stem: &str) -> Result<String, Diagnostic> {
    std::fs::read_to_string(path).map_err(|source| {
        Diagnostic::new(
            stem,
            Cause::Read {
                path: path.to_path_buf(),
                source,
            },
        )
    })
}

/// `IMPLICIT_SOME`, so optional fields (`p1`, `p2`, `default`,
/// `operating_modes`, `unit_source`, ...) are written without `Some(...)`.
fn options() -> ron::Options {
    ron::Options::default().with_default_extension(Extensions::IMPLICIT_SOME)
}

/// Rewrites `{ key: value }` to `( key: value )`, leaving strings alone.
///
/// The control tables write records with braces, which RON reads as maps
/// whose bare keys turn into `()`, losing every field name. Parentheses make
/// them structs. Braces never mean a real map in a control table.
pub(crate) fn braces_to_parens(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_string = false;
    let mut escaped = false;
    for c in text.chars() {
        out.push(match c {
            _ if in_string && escaped => {
                escaped = false;
                c
            }
            '\\' if in_string => {
                escaped = true;
                c
            }
            '"' => {
                in_string = !in_string;
                c
            }
            '{' if !in_string => '(',
            '}' if !in_string => ')',
            _ => c,
        });
    }
    out
}

#[cfg(test)]
#[allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    reason = "fixture angles copy table text"
)]
mod tests {
    use super::*;
    use crate::codegen::fixtures::{Dir, Table};

    const TYPE_INFO: &str = "type_info: { value_of_zero_radian_position: 2048, \
        value_of_max_radian_position: 4095, value_of_min_radian_position: 0, \
        min_radian: -3.14159265, max_radian: 3.14159265 },";

    const ROWS: &str = r#"control_table: [
        { address: 0, size: 2, data_name: "Model Number", access: R, area: EEPROM },
        { address: 116, size: 4, data_name: "Goal Position", access: RW, area: RAM },
        { address: 896, size: 1, data_name: "Hybrid Save", access: RW, area: Hybrid },
    ],"#;

    fn table(head: &str, rows: &str) -> Result<ControlTable, Diagnostic> {
        let dir = Dir::new();
        let path = dir.path().join("t.ron");
        std::fs::write(&path, format!("(\n{head}\n{TYPE_INFO}\n{rows}\n)")).unwrap();
        parse_table(&path, "t")
    }

    fn map(text: &str) -> Result<ModelMap, Diagnostic> {
        let dir = Dir::new();
        let path = dir.path().join("dynamixel.ron");
        std::fs::write(&path, text).unwrap();
        parse_map(&path)
    }

    fn err(result: Result<ControlTable, Diagnostic>) -> String {
        let diagnostic = result.unwrap_err();
        assert_eq!(diagnostic.file, "t");
        diagnostic.cause.to_string()
    }

    #[test]
    fn parses_unverified_table() {
        let t = table("source: Unverified, protocols: [P2],", ROWS).unwrap();
        assert_eq!(t.source, Source::Unverified);
        assert_eq!(t.protocols, [ProtocolTag::P2]);
        assert_eq!(t.type_info.value_of_zero_radian_position, 2048);
        assert_eq!(t.type_info.value_of_max_radian_position, 4095);
        assert_eq!(t.type_info.min_radian, -3.14159265_f32);
        assert_eq!(
            t.control_table[1],
            Row {
                address: 116,
                size: 4,
                data_name: "Goal Position".into(),
                access: Access::RW,
                area: Area::Ram,
                range: None,
            }
        );
    }

    #[test]
    fn parses_verified_source() {
        let t = table(
            r#"source: Verified(url: "https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/", verified: "2026-10-28"), protocols: [P1],"#,
            ROWS,
        )
        .unwrap();
        assert_eq!(
            t.source,
            Source::Verified {
                url: "https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/".into(),
                verified: "2026-10-28".into(),
            }
        );
    }

    #[test]
    fn row_without_access_fails() {
        let rows =
            r#"control_table: [{ address: 0, size: 2, data_name: "Model Number", area: EEPROM }],"#;
        assert!(err(table("source: Unverified, protocols: [P1],", rows)).contains("access"));
    }

    #[test]
    fn row_without_area_fails() {
        let rows =
            r#"control_table: [{ address: 0, size: 2, data_name: "Model Number", access: R }],"#;
        assert!(err(table("source: Unverified, protocols: [P1],", rows)).contains("area"));
    }

    #[test]
    fn table_without_source_fails() {
        assert!(err(table("protocols: [P1],", ROWS)).contains("source"));
    }

    #[test]
    fn table_without_protocols_fails() {
        assert!(err(table("source: Unverified,", ROWS)).contains("protocols"));
    }

    #[test]
    fn parses_operating_modes() {
        let t = table(
            "source: Unverified, protocols: [P2], operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16]),",
            ROWS,
        )
        .unwrap();
        assert_eq!(
            t.operating_modes,
            Some(OperatingModes {
                bounded: vec![3],
                unbounded: vec![4, 5],
                no_position: vec![0, 1, 16],
                current: None,
                velocity: None,
                pwm: None,
                current_position: None,
            })
        );
        let t = table(
            "source: Unverified, protocols: [P2], operating_modes: (bounded: [2], unbounded: [], no_position: [1]),",
            ROWS,
        )
        .unwrap();
        assert_eq!(t.operating_modes.unwrap().unbounded, Vec::<u8>::new());
    }

    #[test]
    fn absent_operating_modes_is_none() {
        let t = table("source: Unverified, protocols: [P1],", ROWS).unwrap();
        assert_eq!(t.operating_modes, None);
    }

    #[test]
    fn operating_modes_value_above_255_fails() {
        let message = err(table(
            "source: Unverified, protocols: [P2], operating_modes: (bounded: [300], unbounded: [], no_position: []),",
            ROWS,
        ));
        assert!(message.contains("out of bounds"), "{message}");
    }

    #[test]
    fn operating_modes_missing_key_fails() {
        let message = err(table(
            "source: Unverified, protocols: [P2], operating_modes: (bounded: [3], unbounded: []),",
            ROWS,
        ));
        assert!(message.contains("no_position"), "{message}");
    }

    #[test]
    fn reads_instructions_and_legacy_unit_info() {
        let head = r#"source: Unverified, protocols: [P1],
            instructions: [(protocol: P1, source: "x", list: [Ping, Read]),
                (protocol: P1, source: "x", list: [BulkRead], from_fw: 40)],
            unit_info: [{ data_name: "Position", value: 0.29, unit: "deg", sign_type: "unsigned" }],"#;
        let t = table(head, ROWS).unwrap();
        assert!(t.instructions[0].always_has(ProtocolTag::P1, Instruction::Read));
        assert!(!t.instructions[0].always_has(ProtocolTag::P2, Instruction::Read));
        assert!(!t.instructions[0].always_has(ProtocolTag::P1, Instruction::BulkRead));
        assert!(!t.instructions[1].always_has(ProtocolTag::P1, Instruction::BulkRead));
        assert_eq!(t.instructions[1].from_fw, Some(40));
        assert_eq!(t.unit_source, None);
        assert_eq!(t.unit_info[0].sign_type, "unsigned");
        assert_eq!(t.unit_info[0].modes, None);
        assert!(!t.unit_info[0].zero_is_unlimited);
    }

    #[test]
    fn parses_fr9_fields() {
        let head = r#"source: Unverified,
            unit_source: Verified(url: "https://emanual.robotis.com/docs/en/dxl/mx/mx-64/", verified: "2026-11-03"),
            protocols: [P1],
            operating_modes: (bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16],
                current: [0], velocity: [1], pwm: [16], current_position: [5]),
            torque_control_mode: (register: "Torque Ctrl Mode Enable", on: 1, goal: "Goal Torque"),
            unit_info: [{ data_name: "Moving Speed", value: 0.114, unit: "rpm",
                sign_type: "direction_bit_10", modes: ["wheel"], profile: "velocity_based",
                zero_is_unlimited: true, range: (min: 0, max: 2047),
                bounded_by: "Velocity Limit", from_fw: 40 }],"#;
        let rows = r#"control_table: [
            { address: 70, size: 1, data_name: "Torque Ctrl Mode Enable", access: RW, area: RAM, range: (min: 0, max: 1) },
        ],"#;
        let t = table(head, rows).unwrap();
        assert!(matches!(t.unit_source, Some(Source::Verified { .. })));
        let modes = t.operating_modes.unwrap();
        assert_eq!(
            (
                modes.current,
                modes.velocity,
                modes.pwm,
                modes.current_position
            ),
            (Some(vec![0]), Some(vec![1]), Some(vec![16]), Some(vec![5]))
        );
        assert_eq!(
            t.torque_control_mode,
            Some(TorqueControlMode {
                register: "Torque Ctrl Mode Enable".into(),
                on: 1,
                goal: "Goal Torque".into(),
            })
        );
        assert_eq!(
            t.unit_info[0],
            UnitEntry {
                data_name: "Moving Speed".into(),
                value: 0.114,
                unit: "rpm".into(),
                sign_type: "direction_bit_10".into(),
                modes: Some(vec!["wheel".into()]),
                profile: Some("velocity_based".into()),
                zero_is_unlimited: true,
                range: Some(RawRange { min: 0, max: 2047 }),
                bounded_by: Some("Velocity Limit".into()),
                from_fw: Some(40),
            }
        );
        assert_eq!(t.control_table[0].range, Some(RawRange { min: 0, max: 1 }));
    }

    #[test]
    fn area_names() {
        let t = table("source: Unverified, protocols: [P2],", ROWS).unwrap();
        let areas: Vec<Area> = t.control_table.iter().map(|r| r.area).collect();
        assert_eq!(areas, [Area::Eeprom, Area::Ram, Area::Hybrid]);
    }

    #[test]
    fn parses_map_with_implicit_some() {
        let m = map(r#"(
            models: {
                12: (family: "ax", name: "AX-12A", also: ["AX-12+"], p1: [(from_fw: 0, file: "ax_12a")]),
                1020: (family: "x-series", name: "XM430-W350",
                       p1: [(from_fw: 0, file: "xm430_w350")], p2: [(from_fw: 0, file: "xm430_w350")],
                       default: P2),
            },
            not_mapped: [
                (file: "ax_18f", reason: NoModelNumber),
                (file: "xw430_t200", reason: SameNumberAs(1270)),
            ],
        )"#)
        .unwrap();
        let ax = &m.models[&12];
        assert_eq!(ax.also, ["AX-12+"]);
        assert_eq!(ax.p1.as_ref().unwrap()[0].file, "ax_12a");
        assert!(ax.p2.is_none());
        assert_eq!(ax.default, None);
        let xm = &m.models[&1020];
        assert_eq!(xm.p2.as_ref().unwrap()[0].from_fw, 0);
        assert_eq!(xm.default, Some(ProtocolTag::P2));
        assert_eq!(m.not_mapped.len(), 2);
        assert!(matches!(
            m.not_mapped[0].reason,
            NotMappedReason::NoModelNumber
        ));
        assert!(matches!(
            m.not_mapped[1].reason,
            NotMappedReason::SameNumberAs(1270)
        ));
    }

    #[test]
    fn legacy_tables_field_is_ignored() {
        let m = map(r#"(models: { 12: (family: "ax", name: "AX-12A",
            p1: [(from_fw: 0, file: "ax_12a")], tables: [(from_fw: 0, file: "ax_12a")]) })"#)
        .unwrap();
        assert_eq!(m.models[&12].p1.as_ref().unwrap().len(), 1);
        assert!(m.not_mapped.is_empty());
    }

    #[test]
    fn empty_list_differs_from_absent() {
        let m = map(r#"(models: { 12: (family: "ax", name: "AX-12A", p1: []) })"#).unwrap();
        assert_eq!(m.models[&12].p1.as_ref().map(Vec::len), Some(0));
        assert!(m.models[&12].p2.is_none());
    }

    #[test]
    fn unreadable_file_names_the_stem() {
        let dir = Dir::new();
        let d = parse_table(&dir.path().join("gone.ron"), "gone").unwrap_err();
        assert_eq!(d.file, "gone");
        assert!(d.cause.to_string().contains("cannot read"));
        let d = parse_map(&dir.path().join("dynamixel.ron")).unwrap_err();
        assert_eq!(d.file, "dynamixel");
    }

    #[test]
    fn parse_errors_keep_position_and_code() {
        let d = parse_map_str("(models: {").unwrap_err();
        assert_eq!(d.file, "dynamixel");
        assert!(
            matches!(&d.cause, Cause::Parse { line: 1, column: 10, code } if **code == ron::Error::Eof),
            "{d:?}"
        );
        assert_eq!(d.cause.to_string(), "1:10: Unexpected end of RON");

        let text = Table::xm().render().replace(
            "data_name: \"Goal Position\", access: RW, area: RAM",
            "data_name: \"Goal Position\", access: RW, area: RAMX",
        );
        let (line, row) = text
            .lines()
            .enumerate()
            .find(|(_, l)| l.contains("RAMX"))
            .unwrap();
        let column = row.find("RAMX").unwrap() + 1;
        let d = parse_table_str(&text, "xm").unwrap_err();
        assert_eq!(d.file, "xm");
        let Cause::Parse {
            line: got_line,
            column: got_column,
            code,
        } = &d.cause
        else {
            panic!("{d:?}");
        };
        assert_eq!((*got_line, *got_column), (line + 1, column));
        assert!(
            matches!(&**code, ron::Error::NoSuchEnumVariant { found, .. } if found == "RAMX"),
            "{code:?}"
        );
    }

    #[test]
    fn braces_become_parens_outside_strings() {
        assert_eq!(
            braces_to_parens(r#"{ a: "{x}", b: "q\"}" }"#),
            r#"( a: "{x}", b: "q\"}" )"#
        );
    }
}
