//! Initialize monorepo workspace or service catalog files

use crate::error::{CliError, CliResult};
use crate::utils::atomic_write::atomic_write_string;
use crate::utils::runtime;
use dialoguer::Confirm;
use std::fs;
use std::path::{Path, PathBuf};

const WORKSPACE_FILE: &str = "control-path.workspace.yaml";
const CATALOG_FILE: &str = "control-path.yaml";

pub struct Options {
    pub monorepo: Option<bool>,
    pub namespace: Option<String>,
    pub service_id: Option<String>,
}

pub fn run(options: &Options) -> i32 {
    match run_inner(options) {
        Ok(msg) => {
            println!("✓ {msg}");
            0
        }
        Err(e) => {
            eprintln!("✗ Error: {e}");
            1
        }
    }
}

fn run_inner(options: &Options) -> CliResult<String> {
    let cwd = std::env::current_dir()
        .map_err(|e| CliError::Message(format!("Failed to resolve working directory: {e}")))?;

    if let Some(workspace) = find_workspace_file(&cwd) {
        return scaffold_service_catalog(&cwd, &workspace, options);
    }

    let monorepo = match options.monorepo {
        Some(value) => value,
        None => {
            runtime::require_interactive("choose monorepo or multi-repo setup")?;
            Confirm::new()
                .with_prompt("Set up a monorepo workspace at this directory?")
                .default(false)
                .interact()
                .map_err(|e| CliError::Message(format!("Failed to read input: {e}")))?
        }
    };

    if monorepo {
        create_workspace_file(&cwd, options)?;
        Ok(format!("Created {WORKSPACE_FILE} at {}", cwd.display()))
    } else {
        create_standalone_catalog(&cwd, options)?;
        Ok(format!("Created {CATALOG_FILE} at {}", cwd.display()))
    }
}

fn find_workspace_file(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let candidate = current.join(WORKSPACE_FILE);
        if candidate.is_file() {
            return Some(candidate);
        }
        if !current.pop() {
            return None;
        }
    }
}

fn create_workspace_file(dir: &Path, options: &Options) -> CliResult<()> {
    let path = dir.join(WORKSPACE_FILE);
    if path.exists() {
        return Err(CliError::Message(format!(
            "{WORKSPACE_FILE} already exists at {}",
            path.display()
        )));
    }

    let namespace = options
        .namespace
        .clone()
        .unwrap_or_else(|| "acme".to_string());
    let content = format!(
        r"namespace: {namespace}

scaffold:
  defaults:
    owner: platform-team
  mode: local
"
    );
    atomic_write_string(&path, &content)?;
    Ok(())
}

fn create_standalone_catalog(dir: &Path, options: &Options) -> CliResult<()> {
    let path = dir.join(CATALOG_FILE);
    if path.exists() {
        return Err(CliError::Message(format!(
            "{CATALOG_FILE} already exists at {}",
            path.display()
        )));
    }

    let namespace = match &options.namespace {
        Some(ns) => ns.clone(),
        None => {
            runtime::require_interactive("prompt for catalog namespace")?;
            dialoguer::Input::new()
                .with_prompt("Catalog namespace")
                .default("acme".into())
                .interact_text()
                .map_err(|e| CliError::Message(format!("Failed to read input: {e}")))?
        }
    };

    let service_id = options
        .service_id
        .clone()
        .unwrap_or_else(|| "example-service".to_string());

    let content = format!(
        r"catalog:
  id: {service_id}
  namespace: {namespace}
mode: local
flags: {{}}
"
    );
    atomic_write_string(&path, &content)?;
    Ok(())
}

fn scaffold_service_catalog(
    cwd: &Path,
    workspace_path: &Path,
    options: &Options,
) -> CliResult<String> {
    let catalog_path = cwd.join(CATALOG_FILE);
    if catalog_path.exists() {
        return Err(CliError::Message(format!(
            "{CATALOG_FILE} already exists at {}",
            catalog_path.display()
        )));
    }

    let workspace_content = fs::read_to_string(workspace_path).map_err(|e| {
        CliError::Message(format!("Failed to read {}: {e}", workspace_path.display()))
    })?;
    let workspace: serde_json::Value = serde_yaml::from_str(&workspace_content)
        .map_err(|e| CliError::Message(format!("Failed to parse workspace file: {e}")))?;

    let namespace = workspace
        .get("namespace")
        .and_then(|n| n.as_str())
        .unwrap_or("acme");
    let service_id = options
        .service_id
        .clone()
        .unwrap_or_else(|| directory_name(cwd));

    let mut catalog: serde_json::Value = serde_yaml::from_str(
        r"catalog:
  id: placeholder
mode: local
flags: {}
",
    )
    .map_err(|e| CliError::Message(format!("Failed to build catalog scaffold: {e}")))?;

    if let Some(catalog_obj) = catalog.get_mut("catalog").and_then(|c| c.as_object_mut()) {
        catalog_obj.insert("id".to_string(), serde_json::json!(service_id));
    }

    if let Some(scaffold) = workspace.get("scaffold") {
        merge_scaffold_into_catalog(&mut catalog, scaffold);
    } else {
        let _ = namespace;
    }

    let yaml = serde_yaml::to_string(&catalog)
        .map_err(|e| CliError::Message(format!("Failed to serialize catalog: {e}")))?;
    atomic_write_string(&catalog_path, &yaml)?;

    Ok(format!(
        "Scaffolded {CATALOG_FILE} from {}",
        workspace_path.display()
    ))
}

fn merge_scaffold_into_catalog(catalog: &mut serde_json::Value, scaffold: &serde_json::Value) {
    let Some(scaffold_obj) = scaffold.as_object() else {
        return;
    };
    let Some(catalog_obj) = catalog.as_object_mut() else {
        return;
    };

    for (key, value) in scaffold_obj {
        if key == "defaults" {
            continue;
        }
        catalog_obj.insert(key.clone(), value.clone());
    }

    if let Some(defaults) = scaffold.get("defaults").and_then(|d| d.as_object()) {
        if let Some(flags) = catalog_obj.get_mut("flags").and_then(|f| f.as_object_mut()) {
            for (flag_name, flag_defaults) in defaults {
                if !flags.contains_key(flag_name) {
                    flags.insert(flag_name.clone(), flag_defaults.clone());
                }
            }
        }
    }
}

fn directory_name(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("service")
        .to_string()
}
