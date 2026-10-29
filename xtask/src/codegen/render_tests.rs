//! Renders `src/models/tests.rs`: crate tests of the generated lookup and
//! model data against every model number of the real tables.
//!
//! The tests name registers and model data only by constant, never by a
//! literal address, so a one-row table change still changes one file.

use std::fmt::Write as _;

use super::HEADER;
use super::resolve::Resolved;

/// The text of `src/models/tests.rs`, or `None` with no model.
pub(crate) fn render_tests(resolved: &Resolved) -> Option<String> {
    if resolved.models.is_empty() {
        return None;
    }
    let mut out = String::from(HEADER);
    out.push_str(
        "//! Tests of the generated models and lookup, one or more per model number and\n\
         //! model type of `control_tables/`.\n\n",
    );
    for model in &resolved.models {
        let ty = &model.type_ident;
        let _ = writeln!(
            out,
            "#[cfg(feature = \"{family}\")]\n\
             #[test]\n\
             fn {module}_describe_round_trip() {{\n\
             \x20   use crate::actuator::{{Cache, Describe, LimitMode}};\n\
             \x20   use crate::models::{ty};\n\n\
             \x20   let id = crate::id::Id::new(1).unwrap();\n\
             \x20   let mut kind = crate::models::ActuatorKind::{ty}({ty}::new(id));\n\
             \x20   assert_eq!(kind.data(), &{ty}::DATA);\n\
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
                format!("letmutkind=crate::models::ActuatorKind::{ty}({ty}::new(id));"),
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
}
