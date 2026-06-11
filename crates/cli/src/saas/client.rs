//! SaaS API boundary types and client trait.

use controlpath_compiler::catalog::{CatalogDocument, Environment, FlagDefinition};
use controlpath_compiler::EffectiveCatalogId;
use std::collections::BTreeMap;

use crate::error::CliResult;

/// Flag catalog snapshot sent to SaaS (declared metadata only — no telemetry).
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogSyncPayload {
    pub catalog_id: EffectiveCatalogId,
    pub project: String,
    pub flags: BTreeMap<String, FlagDefinition>,
    /// Transitional Git environment rules for OSS-to-SaaS bootstrap sync only.
    pub environments: BTreeMap<String, Environment>,
}

impl CatalogSyncPayload {
    pub fn from_catalog(
        catalog: &CatalogDocument,
        catalog_id: EffectiveCatalogId,
    ) -> CliResult<Self> {
        let project = catalog
            .saas
            .as_ref()
            .map(|s| s.project.clone())
            .ok_or_else(|| {
                crate::error::CliError::Message(
                    "SaaS mode requires saas.project in control-path.yaml".to_string(),
                )
            })?;

        Ok(Self {
            catalog_id,
            project,
            flags: catalog.flags.clone(),
            environments: catalog.environments.clone(),
        })
    }
}

/// Request to list flag keys currently active in SaaS for a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListActiveFlagsRequest {
    pub catalog_id: EffectiveCatalogId,
    pub project: String,
}

/// Request to retire flags removed from Git (history preserved remotely).
///
/// Called explicitly by the CLI when an engineer removes flags from
/// `control-path.yaml` — not by SaaS dashboard users.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetireFlagsRequest {
    pub catalog_id: EffectiveCatalogId,
    pub project: String,
    pub flag_keys: Vec<String>,
}

/// Remote compiled AST bytes for one environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteAstArtifact {
    pub environment: String,
    pub bytes: Vec<u8>,
}

/// Request to download SaaS-compiled AST artifacts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadCompiledAstsRequest {
    pub catalog_id: EffectiveCatalogId,
    pub project: String,
}

/// Result of upserting the Git flag catalog to SaaS.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CatalogSyncResult {
    pub upserted_flags: Vec<String>,
}

/// Outcome of a full Git-to-SaaS sync (retirements + upserts).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CatalogSyncOutcome {
    pub retired_flags: Vec<String>,
    pub upserted_flags: Vec<String>,
}

/// Observed runtime signals for one flag (SaaS only — never written to Git).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FlagTelemetry {
    pub flag_key: String,
    pub last_evaluated: Option<String>,
    pub evaluation_count: u64,
    pub rot_suggestion: Option<String>,
}

/// Request to fetch observed telemetry for flags in a SaaS project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchFlagTelemetryRequest {
    pub catalog_id: EffectiveCatalogId,
    pub project: String,
}

/// Boundary for SaaS catalog sync and remote AST download.
pub trait SaasClient {
    /// List flag keys currently active in SaaS (excludes retired flags).
    fn list_active_flags(&self, request: &ListActiveFlagsRequest) -> CliResult<Vec<String>>;

    /// Upsert declared flag metadata from Git. Does not retire removed flags.
    fn sync_catalog(&mut self, payload: &CatalogSyncPayload) -> CliResult<CatalogSyncResult>;

    /// Retire flags removed from Git by a CLI engineer (history preserved remotely).
    fn retire_flags(&mut self, request: &RetireFlagsRequest) -> CliResult<()>;

    fn download_compiled_asts(
        &self,
        request: &DownloadCompiledAstsRequest,
    ) -> CliResult<Vec<RemoteAstArtifact>>;

    /// Fetch read-only observed telemetry (evaluation counts, rot suggestions).
    fn fetch_flag_telemetry(
        &self,
        request: &FetchFlagTelemetryRequest,
    ) -> CliResult<Vec<FlagTelemetry>>;
}
