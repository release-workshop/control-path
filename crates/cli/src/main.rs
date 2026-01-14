//! Control Path CLI
//!
//! Copyright 2025 Release Workshop Ltd
//! Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
//! See the LICENSE file in the project root for details.

mod commands;
mod error;
mod generator;
mod monorepo;
mod utils;

use clap::{CommandFactory, Parser, Subcommand};
use commands::{
    compile, completion, debug, env, explain, flag, generate_sdk, init, r#override as override_cmd,
    services, setup, validate, watch, workflow,
};
use std::path::PathBuf;

// Version from VERSION file (set by build.rs) or fallback to Cargo.toml version
// build.rs always sets CONTROLPATH_VERSION, so this is safe
const VERSION: &str = env!("CONTROLPATH_VERSION");

/// Control Path CLI - Compile and validate flag definitions
#[derive(Parser)]
#[command(name = "controlpath")]
#[command(about = "Control Path CLI - Compile and validate flag definitions", long_about = None)]
#[command(version = VERSION)]
struct Cli {
    /// Operate on a specific service (monorepo mode)
    ///
    /// When in a monorepo, specifies which service to operate on.
    /// Can be a service name or a path relative to workspace root.
    #[arg(long, global = true)]
    service: Option<String>,

    /// Explicitly set workspace root (monorepo mode)
    ///
    /// When in a monorepo, explicitly sets the workspace root.
    /// If not provided, the CLI will auto-detect the workspace root.
    #[arg(long, global = true)]
    workspace_root: Option<PathBuf>,

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
        /// Validate all services in monorepo
        #[arg(long)]
        all_services: bool,
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
        /// Compile all services in monorepo
        #[arg(long)]
        all_services: bool,
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
    ///
    /// One-command setup for new projects. Creates project structure, sample flags,
    /// compiles ASTs, installs runtime SDK, and generates type-safe SDKs.
    ///
    /// Examples:
    ///   # Auto-detect language and setup
    ///   controlpath setup
    ///
    ///   # Setup with specific language
    ///   controlpath setup --lang typescript
    ///
    ///   # Setup without installing runtime SDK
    ///   controlpath setup --lang typescript --skip-install
    Setup {
        /// Language for SDK generation (auto-detected if not provided)
        ///
        /// Specifies the language for SDK generation. If not provided, the CLI
        /// will attempt to auto-detect from project files (package.json, requirements.txt, etc.).
        /// Supported languages: typescript, python, go, rust
        #[arg(long)]
        lang: Option<String>,
        /// Skip installing runtime SDK package
        ///
        /// When set, skips the step of installing the runtime SDK package
        /// (e.g., npm install, pip install). Useful if you want to install it manually.
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
        /// Generate SDKs for all services in monorepo
        #[arg(long)]
        all_services: bool,
    },
    /// Watch for file changes and auto-compile/regenerate
    ///
    /// Monitors flag definitions and deployment files for changes and automatically
    /// regenerates SDKs or recompiles ASTs when files are modified.
    ///
    /// Examples:
    ///   # Watch everything (definitions + deployments)
    ///   controlpath watch --lang typescript
    ///
    ///   # Watch definitions only (regenerates SDK on change)
    ///   controlpath watch --definitions --lang typescript
    ///
    ///   # Watch deployments only (recompiles AST on change)
    ///   controlpath watch --deployments
    Watch {
        /// Language for SDK generation (default: typescript)
        ///
        /// Required when watching definitions file. Used to determine which SDK
        /// generator to use when flags.definitions.yaml changes.
        #[arg(long)]
        lang: Option<String>,
        /// Watch definitions file only
        ///
        /// When set, only watches flags.definitions.yaml and regenerates the SDK
        /// when it changes. Requires --lang to be specified.
        #[arg(long)]
        definitions: bool,
        /// Watch deployment files only
        ///
        /// When set, only watches .controlpath/*.deployment.yaml files and
        /// recompiles ASTs when they change.
        #[arg(long)]
        deployments: bool,
    },
    /// Explain flag evaluation with user/context
    ///
    /// Shows detailed information about how a flag evaluates for a given user
    /// and context, including which rules matched and why.
    ///
    /// Examples:
    ///   # Explain with user file
    ///   controlpath explain --flag new_dashboard --user user.json --env production
    ///
    ///   # Explain with detailed trace
    ///   controlpath explain --flag new_dashboard --user user.json --env production --trace
    ///
    ///   # Explain with JSON string
    ///   controlpath explain --flag new_dashboard --user '{"id":"123","role":"admin"}' --env production
    Explain {
        /// Flag name to explain
        #[arg(long)]
        flag: String,
        /// Path to user JSON file or JSON string
        ///
        /// The user object used for evaluation. Can be a file path or a JSON string.
        /// Example: --user user.json or --user '{"id":"123","role":"admin"}'
        #[arg(long)]
        user: Option<String>,
        /// Path to context JSON file or JSON string (optional)
        ///
        /// The context object used for evaluation. Can be a file path or a JSON string.
        #[arg(long)]
        context: Option<String>,
        /// Environment name (uses .controlpath/<env>.ast)
        ///
        /// Specifies which environment's AST to use for evaluation.
        /// If not provided and only one environment exists, it will be used automatically.
        #[arg(long)]
        env: Option<String>,
        /// Path to AST file (alternative to --env)
        ///
        /// Direct path to an AST file. Alternative to --env when you want to
        /// use a specific AST file rather than one from .controlpath/.
        #[arg(long)]
        ast: Option<String>,
        /// Show detailed trace of evaluation
        ///
        /// When set, shows step-by-step evaluation details including expression
        /// parsing, rule matching logic, and intermediate evaluation results.
        #[arg(long)]
        trace: bool,
    },
    /// Start interactive debug UI
    ///
    /// Launches a web-based UI for debugging flag evaluation. The UI allows
    /// you to test flags with different user and context values, see which
    /// rules match, and view detailed evaluation information.
    ///
    /// The debug UI is available at http://localhost:8080 by default.
    ///
    /// Examples:
    ///   # Start debug UI with default settings
    ///   controlpath debug
    ///
    ///   # Start on custom port
    ///   controlpath debug --port 3000
    ///
    ///   # Start and open browser automatically
    ///   controlpath debug --open
    Debug {
        /// Port for web server (default: 8080)
        #[arg(long)]
        port: Option<u16>,
        /// Environment name (uses .controlpath/<env>.ast)
        ///
        /// Specifies which environment's AST to load in the debug UI.
        /// If not provided and only one environment exists, it will be used automatically.
        #[arg(long)]
        env: Option<String>,
        /// Path to AST file (alternative to --env)
        ///
        /// Direct path to an AST file. Alternative to --env when you want to
        /// use a specific AST file rather than one from .controlpath/.
        #[arg(long)]
        ast: Option<String>,
        /// Open browser automatically
        ///
        /// When set, automatically opens the default web browser to the debug UI.
        #[arg(long)]
        open: bool,
    },
    /// Manage flags (add, list, show, remove)
    ///
    /// Commands for managing flag definitions and deployments.
    ///
    /// Examples:
    ///   # Add a new flag
    ///   controlpath flag add --name my_feature --type boolean
    ///
    ///   # List all flags
    ///   controlpath flag list
    ///
    ///   # Show flag details
    ///   controlpath flag show --name my_feature
    ///
    ///   # Remove a flag
    ///   controlpath flag remove --name my_feature
    Flag {
        #[command(subcommand)]
        subcommand: FlagSubcommand,
    },
    /// Manage environments (add, sync, list, remove)
    ///
    /// Commands for managing deployment environments.
    ///
    /// Examples:
    ///   # Add a new environment
    ///   controlpath env add --name staging
    ///
    ///   # Sync flags to all environments
    ///   controlpath env sync
    ///
    ///   # List all environments
    ///   controlpath env list
    ///
    ///   # Remove an environment
    ///   controlpath env remove --name staging
    Env {
        #[command(subcommand)]
        subcommand: EnvSubcommand,
    },
    /// Complete workflow for adding a new flag (add, sync, regenerate SDK)
    NewFlag {
        /// Flag name (optional, prompts if not provided)
        #[arg(value_name = "NAME")]
        name: Option<String>,
        /// Flag type (boolean, multivariate)
        #[arg(long)]
        r#type: Option<String>,
        /// Default value
        #[arg(long)]
        default: Option<String>,
        /// Description
        #[arg(long)]
        description: Option<String>,
        /// Enable flag in specific environment(s) (comma-separated)
        #[arg(long)]
        enable_in: Option<String>,
        /// Don't sync to environments
        #[arg(long)]
        skip_sync: bool,
        /// Don't regenerate SDK
        #[arg(long)]
        skip_sdk: bool,
    },
    /// Enable a flag in one or more environments with a rule
    Enable {
        /// Flag name (required)
        #[arg(value_name = "NAME")]
        name: String,
        /// Environment(s) (comma-separated, prompts if not provided)
        #[arg(long)]
        env: Option<String>,
        /// Rule expression (e.g., "user.role == 'admin'")
        #[arg(long)]
        rule: Option<String>,
        /// Enable for all users (no rule, just serve default)
        #[arg(long)]
        all: bool,
        /// Value to serve (for boolean: true/false, for multivariate: variation name)
        #[arg(long)]
        value: Option<String>,
        /// Interactive rule builder
        #[arg(long)]
        interactive: bool,
    },
    /// Validate, compile, and prepare flags for deployment
    Deploy {
        /// Environment(s) to deploy (comma-separated, defaults to all)
        #[arg(long)]
        env: Option<String>,
        /// Validate and compile but show what would happen
        #[arg(long)]
        dry_run: bool,
        /// Skip validation step
        #[arg(long)]
        skip_validation: bool,
        /// Deploy all services in monorepo
        #[arg(long)]
        all_services: bool,
    },
    /// Manage override files (kill switches)
    ///
    /// Commands for managing runtime flag overrides without redeploying code.
    /// Override files can be stored locally and uploaded to any URL-accessible location.
    ///
    /// Examples:
    ///   # Set an override
    ///   controlpath override set new_dashboard OFF --file overrides.json --reason "Emergency kill switch"
    ///
    ///   # Clear an override
    ///   controlpath override clear new_dashboard --file overrides.json
    ///
    ///   # List all overrides
    ///   controlpath override list --file overrides.json
    ///
    ///   # View override history
    ///   controlpath override history new_dashboard --file overrides.json
    Override {
        #[command(subcommand)]
        subcommand: OverrideSubcommand,
    },
    /// Manage services in a monorepo
    ///
    /// Commands for listing and checking status of services in a monorepo.
    ///
    /// Examples:
    ///   # List all services
    ///   controlpath services list
    ///
    ///   # List with detailed information
    ///   controlpath services list --detailed
    ///
    ///   # List as JSON
    ///   controlpath services list --format json
    ///
    ///   # Check status of all services
    ///   controlpath services status
    ///
    ///   # Check status of specific service
    ///   controlpath services status --service service-a
    ///
    ///   # Check sync status
    ///   controlpath services status --check-sync
    Services {
        #[command(subcommand)]
        subcommand: ServicesSubcommand,
    },
    /// Generate shell completion scripts
    Completion {
        /// Shell type (bash, zsh, fish)
        #[arg(value_name = "SHELL")]
        shell: String,
    },
}

