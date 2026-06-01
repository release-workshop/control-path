/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 *
 * Compile v2 boolean catalog documents into AST artifacts.
 */

use std::collections::BTreeMap;

use crate::ast::Artifact;
use crate::catalog::{
    validate_catalog, CatalogDocument, CatalogMode, CatalogValidationContext,
    CatalogValidationResult, Rule as CatalogRule,
};
use crate::compiler;
use crate::error::{CompilationError, CompilerError, ValidationError};

/// Compile a local-mode v2 catalog for the given environment into an AST artifact.
///
/// **Local flags only.** Catalogs with `imports` must use [`compile_catalog_with_imports`]
/// (or the `*_with_imports` validate-and-compile helpers) after resolving import paths.
///
/// Environment rules come from `environments.<env>.rules`. Flags without rules for the
/// environment receive only a trailing default serve rule from the catalog `default`.
/// Top-level `segments` are included in the artifact projection.
///
/// # Validation
///
/// This is a low-level entry point: it does **not** run JSON Schema or semantic catalog
/// validation. Callers that accept untrusted YAML must use [`validate_and_compile_catalog`]
/// or [`load_validate_and_compile_catalog`] instead, passing resolved imports when the
/// catalog declares any. Even with a typed [`CatalogDocument`], prefer those helpers
/// unless the catalog was already validated in the same pipeline.
///
/// Empty rules (neither `serve` nor `rollout`), rules with both `serve` and `rollout`,
/// and rollout percentages outside `0..=100` are rejected at compile time so invalid
/// state cannot silently drop rules or rely on legacy serve-wins behavior.
///
/// Optional rule `reason` is catalog metadata only; it is not stored in the AST (see issue 10
/// for explain/audit surfaces that read reason from source YAML).
///
/// # Errors
///
/// Returns [`CompilerError::Compilation`] when `catalog.mode` is [`CatalogMode::Saas`]
/// or when rule compilation fails.
pub fn compile_catalog(
    catalog: &CatalogDocument,
    environment: &str,
) -> Result<Artifact, CompilerError> {
    compile_catalog_with_imports(catalog, &BTreeMap::new(), environment)
}

/// Compile a local-mode v2 catalog and resolved imports for the given environment.
///
/// Local flags use rules from the service catalog's `environments.<env>`. Imported flags
/// are qualified as `{import_namespace}.{flag_key}` and use rules from the matching
/// environment in each source catalog. Segments from imported catalogs are included when
/// their names do not collide with service-catalog segments.
pub fn compile_catalog_with_imports(
    catalog: &CatalogDocument,
    imports: &BTreeMap<String, CatalogDocument>,
    environment: &str,
) -> Result<Artifact, CompilerError> {
    if catalog.mode == CatalogMode::Saas {
        return Err(CompilerError::Compilation(CompilationError::InvalidRule(
            "SaaS mode catalogs have no local environments to compile".to_string(),
        )));
    }

    let definitions = catalog_to_definitions(catalog, imports);
    let deployment = catalog_to_deployment(catalog, imports, environment)?;
    compiler::compile(&deployment, &definitions)
}

/// Validate a catalog, then compile an environment into an AST artifact.
///
/// Preferred entry point when the catalog has not yet been validated in this pipeline.
/// Pass resolved import documents when `catalog.imports` is non-empty; an empty map is
/// correct only for catalogs with no imports.
pub fn validate_and_compile_catalog(
    file_path: &str,
    catalog: &CatalogDocument,
    imports: &BTreeMap<String, CatalogDocument>,
    environment: &str,
    ctx: &CatalogValidationContext,
) -> Result<Artifact, CompilerError> {
    let validation = validate_catalog(
        file_path,
        catalog,
        &effective_validation_context(ctx, imports),
    );
    ensure_catalog_valid(validation)?;
    compile_catalog_with_imports(catalog, imports, environment)
}

/// Parse, validate, and compile catalog content into an AST artifact.
///
/// End-to-end entry point for untrusted catalog YAML (mirrors [`super::load_and_validate_catalog`]).
/// Import resolution is caller-owned (no file I/O in the compiler); pass resolved imports
/// when the catalog declares any.
pub fn load_validate_and_compile_catalog(
    content: &str,
    file_path: &str,
    imports: &BTreeMap<String, CatalogDocument>,
    environment: &str,
    ctx: &CatalogValidationContext,
) -> Result<Artifact, CompilerError> {
    let effective_ctx = effective_validation_context(ctx, imports);
    let (catalog, validation) =
        super::load_and_validate_catalog(content, file_path, &effective_ctx)
            .map_err(|e| CompilerError::Parse(e.into()))?;
    ensure_catalog_valid(validation)?;
    compile_catalog_with_imports(&catalog, imports, environment)
}

fn effective_validation_context(
    ctx: &CatalogValidationContext,
    imports: &BTreeMap<String, CatalogDocument>,
) -> CatalogValidationContext {
    if ctx.imported_flag_keys.is_empty() && !imports.is_empty() {
        CatalogValidationContext {
            workspace: ctx.workspace.clone(),
            imported_flag_keys: super::imported_flag_keys_from_imports(imports),
        }
    } else {
        ctx.clone()
    }
}

fn ensure_catalog_valid(result: CatalogValidationResult) -> Result<(), CompilerError> {
    if result.is_ok() {
        return Ok(());
    }
    let messages: Vec<String> = result.errors.iter().map(|e| e.message.clone()).collect();
    Err(CompilerError::Validation(
        ValidationError::SchemaValidation(messages.join("; ")),
    ))
}

