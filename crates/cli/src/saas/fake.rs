//! In-memory SaaS client for tests and offline SaaS-mode workflows.
//!
//! Remote AST bytes in [`PersistedState::remote_asts`] are written to `.controlpath/<env>.ast`
//! on sync. `controlpath generate-sdk` discovers those environments via
//! [`crate::saas::ast_cache::FilesystemAstCache`] and embeds CDN poll URLs using
//! [`controlpath_compiler::build_saas_runtime_url_maps`]. Tests should assert embedded URLs
//! match that builder, not hard-code divergent paths.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use controlpath_compiler::catalog::FlagDefinition;
use controlpath_compiler::EffectiveCatalogId;
use serde::{Deserialize, Serialize};

use crate::error::{CliError, CliResult};
use crate::saas::client::{
    CatalogSyncPayload, CatalogSyncResult, DownloadCompiledAstsRequest, FetchFlagTelemetryRequest,
    FlagTelemetry, ListActiveFlagsRequest, RemoteAstArtifact, RetireFlagsRequest, SaasClient,
};
use crate::utils::atomic_write::atomic_write_string;

const STATE_FILE: &str = ".controlpath/saas-fake-state.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ProjectState {
    catalog_id: Option<String>,
    synced_flags: BTreeMap<String, FlagDefinition>,
    retired_flags: BTreeSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct StoredFlagTelemetry {
    last_evaluated: Option<String>,
    evaluation_count: u64,
    rot_suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PersistedState {
    projects: BTreeMap<String, ProjectState>,
    remote_asts: BTreeMap<String, Vec<u8>>,
    #[serde(default)]
    flag_telemetry: BTreeMap<String, BTreeMap<String, StoredFlagTelemetry>>,
}

#[derive(Debug, Clone)]
pub struct FakeSaasClient {
    base_dir: Option<PathBuf>,
    state: PersistedState,
}

impl FakeSaasClient {
    #[must_use]
    pub fn new() -> Self {
        Self {
            base_dir: None,
            state: PersistedState::default(),
        }
    }

    pub fn open(base_dir: &Path) -> CliResult<Self> {
        let path = base_dir.join(STATE_FILE);
        let state = if path.exists() {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| CliError::Message(format!("Failed to read SaaS fake state: {e}")))?;
            serde_json::from_str(&content)
                .map_err(|e| CliError::Message(format!("Failed to parse SaaS fake state: {e}")))?
        } else {
            PersistedState::default()
        };

        Ok(Self {
            base_dir: Some(base_dir.to_path_buf()),
            state,
        })
    }

    fn save(&self) -> CliResult<()> {
        let Some(base_dir) = &self.base_dir else {
            return Ok(());
        };

        let path = base_dir.join(STATE_FILE);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                CliError::Message(format!("Failed to create .controlpath directory: {e}"))
            })?;
        }

        let content = serde_json::to_string_pretty(&self.state)
            .map_err(|e| CliError::Message(format!("Failed to serialize SaaS fake state: {e}")))?;
        atomic_write_string(&path, &content)
            .map_err(|e| CliError::Message(format!("Failed to write SaaS fake state: {e}")))?;
        Ok(())
    }

    fn project_state(&self, project: &str) -> Option<&ProjectState> {
        self.state.projects.get(project)
    }

    fn project_state_mut(&mut self, project: &str) -> &mut ProjectState {
        self.state.projects.entry(project.to_string()).or_default()
    }

    fn ensure_project(&mut self, project: &str, catalog_id: &EffectiveCatalogId) -> CliResult<()> {
        let entry = self.project_state_mut(project);
        let catalog_id_str = catalog_id.as_str();
        if let Some(existing) = &entry.catalog_id {
            if existing != &catalog_id_str {
                return Err(CliError::Message(format!(
                    "SaaS project '{project}' is bound to catalog '{existing}', not '{catalog_id_str}'"
                )));
            }
        } else {
            entry.catalog_id = Some(catalog_id_str);
        }
        Ok(())
    }
}

impl Default for FakeSaasClient {
    fn default() -> Self {
        Self::new()
    }
}

impl SaasClient for FakeSaasClient {
    fn list_active_flags(&self, request: &ListActiveFlagsRequest) -> CliResult<Vec<String>> {
        Ok(self
            .project_state(&request.project)
            .map(|state| state.synced_flags.keys().cloned().collect())
            .unwrap_or_default())
    }

    fn sync_catalog(&mut self, payload: &CatalogSyncPayload) -> CliResult<CatalogSyncResult> {
        self.ensure_project(&payload.project, &payload.catalog_id)?;

        let mut upserted = Vec::new();
        let project_state = self.project_state_mut(&payload.project);
        for (key, flag) in &payload.flags {
            let changed = match project_state.synced_flags.get(key) {
                Some(existing) => existing != flag,
                None => true,
            };
            if changed {
                upserted.push(key.clone());
            }
            project_state.synced_flags.insert(key.clone(), flag.clone());
            project_state.retired_flags.remove(key);
        }

        self.save()?;

        Ok(CatalogSyncResult {
            upserted_flags: upserted,
        })
    }

    fn retire_flags(&mut self, request: &RetireFlagsRequest) -> CliResult<()> {
        self.ensure_project(&request.project, &request.catalog_id)?;

        let project_state = self.project_state_mut(&request.project);
        for key in &request.flag_keys {
            project_state.synced_flags.remove(key);
            project_state.retired_flags.insert(key.clone());
        }

        self.save()?;
        Ok(())
    }

    fn download_compiled_asts(
        &self,
        request: &DownloadCompiledAstsRequest,
    ) -> CliResult<Vec<RemoteAstArtifact>> {
        let _ = request;
        Ok(self
            .state
            .remote_asts
            .iter()
            .map(|(environment, bytes)| RemoteAstArtifact {
                environment: environment.clone(),
                bytes: bytes.clone(),
            })
            .collect())
    }

