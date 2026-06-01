//! Load and validate v2 catalogs for CLI operations.
//!
//! Primary entry points:
//! - [`load_for_explain`] — explain/audit (`CatalogBundle`, SdkGenerate validation)
//! - [`load_for_sdk_generate`] — SDK generation (`SdkCatalog` + SaaS CDN URLs when applicable)
//! - [`compile_catalog_envs`] — local compile to `.controlpath/*.ast`
//!
//! Store helper (after authoring via [`crate::utils::catalog_store::CatalogStore`]):
//! - [`load_for_explain_with_document`] / [`load_for_sdk_generate_with_document`] — same as above
//!   but use an in-memory service catalog (imports still resolved from disk).
//!
//! SaaS CDN URLs: only [`load_for_sdk_generate`] and [`load_for_sdk_generate_with_document`]
//! (including [`crate::utils::catalog_store::CatalogStore::sdk_for_generate`]) embed
//! `artifact_urls` / `kill_switch_urls` via [`crate::saas::ast_cache::FilesystemAstCache`]
//! and [`controlpath_compiler::build_saas_runtime_url_maps`]. Watch intentionally skips SaaS
//! URL embedding ([`crate::commands::watch`]); validate and explain use the explain entry point.

use crate::error::{CliError, CliResult};
use crate::saas::FilesystemAstCache;
use crate::utils::atomic_write::{atomic_write, atomic_write_string};
use controlpath_compiler::{
    build_saas_runtime_url_maps, build_sdk_catalog, compile_catalog_with_imports,
    effective_catalog_id, load_and_validate_catalog, load_and_validate_workspace, parse_workspace,
    saas_cdn_base_url, serialize, validate_catalog, CatalogDocument, CatalogMode,
    CatalogValidationContext, SdkCatalog, ValidationMode, WorkspaceDocument,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) const CATALOG_FILE: &str = "control-path.yaml";
const WORKSPACE_FILE: &str = "control-path.workspace.yaml";

/// Validated service catalog, imports, and SDK projection (single read/validate path).
#[derive(Debug)]
pub struct CatalogBundle {
    pub catalog: CatalogDocument,
    pub imports: BTreeMap<String, CatalogDocument>,
    pub sdk: SdkCatalog,
    pub workspace: Option<WorkspaceDocument>,
}

fn load_validated_catalog_bundle(
    base_dir: &Path,
    import_validation_mode: ValidationMode,
) -> CliResult<CatalogBundle> {
    let catalog_path = base_dir.join(CATALOG_FILE);
    if !catalog_path.exists() {
        return Err(CliError::Message(format!(
            "{CATALOG_FILE} not found. Run 'controlpath setup' to create it."
        )));
    }

    let content = fs::read_to_string(&catalog_path).map_err(|e| {
        CliError::Message(format!("Failed to read {}: {e}", catalog_path.display()))
    })?;

    let workspace = discover_workspace(base_dir)?;
    let (catalog, initial_validation) = load_and_validate_catalog(
        &content,
        catalog_path.to_string_lossy().as_ref(),
        &CatalogValidationContext {
            workspace: workspace.clone(),
            ..Default::default()
        },
        ValidationMode::Authoring,
    )
    .map_err(|e| CliError::Message(format!("Failed to parse {}: {e}", catalog_path.display())))?;

    if !initial_validation.is_ok() {
        let messages: Vec<String> = initial_validation
            .errors
            .iter()
            .map(|e| e.message.clone())
            .collect();
        return Err(CliError::Message(format!(
            "Config is invalid: {}",
            messages.join("; ")
        )));
    }

    build_validated_bundle(
        base_dir,
        &catalog_path,
        catalog,
        workspace,
        import_validation_mode,
    )
}

fn build_validated_bundle(
    base_dir: &Path,
    catalog_path: &Path,
    catalog: CatalogDocument,
    workspace: Option<WorkspaceDocument>,
    import_validation_mode: ValidationMode,
) -> CliResult<CatalogBundle> {
    let imports = resolve_imports(base_dir, &catalog, workspace.as_ref())?;
    let validation = validate_catalog(
        catalog_path.to_string_lossy().as_ref(),
        &catalog,
        &CatalogValidationContext::with_imports(workspace.clone(), &imports),
        import_validation_mode,
    );

    if !validation.is_ok() {
        let messages: Vec<String> = validation
            .errors
            .iter()
            .map(|e| e.message.clone())
            .collect();
        return Err(CliError::Message(format!(
            "Config is invalid: {}",
            messages.join("; ")
        )));
    }

    let sdk = build_sdk_catalog(&catalog, &imports)
        .map_err(|e| CliError::Message(format!("Failed to build SDK catalog: {e}")))?;

    Ok(CatalogBundle {
        catalog,
        imports,
        sdk,
        workspace,
    })
}

