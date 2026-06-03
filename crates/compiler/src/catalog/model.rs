/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Resolved catalog identity after namespace resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveCatalogId {
    pub namespace: Option<String>,
    pub id: String,
}

impl EffectiveCatalogId {
    /// Dotted effective id (`namespace.id`) or bare `id` when no namespace resolves.
    #[must_use]
    pub fn as_str(&self) -> String {
        match &self.namespace {
            Some(ns) => format!("{ns}.{}", self.id),
            None => self.id.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogIdentity {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CatalogMode {
    #[default]
    Local,
    Saas,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaasConfig {
    pub project: String,
    /// Optional Control Path API base URL for self-hosted deployments (catalog sync, not runtime poll).
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "url")]
    pub api_url: Option<String>,
    /// Optional CDN origin for runtime kill switch and compiled artifact polls (`generate-sdk` embedding).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cdn_url: Option<String>,
    /// Base64-encoded Ed25519 public key for verifying downloaded AST artifacts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ast_public_key: Option<String>,
    /// When true, downloaded AST artifacts must carry a valid signature.
    #[serde(default)]
    pub require_ast_signature: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogImport {
    pub path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlagKind {
    Release,
    KillSwitch,
    Entitlement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FlagLifecycle {
    #[default]
    Active,
    Deprecated,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlagDefinition {
    pub default: bool,
    pub kind: FlagKind,
    #[serde(default)]
    pub lifecycle: FlagLifecycle,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ticket: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Segment {
    pub when: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rollout {
    pub percentage: f64,
    pub serve: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serve: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rollout: Option<Rollout>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Environment {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub rules: BTreeMap<String, Vec<Rule>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KillSwitchTarget {
    pub url: String,
}

/// Per-environment compiled artifact URL (`artifacts.<env>.url` in local mode).
pub type ArtifactTarget = KillSwitchTarget;

/// Scalar type for catalog `attributes:` entries (v1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttributeScalarType {
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
}

/// Typed v2 `control-path.yaml` catalog document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatalogDocument {
    pub catalog: CatalogIdentity,
    #[serde(default)]
    pub mode: CatalogMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saas: Option<SaasConfig>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub imports: BTreeMap<String, CatalogImport>,
    /// Present when the catalog opts in to attribute schema (`attributes:` key in YAML).
    /// `None` = omitted (legacy); `Some({})` = opted in with no service fields yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attributes: Option<BTreeMap<String, AttributeScalarType>>,
    pub flags: BTreeMap<String, FlagDefinition>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub environments: BTreeMap<String, Environment>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub segments: BTreeMap<String, Segment>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub kill_switches: BTreeMap<String, KillSwitchTarget>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub artifacts: BTreeMap<String, ArtifactTarget>,
}

impl CatalogDocument {
    /// Whether the catalog opted in to attribute schema (`attributes:` key present, even if `{}`).
    #[must_use]
    pub fn attribute_schema_opted_in(&self) -> bool {
        self.attributes.is_some()
    }

    /// Declared attribute schema fields when opted in.
    #[must_use]
    pub fn attribute_schema_fields(&self) -> Option<&BTreeMap<String, AttributeScalarType>> {
        self.attributes.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceScaffold {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub imports: BTreeMap<String, CatalogImport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defaults: Option<BTreeMap<String, serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<CatalogMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saas: Option<SaasConfig>,
}

/// Typed `control-path.workspace.yaml` document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceDocument {
    pub namespace: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scaffold: Option<WorkspaceScaffold>,
}
