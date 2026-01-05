//! Control Path CLI
//!
//! Copyright 2025 Release Workshop Ltd
//! Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
//! See the LICENSE file in the project root for details.

mod commands;
mod error;
mod generator;

use clap::{Parser, Subcommand};
use commands::{compile, explain, generate_sdk, init, validate, watch};

/// Control Path CLI - Compile and validate flag definitions
#[derive(Parser)]
#[command(name = "controlpath")]
#[command(about = "Control Path CLI - Compile and validate flag definitions", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate flag definitions and deployment files
    Validate {
        /// Path to flag definitions file
        #[arg(long)]
        definitions: Option<String>,
        /// Path to deployment file
        #[arg(long)]
        deployment: Option<String>,
        /// Environment name (uses .controlpath/<env>.deployment.yaml)
        #[arg(long)]
        env: Option<String>,
        /// Validate all files (auto-detect)
        #[arg(long)]
        all: bool,
    },
    /// Compile deployment files to AST artifacts
    Compile {
        /// Path to deployment file
        #[arg(long)]
        deployment: Option<String>,
        /// Environment name (uses .controlpath/<env>.deployment.yaml)
        #[arg(long)]
        env: Option<String>,
        /// Output path for AST file
        #[arg(long)]
        output: Option<String>,
        /// Path to flag definitions file
        #[arg(long)]
        definitions: Option<String>,
    },
    /// Initialize a new Control Path project
    Init {
        /// Overwrite existing files
        #[arg(long)]
        force: bool,
        /// Create example flags
        #[arg(long)]
        example_flags: bool,
        /// Skip creating example files
        #[arg(long)]
        no_examples: bool,
    },
    /// Generate type-safe SDKs from flag definitions
    GenerateSdk {
        /// Language (typescript, python, etc.)
        #[arg(long)]
        lang: Option<String>,
        /// Output directory
        #[arg(long)]
        output: Option<String>,
        /// Path to flag definitions file
        #[arg(long)]
        definitions: Option<String>,
    },
    /// Watch for file changes and auto-compile/regenerate
    Watch {
        /// Language for SDK generation (default: typescript)
        #[arg(long)]
        lang: Option<String>,
        /// Watch definitions file only
        #[arg(long)]
        definitions: bool,
        /// Watch deployment files only
        #[arg(long)]
        deployments: bool,
    },
    /// Explain flag evaluation with user/context
    Explain {
        /// Flag name to explain
        #[arg(long)]
        flag: String,
        /// Path to user JSON file
        #[arg(long)]
        user: Option<String>,
        /// Path to context JSON file (optional)
        #[arg(long)]
        context: Option<String>,
        /// Environment name (uses .controlpath/<env>.ast)
        #[arg(long)]
        env: Option<String>,
        /// Path to AST file (alternative to --env)
        #[arg(long)]
        ast: Option<String>,
        /// Show detailed trace of evaluation
        #[arg(long)]
        trace: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        Commands::Validate {
            definitions,
            deployment,
            env,
            all,
        } => {
            let opts = validate::Options {
                definitions,
                deployment,
                env,
                all,
            };
            validate::run(&opts)
        }
        Commands::Compile {
            deployment,
            env,
            output,
            definitions,
        } => {
            let opts = compile::Options {
                deployment,
                env,
                output,
                definitions,
            };
            compile::run(&opts)
        }
        Commands::Init {
            force,
            example_flags,
            no_examples,
        } => {
            let opts = init::Options {
                force,
                example_flags,
                no_examples,
            };
            init::run(&opts)
        }
        Commands::GenerateSdk {
            lang,
            output,
            definitions,
        } => {
            let opts = generate_sdk::Options {
                lang,
                output,
                definitions,
            };
            generate_sdk::run(&opts)
        }
        Commands::Watch {
            lang,
            definitions,
            deployments,
        } => {
            let opts = watch::Options {
                lang,
                definitions,
                deployments,
            };
            watch::run(&opts)
        }
        Commands::Explain {
            flag,
            user,
            context,
            env,
            ast,
            trace,
        } => {
            let opts = explain::Options {
                flag,
                user,
                context,
                env,
                ast,
                trace,
            };
            explain::run(&opts)
        }
    };

    std::process::exit(exit_code);
}
