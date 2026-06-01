//! Generate SDK command implementation

use crate::error::CliResult;
use crate::ops::generate_sdk::{generate_sdk_helper, GenerateOptions};
use crate::utils::runtime;
use std::path::PathBuf;

pub struct Options {
    pub lang: Option<String>,
    pub output: Option<String>,
}

pub fn run(options: &Options) -> i32 {
    match run_inner(options) {
        Ok(output_path) => {
            if runtime::is_json_output() {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "ok",
                        "command": "generate-sdk",
                        "artifacts": [output_path.display().to_string()],
                        "warnings": [],
                        "errors": []
                    })
                );
            } else {
                println!("✓ SDK generated successfully");
            }
            0
        }
        Err(e) => {
            if runtime::is_json_output() {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "error",
                        "command": "generate-sdk",
                        "artifacts": [],
                        "warnings": [],
                        "errors": [e.to_string()]
                    })
                );
            } else {
                eprintln!("✗ SDK generation failed");
                eprintln!("  Error: {e}");
            }
            1
        }
    }
}

fn run_inner(options: &Options) -> CliResult<PathBuf> {
    let generate_opts = GenerateOptions {
        lang: options.lang.clone(),
        output: options.output.clone(),
    };
    let output_path = generate_sdk_helper(&generate_opts)?;

    if !runtime::is_json_output() {
        println!("  Generated SDK to {}", output_path.display());
        println!();
        println!("Next steps:");
        println!(
            "  • Import SDK in your code: import {{ evaluator }} from '@controlpath/generated'"
        );
        println!(
            "  • Initialize evaluator:   await evaluator.init({{ artifact: './.controlpath/<env>.ast' }})"
        );
        println!("  • Use flags in code:       const enabled = await evaluator.<flagName>(user)");
    }
    Ok(output_path)
}