fn catalog_to_definitions(
    catalog: &CatalogDocument,
    imports: &BTreeMap<String, CatalogDocument>,
) -> serde_json::Value {
    let mut flags: Vec<serde_json::Value> = catalog
        .flags
        .iter()
        .map(|(name, flag)| {
            serde_json::json!({
                "name": name,
                "type": "boolean",
                "defaultValue": flag.default,
            })
        })
        .collect();

    for (import_namespace, imported) in imports {
        for (flag_key, flag) in &imported.flags {
            flags.push(serde_json::json!({
                "name": format!("{import_namespace}.{flag_key}"),
                "type": "boolean",
                "defaultValue": flag.default,
            }));
        }
    }

    serde_json::json!({ "flags": flags })
}

fn catalog_to_deployment(
    catalog: &CatalogDocument,
    imports: &BTreeMap<String, CatalogDocument>,
    environment: &str,
) -> Result<serde_json::Value, CompilerError> {
    let mut rules = serde_json::Map::new();

    if let Some(env) = catalog.environments.get(environment) {
        for (flag_name, flag_rules) in &env.rules {
            if !flag_rules.is_empty() {
                let legacy_rules: Vec<serde_json::Value> = flag_rules
                    .iter()
                    .enumerate()
                    .map(|(index, rule)| prepare_catalog_rule(rule, flag_name, index))
                    .collect::<Result<_, _>>()?;
                rules.insert(
                    flag_name.clone(),
                    serde_json::json!({ "rules": legacy_rules }),
                );
            }
        }
    }

    for (import_namespace, imported) in imports {
        if let Some(env) = imported.environments.get(environment) {
            for (flag_key, flag_rules) in &env.rules {
                if flag_rules.is_empty() {
                    continue;
                }
                let qualified = format!("{import_namespace}.{flag_key}");
                let legacy_rules: Vec<serde_json::Value> = flag_rules
                    .iter()
                    .enumerate()
                    .map(|(index, rule)| prepare_catalog_rule(rule, &qualified, index))
                    .collect::<Result<_, _>>()?;
                rules.insert(qualified, serde_json::json!({ "rules": legacy_rules }));
            }
        }
    }

    let mut deployment = serde_json::json!({
        "environment": environment,
        "rules": rules,
    });

    let mut segments: serde_json::Map<String, serde_json::Value> = catalog
        .segments
        .iter()
        .map(|(name, segment)| (name.clone(), serde_json::json!({ "when": segment.when })))
        .collect();

    let mut segment_sources: BTreeMap<String, String> = catalog
        .segments
        .keys()
        .map(|name| (name.clone(), "service catalog".to_string()))
        .collect();

    for (import_namespace, imported) in imports {
        for (name, segment) in &imported.segments {
            if let Some(existing) = segment_sources.get(name) {
                let message = if existing == "service catalog" {
                    format!(
                        "Segment '{name}' is defined in both the service catalog and import '{import_namespace}'"
                    )
                } else {
                    format!(
                        "Segment '{name}' is defined in both import '{existing}' and import '{import_namespace}'"
                    )
                };
                return Err(CompilerError::Compilation(CompilationError::InvalidRule(
                    message,
                )));
            }
            segment_sources.insert(name.clone(), import_namespace.clone());
            segments.insert(name.clone(), serde_json::json!({ "when": segment.when }));
        }
    }

    if !segments.is_empty() {
        deployment["segments"] = serde_json::Value::Object(segments);
    }

    Ok(deployment)
}

fn prepare_catalog_rule(
    rule: &CatalogRule,
    flag_name: &str,
    rule_index: usize,
) -> Result<serde_json::Value, CompilerError> {
    if rule.serve.is_none() && rule.rollout.is_none() {
        return Err(CompilerError::Compilation(CompilationError::InvalidRule(
            format!(
                "Flag \"{flag_name}\" rule {} must specify serve or rollout",
                rule_index + 1
            ),
        )));
    }

    if rule.serve.is_some() && rule.rollout.is_some() {
        return Err(CompilerError::Compilation(CompilationError::InvalidRule(
            format!(
                "Flag \"{flag_name}\" rule {} must specify serve or rollout, not both",
                rule_index + 1
            ),
        )));
    }

    if let Some(rollout) = &rule.rollout {
        validate_rollout_percentage(rollout.percentage, flag_name, rule_index)?;
    }

    Ok(catalog_rule_to_legacy_json(rule))
}

fn validate_rollout_percentage(
    percentage: f64,
    flag_name: &str,
    rule_index: usize,
) -> Result<(), CompilerError> {
    if !(0.0..=100.0).contains(&percentage) {
        return Err(CompilerError::Compilation(CompilationError::InvalidRule(
            format!(
                "Flag \"{flag_name}\" rule {} rollout percentage must be between 0 and 100 (got {percentage})",
                rule_index + 1
            ),
        )));
    }
    Ok(())
}

fn catalog_rule_to_legacy_json(rule: &CatalogRule) -> serde_json::Value {
    let mut obj = serde_json::Map::new();

    if let Some(when) = &rule.when {
        obj.insert("when".to_string(), serde_json::Value::String(when.clone()));
    }

    if let Some(serve) = rule.serve {
        obj.insert("serve".to_string(), serde_json::Value::Bool(serve));
    }

    if let Some(rollout) = &rule.rollout {
        obj.insert(
            "rollout".to_string(),
            serde_json::json!({
                "percentage": rollout.percentage,
                "variation": bool_to_rollout_variation(rollout.serve),
            }),
        );
    }

    serde_json::Value::Object(obj)
}

fn bool_to_rollout_variation(serve: bool) -> &'static str {
    if serve {
        "ON"
    } else {
        "OFF"
    }
}

#[cfg(test)]
#[path = "compile_tests.rs"]
mod compile_tests;
