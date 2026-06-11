/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::catalog::base_attributes;
use crate::catalog::model::{
    AttributeScalarType, CatalogDocument, CatalogIdentity, CatalogMode, FlagKind, WorkspaceDocument,
};
use crate::catalog::parse::{parse_catalog_value, parse_workspace_value};
use crate::compiler::expressions::expression_top_level_property_references;
use crate::error::{CompilationError, CompilerError};
use crate::schemas;
use crate::validator::common::validate_with_schema;
use crate::validator::error::{ValidationError, ValidationResult, ValidationWarning};

/// Outcome of catalog validation including non-fatal warnings.
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

impl CatalogValidationResult {
    #[must_use]
    pub fn valid() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.valid
    }
}

/// Context for semantic catalog validation.
///
/// Import resolution is not performed inside the compiler: callers (typically the CLI)
/// must walk `imports`, load referenced catalogs, and populate `imported_flag_keys` with
/// flag names that must not have environment rules in the consuming catalog. Prefer
/// [`CatalogValidationContext::with_imports`] or let [`super::validate_and_compile_catalog`]
/// derive keys from resolved imports when `imported_flag_keys` is empty.
#[derive(Debug, Clone, Default)]
pub struct CatalogValidationContext {
    /// Workspace discovered via walk-up (namespace fallback only).
    pub workspace: Option<WorkspaceDocument>,
    /// Local flag names re-exported from imported catalogs; environment rules for these keys are rejected.
    pub imported_flag_keys: BTreeSet<String>,
}

impl CatalogValidationContext {
    /// Build validation context from resolved import documents.
    #[must_use]
    pub fn with_imports(
        workspace: Option<WorkspaceDocument>,
        imports: &BTreeMap<String, CatalogDocument>,
    ) -> Self {
        Self {
            workspace,
            imported_flag_keys: imported_flag_keys_from_imports(imports),
        }
    }
}

/// Collect unqualified flag keys from resolved import documents.
#[must_use]
pub fn imported_flag_keys_from_imports(
    imports: &BTreeMap<String, CatalogDocument>,
) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for imported in imports.values() {
        for flag_key in imported.flags.keys() {
            keys.insert(flag_key.clone());
        }
    }
    keys
}

/// Which validation phases run for a catalog document.
///
/// Used by [`validate_catalog_value`] and [`load_and_validate_catalog`]:
///
/// - [`ValidationMode::Authoring`] — JSON Schema and semantic rules on the document alone.
///   Import cross-catalog rules (environment rules for imported flags) are skipped until
///   imports are resolved.
/// - [`ValidationMode::SdkGenerate`] — full validation for SDK generation, including import semantics
///   when [`CatalogValidationContext::imported_flag_keys`] is populated.
/// - [`ValidationMode::Compile`] — full validation for compilation. Today runs the same phases as
///   [`ValidationMode::SdkGenerate`]; reserved so issue 06+ can add compile-only checks without
///   changing SDK callers.
/// - [`ValidationMode::BootstrapSync`] — OSS-to-SaaS migration: like [`ValidationMode::SdkGenerate`]
///   but permits a transitional `environments` block in SaaS mode for one-time catalog sync only.
///   `validate` and steady-state CI continue to use [`ValidationMode::SdkGenerate`].
///
/// User-facing compile paths must pass [`ValidationMode::Compile`] on the post-import pass;
/// SDK paths use [`ValidationMode::SdkGenerate`].
///
/// [`ValidationMode::Authoring`] still runs import *namespace collision* checks using inline
/// `imports` keys (not resolved documents). Cross-catalog environment rules for imported flags
/// require `SdkGenerate` or `Compile`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ValidationMode {
    Authoring,
    #[default]
    SdkGenerate,
    Compile,
    BootstrapSync,
}

