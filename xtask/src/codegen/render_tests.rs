//! Renders `src/models/tests.rs`: crate tests of the generated lookup and
//! model data against every model number of the real tables.
//!
//! The tests name registers and model data only by constant, never by a
//! literal address, so a one-row table change still changes one file.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use super::HEADER;
use super::fr9::{self, ALL, GenRole, GenScale, Quantity, SignKind};
use super::names::const_ident;
use super::render::sign_value;
use super::resolve::{GenModel, LookupList, ModeRole, Resolved};
use super::schema::ProtocolTag;

/// The text of `src/models/tests.rs`, or `None` with no model.
pub(crate) fn render_tests(resolved: &Resolved) -> Option<String> {
    if resolved.models.is_empty() {
        return None;
    }
    let mut out = String::from(HEADER);
    out.push_str(
        "//! Tests of the generated models and lookup, one or more per model number and\n\
         //! model type of `control_tables/`.\n\n\
         use super::ActuatorKind;\n\
         use crate::actuator::ModelNumber;\n\
         use crate::error::SetupError;\n\
         use crate::id::Id;\n\
         use crate::protocol::ProtocolVersion;\n\n\
         fn id() -> Id {\n    Id::new(1).unwrap()\n}\n\n",
    );
    render_helpers(&mut out, resolved);
    render_unknown(&mut out, resolved);
    for list in &resolved.lookups {
        render_lookup(&mut out, list, resolved.reads_firmware(list));
    }
    render_family_off(&mut out, resolved);
    for model in &resolved.models {
        render_concrete_match(&mut out, model);
    }
    for model in &resolved.models {
        let ty = &model.type_ident;
        let _ = writeln!(
            out,
            "#[cfg(feature = \"{family}\")]\n\
             #[test]\n\
             fn {module}_describe_round_trip() {{\n\
             \x20   use crate::actuator::{{Cache, Describe, LimitMode}};\n\
             \x20   use crate::models::{ty};\n\n\
             \x20   let id = id();\n\
             \x20   let mut kind = ActuatorKind::{ty}({ty}::new(id));\n\
             \x20   assert_eq!(kind.data(), &{ty}::DATA);\n\
             \x20   assert_eq!(super::MODEL_NUMBER, {ty}::MODEL_NUMBER);\n\
             \x20   assert_eq!(kind.cache().id(), id);\n\
             \x20   kind.cache_mut().record_mode(LimitMode::Unbounded);\n\
             \x20   assert!(kind.cache_mut().record_limits(10, 20).is_ok());\n\
             \x20   let limits = kind.cache().limits().map(|l| (l.min_raw(), l.max_raw()));\n\
             \x20   assert_eq!(limits, Some((10, 20)));\n\
             \x20   let mut expected = Cache::new(id);\n\
             \x20   expected.record_mode(LimitMode::Unbounded);\n\
             \x20   assert!(expected.record_limits(10, 20).is_ok());\n\
             \x20   assert_eq!(*kind.cache(), expected);\n\
             }}\n",
            family = model.family,
            module = model.module_ident,
        );
    }
    for model in &resolved.models {
        render_roles(&mut out, model);
        render_mode(&mut out, model);
    }
    Some(out)
}

/// The raw value the scale tests convert.
const SAMPLE: i64 = 100;

/// The FR-9 test helpers, compiled with any family on.
fn render_helpers(out: &mut String, resolved: &Resolved) {
    let mut families: Vec<&str> = resolved.models.iter().map(|m| m.family.as_str()).collect();
    families.sort_unstable();
    families.dedup();
    let any: Vec<String> = families
        .iter()
        .map(|f| format!("feature = \"{f}\""))
        .collect();
    let _ = writeln!(
        out,
        "/// Every limit mode.\n\
         #[cfg(any({any}))]\n\
         const ALL_MODES: [crate::actuator::LimitMode; 7] = {{\n\
         \x20   use crate::actuator::LimitMode;\n\
         \x20   [LimitMode::Bounded, LimitMode::Unbounded, LimitMode::Pwm, LimitMode::UnboundedCurrent, LimitMode::Velocity, LimitMode::Current, LimitMode::Wheel]\n\
         }};\n\n\
         /// Whether `got` is `want` to `f32` precision.\n\
         #[cfg(any({any}))]\n\
         fn close(got: f32, want: f32) -> bool {{\n\
         \x20   (got - want).abs() <= 1e-5 * want.abs().max(1.0)\n\
         }}\n\n\
         /// The Tier 1 lookup of a role finds the Tier 2 trait's register.\n\
         #[cfg(any({any}))]\n\
         fn same_role(\n\
         \x20   found: Option<&crate::actuator::RoleRegister>,\n\
         \x20   tier2: crate::actuator::RoleRegister,\n\
         \x20   register: crate::register::Register,\n\
         \x20   name: &str,\n\
         ) {{\n\
         \x20   assert_eq!(found, Some(&tier2), \"{{name}}\");\n\
         \x20   assert_eq!(tier2.register(), register, \"{{name}}\");\n\
         \x20   assert_eq!((tier2.address(), tier2.size()), (register.addr, register.size), \"{{name}}\");\n\
         \x20   assert_eq!(tier2.name(), name);\n\
         }}\n",
        any = any.join(", ")
    );
}