/// Validated service catalog, imports, and SDK projection for explain/audit workflows.
///
/// Post-import validation uses [`ValidationMode::SdkGenerate`].
pub fn load_for_explain(base_dir: &Path) -> CliResult<CatalogBundle> {
    load_validated_catalog_bundle(base_dir, ValidationMode::SdkGenerate)
}

/// Like [`load_for_explain`] but uses an already-loaded service catalog (e.g. after
/// [`crate::utils::catalog_store::CatalogStore::save`]).
pub fn load_for_explain_with_document(
    base_dir: &Path,
    catalog_path: &Path,
    catalog: &CatalogDocument,
    workspace: Option<WorkspaceDocument>,
) -> CliResult<CatalogBundle> {
    build_validated_bundle(
        base_dir,
        catalog_path,
        catalog.clone(),
        workspace,
        ValidationMode::SdkGenerate,
    )
}

/// Validated flag catalog and imports for SDK generation, with SaaS CDN URLs when applicable.
pub fn load_for_sdk_generate(base_dir: &Path) -> CliResult<SdkCatalog> {
    let bundle = load_for_explain(base_dir)?;
    sdk_from_bundle(base_dir, bundle)
}

/// Like [`load_for_sdk_generate`] but uses an already-loaded service catalog.
pub fn load_for_sdk_generate_with_document(
    base_dir: &Path,
    catalog_path: &Path,
    catalog: &CatalogDocument,
    workspace: Option<WorkspaceDocument>,
) -> CliResult<SdkCatalog> {
    let bundle = load_for_explain_with_document(base_dir, catalog_path, catalog, workspace)?;
    sdk_from_bundle(base_dir, bundle)
}

fn sdk_from_bundle(base_dir: &Path, bundle: CatalogBundle) -> CliResult<SdkCatalog> {
    if bundle.catalog.mode != CatalogMode::Saas {
        return Ok(bundle.sdk);
    }

    let saas = bundle.catalog.saas.as_ref().ok_or_else(|| {
        CliError::Message("SaaS mode requires saas.project in control-path.yaml".to_string())
    })?;
    let environments = FilesystemAstCache::discover_environments(base_dir)?;
    let cdn_base = saas_cdn_base_url(saas.cdn_url.as_deref());
    let catalog_id = effective_catalog_id(&bundle.catalog.catalog, bundle.workspace.as_ref());
    let url_maps = build_saas_runtime_url_maps(cdn_base, &saas.project, &catalog_id, &environments);

    Ok(SdkCatalog {
        flags: bundle.sdk.flags,
        artifact_urls: url_maps.artifact_urls,
        kill_switch_urls: url_maps.kill_switch_urls,
    })
}

pub(crate) fn discover_workspace(base_dir: &Path) -> CliResult<Option<WorkspaceDocument>> {
    let mut current = base_dir.to_path_buf();
    loop {
        let workspace_path = current.join(WORKSPACE_FILE);
        if workspace_path.is_file() {
            let content = fs::read_to_string(&workspace_path).map_err(|e| {
                CliError::Message(format!("Failed to read {}: {e}", workspace_path.display()))
            })?;
            let (_, validation) =
                load_and_validate_workspace(&content, workspace_path.to_string_lossy().as_ref())
                    .map_err(|e| {
                        CliError::Message(format!(
                            "Failed to parse {}: {e}",
                            workspace_path.display()
                        ))
                    })?;
            if !validation.valid {
                let messages: Vec<String> = validation
                    .errors
                    .iter()
                    .map(|e| e.message.clone())
                    .collect();
                return Err(CliError::Message(format!(
                    "Workspace file is invalid: {}",
                    messages.join("; ")
                )));
            }
            return Ok(Some(
                parse_workspace(&content, Some(workspace_path.to_string_lossy().as_ref()))
                    .map_err(|e| CliError::Message(format!("Failed to parse workspace: {e}")))?,
            ));
        }

        if !current.pop() {
            return Ok(None);
        }
    }
}