impl ValidationMode {
    fn runs_import_semantics(self) -> bool {
        match self {
            ValidationMode::Authoring => false,
            // Identical today; keep separate variants for future compile-only rules.
            ValidationMode::SdkGenerate
            | ValidationMode::Compile
            | ValidationMode::BootstrapSync => true,
        }
    }

    fn allows_saas_bootstrap_environments(self) -> bool {
        self == ValidationMode::BootstrapSync
    }
}

/// Validate a catalog `Value` (JSON Schema + semantic rules; import semantics depend on [`ValidationMode`]).
pub fn validate_catalog_value(
    file_path: &str,
    data: &Value,
    ctx: &CatalogValidationContext,
    mode: ValidationMode,
) -> CatalogValidationResult {
    let schema = schemas::load_catalog_schema();
    let base = validate_with_schema(&schema, file_path, data, |path, value| {
        semantic_errors(path, value, ctx, mode)
    });

    let warnings = semantic_warnings(file_path, data);

    CatalogValidationResult {
        valid: base.valid,
        errors: base.errors,
        warnings,
    }
}

/// Validate a typed catalog document (re-validates via JSON round-trip for schema checks).
///
/// **Unsafe on untrusted input** if the document was produced by [`super::parse_catalog`]
/// without an earlier [`validate_catalog_value`] on the raw parse: serde already removed v1 fields.
/// Prefer [`validate_catalog_value`] or [`load_and_validate_catalog`] for untrusted YAML.
pub fn validate_catalog(
    file_path: &str,
    doc: &CatalogDocument,
    ctx: &CatalogValidationContext,
    mode: ValidationMode,
) -> CatalogValidationResult {
    let value = serde_json::to_value(doc).expect("catalog document must serialize to JSON");
    validate_catalog_value(file_path, &value, ctx, mode)
}

/// Validate workspace YAML/JSON.
pub fn validate_workspace_value(file_path: &str, data: &Value) -> ValidationResult {
    let schema = schemas::load_workspace_schema();
    validate_with_schema(&schema, file_path, data, |_, _| Vec::new())
}

/// Parse and validate catalog content end-to-end (preferred entry point for untrusted catalogs).
///
/// Validates the parsed JSON `Value` before deserializing so v1 fields and schema violations
/// are not stripped by serde. Always check [`CatalogValidationResult::is_ok`]; on failure this
/// still returns `Ok((doc, validation))` with a best-effort [`CatalogDocument`] (full parse when
/// possible, otherwise a shell with catalog identity and empty flags).
pub fn load_and_validate_catalog(
    content: &str,
    file_path: &str,
    ctx: &CatalogValidationContext,
    mode: ValidationMode,
) -> Result<(CatalogDocument, CatalogValidationResult), crate::parser::error::ParseError> {
    let value = parse_catalog_value(content, Some(file_path))?;
    let validation = validate_catalog_value(file_path, &value, ctx, mode);
    let doc = deserialize_catalog_document(value, validation.valid)?;
    Ok((doc, validation))
}

/// Deserialize after validation. When validation failed, returns a minimal shell (catalog identity,
/// empty flags) if full typing fails (e.g. unknown v1 fields still present in the `Value`).
fn deserialize_catalog_document(
    value: Value,
    validation_valid: bool,
) -> Result<CatalogDocument, crate::parser::error::ParseError> {
    if validation_valid {
        return serde_json::from_value(value).map_err(|e| {
            crate::parser::error::ParseError::InvalidFieldType(format!(
                "Failed to deserialize catalog: {e}"
            ))
        });
    }

    match serde_json::from_value::<CatalogDocument>(value.clone()) {
        Ok(doc) => Ok(doc),
        Err(_) => catalog_document_shell(&value),
    }
}