/// The raw value of signed `value` under `sign` in a `size`-byte register.
fn raw_of(value: i64, sign: SignKind, size: u8) -> i64 {
    match sign {
        SignKind::Unsigned => value,
        SignKind::DirectionBit(n) if value < 0 => -value | (1 << n),
        SignKind::DirectionBit(_) => value,
        SignKind::TwosComplement => value & ((1i64 << (u32::from(size) * 8)) - 1),
    }
}

/// `x` as the `f32` the generated test compares with.
#[allow(
    clippy::cast_possible_truncation,
    reason = "the generated value is an f32"
)]
fn f32_of(x: f64) -> f32 {
    x as f32
}

/// The expression converting raw `raw` with `scale` to its SI `f32`.
fn convert(quantity: Quantity, raw: i64) -> String {
    match quantity {
        Quantity::Velocity => format!("scale.velocity({raw}).rad_per_s()"),
        Quantity::Current => format!("scale.current({raw}).amps()"),
        Quantity::Voltage => format!("scale.voltage({raw}).volts()"),
        Quantity::Temperature => format!("scale.temperature({raw}).celsius()"),
        Quantity::Load => format!("scale.load({raw}).ratio()"),
        Quantity::Tick | Quantity::Angle | Quantity::Raw => format!("scale.si_of({raw})"),
    }
}

/// `Some(min..=max)` or `None`, as the generated test compares it.
fn range_value(range: Option<(i64, i64)>) -> String {
    range.map_or_else(
        || "None".to_string(),
        |(min, max)| format!("Some({min}..={max})"),
    )
}

/// Checks of one scale of `role`, in its first mode.
fn scale_checks(role: &GenRole, scale: &GenScale) -> String {
    let mode = fr9::mode_names(scale.modes)[0];
    let factor = role.def.quantity.factor(&scale.unit).unwrap_or(1.0);
    let step = match (role.def.quantity, scale.sign) {
        (Quantity::Load, SignKind::DirectionBit(n)) => fr9::load_step(n),
        _ => scale.value * factor,
    };
    let mut out = format!(
        "let scale = role.scale(LimitMode::{mode}).unwrap();\n\
         assert_eq!((scale.sign(), scale.size(), scale.zero_is_unlimited()), ({}, {}, {}));\n\
         assert!(close({}, {:?}));\n",
        sign_value(scale.sign),
        scale.size,
        scale.zero_is_unlimited,
        convert(role.def.quantity, SAMPLE),
        f32_of(SAMPLE as f64 * step),
    );
    if scale.sign != SignKind::Unsigned {
        let raw = raw_of(-SAMPLE, scale.sign, scale.size);
        let _ = writeln!(
            out,
            "assert!(close({}, {:?}));",
            convert(role.def.quantity, raw),
            f32_of(-(SAMPLE as f64) * step)
        );
    }
    if scale.zero_is_unlimited && role.def.quantity == Quantity::Velocity {
        out.push_str("assert_eq!(scale.speed_limit(0), SpeedLimit::Unlimited);\n");
    }
    let range = range_value(scale.range.or(role.row_range));
    let _ = writeln!(
        out,
        "assert_eq!(role.raw_range(LimitMode::{mode}), {range});"
    );
    if role.def.quantity == Quantity::Tick
        && let Some((_, max)) = scale.range.or(role.row_range)
    {
        let _ = writeln!(
            out,
            "assert_eq!(role.tick_modulus(LimitMode::{mode}), Some({}));",
            (max + 1) & 0xFFFF
        );
    }
    let check = "assert_eq!(role.scale(mode), Some(scale));\n";
    if scale.modes == ALL {
        let _ = writeln!(out, "for mode in ALL_MODES {{\n{check}}}");
    } else {
        let modes: Vec<String> = fr9::mode_names(scale.modes)
            .iter()
            .map(|m| format!("LimitMode::{m}"))
            .collect();
        out.push_str(&each("mode", &modes, check));
    }
    out
}

/// The raw range the first untyped entry of `role` covering mode `bit`
/// gives, if it gives one.
fn untyped_range(role: &GenRole, bit: fr9::Modes) -> Option<(i64, i64)> {
    role.untyped
        .iter()
        .find(|u| u.modes & bit != 0)
        .and_then(|u| u.range)
}

