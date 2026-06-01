/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 *
 * Compile v2 boolean catalog documents into AST artifacts.
 */

use std::collections::BTreeMap;

use crate::ast::{Artifact, Expression, RolloutPayload, RolloutValue, Rule, ServePayload};
use crate::catalog::{
    validate_catalog, CatalogDocument, CatalogMode, CatalogValidationContext,
    CatalogValidationResult, Rule as CatalogRule, Segment, ValidationMode,
};
use crate::compiler::expressions::parse_expression;
use crate::compiler::string_table::StringTable;
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

    lower_catalog_to_artifact(catalog, imports, environment)
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
        ValidationMode::Compile,
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
    let (catalog, validation) = super::load_and_validate_catalog(
        content,
        file_path,
        &effective_ctx,
        ValidationMode::Compile,
    )
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

struct FlagEntry<'a> {
    name: String,
    default: bool,
    rules: &'a [CatalogRule],
}

fn lower_catalog_to_artifact(
    catalog: &CatalogDocument,
    imports: &BTreeMap<String, CatalogDocument>,
    environment: &str,
) -> Result<Artifact, CompilerError> {
    reject_unknown_env_rule_keys(catalog, imports, environment)?;
    let merged_segments = merge_segments(catalog, imports)?;
    let flag_entries = collect_flag_entries(catalog, imports, environment);

    let mut string_table = StringTable::new();
    let segments = compile_segments(&merged_segments, &mut string_table)?;

    let mut compiled_flags: Vec<Vec<Rule>> = Vec::with_capacity(flag_entries.len());
    for entry in &flag_entries {
        let mut rules = compile_flag_rules(&entry.name, entry.rules, &mut string_table)?;
        append_default_serve_rule(entry.default, &mut rules, &mut string_table)?;
        compiled_flags.push(rules);
    }

    let mut flag_names: Vec<u16> = Vec::with_capacity(flag_entries.len());
    for entry in &flag_entries {
        flag_names.push(string_table.add(&entry.name)?);
    }

    Ok(Artifact {
        version: "1.0".to_string(),
        environment: environment.to_string(),
        string_table: string_table.to_vec(),
        flags: compiled_flags,
        flag_names,
        segments: if segments.is_empty() {
            None
        } else {
            Some(segments)
        },
        signature: None,
    })
}

fn reject_unknown_env_rule_keys(
    catalog: &CatalogDocument,
    imports: &BTreeMap<String, CatalogDocument>,
    environment: &str,
) -> Result<(), CompilerError> {
    if let Some(env) = catalog.environments.get(environment) {
        for (flag_key, flag_rules) in &env.rules {
            if flag_rules.is_empty() {
                continue;
            }
            if !catalog.flags.contains_key(flag_key.as_str()) {
                return Err(CompilerError::Compilation(CompilationError::InvalidRule(
                    format!("Flag \"{flag_key}\" not found in flag definitions"),
                )));
            }
        }
    }

    for (import_namespace, imported) in imports {
        if let Some(env) = imported.environments.get(environment) {
            for (flag_key, flag_rules) in &env.rules {
                if flag_rules.is_empty() {
                    continue;
                }
                if !imported.flags.contains_key(flag_key.as_str()) {
                    let qualified = format!("{import_namespace}.{flag_key}");
                    return Err(CompilerError::Compilation(CompilationError::InvalidRule(
                        format!("Flag \"{qualified}\" not found in flag definitions"),
                    )));
                }
            }
        }
    }

    Ok(())
}