fn resolve_imports(
    base_dir: &Path,
    catalog: &CatalogDocument,
    workspace: Option<&WorkspaceDocument>,
) -> CliResult<BTreeMap<String, CatalogDocument>> {
    let mut imports = BTreeMap::new();

    for (namespace, import_ref) in &catalog.imports {
        let import_path = resolve_import_path(base_dir, &import_ref.path)?;
        let content = fs::read_to_string(&import_path).map_err(|e| {
            CliError::Message(format!(
                "Failed to read import {} at {}: {e}",
                namespace,
                import_path.display()
            ))
        })?;

        let (imported, validation) = load_and_validate_catalog(
            &content,
            import_path.to_string_lossy().as_ref(),
            &CatalogValidationContext {
                workspace: workspace.cloned(),
                ..Default::default()
            },
            ValidationMode::Authoring,
        )
        .map_err(|e| {
            CliError::Message(format!(
                "Failed to parse import {} at {}: {e}",
                namespace,
                import_path.display()
            ))
        })?;

        if !validation.is_ok() {
            let messages: Vec<String> = validation
                .errors
                .iter()
                .map(|e| e.message.clone())
                .collect();
            return Err(CliError::Message(format!(
                "Import '{namespace}' is invalid: {}",
                messages.join("; ")
            )));
        }

        imports.insert(namespace.clone(), imported);
    }

    Ok(imports)
}

fn resolve_import_path(base_dir: &Path, import_path: &str) -> CliResult<PathBuf> {
    let path = PathBuf::from(import_path);
    let resolved = if path.is_absolute() {
        path
    } else {
        base_dir.join(path)
    };

    if !resolved.exists() {
        return Err(CliError::Message(format!(
            "Import path does not exist: {}",
            resolved.display()
        )));
    }

    Ok(resolved)
}

/// Compile AST artifacts for one or more environments from a v2 catalog.
pub fn compile_catalog_envs(base_dir: &Path, envs: Option<Vec<String>>) -> CliResult<Vec<String>> {
    let bundle = load_validated_catalog_bundle(base_dir, ValidationMode::Compile)?;
    let catalog = bundle.catalog;
    let imports = bundle.imports;

    let target_envs =
        envs.unwrap_or_else(|| catalog.environments.keys().cloned().collect::<Vec<_>>());

    if target_envs.is_empty() {
        return Err(CliError::Message(
            "No environments found in control-path.yaml.".to_string(),
        ));
    }

    fs::create_dir_all(base_dir.join(".controlpath"))
        .map_err(|e| CliError::Message(format!("Failed to create .controlpath directory: {e}")))?;

    let mut compiled = Vec::new();
    for env in target_envs {
        let artifact = compile_catalog_with_imports(&catalog, &imports, &env)
            .map_err(|e| CliError::Message(format!("Failed to compile {env}: {e}")))?;
        let ast_bytes = serialize(&artifact)
            .map_err(|e| CliError::Message(format!("Failed to serialize AST for {env}: {e}")))?;
        let output_path = base_dir.join(format!(".controlpath/{env}.ast"));
        atomic_write(&output_path, &ast_bytes)
            .map_err(|e| CliError::Message(format!("Failed to write AST for {env}: {e}")))?;
        write_kill_switch_artifact(base_dir, &env)?;
        compiled.push(env);
    }

    Ok(compiled)
}

