//! Rust identifiers built from table file stems and register names.

/// `xm430_w350` -> `Xm430W350`. A leading digit is spelled out
/// (`2xl430_w250` -> `TwoXl430W250`) since identifiers can't start with one.
pub(crate) fn type_ident(file: &str) -> String {
    spell_leading_digit(file)
        .split('_')
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_ascii_uppercase().to_string() + chars.as_str()
            })
        })
        .collect()
}

/// `xm430_w350` -> `xm430_w350`, `2xl430_w250` -> `two_xl430_w250`.
pub(crate) fn module_ident(file: &str) -> String {
    spell_leading_digit(file)
}

fn spell_leading_digit(file: &str) -> String {
    const DIGITS: [&str; 10] = [
        "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
    ];
    let mut chars = file.chars();
    match chars.next().and_then(|c| c.to_digit(10)) {
        Some(d) => format!("{}_{}", DIGITS[d as usize], chars.as_str()),
        None => file.to_string(),
    }
}

/// `Secondary(Shadow) ID` -> `SECONDARY_SHADOW_ID`.
pub(crate) fn const_ident(data_name: &str) -> String {
    data_name
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(str::to_ascii_uppercase)
        .collect::<Vec<_>>()
        .join("_")
}

/// Whether `ident` cannot name a generated model module: `tests` is taken by
/// the generated test module, and Rust 2024 keywords (strict and reserved)
/// are not identifiers.
#[allow(dead_code, reason = "check 16 uses it from US4")]
pub(crate) fn is_reserved_module(ident: &str) -> bool {
    const RESERVED: &[&str] = &[
        "tests", "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false",
        "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
        "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type",
        "unsafe", "use", "where", "while", "async", "await", "dyn", "abstract", "become", "box",
        "do", "final", "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "try",
        "gen",
    ];
    RESERVED.contains(&ident)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_names() {
        assert_eq!(type_ident("xm430_w350"), "Xm430W350");
        assert_eq!(type_ident("xl320"), "Xl320");
        assert_eq!(type_ident("2xl430_w250"), "TwoXl430W250");
        assert_eq!(type_ident("xl330_m077_fw52"), "Xl330M077Fw52");
    }

    #[test]
    fn module_names() {
        assert_eq!(module_ident("xm430_w350"), "xm430_w350");
        assert_eq!(module_ident("2xc430_w250"), "two_xc430_w250");
    }

    #[test]
    fn const_names() {
        assert_eq!(const_ident("Goal Position"), "GOAL_POSITION");
        assert_eq!(const_ident("Secondary(Shadow) ID"), "SECONDARY_SHADOW_ID");
        assert_eq!(const_ident("Baud Rate (Bus)"), "BAUD_RATE_BUS");
        assert_eq!(
            const_ident("In-Position Threshold"),
            "IN_POSITION_THRESHOLD"
        );
        assert_eq!(const_ident("Indirect Address 1"), "INDIRECT_ADDRESS_1");
    }

    #[test]
    fn two_xc430_w250() {
        assert_eq!(type_ident("2xc430_w250"), "TwoXc430W250");
        assert_eq!(module_ident("2xc430_w250"), "two_xc430_w250");
    }

    #[test]
    fn reserved_module_idents() {
        for ident in ["tests", "type", "mod", "async", "gen"] {
            assert!(is_reserved_module(ident), "{ident}");
        }
        assert!(!is_reserved_module("xm430_w350"));
    }
}
