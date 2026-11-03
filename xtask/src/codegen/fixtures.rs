//! Test fixtures: a self-cleaning temp directory, builders for T1-shaped
//! tables and maps, a temp workspace root, and one failing root per check.

#![allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    reason = "fixture angles copy table text"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::SystemTime;

use super::resolve::{Outcome, Resolved, resolve};
use super::schema::{parse_map_str, parse_table_str};

/// Minimal self-cleaning temp directory, to avoid a dependency for tests.
pub(crate) struct Dir(PathBuf);

impl Dir {
    /// A new empty directory under the system temp dir.
    pub(crate) fn new() -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!(
            "xtask-codegen-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    /// The directory.
    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// One fixture row: address, size, name, access, area.
type RowSpec = (u16, u8, String, &'static str, &'static str);

/// A T1-shaped control table, rendered as RON with braces like the real
/// tables.
#[derive(Debug, Clone)]
pub(crate) struct Table {
    source: Option<String>,
    unit_source: Option<String>,
    protocols: Option<String>,
    operating_modes: Option<String>,
    torque_control_mode: Option<String>,
    type_info: (i32, i32, i32, f32, f32),
    units: Vec<String>,
    rows: Vec<RowSpec>,
    ranges: BTreeMap<String, (i64, i64)>,
}

/// The `unit_source` of the FR-9 fixtures.
pub(crate) const UNIT_SOURCE: &str = "Verified(url: \"https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/\", verified: \"2026-11-03\")";

impl Table {
    fn new(protocols: &str, type_info: (i32, i32, i32, f32, f32), rows: &[RowSpec]) -> Self {
        Self {
            source: Some("Unverified".to_string()),
            unit_source: None,
            protocols: Some(protocols.to_string()),
            operating_modes: None,
            torque_control_mode: None,
            type_info,
            units: Vec::new(),
            rows: rows.to_vec(),
            ranges: BTreeMap::new(),
        }
    }

    /// An X-series layout on P1 and P2: min/max limits, Operating Mode,
    /// Hardware Error Status.
    pub(crate) fn xm() -> Self {
        let mut t = Self::new(
            "[P1, P2]",
            (2048, 0, 4095, -3.14159265, 3.14159265),
            &[
                (0, 2, "Model Number".into(), "R", "EEPROM"),
                (6, 1, "Firmware Version".into(), "R", "EEPROM"),
                (11, 1, "Operating Mode".into(), "RW", "EEPROM"),
                (48, 4, "Max Position Limit".into(), "RW", "EEPROM"),
                (52, 4, "Min Position Limit".into(), "RW", "EEPROM"),
                (64, 1, "Torque Enable".into(), "RW", "RAM"),
                (70, 1, "Hardware Error Status".into(), "R", "RAM"),
                (116, 4, "Goal Position".into(), "RW", "RAM"),
                (132, 4, "Present Position".into(), "R", "RAM"),
            ],
        );
        t.operating_modes =
            Some("(bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16])".into());
        t
    }

    /// An AX layout on P1: CW/CCW limits, no fault register, no mode row.
    pub(crate) fn ax() -> Self {
        Self::new(
            "[P1]",
            (512, 0, 1023, -2.6205530, 2.6154347),
            &[
                (0, 2, "Model Number".into(), "R", "EEPROM"),
                (2, 1, "Firmware Version".into(), "R", "EEPROM"),
                (6, 2, "CW Angle Limit".into(), "RW", "EEPROM"),
                (8, 2, "CCW Angle Limit".into(), "RW", "EEPROM"),
                (24, 1, "Torque Enable".into(), "RW", "RAM"),
                (30, 2, "Goal Position".into(), "RW", "RAM"),
                (36, 2, "Present Position".into(), "R", "RAM"),
            ],
        )
    }

    /// The XL-320 layout on P2: CW/CCW limits beside a Control Mode row.
    pub(crate) fn xl320() -> Self {
        let mut t = Self::new(
            "[P2]",
            (512, 0, 1023, -2.6205530, 2.6154347),
            &[
                (0, 2, "Model Number".into(), "R", "EEPROM"),
                (2, 1, "Firmware Version".into(), "R", "EEPROM"),
                (6, 2, "CW Angle Limit".into(), "RW", "EEPROM"),
                (8, 2, "CCW Angle Limit".into(), "RW", "EEPROM"),
                (11, 1, "Control Mode".into(), "RW", "EEPROM"),
                (24, 1, "Torque Enable".into(), "RW", "RAM"),
                (30, 2, "Goal Position".into(), "RW", "RAM"),
                (37, 2, "Present Position".into(), "R", "RAM"),
                (50, 1, "Hardware Error Status".into(), "R", "RAM"),
            ],
        );
        t.operating_modes = Some("(bounded: [2], unbounded: [], no_position: [1])".into());
        t
    }

    /// [`Table::ax`] with the FR-9 rows of an AX-12A and their units, as
    /// the verified AX-12A table gives them.
    pub(crate) fn ax_fr9() -> Self {
        Self::ax()
            .with_row(25, 1, "LED", "RW", "RAM")
            .with_row(26, 1, "CW Compliance Margin", "RW", "RAM")
            .with_row(27, 1, "CCW Compliance Margin", "RW", "RAM")
            .with_row(28, 1, "CW Compliance Slope", "RW", "RAM")
            .with_row(29, 1, "CCW Compliance Slope", "RW", "RAM")
            .with_row(32, 2, "Moving Speed", "RW", "RAM")
            .with_row(38, 2, "Present Speed", "R", "RAM")
            .with_row(40, 2, "Present Load", "R", "RAM")
            .with_row(42, 1, "Present Voltage", "R", "RAM")
            .with_row(43, 1, "Present Temperature", "R", "RAM")
            .with_row(46, 1, "Moving", "R", "RAM")
            .row_range("CW Compliance Slope", 0, 254)
            .unit_source(UNIT_SOURCE)
            .unit(r#"data_name: "CW Compliance Margin", value: 0.29, unit: "deg", sign_type: "unsigned", range: (min: 0, max: 255)"#)
            .unit(r#"data_name: "Moving Speed", value: 0.111, unit: "rpm", sign_type: "unsigned", modes: ["joint"], zero_is_unlimited: true, range: (min: 0, max: 1023)"#)
            .unit(r#"data_name: "Moving Speed", value: 0.1, unit: "%", sign_type: "direction_bit_10", modes: ["wheel"], range: (min: 0, max: 2047)"#)
            .unit(r#"data_name: "Present Speed", value: 0.111, unit: "rpm", sign_type: "direction_bit_10", modes: ["joint"]"#)
            .unit(r#"data_name: "Present Speed", value: 0.1, unit: "%", sign_type: "direction_bit_10", modes: ["wheel"]"#)
            .unit(r#"data_name: "Present Load", value: 0.1, unit: "%", sign_type: "direction_bit_10", range: (min: 0, max: 2047)"#)
            .unit(r#"data_name: "Present Voltage", value: 0.1, unit: "V", sign_type: "unsigned""#)
            .unit(r#"data_name: "Present Temperature", value: 1.0, unit: "°C", sign_type: "unsigned""#)
    }

    /// [`Table::ax`] as an MX-64 on Protocol 1: wheel-mode speed, Realtime
    /// Tick from firmware 40, and the torque control switch.
    pub(crate) fn mx_p1() -> Self {
        Self::ax()
            .with_row(28, 1, "P Gain", "RW", "RAM")
            .with_row(32, 2, "Moving Speed", "RW", "RAM")
            .with_row(50, 2, "Realtime Tick", "R", "RAM")
            .with_row(70, 1, "Torque Ctrl Mode Enable", "RW", "RAM")
            .with_row(71, 2, "Goal Torque", "RW", "RAM")
            .row_range("Torque Ctrl Mode Enable", 0, 1)
            .row_range("P Gain", 0, 254)
            .unit_source(UNIT_SOURCE)
            .torque_control_mode(r#"(register: "Torque Ctrl Mode Enable", on: 1, goal: "Goal Torque")"#)
            .unit(r#"data_name: "Moving Speed", value: 0.114, unit: "rpm", sign_type: "unsigned", modes: ["joint", "multi_turn"], zero_is_unlimited: true"#)
            .unit(r#"data_name: "Moving Speed", value: 0.114, unit: "rpm", sign_type: "direction_bit_10", modes: ["wheel"]"#)
            .unit(r#"data_name: "Realtime Tick", value: 1.0, unit: "ms", sign_type: "unsigned", range: (min: 0, max: 32767), from_fw: 40"#)
            .unit(r#"data_name: "Goal Torque", value: 4.5, unit: "mA", sign_type: "direction_bit_10", range: (min: 0, max: 2047)"#)
    }

    /// [`Table::xm`] with the FR-9 rows and units of an XM430-W350 and its
    /// per-mode lists.
    pub(crate) fn xm_fr9() -> Self {
        Self::xm()
            .with_row(38, 2, "Current Limit", "RW", "EEPROM")
            .with_row(44, 4, "Velocity Limit", "RW", "EEPROM")
            .with_row(65, 1, "LED", "RW", "RAM")
            .with_row(102, 2, "Goal Current", "RW", "RAM")
            .with_row(104, 4, "Goal Velocity", "RW", "RAM")
            .with_row(112, 4, "Profile Velocity", "RW", "RAM")
            .with_row(126, 2, "Present Current", "R", "RAM")
            .with_row(128, 4, "Present Velocity", "R", "RAM")
            .with_row(144, 2, "Present Input Voltage", "R", "RAM")
            .with_row(146, 1, "Present Temperature", "R", "RAM")
            .unit_source(UNIT_SOURCE)
            .operating_modes("(bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16], current: [0], velocity: [1], pwm: [16], current_position: [5])")
            .unit(r#"data_name: "Current Limit", value: 2.69, unit: "mA", sign_type: "unsigned""#)
            .unit(r#"data_name: "Velocity Limit", value: 0.229, unit: "rpm", sign_type: "unsigned""#)
            .unit(r#"data_name: "Goal Current", value: 2.69, unit: "mA", sign_type: "twos_complement", bounded_by: "Current Limit""#)
            .unit(r#"data_name: "Goal Velocity", value: 0.229, unit: "rpm", sign_type: "twos_complement", bounded_by: "Velocity Limit""#)
            .unit(r#"data_name: "Profile Velocity", value: 0.229, unit: "rpm", sign_type: "unsigned", profile: "velocity_based", zero_is_unlimited: true, range: (min: 0, max: 32767)"#)
            .unit(r#"data_name: "Profile Velocity", value: 1.0, unit: "ms", sign_type: "unsigned", profile: "time_based", from_fw: 42"#)
            .unit(r#"data_name: "Present Current", value: 2.69, unit: "mA", sign_type: "twos_complement""#)
            .unit(r#"data_name: "Present Velocity", value: 0.229, unit: "rpm", sign_type: "twos_complement""#)
            .unit(r#"data_name: "Present Input Voltage", value: 0.1, unit: "V", sign_type: "unsigned""#)
            .unit(r#"data_name: "Present Temperature", value: 1.0, unit: "°C", sign_type: "unsigned""#)
    }

    /// Sets the `unit_source` text, e.g. [`UNIT_SOURCE`].
    pub(crate) fn unit_source(mut self, text: &str) -> Self {
        self.unit_source = Some(text.to_string());
        self
    }

    /// Appends one `unit_info` entry; `fields` is the text inside its
    /// braces.
    pub(crate) fn unit(mut self, fields: &str) -> Self {
        self.units.push(fields.to_string());
        self
    }

    /// Removes every `unit_info` entry for the row named `name`.
    pub(crate) fn without_unit(mut self, name: &str) -> Self {
        let key = format!("data_name: {name:?},");
        self.units.retain(|fields| !fields.starts_with(&key));
        self
    }

    /// Sets the `torque_control_mode` text.
    pub(crate) fn torque_control_mode(mut self, text: &str) -> Self {
        self.torque_control_mode = Some(text.to_string());
        self
    }

    /// Gives the row named `name` a `range`.
    pub(crate) fn row_range(mut self, name: &str, min: i64, max: i64) -> Self {
        self.ranges.insert(name.to_string(), (min, max));
        self
    }

    /// Removes the range of the row named `name`.
    pub(crate) fn without_range(mut self, name: &str) -> Self {
        self.ranges.remove(name);
        self
    }

    /// Removes the row named `name`, and its range.
    pub(crate) fn without(mut self, name: &str) -> Self {
        self.rows.retain(|row| row.2 != name);
        self.ranges.remove(name);
        self
    }

    /// Appends a row.
    pub(crate) fn with_row(
        mut self,
        address: u16,
        size: u8,
        name: &str,
        access: &'static str,
        area: &'static str,
    ) -> Self {
        self.rows
            .push((address, size, name.to_string(), access, area));
        self
    }

    /// Sets the `protocols` text, e.g. `[P1]`.
    pub(crate) fn protocols(mut self, text: &str) -> Self {
        self.protocols = Some(text.to_string());
        self
    }

    /// Sets the `source` text, e.g. `Verified(url: "..", verified: "..")`.
    pub(crate) fn source(mut self, text: &str) -> Self {
        self.source = Some(text.to_string());
        self
    }

    /// Sets `type_info`.
    pub(crate) fn type_info(
        mut self,
        zero: i32,
        min: i32,
        max: i32,
        min_rad: f32,
        max_rad: f32,
    ) -> Self {
        self.type_info = (zero, min, max, min_rad, max_rad);
        self
    }

    /// Sets the `operating_modes` text, e.g.
    /// `(bounded: [3], unbounded: [], no_position: [])`.
    pub(crate) fn operating_modes(mut self, text: &str) -> Self {
        self.operating_modes = Some(text.to_string());
        self
    }

    /// Leaves out `source`, `unit_source`, `protocols` or
    /// `operating_modes`.
    pub(crate) fn omit_field(mut self, field: &str) -> Self {
        match field {
            "source" => self.source = None,
            "unit_source" => self.unit_source = None,
            "protocols" => self.protocols = None,
            "operating_modes" => self.operating_modes = None,
            _ => panic!("unknown field {field}"),
        }
        self
    }

    /// Whether `protocols` names P1, and P2.
    pub(crate) fn lists(&self) -> (bool, bool) {
        let text = self.protocols.as_deref().unwrap_or("");
        (text.contains("P1"), text.contains("P2"))
    }

    /// The fixture's radians, as the table gives them.
    pub(crate) fn radians(&self) -> (f32, f32) {
        (self.type_info.3, self.type_info.4)
    }

    /// The RON text.
    pub(crate) fn render(&self) -> String {
        let mut out = String::from("(\n");
        if let Some(source) = &self.source {
            let _ = writeln!(out, "  source: {source},");
        }
        if let Some(source) = &self.unit_source {
            let _ = writeln!(out, "  unit_source: {source},");
        }
        if let Some(protocols) = &self.protocols {
            let _ = writeln!(out, "  protocols: {protocols},");
        }
        if let Some(modes) = &self.operating_modes {
            let _ = writeln!(out, "  operating_modes: {modes},");
        }
        if let Some(mode) = &self.torque_control_mode {
            let _ = writeln!(out, "  torque_control_mode: {mode},");
        }
        let (zero, min, max, min_rad, max_rad) = self.type_info;
        let _ = writeln!(
            out,
            "  type_info: {{\n    value_of_zero_radian_position: {zero},\n    \
             value_of_max_radian_position: {max},\n    value_of_min_radian_position: {min},\n    \
             min_radian: {min_rad:?},\n    max_radian: {max_rad:?},\n  }},"
        );
        out.push_str("  unit_info: [\n");
        for fields in &self.units {
            let _ = writeln!(out, "    {{ {fields} }},");
        }
        out.push_str("  ],\n  control_table: [\n");
        for (address, size, name, access, area) in &self.rows {
            let range = self
                .ranges
                .get(name)
                .map_or_else(String::new, |(min, max)| {
                    format!(", range: (min: {min}, max: {max})")
                });
            let _ = writeln!(
                out,
                "    {{ address: {address}, size: {size}, data_name: {name:?}, access: {access}, area: {area}{range} }},"
            );
        }
        out.push_str("  ],\n)\n");
        out
    }
}

#[derive(Debug, Clone, Default)]
struct ModelSpec {
    number: u16,
    family: String,
    name: String,
    also: Vec<String>,
    p1: Option<Vec<(u8, String)>>,
    p2: Option<Vec<(u8, String)>>,
    default: Option<&'static str>,
    legacy: Option<Vec<(u8, String)>>,
}

/// A `dynamixel.ron` builder. `p1`, `p2`, `default_p2`, `also` and
/// `legacy_tables` apply to the last `model`.
#[derive(Debug, Clone, Default)]
pub(crate) struct Map {
    models: Vec<ModelSpec>,
    not_mapped: Vec<(String, String)>,
}

fn list(tables: &[(u8, &str)]) -> Vec<(u8, String)> {
    tables
        .iter()
        .map(|&(fw, file)| (fw, file.to_string()))
        .collect()
}

impl Map {
    /// An empty map.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// A map with one model whose lists name `stem` under each protocol
    /// `table` has (`p2` alone when it has none).
    pub(crate) fn single(number: u16, family: &str, name: &str, stem: &str, table: &Table) -> Self {
        let mut map = Self::new().model(number, family, name);
        let (p1, p2) = table.lists();
        if p1 {
            map = map.p1(&[(0, stem)]);
        }
        if p2 || !p1 {
            map = map.p2(&[(0, stem)]);
        }
        if p1 && p2 {
            map = map.default_p2();
        }
        map
    }

    /// Starts a model entry.
    pub(crate) fn model(mut self, number: u16, family: &str, name: &str) -> Self {
        self.models.push(ModelSpec {
            number,
            family: family.to_string(),
            name: name.to_string(),
            ..ModelSpec::default()
        });
        self
    }

    fn last(&mut self) -> &mut ModelSpec {
        self.models.last_mut().expect("call model() first")
    }

    /// Sets the `p1` list.
    pub(crate) fn p1(mut self, tables: &[(u8, &str)]) -> Self {
        self.last().p1 = Some(list(tables));
        self
    }

    /// Sets the `p2` list.
    pub(crate) fn p2(mut self, tables: &[(u8, &str)]) -> Self {
        self.last().p2 = Some(list(tables));
        self
    }

    /// Sets `default: P2`.
    pub(crate) fn default_p2(mut self) -> Self {
        self.last().default = Some("P2");
        self
    }

    /// Sets `default: P1`.
    pub(crate) fn default_p1(mut self) -> Self {
        self.last().default = Some("P1");
        self
    }

    /// Sets `also`.
    pub(crate) fn also(mut self, names: &[&str]) -> Self {
        self.last().also = names.iter().map(ToString::to_string).collect();
        self
    }

    /// Sets the legacy `tables` list.
    pub(crate) fn legacy_tables(mut self, tables: &[(u8, &str)]) -> Self {
        self.last().legacy = Some(list(tables));
        self
    }

    /// Adds a `not_mapped` record; `reason` is its RON text.
    pub(crate) fn not_mapped(mut self, file: &str, reason: &str) -> Self {
        self.not_mapped.push((file.to_string(), reason.to_string()));
        self
    }

    /// The RON text.
    pub(crate) fn render(&self) -> String {
        let render_list = |l: &[(u8, String)]| {
            let items: Vec<String> = l
                .iter()
                .map(|(fw, file)| format!("(from_fw: {fw}, file: {file:?})"))
                .collect();
            format!("[{}]", items.join(", "))
        };
        let mut out = String::from("(\n  models: {\n");
        for m in &self.models {
            let mut fields = vec![
                format!("family: {:?}", m.family),
                format!("name: {:?}", m.name),
            ];
            if !m.also.is_empty() {
                fields.push(format!("also: {:?}", m.also));
            }
            if let Some(l) = &m.p1 {
                fields.push(format!("p1: {}", render_list(l)));
            }
            if let Some(l) = &m.p2 {
                fields.push(format!("p2: {}", render_list(l)));
            }
            if let Some(d) = m.default {
                fields.push(format!("default: {d}"));
            }
            if let Some(l) = &m.legacy {
                fields.push(format!("tables: {}", render_list(l)));
            }
            let _ = writeln!(out, "    {}: ({}),", m.number, fields.join(", "));
        }
        out.push_str("  },\n  not_mapped: [\n");
        for (file, reason) in &self.not_mapped {
            let _ = writeln!(out, "    (file: {file:?}, reason: {reason}),");
        }
        out.push_str("  ],\n)\n");
        out
    }
}

/// Paths, bytes and modification times of the files under `src/models*`.
pub(crate) type Snapshot = Vec<(PathBuf, Vec<u8>, SystemTime)>;

/// A temp workspace root with `control_tables/` and `src/`.
pub(crate) struct Root {
    dir: Dir,
}

impl Root {
    /// Writes `map` and `tables` under `control_tables/`, and creates `src/`.
    pub(crate) fn new(map: &Map, tables: &[(&str, Table)]) -> Self {
        let root = Self { dir: Dir::new() };
        std::fs::create_dir_all(root.control_tables()).unwrap();
        std::fs::create_dir_all(root.path().join("src")).unwrap();
        root.write("dynamixel", &map.render());
        for (stem, table) in tables {
            root.write(stem, &table.render());
        }
        root
    }

    /// The workspace root.
    pub(crate) fn path(&self) -> &Path {
        self.dir.path()
    }

    /// `control_tables/`.
    pub(crate) fn control_tables(&self) -> PathBuf {
        self.path().join("control_tables")
    }

    /// Writes `control_tables/<stem>.ron`.
    pub(crate) fn write(&self, stem: &str, text: &str) {
        std::fs::write(self.control_tables().join(format!("{stem}.ron")), text).unwrap();
    }

    /// Writes `src/models.rs` and `src/models/old.rs` with known bytes and
    /// returns a snapshot of `src/models*`.
    pub(crate) fn sentinels(&self) -> Snapshot {
        let src = self.path().join("src");
        std::fs::create_dir_all(src.join("models")).unwrap();
        std::fs::write(src.join("models.rs"), "// sentinel models.rs\n").unwrap();
        std::fs::write(src.join("models/old.rs"), "// sentinel old.rs\n").unwrap();
        self.snapshot()
    }

    /// The files under `src/models*` now.
    pub(crate) fn snapshot(&self) -> Snapshot {
        let src = self.path().join("src");
        let mut paths = vec![src.join("models.rs")];
        if let Ok(entries) = std::fs::read_dir(src.join("models")) {
            paths.extend(entries.map(|e| e.unwrap().path()));
        }
        let mut snapshot: Snapshot = paths
            .into_iter()
            .filter(|p| p.is_file())
            .map(|p| {
                let bytes = std::fs::read(&p).unwrap();
                let modified = std::fs::metadata(&p).unwrap().modified().unwrap();
                (p, bytes, modified)
            })
            .collect();
        snapshot.sort();
        snapshot
    }

    /// Asserts `src/models*` still holds exactly `snapshot`, bytes and
    /// modification times included.
    pub(crate) fn assert_unchanged(&self, snapshot: &Snapshot) {
        assert_eq!(&self.snapshot(), snapshot);
    }
}

/// A root that fails exactly one check of data-model §3, or one FR-9
/// check (18 to 22, `fr9`).
pub(crate) struct Failing {
    /// The check number, 1 to 22.
    pub check: u8,
    /// The root.
    pub root: Root,
    /// The diagnostic's file stem.
    pub file: &'static str,
    /// Words the diagnostic's message contains.
    pub keywords: &'static [&'static str],
}

/// A valid one-model root: `xm` under 1020 on P1 and P2.
pub(crate) fn xm_map() -> Map {
    Map::new()
        .model(1020, "x-series", "XM430-W350")
        .p1(&[(0, "xm")])
        .p2(&[(0, "xm")])
        .default_p2()
}

/// Resolves `map` and `tables` in memory; every table is on disk.
pub(crate) fn outcome(map: &Map, tables: &[(&str, Table)]) -> Outcome {
    let map = parse_map_str(&map.render()).unwrap();
    let mut parsed = BTreeMap::new();
    let mut on_disk = BTreeSet::new();
    for (stem, table) in tables {
        on_disk.insert((*stem).to_string());
        parsed.insert(
            (*stem).to_string(),
            parse_table_str(&table.render(), stem).unwrap(),
        );
    }
    resolve(&map, &parsed, &on_disk)
}

/// Resolves `map` and `tables`, which must pass every check.
pub(crate) fn resolved(map: &Map, tables: &[(&str, Table)]) -> Resolved {
    let out = outcome(map, tables);
    assert!(out.errors.is_empty(), "{:?}", out.errors);
    out.resolved
}

/// A one-model root for `table`, saved as `xm`.
fn xm_root(table: Table) -> Root {
    Root::new(
        &Map::single(1020, "x-series", "XM430-W350", "xm", &table),
        &[("xm", table)],
    )
}

/// One failing root per check of data-model §3.
pub(crate) fn failing() -> Vec<Failing> {
    vec![
        Failing {
            check: 1,
            root: Root::new(&xm_map(), &[]),
            file: "xm",
            keywords: &["cannot read"],
        },
        Failing {
            check: 2,
            root: Root::new(
                &Map::new()
                    .model(1020, "x-series", "XM430-W350")
                    .p2(&[(1, "xm")]),
                &[("xm", Table::xm().protocols("[P2]"))],
            ),
            file: "dynamixel",
            keywords: &["model 1020", "p2 from_fw must start at 0"],
        },
        Failing {
            check: 3,
            root: Root::new(
                &Map::new()
                    .model(1020, "x-series", "XM430-W350")
                    .p2(&[(0, "xm")])
                    .model(1030, "x-series", "XM430-W210"),
                &[("xm", Table::xm().protocols("[P2]"))],
            ),
            file: "dynamixel",
            keywords: &["model 1030", "no p1 or p2 list"],
        },
        Failing {
            check: 4,
            root: Root::new(
                &Map::new()
                    .model(1020, "x-series", "XM430-W350")
                    .p1(&[(0, "xm")])
                    .p2(&[(0, "xm")]),
                &[("xm", Table::xm())],
            ),
            file: "dynamixel",
            keywords: &["model 1020", "default: P2 is required"],
        },
        Failing {
            check: 5,
            root: Root::new(
                &Map::new()
                    .model(1020, "x_series", "XM430-W350")
                    .p2(&[(0, "xm")]),
                &[("xm", Table::xm().protocols("[P2]"))],
            ),
            file: "dynamixel",
            keywords: &["model 1020", "unknown family \"x_series\""],
        },
        Failing {
            check: 6,
            root: Root::new(
                &Map::new()
                    .model(1020, "x-series", "XM430-W350")
                    .p2(&[(0, "xm")])
                    .model(1030, "x-series", "XM430-W210")
                    .p2(&[(0, "xm")]),
                &[("xm", Table::xm().protocols("[P2]"))],
            ),
            file: "dynamixel",
            keywords: &["table xm", "model 1020", "model 1030"],
        },
        Failing {
            check: 7,
            root: Root::new(
                &xm_map().not_mapped("gone", "NoModelNumber"),
                &[("xm", Table::xm())],
            ),
            file: "dynamixel",
            keywords: &["not_mapped names gone"],
        },
        Failing {
            check: 8,
            root: xm_root(Table::xm().omit_field("protocols")),
            file: "xm",
            keywords: &["protocols"],
        },
        Failing {
            check: 9,
            root: Root::new(
                &Map::new()
                    .model(1020, "x-series", "XM430-W350")
                    .p1(&[(0, "xm")]),
                &[("xm", Table::xm().protocols("[P2]"))],
            ),
            file: "xm",
            keywords: &["listed under p1", "lack P1"],
        },
        Failing {
            check: 12,
            root: xm_root(Table::xm().without("Model Number").with_row(
                2,
                2,
                "Model Number",
                "R",
                "EEPROM",
            )),
            file: "xm",
            keywords: &["Model Number must be at address 0, size 2"],
        },
        Failing {
            check: 13,
            root: Root::new(
                &Map::new()
                    .model(1190, "x-series", "XL330-M077")
                    .p2(&[(0, "old"), (53, "new")]),
                &[
                    ("old", Table::xm().protocols("[P2]")),
                    (
                        "new",
                        Table::xm()
                            .protocols("[P2]")
                            .without("Firmware Version")
                            .with_row(2, 1, "Firmware Version", "R", "EEPROM"),
                    ),
                ],
            ),
            file: "dynamixel",
            keywords: &["model 1190", "old", "new", "Firmware Version"],
        },
        Failing {
            check: 14,
            root: xm_root(Table::xm().without("Goal Position").with_row(
                564,
                4,
                "Goal Position",
                "RW",
                "RAM",
            )),
            file: "xm",
            keywords: &["listed under p1 but Goal Position is at address 564 (> 255)"],
        },
        Failing {
            check: 10,
            root: xm_root(Table::xm().without("Goal Position")),
            file: "xm",
            keywords: &["missing role Goal Position"],
        },
        Failing {
            check: 11,
            root: xm_root(Table::xm().without("Torque Enable").with_row(
                64,
                1,
                "Torque Enable",
                "R",
                "RAM",
            )),
            file: "xm",
            keywords: &["Torque Enable", "must be RW/RAM, found R/RAM"],
        },
        Failing {
            check: 15,
            root: xm_root(Table::xm().type_info(2048, 4095, 0, -1.0, 1.0)),
            file: "xm",
            keywords: &["value_of_max_radian_position"],
        },
        Failing {
            check: 16,
            root: xm_root(
                Table::xm()
                    .with_row(65, 1, "LED", "RW", "RAM")
                    .with_row(66, 1, "Led", "RW", "RAM"),
            ),
            file: "xm",
            keywords: &["\"LED\"", "\"Led\""],
        },
        Failing {
            check: 17,
            root: xm_root(Table::xm().omit_field("operating_modes")),
            file: "xm",
            keywords: &["\"Operating Mode\" row without operating_modes"],
        },
        Failing {
            check: 18,
            root: xm_root(Table::xm_fr9().unit_source("Unverified")),
            file: "xm",
            keywords: &["unit_source must be Verified"],
        },
        Failing {
            check: 19,
            root: xm_root(Table::xm_fr9().unit(
                r#"data_name: "Goal PWM", value: 0.113, unit: "%", sign_type: "twos_complement""#,
            )),
            file: "xm",
            keywords: &["unit_info \"Goal PWM\"", "names no row"],
        },
        Failing {
            check: 20,
            root: xm_root(Table::xm_fr9().operating_modes(
                "(bounded: [3], unbounded: [4, 5], no_position: [0, 1, 16], current: [0])",
            )),
            file: "xm",
            keywords: &["operating_modes", "all together"],
        },
        Failing {
            check: 21,
            root: Root::new(
                &Map::single(12, "ax", "MX-64", "ax", &Table::mx_p1()),
                &[(
                    "ax",
                    Table::mx_p1().torque_control_mode(
                        r#"(register: "Torque Mode", on: 1, goal: "Goal Torque")"#,
                    ),
                )],
            ),
            file: "ax",
            keywords: &["torque_control_mode", "\"Torque Mode\" names no row"],
        },
        Failing {
            check: 22,
            root: xm_root(Table::xm_fr9().with_row(42, 1, "Present Voltage", "R", "RAM")),
            file: "xm",
            keywords: &["role present_voltage has two rows"],
        },
    ]
}
