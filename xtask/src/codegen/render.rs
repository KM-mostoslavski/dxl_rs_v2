//! Renders `src/models.rs` and one `src/models/<module>.rs` per model, as
//! unformatted Rust text (rustfmt runs later).

use std::fmt::Write as _;

use super::fr9::{self, ALL, GenRole, GenScale, Quantity, SignKind, Untyped};
use super::names::const_ident;
use super::resolve::{GenModel, Limits, ModeRole, Resolved};
use super::schema::{Access, Area, ProtocolTag, Row, Source, TypeInfo};
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

/// `XM430-W350 (model number 1020)`, with `also` names.
fn name_line(model: &GenModel) -> String {
    if model.also.is_empty() {
        format!("{} (model number {})", model.name, model.number)
    } else {
        format!(
            "{} (model number {}, also sold as {})",
            model.name,
            model.number,
            model.also.join(", ")
        )
    }
}

/// `all firmware`, `firmware up to v52`, `firmware v53 and later` or
/// `firmware v53 to v60`.
fn firmware_range(from: u8, until: Option<u8>) -> String {
    match (from, until.map(|u| u.saturating_sub(1))) {
        (0, None) => "all firmware".to_string(),
        (0, Some(last)) => format!("firmware up to v{last}"),
        (from, None) => format!("firmware v{from} and later"),
        (from, Some(last)) => format!("firmware v{from} to v{last}"),
    }
}

fn protocol_number(protocol: ProtocolTag) -> &'static str {
    match protocol {
        ProtocolTag::P1 => "1",
        ProtocolTag::P2 => "2",
    }
}

/// One sentence per protocol: `Protocol 1, all firmware. Protocol 2, all
/// firmware.`
fn protocol_lines(model: &GenModel) -> String {
    let sentences: Vec<String> = model
        .appearances
        .iter()
        .map(|&(protocol, from, until)| {
            format!(
                "Protocol {}, {}.",
                protocol_number(protocol),
                firmware_range(from, until)
            )
        })
        .collect();
    sentences.join(" ")
}

/// The protocols and firmware of an `ActuatorKind` variant, merged when
/// both protocols share one range: `Protocol 1 and 2, all firmware.`
fn variant_protocols(model: &GenModel) -> String {
    match model.appearances.as_slice() {
        [
            (ProtocolTag::P1, a_from, a_until),
            (ProtocolTag::P2, b_from, b_until),
        ] if (a_from, a_until) == (b_from, b_until) => {
            format!("Protocol 1 and 2, {}.", firmware_range(*a_from, *a_until))
        }
        _ => protocol_lines(model),
    }
}

