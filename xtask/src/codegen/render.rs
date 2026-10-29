//! Renders `src/models.rs` and one `src/models/<module>.rs` per model, as
//! unformatted Rust text (rustfmt runs later).

use std::fmt::Write as _;

use super::resolve::{GenModel, LookupList, Resolved};
use super::schema::ProtocolTag;
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
    files.push((MODULE_FILE.to_string(), render_module(resolved)));
    if let Some(tests) = render_tests::render_tests(resolved) {
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

fn render_model(model: &GenModel) -> String {
    let ty = &model.type_ident;
    let mut out = String::from(HEADER);
    let _ = writeln!(
        out,
        "// Source: control_tables/{}.ron\n\nuse crate::{{Actuator, Id, register::Register}};\n",
        model.file
    );
    let _ = writeln!(
        out,
        "/// {} (model number {}{}).\n#[derive(Debug, Clone, PartialEq, Eq)]\npub struct {ty} {{\n    id: Id,\n}}\n",
        model.name,
        model.number,
        firmware_doc(model)
    );
    let _ = writeln!(
        out,
        "impl {ty} {{\n    pub(crate) const fn new(id: Id) -> Self {{\n        Self {{ id }}\n    }}\n"
    );
    for (name, row) in &model.registers {
        let _ = writeln!(
            out,
            "    pub(crate) const {name}: Register = Register::new({}, {});",
            row.address, row.size
        );
    }
    out.push_str("}\n\n");
    let _ = writeln!(
        out,
        "impl Actuator for {ty} {{\n    fn id(&self) -> Id {{\n        self.id\n    }}\n\n    fn model_number(&self) -> u16 {{\n        {}\n    }}\n\n    fn name(&self) -> &'static str {{\n        {:?}\n    }}\n}}",
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

fn render_module(resolved: &Resolved) -> String {
    let mut out = String::from(HEADER);
    out.push_str(
        "//! Actuator model types, one per control table.\n\n\
         #![allow(\n    dead_code,\n    unused_variables,\n    reason = \"every register is generated but only some are wrapped; \
         feature-gated arms can leave parameters unused\"\n)]\n\n\
         use crate::{Actuator, Error, Id};\n\n",
    );

    for model in &resolved.models {
        let cfg = cfg_attr(&model.family);
        let _ = writeln!(
            out,
            "{cfg}\nmod {};\n{cfg}\npub use {}::{};",
            model.module_ident, model.module_ident, model.type_ident
        );
    }

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
                "            {}\n            Self::{}(ref a) => a.{method}(),",
                cfg_attr(&model.family),
                model.type_ident
            );
        }
        out.push_str("        }\n    }\n\n");
    }
    out.push_str("}\n");
    out
}
