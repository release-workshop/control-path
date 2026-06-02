//! Kill switch management command (v2 boolean catalog).

use crate::error::{CliError, CliResult};
use crate::utils::kill_switch;
use crate::utils::unified_config;

pub struct Options {
    pub subcommand: KillSwitchSubcommand,
}

#[derive(Debug, Clone)]
pub enum KillSwitchSubcommand {
    Set {
        flag: String,
        value: String,
        env: String,
    },
    Clear {
        flag: String,
        env: String,
    },
    List {
        env: String,
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

fn run_inner(options: &Options) -> CliResult<()> {
    if !unified_config::unified_config_exists() {
        return Err(CliError::Message(
            "control-path.yaml not found. Kill switch commands require a v2 catalog.".to_string(),
        ));
    }

    let unified = unified_config::read_unified_config()?;
    if unified_config::is_saas_mode(&unified) {
        match options.subcommand {
            KillSwitchSubcommand::List { .. } => {}
            _ => {
                return Err(CliError::Message(
                    "Kill switch set/clear are local-mode only. In SaaS mode, manage kill switches in the control plane."
                        .to_string(),
                ));
            }
        }
    }

    match &options.subcommand {
        KillSwitchSubcommand::Set { flag, value, env } => {
            let env_name = kill_switch::require_kill_switch_env(Some(env))?;
            let path = kill_switch::kill_switch_path(&env_name);
            kill_switch::set_kill_switch_flag(&path, flag, value)?;
            println!(
                "✓ Set kill switch for '{flag}' in {} (environment: {env_name})",
                path.display()
            );
            Ok(())
        }
        KillSwitchSubcommand::Clear { flag, env } => {
            let env_name = kill_switch::require_kill_switch_env(Some(env))?;
            let path = kill_switch::kill_switch_path(&env_name);
            kill_switch::clear_kill_switch_flag(&path, flag)?;
            println!("✓ Cleared kill switch for '{flag}' in {}", path.display());
            Ok(())
        }
        KillSwitchSubcommand::List { env } => {
            let env_name = kill_switch::require_kill_switch_env(Some(env))?;
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
  emergency_stop:
    default: false
    kind: kill_switch
environments:
  production:
    rules:
      emergency_stop:
        - serve: false
",
        )
        .unwrap();

        let options = Options {
            subcommand: KillSwitchSubcommand::Set {
                flag: "emergency_stop".to_string(),
                value: "true".to_string(),
                env: "production".to_string(),
            },
        };
        assert_eq!(run(&options), 0);

        let list = Options {
            subcommand: KillSwitchSubcommand::List {
                env: "production".to_string(),
            },
        };
        assert_eq!(run(&list), 0);
    }

    #[test]
    #[serial]
    fn test_kill_switch_set_rejects_non_kill_switch_flag() {
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
            subcommand: KillSwitchSubcommand::Set {
                flag: "new_dashboard".to_string(),
                value: "true".to_string(),
                env: "production".to_string(),
            },
        };
        assert_eq!(run(&options), 1);
    }

    #[test]
    #[serial]
    fn test_kill_switch_set_rejects_saas_mode() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = DirGuard::new(temp_dir.path()).unwrap();

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            "control-path.yaml",
            r"catalog:
  id: test
mode: saas
saas:
  project: acme/test
flags:
  emergency_stop:
    default: false
    kind: kill_switch
",
        )
        .unwrap();

        let options = Options {
            subcommand: KillSwitchSubcommand::Set {
                flag: "emergency_stop".to_string(),
                value: "true".to_string(),
                env: "production".to_string(),
            },
        };
        assert_eq!(run(&options), 1);
    }
}