/// Write deploy-time kill switch JSON for an environment.
pub fn write_kill_switch_artifact(base_dir: &Path, env: &str) -> CliResult<()> {
    let path = base_dir.join(format!(".controlpath/{env}.kill-switches.json"));
    let content = if path.exists() {
        fs::read_to_string(&path)
            .map_err(|e| CliError::Message(format!("Failed to read {}: {e}", path.display())))?
    } else {
        r#"{"version":"2.0","flags":{}}"#.to_string()
    };

    let mut value: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
        CliError::Message(format!("Invalid kill switch file {}: {e}", path.display()))
    })?;
    if value.get("version").is_none() {
        value["version"] = serde_json::json!("2.0");
    }
    if value.get("flags").is_none() {
        value["flags"] = serde_json::json!({});
    }

    let serialized = serde_json::to_string_pretty(&value)
        .map_err(|e| CliError::Message(format!("Failed to serialize kill switches: {e}")))?;
    atomic_write_string(&path, &format!("{serialized}\n"))
        .map_err(|e| CliError::Message(format!("Failed to write {}: {e}", path.display())))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    #[test]
    fn load_for_explain_local_catalog_without_imports() {
        let temp_dir = TempDir::new().unwrap();
        let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/examples");
        let catalog =
            fs::read_to_string(fixture_root.join("local-only.control-path.yaml")).unwrap();
        fs::write(temp_dir.path().join(CATALOG_FILE), catalog).unwrap();
        fs::write(
            temp_dir.path().join(WORKSPACE_FILE),
            include_str!("../../../../schemas/examples/control-path.workspace.yaml"),
        )
        .unwrap();

        let bundle = load_for_explain(temp_dir.path()).unwrap();
        assert_eq!(bundle.sdk.flags.len(), 2);
        assert!(bundle
            .sdk
            .flags
            .iter()
            .any(|f| f.qualified_name == "new_dashboard"));
    }

    struct LoadMatrixCase {
        name: &'static str,
        setup: fn(&Path),
        entry: fn(&Path) -> CliResult<()>,
        expect_ok: bool,
        error_contains: Option<&'static str>,
    }

    fn setup_local_with_imports(dir: &Path) {
        let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/examples");
        let platform_dir = dir.join("platform");
        fs::create_dir_all(&platform_dir).unwrap();
        fs::copy(
            fixture_root.join("shared-platform.control-path.yaml"),
            platform_dir.join(CATALOG_FILE),
        )
        .unwrap();
        let mut imported =
            fs::read_to_string(fixture_root.join("imported-global.control-path.yaml")).unwrap();
        imported = imported.replace(
            "path: ../../platform/control-path.yaml",
            "path: platform/control-path.yaml",
        );
        fs::write(dir.join(CATALOG_FILE), imported).unwrap();
    }

    fn setup_missing_import_path(dir: &Path) {
        let content = r"catalog:
  id: checkout-service
mode: local
imports:
  platform:
    path: platform/does-not-exist.control-path.yaml
flags:
  new_dashboard:
    kind: release
    default: false
    owner: team-web
environments:
  staging:
    rules:
      new_dashboard:
        - serve: true
";
        fs::write(dir.join(CATALOG_FILE), content).unwrap();
    }

    fn setup_saas_without_ast_cache(dir: &Path) {
        write_saas_catalog_fixture(
            dir,
            "  feature_a:\n    kind: release\n    default: false\n    owner: team-a\n",
        );
    }

    fn run_explain(dir: &Path) -> CliResult<()> {
        load_for_explain(dir).map(|_| ())
    }

    fn run_sdk_generate(dir: &Path) -> CliResult<()> {
        load_for_sdk_generate(dir).map(|_| ())
    }

    fn run_compile(dir: &Path) -> CliResult<()> {
        compile_catalog_envs(dir, Some(vec!["staging".to_string()])).map(|_| ())
    }

    #[test]
    fn catalog_load_entry_points_matrix() {
        let cases = [
            LoadMatrixCase {
                name: "local with imports via explain",
                setup: setup_local_with_imports,
                entry: run_explain,
                expect_ok: true,
                error_contains: None,
            },
            LoadMatrixCase {
                name: "local with imports via sdk generate",
                setup: setup_local_with_imports,
                entry: run_sdk_generate,
                expect_ok: true,
                error_contains: None,
            },
            LoadMatrixCase {
                name: "missing import path via explain",
                setup: setup_missing_import_path,
                entry: run_explain,
                expect_ok: false,
                error_contains: Some("Import path does not exist"),
            },
            LoadMatrixCase {
                name: "missing import path via sdk generate",
                setup: setup_missing_import_path,
                entry: run_sdk_generate,
                expect_ok: false,
                error_contains: Some("Import path does not exist"),
            },
            LoadMatrixCase {
                name: "missing import path via compile",
                setup: setup_missing_import_path,
                entry: run_compile,
                expect_ok: false,
                error_contains: Some("Import path does not exist"),
            },
            LoadMatrixCase {
                name: "saas without ast cache via explain",
                setup: setup_saas_without_ast_cache,
                entry: run_explain,
                expect_ok: true,
                error_contains: None,
            },
            LoadMatrixCase {
                name: "saas without ast cache via sdk generate",
                setup: setup_saas_without_ast_cache,
                entry: run_sdk_generate,
                expect_ok: false,
                error_contains: Some("no compiled artifacts"),
            },
        ];

        for case in cases {
            let temp_dir = TempDir::new().unwrap();
            (case.setup)(temp_dir.path());
            let result = (case.entry)(temp_dir.path());
            if case.expect_ok {
                assert!(result.is_ok(), "{}: expected Ok, got {result:?}", case.name);
            } else {
                let err = result.expect_err(&format!("{}: expected Err", case.name));
                if let Some(needle) = case.error_contains {
                    assert!(
                        err.to_string().contains(needle),
                        "{}: error {:?} should contain {:?}",
                        case.name,
                        err,
                        needle
                    );
                }
            }
        }
    }

    #[test]
    fn load_for_sdk_generate_embeds_saas_cdn_urls_for_sync_cached_envs() {
        use controlpath_compiler::ast::Artifact;
        use controlpath_compiler::build_saas_runtime_url_maps;
        use controlpath_compiler::serialize;

        let temp_dir = TempDir::new().unwrap();
        write_saas_catalog_fixture(
            temp_dir.path(),
            "  feature_a:\n    kind: release\n    default: false\n    owner: team-a\n",
        );

        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "production".to_string(),
            string_table: vec!["feature_a".to_string()],
            flags: vec![vec![]],
            flag_names: vec![0],
            segments: None,
            signature: None,
        };
        fs::create_dir_all(temp_dir.path().join(".controlpath")).unwrap();
        fs::write(
            temp_dir.path().join(".controlpath/production.ast"),
            serialize(&artifact).unwrap(),
        )
        .unwrap();
        fs::write(
            temp_dir.path().join(".controlpath/staging.ast"),
            serialize(&Artifact {
                environment: "staging".to_string(),
                ..artifact.clone()
            })
            .unwrap(),
        )
        .unwrap();

        let sdk = load_for_sdk_generate(temp_dir.path()).unwrap();
        let catalog_id = controlpath_compiler::effective_catalog_id(
            &controlpath_compiler::CatalogIdentity {
                id: "checkout-service".to_string(),
                namespace: Some("acme".to_string()),
            },
            None,
        );
        let expected = build_saas_runtime_url_maps(
            "https://cdn.controlpath.dev",
            "acme/checkout",
            &catalog_id,
            &["production", "staging"],
        );
        assert_eq!(
            sdk.artifact_urls.get("production"),
            expected.artifact_urls.get("production")
        );
        assert_eq!(
            sdk.kill_switch_urls.get("production"),
            expected.kill_switch_urls.get("production")
        );
        assert_eq!(
            sdk.artifact_urls.get("staging"),
            expected.artifact_urls.get("staging")
        );
        assert_eq!(
            sdk.kill_switch_urls.get("staging"),
            expected.kill_switch_urls.get("staging")
        );
        assert!(!sdk.artifact_urls.contains_key("saas-fake-state"));
    }

    fn write_saas_catalog_fixture(dir: &Path, flags_yaml: &str) {
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

    #[test]
    fn load_for_explain_from_local_only_fixture() {
        let temp_dir = TempDir::new().unwrap();
        let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/examples");
        let catalog =
            fs::read_to_string(fixture_root.join("local-only.control-path.yaml")).unwrap();
        fs::write(temp_dir.path().join(CATALOG_FILE), catalog).unwrap();
        fs::write(
            temp_dir.path().join(WORKSPACE_FILE),
            include_str!("../../../../schemas/examples/control-path.workspace.yaml"),
        )
        .unwrap();

        let sdk = load_for_explain(temp_dir.path()).unwrap().sdk;
        assert_eq!(sdk.flags.len(), 2);
        assert!(sdk
            .flags
            .iter()
            .any(|f| f.qualified_name == "new_dashboard"));
    }

    #[test]
    fn compile_catalog_envs_includes_imported_flags() {
        let temp_dir = TempDir::new().unwrap();
        let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/examples");

        let platform_dir = temp_dir.path().join("platform");
        fs::create_dir_all(&platform_dir).unwrap();
        fs::copy(
            fixture_root.join("shared-platform.control-path.yaml"),
            platform_dir.join(CATALOG_FILE),
        )
        .unwrap();

        let mut imported =
            fs::read_to_string(fixture_root.join("imported-global.control-path.yaml")).unwrap();
        imported = imported.replace(
            "path: ../../platform/control-path.yaml",
            "path: platform/control-path.yaml",
        );
        fs::write(temp_dir.path().join(CATALOG_FILE), imported).unwrap();

        compile_catalog_envs(temp_dir.path(), Some(vec!["production".to_string()])).unwrap();

        let ast_bytes = fs::read(temp_dir.path().join(".controlpath/production.ast")).unwrap();
        let ast_text = String::from_utf8_lossy(&ast_bytes);
        assert!(ast_text.contains("platform.emergency_kill_switch"));
        assert!(ast_text.contains("new_dashboard"));
    }

    #[test]
    fn load_for_explain_resolves_imports() {
        let temp_dir = TempDir::new().unwrap();
        let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/examples");

        let platform_dir = temp_dir.path().join("platform");
        fs::create_dir_all(&platform_dir).unwrap();
        fs::copy(
            fixture_root.join("shared-platform.control-path.yaml"),
            platform_dir.join(CATALOG_FILE),
        )
        .unwrap();

        let mut imported =
            fs::read_to_string(fixture_root.join("imported-global.control-path.yaml")).unwrap();
        imported = imported.replace(
            "path: ../../platform/control-path.yaml",
            "path: platform/control-path.yaml",
        );
        fs::write(temp_dir.path().join(CATALOG_FILE), imported).unwrap();

        let sdk = load_for_explain(temp_dir.path()).unwrap().sdk;
        assert!(sdk
            .flags
            .iter()
            .any(|f| f.qualified_name == "platform.emergency_kill_switch"));
    }

    #[test]
    fn load_for_explain_rejects_imported_flag_environment_rules() {
        let temp_dir = TempDir::new().unwrap();
        let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/examples");

        let platform_dir = temp_dir.path().join("platform");
        fs::create_dir_all(&platform_dir).unwrap();
        fs::copy(
            fixture_root.join("shared-platform.control-path.yaml"),
            platform_dir.join(CATALOG_FILE),
        )
        .unwrap();

        let mut imported =
            fs::read_to_string(fixture_root.join("imported-global.control-path.yaml")).unwrap();
        imported = imported.replace(
            "path: ../../platform/control-path.yaml",
            "path: platform/control-path.yaml",
        );
        imported = imported.replace(
            "      new_dashboard:\n        - serve: true",
            "      new_dashboard:\n        - serve: true\n      emergency_kill_switch:\n        - serve: true",
        );
        fs::write(temp_dir.path().join(CATALOG_FILE), imported).unwrap();

        let err = load_for_explain(temp_dir.path())
            .expect_err("expected imported flag environment rule rejection");
        assert!(err.to_string().contains("imported flag"));
    }

    #[test]
    fn compile_catalog_envs_rejects_imported_flag_environment_rules() {
        let temp_dir = TempDir::new().unwrap();
        let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/examples");

        let platform_dir = temp_dir.path().join("platform");
        fs::create_dir_all(&platform_dir).unwrap();
        fs::copy(
            fixture_root.join("shared-platform.control-path.yaml"),
            platform_dir.join(CATALOG_FILE),
        )
        .unwrap();

        let mut imported =
            fs::read_to_string(fixture_root.join("imported-global.control-path.yaml")).unwrap();
        imported = imported.replace(
            "path: ../../platform/control-path.yaml",
            "path: platform/control-path.yaml",
        );
        imported = imported.replace(
            "      new_dashboard:\n        - serve: true",
            "      new_dashboard:\n        - serve: true\n      emergency_kill_switch:\n        - serve: true",
        );
        fs::write(temp_dir.path().join(CATALOG_FILE), imported).unwrap();

        let err =
            compile_catalog_envs(temp_dir.path(), Some(vec!["staging".to_string()])).unwrap_err();
        assert!(err.to_string().contains("imported flag"));
    }
}
