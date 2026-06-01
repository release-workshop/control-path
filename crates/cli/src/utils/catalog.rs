//! Load and validate v2 catalogs for CLI operations.

use crate::error::{CliError, CliResult};
use crate::utils::atomic_write::{atomic_write, atomic_write_string};
use controlpath_compiler::{
    build_saas_runtime_urls, build_sdk_catalog, compile_catalog_with_imports, effective_catalog_id,
    load_and_validate_catalog, load_and_validate_workspace, parse_workspace, saas_cdn_base_url,
    serialize, validate_catalog, CatalogDocument, CatalogMode, CatalogValidationContext,
    SdkCatalog, ValidationMode, WorkspaceDocument,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) const CATALOG_FILE: &str = "control-path.yaml";
const WORKSPACE_FILE: &str = "control-path.workspace.yaml";

/// Validated service catalog, imports, and SDK projection (single read/validate path).
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

/// Load catalog, imports, and SDK projection with post-import validation for SDK generation.
pub fn load_catalog_bundle(base_dir: &Path) -> CliResult<CatalogBundle> {
    load_validated_catalog_bundle(base_dir, ValidationMode::SdkGenerate)
}

/// Same as [`load_catalog_bundle`] but post-import validation uses [`ValidationMode::Compile`].
pub fn load_catalog_bundle_for_compile(base_dir: &Path) -> CliResult<CatalogBundle> {
    load_validated_catalog_bundle(base_dir, ValidationMode::Compile)
}

/// Load the SDK catalog projection from `control-path.yaml` and resolved imports.
pub fn load_sdk_catalog(base_dir: &Path) -> CliResult<SdkCatalog> {
    Ok(load_validated_catalog_bundle(base_dir, ValidationMode::SdkGenerate)?.sdk)
}

/// Load SDK catalog and embed SaaS CDN runtime URLs (requires `.controlpath/*.ast` on disk).
pub fn load_sdk_catalog_for_generate(base_dir: &Path) -> CliResult<SdkCatalog> {
    let bundle = load_validated_catalog_bundle(base_dir, ValidationMode::SdkGenerate)?;
    let mut sdk = bundle.sdk;
    apply_saas_runtime_urls(
        base_dir,
        &bundle.catalog,
        bundle.workspace.as_ref(),
        &mut sdk,
    )?;
    Ok(sdk)
}

/// Load validated service catalog and resolved import documents.
#[allow(dead_code)]
pub fn load_catalog_documents(
    base_dir: &Path,
) -> CliResult<(CatalogDocument, BTreeMap<String, CatalogDocument>)> {
    let bundle = load_validated_catalog_bundle(base_dir, ValidationMode::SdkGenerate)?;
    Ok((bundle.catalog, bundle.imports))
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

/// Embed SaaS CDN artifact and kill switch URLs for every `.controlpath/<env>.ast` on disk.
///
/// Stale files are removed only when SaaS sync downloads ASTs (`write_remote_asts`);
/// manually added `*.ast` files are embedded until deleted or the next sync prunes them.
fn apply_saas_runtime_urls(
    base_dir: &Path,
    catalog: &CatalogDocument,
    workspace: Option<&WorkspaceDocument>,
    sdk: &mut SdkCatalog,
) -> CliResult<()> {
    if catalog.mode != CatalogMode::Saas {
        return Ok(());
    }

    let saas = catalog.saas.as_ref().ok_or_else(|| {
        CliError::Message("SaaS mode requires saas.project in control-path.yaml".to_string())
    })?;
    let cdn_base = saas_cdn_base_url(saas.cdn_url.as_deref());
    let catalog_id = effective_catalog_id(&catalog.catalog, workspace);

    let cache_dir = base_dir.join(".controlpath");
    if !cache_dir.is_dir() {
        return Err(no_saas_sync_cache_error());
    }

    let mut embedded = 0usize;
    for entry in fs::read_dir(&cache_dir)
        .map_err(|e| CliError::Message(format!("Failed to read {}: {e}", cache_dir.display())))?
    {
        let entry = entry.map_err(|e| {
            CliError::Message(format!("Failed to read {} entry: {e}", cache_dir.display()))
        })?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(environment) = file_name.strip_suffix(".ast") else {
            continue;
        };
        if environment.is_empty()
            || environment.contains('/')
            || environment.contains('\\')
            || environment.contains("..")
        {
            continue;
        }

        let urls = build_saas_runtime_urls(cdn_base, &saas.project, &catalog_id, environment);
        sdk.artifact_urls
            .insert(environment.to_string(), urls.artifact_url);
        sdk.kill_switch_urls
            .insert(environment.to_string(), urls.kill_switch_url);
        embedded += 1;
    }

    if embedded == 0 {
        return Err(no_saas_sync_cache_error());
    }

    Ok(())
}

fn no_saas_sync_cache_error() -> CliError {
    CliError::Message(
        "SaaS mode: no compiled artifacts in .controlpath/*.ast. \
         Run `controlpath ci` (or sync with the SaaS client) before `generate-sdk`. \
         Remove stray *.ast files you did not intend to embed (sync prunes only on download)."
            .to_string(),
    )
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
    let bundle = load_catalog_bundle_for_compile(base_dir)?;
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
    fn apply_saas_runtime_urls_embeds_cdn_urls_for_sync_cached_envs() {
        use controlpath_compiler::ast::Artifact;
        use controlpath_compiler::build_saas_runtime_urls;
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

        let sdk = load_sdk_catalog_for_generate(temp_dir.path()).unwrap();
        let catalog_id = controlpath_compiler::effective_catalog_id(
            &controlpath_compiler::CatalogIdentity {
                id: "checkout-service".to_string(),
                namespace: Some("acme".to_string()),
            },
            None,
        );
        let expected_production = build_saas_runtime_urls(
            "https://cdn.controlpath.dev",
            "acme/checkout",
            &catalog_id,
            "production",
        );
        assert_eq!(
            sdk.artifact_urls.get("production"),
            Some(&expected_production.artifact_url)
        );
        assert_eq!(
            sdk.kill_switch_urls.get("production"),
            Some(&expected_production.kill_switch_url)
        );
        assert!(sdk.artifact_urls.contains_key("staging"));
        assert!(sdk.kill_switch_urls.contains_key("staging"));
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
    fn load_sdk_catalog_from_local_only_fixture() {
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

        let sdk = load_sdk_catalog(temp_dir.path()).unwrap();
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
    fn load_sdk_catalog_resolves_imports() {
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

        let sdk = load_sdk_catalog(temp_dir.path()).unwrap();
        assert!(sdk
            .flags
            .iter()
            .any(|f| f.qualified_name == "platform.emergency_kill_switch"));
    }

    #[test]
    fn load_sdk_catalog_rejects_imported_flag_environment_rules() {
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

        let err = load_sdk_catalog(temp_dir.path()).unwrap_err();
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
