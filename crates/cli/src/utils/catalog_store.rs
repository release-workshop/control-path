//! Read/write seam for service `control-path.yaml` authoring.
//!
//! - [`CatalogStore::open`] requires schema-valid catalog content ([`ValidationMode::Authoring`]);
//!   unlike legacy `read_unified_config`, invalid files cannot be mutated until fixed manually.
//! - Top-level extension keys (e.g. `sdk`) are preserved across save; unknown **flag** fields are
//!   not round-tripped (see `.scratch/platform-spine/migration-02-catalog-document-store.md`).
//! - After [`CatalogStore::save`], call [`CatalogStore::validate_sdk_generate`] when the catalog
//!   has imports or you need the same post-edit check as `load_for_explain`.

use crate::error::{CliError, CliResult};
use crate::utils::atomic_write::atomic_write_string;
use crate::utils::catalog::{
    discover_workspace, load_for_explain_with_document, load_for_sdk_generate_with_document,
    CATALOG_FILE,
};
use controlpath_compiler::catalog::parse_catalog_value;
use controlpath_compiler::catalog::{Environment, FlagDefinition, Rule};
use controlpath_compiler::{
    load_and_validate_catalog, validate_catalog, CatalogDocument, CatalogValidationContext,
    FlagKind, FlagLifecycle, ValidationMode, WorkspaceDocument,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const CATALOG_TOP_LEVEL_KEYS: &[&str] = &[
    "catalog",
    "mode",
    "saas",
    "imports",
    "attributes",
    "flags",
    "environments",
    "segments",
    "kill_switches",
    "artifacts",
];

/// In-memory service catalog with validated load/save.
pub struct CatalogStore {
    path: PathBuf,
    document: CatalogDocument,
    workspace: Option<WorkspaceDocument>,
    /// Top-level YAML keys not modeled on [`CatalogDocument`] (e.g. `sdk`).
    preserved: BTreeMap<String, Value>,
}

impl CatalogStore {
    /// Open `control-path.yaml` in the current working directory.
    pub fn open_default() -> CliResult<Self> {
        Self::open(Path::new(CATALOG_FILE))
    }

    /// Open and validate a catalog file at `path`.
    pub fn open(path: &Path) -> CliResult<Self> {
        if !path.exists() {
            return Err(CliError::Message(format!(
                "{} not found at {}. Run 'controlpath setup' to create it.",
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(CATALOG_FILE),
                path.display()
            )));
        }
        let content = fs::read_to_string(path)
            .map_err(|e| CliError::Message(format!("Failed to read {}: {e}", path.display())))?;
        let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
        let workspace = discover_workspace(base_dir)?;
        let file_label = path.to_string_lossy();
        let raw = parse_catalog_value(&content, Some(file_label.as_ref()))
            .map_err(|e| CliError::Message(format!("Failed to parse {}: {e}", path.display())))?;
        let preserved = preserved_top_level_fields(&raw);
        let ctx = CatalogValidationContext {
            workspace: workspace.clone(),
            ..Default::default()
        };
        // Re-stringify stripped YAML so load uses the compiler validate→deserialize path.
        // A future in-place Value validator would avoid scalar normalization side effects.
        let stripped_yaml = serde_yaml::to_string(&strip_preserved_fields(&raw)).map_err(|e| {
            CliError::Message(format!("Failed to serialize catalog for validation: {e}"))
        })?;
        let (document, validation) = load_and_validate_catalog(
            &stripped_yaml,
            file_label.as_ref(),
            &ctx,
            ValidationMode::Authoring,
        )
        .map_err(|e| CliError::Message(format!("Failed to load {}: {e}", path.display())))?;

        if !validation.is_ok() {
            return Err(validation_errors(&validation));
        }

        Ok(Self {
            path: path.to_path_buf(),
            document,
            workspace,
            preserved,
        })
    }

    #[must_use]
    pub fn document(&self) -> &CatalogDocument {
        &self.document
    }

    #[cfg(test)]
    pub fn document_mut(&mut self) -> &mut CatalogDocument {
        &mut self.document
    }

    #[must_use]
    pub fn flag_exists(&self, name: &str) -> bool {
        self.document.flags.contains_key(name)
    }

    #[must_use]
    pub fn is_flag_deprecated(&self, name: &str) -> bool {
        self.document
            .flags
            .get(name)
            .is_some_and(|f| f.lifecycle == FlagLifecycle::Deprecated)
    }

    #[must_use]
    pub fn environment_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.document.environments.keys().cloned().collect();
        names.sort();
        names
    }

    #[must_use]
    pub fn sdk_output_path(&self) -> Option<String> {
        self.preserved
            .get("sdk")
            .and_then(|sdk| sdk.get("output"))
            .and_then(|output| output.as_str())
            .map(str::to_string)
    }

    /// Add a boolean flag and optionally seed environment rules (`--sync`).
    pub fn add_flag(
        &mut self,
        flag_name: &str,
        default: bool,
        kind: FlagKind,
        description: Option<&str>,
        sync_envs: &[String],
    ) -> CliResult<()> {
        if self.document.flags.contains_key(flag_name) {
            return Err(CliError::Message(format!(
                "Flag '{flag_name}' already exists"
            )));
        }

        self.document.flags.insert(
            flag_name.to_string(),
            FlagDefinition {
                default,
                kind,
                lifecycle: FlagLifecycle::Active,
                description: description.map(str::to_string),
                owner: None,
                ticket: None,
                expires: None,
                tags: None,
                metadata: None,
            },
        );

        if !sync_envs.is_empty() {
            for env in sync_envs {
                let entry = self
                    .document
                    .environments
                    .entry(env.clone())
                    .or_insert_with(|| Environment {
                        description: None,
                        rules: BTreeMap::new(),
                    });
                entry.rules.insert(
                    flag_name.to_string(),
                    vec![Rule {
                        when: None,
                        serve: Some(default),
                        rollout: None,
                        reason: None,
                    }],
                );
            }
        }

        Ok(())
    }

    /// Mark a flag as deprecated.
    pub fn deprecate_flag(&mut self, flag_name: &str) -> CliResult<()> {
        let flag = self
            .document
            .flags
            .get_mut(flag_name)
            .ok_or_else(|| CliError::Message(format!("Flag '{flag_name}' not found")))?;
        flag.lifecycle = FlagLifecycle::Deprecated;
        Ok(())
    }

    /// Enable a flag in an environment by appending a rule.
    pub fn enable_flag_in_environment(
        &mut self,
        flag_name: &str,
        environment: &str,
        rule_expr: Option<&str>,
        serve: bool,
        force_deprecated: bool,
    ) -> CliResult<()> {
        if !self.flag_exists(flag_name) {
            return Err(CliError::Message(format!("Flag '{flag_name}' not found")));
        }
        if self.is_flag_deprecated(flag_name) && !force_deprecated {
            return Err(CliError::Message(format!(
                "Flag '{flag_name}' is deprecated. Rule changes are blocked unless --force is set."
            )));
        }

        let env_entry = self
            .document
            .environments
            .entry(environment.to_string())
            .or_insert_with(|| Environment {
                description: None,
                rules: BTreeMap::new(),
            });
        let rules = env_entry.rules.entry(flag_name.to_string()).or_default();
        rules.push(Rule {
            when: rule_expr.map(str::to_string),
            serve: Some(serve),
            rollout: None,
            reason: None,
        });
        Ok(())
    }

    /// Remove a flag or environment rules for a flag.
    pub fn remove_flag(&mut self, flag_name: &str, env: Option<&str>) -> CliResult<()> {
        if let Some(target_env) = env {
            if let Some(env) = self.document.environments.get_mut(target_env) {
                env.rules.remove(flag_name);
            }
            return Ok(());
        }

        if self.document.flags.remove(flag_name).is_none() {
            return Err(CliError::Message(format!("Flag '{flag_name}' not found.")));
        }

        for env in self.document.environments.values_mut() {
            env.rules.remove(flag_name);
        }
        Ok(())
    }

    /// Add a top-level environment with an empty rules map.
    pub fn add_environment(&mut self, name: &str) -> CliResult<()> {
        if self.environment_names().iter().any(|e| e == name) {
            return Err(CliError::Message(format!(
                "Environment '{name}' already exists."
            )));
        }
        self.document.environments.insert(
            name.to_string(),
            Environment {
                description: None,
                rules: BTreeMap::new(),
            },
        );
        Ok(())
    }

    /// Remove a top-level environment block.
    pub fn remove_environment(&mut self, name: &str) -> CliResult<()> {
        if self.document.environments.remove(name).is_none() {
            return Err(CliError::Message(format!(
                "Environment '{name}' not found."
            )));
        }
        Ok(())
    }

    /// Validate and atomically write the catalog to disk.
    pub fn save(&self) -> CliResult<()> {
        self.validate_before_write()?;
        let yaml = catalog_document_to_yaml(&self.document, &self.preserved)?;
        atomic_write_string(&self.path, &yaml)
            .map_err(|e| CliError::Message(format!("Failed to write {}: {e}", self.path.display())))
    }

    /// Run SdkGenerate validation (including import resolution) on the in-memory document.
    ///
    /// Uses [`catalog::load_for_explain_with_document`]; the service catalog YAML on disk is not
    /// re-parsed. Call after [`Self::save`] when a command needs the same post-edit gate as
    /// [`catalog::load_for_explain`] (e.g. `flag add` without `--lang`). Other mutators
    /// (`flag remove`, `env add`, …) only run Authoring on save unless they call this or
    /// [`Self::validate_sdk_generate_if_imported`].
    pub fn validate_sdk_generate(&self) -> CliResult<()> {
        let base_dir = self.path.parent().unwrap_or_else(|| Path::new("."));
        load_for_explain_with_document(
            base_dir,
            &self.path,
            &self.document,
            self.workspace.clone(),
        )?;
        Ok(())
    }

    /// Build an SDK catalog from the in-memory document (SdkGenerate + SaaS CDN URLs when applicable).
    ///
    /// Prefer this over [`catalog::load_for_sdk_generate`] immediately after [`Self::save`] so regen
    /// uses the same document as [`Self::validate_sdk_generate`].
    pub fn sdk_for_generate(&self) -> CliResult<controlpath_compiler::SdkCatalog> {
        let base_dir = self.path.parent().unwrap_or_else(|| Path::new("."));
        load_for_sdk_generate_with_document(
            base_dir,
            &self.path,
            &self.document,
            self.workspace.clone(),
        )
    }

    /// SdkGenerate post-check only when `imports` is non-empty (e.g. workflow enable/new-flag).
    pub fn validate_sdk_generate_if_imported(&self) -> CliResult<()> {
        if self.document.imports.is_empty() {
            return Ok(());
        }
        self.validate_sdk_generate()
    }

    fn validate_before_write(&self) -> CliResult<()> {
        let validation = validate_catalog(
            self.path.to_string_lossy().as_ref(),
            &self.document,
            &CatalogValidationContext {
                workspace: self.workspace.clone(),
                ..Default::default()
            },
            ValidationMode::Authoring,
        );
        if !validation.is_ok() {
            return Err(validation_errors(&validation));
        }
        Ok(())
    }
}

