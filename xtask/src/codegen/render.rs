//! Renders `src/models.rs` and one `src/models/<module>.rs` per model, as
//! unformatted Rust text (rustfmt runs later).

use std::fmt::Write as _;

use super::names::const_ident;
use super::resolve::{GenModel, Limits, LookupList, ModeRole, Resolved};
use super::schema::{Access, Area, ProtocolTag, Row, TypeInfo};
use super::{HEADER, MODULE_DIR, MODULE_FILE, render_tests};

/// Every generated file: path relative to `src/`, and its text.
pub(crate) fn render(resolved: &Resolved) -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = resolved
        .models
        .iter()
        .map(|model| {
            (
                format!("{MODULE_DIR}/{}.rs", model.module_ident),
                render_model(model),
            )
        })
        .collect();
    let tests = render_tests::render_tests(resolved);
    files.push((
        MODULE_FILE.to_string(),
        render_module(resolved, tests.is_some()),
    ));
    if let Some(tests) = tests {
        files.push((format!("{MODULE_DIR}/tests.rs"), tests));
    }
    files
}

fn cfg_attr(family: &str) -> String {
    format!("#[cfg(feature = \"{family}\")]")
}

/// The appearance whose firmware range the legacy docs show: the P2 list
/// when the number has one, else P1.
fn default_appearance(model: &GenModel) -> (ProtocolTag, u8, Option<u8>) {
    model
        .appearances
        .iter()
        .copied()
        .max_by_key(|a| a.0)
        .unwrap_or((ProtocolTag::P2, 0, None))
}

fn firmware_doc(model: &GenModel) -> String {
    match default_appearance(model) {
        (_, 0, None) => String::new(),
        (_, 0, Some(until)) => format!(", firmware up to v{}", until - 1),
        (_, from, None) => format!(", firmware v{from} and later"),
        (_, from, Some(until)) => format!(", firmware v{from} to v{}", until - 1),
    }
}

fn access(access: Access) -> &'static str {
    match access {
        Access::R => "Access::R",
        Access::RW => "Access::RW",
    }
}

fn area(area: Area) -> &'static str {
    match area {
        Area::Eeprom => "Area::Eeprom",
        Area::Ram => "Area::Ram",
        Area::Hybrid => "Area::Hybrid",
    }
}

/// `super::reg(addr, size, Access::.., Area::..)` for one row.
pub(crate) fn reg_call(row: &Row) -> String {
    format!(
        "super::reg({}, {}, {}, {})",
        row.address,
        row.size,
        access(row.access),
        area(row.area)
    )
}

/// `Self::<CONST>` for one role row.
fn role(row: &Row) -> String {
    format!("Self::{}", const_ident(&row.data_name))
}

fn slice(values: &[u8]) -> String {
    let items: Vec<String> = values.iter().map(u8::to_string).collect();
    format!("&[{}]", items.join(", "))
}

/// The `limits:` value of `DATA`.
pub(crate) fn limits_value(limits: &Limits) -> String {
    match limits {
        Limits::MinMax { min, max } => format!(
            "LimitRegisters::MinMax {{ min: {}, max: {} }}",
            role(min),
            role(max)
        ),
        Limits::CwCcw { cw, ccw } => format!(
            "LimitRegisters::CwCcw {{ cw: {}, ccw: {} }}",
            role(cw),
            role(ccw)
        ),
    }
}

/// The `mode:` value of `DATA`.
pub(crate) fn mode_value(mode: &ModeRole) -> String {
    match mode {
        ModeRole::Register { row, values } => format!(
            "ModeSource::Register {{ register: {}, values: ModeValues {{ bounded: {}, unbounded: {}, no_position: {} }} }}",
            role(row),
            slice(&values.bounded),
            slice(&values.unbounded),
            slice(&values.no_position)
        ),
        ModeRole::AngleLimits => "ModeSource::AngleLimits".to_string(),
    }
}

/// The `scale:` value of `DATA`; radians in Rust's shortest `f32` form.
pub(crate) fn scale_value(scale: &TypeInfo) -> String {
    format!(
        "PositionScale::new({}, {}, {}, {:?}, {:?})",
        scale.value_of_zero_radian_position,
        scale.value_of_min_radian_position,
        scale.value_of_max_radian_position,
        scale.min_radian,
        scale.max_radian
    )
}

