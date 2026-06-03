/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 *
 * Build SDK catalog projections from v2 boolean catalogs and resolved imports.
 */

use std::collections::{BTreeMap, BTreeSet};

use crate::catalog::model::{
    AttributeScalarType, CatalogDocument, FlagDefinition, FlagKind, FlagLifecycle,
};

/// One boolean flag exposed through the generated SDK.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkFlag {
    /// Stable reference for rules and artifact lookup (`flag` or `namespace.flag`).
    pub qualified_name: String,
    /// TypeScript method and `FlagName` literal (camelCase).
    pub sdk_method_name: String,
    pub default: bool,
    pub kind: FlagKind,
    pub lifecycle: FlagLifecycle,
    pub description: Option<String>,
    /// Imported from another catalog; rules are compiled from the source catalog.
    pub is_imported: bool,
    /// Import namespace when [`SdkFlag::is_imported`] is true.
    pub import_namespace: Option<String>,
}

/// Declared attribute fields for one import namespace (when that catalog opted in).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkAttributeNamespace {
    pub namespace: String,
    pub fields: BTreeMap<String, AttributeScalarType>,
}

/// Closed attribute schema for SDK generation when the service catalog opts in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkAttributeSchema {
    pub service_fields: BTreeMap<String, AttributeScalarType>,
    pub import_namespaces: Vec<SdkAttributeNamespace>,
}

/// SDK input derived from catalog flag definitions and kill switch URL configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkCatalog {
    pub flags: Vec<SdkFlag>,
    /// Per-environment kill switch file URLs (`kill_switches.<env>.url` in local mode).
    pub kill_switch_urls: BTreeMap<String, String>,
    /// Per-environment compiled artifact URLs (`artifacts.<env>.url` in local mode).
    pub artifact_urls: BTreeMap<String, String>,
    /// Present when the service catalog opts in to `attributes:` (closed SDK typing).
    pub attribute_schema: Option<SdkAttributeSchema>,
}

/// Build the SDK catalog from a local catalog and resolved import documents.
///
/// Environment rules, segments, and kill switch URLs are intentionally ignored.
///
/// # Errors
///
/// Returns a message when two flags resolve to the same qualified SDK name.
pub fn build_sdk_catalog(
    catalog: &CatalogDocument,
    imports: &BTreeMap<String, CatalogDocument>,
) -> Result<SdkCatalog, String> {
    let mut flags = Vec::new();
    let mut seen = BTreeSet::new();

    for (flag_key, flag) in &catalog.flags {
        push_flag(&mut flags, &mut seen, flag_key.clone(), flag, &None, false)?;
    }

    for (import_namespace, imported) in imports {
        for (flag_key, flag) in &imported.flags {
            let qualified = format!("{import_namespace}.{flag_key}");
            push_flag(
                &mut flags,
                &mut seen,
                qualified,
                flag,
                &Some(import_namespace.clone()),
                true,
            )?;
        }
    }

    ensure_unique_sdk_method_names(&flags)?;

    let kill_switch_urls = catalog
        .kill_switches
        .iter()
        .map(|(env, target)| (env.clone(), target.url.clone()))
        .collect();

    let artifact_urls = catalog
        .artifacts
        .iter()
        .map(|(env, target)| (env.clone(), target.url.clone()))
        .collect();

    let attribute_schema = build_attribute_schema(catalog, imports);

    Ok(SdkCatalog {
        flags,
        kill_switch_urls,
        artifact_urls,
        attribute_schema,
    })
}

fn build_attribute_schema(
    catalog: &CatalogDocument,
    imports: &BTreeMap<String, CatalogDocument>,
) -> Option<SdkAttributeSchema> {
    if !catalog.attribute_schema_opted_in() {
        return None;
    }

    let service_fields = catalog
        .attribute_schema_fields()
        .cloned()
        .unwrap_or_default();

    let mut import_namespaces = Vec::new();
    for (namespace, imported) in imports {
        let Some(fields) = imported.attribute_schema_fields() else {
            continue;
        };
        import_namespaces.push(SdkAttributeNamespace {
            namespace: namespace.clone(),
            fields: fields.clone(),
        });
    }
    import_namespaces.sort_by(|a, b| a.namespace.cmp(&b.namespace));

    Some(SdkAttributeSchema {
        service_fields,
        import_namespaces,
    })
}

fn ensure_unique_sdk_method_names(flags: &[SdkFlag]) -> Result<(), String> {
    let mut by_method: BTreeMap<&str, &str> = BTreeMap::new();
    for flag in flags {
        if let Some(other) = by_method.insert(&flag.sdk_method_name, &flag.qualified_name) {
            return Err(format!(
                "duplicate SDK method '{}' (from '{}' and '{}')",
                flag.sdk_method_name, other, flag.qualified_name
            ));
        }
    }
    Ok(())
}

fn push_flag(
    flags: &mut Vec<SdkFlag>,
    seen: &mut BTreeSet<String>,
    qualified_name: String,
    flag: &FlagDefinition,
    import_namespace: &Option<String>,
    is_imported: bool,
) -> Result<(), String> {
    if !seen.insert(qualified_name.clone()) {
        return Err(format!("duplicate SDK flag '{qualified_name}'"));
    }

    flags.push(SdkFlag {
        sdk_method_name: to_sdk_method_name(&qualified_name, import_namespace),
        qualified_name,
        default: flag.default,
        kind: flag.kind,
        lifecycle: flag.lifecycle,
        description: flag.description.clone(),
        is_imported,
        import_namespace: import_namespace.clone(),
    });
    Ok(())
}

fn to_sdk_method_name(qualified_name: &str, import_namespace: &Option<String>) -> String {
    if let Some(ns) = import_namespace {
        let flag_key = qualified_name
            .strip_prefix(&format!("{ns}."))
            .unwrap_or(qualified_name);
        format!("{}{}", ns, capitalize_first(&to_camel_case(flag_key)))
    } else {
        to_camel_case(qualified_name)
    }
}

fn capitalize_first(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn to_camel_case(snake: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;
    for c in snake.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_uppercase().next().unwrap_or(c));
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
#[path = "sdk_tests.rs"]
mod sdk_tests;