/// One model's FR-9 roles: Tier 1 finds what Tier 2 names, absent roles
/// are absent, and every scale converts the table's unit to SI.
fn render_roles(out: &mut String, model: &GenModel) {
    let ty = &model.type_ident;
    let mut body = String::from("let data = &");
    let _ = writeln!(body, "{ty}::DATA;");
    let present: Vec<String> = model
        .typed
        .iter()
        .map(|role| {
            format!(
                "(Role::{variant}, <{ty} as {has}>::ROLE, {ty}::{constant}, {name:?})",
                variant = role.def.variant,
                has = role.def.has,
                constant = const_ident(&role.row.data_name),
                name = role.row.data_name,
            )
        })
        .collect();
    if !present.is_empty() {
        body.push_str(&each(
            "(role, tier2, register, name)",
            &present,
            "same_role(data.role(role), tier2, register, name);\n",
        ));
    }
    let absent: Vec<String> = fr9::ROLES
        .iter()
        .filter(|def| !model.typed.iter().any(|r| r.def.field == def.field))
        .map(|def| format!("Role::{}", def.variant))
        .collect();
    if !absent.is_empty() {
        let _ = writeln!(
            body,
            "for role in [{}] {{\nassert_eq!(data.role(role), None);\nassert_eq!(data.typed(role, LimitMode::Bounded), None);\n}}",
            absent.join(", ")
        );
    }
    let plain = |r: &&GenRole| {
        r.scales.is_empty()
            && r.from_fw == 0
            && r.bounded_by.is_none()
            && r.untyped.iter().all(|u| u.range.is_none())
    };
    let raw: Vec<String> = model
        .typed
        .iter()
        .filter(plain)
        .map(|r| {
            format!(
                "(Role::{}, <{ty} as {}>::ROLE, {ty}::{}, {})",
                r.def.variant,
                r.def.has,
                const_ident(&r.row.data_name),
                range_value(r.row_range)
            )
        })
        .collect();
    if !raw.is_empty() {
        body.push_str(&each(
            "(role, tier2, register, range)",
            &raw,
            "assert_eq!((tier2.from_firmware(), tier2.bounded_by()), (0, None));\n\
             for mode in ALL_MODES {\n\
             assert_eq!(tier2.scale(mode), None);\n\
             assert_eq!(tier2.raw_range(mode), range);\n\
             assert_eq!(data.typed(role, mode), Some((register, None)));\n\
             }\n",
        ));
    }
    for role in model.typed.iter().filter(|r| !plain(r)) {
        let _ = writeln!(
            body,
            "{{\nlet role = <{ty} as {}>::ROLE;\nassert_eq!(role.from_firmware(), {});",
            role.def.has, role.from_fw
        );
        let bound = role.bounded_by.as_ref().map_or_else(
            || "None".to_string(),
            |row| format!("Some({ty}::{})", const_ident(&row.data_name)),
        );
        let _ = writeln!(body, "assert_eq!(role.bounded_by(), {bound});");
        let covered = role.scales.iter().fold(0, |acc, s| acc | s.modes);
        let mut by_range: Vec<(Option<(i64, i64)>, Vec<String>)> = Vec::new();
        for bit in (0..7)
            .map(|i| 1u8 << i)
            .filter(|bit| ALL & !covered & bit != 0)
        {
            let range = untyped_range(role, bit).or(role.row_range);
            let name = format!("LimitMode::{}", fr9::mode_names(bit)[0]);
            match by_range.iter_mut().find(|(r, _)| *r == range) {
                Some((_, modes)) => modes.push(name),
                None => by_range.push((range, vec![name])),
            }
        }
        for (range, modes) in by_range {
            let check = format!(
                "assert_eq!(role.scale(mode), None);\nassert_eq!(role.raw_range(mode), {});\nassert_eq!(data.typed(Role::{}, mode), Some(({ty}::{}, None)));\n",
                range_value(range),
                role.def.variant,
                const_ident(&role.row.data_name),
            );
            let _ = writeln!(body, "{{\n{}}}", each("mode", &modes, &check));
        }
        for scale in &role.scales {
            body.push_str("{\n");
            body.push_str(&scale_checks(role, scale));
            let _ = writeln!(
                body,
                "assert_eq!(data.typed(Role::{}, LimitMode::{}), Some(({ty}::{}, Some(scale))));",
                role.def.variant,
                fr9::mode_names(scale.modes)[0],
                const_ident(&role.row.data_name),
            );
            body.push_str("}\n");
        }
        body.push_str("}\n");
    }
    let mut actuator: Vec<&str> = vec!["LimitMode", "Role"];
    actuator.extend(model.typed.iter().map(|r| r.def.has));
    actuator.sort_unstable();
    let scales = || {
        model
            .typed
            .iter()
            .flat_map(|r| r.scales.iter().map(move |s| (r, s)))
    };
    let unlimited =
        scales().any(|(r, s)| s.zero_is_unlimited && r.def.quantity == Quantity::Velocity);
    let units = match (scales().next().is_some(), unlimited) {
        (true, true) => "use crate::units::{Sign, SpeedLimit};\n",
        (true, false) => "use crate::units::Sign;\n",
        (false, _) => "",
    };
    let _ = writeln!(
        out,
        "#[cfg(feature = \"{family}\")]\n\
         #[test]\n\
         fn {module}_roles() {{\n\
         \x20   use crate::actuator::{{{actuator}}};\n\
         \x20   use crate::models::{ty};\n\
         {units}\n\
         {body}\
         }}\n",
        family = model.family,
        module = model.module_ident,
        actuator = actuator.join(", "),
    );
}