fn render_model(model: &GenModel) -> String {
    let ty = &model.type_ident;
    let roles = &model.roles;
    let mut out = String::from(HEADER);
    let mode_values = match roles.mode {
        ModeRole::Register { .. } => " ModeValues,",
        ModeRole::AngleLimits => "",
    };
    let _ = writeln!(
        out,
        "// Source: control_tables/{}.ron\n\n\
         use crate::actuator::{{Actuator, Cache, Describe, LimitRegisters, ModeSource,{mode_values} ModelData, Protocols}};\n\
         use crate::id::Id;\n\
         use crate::register::{{Access, Area, Register}};\n\
         use crate::units::PositionScale;\n",
        model.file
    );
    let _ = writeln!(
        out,
        "/// {} (model number {}{}).\n#[derive(Debug, Clone, PartialEq, Eq)]\npub struct {ty} {{\n    cache: Cache,\n}}\n",
        model.name,
        model.number,
        firmware_doc(model)
    );

    let has = |p| model.protocols.contains(&p);
    let hardware_error = roles
        .hardware_error
        .as_ref()
        .map_or_else(|| "None".to_string(), |row| format!("Some({})", role(row)));
    let _ = writeln!(
        out,
        "impl {ty} {{\n    pub(crate) const fn new(id: Id) -> Self {{\n        Self {{ cache: Cache::new(id) }}\n    }}\n\n\
         \x20   pub(crate) const DATA: ModelData = ModelData {{\n\
         \x20       name: {name:?},\n\
         \x20       model_number: {number},\n\
         \x20       protocols: Protocols {{ p1: {p1}, p2: {p2} }},\n\
         \x20       torque_enable: {torque},\n\
         \x20       goal_position: {goal},\n\
         \x20       present_position: {present},\n\
         \x20       limits: {limits},\n\
         \x20       hardware_error: {hardware_error},\n\
         \x20       mode: {mode},\n\
         \x20       scale: {scale},\n\
         \x20   }};\n",
        name = model.name,
        number = model.number,
        p1 = has(ProtocolTag::P1),
        p2 = has(ProtocolTag::P2),
        torque = role(&roles.torque_enable),
        goal = role(&roles.goal_position),
        present = role(&roles.present_position),
        limits = limits_value(&roles.limits),
        mode = mode_value(&roles.mode),
        scale = scale_value(&model.scale),
    );
    for (name, row) in &model.registers {
        let _ = writeln!(
            out,
            "    pub(crate) const {name}: Register = {};",
            reg_call(row)
        );
    }
    out.push_str("}\n\n");
    let _ = writeln!(
        out,
        "impl Describe for {ty} {{\n    fn data(&self) -> &'static ModelData {{\n        &Self::DATA\n    }}\n\n\
         \x20   fn cache(&self) -> &Cache {{\n        &self.cache\n    }}\n\n\
         \x20   fn cache_mut(&mut self) -> &mut Cache {{\n        &mut self.cache\n    }}\n}}\n"
    );
    let _ = writeln!(
        out,
        "impl Actuator for {ty} {{\n    fn id(&self) -> Id {{\n        self.cache.id()\n    }}\n\n    fn model_number(&self) -> u16 {{\n        {}\n    }}\n\n    fn name(&self) -> &'static str {{\n        {:?}\n    }}\n}}",
        model.number, model.name
    );
    out
}

/// The list today's lookup uses for each number: P2 when present, else P1.
fn default_lists(resolved: &Resolved) -> Vec<&LookupList> {
    let mut lists: Vec<&LookupList> = Vec::new();
    for list in &resolved.lookups {
        match lists.last_mut() {
            Some(last) if last.number == list.number => *last = list,
            _ => lists.push(list),
        }
    }
    lists
}