fn catalog_document_shell(
    value: &Value,
) -> Result<CatalogDocument, crate::parser::error::ParseError> {
    let catalog_value = value.get("catalog").ok_or_else(|| {
        crate::parser::error::ParseError::MissingField(
            "Invalid catalog: missing required field \"catalog\"".to_string(),
        )
    })?;
    let catalog: CatalogIdentity = serde_json::from_value(catalog_value.clone()).map_err(|e| {
        crate::parser::error::ParseError::InvalidFieldType(format!(
            "Failed to deserialize catalog identity: {e}"
        ))
    })?;
    Ok(CatalogDocument {
        catalog,
        mode: value
            .get("mode")
            .and_then(|m| m.as_str())
            .and_then(|s| match s {
                "saas" => Some(CatalogMode::Saas),
                "local" => Some(CatalogMode::Local),
                _ => None,
            })
            .unwrap_or_default(),
        saas: value
            .get("saas")
            .and_then(|s| serde_json::from_value(s.clone()).ok()),
        imports: BTreeMap::new(),
        attributes: shell_attributes(value),
        flags: BTreeMap::new(),
        environments: BTreeMap::new(),
        segments: BTreeMap::new(),
        kill_switches: BTreeMap::new(),
        artifacts: BTreeMap::new(),
    })
}

/// Preserve attribute-schema opt-in when building a recovery shell from raw JSON.
///
/// Only object-shaped `attributes` values imply opt-in; `null`, arrays, and scalars do not.
fn shell_attributes(value: &Value) -> Option<BTreeMap<String, AttributeScalarType>> {
    let attrs = value.get("attributes")?;
    if !attrs.is_object() {
        return None;
    }
    Some(serde_json::from_value(attrs.clone()).unwrap_or_default())
}

/// Parse and validate workspace content.
///
/// Check [`ValidationResult::valid`] on the returned validation; parse still succeeds when
/// validation fails.
pub fn load_and_validate_workspace(
    content: &str,
    file_path: &str,
) -> Result<(WorkspaceDocument, ValidationResult), crate::parser::error::ParseError> {
    let value = parse_workspace_value(content, Some(file_path))?;
    let validation = validate_workspace_value(file_path, &value);
    let doc: WorkspaceDocument = serde_json::from_value(value).map_err(|e| {
        crate::parser::error::ParseError::InvalidFieldType(format!(
            "Failed to deserialize workspace: {e}"
        ))
    })?;
    Ok((doc, validation))
}

const V1_TOP_LEVEL_FIELDS: &[&str] = &["context", "defaultValue"];
const V1_FLAG_FIELDS: &[&str] = &["type", "defaultValue", "variations", "environments", "name"];
const TELEMETRY_METADATA_KEYS: &[&str] = &[
    "lastEvaluated",
    "last_evaluated",
    "evaluationCount",
    "evaluation_count",
    "evaluations",
    "unusedFlag",
    "unused_flag",
    "rotSuggestion",
    "rot_suggestion",
];

