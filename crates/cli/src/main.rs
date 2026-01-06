//! Control Path CLI
//!
//! Copyright 2025 Release Workshop Ltd
//! Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
//! See the LICENSE file in the project root for details.

mod commands;
mod error;
mod generator;
mod utils;

use clap::{Parser, Subcommand};
use commands::{compile, debug, env, explain, flag, generate_sdk, init, setup, validate, watch};

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
    /// Setup a new Control Path project (init + compile + SDK generation)
    Setup {
        /// Language for SDK generation (auto-detected if not provided)
        #[arg(long)]
        lang: Option<String>,
        /// Skip installing runtime SDK package
        #[arg(long)]
        skip_install: bool,
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
    /// Start interactive debug UI
    Debug {
        /// Port for web server (default: 8080)
        #[arg(long)]
        port: Option<u16>,
        /// Environment name (uses .controlpath/<env>.ast)
        #[arg(long)]
        env: Option<String>,
        /// Path to AST file (alternative to --env)
        #[arg(long)]
        ast: Option<String>,
        /// Open browser automatically
        #[arg(long)]
        open: bool,
    },
    /// Manage flags (add, list, show, remove)
    Flag {
        #[command(subcommand)]
        subcommand: FlagSubcommand,
    },
    /// Manage environments (add, sync, list)
    Env {
        #[command(subcommand)]
        subcommand: EnvSubcommand,
    },
}

#[derive(Subcommand)]
enum FlagSubcommand {
    /// Add a new flag to definitions and sync to deployments
    Add {
        /// Flag name (required)
        #[arg(long)]
        name: Option<String>,
        /// Flag type (boolean or multivariate)
        #[arg(long)]
        r#type: Option<String>,
        /// Default value
        #[arg(long)]
        default: Option<String>,
        /// Description
        #[arg(long)]
        description: Option<String>,
        /// Language for SDK regeneration (typescript, python, etc.)
        #[arg(long)]
        lang: Option<String>,
        /// Sync to deployment files (default: prompts)
        #[arg(long)]
        sync: bool,
        /// Disable interactive mode
        #[arg(long)]
        no_interactive: bool,
    },
    /// List flags from definitions or deployment
    List {
        /// List from definitions file
        #[arg(long)]
        definitions: bool,
        /// List from deployment file (specify environment)
        #[arg(long)]
        deployment: Option<String>,
        /// Output format (table, json, yaml)
        #[arg(long, default_value = "table")]
        format: String,
    },
    /// Show detailed information about a flag
    Show {
        /// Flag name
        #[arg(long)]
        name: String,
        /// Show deployment info for environment
        #[arg(long)]
        deployment: Option<String>,
        /// Output format (table, json, yaml)
        #[arg(long)]
        format: Option<String>,
    },
    /// Remove a flag from definitions and deployments
    Remove {
        /// Flag name
        #[arg(long)]
        name: String,
        /// Remove from deployment files (default: true)
        #[arg(long)]
        from_deployments: bool,
        /// Remove from specific environment only
        #[arg(long)]
        env: Option<String>,
        /// Force removal without confirmation
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum EnvSubcommand {
    /// Add a new environment
    Add {
        /// Environment name
        #[arg(long)]
        name: Option<String>,
        /// Template environment to copy from
        #[arg(long)]
        template: Option<String>,
        /// Interactive mode (prompts for missing values)
        #[arg(long)]
        interactive: bool,
    },
    /// Sync flags from definitions to deployment files
    Sync {
        /// Environment to sync (syncs all if not specified)
        #[arg(long)]
        env: Option<String>,
        /// Show what would be synced without making changes
        #[arg(long)]
        dry_run: bool,
    },
    /// List all environments
    List {
        /// Output format (table, json, yaml)
        #[arg(long, default_value = "table")]
        format: String,
    },
    /// Remove an environment
    Remove {
        /// Environment name
        #[arg(long)]
        name: String,
        /// Force removal without confirmation
        #[arg(long)]
        force: bool,
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
        Commands::Setup { lang, skip_install } => {
            let opts = setup::Options {
                lang: lang.clone(),
                skip_install,
            };
            setup::run(&opts)
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
        Commands::Debug {
            port,
            env,
            ast,
            open,
        } => {
            let opts = debug::Options {
                port,
                env,
                ast,
                open,
            };
            debug::run(&opts)
        }
        Commands::Flag { subcommand } => {
            let flag_subcommand = match subcommand {
                FlagSubcommand::Add {
                    name,
                    r#type,
                    default,
                    description,
                    lang,
                    sync,
                    no_interactive,
                } => flag::FlagSubcommand::Add {
                    name,
                    flag_type: r#type,
                    default,
                    description,
                    lang,
                    sync,
                    interactive: !no_interactive,
                },
                FlagSubcommand::List {
                    definitions,
                    deployment,
                    format,
                } => {
                    // Detect TTY for format selection
                    let format_str = if format == "table" && !atty::is(atty::Stream::Stdout) {
                        "json".to_string()
                    } else {
                        format
                    };
                    let output_format = flag::OutputFormat::from_str(&format_str)
                        .unwrap_or(flag::OutputFormat::Table);
                    flag::FlagSubcommand::List {
                        definitions,
                        deployment,
                        format: output_format,
                    }
                }
                FlagSubcommand::Show {
                    name,
                    deployment,
                    format,
                } => {
                    let output_format = format
                        .as_ref()
                        .and_then(|f| flag::OutputFormat::from_str(f).ok())
                        .unwrap_or_else(|| {
                            if atty::is(atty::Stream::Stdout) {
                                flag::OutputFormat::Table
                            } else {
                                flag::OutputFormat::Json
                            }
                        });
                    flag::FlagSubcommand::Show {
                        name,
                        deployment,
                        format: output_format,
                    }
                }
                FlagSubcommand::Remove {
                    name,
                    from_deployments,
                    env,
                    force,
                } => flag::FlagSubcommand::Remove {
                    name,
                    from_deployments,
                    env,
                    force,
                },
            };

            let opts = flag::Options {
                subcommand: flag_subcommand,
            };
            flag::run(&opts)
        }
        Commands::Env { subcommand } => {
            let env_subcommand = match subcommand {
                EnvSubcommand::Add {
                    name,
                    template,
                    interactive,
                } => env::EnvSubcommand::Add {
                    name: name.clone(),
                    template: template.clone(),
                    interactive: interactive || name.is_none(),
                },
                EnvSubcommand::Sync { env, dry_run } => env::EnvSubcommand::Sync {
                    env: env.clone(),
                    dry_run,
                },
                EnvSubcommand::List { format } => {
                    // Detect TTY for format selection
                    let format_str = if format == "table" && !atty::is(atty::Stream::Stdout) {
                        "json".to_string()
                    } else {
                        format.clone()
                    };
                    let output_format = env::OutputFormat::from_str(&format_str)
                        .unwrap_or(env::OutputFormat::Table);
                    env::EnvSubcommand::List {
                        format: output_format,
                    }
                }
                EnvSubcommand::Remove { name, force } => env::EnvSubcommand::Remove {
                    name: name.clone(),
                    force,
                },
            };

            let opts = env::Options {
                subcommand: env_subcommand,
            };
            env::run(&opts)
        }
    };

    std::process::exit(exit_code);
}
