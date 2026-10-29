//! Test fixtures: a self-cleaning temp directory, builders for T1-shaped
//! tables and maps, a temp workspace root, and one failing root per check.

#![allow(
    dead_code,
    clippy::approx_constant,
    clippy::excessive_precision,
    reason = "builders are used as the stories add tests; fixture angles copy table text"
)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::SystemTime;

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
    protocols: Option<String>,
    operating_modes: Option<String>,
    type_info: (i32, i32, i32, f32, f32),
    rows: Vec<RowSpec>,
}

impl Table {
    fn new(protocols: &str, type_info: (i32, i32, i32, f32, f32), rows: &[RowSpec]) -> Self {
        Self {
            source: Some("Unverified".to_string()),
            protocols: Some(protocols.to_string()),
            operating_modes: None,
            type_info,
            rows: rows.to_vec(),
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

    /// Removes the row named `name`.
    pub(crate) fn without(mut self, name: &str) -> Self {
        self.rows.retain(|row| row.2 != name);
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

    /// Leaves out `source`, `protocols` or `operating_modes`.
    pub(crate) fn omit_field(mut self, field: &str) -> Self {
        match field {
            "source" => self.source = None,
            "protocols" => self.protocols = None,
            "operating_modes" => self.operating_modes = None,
            _ => panic!("unknown field {field}"),
        }
        self
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
        if let Some(protocols) = &self.protocols {
            let _ = writeln!(out, "  protocols: {protocols},");
        }
        if let Some(modes) = &self.operating_modes {
            let _ = writeln!(out, "  operating_modes: {modes},");
        }
        let (zero, min, max, min_rad, max_rad) = self.type_info;
        let _ = writeln!(
            out,
            "  type_info: {{\n    value_of_zero_radian_position: {zero},\n    \
             value_of_max_radian_position: {max},\n    value_of_min_radian_position: {min},\n    \
             min_radian: {min_rad:?},\n    max_radian: {max_rad:?},\n  }},"
        );
        out.push_str("  unit_info: [],\n  control_table: [\n");
        for (address, size, name, access, area) in &self.rows {
            let _ = writeln!(
                out,
                "    {{ address: {address}, size: {size}, data_name: {name:?}, access: {access}, area: {area} }},"
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

/// A root that fails exactly one check of data-model §3.
pub(crate) struct Failing {
    /// The check number, 1 to 17.
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

/// One failing root per check; the stories add their checks.
pub(crate) fn failing() -> Vec<Failing> {
    Vec::new()
}