/// The verification line of a model's doc.
fn source_line(source: &Source) -> String {
    match source {
        Source::Unverified => {
            "Unverified: not yet checked against the ROBOTIS e-Manual.".to_string()
        }
        Source::Verified { url, verified } => format!("Verified against <{url}> on {verified}."),
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

/// `super::reg("name", addr, size, Access::.., Area::..)` for one row.
pub(crate) fn reg_call(row: &Row) -> String {
    format!(
        "super::reg({:?}, {}, {}, {}, {})",
        row.data_name,
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

/// The `mode:` value of `DATA`. A table without the per-mode lists gets
/// them empty, so its `no_position` values keep giving `LimitMode::Pwm`.
pub(crate) fn mode_value(mode: &ModeRole) -> String {
    let list = |l: &Option<Vec<u8>>| slice(l.as_deref().unwrap_or(&[]));
    match mode {
        ModeRole::Register { row, values } => format!(
            "ModeSource::Register {{ register: {}, values: ModeValues {{ bounded: {}, unbounded: {}, no_position: {}, current: {}, velocity: {}, pwm: {}, current_position: {} }} }}",
            role(row),
            slice(&values.bounded),
            slice(&values.unbounded),
            slice(&values.no_position),
            list(&values.current),
            list(&values.velocity),
            list(&values.pwm),
            list(&values.current_position)
        ),
        ModeRole::AngleLimits => "ModeSource::AngleLimits".to_string(),
        ModeRole::TorqueControl { row, on, values } => format!(
            "ModeSource::TorqueControl {{ register: {}, on: {on}, values: ({}, {}) }}",
            role(row),
            values.0,
            values.1
        ),
    }
}

/// `ModeSet::ALL`, or `ModeSet::of(&[LimitMode::..])`.
pub(crate) fn modes_value(modes: fr9::Modes) -> String {
    if modes == ALL {
        return "ModeSet::ALL".to_string();
    }
    let names: Vec<String> = fr9::mode_names(modes)
        .into_iter()
        .map(|m| format!("LimitMode::{m}"))
        .collect();
    format!("ModeSet::of(&[{}])", names.join(", "))
}

/// `Sign::..` for one sign encoding.
pub(crate) fn sign_value(sign: SignKind) -> String {
    match sign {
        SignKind::Unsigned => "Sign::Unsigned".to_string(),
        SignKind::TwosComplement => "Sign::TwosComplement".to_string(),
        SignKind::DirectionBit(n) => format!("Sign::DirectionBit({n})"),
    }
}

/// The SI step of a scale, in Rust's shortest `f32` form.
pub(crate) fn si_literal(scale: &GenScale) -> String {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "the generated scale is an f32"
    )]
    let si = scale.si_per_raw as f32;
    format!("{si:?}")
}

/// One `ModeScale { .. }` literal.
fn mode_scale_value(scale: &GenScale) -> String {
    let range = scale.range.map_or_else(
        || "None".to_string(),
        |(min, max)| format!("Some(({min}, {max}))"),
    );
    format!(
        "ModeScale {{ modes: {}, scale: Some(RegisterScale::new({}, {}, {}, {})), range: {range} }}",
        modes_value(scale.modes),
        si_literal(scale),
        sign_value(scale.sign),
        scale.size,
        scale.zero_is_unlimited
    )
}

/// One `ModeScale { .. }` literal without a scale, for an entry whose unit
/// is not the role's quantity: it keeps the entry's raw range.
fn untyped_value(untyped: &Untyped) -> String {
    let range = untyped.range.map_or_else(
        || "None".to_string(),
        |(min, max)| format!("Some(({min}, {max}))"),
    );
    format!(
        "ModeScale {{ modes: {}, scale: None, range: {range} }}",
        modes_value(untyped.modes)
    )
}

/// `0.229 rpm per raw step in every mode, velocity-based profile`.
fn unit_phrase(value: f64, unit: &str, modes: fr9::Modes, profile: Option<&str>) -> String {
    let modes = if modes == ALL {
        "every mode".to_string()
    } else {
        format!("{} mode", fr9::mode_names(modes).join(" and "))
    };
    let profile = profile.map_or_else(String::new, |p| {
        format!(", {} profile", p.replace('_', "-"))
    });
    format!("{value} {unit} per raw step in {modes}{profile}")
}

/// The doc lines of one role's trait impl: the table's units per mode.
fn role_doc(role: &GenRole) -> String {
    let mut doc = if role.def.quantity == Quantity::Raw {
        format!("/// {}: in register units, no scale.", role.row.data_name)
    } else if role.scales.is_empty() && role.untyped.is_empty() {
        format!(
            "/// {}: raw only, no unit in the table.",
            role.row.data_name
        )
    } else if role.scales.is_empty() {
        format!("/// {}: raw only.", role.row.data_name)
    } else {
        let parts: Vec<String> = role
            .scales
            .iter()
            .map(|s| {
                let phrase = unit_phrase(s.value, &s.unit, s.modes, s.profile.as_deref());
                if s.zero_is_unlimited {
                    format!("{phrase}, raw 0 meaning no limit")
                } else {
                    phrase
                }
            })
            .collect();
        format!("/// {}: {}.", role.row.data_name, parts.join("; "))
    };
    for untyped in &role.untyped {
        let _ = write!(
            doc,
            "\n/// No typed form for {}: not a {}.",
            unit_phrase(
                untyped.value,
                &untyped.unit,
                untyped.modes,
                untyped.profile.as_deref()
            ),
            role.def.field.replace('_', " ")
        );
    }
    for scale in &role.scales {
        if role.def.quantity == Quantity::Load
            && let SignKind::DirectionBit(n) = scale.sign
        {
            let _ = write!(
                doc,
                "\n/// One raw step is 1/{} of the maximum: the full magnitude below direction bit {n} is 100 %.",
                fr9::full_magnitude(n)
            );
        }
    }
    if let Some((min, max)) = role.row_range {
        let _ = write!(doc, "\n/// Raw range {min} to {max}.");
    }
    if role.scales.iter().any(|s| s.profile.is_some()) {
        doc.push_str(
            "\n/// The scale holds only with that Drive Mode profile; Setup does not read Drive Mode.",
        );
    }
    if role.from_fw > 0 {
        let _ = write!(doc, "\n/// From firmware v{} on.", role.from_fw);
    }
    doc
}

/// The Tier 2 trait impl of one role, holding its `RoleRegister`.
fn render_role(out: &mut String, ty: &str, role: &GenRole) {
    let scales: Vec<String> = role
        .scales
        .iter()
        .map(mode_scale_value)
        .chain(role.untyped.iter().map(untyped_value))
        .collect();
    let bounded_by = role.bounded_by.as_ref().map_or_else(
        || "None".to_string(),
        |row| format!("Some({})", self::role(row)),
    );
    let _ = writeln!(
        out,
        "{doc}\nimpl {has} for {ty} {{\n    const ROLE: RoleRegister = RoleRegister {{\n\
         \x20       name: {name:?},\n\
         \x20       register: {register},\n\
         \x20       scales: &[{scales}],\n\
         \x20       bounded_by: {bounded_by},\n\
         \x20       from_fw: {from_fw},\n\
         \x20       row_range: {row_range},\n\
         \x20   }};\n}}\n",
        doc = role_doc(role),
        has = role.def.has,
        name = role.row.data_name,
        register = self::role(&role.row),
        scales = scales.join(", "),
        from_fw = role.from_fw,
        row_range = role.row_range.map_or_else(
            || "None".to_string(),
            |(min, max)| format!("Some(({min}, {max}))"),
        ),
    );
}

/// The `roles:` value of `DATA`.
pub(crate) fn roles_value(model: &GenModel) -> String {
    if model.typed.is_empty() {
        return "Roles::NONE".to_string();
    }
    let fields: Vec<String> = model
        .typed
        .iter()
        .map(|r| format!("{}: Some(<Self as {}>::ROLE)", r.def.field, r.def.has))
        .collect();
    let rest = if model.typed.len() < fr9::ROLES.len() {
        ", ..Roles::NONE"
    } else {
        ""
    };
    format!("Roles {{ {}{rest} }}", fields.join(", "))
}

/// The `use` lines of a model file.
fn model_imports(model: &GenModel) -> String {
    let mut actuator: Vec<&str> = vec![
        "Actuator",
        "Cache",
        "Describe",
        "LimitRegisters",
        "ModeSource",
    ];
    if matches!(model.roles.mode, ModeRole::Register { .. }) {
        actuator.push("ModeValues");
    }
    actuator.extend(["Model", "ModelData", "Protocols", "Roles"]);
    let scales = || model.typed.iter().flat_map(|r| r.scales.iter());
    let untyped = || model.typed.iter().flat_map(|r| r.untyped.iter());
    if scales().next().is_some() || untyped().next().is_some() {
        actuator.extend(["ModeScale", "ModeSet"]);
    }
    if scales().any(|s| s.modes != ALL) || untyped().any(|u| u.modes != ALL) {
        actuator.push("LimitMode");
    }
    if !model.typed.is_empty() {
        actuator.push("RoleRegister");
    }
    actuator.extend(model.typed.iter().map(|r| r.def.has));
    actuator.sort_unstable();
    let units = if scales().next().is_some() {
        "{PositionScale, RegisterScale, Sign}"
    } else {
        "PositionScale"
    };
    format!(
        "use crate::actuator::{{{}}};\n\
         use crate::id::Id;\n\
         use crate::register::{{Access, Area, Register}};\n\
         use crate::units::{units};\n",
        actuator.join(", ")
    )
}

/// The `scale:` value of `DATA`; radians in Rust's shortest `f32` form.
pub(crate) fn position_scale_value(scale: &TypeInfo) -> String {
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
    let _ = writeln!(
        out,
        "// Source: control_tables/{}.ron\n\n{}",
        model.file,
        model_imports(model)
    );
    let _ = writeln!(
        out,
        "/// {}.\n///\n/// {}\n///\n/// Source table: `control_tables/{}.ron`.\n/// {}\n\
         #[derive(Debug, Clone, PartialEq, Eq)]\npub struct {ty} {{\n    cache: Cache,\n}}\n",
        name_line(model),
        protocol_lines(model),
        model.file,
        source_line(&model.source),
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
         \x20       roles: {roles},\n\
         \x20       bulk_read_p1: {bulk_read_p1},\n\
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
        scale = position_scale_value(&model.scale),
        roles = roles_value(model),
        bulk_read_p1 = model.bulk_read_p1,
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
    let _ = writeln!(out, "impl Actuator for {ty} {{}}\n");
    let _ = writeln!(
        out,
        "impl Model for {ty} {{\n    const DATA: &'static ModelData = &Self::DATA;\n}}"
    );
    for role in &model.typed {
        out.push('\n');
        render_role(&mut out, ty, role);
    }
    out
}

fn type_of<'a>(resolved: &'a Resolved, ty: &str) -> &'a GenModel {
    resolved
        .models
        .iter()
        .find(|m| m.type_ident == ty)
        .unwrap_or_else(|| panic!("lookup names unknown type {ty}"))
}

/// `impl ActuatorKind { new, firmware_register }` (research R-8, R-9).
fn render_lookup(out: &mut String, resolved: &Resolved) {
    out.push_str(
        "impl ActuatorKind {\n\
         \x20   /// The variant for `model_number` on `protocol`, using the table whose `from_fw` is the\n\
         \x20   /// highest not above `firmware`.\n\
         \x20   ///\n\
         \x20   /// Fails with [`SetupError::UnsupportedModel`]: `feature` is the family to enable when the\n\
         \x20   /// model has tables on this protocol but its family is off, and `None` otherwise.\n\
         \x20   pub(crate) fn new(\n\
         \x20       protocol: ProtocolVersion,\n\
         \x20       model_number: ModelNumber,\n\
         \x20       firmware: u8,\n\
         \x20       id: Id,\n\
         \x20   ) -> Result<Self, SetupError> {\n\
         \x20       match (protocol, model_number.get()) {\n",
    );
    for list in &resolved.lookups {
        let cfg = cfg_attr(&list.family);
        let key = format!(
            "(ProtocolVersion::{}, {})",
            list.protocol.variant(),
            list.number
        );
        for (i, (_, ty)) in list.variants.iter().enumerate() {
            let guard = list
                .variants
                .get(i + 1)
                .map_or_else(String::new, |next| format!(" if firmware < {}", next.0));
            let _ = writeln!(
                out,
                "            {cfg}\n            {key}{guard} => Ok(Self::{ty}({ty}::new(id))),"
            );
        }
        let _ = writeln!(
            out,
            "            #[cfg(not(feature = \"{family}\"))]\n            {key} => Err(SetupError::UnsupportedModel {{ id, model_number: {number}, protocol, feature: Some(\"{family}\") }}),",
            family = list.family,
            number = list.number,
        );
    }
    out.push_str(
        "            _ => Err(SetupError::UnsupportedModel { id, model_number: model_number.get(), protocol, feature: None }),\n        }\n    }\n\n",
    );

    out.push_str(
        "    /// `Some` when this model's table depends on firmware on this protocol, or has an FR-9\n\
         \x20   /// register only from some firmware on: the Firmware Version register to read before\n\
         \x20   /// calling [`ActuatorKind::new`]. `None` otherwise, including when the model's family\n\
         \x20   /// is off.\n\
         \x20   pub(crate) fn firmware_register(\n\
         \x20       protocol: ProtocolVersion,\n\
         \x20       model_number: ModelNumber,\n\
         \x20   ) -> Option<Register> {\n\
         \x20       match (protocol, model_number.get()) {\n",
    );
    for list in resolved
        .lookups
        .iter()
        .filter(|list| resolved.reads_firmware(list))
    {
        let Some((_, first)) = list.variants.first() else {
            continue;
        };
        let _ = writeln!(
            out,
            "            {}\n            (ProtocolVersion::{}, {}) => Some({}::{}),",
            cfg_attr(&list.family),
            list.protocol.variant(),
            list.number,
            first,
            const_ident(&type_of(resolved, first).roles.firmware_version.data_name)
        );
    }
    out.push_str("            _ => None,\n        }\n    }\n}\n\n");
}

fn render_module(resolved: &Resolved, has_tests: bool) -> String {
    let mut out = String::from(HEADER);
    out.push_str(
        "//! Actuator model types, one per control table.\n\n\
         #![allow(\n    dead_code,\n    unused_variables,\n    clippy::approx_constant,\n    reason = \"every register is generated but only some are used; \
         with no family enabled some parameters are unused; \
         type_info angles are table data, not uses of π\"\n)]\n\n\
         use crate::actuator::{Actuator, Cache, Describe, ModelData, ModelNumber};\n\
         use crate::error::SetupError;\n\
         use crate::id::Id;\n\
         use crate::protocol::ProtocolVersion;\n\
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

    let mut traits: Vec<&str> = fr9::ROLES.iter().map(|r| r.has).collect();
    traits.push("RoleRegister");
    traits.sort_unstable();
    let _ = writeln!(
        out,
        "\n/// The FR-9 typed registers: one trait per register role, implemented by\n\
         /// each model type whose table has the register.\n\
         pub use crate::actuator::{{{}}};",
        traits.join(", ")
    );
    out.push_str(
        "\n/// The model types, for the typed handle of\n\
         /// [`ActuatorRef::model`](crate::ActuatorRef::model).\n\
         pub use crate::actuator::Model;\n",
    );

    if has_tests {
        out.push_str("\n#[cfg(test)]\nmod tests;\n");
    }
    out.push_str(
        "\n/// Builds one register row. Private to `models`; model files call `super::reg`.\n\
         const fn reg(name: &'static str, addr: u16, size: u8, access: Access, area: Area) -> Register {\n\
         \x20   Register { name, addr, size, access, area }\n}\n",
    );
    let (mn_access, mn_area) = resolved.model_number.unwrap_or((Access::R, Area::Eeprom));
    let _ = writeln!(
        out,
        "\n/// Model Number location, the same in every table (codegen checks it). Read on P1 Setup.\n\
         pub(crate) const MODEL_NUMBER: Register = reg(\"Model Number\", 0, 2, {}, {});",
        access(mn_access),
        area(mn_area)
    );

    out.push_str(
        "\n/// Any supported actuator, one variant per model type.\n\
         ///\n\
         /// Match on it to reach a model's own features; use the [`Actuator`] trait for what every\n\
         /// model has in common.\n\
         ///\n\
         /// The enum is `#[non_exhaustive]`: models are added without a breaking change, so a\n\
         /// `match` outside this crate needs a wildcard arm.\n\
         ///\n\
         /// ```compile_fail,E0004\n\
         /// fn f(kind: dxl_rs_v2::ActuatorKind) {\n\
         ///     match kind {}\n\
         /// }\n\
         /// ```\n\
         #[derive(Debug, Clone, PartialEq, Eq)]\n#[non_exhaustive]\npub enum ActuatorKind {\n",
    );
    for model in &resolved.models {
        let ty = &model.type_ident;
        let _ = writeln!(
            out,
            "    /// {}. {}\n    {}\n    {ty}({ty}),",
            name_line(model),
            variant_protocols(model),
            cfg_attr(&model.family)
        );
    }
    out.push_str("}\n\n");

    render_lookup(&mut out, resolved);
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

    out.push_str("impl Actuator for ActuatorKind {}\n");
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
            "pub(crate) const GOAL_POSITION: Register = super::reg(\"Goal Position\", 116, 4, Access::RW, Area::Ram);"
        ));
        assert!(xm.contains(
            "pub(crate) const OPERATING_MODE: Register = super::reg(\"Operating Mode\", 11, 1, Access::RW, Area::Eeprom);"
        ));
        assert!(xm.contains("pub(crate) const DATA: ModelData = ModelData {"));
        for part in [
            "limits:LimitRegisters::MinMax{min:Self::MIN_POSITION_LIMIT,max:Self::MAX_POSITION_LIMIT}",
            "hardware_error:Some(Self::HARDWARE_ERROR_STATUS)",
            "mode:ModeSource::Register{register:Self::OPERATING_MODE,values:ModeValues{bounded:&[3],unbounded:&[4,5],no_position:&[0,1,16],current:&[],velocity:&[],pwm:&[],current_position:&[]}}",
            "roles:Roles{hardware_error_status:Some(<SelfasHasHardwareErrorStatus>::ROLE),..Roles::NONE},",
            "implHasHardwareErrorStatusforXm{constROLE:RoleRegister=RoleRegister{name:\"HardwareErrorStatus\",register:Self::HARDWARE_ERROR_STATUS,scales:&[],bounded_by:None,from_fw:0,row_range:None,};}",
            "protocols:Protocols{p1:true,p2:true}",
            "implActuatorforXm{}",
            "implModelforXm{constDATA:&'staticModelData=&Self::DATA;}",
            "bulk_read_p1:false,};",
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
                "usecrate::actuator::{Actuator,Cache,Describe,HasHardwareErrorStatus,LimitRegisters,ModeSource,ModeValues,Model,ModelData,Protocols,RoleRegister,Roles}",
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
        assert!(c.contains("bulk_read_p1:false,"));
        let bulk = one(
            "ax",
            Table::ax().instructions(
                "[(protocol: P1, source: \"x\", list: [Ping, BulkRead]), (protocol: P2, source: \"x\", list: [BulkRead])]",
            ),
        );
        assert!(compact(file(&bulk, "models/ax.rs")).contains("bulk_read_p1:true,"));
        let later = one(
            "ax",
            Table::ax()
                .instructions("[(protocol: P1, source: \"x\", list: [BulkRead], from_fw: 40)]"),
        );
        assert!(compact(file(&later, "models/ax.rs")).contains("bulk_read_p1:false,"));
        assert!(uses(ax).contains(
            &"usecrate::actuator::{Actuator,Cache,Describe,LimitRegisters,ModeSource,Model,ModelData,Protocols,Roles}"
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
            "mode:ModeSource::Register{register:Self::CONTROL_MODE,values:ModeValues{bounded:&[2],unbounded:&[],no_position:&[1],current:&[],velocity:&[],pwm:&[],current_position:&[]}}"
        ));
    }

    #[test]
    fn model_file_with_fr9_roles() {
        let files = one("ax", Table::ax_fr9());
        let ax = file(&files, "models/ax.rs");
        let c = compact(ax);
        for part in [
            "roles:Roles{present_temperature:Some(<SelfasHasPresentTemperature>::ROLE),",
            "speed_limit:Some(<SelfasHasSpeedLimit>::ROLE),",
            "..Roles::NONE}",
            "implHasSpeedLimitforAx{constROLE:RoleRegister=RoleRegister{name:\"MovingSpeed\",register:Self::MOVING_SPEED,scales:&[ModeScale{modes:ModeSet::of(&[LimitMode::Bounded]),scale:Some(RegisterScale::new(0.011623893,Sign::Unsigned,2,true)),range:Some((0,1023))}],bounded_by:None,from_fw:0,row_range:None,};}",
            "implHasGoalVelocityforAx{constROLE:RoleRegister=RoleRegister{name:\"MovingSpeed\",register:Self::MOVING_SPEED,scales:&[ModeScale{modes:ModeSet::of(&[LimitMode::Wheel]),scale:None,range:Some((0,2047))}],",
            "implHasPresentVelocityforAx{constROLE:RoleRegister=RoleRegister{name:\"PresentSpeed\",register:Self::PRESENT_SPEED,scales:&[ModeScale{modes:ModeSet::of(&[LimitMode::Bounded]),scale:Some(RegisterScale::new(0.011623893,Sign::DirectionBit(10),2,false)),range:None},ModeScale{modes:ModeSet::of(&[LimitMode::Wheel]),scale:None,range:None}],",
            "scale:Some(RegisterScale::new(0.0009775171,Sign::DirectionBit(10),2,false)),range:Some((0,2047))",
            "implHasCwComplianceSlopeforAx{constROLE:RoleRegister=RoleRegister{name:\"CWComplianceSlope\",register:Self::CW_COMPLIANCE_SLOPE,scales:&[],bounded_by:None,from_fw:0,row_range:Some((0,254)),};}",
            "scale:Some(RegisterScale::new(0.0050614546,Sign::Unsigned,1,false))",
        ] {
            assert!(c.contains(part), "{part}");
        }
        for line in [
            "/// Moving Speed: 0.111 rpm per raw step in Bounded mode, raw 0 meaning no limit.",
            "/// Moving Speed: raw only.",
            "/// No typed form for 0.1 % per raw step in Wheel mode: not a goal velocity.",
            "/// Moving: in register units, no scale.",
            "/// Present Load: 0.1 % per raw step in every mode.",
            "/// One raw step is 1/1023 of the maximum: the full magnitude below direction bit 10 is 100 %.",
            "/// Raw range 0 to 254.",
            "/// CCW Compliance Slope: in register units, no scale.",
        ] {
            assert!(ax.lines().any(|l| l == line), "{line}");
        }
        let header = &uses(ax)[0];
        for name in [
            "HasSpeedLimit",
            "LimitMode",
            "ModeScale",
            "ModeSet",
            "RoleRegister",
            "Roles",
        ] {
            assert!(header.contains(name), "{name}");
        }
        assert_eq!(
            uses(ax)[3],
            "usecrate::units::{PositionScale,RegisterScale,Sign}"
        );
    }

    #[test]
    fn model_file_with_torque_switch() {
        let files = one("mx", Table::mx_p1());
        let mx = file(&files, "models/mx.rs");
        let c = compact(mx);
        assert!(c.contains(
            "mode:ModeSource::TorqueControl{register:Self::TORQUE_CTRL_MODE_ENABLE,on:1,values:(0,1)}"
        ));
        assert!(c.contains("implHasGoalCurrentforMx{constROLE:RoleRegister=RoleRegister{name:\"GoalTorque\",register:Self::GOAL_TORQUE,scales:&[ModeScale{modes:ModeSet::ALL,scale:Some(RegisterScale::new(0.0045,Sign::DirectionBit(10),2,false)),range:Some((0,2047))}]"));
        assert!(c.contains("implHasTorqueCtrlModeEnableforMx{"));
        assert!(c.contains("modes:ModeSet::of(&[LimitMode::Bounded,LimitMode::Unbounded])"));
        assert!(c.contains("modes:ModeSet::of(&[LimitMode::Wheel])"));
        assert!(c.contains("from_fw:40,"));
        assert!(mx.contains(
            "/// Realtime Tick: 1 ms per raw step in every mode.\n/// From firmware v40 on.\n"
        ));
        assert!(mx.contains(
            "pub(crate) const TORQUE_CTRL_MODE_ENABLE: Register = super::reg(\"Torque Ctrl Mode Enable\", 70, 1, Access::RW, Area::Ram);"
        ));
        assert!(!uses(mx)[0].contains("ModeValues"));
        let module = compact(file(&files, "models.rs"));
        assert!(module.contains("(ProtocolVersion::P1,1020)=>Some(Mx::FIRMWARE_VERSION),"));
        let ax = compact(file(&one("ax", Table::ax_fr9()), "models.rs"));
        assert!(!ax.contains("=>Some(Ax::FIRMWARE_VERSION)"));
    }

    #[test]
    fn model_file_with_profile_and_bounds() {
        let files = one("xm", Table::xm_fr9());
        let xm = file(&files, "models/xm.rs");
        let c = compact(xm);
        assert!(c.contains("current:&[0],velocity:&[1],pwm:&[16],current_position:&[5]"));
        assert!(c.contains("register:Self::GOAL_VELOCITY,scales:&[ModeScale{modes:ModeSet::ALL,scale:Some(RegisterScale::new(0.023980824,Sign::TwosComplement,4,false)),range:None}],bounded_by:Some(Self::VELOCITY_LIMIT),"));
        assert!(c.contains("bounded_by:Some(Self::CURRENT_LIMIT),"));
        assert!(c.contains("RegisterScale::new(0.00269,Sign::TwosComplement,2,false)"));
        for line in [
            "/// Profile Velocity: 0.229 rpm per raw step in every mode, velocity-based profile, raw 0 meaning no limit.",
            "/// No typed form for 1 ms per raw step in every mode, time-based profile: not a speed limit.",
            "/// The scale holds only with that Drive Mode profile; Setup does not read Drive Mode.",
        ] {
            assert!(xm.lines().any(|l| l == line), "{line}");
        }
        assert!(!uses(xm)[0].contains("LimitMode"));
    }

    #[test]
    fn module_reexports_the_role_traits() {
        let files = one("xm", Table::xm());
        let module = file(&files, "models.rs");
        let c = compact(module);
        assert!(c.contains("pubusecrate::actuator::{HasCcwComplianceMargin,"));
        for role in &fr9::ROLES {
            assert!(c.contains(&format!("{},", role.has)), "{}", role.has);
        }
        assert!(c.contains("RoleRegister};"));
        assert!(c.contains("pubusecrate::actuator::Model;"));
    }

    #[test]
    fn value_helpers() {
        assert_eq!(
            sign_value(SignKind::DirectionBit(10)),
            "Sign::DirectionBit(10)"
        );
        assert_eq!(sign_value(SignKind::TwosComplement), "Sign::TwosComplement");
        assert_eq!(modes_value(ALL), "ModeSet::ALL");
        assert_eq!(
            modes_value(0b100_0001),
            "ModeSet::of(&[LimitMode::Bounded, LimitMode::Wheel])"
        );
        assert_eq!(
            unit_phrase(1.0, "ms", ALL, Some("time_based")),
            "1 ms per raw step in every mode, time-based profile"
        );
    }

    #[test]
    fn hybrid_area() {
        let files = one(
            "xm",
            Table::xm().with_row(896, 1, "Hybrid Save", "RW", "Hybrid"),
        );
        assert!(file(&files, "models/xm.rs").contains(
            "pub(crate) const HYBRID_SAVE: Register = super::reg(\"Hybrid Save\", 896, 1, Access::RW, Area::Hybrid);"
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
            "const fn reg(name: &'static str, addr: u16, size: u8, access: Access, area: Area) -> Register {"
        ));
        assert!(module.contains("Register { name, addr, size, access, area }"));
        assert!(c.contains("#![allow(dead_code,unused_variables,clippy::approx_constant,reason="));
        assert!(
            c.contains("implDescribeforActuatorKind{fndata(&self)->&'staticModelData{match*self{")
        );
        assert!(module.contains("Self::Xm(ref m) => m.data(),"));
        assert!(module.contains("Self::Xm(ref mut m) => m.cache_mut(),"));
        assert!(module.contains("impl Actuator for ActuatorKind {}"));
        assert!(module.contains("#[cfg(test)]\nmod tests;"));
        for (name, text) in &files {
            for method in ["fn id(&self", "fn model_number(&self", "fn name(&self"] {
                assert!(!text.contains(method), "{name}: {method}");
            }
            assert!(!text.contains("Register::new"), "{name}");
            assert!(!text.contains("use crate::{"), "{name}");
            assert!(!text.contains("ModeSource::OperatingMode"), "{name}");
            assert!(!text.contains("ModeSource::ControlMode"), "{name}");
        }
    }

    fn lookup_files() -> Vec<(String, String)> {
        files_for(
            &Map::new()
                .model(12, "ax", "AX-12A")
                .p1(&[(0, "ax")])
                .model(1020, "x-series", "XM430-W350")
                .p1(&[(0, "xm")])
                .p2(&[(0, "xm")])
                .default_p2()
                .model(1190, "x-series", "XL330-M077")
                .p2(&[(0, "fw52"), (53, "new")]),
            &[
                ("ax", Table::ax()),
                ("xm", Table::xm()),
                ("fw52", Table::xm().protocols("[P2]")),
                ("new", Table::xm().protocols("[P2]")),
            ],
        )
    }

    #[test]
    fn lookup_per_protocol() {
        let files = lookup_files();
        let module = file(&files, "models.rs");
        let c = compact(module);
        assert!(c.contains("pub(crate)fnnew(protocol:ProtocolVersion,model_number:ModelNumber,firmware:u8,id:Id,)->Result<Self,SetupError>{match(protocol,model_number.get()){"));
        let fw52 = module
            .find("(ProtocolVersion::P2, 1190) if firmware < 53 => Ok(Self::Fw52(Fw52::new(id))),")
            .unwrap();
        let new = module
            .find("(ProtocolVersion::P2, 1190) => Ok(Self::New(New::new(id))),")
            .unwrap();
        assert!(fw52 < new);
        assert!(c.contains("#[cfg(not(feature=\"x-series\"))](ProtocolVersion::P2,1190)=>Err(SetupError::UnsupportedModel{id,model_number:1190,protocol,feature:Some(\"x-series\")}),"));
        assert!(c.contains("_=>Err(SetupError::UnsupportedModel{id,model_number:model_number.get(),protocol,feature:None}),"));
        let p1 = module.find("(ProtocolVersion::P1, 1020) =>").unwrap();
        let p2 = module.find("(ProtocolVersion::P2, 1020) =>").unwrap();
        assert!(p1 < p2);
        assert!(c.contains("(ProtocolVersion::P2,1190)=>Some(Fw52::FIRMWARE_VERSION),"));
        assert!(!c.contains("(ProtocolVersion::P2,1020)=>Some("));
        assert!(module.contains(
            "pub(crate) const MODEL_NUMBER: Register = reg(\"Model Number\", 0, 2, Access::R, Area::Eeprom);"
        ));
        assert_eq!(
            uses(module),
            [
                "usecrate::actuator::{Actuator,Cache,Describe,ModelData,ModelNumber}",
                "usecrate::error::SetupError",
                "usecrate::id::Id",
                "usecrate::protocol::ProtocolVersion",
                "usecrate::register::{Access,Area,Register}",
            ]
        );
        assert!(
            compact(file(&files, "models/ax.rs")).contains("protocols:Protocols{p1:true,p2:false}")
        );
    }

    #[test]
    fn every_model_item_is_gated() {
        let files = files_for(
            &Map::new()
                .model(12, "ax", "AX-12A")
                .p1(&[(0, "ax")])
                .model(1020, "x-series", "XM430-W350")
                .p1(&[(0, "xm")])
                .p2(&[(0, "xm")])
                .default_p2(),
            &[("ax", Table::ax()), ("xm", Table::xm())],
        );
        let module = file(&files, "models.rs");
        let lines: Vec<&str> = module.lines().map(str::trim).collect();
        for (family, ty) in [("ax", "Ax"), ("x-series", "Xm")] {
            let on = format!("#[cfg(feature = \"{family}\")]");
            let off = format!("#[cfg(not(feature = \"{family}\"))]");
            let mut gated = 0;
            for (i, line) in lines.iter().enumerate() {
                let names_model = line.contains(&format!("Self::{ty}("))
                    || line.contains(&format!("{ty}::"))
                    || *line == format!("mod {};", ty.to_lowercase())
                    || line.starts_with(&format!("pub use {}::", ty.to_lowercase()))
                    || *line == format!("{ty}({ty}),");
                if names_model {
                    assert_eq!(lines[i - 1], on, "line {line}");
                    gated += 1;
                }
                if line.contains(&format!("feature: Some(\"{family}\")")) {
                    let arm = lines[..i].iter().rposition(|l| l.starts_with('#')).unwrap();
                    assert_eq!(lines[arm], off, "line {line}");
                }
            }
            assert!(gated >= 7, "{family}: {gated}");
        }
        let header: String = uses(module).concat();
        for name in [
            "LimitRegisters",
            "Protocols",
            "ModeSource",
            "ModeValues",
            "PositionScale",
        ] {
            assert!(!header.contains(name), "{name}");
        }
        assert!(uses(file(&files, "models/xm.rs"))[0].contains("ModeValues"));
        assert!(!uses(file(&files, "models/ax.rs"))[0].contains("ModeValues"));
    }

    #[test]
    fn model_docs() {
        let files = files_for(
            &Map::new()
                .model(12, "ax", "AX-12A")
                .also(&["AX-12+"])
                .p1(&[(0, "ax")])
                .model(1020, "x-series", "XM430-W350")
                .p1(&[(0, "xm430_w350")])
                .p2(&[(0, "xm430_w350")])
                .default_p2()
                .model(1190, "x-series", "XL330-M077")
                .p2(&[(0, "fw52"), (53, "new")]),
            &[
                (
                    "ax",
                    Table::ax().source(
                        "Verified(url: \"https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/\", verified: \"2026-10-28\")",
                    ),
                ),
                ("xm430_w350", Table::xm()),
                ("fw52", Table::xm().protocols("[P2]")),
                ("new", Table::xm().protocols("[P2]")),
            ],
        );
        let xm = file(&files, "models/xm430_w350.rs");
        assert!(xm.starts_with(&format!(
            "{HEADER}// Source: control_tables/xm430_w350.ron\n"
        )));
        for line in [
            "/// XM430-W350 (model number 1020).",
            "/// Protocol 1, all firmware. Protocol 2, all firmware.",
            "/// Source table: `control_tables/xm430_w350.ron`.",
            "/// Unverified: not yet checked against the ROBOTIS e-Manual.",
        ] {
            assert!(xm.lines().any(|l| l == line), "{line}");
        }
        let ax = file(&files, "models/ax.rs");
        assert!(ax.contains("/// AX-12A (model number 12, also sold as AX-12+)."));
        assert!(ax.contains(
            "/// Verified against <https://emanual.robotis.com/docs/en/dxl/ax/ax-12a/> on 2026-10-28."
        ));
        assert!(!ax.contains("Unverified"));
        assert!(file(&files, "models/fw52.rs").contains("/// Protocol 2, firmware up to v52."));
        assert!(file(&files, "models/new.rs").contains("/// Protocol 2, firmware v53 and later."));

        let module = file(&files, "models.rs");
        assert!(module.starts_with(HEADER));
        assert!(
            module.contains("/// XM430-W350 (model number 1020). Protocol 1 and 2, all firmware.")
        );
        assert!(
            module.contains("/// XL330-M077 (model number 1190). Protocol 2, firmware up to v52.")
        );
        assert!(module.contains(
            "/// AX-12A (model number 12, also sold as AX-12+). Protocol 1, all firmware."
        ));
        assert!(file(&files, "models/tests.rs").starts_with(HEADER));
    }

    #[test]
    fn firmware_ranges() {
        assert_eq!(firmware_range(0, None), "all firmware");
        assert_eq!(firmware_range(0, Some(53)), "firmware up to v52");
        assert_eq!(firmware_range(53, None), "firmware v53 and later");
        assert_eq!(firmware_range(53, Some(61)), "firmware v53 to v60");
    }

    #[test]
    fn visibility() {
        let files = lookup_files();
        let module = file(&files, "models.rs");
        let c = compact(module);
        assert!(
            c.contains("#[derive(Debug,Clone,PartialEq,Eq)]#[non_exhaustive]pubenumActuatorKind{")
        );
        assert!(module.contains(
            "/// ```compile_fail,E0004\n/// fn f(kind: dxl_rs_v2::ActuatorKind) {\n///     match kind {}\n/// }\n/// ```"
        ));
        assert!(module.contains("Any supported actuator, one variant per model type."));
        for (module_name, ty) in [("ax", "Ax"), ("xm", "Xm"), ("fw52", "Fw52"), ("new", "New")] {
            assert!(module.contains(&format!("pub use {module_name}::{ty};")));
            let text = file(&files, &format!("models/{module_name}.rs"));
            let at = text.find(&format!("pub struct {ty} {{")).unwrap();
            assert!(text[..at].trim_end().lines().any(|l| l.starts_with("/// ")));
            assert!(!text.contains("pub const"), "{module_name}");
            assert!(!text.contains("pub fn"), "{module_name}");
            assert!(text.contains("pub(crate) const fn new(id: Id) -> Self"));
            assert!(text.contains("pub(crate) const DATA: ModelData"));
        }
    }
}