fn render_module(resolved: &Resolved, has_tests: bool) -> String {
    let mut out = String::from(HEADER);
    out.push_str(
        "//! Actuator model types, one per control table.\n\n\
         #![allow(\n    dead_code,\n    unused_variables,\n    clippy::approx_constant,\n    reason = \"every register is generated but only some are used; \
         with no family enabled some parameters are unused; \
         type_info angles are table data, not uses of π\"\n)]\n\n\
         use crate::actuator::{Actuator, Cache, Describe, ModelData};\n\
         use crate::error::Error;\n\
         use crate::id::Id;\n\
         use crate::register::{Access, Area, Register};\n\n",
    );

    for model in &resolved.models {
        let cfg = cfg_attr(&model.family);
        let _ = writeln!(
            out,
            "{cfg}\nmod {};\n{cfg}\npub use {}::{};",
            model.module_ident, model.module_ident, model.type_ident
        );
    }

    if has_tests {
        out.push_str("\n#[cfg(test)]\nmod tests;\n");
    }
    out.push_str(
        "\n/// Builds one register row. Private to `models`; model files call `super::reg`.\n\
         const fn reg(addr: u16, size: u8, access: Access, area: Area) -> Register {\n\
         \x20   Register { addr, size, access, area }\n}\n",
    );

    out.push_str(
        "\n/// Any supported actuator, one variant per model type.\n\
         ///\n\
         /// Match on it to reach a model's specific features; use the\n\
         /// [`Actuator`] trait for what every model has in common.\n\
         #[derive(Debug, Clone, PartialEq, Eq)]\n#[non_exhaustive]\npub enum ActuatorKind {\n",
    );
    for model in &resolved.models {
        let ty = &model.type_ident;
        let _ = writeln!(
            out,
            "    /// {}{}.\n    {}\n    {ty}({ty}),",
            model.name,
            firmware_doc(model),
            cfg_attr(&model.family)
        );
    }
    out.push_str("}\n\n");

    out.push_str(
        "impl ActuatorKind {\n\
         \x20   /// Builds the model type for `model_number`, picking the control table\n\
         \x20   /// that matches `firmware`.\n\
         \x20   ///\n\
         \x20   /// Fails with [`Error::UnsupportedModel`] if the model is unknown or its\n\
         \x20   /// family's Cargo feature is off.\n\
         \x20   pub(crate) fn new(model_number: u16, firmware: u8, id: Id) -> Result<Self, Error> {\n\
         \x20       match model_number {\n",
    );
    for list in default_lists(resolved) {
        let cfg = cfg_attr(&list.family);
        let number = list.number;
        for (i, (_, ty)) in list.variants.iter().enumerate() {
            let guard = list
                .variants
                .get(i + 1)
                .map_or_else(String::new, |next| format!(" if firmware < {}", next.0));
            let _ = writeln!(
                out,
                "            {cfg}\n            {number}{guard} => Ok(Self::{ty}({ty}::new(id))),"
            );
        }
        let _ = writeln!(
            out,
            "            #[cfg(not(feature = \"{}\"))]\n            {number} => Err(Error::UnsupportedModel {{ model_number, feature: Some(\"{}\") }}),",
            list.family, list.family
        );
    }
    out.push_str(
        "            _ => Err(Error::UnsupportedModel { model_number, feature: None }),\n        }\n    }\n}\n\n",
    );

    out.push_str("impl Describe for ActuatorKind {\n");
    for (method, receiver, ret, binding) in [
        ("data", "&self", "&'static ModelData", "ref"),
        ("cache", "&self", "&Cache", "ref"),
        ("cache_mut", "&mut self", "&mut Cache", "ref mut"),
    ] {
        let _ = writeln!(
            out,
            "    fn {method}({receiver}) -> {ret} {{\n        match *self {{"
        );
        for model in &resolved.models {
            let _ = writeln!(
                out,
                "            {}\n            Self::{}({binding} m) => m.{method}(),",
                cfg_attr(&model.family),
                model.type_ident
            );
        }
        out.push_str("        }\n    }\n\n");
    }
    out.push_str("}\n\n");

    out.push_str("impl Actuator for ActuatorKind {\n");
    for (method, ret) in [
        ("id", "Id"),
        ("model_number", "u16"),
        ("name", "&'static str"),
    ] {
        let _ = writeln!(
            out,
            "    fn {method}(&self) -> {ret} {{\n        match *self {{"
        );
        for model in &resolved.models {
            let _ = writeln!(
                out,
                "            {}\n            Self::{}(ref m) => m.{method}(),",
                cfg_attr(&model.family),
                model.type_ident
            );
        }
        out.push_str("        }\n    }\n\n");
    }
    out.push_str("}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::fixtures::{Map, Table, resolved};

    fn files_for(map: &Map, tables: &[(&str, Table)]) -> Vec<(String, String)> {
        render(&resolved(map, tables))
    }

    fn file<'a>(files: &'a [(String, String)], name: &str) -> &'a str {
        &files
            .iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("no {name}"))
            .1
    }

    fn compact(text: &str) -> String {
        text.split_whitespace().collect()
    }

    fn one(stem: &str, table: Table) -> Vec<(String, String)> {
        files_for(
            &Map::single(1020, "x-series", "XM430-W350", stem, &table),
            &[(stem, table)],
        )
    }

    fn uses(text: &str) -> Vec<String> {
        text.split(';')
            .filter_map(|s| s.rfind("\nuse ").map(|i| compact(&s[i..])))
            .collect()
    }

    #[test]
    fn model_file_with_mode_register() {
        let files = one("xm", Table::xm());
        let xm = file(&files, "models/xm.rs");
        let c = compact(xm);
        assert!(xm.contains("pub struct Xm {\n    cache: Cache,\n}"));
        assert!(xm.contains("Self { cache: Cache::new(id) }"));
        assert!(xm.contains(
            "pub(crate) const GOAL_POSITION: Register = super::reg(116, 4, Access::RW, Area::Ram);"
        ));
        assert!(xm.contains(
            "pub(crate) const OPERATING_MODE: Register = super::reg(11, 1, Access::RW, Area::Eeprom);"
        ));
        assert!(xm.contains("pub(crate) const DATA: ModelData = ModelData {"));
        for part in [
            "limits:LimitRegisters::MinMax{min:Self::MIN_POSITION_LIMIT,max:Self::MAX_POSITION_LIMIT}",
            "hardware_error:Some(Self::HARDWARE_ERROR_STATUS)",
            "mode:ModeSource::Register{register:Self::OPERATING_MODE,values:ModeValues{bounded:&[3],unbounded:&[4,5],no_position:&[0,1,16]}}",
            "protocols:Protocols{p1:true,p2:true}",
            "fnid(&self)->Id{self.cache.id()}",
            "implDescribeforXm{fndata(&self)->&'staticModelData{&Self::DATA}fncache(&self)->&Cache{&self.cache}fncache_mut(&mutself)->&mutCache{&mutself.cache}}",
        ] {
            assert!(c.contains(part), "{part}");
        }
        let (min_rad, max_rad) = Table::xm().radians();
        assert!(xm.contains(&format!(
            "scale: PositionScale::new(2048, 0, 4095, {min_rad:?}, {max_rad:?})"
        )));
        assert_eq!(
            uses(xm),
            [
                "usecrate::actuator::{Actuator,Cache,Describe,LimitRegisters,ModeSource,ModeValues,ModelData,Protocols}",
                "usecrate::id::Id",
                "usecrate::register::{Access,Area,Register}",
                "usecrate::units::PositionScale",
            ]
        );
    }

    #[test]
    fn model_file_with_angle_limits() {
        let files = one("ax", Table::ax());
        let ax = file(&files, "models/ax.rs");
        let c = compact(ax);
        assert!(c.contains(
            "limits:LimitRegisters::CwCcw{cw:Self::CW_ANGLE_LIMIT,ccw:Self::CCW_ANGLE_LIMIT}"
        ));
        assert!(c.contains("hardware_error:None"));
        assert!(c.contains("mode:ModeSource::AngleLimits"));
        assert!(uses(ax).contains(
            &"usecrate::actuator::{Actuator,Cache,Describe,LimitRegisters,ModeSource,ModelData,Protocols}"
                .to_string()
        ));
    }

    #[test]
    fn model_file_with_control_mode() {
        let files = one("xl320", Table::xl320());
        let c = compact(file(&files, "models/xl320.rs"));
        assert!(c.contains(
            "limits:LimitRegisters::CwCcw{cw:Self::CW_ANGLE_LIMIT,ccw:Self::CCW_ANGLE_LIMIT}"
        ));
        assert!(c.contains(
            "mode:ModeSource::Register{register:Self::CONTROL_MODE,values:ModeValues{bounded:&[2],unbounded:&[],no_position:&[1]}}"
        ));
    }

    #[test]
    fn hybrid_area() {
        let files = one(
            "xm",
            Table::xm().with_row(896, 1, "Hybrid Save", "RW", "Hybrid"),
        );
        assert!(file(&files, "models/xm.rs").contains(
            "pub(crate) const HYBRID_SAVE: Register = super::reg(896, 1, Access::RW, Area::Hybrid);"
        ));
    }

    #[test]
    fn module_file_builds_registers_and_delegates() {
        let files = files_for(
            &Map::single(1020, "x-series", "XM430-W350", "xm", &Table::xm()),
            &[("xm", Table::xm())],
        );
        let module = file(&files, "models.rs");
        let c = compact(module);
        assert!(module.contains(
            "const fn reg(addr: u16, size: u8, access: Access, area: Area) -> Register {"
        ));
        assert!(module.contains("Register { addr, size, access, area }"));
        assert!(c.contains("#![allow(dead_code,unused_variables,clippy::approx_constant,reason="));
        assert!(
            c.contains("implDescribeforActuatorKind{fndata(&self)->&'staticModelData{match*self{")
        );
        assert!(module.contains("Self::Xm(ref m) => m.data(),"));
        assert!(module.contains("Self::Xm(ref mut m) => m.cache_mut(),"));
        assert!(module.contains("#[cfg(test)]\nmod tests;"));
        for (name, text) in &files {
            assert!(!text.contains("Register::new"), "{name}");
            assert!(!text.contains("use crate::{"), "{name}");
            assert!(!text.contains("ModeSource::OperatingMode"), "{name}");
            assert!(!text.contains("ModeSource::ControlMode"), "{name}");
        }
    }
}