fn collect_flag_entries<'a>(
    catalog: &'a CatalogDocument,
    imports: &'a BTreeMap<String, CatalogDocument>,
    environment: &str,
) -> Vec<FlagEntry<'a>> {
    let mut entries = Vec::new();

    for (name, flag) in &catalog.flags {
        let rules = catalog
            .environments
            .get(environment)
            .and_then(|env| env.rules.get(name))
            .map_or(&[] as &[CatalogRule], |r| r.as_slice());
        entries.push(FlagEntry {
            name: name.clone(),
            default: flag.default,
            rules,
        });
    }

    for (import_namespace, imported) in imports {
        if let Some(env) = imported.environments.get(environment) {
            for (flag_key, flag) in &imported.flags {
                let qualified = format!("{import_namespace}.{flag_key}");
                let rules = env
                    .rules
                    .get(flag_key)
                    .map_or(&[] as &[CatalogRule], |r| r.as_slice());
                entries.push(FlagEntry {
                    name: qualified,
                    default: flag.default,
                    rules,
                });
            }
        } else {
            for (flag_key, flag) in &imported.flags {
                let qualified = format!("{import_namespace}.{flag_key}");
                entries.push(FlagEntry {
                    name: qualified,
                    default: flag.default,
                    rules: &[],
                });
            }
        }
    }

    entries
}

fn merge_segments(
    catalog: &CatalogDocument,
    imports: &BTreeMap<String, CatalogDocument>,
) -> Result<BTreeMap<String, Segment>, CompilerError> {
    let mut segments: BTreeMap<String, Segment> = catalog.segments.clone();
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
            segments.insert(name.clone(), segment.clone());
        }
    }

    Ok(segments)
}

fn compile_segments(
    segments: &BTreeMap<String, Segment>,
    string_table: &mut StringTable,
) -> Result<Vec<(u16, Expression)>, CompilerError> {
    let mut compiled = Vec::new();
    for (name, segment) in segments {
        let segment_expr = parse_expression(&segment.when)?;
        let processed_expr = string_table.process_expression(&segment_expr)?;
        let name_index = string_table.add(name)?;
        compiled.push((name_index, processed_expr));
    }
    Ok(compiled)
}

fn compile_flag_rules(
    flag_name: &str,
    rules: &[CatalogRule],
    string_table: &mut StringTable,
) -> Result<Vec<Rule>, CompilerError> {
    if rules.is_empty() {
        return Ok(Vec::new());
    }
    let mut compiled = Vec::with_capacity(rules.len());
    for (index, rule) in rules.iter().enumerate() {
        compiled.push(compile_catalog_rule(rule, flag_name, index, string_table)?);
    }
    Ok(compiled)
}

fn compile_catalog_rule(
    rule: &CatalogRule,
    flag_name: &str,
    rule_index: usize,
    string_table: &mut StringTable,
) -> Result<Rule, CompilerError> {
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

    let when_expr = if let Some(when) = &rule.when {
        let parsed = parse_expression(when)?;
        Some(string_table.process_expression(&parsed)?)
    } else {
        None
    };

    if let Some(serve) = rule.serve {
        let value_index = string_table.add(bool_to_serve_value(serve))?;
        let payload = ServePayload::Number(value_index);
        return Ok(match when_expr {
            Some(w) => Rule::ServeWithWhen(w, payload),
            None => Rule::ServeWithoutWhen(payload),
        });
    }

    if let Some(rollout) = &rule.rollout {
        validate_rollout_percentage(rollout.percentage, flag_name, rule_index)?;
        let value_index = string_table.add(bool_to_serve_value(rollout.serve))?;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let percentage = rollout.percentage.round() as u8;
        let payload = RolloutPayload {
            value_index: RolloutValue::Number(value_index),
            percentage,
        };
        return Ok(match when_expr {
            Some(w) => Rule::RolloutWithWhen(w, payload),
            None => Rule::RolloutWithoutWhen(payload),
        });
    }

    unreachable!("serve or rollout presence checked above")
}

fn append_default_serve_rule(
    default: bool,
    rules: &mut Vec<Rule>,
    string_table: &mut StringTable,
) -> Result<(), CompilerError> {
    let default_index = string_table.add(bool_to_serve_value(default))?;
    rules.push(Rule::ServeWithoutWhen(ServePayload::Number(default_index)));
    Ok(())
}

fn bool_to_serve_value(serve: bool) -> &'static str {
    if serve {
        "ON"
    } else {
        "OFF"
    }
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

#[cfg(test)]
#[path = "compile_tests.rs"]
mod compile_tests;