fn strip_preserved_fields(raw: &Value) -> Value {
    let mut value = raw.clone();
    if let Some(obj) = value.as_object_mut() {
        obj.retain(|key, _| CATALOG_TOP_LEVEL_KEYS.contains(&key.as_str()));
    }
    value
}

fn preserved_top_level_fields(raw: &Value) -> BTreeMap<String, Value> {
    let Some(obj) = raw.as_object() else {
        return BTreeMap::new();
    };
    obj.iter()
        .filter(|(key, _)| !CATALOG_TOP_LEVEL_KEYS.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn catalog_document_to_yaml(
    document: &CatalogDocument,
    preserved: &BTreeMap<String, Value>,
) -> CliResult<String> {
    let mut value = serde_json::to_value(document)
        .map_err(|e| CliError::Message(format!("Failed to serialize catalog: {e}")))?;
    if let Some(obj) = value.as_object_mut() {
        for (key, extra) in preserved {
            obj.insert(key.clone(), extra.clone());
        }
    }
    serde_yaml::to_string(&value)
        .map_err(|e| CliError::Message(format!("Failed to serialize catalog: {e}")))
}

fn validation_errors(validation: &controlpath_compiler::CatalogValidationResult) -> CliError {
    let messages: Vec<String> = validation
        .errors
        .iter()
        .map(|e| e.message.clone())
        .collect();
    CliError::Message(format!("Config is invalid: {}", messages.join("; ")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::DirGuard;
    use serial_test::serial;
    use std::path::PathBuf;
    use tempfile::TempDir;

    const FIXTURE: &str = r"catalog:
  id: test-service
mode: local
flags:
  existing_flag:
    default: false
    kind: release
environments:
  production:
    rules:
      existing_flag:
        - serve: false
";

    fn write_import_fixture(temp_dir: &TempDir) {
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
    }

    fn write_saas_fixture(temp_dir: &TempDir) {
        let content = r"catalog:
  namespace: acme
  id: checkout-service
mode: saas
saas:
  project: acme/checkout
flags:
  feature_a:
    kind: release
    default: false
    owner: team-a
";
        fs::write(temp_dir.path().join(CATALOG_FILE), content).unwrap();
    }

    fn write_saas_ast_cache(temp_dir: &TempDir) {
        use controlpath_compiler::ast::Artifact;
        use controlpath_compiler::serialize;

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
    }

    #[test]
    #[serial]
    fn open_loads_valid_catalog_from_disk() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = DirGuard::new(temp_dir.path()).unwrap();
        fs::write(CATALOG_FILE, FIXTURE).unwrap();

        let store = CatalogStore::open_default().unwrap();
        assert!(store.flag_exists("existing_flag"));
        assert_eq!(store.environment_names(), vec!["production"]);
    }

    #[test]
    #[serial]
    fn save_round_trips_flag_addition() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = DirGuard::new(temp_dir.path()).unwrap();
        fs::write(CATALOG_FILE, FIXTURE).unwrap();

        let mut store = CatalogStore::open_default().unwrap();
        store
            .add_flag(
                "new_flag",
                false,
                FlagKind::Release,
                Some("A new flag"),
                &["production".to_string()],
            )
            .unwrap();
        store.save().unwrap();

        let reloaded = CatalogStore::open_default().unwrap();
        assert!(reloaded.flag_exists("new_flag"));
        let rules = reloaded
            .document()
            .environments
            .get("production")
            .and_then(|e| e.rules.get("new_flag"));
        assert!(rules.is_some_and(|r| !r.is_empty()));
    }

    #[test]
    #[serial]
    fn invalid_save_does_not_modify_disk() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = DirGuard::new(temp_dir.path()).unwrap();
        fs::write(CATALOG_FILE, FIXTURE).unwrap();
        let original = fs::read_to_string(CATALOG_FILE).unwrap();

        let mut store = CatalogStore::open_default().unwrap();
        store.document_mut().catalog.id.clear();
        let err = store.save().unwrap_err();
        assert!(err.to_string().contains("invalid"));

        assert_eq!(fs::read_to_string(CATALOG_FILE).unwrap(), original);
    }

    #[test]
    #[serial]
    fn save_preserves_unmodeled_top_level_keys() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = DirGuard::new(temp_dir.path()).unwrap();
        let yaml = format!("{FIXTURE}sdk:\n  output: ./custom-sdk\n");
        fs::write(CATALOG_FILE, yaml).unwrap();

        let mut store = CatalogStore::open_default().unwrap();
        store
            .add_flag("other_flag", true, FlagKind::Release, None, &[])
            .unwrap();
        store.save().unwrap();

        let content = fs::read_to_string(CATALOG_FILE).unwrap();
        assert!(content.contains("output: ./custom-sdk"));
    }

    #[test]
    #[serial]
    fn save_round_trips_empty_attribute_schema_opt_in() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = DirGuard::new(temp_dir.path()).unwrap();
        let yaml = format!("{FIXTURE}attributes: {{}}\n");
        fs::write(CATALOG_FILE, yaml).unwrap();

        let store = CatalogStore::open_default().unwrap();
        assert!(store.document().attribute_schema_opted_in());
        assert_eq!(
            store.document().attribute_schema_fields(),
            Some(&BTreeMap::new())
        );
        store.save().unwrap();

        let content = fs::read_to_string(CATALOG_FILE).unwrap();
        assert!(content.contains("attributes:"));
        let reloaded = CatalogStore::open_default().unwrap();
        assert!(reloaded.document().attribute_schema_opted_in());
        assert_eq!(
            reloaded.document().attribute_schema_fields(),
            Some(&BTreeMap::new())
        );
    }

    #[test]
    #[serial]
    fn add_and_remove_environment() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = DirGuard::new(temp_dir.path()).unwrap();
        fs::write(CATALOG_FILE, FIXTURE).unwrap();

        let mut store = CatalogStore::open_default().unwrap();
        store.add_environment("staging").unwrap();
        store.save().unwrap();

        let reloaded = CatalogStore::open_default().unwrap();
        assert!(reloaded
            .environment_names()
            .contains(&"staging".to_string()));

        let mut store = CatalogStore::open_default().unwrap();
        store.remove_environment("staging").unwrap();
        store.save().unwrap();
        assert!(!CatalogStore::open_default()
            .unwrap()
            .environment_names()
            .contains(&"staging".to_string()));
    }

    #[test]
    #[serial]
    fn deprecated_flag_blocks_enable_without_force() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = DirGuard::new(temp_dir.path()).unwrap();
        fs::write(CATALOG_FILE, FIXTURE).unwrap();

        let mut store = CatalogStore::open_default().unwrap();
        store.deprecate_flag("existing_flag").unwrap();
        let err = store
            .enable_flag_in_environment("existing_flag", "production", None, true, false)
            .unwrap_err();
        assert!(err.to_string().contains("deprecated"));
    }

    #[test]
    #[serial]
    fn remove_flag_env_only_leaves_definition() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = DirGuard::new(temp_dir.path()).unwrap();
        fs::write(CATALOG_FILE, FIXTURE).unwrap();

        let mut store = CatalogStore::open_default().unwrap();
        store
            .remove_flag("existing_flag", Some("production"))
            .unwrap();
        store.save().unwrap();

        let reloaded = CatalogStore::open_default().unwrap();
        assert!(reloaded.flag_exists("existing_flag"));
        assert!(!reloaded
            .document()
            .environments
            .get("production")
            .and_then(|e| e.rules.get("existing_flag"))
            .is_some_and(|r| !r.is_empty()));
    }

    #[test]
    #[serial]
    fn add_flag_sync_replaces_existing_env_rules() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = DirGuard::new(temp_dir.path()).unwrap();
        fs::write(CATALOG_FILE, FIXTURE).unwrap();

        let mut store = CatalogStore::open_default().unwrap();
        store
            .add_flag(
                "sync_flag",
                true,
                FlagKind::Release,
                None,
                &["production".to_string()],
            )
            .unwrap();
        let rules = store
            .document()
            .environments
            .get("production")
            .and_then(|e| e.rules.get("sync_flag"))
            .unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].serve, Some(true));
    }

    #[test]
    #[serial]
    fn validate_sdk_generate_uses_in_memory_document_after_save() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = DirGuard::new(temp_dir.path()).unwrap();
        write_import_fixture(&temp_dir);

        let store = CatalogStore::open_default().unwrap();
        store.save().unwrap();
        fs::write(temp_dir.path().join(CATALOG_FILE), "not: valid: yaml: [\n").unwrap();
        store.validate_sdk_generate().unwrap();
    }

    #[test]
    #[serial]
    fn validate_sdk_generate_after_save_with_imports() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = DirGuard::new(temp_dir.path()).unwrap();
        write_import_fixture(&temp_dir);

        let store = CatalogStore::open_default().unwrap();
        assert!(!store.document().imports.is_empty());
        store.save().unwrap();
        store.validate_sdk_generate().unwrap();
    }

    #[test]
    #[serial]
    fn sdk_for_generate_uses_in_memory_document_after_save() {
        use crate::utils::catalog::load_for_sdk_generate;

        let temp_dir = TempDir::new().unwrap();
        let _guard = DirGuard::new(temp_dir.path()).unwrap();
        write_import_fixture(&temp_dir);

        let store = CatalogStore::open_default().unwrap();
        store.save().unwrap();
        fs::write(temp_dir.path().join(CATALOG_FILE), "not: valid: yaml: [\n").unwrap();

        let sdk = store.sdk_for_generate().unwrap();
        assert!(sdk
            .flags
            .iter()
            .any(|f| f.qualified_name == "platform.emergency_kill_switch"));
        assert!(load_for_sdk_generate(temp_dir.path()).is_err());
    }

    #[test]
    #[serial]
    fn sdk_for_generate_embeds_saas_urls_from_memory_after_save() {
        use crate::utils::catalog::load_for_sdk_generate;
        use controlpath_compiler::build_saas_runtime_urls;

        let temp_dir = TempDir::new().unwrap();
        let _guard = DirGuard::new(temp_dir.path()).unwrap();
        write_saas_fixture(&temp_dir);
        write_saas_ast_cache(&temp_dir);

        let store = CatalogStore::open_default().unwrap();
        store.save().unwrap();
        fs::write(temp_dir.path().join(CATALOG_FILE), "not: valid: yaml: [\n").unwrap();

        let sdk = store.sdk_for_generate().unwrap();
        let catalog_id = controlpath_compiler::effective_catalog_id(
            &controlpath_compiler::CatalogIdentity {
                id: "checkout-service".to_string(),
                namespace: Some("acme".to_string()),
                scope: Default::default(),
            },
            None,
        );
        let expected = build_saas_runtime_urls(
            "https://cdn.controlpath.dev",
            "acme/checkout",
            &catalog_id,
            "production",
        );
        assert_eq!(
            sdk.artifact_urls.get("production"),
            Some(&expected.artifact_url)
        );
        assert_eq!(
            sdk.kill_switch_urls.get("production"),
            Some(&expected.kill_switch_url)
        );
        assert!(load_for_sdk_generate(temp_dir.path()).is_err());
    }
}
