//! Kill switch management command (v2 boolean catalog).

use crate::error::{CliError, CliResult};
use crate::utils::kill_switch;
use crate::utils::unified_config;
use std::path::{Path, PathBuf};

pub struct Options {
    pub subcommand: OverrideSubcommand,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // CLI flags retained for backward-compatible argument parsing
pub enum OverrideSubcommand {
    Set {
        flag: String,
        value: String,
        reason: Option<String>,
        operator: Option<String>,
        file: Option<PathBuf>,
        definitions: Option<PathBuf>,
        env: Option<String>,
    },
    Clear {
        flag: String,
        file: Option<PathBuf>,
        env: Option<String>,
    },
    List {
        file: Option<PathBuf>,
        env: Option<String>,
    },
    History {
        flag: Option<String>,
        file: Option<PathBuf>,
        env: Option<String>,
    },
}

pub fn run(options: &Options) -> i32 {
    match run_inner(options) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("✗ Error: {e}");
            1
        }
    }
}

fn warn_ignored_file_flag(file: Option<&Path>, env: Option<&str>) -> CliResult<()> {
    let Some(file) = file else {
        return Ok(());
    };
    let env_name = kill_switch::resolve_kill_switch_env(env)?;
    let canonical = kill_switch::kill_switch_path(&env_name);
    if file != canonical.as_path() {
        eprintln!(
            "⚠ Note: --file {} is ignored; kill switches are stored in {}",
            file.display(),
            canonical.display()
        );
    }
    Ok(())
}

fn run_inner(options: &Options) -> CliResult<()> {
    if !unified_config::unified_config_exists() {
        return Err(CliError::Message(
            "control-path.yaml not found. Kill switch commands require a v2 catalog.".to_string(),
        ));
    }

    match &options.subcommand {
        OverrideSubcommand::Set {
            flag,
            value,
            reason,
            operator,
            file,
            definitions,
            env,
        } => {
            warn_ignored_file_flag(file.as_deref(), env.as_deref())?;
            if operator.is_some() {
                eprintln!("⚠ Note: --operator is not persisted in kill switch files.");
            }
            if definitions.is_some() {
                eprintln!("⚠ Note: --definitions is ignored for kill switch commands.");
            }
            if reason.is_some() {
                eprintln!(
                    "⚠ Note: kill switch files store boolean values only; --reason is not persisted."
                );
            }
            let env_name = kill_switch::resolve_kill_switch_env(env.as_deref())?;
            let path = kill_switch::kill_switch_path(&env_name);
            kill_switch::set_kill_switch_flag(&path, flag, value)?;
            println!(
                "✓ Set kill switch for '{flag}' in {} (environment: {env_name})",
                path.display()
            );
            Ok(())
        }
        OverrideSubcommand::Clear { flag, file, env } => {
            warn_ignored_file_flag(file.as_deref(), env.as_deref())?;
            let env_name = kill_switch::resolve_kill_switch_env(env.as_deref())?;
            let path = kill_switch::kill_switch_path(&env_name);
            kill_switch::clear_kill_switch_flag(&path, flag)?;
            println!("✓ Cleared kill switch for '{flag}' in {}", path.display());
            Ok(())
        }
        OverrideSubcommand::List { file, env } => {
            warn_ignored_file_flag(file.as_deref(), env.as_deref())?;
            let env_name = kill_switch::resolve_kill_switch_env(env.as_deref())?;
            let path = kill_switch::kill_switch_path(&env_name);
            let flags = kill_switch::list_kill_switches(&path)?;
            if flags.is_empty() {
                println!("No kill switches set for {env_name}.");
            } else {
                println!("Kill switches ({env_name}):");
                for (name, value) in flags {
                    println!("  {name}: {value}");
                }
            }
            Ok(())
        }
        OverrideSubcommand::History { flag, file, env } => {
            warn_ignored_file_flag(file.as_deref(), env.as_deref())?;
            let env_name = kill_switch::resolve_kill_switch_env(env.as_deref())?;
            let path = kill_switch::kill_switch_path(&env_name);
            let flags = kill_switch::list_kill_switches(&path)?;
            match flag {
                Some(name) => {
                    if let Some(value) = flags.get(name) {
                        println!("{name}: {value}");
                    } else {
                        return Err(CliError::Message(format!(
                            "No kill switch set for '{name}' in {}",
                            path.display()
                        )));
                    }
                }
                None => {
                    if flags.is_empty() {
                        println!("No kill switches set for {env_name}.");
                    } else {
                        println!("Kill switches ({env_name}):");
                        for (name, value) in flags {
                            println!("  {name}: {value}");
                        }
                    }
                }
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::DirGuard;
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    #[serial]
    fn test_kill_switch_set_and_list() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = DirGuard::new(temp_dir.path()).unwrap();

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            "control-path.yaml",
            r"catalog:
  id: test
mode: local
flags:
  new_dashboard:
    default: false
    kind: release
environments:
  production:
    rules:
      new_dashboard:
        - serve: false
",
        )
        .unwrap();

        let options = Options {
            subcommand: OverrideSubcommand::Set {
                flag: "new_dashboard".to_string(),
                value: "true".to_string(),
                reason: None,
                operator: None,
                file: None,
                definitions: None,
                env: Some("production".to_string()),
            },
        };
        assert_eq!(run(&options), 0);

        let list = Options {
            subcommand: OverrideSubcommand::List {
                file: None,
                env: Some("production".to_string()),
            },
        };
        assert_eq!(run(&list), 0);
    }

    #[test]
    #[serial]
    fn test_kill_switch_rejects_unknown_env() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = DirGuard::new(temp_dir.path()).unwrap();

        fs::write(
            "control-path.yaml",
            r"catalog:
  id: test
mode: local
flags:
  new_dashboard:
    default: false
    kind: release
environments:
  production:
    rules:
      new_dashboard:
        - serve: false
",
        )
        .unwrap();

        let options = Options {
            subcommand: OverrideSubcommand::Set {
                flag: "new_dashboard".to_string(),
                value: "true".to_string(),
                reason: None,
                operator: None,
                file: None,
                definitions: None,
                env: Some("staging".to_string()),
            },
        };
        assert_eq!(run(&options), 1);
    }

    #[test]
    #[serial]
    fn test_kill_switch_rejects_stale_default_env() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = DirGuard::new(temp_dir.path()).unwrap();

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(".controlpath/config.yaml", "defaultEnv: staging\n").unwrap();
        fs::write(
            "control-path.yaml",
            r"catalog:
  id: test
mode: local
flags:
  new_dashboard:
    default: false
    kind: release
environments:
  production:
    rules:
      new_dashboard:
        - serve: false
",
        )
        .unwrap();

        let options = Options {
            subcommand: OverrideSubcommand::Set {
                flag: "new_dashboard".to_string(),
                value: "true".to_string(),
                reason: None,
                operator: None,
                file: None,
                definitions: None,
                env: None,
            },
        };
        assert_eq!(run(&options), 1);
    }
}
