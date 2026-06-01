//! SaaS catalog sync orchestration.

use std::collections::BTreeSet;
use std::path::Path;

use controlpath_compiler::catalog::{CatalogDocument, CatalogMode};
use controlpath_compiler::{effective_catalog_id, WorkspaceDocument};

use crate::error::{CliError, CliResult};
use crate::saas::ast::{write_remote_asts, RemoteAstOptions};
use crate::saas::client::{
    CatalogSyncOutcome, CatalogSyncPayload, DownloadCompiledAstsRequest, ListActiveFlagsRequest,
    RetireFlagsRequest, SaasClient,
};
use crate::utils::catalog::{discover_workspace, load_for_explain, CatalogBundle, CATALOG_FILE};

/// Parse a SaaS-mode catalog document without schema/semantic validation.
///
/// Used by rot/telemetry paths that only need catalog identity and flag keys. Prefer
/// [`load_saas_catalog_for_ci`] or [`load_for_explain`] for CI and sync.
pub fn parse_saas_catalog_document(
    base_dir: &Path,
) -> CliResult<(CatalogDocument, Option<WorkspaceDocument>)> {
    let catalog_path = base_dir.join(CATALOG_FILE);
    let content = std::fs::read_to_string(&catalog_path).map_err(|e| {
        CliError::Message(format!("Failed to read {}: {e}", catalog_path.display()))
    })?;

    let catalog = controlpath_compiler::parse_catalog(
        &content,
        Some(catalog_path.to_string_lossy().as_ref()),
    )
    .map_err(|e| CliError::Message(format!("Failed to parse catalog: {e}")))?;

    if catalog.mode != CatalogMode::Saas {
        return Err(CliError::Message("Expected SaaS mode catalog".to_string()));
    }

    let workspace = discover_workspace(base_dir)?;
    Ok((catalog, workspace))
}

/// Load a SaaS catalog for CI after full schema, semantic, and import validation.
pub fn load_saas_catalog_for_ci(base_dir: &Path) -> CliResult<CatalogBundle> {
    let bundle = load_for_explain(base_dir)?;
    if bundle.catalog.mode != CatalogMode::Saas {
        return Err(CliError::Message("Expected SaaS mode catalog".to_string()));
    }
    Ok(bundle)
}

/// Sync the Git catalog to SaaS and optionally download remote AST artifacts.
#[allow(dead_code)] // Public entry point for callers that do not pre-load the catalog.
pub fn sync_saas_catalog(
    base_dir: &Path,
    client: &mut dyn SaasClient,
    ast_options: &RemoteAstOptions,
) -> CliResult<SaasSyncOutcome> {
    let bundle = load_for_explain(base_dir)?;
    if bundle.catalog.mode != CatalogMode::Saas {
        return Err(CliError::Message("Expected SaaS mode catalog".to_string()));
    }
    sync_saas_catalog_with_catalog(
        base_dir,
        &bundle.catalog,
        bundle.workspace.as_ref(),
        client,
        ast_options,
    )
}

/// Sync a pre-validated SaaS catalog to SaaS and optionally download remote AST artifacts.
pub fn sync_saas_catalog_with_catalog(
    base_dir: &Path,
    catalog: &CatalogDocument,
    workspace: Option<&WorkspaceDocument>,
    client: &mut dyn SaasClient,
    ast_options: &RemoteAstOptions,
) -> CliResult<SaasSyncOutcome> {
    if catalog.mode != CatalogMode::Saas {
        return Err(CliError::Message("Expected SaaS mode catalog".to_string()));
    }

    let catalog_id = effective_catalog_id(&catalog.catalog, workspace);
    let payload = CatalogSyncPayload::from_catalog(catalog, catalog_id.clone())?;
    let catalog_sync = sync_catalog_to_saas(client, &payload, &catalog_id)?;

    let download_request = DownloadCompiledAstsRequest {
        catalog_id: catalog_id.clone(),
        project: payload.project.clone(),
    };
    let remote_asts = client.download_compiled_asts(&download_request)?;
    let downloaded_envs = write_remote_asts(base_dir, &remote_asts, ast_options)?;

    Ok(SaasSyncOutcome {
        catalog_id,
        catalog_sync,
        downloaded_envs,
    })
}

/// Diff Git against SaaS, retire removed flags, then upsert the current catalog.
fn sync_catalog_to_saas(
    client: &mut dyn SaasClient,
    payload: &CatalogSyncPayload,
    catalog_id: &controlpath_compiler::EffectiveCatalogId,
) -> CliResult<CatalogSyncOutcome> {
    let list_request = ListActiveFlagsRequest {
        catalog_id: catalog_id.clone(),
        project: payload.project.clone(),
    };
    let remote_active = client.list_active_flags(&list_request)?;

    let git_keys: BTreeSet<&String> = payload.flags.keys().collect();
    let mut retired_flags: Vec<String> = remote_active
        .into_iter()
        .filter(|key| !git_keys.contains(key))
        .collect();
    retired_flags.sort();

    if !retired_flags.is_empty() {
        client.retire_flags(&RetireFlagsRequest {
            catalog_id: catalog_id.clone(),
            project: payload.project.clone(),
            flag_keys: retired_flags.clone(),
        })?;
    }

    let upsert_result = client.sync_catalog(payload)?;

    Ok(CatalogSyncOutcome {
        retired_flags,
        upserted_flags: upsert_result.upserted_flags,
    })
}