    fn fetch_flag_telemetry(
        &self,
        request: &FetchFlagTelemetryRequest,
    ) -> CliResult<Vec<FlagTelemetry>> {
        if let Some(state) = self.project_state(&request.project) {
            if let Some(bound) = &state.catalog_id {
                let requested = request.catalog_id.as_str();
                if bound.as_str() != requested {
                    return Err(CliError::Message(format!(
                        "Telemetry request catalog '{requested}' does not match SaaS project binding '{bound}'"
                    )));
                }
            }
        }

        let mut rows: Vec<FlagTelemetry> = self
            .state
            .flag_telemetry
            .get(&request.project)
            .map(|by_flag| {
                by_flag
                    .iter()
                    .map(|(key, tel)| FlagTelemetry {
                        flag_key: key.clone(),
                        last_evaluated: tel.last_evaluated.clone(),
                        evaluation_count: tel.evaluation_count,
                        rot_suggestion: tel.rot_suggestion.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        rows.sort_by(|a, b| a.flag_key.cmp(&b.flag_key));
        Ok(rows)
    }
}

#[cfg(test)]
impl FakeSaasClient {
    pub fn with_remote_ast(mut self, environment: impl Into<String>, bytes: Vec<u8>) -> Self {
        self.state.remote_asts.insert(environment.into(), bytes);
        self
    }

    #[must_use]
    pub fn is_retired(&self, project: &str, flag_key: &str) -> bool {
        self.project_state(project)
            .is_some_and(|state| state.retired_flags.contains(flag_key))
    }

    #[must_use]
    pub fn synced_flag(&self, project: &str, flag_key: &str) -> Option<&FlagDefinition> {
        self.project_state(project)
            .and_then(|state| state.synced_flags.get(flag_key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use controlpath_compiler::catalog::{CatalogIdentity, FlagDefinition, FlagKind};
    use controlpath_compiler::effective_catalog_id;
    use tempfile::TempDir;

    const PROJECT: &str = "acme/checkout";

    fn sample_payload(flags: &[(&str, bool)]) -> CatalogSyncPayload {
        let mut flag_map = std::collections::BTreeMap::new();
        for (key, default) in flags {
            flag_map.insert(
                (*key).to_string(),
                FlagDefinition {
                    default: *default,
                    kind: FlagKind::Release,
                    lifecycle: Default::default(),
                    owner: Some("team".to_string()),
                    ticket: None,
                    expires: None,
                    tags: None,
                    description: None,
                    metadata: None,
                },
            );
        }

        CatalogSyncPayload {
            catalog_id: effective_catalog_id(
                &CatalogIdentity {
                    id: "checkout".to_string(),
                    namespace: Some("acme".to_string()),
                },
                None,
            ),
            project: PROJECT.to_string(),
            flags: flag_map,
        }
    }

    #[test]
    fn sync_catalog_alone_does_not_retire_removed_flags() {
        let mut client = FakeSaasClient::new();
        let payload = sample_payload(&[("keep", true), ("remove", false)]);
        client.sync_catalog(&payload).unwrap();

        let reduced = sample_payload(&[("keep", true)]);
        client.sync_catalog(&reduced).unwrap();

        assert!(client.synced_flag(PROJECT, "remove").is_some());
        assert!(!client.is_retired(PROJECT, "remove"));
    }

    #[test]
    fn retire_flags_preserves_history_without_hard_delete() {
        let mut client = FakeSaasClient::new();
        client
            .sync_catalog(&sample_payload(&[("keep", true), ("remove", false)]))
            .unwrap();

        let catalog_id = effective_catalog_id(
            &CatalogIdentity {
                id: "checkout".to_string(),
                namespace: Some("acme".to_string()),
            },
            None,
        );
        client
            .retire_flags(&RetireFlagsRequest {
                catalog_id,
                project: PROJECT.to_string(),
                flag_keys: vec!["remove".to_string()],
            })
            .unwrap();

        assert!(client.synced_flag(PROJECT, "remove").is_none());
        assert!(client.is_retired(PROJECT, "remove"));
        assert!(client.synced_flag(PROJECT, "keep").is_some());
    }

    #[test]
    fn open_persists_state_across_clients() {
        let temp_dir = TempDir::new().unwrap();
        let payload = sample_payload(&[("keep", true), ("remove", false)]);

        {
            let mut client = FakeSaasClient::open(temp_dir.path()).unwrap();
            client.sync_catalog(&payload).unwrap();
        }

        let client = FakeSaasClient::open(temp_dir.path()).unwrap();
        assert!(client.synced_flag(PROJECT, "keep").is_some());
        assert!(client.synced_flag(PROJECT, "remove").is_some());
        assert!(temp_dir.path().join(STATE_FILE).exists());
    }

    #[test]
    fn open_rejects_mismatched_catalog_id_for_project() {
        let temp_dir = TempDir::new().unwrap();
        let mut client = FakeSaasClient::open(temp_dir.path()).unwrap();
        client
            .sync_catalog(&sample_payload(&[("keep", true)]))
            .unwrap();

        let other_catalog = effective_catalog_id(
            &CatalogIdentity {
                id: "other".to_string(),
                namespace: Some("acme".to_string()),
            },
            None,
        );
        let err = client
            .sync_catalog(&CatalogSyncPayload {
                catalog_id: other_catalog,
                project: PROJECT.to_string(),
                flags: sample_payload(&[("keep", true)]).flags,
            })
            .unwrap_err();

        assert!(err.to_string().contains("bound to catalog"));
    }
}
