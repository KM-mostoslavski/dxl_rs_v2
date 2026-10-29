//! Renders `src/models/tests.rs`: crate tests of the generated lookup and
//! model data against every model number of the real tables.
//!
//! The tests name registers and model data only by constant, never by a
//! literal address, so a one-row table change still changes one file.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use super::HEADER;
use super::resolve::{LookupList, Resolved};
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
    render_unknown(&mut out, resolved);
    for list in &resolved.lookups {
        render_lookup(&mut out, list);
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
    Some(out)
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
         \x20   for (protocol, number) in [{}] {{\n\
         \x20       let found = ActuatorKind::new(protocol, ModelNumber(number), 0, id);\n\
         \x20       let error = SetupError::UnsupportedModel {{ id, model_number: number, protocol, feature: None }};\n\
         \x20       assert_eq!(found, Err(error));\n\
         \x20   }}\n\
         }}\n",
        missing.join(", ")
    );
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
fn render_lookup(out: &mut String, list: &LookupList) {
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
        Some((_, first)) if list.firmware_dependent() => format!("Some({first}::FIRMWARE_VERSION)"),
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
        assert!(c.contains("for(protocol,number)in[(ProtocolVersion::P2,65535)]"));
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
}
