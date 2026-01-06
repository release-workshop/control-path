//! Shell completion command implementation

use crate::error::{CliError, CliResult};
use crate::get_cli_command;
use clap_complete::{generate, Shell};
use std::io;

pub struct Options {
    pub shell: String,
}

/// Generate shell completion script
pub fn run(options: &Options) -> i32 {
    match run_inner(options) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("✗ Completion generation failed");
            eprintln!("  Error: {e}");
            1
        }
    }
}

fn run_inner(options: &Options) -> CliResult<()> {
    if options.shell.is_empty() {
        return Err(CliError::Message(
            "Shell name is required. Supported shells: bash, zsh, fish".to_string(),
        ));
    }

    let shell = match options.shell.to_lowercase().as_str() {
        "bash" => Shell::Bash,
        "zsh" => Shell::Zsh,
        "fish" => Shell::Fish,
        _ => {
            return Err(CliError::Message(format!(
                "Unsupported shell: {}. Supported shells: bash, zsh, fish",
                options.shell
            )));
        }
    };

    // Get the CLI command structure from main.rs
    let mut cmd = get_cli_command();

    // Generate completion script
    generate(shell, &mut cmd, "controlpath", &mut io::stdout());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_cli_command() {
        // Test that we can get the CLI command structure
        let cmd = get_cli_command();
        assert_eq!(cmd.get_name(), "controlpath");
    }

    #[test]
    fn test_completion_options_shell_validation() {
        // Test valid shells (case-insensitive)
        let valid_shells = vec![
            "bash", "BASH", "Bash", "zsh", "ZSH", "Zsh", "fish", "FISH", "Fish",
        ];

        for shell in valid_shells {
            let options = Options {
                shell: shell.to_string(),
            };
            // Should not panic - we can't easily test the output without capturing stdout
            // but we can verify the shell parsing works
            let result = run_inner(&options);
            assert!(result.is_ok(), "Shell '{}' should be valid", shell);
        }
    }

    #[test]
    fn test_completion_options_invalid_shell() {
        let options = Options {
            shell: "powershell".to_string(),
        };
        let result = run_inner(&options);
        assert!(result.is_err());
        if let Err(CliError::Message(msg)) = result {
            assert!(msg.contains("Unsupported shell"));
            assert!(msg.contains("powershell"));
        } else {
            panic!("Expected CliError::Message for invalid shell");
        }
    }

    #[test]
    fn test_completion_options_empty_shell() {
        let options = Options {
            shell: String::new(),
        };
        let result = run_inner(&options);
        assert!(result.is_err());
        if let Err(CliError::Message(msg)) = result {
            assert!(msg.contains("Shell name is required"));
        } else {
            panic!("Expected CliError::Message for empty shell");
        }
    }
}
