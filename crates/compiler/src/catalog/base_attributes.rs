/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 *
 * Canonical base evaluation attribute names (`schemas/base-attributes.json`).
 * Runtime `BaseAttributes` and collision validation both derive from this list today;
 * SDK generation work (issues 05/06) may codegen TypeScript from the JSON file.
 */

use std::sync::OnceLock;

static NAMES: OnceLock<Vec<String>> = OnceLock::new();

fn loaded_names() -> &'static [String] {
    NAMES.get_or_init(|| {
        serde_json::from_str(include_str!("../../../../schemas/base-attributes.json"))
            .expect("schemas/base-attributes.json must be a JSON string array")
    })
}

/// Canonical base evaluation attribute names (`schemas/base-attributes.json`).
pub fn names() -> &'static [String] {
    loaded_names()
}

/// Returns true when `name` is a platform-owned base evaluation attribute.
pub fn contains(name: &str) -> bool {
    loaded_names().iter().any(|n| n == name)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    fn parse_base_attributes_interface_fields(block: &str) -> BTreeSet<String> {
        block
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                let name = line.split(':').next()?.trim().trim_end_matches('?');
                if name.is_empty() || !line.contains("?:") {
                    return None;
                }
                if !name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
                {
                    return None;
                }
                if !name.chars().next()?.is_ascii_lowercase() {
                    return None;
                }
                Some(name.to_string())
            })
            .collect()
    }

    #[test]
    fn base_attribute_names_match_runtime_base_attributes_interface() {
        let canonical: BTreeSet<String> =
            serde_json::from_str(include_str!("../../../../schemas/base-attributes.json")).unwrap();
        let ts = include_str!("../../../../runtime/typescript/src/types.ts");
        let start = ts
            .find("export interface BaseAttributes {")
            .expect("BaseAttributes interface");
        let rest = &ts[start..];
        let end = rest.find("\n}").expect("BaseAttributes closing brace");
        let block = &rest[..end];

        let interface_fields = parse_base_attributes_interface_fields(block);

        assert_eq!(
            interface_fields, canonical,
            "schemas/base-attributes.json must stay in sync with @controlpath/runtime BaseAttributes"
        );
    }
}