/// Outcome of a SaaS sync workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaasSyncOutcome {
    pub catalog_id: controlpath_compiler::EffectiveCatalogId,
    pub catalog_sync: CatalogSyncOutcome,
    pub downloaded_envs: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::saas::fake::FakeSaasClient;
    use controlpath_compiler::catalog::FlagKind;
    use controlpath_compiler::serialize;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    fn write_saas_catalog(dir: &Path, flags_yaml: &str) {
        let content = format!(
            r"catalog:
  namespace: acme
  id: checkout-service
mode: saas
saas:
  project: acme/checkout
flags:
{flags_yaml}"
        );
        fs::write(dir.join(CATALOG_FILE), content).unwrap();
    }

    fn write_saas_catalog_with_missing_import(dir: &Path) {
        let content = r"catalog:
  id: checkout-service
mode: saas
saas:
  project: acme/checkout
imports:
  platform:
    path: platform/does-not-exist.control-path.yaml
flags:
  feature_a:
    kind: release
    default: false
    owner: team-a
";
        fs::write(dir.join(CATALOG_FILE), content).unwrap();
    }

    #[test]
    fn sync_rejects_missing_import_path_via_shared_loader() {
        let temp_dir = TempDir::new().unwrap();
        write_saas_catalog_with_missing_import(temp_dir.path());

        let mut client = FakeSaasClient::new();
        let err = sync_saas_catalog(temp_dir.path(), &mut client, &RemoteAstOptions::default())
            .unwrap_err();
        assert!(err.to_string().contains("Import path does not exist"));
    }

    #[test]
    fn sync_resolves_imports_before_push() {
        let temp_dir = TempDir::new().unwrap();
        let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/examples");

        let platform_dir = temp_dir.path().join("platform");
        fs::create_dir_all(&platform_dir).unwrap();
        fs::copy(
            fixture_root.join("shared-platform.control-path.yaml"),
            platform_dir.join(CATALOG_FILE),
        )
        .unwrap();

        let content = r"catalog:
  id: checkout-service
mode: saas
saas:
  project: acme/checkout
imports:
  platform:
    path: platform/control-path.yaml
flags:
  new_dashboard:
    kind: release
    default: false
    owner: team-web
";
        fs::write(temp_dir.path().join(CATALOG_FILE), content).unwrap();

        let mut client = FakeSaasClient::new();
        sync_saas_catalog(temp_dir.path(), &mut client, &RemoteAstOptions::default()).unwrap();

        assert!(client
            .synced_flag("acme/checkout", "new_dashboard")
            .is_some());
    }

    #[test]
    fn sync_pushes_flag_catalog_without_telemetry() {
        let temp_dir = TempDir::new().unwrap();
        write_saas_catalog(
            temp_dir.path(),
            "  feature_a:\n    kind: release\n    default: false\n    owner: team-a\n",
        );

        let mut client = FakeSaasClient::new();
        let outcome =
            sync_saas_catalog(temp_dir.path(), &mut client, &RemoteAstOptions::default()).unwrap();

        assert_eq!(outcome.catalog_sync.upserted_flags, vec!["feature_a"]);
        assert!(client.synced_flag("acme/checkout", "feature_a").is_some());
        assert_eq!(
            client
                .synced_flag("acme/checkout", "feature_a")
                .unwrap()
                .kind,
            FlagKind::Release
        );
    }

    #[test]
    fn removing_flag_from_git_retires_it_in_saas() {
        let temp_dir = TempDir::new().unwrap();
        write_saas_catalog(
            temp_dir.path(),
            "  keep_me:\n    kind: release\n    default: true\n    owner: team-a\n  remove_me:\n    kind: release\n    default: false\n    owner: team-a\n",
        );

        let mut client = FakeSaasClient::new();
        sync_saas_catalog(temp_dir.path(), &mut client, &RemoteAstOptions::default()).unwrap();

        write_saas_catalog(
            temp_dir.path(),
            "  keep_me:\n    kind: release\n    default: true\n    owner: team-a\n",
        );

        let outcome =
            sync_saas_catalog(temp_dir.path(), &mut client, &RemoteAstOptions::default()).unwrap();

        assert_eq!(outcome.catalog_sync.retired_flags, vec!["remove_me"]);
        assert!(client.is_retired("acme/checkout", "remove_me"));
        assert!(client.synced_flag("acme/checkout", "remove_me").is_none());
        assert!(client.synced_flag("acme/checkout", "keep_me").is_some());
    }

    #[test]
    fn downloads_remote_asts_to_controlpath() {
        let temp_dir = TempDir::new().unwrap();
        write_saas_catalog(
            temp_dir.path(),
            "  feature_a:\n    kind: release\n    default: false\n    owner: team-a\n",
        );

        let artifact = controlpath_compiler::ast::Artifact {
            version: "1.0".to_string(),
            environment: "production".to_string(),
            string_table: vec!["feature_a".to_string()],
            flags: vec![vec![]],
            flag_names: vec![0],
            segments: None,
            signature: None,
        };
        let bytes = serialize(&artifact).unwrap();
        let mut client = FakeSaasClient::new().with_remote_ast("production", bytes);

        let outcome =
            sync_saas_catalog(temp_dir.path(), &mut client, &RemoteAstOptions::default()).unwrap();

        assert_eq!(outcome.downloaded_envs, vec!["production"]);
        assert!(temp_dir.path().join(".controlpath/production.ast").exists());
    }
}