#[derive(Subcommand)]
enum FlagSubcommand {
    /// Add a new flag to definitions and sync to deployments
    ///
    /// Adds a flag to flags.definitions.yaml and optionally syncs it to all
    /// deployment files. Runs in interactive mode by default, prompting for
    /// missing values.
    ///
    /// Examples:
    ///   # Interactive mode (prompts for values)
    ///   controlpath flag add
    ///
    ///   # Add with all options
    ///   controlpath flag add --name my_feature --type boolean --default false --description "My feature flag"
    ///
    ///   # Add and sync to deployments
    ///   controlpath flag add --name my_feature --sync
    Add {
        /// Flag name (required, snake_case format)
        ///
        /// The name of the flag to add. Must be in snake_case format (e.g., my_feature).
        /// If not provided, will prompt in interactive mode.
        #[arg(long)]
        name: Option<String>,
        /// Flag type (boolean or multivariate)
        ///
        /// The type of flag. Use 'boolean' for true/false flags or 'multivariate'
        /// for flags with multiple variations. Defaults to 'boolean'.
        #[arg(long)]
        r#type: Option<String>,
        /// Default value
        ///
        /// The default value for the flag. For boolean flags, use 'true' or 'false'.
        /// For multivariate flags, use the variation name.
        #[arg(long)]
        default: Option<String>,
        /// Description
        ///
        /// A human-readable description of what the flag controls.
        #[arg(long)]
        description: Option<String>,
        /// Language for SDK regeneration (typescript, python, etc.)
        ///
        /// If provided, regenerates the SDK after adding the flag.
        /// If not provided, SDK is not regenerated automatically.
        #[arg(long)]
        lang: Option<String>,
        /// Sync to deployment files (default: prompts)
        ///
        /// When set, automatically syncs the flag to all deployment files
        /// (disabled by default). If not set, will prompt for confirmation.
        #[arg(long)]
        sync: bool,
        /// Disable interactive mode
        ///
        /// When set, disables interactive prompts. All required values must be
        /// provided via command-line flags.
        #[arg(long)]
        no_interactive: bool,
    },
    /// List flags from definitions or deployment
    ///
    /// Lists all flags from either the definitions file or a deployment file.
    /// Output can be formatted as a table (default), JSON, or YAML.
    ///
    /// Examples:
    ///   # List from definitions (default)
    ///   controlpath flag list
    ///
    ///   # List from specific deployment
    ///   controlpath flag list --deployment production
    ///
    ///   # List as JSON
    ///   controlpath flag list --format json
    List {
        /// List from definitions file
        ///
        /// When set, lists flags from flags.definitions.yaml.
        /// This is the default behavior if no flags are specified.
        #[arg(long)]
        definitions: bool,
        /// List from deployment file (specify environment)
        ///
        /// Lists flags from a specific environment's deployment file.
        /// Example: --deployment production
        #[arg(long)]
        deployment: Option<String>,
        /// Output format (table, json, yaml)
        ///
        /// The output format. Defaults to 'table' for TTY output, 'json' for piped output.
        #[arg(long, default_value = "table")]
        format: String,
    },
    /// Show detailed information about a flag
    ///
    /// Shows comprehensive information about a specific flag, including its
    /// definition and deployment rules across environments.
    ///
    /// Examples:
    ///   # Show flag details
    ///   controlpath flag show --name my_feature
    ///
    ///   # Show flag in specific environment
    ///   controlpath flag show --name my_feature --deployment production
    ///
    ///   # Show as JSON
    ///   controlpath flag show --name my_feature --format json
    Show {
        /// Flag name
        ///
        /// The name of the flag to show details for.
        #[arg(long)]
        name: String,
        /// Show deployment info for environment
        ///
        /// When specified, shows deployment-specific information for the flag
        /// in the given environment.
        #[arg(long)]
        deployment: Option<String>,
        /// Output format (table, json, yaml)
        ///
        /// The output format. Defaults to 'table' for TTY output, 'json' for piped output.
        #[arg(long)]
        format: Option<String>,
    },
    /// Remove a flag from definitions and deployments
    ///
    /// Removes a flag from flags.definitions.yaml and optionally from deployment files.
    /// Shows a confirmation prompt unless --force is used.
    ///
    /// Examples:
    ///   # Remove from definitions only
    ///   controlpath flag remove --name my_feature --from-deployments false
    ///
    ///   # Remove from all deployments
    ///   controlpath flag remove --name my_feature
    ///
    ///   # Remove from specific environment
    ///   controlpath flag remove --name my_feature --env staging
    ///
    ///   # Force removal without confirmation
    ///   controlpath flag remove --name my_feature --force
    Remove {
        /// Flag name
        ///
        /// The name of the flag to remove.
        #[arg(long)]
        name: String,
        /// Remove from deployment files (default: true)
        ///
        /// When set (default), also removes the flag from deployment files.
        /// Set to false to only remove from definitions.
        #[arg(long)]
        from_deployments: bool,
        /// Remove from specific environment only
        ///
        /// When specified, only removes the flag from the given environment's
        /// deployment file. Otherwise removes from all deployment files.
        #[arg(long)]
        env: Option<String>,
        /// Force removal without confirmation
        ///
        /// When set, skips the confirmation prompt and immediately removes the flag.
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum OverrideSubcommand {
    /// Set an override for a flag
    ///
    /// Sets a runtime override for a flag. The override takes precedence over AST evaluation.
    /// Supports both boolean flags (ON/OFF) and multivariate flags (variation name).
    ///
    /// Examples:
    ///   # Set boolean flag to OFF
    ///   controlpath override set new_dashboard OFF --file overrides.json
    ///
    ///   # Set with reason and operator
    ///   controlpath override set new_dashboard OFF --file overrides.json --reason "Emergency kill switch" --operator "alice@example.com"
    ///
    ///   # Set multivariate flag variation
    ///   controlpath override set api_version V1 --file overrides.json
    Set {
        /// Flag name
        #[arg(value_name = "FLAG")]
        flag: String,
        /// Override value (ON/OFF for boolean, variation name for multivariate)
        #[arg(value_name = "VALUE")]
        value: String,
        /// Reason for override (optional, recommended for audit trail)
        #[arg(long)]
        reason: Option<String>,
        /// Operator who set the override (optional, recommended for audit trail)
        #[arg(long)]
        operator: Option<String>,
        /// Path to override file
        #[arg(long, default_value = "overrides.json")]
        file: String,
        /// Path to flag definitions file (for validation)
        #[arg(long)]
        definitions: Option<String>,
    },
    /// Clear an override for a flag
    ///
    /// Removes a runtime override for a flag. The flag will fall back to AST evaluation.
    ///
    /// Examples:
    ///   # Clear override
    ///   controlpath override clear new_dashboard --file overrides.json
    Clear {
        /// Flag name
        #[arg(value_name = "FLAG")]
        flag: String,
        /// Path to override file
        #[arg(long, default_value = "overrides.json")]
        file: String,
    },
    /// List all current overrides
    ///
    /// Displays all current overrides with their values, timestamps, reasons, and operators.
    ///
    /// Examples:
    ///   # List all overrides
    ///   controlpath override list --file overrides.json
    List {
        /// Path to override file
        #[arg(long, default_value = "overrides.json")]
        file: String,
    },
    /// View override history (audit trail)
    ///
    /// Displays current overrides with their audit trail information (reason, timestamp, operator).
    /// The override file itself serves as the audit trail - no separate history file is needed.
    ///
    /// Examples:
    ///   # View history for a specific flag
    ///   controlpath override history new_dashboard --file overrides.json
    ///
    ///   # View history for all flags
    ///   controlpath override history --file overrides.json
    History {
        /// Flag name (optional, shows all flags if not provided)
        #[arg(value_name = "FLAG")]
        flag: Option<String>,
        /// Path to override file
        #[arg(long, default_value = "overrides.json")]
        file: String,
    },
}

#[derive(Subcommand)]
enum EnvSubcommand {
    /// Add a new environment
    ///
    /// Creates a new deployment environment by creating a .controlpath/<name>.deployment.yaml
    /// file. Can optionally copy flags from a template environment.
    ///
    /// Examples:
    ///   # Add new environment (interactive)
    ///   controlpath env add
    ///
    ///   # Add with name
    ///   controlpath env add --name staging
    ///
    ///   # Add with template
    ///   controlpath env add --name staging --template production
    Add {
        /// Environment name
        ///
        /// The name of the environment to create. If not provided, will prompt
        /// in interactive mode.
        #[arg(long)]
        name: Option<String>,
        /// Template environment to copy from
        ///
        /// When specified, copies all flags and rules from the template environment
        /// to the new environment. Otherwise, creates a new deployment with all
        /// flags disabled by default.
        #[arg(long)]
        template: Option<String>,
        /// Interactive mode (prompts for missing values)
        ///
        /// When set, prompts for missing values. This is the default behavior
        /// when name is not provided.
        #[arg(long)]
        interactive: bool,
    },
    /// Sync flags from definitions to deployment files
    ///
    /// Synchronizes flags from flags.definitions.yaml to deployment files.
    /// Adds missing flags (disabled by default) and optionally removes flags
    /// that no longer exist in definitions.
    ///
    /// Examples:
    ///   # Sync all environments
    ///   controlpath env sync
    ///
    ///   # Sync specific environment
    ///   controlpath env sync --env staging
    ///
    ///   # Dry run (show what would be synced)
    ///   controlpath env sync --dry-run
    Sync {
        /// Environment to sync (syncs all if not specified)
        ///
        /// When specified, only syncs the given environment. Otherwise, syncs
        /// all environments found in .controlpath/.
        #[arg(long)]
        env: Option<String>,
        /// Show what would be synced without making changes
        ///
        /// When set, shows what would be synced but doesn't actually modify
        /// any files. Useful for previewing changes.
        #[arg(long)]
        dry_run: bool,
    },
    /// List all environments
    ///
    /// Lists all deployment environments found in .controlpath/.
    ///
    /// Examples:
    ///   # List as table (default)
    ///   controlpath env list
    ///
    ///   # List as JSON
    ///   controlpath env list --format json
    List {
        /// Output format (table, json, yaml)
        ///
        /// The output format. Defaults to 'table' for TTY output, 'json' for piped output.
        #[arg(long, default_value = "table")]
        format: String,
    },
    /// Remove an environment
    ///
    /// Removes a deployment environment by deleting its .controlpath/<name>.deployment.yaml
    /// file. Shows a confirmation prompt unless --force is used.
    ///
    /// Examples:
    ///   # Remove environment (with confirmation)
    ///   controlpath env remove --name staging
    ///
    ///   # Force removal without confirmation
    ///   controlpath env remove --name staging --force
    Remove {
        /// Environment name
        ///
        /// The name of the environment to remove.
        #[arg(long)]
        name: String,
        /// Force removal without confirmation
        ///
        /// When set, skips the confirmation prompt and immediately removes the environment.
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum ServicesSubcommand {
    /// List all services in monorepo
    ///
    /// Lists all services found in the monorepo. Can show simple or detailed information.
    ///
    /// Examples:
    ///   # List all services
    ///   controlpath services list
    ///
    ///   # List with detailed information
    ///   controlpath services list --detailed
    ///
    ///   # List as JSON
    ///   controlpath services list --format json
    List {
        /// Show detailed information (flag counts, environments, etc.)
        #[arg(long)]
        detailed: bool,
        /// Output format (table, json)
        #[arg(long, default_value = "table")]
        format: String,
    },
    /// Show status of services
    ///
    /// Shows status information for services, including flag definitions, deployments,
    /// environments, and SDK generation status. Can optionally check sync status.
    ///
    /// Examples:
    ///   # Check status of all services
    ///   controlpath services status
    ///
    ///   # Check status of specific service
    ///   controlpath services status --service service-a
    ///
    ///   # Check sync status
    ///   controlpath services status --check-sync
    Status {
        /// Specific service to check (shows all if not provided)
        #[arg(long)]
        service: Option<String>,
        /// Check sync status between definitions and deployments
        #[arg(long)]
        check_sync: bool,
    },
}

/// Get the CLI command structure for completion generation
pub fn get_cli_command() -> clap::Command {
    Cli::command()
}

fn main() {
    let cli = Cli::parse();

    // Resolve service context for monorepo support
    let service_context =
        monorepo::resolve_service_context(cli.service.as_deref(), cli.workspace_root.as_deref())
            .unwrap_or_else(|e| {
                eprintln!("Error resolving service context: {e}");
                std::process::exit(1);
            });

    let exit_code = match cli.command {
        Commands::Validate {
            definitions,
            deployment,
            env,
            all,
            all_services,
        } => {
            let opts = validate::Options {
                definitions,
                deployment,
                env,
                all,
                all_services,
                service_context: Some(service_context.clone()),
            };
            validate::run(&opts)
        }
        Commands::Compile {
            deployment,
            env,
            output,
            definitions,
            all_services,
        } => {
            let opts = compile::Options {
                deployment,
                env,
                output,
                definitions,
                service_context: Some(service_context.clone()),
                all_services,
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
            all_services,
        } => {
            let opts = generate_sdk::Options {
                lang,
                output,
                definitions,
                all_services,
                service_context: Some(service_context.clone()),
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
        Commands::NewFlag {
            name,
            r#type,
            default,
            description,
            enable_in,
            skip_sync,
            skip_sdk,
        } => {
            let opts = workflow::NewFlagOptions {
                name,
                flag_type: r#type,
                default,
                description,
                enable_in,
                skip_sync,
                skip_sdk,
            };
            workflow::run_new_flag(&opts)
        }
        Commands::Enable {
            name,
            env,
            rule,
            all,
            value,
            interactive,
        } => {
            let opts = workflow::EnableOptions {
                name,
                env,
                rule,
                all,
                value,
                interactive,
            };
            workflow::run_enable(&opts)
        }
        Commands::Deploy {
            env,
            dry_run,
            skip_validation,
            all_services,
        } => {
            let opts = workflow::DeployOptions {
                env,
                dry_run,
                skip_validation,
                all_services,
                service_context: Some(service_context.clone()),
            };
            workflow::run_deploy(&opts)
        }
        Commands::Override { subcommand } => {
            let override_subcommand = match subcommand {
                OverrideSubcommand::Set {
                    flag,
                    value,
                    reason,
                    operator,
                    file,
                    definitions,
                } => override_cmd::OverrideSubcommand::Set {
                    flag,
                    value,
                    reason,
                    operator,
                    file: PathBuf::from(file),
                    definitions: definitions.map(PathBuf::from),
                },
                OverrideSubcommand::Clear { flag, file } => {
                    override_cmd::OverrideSubcommand::Clear {
                        flag,
                        file: PathBuf::from(file),
                    }
                }
                OverrideSubcommand::List { file } => override_cmd::OverrideSubcommand::List {
                    file: PathBuf::from(file),
                },
                OverrideSubcommand::History { flag, file } => {
                    override_cmd::OverrideSubcommand::History {
                        flag,
                        file: PathBuf::from(file),
                    }
                }
            };

            let opts = override_cmd::Options {
                subcommand: override_subcommand,
            };
            override_cmd::run(&opts)
        }
        Commands::Services { subcommand } => {
            let services_subcommand = match subcommand {
                ServicesSubcommand::List { detailed, format } => {
                    // Detect TTY for format selection
                    let format_str = if format == "table" && !atty::is(atty::Stream::Stdout) {
                        "json".to_string()
                    } else {
                        format
                    };
                    let output_format = services::OutputFormat::from_str(&format_str)
                        .unwrap_or(services::OutputFormat::Table);
                    services::ServicesSubcommand::List {
                        detailed,
                        format: output_format,
                    }
                }
                ServicesSubcommand::Status {
                    service,
                    check_sync,
                } => services::ServicesSubcommand::Status {
                    service,
                    check_sync,
                },
            };

            let opts = services::Options {
                subcommand: services_subcommand,
            };
            services::run(&opts)
        }
        Commands::Completion { shell } => {
            let opts = completion::Options { shell };
            completion::run(&opts)
        }
    };

    std::process::exit(exit_code);
}