/// The `LimitMode` each listed mode value gives, in the decoder's order.
fn listed_modes(values: &super::schema::OperatingModes) -> Vec<(u8, &'static str)> {
    let lists: [(&[u8], &str); 7] = [
        (&values.bounded, "Bounded"),
        (
            values.current_position.as_deref().unwrap_or(&[]),
            "UnboundedCurrent",
        ),
        (&values.unbounded, "Unbounded"),
        (values.current.as_deref().unwrap_or(&[]), "Current"),
        (values.velocity.as_deref().unwrap_or(&[]), "Velocity"),
        (values.pwm.as_deref().unwrap_or(&[]), "Pwm"),
        (&values.no_position, "Pwm"),
    ];
    let mut out: Vec<(u8, &'static str)> = Vec::new();
    for (list, mode) in lists {
        for &value in list {
            if !out.iter().any(|(v, _)| *v == value) {
                out.push((value, mode));
            }
        }
    }
    out.sort_unstable();
    out
}

/// One model's mode source: every listed value decodes to its mode, an
/// unlisted one to none; the torque switch overrides the angle limits.
fn render_mode(out: &mut String, model: &GenModel) {
    let ty = &model.type_ident;
    let body = match &model.roles.mode {
        ModeRole::Register { row, values } => {
            let listed = listed_modes(values);
            let cases: Vec<String> = listed
                .iter()
                .map(|(v, m)| format!("({v}, Some(LimitMode::{m}))"))
                .chain(
                    (0..=u8::MAX)
                        .find(|v| !listed.iter().any(|(l, _)| l == v))
                        .map(|v| format!("({v}, None)")),
                )
                .collect();
            format!(
                "assert_eq!(mode.register(), Some({ty}::{}));\n\
                 for (value, expected) in [{}] {{\n\
                 assert_eq!(mode.limit_mode(value, limits), expected);\n\
                 }}\n",
                const_ident(&row.data_name),
                cases.join(", ")
            )
        }
        ModeRole::AngleLimits => "assert_eq!(mode.register(), None);\n\
             for value in [0, u8::MAX] {\n\
             assert_eq!(mode.limit_mode(value, limits), None);\n\
             }\n"
        .to_string(),
        ModeRole::TorqueControl { row, on, values } => {
            let mut body = format!(
                "assert_eq!(mode.register(), Some({ty}::{}));\n\
                 assert_eq!(mode.limit_mode({on}, limits), Some(LimitMode::Current));\n",
                const_ident(&row.data_name)
            );
            for value in values.0..=values.1 {
                if value != *on {
                    let _ = writeln!(
                        body,
                        "assert_eq!(mode.limit_mode({value}, limits), Some(LimitMode::from_angle_limits(limits)));"
                    );
                }
            }
            if let Some(outside) = values.1.checked_add(1) {
                let _ = writeln!(
                    body,
                    "assert_eq!(mode.limit_mode({outside}, limits), None);"
                );
            }
            body
        }
    };
    let _ = writeln!(
        out,
        "#[cfg(feature = \"{family}\")]\n\
         #[test]\n\
         fn {module}_mode_source() {{\n\
         {limit_mode}\
         \x20   use crate::models::{ty};\n\
         \x20   use crate::units::Limits;\n\n\
         \x20   let mode = {ty}::DATA.mode;\n\
         \x20   let limits = Limits::new(id(), 0, 1).unwrap();\n\
         {body}\
         }}\n",
        family = model.family,
        module = model.module_ident,
        limit_mode = if model.roles.mode == ModeRole::AngleLimits {
            ""
        } else {
            "    use crate::actuator::LimitMode;\n"
        },
    );
}

/// The model type reached through its first lookup, as user code matches
/// it.
fn render_concrete_match(out: &mut String, model: &GenModel) {
    let Some(&(protocol, from_fw, _)) = model.appearances.first() else {
        return;
    };
    let ty = &model.type_ident;
    let _ = writeln!(
        out,
        "#[cfg(feature = \"{family}\")]\n\
         #[test]\n\
         fn {module}_concrete_match() {{\n\
         \x20   use crate::actuator::Actuator;\n\
         \x20   use crate::models::{ty};\n\n\
         \x20   let id = id();\n\
         \x20   let kind = ActuatorKind::new(ProtocolVersion::{p}, ModelNumber({number}), {from_fw}, id);\n\
         \x20   assert!(matches!(&kind, Ok(ActuatorKind::{ty}(m)) if m.id() == id && m.name() == {ty}::DATA.name && m.model_number() == {ty}::DATA.model_number));\n\
         }}\n",
        family = model.family,
        module = model.module_ident,
        p = protocol.variant(),
        number = model.number,
    );
}

/// An unknown number on both protocols, and every known number on a
/// protocol it has no tables on: `feature: None` in every build.
fn render_unknown(out: &mut String, resolved: &Resolved) {
    let numbers: BTreeSet<u16> = resolved.lookups.iter().map(|l| l.number).collect();
    let unknown = (0..=u16::MAX)
        .rev()
        .find(|n| !numbers.contains(n))
        .unwrap_or(u16::MAX);
    let _ = writeln!(
        out,
        "#[test]\n\
         fn unknown_model_number() {{\n\
         \x20   let id = id();\n\
         \x20   for protocol in [ProtocolVersion::P1, ProtocolVersion::P2] {{\n\
         \x20       let found = ActuatorKind::new(protocol, ModelNumber({unknown}), 0, id);\n\
         \x20       let error = SetupError::UnsupportedModel {{ id, model_number: {unknown}, protocol, feature: None }};\n\
         \x20       assert_eq!(found, Err(error));\n\
         \x20       assert_eq!(ActuatorKind::firmware_register(protocol, ModelNumber({unknown})), None);\n\
         \x20   }}\n\
         }}\n"
    );
    let missing: Vec<String> = numbers
        .iter()
        .flat_map(|&n| [(ProtocolTag::P1, n), (ProtocolTag::P2, n)])
        .filter(|&(p, n)| {
            !resolved
                .lookups
                .iter()
                .any(|l| l.protocol == p && l.number == n)
        })
        .map(|(p, n)| format!("(ProtocolVersion::{}, {n})", p.variant()))
        .collect();
    if missing.is_empty() {
        return;
    }
    let _ = writeln!(
        out,
        "#[test]\n\
         fn no_tables_on_this_protocol() {{\n\
         \x20   let id = id();\n\
         {}\
         }}\n",
        each(
            "(protocol, number)",
            &missing,
            "let found = ActuatorKind::new(protocol, ModelNumber(number), 0, id);\n\
             let error = SetupError::UnsupportedModel { id, model_number: number, protocol, feature: None };\n\
             assert_eq!(found, Err(error));\n",
        )
    );
}

/// `for {pattern} in [{items}] {{ {body} }}`, or `let {pattern} = {item};
/// {body}` when there is a single item, so a one-model family does not trip
/// `clippy::single_element_loop`.
fn each(pattern: &str, items: &[String], body: &str) -> String {
    match items {
        [item] => format!("let {pattern} = {item};\n{body}"),
        _ => format!("for {pattern} in [{}] {{\n{body}}}\n", items.join(", ")),
    }
}

/// Per family, with the family off: every (protocol, number) with tables
/// names the family, and needs no firmware read.
fn render_family_off(out: &mut String, resolved: &Resolved) {
    let mut families: Vec<&str> = resolved.lookups.iter().map(|l| l.family.as_str()).collect();
    families.sort_unstable();
    families.dedup();
    for family in families {
        let pairs: Vec<String> = resolved
            .lookups
            .iter()
            .filter(|l| l.family == family)
            .map(|l| format!("(ProtocolVersion::{}, {})", l.protocol.variant(), l.number))
            .collect();
        let body = format!(
            "let found = ActuatorKind::new(protocol, ModelNumber(number), 0, id);\n\
             let error = SetupError::UnsupportedModel {{ id, model_number: number, protocol, feature: Some(\"{family}\") }};\n\
             assert_eq!(found, Err(error));\n\
             assert_eq!(ActuatorKind::firmware_register(protocol, ModelNumber(number)), None);\n"
        );
        let _ = writeln!(
            out,
            "#[cfg(not(feature = \"{family}\"))]\n\
             #[test]\n\
             fn {name}_off() {{\n\
             \x20   let id = id();\n\
             {checks}\
             }}\n",
            name = family.replace('-', "_"),
            checks = each("(protocol, number)", &pairs, &body),
        );
    }
}

/// The firmware values a lookup list is checked at: 0, each boundary and
/// the value before it, and 255.
fn firmware_points(list: &LookupList) -> Vec<u8> {
    let mut points = BTreeSet::from([0, u8::MAX]);
    for &(from, _) in &list.variants {
        if from > 0 {
            points.insert(from);
            points.insert(from - 1);
        }
    }
    points.into_iter().collect()
}

/// The lookup of one (protocol, model number) with its family on.
fn render_lookup(out: &mut String, list: &LookupList, reads_firmware: bool) {
    let protocol = format!("ProtocolVersion::{}", list.protocol.variant());
    let number = list.number;
    let types: BTreeSet<&str> = list.variants.iter().map(|(_, ty)| ty.as_str()).collect();
    let cases: Vec<String> = firmware_points(list)
        .into_iter()
        .map(|fw| {
            let ty = list
                .variants
                .iter()
                .rev()
                .find(|(from, _)| *from <= fw)
                .map_or("", |(_, ty)| ty.as_str());
            format!("({fw}, ActuatorKind::{ty}({ty}::new(id)))")
        })
        .collect();
    let firmware_register = match list.variants.first() {
        Some((_, first)) if reads_firmware => format!("Some({first}::FIRMWARE_VERSION)"),
        _ => "None".to_string(),
    };
    let _ = writeln!(
        out,
        "#[cfg(feature = \"{family}\")]\n\
         #[test]\n\
         fn {p}_{number}_lookup() {{\n\
         \x20   use crate::actuator::Describe;\n\
         \x20   use crate::models::{{{types}}};\n\n\
         \x20   let id = id();\n\
         \x20   for (firmware, expected) in [{cases}] {{\n\
         \x20       assert!(expected.data().protocols.supports({protocol}));\n\
         \x20       assert_eq!(ActuatorKind::new({protocol}, ModelNumber({number}), firmware, id), Ok(expected));\n\
         \x20   }}\n\
         \x20   assert_eq!(ActuatorKind::firmware_register({protocol}, ModelNumber({number})), {firmware_register});\n\
         }}\n",
        family = list.family,
        p = list.protocol.list(),
        types = types.into_iter().collect::<Vec<_>>().join(", "),
        cases = cases.join(", "),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::fixtures::{Map, Table, resolved};

    fn two_models() -> Resolved {
        resolved(
            &Map::new()
                .model(12, "ax", "AX-12A")
                .p1(&[(0, "ax")])
                .model(1020, "x-series", "XM430-W350")
                .p1(&[(0, "xm")])
                .p2(&[(0, "xm")])
                .default_p2(),
            &[("ax", Table::ax()), ("xm", Table::xm())],
        )
    }

    fn compact(text: &str) -> String {
        text.split_whitespace().collect()
    }

    #[test]
    fn round_trip_per_model_type() {
        let text = render_tests(&two_models()).unwrap();
        assert!(text.starts_with(HEADER));
        let c = compact(&text);
        for (family, module, ty) in [("ax", "ax", "Ax"), ("x-series", "xm", "Xm")] {
            assert!(
                c.contains(&format!(
                    "#[cfg(feature=\"{family}\")]#[test]fn{module}_describe_round_trip(){{usecrate::actuator::{{Cache,Describe,LimitMode}};"
                )),
                "{module}"
            );
            for part in [
                format!("letmutkind=ActuatorKind::{ty}({ty}::new(id));"),
                format!("assert_eq!(super::MODEL_NUMBER,{ty}::MODEL_NUMBER);"),
                format!("assert_eq!(kind.data(),&{ty}::DATA);"),
            ] {
                assert!(c.contains(&part), "{part}");
            }
        }
        for part in [
            "assert_eq!(kind.cache().id(),id);",
            "kind.cache_mut().record_mode(LimitMode::Unbounded);",
            "assert!(kind.cache_mut().record_limits(10,20).is_ok());",
            "assert_eq!(limits,Some((10,20)));",
            "assert_eq!(*kind.cache(),expected);",
        ] {
            assert!(c.contains(part), "{part}");
        }
    }

    #[test]
    fn names_no_register_address() {
        let text = render_tests(&two_models()).unwrap();
        assert!(!text.contains("reg("));
        let addresses = [11, 24, 30, 36, 48, 52, 64, 70, 116, 132];
        for token in text.split(|c: char| !c.is_ascii_digit()) {
            if let Ok(n) = token.parse::<u32>() {
                assert!(!addresses.contains(&n), "address {n} in the tests");
            }
        }
    }

    #[test]
    fn no_models_no_tests() {
        assert!(render_tests(&Resolved::default()).is_none());
    }

    fn firmware_model() -> Resolved {
        resolved(
            &Map::new()
                .model(350, "x-series", "XL-320")
                .p2(&[(0, "xl320")])
                .model(1190, "x-series", "XL330-M077")
                .p2(&[(0, "fw52"), (53, "new")]),
            &[
                ("xl320", Table::xl320()),
                ("fw52", Table::xm().protocols("[P2]")),
                ("new", Table::xm().protocols("[P2]")),
            ],
        )
    }

    #[test]
    fn lookup_at_every_boundary() {
        let c = compact(&render_tests(&firmware_model()).unwrap());
        assert!(c.contains("#[cfg(feature=\"x-series\")]#[test]fnp2_1190_lookup(){usecrate::actuator::Describe;usecrate::models::{Fw52,New};"));
        assert!(c.contains(
            "for(firmware,expected)in[(0,ActuatorKind::Fw52(Fw52::new(id))),(52,ActuatorKind::Fw52(Fw52::new(id))),(53,ActuatorKind::New(New::new(id))),(255,ActuatorKind::New(New::new(id)))]{assert!(expected.data().protocols.supports(ProtocolVersion::P2));assert_eq!(ActuatorKind::new(ProtocolVersion::P2,ModelNumber(1190),firmware,id),Ok(expected));}"
        ));
        assert!(c.contains("assert_eq!(ActuatorKind::firmware_register(ProtocolVersion::P2,ModelNumber(1190)),Some(Fw52::FIRMWARE_VERSION));"));
        assert!(c.contains("for(firmware,expected)in[(0,ActuatorKind::Xl320(Xl320::new(id))),(255,ActuatorKind::Xl320(Xl320::new(id)))]"));
        assert!(c.contains("assert_eq!(ActuatorKind::firmware_register(ProtocolVersion::P2,ModelNumber(350)),None);"));
    }

    #[test]
    fn unknown_and_missing_protocol() {
        let c = compact(&render_tests(&firmware_model()).unwrap());
        assert!(c.contains("#[test]fnunknown_model_number(){"));
        assert!(c.contains("ActuatorKind::new(protocol,ModelNumber(65535),0,id)"));
        assert!(
            c.contains("SetupError::UnsupportedModel{id,model_number:65535,protocol,feature:None}")
        );
        assert!(c.contains(
            "for(protocol,number)in[(ProtocolVersion::P1,350),(ProtocolVersion::P1,1190)]"
        ));
        assert!(c.contains(
            "SetupError::UnsupportedModel{id,model_number:number,protocol,feature:None}"
        ));
    }

    #[test]
    fn unknown_number_skips_mapped_ones() {
        let r = resolved(
            &Map::new().model(65535, "ax", "A").p1(&[(0, "ax")]),
            &[("ax", Table::ax())],
        );
        let c = compact(&render_tests(&r).unwrap());
        assert!(c.contains("ModelNumber(65534)"));
        assert!(c.contains("let(protocol,number)=(ProtocolVersion::P2,65535);"));
    }

    #[test]
    fn top_level_imports() {
        let text = render_tests(&firmware_model()).unwrap();
        let top: Vec<&str> = text.lines().filter(|l| l.starts_with("use ")).collect();
        assert_eq!(
            top,
            [
                "use super::ActuatorKind;",
                "use crate::actuator::ModelNumber;",
                "use crate::error::SetupError;",
                "use crate::id::Id;",
                "use crate::protocol::ProtocolVersion;",
            ]
        );
    }

    #[test]
    fn family_off_names_the_family() {
        let c = compact(&render_tests(&two_models()).unwrap());
        assert!(c.contains(
            "#[cfg(not(feature=\"x-series\"))]#[test]fnx_series_off(){letid=id();for(protocol,number)in[(ProtocolVersion::P1,1020),(ProtocolVersion::P2,1020)]{"
        ));
        assert!(c.contains("feature:Some(\"x-series\")};assert_eq!(found,Err(error));assert_eq!(ActuatorKind::firmware_register(protocol,ModelNumber(number)),None);"));
        assert!(c.contains("#[cfg(not(feature=\"ax\"))]#[test]fnax_off(){letid=id();let(protocol,number)=(ProtocolVersion::P1,12);letfound="));
        assert!(!c.contains("in[(ProtocolVersion::P1,12)]"));
    }

    #[test]
    fn each_loops_only_over_several_items() {
        let one = ["1".to_string()];
        let two = ["1".to_string(), "2".to_string()];
        assert_eq!(each("x", &one, "f(x);\n"), "let x = 1;\nf(x);\n");
        assert_eq!(each("x", &two, "f(x);\n"), "for x in [1, 2] {\nf(x);\n}\n");
    }

    fn fr9_models() -> Resolved {
        resolved(
            &Map::new()
                .model(12, "ax", "AX-12A")
                .p1(&[(0, "ax")])
                .model(310, "mx", "MX-64")
                .p1(&[(0, "mx")])
                .model(1020, "x-series", "XM430-W350")
                .p1(&[(0, "xm")])
                .p2(&[(0, "xm")])
                .default_p2(),
            &[
                ("ax", Table::ax_fr9()),
                ("mx", Table::mx_p1()),
                ("xm", Table::xm_fr9()),
            ],
        )
    }

    #[test]
    fn roles_tests_per_model() {
        let c = compact(&render_tests(&fr9_models()).unwrap());
        for part in [
            "#[cfg(any(feature=\"ax\",feature=\"mx\",feature=\"x-series\"))]fnclose(got:f32,want:f32)->bool{",
            "fnsame_role(found:Option<&crate::actuator::RoleRegister>,",
            "#[cfg(feature=\"ax\")]#[test]fnax_roles(){usecrate::actuator::{HasCcwComplianceMargin,",
            "usecrate::models::Ax;usecrate::units::{Sign,SpeedLimit};letdata=&Ax::DATA;",
            "(Role::SpeedLimit,<AxasHasSpeedLimit>::ROLE,Ax::MOVING_SPEED,\"MovingSpeed\"),",
            "]{same_role(data.role(role),tier2,register,name);}",
            "for role in",
            "letrole=<AxasHasSpeedLimit>::ROLE;assert_eq!(role.from_firmware(),0);assert_eq!(role.bounded_by(),None);",
            "letscale=role.scale(LimitMode::Bounded).unwrap();assert_eq!((scale.sign(),scale.size(),scale.zero_is_unlimited()),(Sign::Unsigned,2,true));assert!(close(scale.velocity(100).rad_per_s(),1.1623893));",
            "assert_eq!(scale.speed_limit(0),SpeedLimit::Unlimited);",
            "assert_eq!(role.raw_range(LimitMode::Bounded),Some(0..=1023));",
            "assert_eq!(data.typed(Role::SpeedLimit,LimitMode::Bounded),Some((Ax::MOVING_SPEED,Some(scale))));",
            "assert!(close(scale.velocity(1124).rad_per_s(),-1.1623893));",
            "assert!(close(scale.load(100).ratio(),0.097751714));",
            "assert!(close(scale.load(1124).ratio(),-0.097751714));",
            "assert!(close(scale.si_of(100),0.5061455));",
            "letrole=<AxasHasGoalVelocity>::ROLE;assert_eq!(role.from_firmware(),0);assert_eq!(role.bounded_by(),None);{formodein[LimitMode::Bounded,LimitMode::Unbounded,LimitMode::Pwm,LimitMode::UnboundedCurrent,LimitMode::Velocity,LimitMode::Current]{assert_eq!(role.scale(mode),None);assert_eq!(role.raw_range(mode),None);assert_eq!(data.typed(Role::GoalVelocity,mode),Some((Ax::MOVING_SPEED,None)));}}{letmode=LimitMode::Wheel;assert_eq!(role.scale(mode),None);assert_eq!(role.raw_range(mode),Some(0..=2047));assert_eq!(data.typed(Role::GoalVelocity,mode),Some((Ax::MOVING_SPEED,None)));}",
            "(Role::CwComplianceSlope,<AxasHasCwComplianceSlope>::ROLE,Ax::CW_COMPLIANCE_SLOPE,Some(0..=254)),",
            "assert_eq!((tier2.from_firmware(),tier2.bounded_by()),(0,None));formodeinALL_MODES{assert_eq!(tier2.scale(mode),None);assert_eq!(tier2.raw_range(mode),range);assert_eq!(data.typed(role,mode),Some((register,None)));}",
            "formodein[LimitMode::Unbounded,LimitMode::Pwm,LimitMode::UnboundedCurrent,LimitMode::Velocity,LimitMode::Current,LimitMode::Wheel]{assert_eq!(role.scale(mode),None);assert_eq!(role.raw_range(mode),None);assert_eq!(data.typed(Role::SpeedLimit,mode),Some((Ax::MOVING_SPEED,None)));}",
            "letrole=<MxasHasRealtimeTick>::ROLE;assert_eq!(role.from_firmware(),40);",
            "assert_eq!(role.tick_modulus(LimitMode::Bounded),Some(32768));",
            "assert!(close(scale.current(1124).amps(),-0.45));",
            "assert_eq!(role.bounded_by(),Some(Xm::VELOCITY_LIMIT));",
            "assert!(close(scale.velocity(4294967196).rad_per_s(),-2.3980825));",
            "assert!(close(scale.current(65436).amps(),-0.269));",
            "assert!(close(scale.temperature(100).celsius(),100.0));",
            "assert!(close(scale.voltage(100).volts(),10.0));",
        ] {
            let part = compact(part);
            assert!(c.contains(&part), "{part}");
        }
        assert!(c.contains("fnmx_roles()"));
        assert!(c.contains("fnxm_roles()"));
    }

    #[test]
    fn mode_tests_per_model() {
        let c = compact(&render_tests(&fr9_models()).unwrap());
        assert!(c.contains("#[cfg(feature=\"ax\")]#[test]fnax_mode_source(){usecrate::models::Ax;usecrate::units::Limits;letmode=Ax::DATA.mode;letlimits=Limits::new(id(),0,1).unwrap();assert_eq!(mode.register(),None);"));
        assert!(c.contains("assert_eq!(mode.register(),Some(Mx::TORQUE_CTRL_MODE_ENABLE));assert_eq!(mode.limit_mode(1,limits),Some(LimitMode::Current));assert_eq!(mode.limit_mode(0,limits),Some(LimitMode::from_angle_limits(limits)));assert_eq!(mode.limit_mode(2,limits),None);"));
        assert!(c.contains("for(value,expected)in[(0,Some(LimitMode::Current)),(1,Some(LimitMode::Velocity)),(3,Some(LimitMode::Bounded)),(4,Some(LimitMode::Unbounded)),(5,Some(LimitMode::UnboundedCurrent)),(16,Some(LimitMode::Pwm)),(2,None)]"));
        let coarse = compact(&render_tests(&two_models()).unwrap());
        assert!(coarse.contains("(0,Some(LimitMode::Pwm)),(1,Some(LimitMode::Pwm)),(3,Some(LimitMode::Bounded)),(4,Some(LimitMode::Unbounded)),(5,Some(LimitMode::Unbounded)),(16,Some(LimitMode::Pwm)),(2,None)"));
    }

    #[test]
    fn raw_values_per_sign() {
        assert_eq!(raw_of(-100, SignKind::DirectionBit(10), 2), 1124);
        assert_eq!(raw_of(100, SignKind::DirectionBit(10), 2), 100);
        assert_eq!(raw_of(-100, SignKind::TwosComplement, 2), 65_436);
        assert_eq!(raw_of(-100, SignKind::TwosComplement, 4), 4_294_967_196);
        assert_eq!(raw_of(100, SignKind::Unsigned, 1), 100);
        assert_eq!(f32_of(0.1), 0.1_f32);
    }

    #[test]
    fn concrete_match_per_model_type() {
        let c = compact(&render_tests(&firmware_model()).unwrap());
        assert!(c.contains(
            "#[cfg(feature=\"x-series\")]#[test]fnnew_concrete_match(){usecrate::actuator::Actuator;usecrate::models::New;letid=id();letkind=ActuatorKind::new(ProtocolVersion::P2,ModelNumber(1190),53,id);assert!(matches!(&kind,Ok(ActuatorKind::New(m))ifm.id()==id&&m.name()==New::DATA.name&&m.model_number()==New::DATA.model_number));}"
        ));
        assert!(c.contains("fnfw52_concrete_match()"));
        assert!(c.contains("ActuatorKind::new(ProtocolVersion::P2,ModelNumber(350),0,id)"));
    }
}