fn semantic_errors(
    file_path: &str,
    data: &Value,
    ctx: &CatalogValidationContext,
    mode: ValidationMode,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let Some(obj) = data.as_object() else {
        return errors;
    };

    for field in V1_TOP_LEVEL_FIELDS {
        if obj.contains_key(*field) {
            errors.push(validation_error(
                file_path,
                format!("Unsupported v1 field '{field}'"),
                Some(field.to_string()),
                Some("Remove legacy v1 fields; see control-path schema v2".to_string()),
            ));
        }
    }

    if let Some(flags) = obj.get("flags") {
        if flags.is_array() {
            errors.push(validation_error(
                file_path,
                "v1 array \"flags\" is not supported; use map-keyed flags".to_string(),
                Some("flags".to_string()),
                None,
            ));
        } else if let Some(flags_obj) = flags.as_object() {
            for (flag_key, flag_val) in flags_obj {
                errors.extend(v1_flag_field_errors(file_path, flag_key, flag_val));
            }
        }
    }

    let catalog_mode = obj.get("mode").and_then(|m| m.as_str()).unwrap_or("local");

    if catalog_mode == "saas" {
        for forbidden in ["segments", "kill_switches", "artifacts"] {
            if obj.contains_key(forbidden) {
                errors.push(validation_error(
                    file_path,
                    format!("'{forbidden}' is not allowed when mode is 'saas'"),
                    Some(forbidden.to_string()),
                    Some("Remove local-only blocks in SaaS mode".to_string()),
                ));
            }
        }

        if !mode.allows_saas_bootstrap_environments() && obj.contains_key("environments") {
            errors.push(validation_error(
                file_path,
                "'environments' is not allowed when mode is 'saas'".to_string(),
                Some("environments".to_string()),
                Some("Remove local-only blocks in SaaS mode".to_string()),
            ));
        }

        if let Some(saas) = obj.get("saas").and_then(|s| s.as_object()) {
            let require_signature = saas
                .get("require_ast_signature")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let has_public_key = saas
                .get("ast_public_key")
                .and_then(|v| v.as_str())
                .is_some_and(|key| !key.trim().is_empty());
            if require_signature && !has_public_key {
                errors.push(validation_error(
                    file_path,
                    "saas.ast_public_key is required when require_ast_signature is true"
                        .to_string(),
                    Some("saas.ast_public_key".to_string()),
                    Some("Provide a base64-encoded Ed25519 public key".to_string()),
                ));
            }
        }
    }

    // Note: duplicate keys under `imports` cannot be detected here — YAML/JSON parsers keep
    // the last value per key. Duplicate import namespaces require a custom loader if needed.

    // Local flag keys vs import namespace prefixes
    if let (Some(flags_obj), Some(imports_obj)) = (
        obj.get("flags").and_then(|f| f.as_object()),
        obj.get("imports").and_then(|i| i.as_object()),
    ) {
        for ns in imports_obj.keys() {
            for flag_key in flags_obj.keys() {
                if flag_key == ns || flag_key.starts_with(&format!("{ns}_")) {
                    errors.push(validation_error(
                        file_path,
                        format!("Local flag '{flag_key}' collides with import namespace '{ns}'"),
                        Some(format!("flags.{flag_key}")),
                        Some(format!(
                            "Rename the local flag or change the import namespace '{ns}'"
                        )),
                    ));
                }
            }
        }
    }

    // Environment rules for imported flags (requires resolved imports; SdkGenerate / Compile only)
    if mode.runs_import_semantics() && !ctx.imported_flag_keys.is_empty() {
        if let Some(envs) = obj.get("environments").and_then(|e| e.as_object()) {
            for (env_name, env_val) in envs {
                if let Some(rules) = env_val
                    .as_object()
                    .and_then(|e| e.get("rules"))
                    .and_then(|r| r.as_object())
                {
                    for flag_key in rules.keys() {
                        if ctx.imported_flag_keys.contains(flag_key) {
                            errors.push(validation_error(
                                file_path,
                                format!(
                                    "Environment '{env_name}' defines rules for imported flag '{flag_key}'"
                                ),
                                Some(format!("environments.{env_name}.rules.{flag_key}")),
                                Some(
                                    "Environment rules for imported flags belong in the source catalog only"
                                        .to_string(),
                                ),
                            ));
                        }
                    }
                }
            }
        }
    }

    // Kill switch rule constraints + flag kinds from flags map
    let flag_kinds: BTreeMap<String, FlagKind> = obj
        .get("flags")
        .and_then(|f| f.as_object())
        .map(|flags_obj| {
            flags_obj
                .iter()
                .filter_map(|(k, v)| {
                    v.get("kind")
                        .and_then(|knd| knd.as_str())
                        .and_then(flag_kind_from_str)
                        .map(|kind| (k.clone(), kind))
                })
                .collect()
        })
        .unwrap_or_default();

    if let Some(envs) = obj.get("environments").and_then(|e| e.as_object()) {
        for (env_name, env_val) in envs {
            if let Some(rules) = env_val
                .as_object()
                .and_then(|e| e.get("rules"))
                .and_then(|r| r.as_object())
            {
                for (flag_key, rule_list) in rules {
                    let kind = flag_kinds.get(flag_key);
                    if let Some(arr) = rule_list.as_array() {
                        for (idx, rule) in arr.iter().enumerate() {
                            if kind == Some(&FlagKind::KillSwitch) {
                                errors.extend(kill_switch_rule_errors(
                                    file_path, env_name, flag_key, idx, rule,
                                ));
                            }
                            if kind == Some(&FlagKind::Entitlement) {
                                errors.extend(entitlement_rule_errors(
                                    file_path, env_name, flag_key, idx, rule,
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    // Telemetry in flag metadata
    if let Some(flags_obj) = obj.get("flags").and_then(|f| f.as_object()) {
        for (flag_key, flag_val) in flags_obj {
            if let Some(meta) = flag_val.get("metadata").and_then(|m| m.as_object()) {
                for key in meta.keys() {
                    if is_telemetry_metadata_key(key) {
                        errors.push(validation_error(
                            file_path,
                            format!(
                                "Flag '{flag_key}' metadata must not contain SaaS telemetry field '{key}'"
                            ),
                            Some(format!("flags.{flag_key}.metadata.{key}")),
                            Some(
                                "Observed telemetry is read-only from SaaS and must not be committed to Git"
                                    .to_string(),
                            ),
                        ));
                    }
                }
            }
        }
    }

    errors.extend(catalog_scope_errors(file_path, data));
    errors.extend(attribute_schema_errors(file_path, data));
    errors.extend(attribute_schema_rule_property_errors(file_path, data));
    errors.extend(refresh_target_errors(
        file_path,
        "kill_switches",
        obj.get("kill_switches"),
    ));
    errors.extend(refresh_target_errors(
        file_path,
        "artifacts",
        obj.get("artifacts"),
    ));

    errors
}

fn catalog_scope_errors(file_path: &str, data: &Value) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let Some(obj) = data.as_object() else {
        return errors;
    };

    let scope = obj
        .get("catalog")
        .and_then(|c| c.as_object())
        .and_then(|c| c.get("scope"))
        .and_then(|s| s.as_str())
        .unwrap_or("service");

    if scope != "org" {
        return errors;
    }

    let Some(flags_obj) = obj.get("flags").and_then(|f| f.as_object()) else {
        return errors;
    };

    for (flag_key, flag_val) in flags_obj {
        let kind = flag_val
            .get("kind")
            .and_then(|k| k.as_str())
            .unwrap_or("release");
        if kind == "release" {
            errors.push(validation_error(
                file_path,
                format!(
                    "Flag '{flag_key}' has kind 'release' but catalog.scope is 'org'"
                ),
                Some(format!("flags.{flag_key}.kind")),
                Some(
                    "Org-scoped catalogs must not define kind: release flags; use kind: entitlement or kind: kill_switch"
                        .to_string(),
                ),
            ));
        }
    }

    errors
}

fn refresh_target_errors(
    file_path: &str,
    block: &str,
    targets: Option<&Value>,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let Some(targets_obj) = targets.and_then(|t| t.as_object()) else {
        return errors;
    };

    for (env, target) in targets_obj {
        let path_key = format!("{block}.{env}");
        let Some(target_obj) = target.as_object() else {
            continue;
        };

        let url = target_obj.get("url").and_then(|v| v.as_str());
        let path = target_obj.get("path").and_then(|v| v.as_str());
        let has_url = url.is_some_and(|u| !u.is_empty());
        let has_path = path.is_some_and(|p| !p.is_empty());

        if has_url && has_path {
            errors.push(validation_error(
                file_path,
                format!("'{block}.{env}' must specify either 'url' or 'path', not both"),
                Some(path_key.clone()),
                Some("Use a single refresh transport per environment target".to_string()),
            ));
            continue;
        }

        if !has_url && !has_path {
            errors.push(validation_error(
                file_path,
                format!("'{block}.{env}' must specify either 'url' or 'path'"),
                Some(path_key),
                None,
            ));
            continue;
        }

        if let Some(path_value) = path {
            if let Some(message) = posix_absolute_path_error(path_value) {
                errors.push(validation_error(
                    file_path,
                    message,
                    Some(format!("{block}.{env}.path")),
                    Some(
                        "Use a POSIX absolute path starting with '/' (e.g. /mnt/flags/kill.json)"
                            .to_string(),
                    ),
                ));
            }
        }
    }

    errors
}

/// Returns an error message when `path` is not a valid v1 POSIX absolute refresh path.
fn posix_absolute_path_error(path: &str) -> Option<String> {
    if path.is_empty() {
        return Some("path must not be empty".to_string());
    }
    if !path.starts_with('/') {
        return Some(format!(
            "path '{path}' must be a POSIX absolute path starting with '/'"
        ));
    }
    if path.contains('\\') {
        return Some(format!("path '{path}' must not contain backslashes"));
    }
    if path.as_bytes().get(1).is_some_and(|b| *b == b':')
        && path.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
    {
        return Some(format!("path '{path}' must not be a Windows drive path"));
    }
    None
}

fn semantic_warnings(file_path: &str, data: &Value) -> Vec<ValidationWarning> {
    let mut warnings = Vec::new();
    if let Some(flags_obj) = data.get("flags").and_then(|f| f.as_object()) {
        for (flag_key, flag_val) in flags_obj {
            warnings.extend(flag_metadata_warnings(file_path, flag_key, flag_val));
        }
    }
    warnings
}

fn flag_metadata_warnings(file_path: &str, flag_key: &str, flag: &Value) -> Vec<ValidationWarning> {
    let owner = flag.get("owner").and_then(|o| o.as_str());
    let expires = flag.get("expires").and_then(|e| e.as_str());
    let kind = flag.get("kind").and_then(|k| k.as_str());
    let default = flag.get("default").and_then(|d| d.as_bool());
    flag_metadata_warnings_inner(file_path, flag_key, owner, expires, kind, default)
}

fn flag_metadata_warnings_inner(
    file_path: &str,
    flag_key: &str,
    owner: Option<&str>,
    expires: Option<&str>,
    kind: Option<&str>,
    default: Option<bool>,
) -> Vec<ValidationWarning> {
    let mut warnings = Vec::new();
    if owner.map(str::is_empty).unwrap_or(true) {
        warnings.push(ValidationWarning {
            file: file_path.to_string(),
            message: format!("Flag '{flag_key}' is missing recommended field 'owner'"),
            path: Some(format!("flags.{flag_key}.owner")),
        });
    }
    if kind == Some("release") && expires.map(str::is_empty).unwrap_or(true) {
        warnings.push(ValidationWarning {
            file: file_path.to_string(),
            message: format!("Flag '{flag_key}' has kind 'release' but no 'expires' date"),
            path: Some(format!("flags.{flag_key}.expires")),
        });
    }
    if kind == Some("entitlement") && default == Some(true) {
        warnings.push(ValidationWarning {
            file: file_path.to_string(),
            message: format!(
                "Flag '{flag_key}' has kind 'entitlement' with default: true; use default: false for deny-by-default access"
            ),
            path: Some(format!("flags.{flag_key}.default")),
        });
    }
    warnings
}

fn v1_flag_field_errors(file_path: &str, flag_key: &str, flag: &Value) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let Some(obj) = flag.as_object() else {
        return errors;
    };
    for field in V1_FLAG_FIELDS {
        if obj.contains_key(*field) {
            errors.push(validation_error(
                file_path,
                format!("Unsupported v1 field '{field}' on flag '{flag_key}'"),
                Some(format!("flags.{flag_key}.{field}")),
                Some("Use v2 boolean flag shape (default, kind, ...)".to_string()),
            ));
        }
    }
    errors
}

fn kill_switch_rule_errors(
    file_path: &str,
    env_name: &str,
    flag_key: &str,
    rule_index: usize,
    rule: &Value,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let path = format!("environments.{env_name}.rules.{flag_key}[{rule_index}]");
    if rule.get("when").is_some() {
        errors.push(validation_error(
            file_path,
            format!("Kill switch flag '{flag_key}' cannot use 'when' in environment rules"),
            Some(format!("{path}.when")),
            Some("Use plain serve rules only for kind: kill_switch".to_string()),
        ));
    }
    if rule.get("rollout").is_some() {
        errors.push(validation_error(
            file_path,
            format!("Kill switch flag '{flag_key}' cannot use 'rollout' in environment rules"),
            Some(format!("{path}.rollout")),
            Some("Use plain serve rules only for kind: kill_switch".to_string()),
        ));
    }
    errors
}

fn entitlement_rule_errors(
    file_path: &str,
    env_name: &str,
    flag_key: &str,
    rule_index: usize,
    rule: &Value,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let path = format!("environments.{env_name}.rules.{flag_key}[{rule_index}]");
    if rule.get("rollout").is_some() {
        errors.push(validation_error(
            file_path,
            format!(
                "Entitlement flag '{flag_key}' cannot use 'rollout' in environment rules"
            ),
            Some(format!("{path}.rollout")),
            Some(
                "Use when and plain serve for kind: entitlement; use a separate kind: release flag for gradual rollout"
                    .to_string(),
            ),
        ));
    }
    errors
}

fn validation_error(
    file_path: &str,
    message: String,
    path: Option<String>,
    suggestion: Option<String>,
) -> ValidationError {
    ValidationError {
        file: file_path.to_string(),
        line: None,
        column: None,
        message,
        path,
        suggestion,
    }
}

fn flag_kind_from_str(s: &str) -> Option<FlagKind> {
    match s {
        "release" => Some(FlagKind::Release),
        "kill_switch" => Some(FlagKind::KillSwitch),
        "entitlement" => Some(FlagKind::Entitlement),
        _ => None,
    }
}

fn is_telemetry_metadata_key(key: &str) -> bool {
    TELEMETRY_METADATA_KEYS
        .iter()
        .any(|k| k.eq_ignore_ascii_case(key))
}

fn attribute_schema_errors(file_path: &str, data: &Value) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let Some(attrs_obj) = data.get("attributes").and_then(|a| a.as_object()) else {
        return errors;
    };
    for key in attrs_obj.keys() {
        if base_attributes::contains(key) {
            errors.push(validation_error(
                file_path,
                format!("Attribute schema key '{key}' collides with base attribute '{key}'"),
                Some(format!("attributes.{key}")),
                Some(
                    "Remove base attribute names from attribute schema; they are platform-owned"
                        .to_string(),
                ),
            ));
        }
    }
    if let Some(imports_obj) = data.get("imports").and_then(|i| i.as_object()) {
        for key in attrs_obj.keys() {
            if imports_obj.contains_key(key.as_str()) {
                errors.push(validation_error(
                    file_path,
                    format!("Attribute schema key '{key}' collides with import namespace '{key}'"),
                    Some(format!("attributes.{key}")),
                    Some(format!(
                        "Rename the service attribute or change the import namespace '{key}'"
                    )),
                ));
            }
        }
    }
    errors
}

fn attribute_schema_rule_property_errors(file_path: &str, data: &Value) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let Some(obj) = data.as_object() else {
        return errors;
    };

    let catalog_mode = obj.get("mode").and_then(|m| m.as_str()).unwrap_or("local");
    if catalog_mode != "local" {
        return errors;
    }

    let Some(attrs_obj) = obj.get("attributes").and_then(|a| a.as_object()) else {
        return errors;
    };

    let allowed = allowed_rule_property_names(attrs_obj);

    if let Some(segments) = obj.get("segments").and_then(|s| s.as_object()) {
        for (segment_name, segment) in segments {
            if let Some(when) = segment.get("when").and_then(|w| w.as_str()) {
                let path = format!("segments.{segment_name}.when");
                errors.extend(rule_when_property_errors(file_path, &path, when, &allowed));
            }
        }
    }

    if let Some(envs) = obj.get("environments").and_then(|e| e.as_object()) {
        for (env_name, env_val) in envs {
            let Some(rules) = env_val
                .as_object()
                .and_then(|e| e.get("rules"))
                .and_then(|r| r.as_object())
            else {
                continue;
            };
            for (flag_key, rule_list) in rules {
                let Some(arr) = rule_list.as_array() else {
                    continue;
                };
                for (idx, rule) in arr.iter().enumerate() {
                    if let Some(when) = rule.get("when").and_then(|w| w.as_str()) {
                        let path = format!("environments.{env_name}.rules.{flag_key}[{idx}].when");
                        errors.extend(rule_when_property_errors(file_path, &path, when, &allowed));
                    }
                }
            }
        }
    }

    errors
}

fn allowed_rule_property_names(attrs_obj: &serde_json::Map<String, Value>) -> BTreeSet<String> {
    let mut allowed: BTreeSet<String> = base_attributes::names().iter().cloned().collect();
    allowed.extend(attrs_obj.keys().cloned());
    allowed
}

fn rule_when_property_errors(
    file_path: &str,
    path: &str,
    when: &str,
    allowed: &BTreeSet<String>,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let referenced = match expression_top_level_property_references(when) {
        Ok(names) => names,
        Err(err) => {
            errors.push(validation_error(
                file_path,
                when_expression_parse_error_message(&err),
                Some(path.to_string()),
                Some("Fix the when expression syntax".to_string()),
            ));
            return errors;
        }
    };
    for name in referenced {
        if allowed.contains(&name) {
            continue;
        }
        errors.push(validation_error(
            file_path,
            format!(
                "Unknown evaluation attribute '{name}' in rule expression; declare it in attributes: or use a base attribute"
            ),
            Some(path.to_string()),
            Some(format!(
                "Add '{name}' to attributes: or fix the property name in the when expression"
            )),
        ));
    }
    errors
}

fn when_expression_parse_error_message(err: &CompilerError) -> String {
    match err {
        CompilerError::Compilation(CompilationError::ExpressionParsing(msg)) => {
            format!("Invalid when expression: {msg}")
        }
        other => format!("Invalid when expression: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saas_rejects_local_environments_block() {
        let data = serde_json::json!({
            "catalog": { "id": "svc" },
            "mode": "saas",
            "saas": { "project": "acme/svc" },
            "flags": { "f": { "default": false, "kind": "release" } },
            "environments": { "prod": { "rules": {} } }
        });
        let errors = semantic_errors(
            "test.yaml",
            &data,
            &CatalogValidationContext::default(),
            ValidationMode::Compile,
        );
        assert!(errors.iter().any(|e| e.message.contains("environments")));
    }

    #[test]
    fn saas_requires_ast_public_key_when_signature_required() {
        let data = serde_json::json!({
            "catalog": { "id": "svc" },
            "mode": "saas",
            "saas": {
                "project": "acme/svc",
                "require_ast_signature": true
            },
            "flags": { "f": { "default": false, "kind": "release" } }
        });
        let errors = semantic_errors(
            "test.yaml",
            &data,
            &CatalogValidationContext::default(),
            ValidationMode::Compile,
        );
        assert!(errors
            .iter()
            .any(|e| e.message.contains("saas.ast_public_key is required")));
    }
}
