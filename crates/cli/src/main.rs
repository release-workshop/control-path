//! Control Path CLI
//!
//! Copyright 2025 Release Workshop Ltd
//! Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
//! See the LICENSE file in the project root for details.

mod commands;
mod error;
mod generator;
mod ops;
mod saas;
mod utils;

#[cfg(test)]
mod test_helpers;

use clap::{ArgAction, CommandFactory, Parser, Subcommand};
use commands::{
    ci, compile, completion, debug, dev, env, explain, flag, generate_sdk, init,
    r#override as override_cmd, setup, validate, watch, workflow,
};
use std::path::PathBuf;
use utils::runtime::{init_runtime_options, RuntimeOptions};

// Version from VERSION file (set by build.rs) or fallback to Cargo.toml version
// build.rs always sets CONTROLPATH_VERSION, so this is safe
const VERSION: &str = env!("CONTROLPATH_VERSION");

/// Control Path CLI - Manage feature flags with a Git-native workflow
#[derive(Parser)]
#[command(name = "controlpath")]
#[command(about = "Control Path CLI - Manage feature flags with a Git-native workflow")]
#[command(version = VERSION)]
#[command(
    long_about = r#"Control Path CLI - Manage feature flags with a Git-native workflow

Getting Started

New to Control Path? Follow these steps:

  1. controlpath setup               # Initialize a new project
  2. controlpath new-flag <name>     # Add your first flag
  3. controlpath flag enable <flag>  # Enable flag in an environment
  4. controlpath dev                 # Start development mode

Core Concepts

Control Path is built around three core concepts:

1. Configuration -> Config file (control-path.yaml)
   Flags, their types/defaults, and environment-specific rules

2. SDK -> Generated code (default: node_modules/@controlpath/generated)
   Type-safe SDK that your application code imports and uses

3. AST Artifacts -> Compiled artifacts (.controlpath/<env>.ast)
   Compiled flag configurations per environment (generated automatically)

Everything else (AST artifacts, compiler details) is handled automatically
by the CLI as part of higher-level workflows.

Argument Syntax

For value-based options, both forms are supported:
  --key value
  --key=value

Command Groups

Workflow Commands (start here):
  setup - Initialize a new Control Path project
  new-flag - Add a new flag (recommended workflow)
  deploy - Prepare flags for deployment
  dev - Development mode with auto-compile/watch

Core Commands:
  validate - Validate catalog and environment configuration
  compile - Compile catalog environments to AST artifacts
  generate-sdk - Generate type-safe SDK from flag definitions

Management Commands:
  flag - Manage flags (add, list, show, remove)
  env - Manage environments (add, sync, list, remove)
  kill-switch - Manage runtime kill switches (alias: override)

Debug Commands:
  explain - Explain flag evaluation with user/context
  debug - Start interactive debug UI

Development Commands:
  watch - Watch for file changes and auto-compile/regenerate
  dev - Development workflow with smart defaults

CI Commands:
  ci - CI pipeline workflow (validate, compile, regenerate SDK)

Utility Commands:
  completion - Generate shell completion scripts"#
)]
struct Cli {
    /// Emit machine-readable JSON output where supported
    #[arg(long, global = true)]
    json: bool,
    /// Disable all interactive prompts
    #[arg(long, global = true)]
    non_interactive: bool,
    /// Increase verbosity (-v, -vv)
    #[arg(short = 'v', long, action = ArgAction::Count, global = true)]
    verbose: u8,
    /// Reduce output to errors only where supported
    #[arg(short = 'q', long, global = true)]
    quiet: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate catalog and environment configuration
    ///
    /// Use this to check that your configuration files are valid before deploying.
    /// Validates control-path.yaml against JSON schemas. Usually called automatically
    /// by deploy and ci commands, but useful for manual validation.
    ///
    /// When to use:
    ///   - Before committing changes to verify configuration is correct
    ///   - In CI/CD pipelines to catch configuration errors early
    ///   - When troubleshooting flag evaluation issues
    Validate {
        /// Environment name (extracts from control-path.yaml)
        #[arg(long)]
        env: Option<String>,
        /// Validate all files (auto-detect)
        #[arg(long)]
        all: bool,
    },
    /// Compile catalog environments to AST artifacts
    ///
    /// Compiles environment rules from control-path.yaml into `.controlpath/<env>.ast`.
    /// Usually called automatically by enable, deploy, dev, and ci commands.
    ///
    /// When to use:
    ///   - Manually compiling after catalog changes
    ///   - Preparing AST files for deployment to production
    ///   - Testing compilation for specific environments
    Compile {
        /// Environment name (extracts from control-path.yaml)
        #[arg(long)]
        env: Option<String>,
        /// Output path for AST file
        #[arg(long)]
        output: Option<String>,
    },
    /// Setup a new Control Path project (primary bootstrap command)
    ///
    /// One-command setup for new projects. Creates:
    /// - Configuration file (control-path.yaml) with example flags
    /// - Generated SDK (default: node_modules/@controlpath/generated) for your application code
    ///
    /// Also installs runtime SDK and compiles ASTs automatically.
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
        /// Skip creating example flags and usage files
        ///
        /// When set, creates a minimal project without example flags or example usage files.
        #[arg(long)]
        no_examples: bool,
    },
    /// Initialize workspace or service catalog files (monorepo scaffold)
    Init {
        /// Create a monorepo workspace file at the current directory
        #[arg(long)]
        monorepo: bool,
        /// Skip monorepo workspace creation (multi-repo setup)
        #[arg(long)]
        no_monorepo: bool,
        /// Catalog namespace (multi-repo or workspace)
        #[arg(long)]
        namespace: Option<String>,
        /// Service catalog id when scaffolding from a workspace
        #[arg(long)]
        service_id: Option<String>,
    },
    /// Generate type-safe SDK from flag definitions
    ///
    /// Use this to regenerate the SDK after adding or modifying flags. Reads control-path.yaml
    /// and generates the SDK (default: node_modules/@controlpath/generated) for your application
    /// code to import. Usually called automatically by setup, new-flag, dev, and ci.
    ///
    /// When to use:
    ///   - After adding new flags to update TypeScript types
    ///   - When SDK generation fails during automated workflows
    ///   - To regenerate SDK with different language or output path
    GenerateSdk {
        /// Language (typescript, python, etc.)
        #[arg(long)]
        lang: Option<String>,
        /// Output directory
        #[arg(long)]
        output: Option<String>,
    },
    /// Watch for file changes and auto-compile/regenerate
    ///
    /// Monitors `control-path.yaml` for changes and automatically regenerates the
    /// SDK and/or recompiles AST artifacts.
    ///
    /// Examples:
    ///   # Watch catalog (SDK + AST recompile)
    ///   controlpath watch --lang typescript
    ///
    ///   # Regenerate SDK only on catalog change
    ///   controlpath watch --definitions --lang typescript
    ///
    ///   # Recompile ASTs only on catalog change
    ///   controlpath watch --deployments
    Watch {
        /// Language for SDK generation (default: typescript)
        #[arg(long)]
        lang: Option<String>,
        /// Regenerate SDK on catalog change (skip AST recompile unless combined with default)
        #[arg(long)]
        definitions: bool,
        /// Recompile ASTs on catalog change (skip SDK regeneration unless combined with default)
        #[arg(long)]
        deployments: bool,
    },
    /// Development workflow with smart defaults
    ///
    /// Use this during development for automatic compilation and SDK regeneration. Watches flag
    /// catalog and environment rules for changes and automatically regenerates SDKs or
    /// recompiles ASTs. Uses config/cached language and smart defaults for environments
    /// (git branch mapping, defaultEnv).
    ///
    /// When to use:
    ///   - During active development when frequently changing flags
    ///   - When you want automatic compilation without manual steps
    ///   - For a streamlined development experience
    ///
    /// Examples:
    ///   # Start dev mode (uses config/cached language)
    ///   controlpath dev
    ///
    ///   # Override language
    ///   controlpath dev --lang python
    Dev {
        /// Language override (if not provided, uses config/cached language)
        #[arg(long)]
        lang: Option<String>,
    },
    /// CI pipeline workflow
    ///
    /// Use this in CI/CD pipelines to ensure flags are valid and ready for deployment.
    /// Validates catalog and environments, compiles ASTs, and optionally
    /// regenerates the SDK. Designed to catch issues before deployment.
    ///
    /// When to use:
    ///   - In CI/CD pipelines (GitHub Actions, GitLab CI, etc.)
    ///   - Pre-commit hooks to validate changes
    ///   - Automated testing of flag configurations
    ///
    /// Examples:
    ///   # Run all CI checks (validate, compile, regenerate SDK)
    ///   controlpath ci
    ///
    ///   # Run CI checks for specific environments
    ///   controlpath ci --env production --env staging
    ///
    ///   # Skip SDK regeneration
    ///   controlpath ci --no-sdk
    ///
    ///   # Skip validation (faster, but less safe)
    ///   controlpath ci --no-validate
    Ci {
        /// Environment names to validate/compile (if not provided, processes all)
        ///
        /// Repeat `--env` for multiple values, or pass comma-separated values.
        /// Examples: `--env production --env staging` or `--env=production,staging`
        #[arg(long)]
        env: Vec<String>,
        /// Skip SDK regeneration
        #[arg(long)]
        no_sdk: bool,
        /// Skip validation
        #[arg(long)]
        no_validate: bool,
    },
    /// Explain flag evaluation with user/context
    ///
    /// Use this to debug why a flag evaluates to a specific value for a user. Shows detailed
    /// information about how a flag evaluates for a given user and context, including which
    /// rules matched and why. Essential for troubleshooting flag behavior.
    ///
    /// When to use:
    ///   - Debugging why a flag isn't working as expected
    ///   - Understanding which rules matched for a specific user
    ///   - Testing flag logic before deploying
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
        /// If not provided, auto-detection will be attempted only when exactly one AST exists.
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
    /// Use this for visual debugging of flag evaluation. Launches a web-based UI for debugging
    /// flag evaluation. The UI allows you to test flags with different user and context values,
    /// see which rules match, and view detailed evaluation information.
    ///
    /// The debug UI is available at http://localhost:8080 by default.
    ///
    /// When to use:
    ///   - Visual debugging of complex flag rules
    ///   - Testing multiple user scenarios quickly
    ///   - Exploring flag behavior interactively
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
        /// If not provided, auto-detection will be attempted only when exactly one AST exists.
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
    /// Commands for managing catalog flags and environments.
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
    /// Complete workflow for adding a new flag
    ///
    /// Adds a flag to control-path.yaml and optionally enables it in environments
    /// and regenerates the SDK. Optionally enables and deploys in one step.
    NewFlag {
        /// Flag name (optional, prompts if not provided)
        #[arg(value_name = "NAME")]
        name: Option<String>,
        /// Flag type (boolean only in v2 catalogs)
        #[arg(long)]
        r#type: Option<String>,
        /// Default value
        #[arg(long)]
        default: Option<String>,
        /// Description
        #[arg(long)]
        description: Option<String>,
        /// Enable flag in specific environment(s)
        ///
        /// Repeat `--enable-in` for multiple values, or pass comma-separated values.
        /// Examples: `--enable-in production --enable-in staging` or `--enable-in=production,staging`
        #[arg(long = "enable-in", value_delimiter = ',', num_args = 1..)]
        enable_in: Vec<String>,
        /// Don't regenerate SDK
        #[arg(long)]
        skip_sdk: bool,
        /// Continue even if follow-up compile/SDK steps fail
        #[arg(long)]
        best_effort: bool,
    },
    /// Enable a flag in one or more environments
    ///
    /// Use this when you want to activate a flag for users. Updates control-path.yaml with
    /// rollout rules for specified environments and automatically compiles ASTs for the
    /// affected environments. Works with smart defaults (detects current git branch).
    ///
    /// When to use:
    ///   - Activating a feature flag for the first time
    ///   - Rolling out a flag to more users or environments
    ///   - Setting up targeted rollouts with rules
    ///
    /// Examples:
    ///   # Enable for all users in staging
    ///   controlpath enable my_flag --env staging --all
    ///
    ///   # Enable with a rule (interactive mode)
    ///   controlpath enable my_flag --env staging --interactive
    /// Validate, compile, and prepare flags for deployment
    ///
    /// Validates control-path.yaml, then compiles ASTs for specified environments.
    Deploy {
        /// Environment(s) to deploy (defaults to all).
        ///
        /// Repeat `--env` for multiple values, or pass comma-separated values.
        /// Examples: `--env production --env staging` or `--env=production,staging`
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        env: Vec<String>,
        /// Validate and compile but show what would happen
        #[arg(long)]
        dry_run: bool,
        /// Skip validation step
        #[arg(long)]
        skip_validation: bool,
    },
    /// Manage kill switch files
    ///
    /// Commands for managing runtime kill switches without redeploying code.
    /// Kill switch files are written to `.controlpath/<env>.kill-switches.json`.
    ///
    /// Examples:
    ///   # Set a kill switch
    ///   controlpath kill-switch set new_dashboard true --env production
    ///
    ///   # Clear a kill switch
    ///   controlpath kill-switch clear new_dashboard --env production
    ///
    ///   # List all kill switches
    ///   controlpath kill-switch list --env production
    #[command(name = "kill-switch", alias = "override")]
    Override {
        #[command(subcommand)]
        subcommand: OverrideSubcommand,
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
    /// Add a new boolean flag to the catalog
    ///
    /// Adds a flag to control-path.yaml. Runs in interactive mode by default,
    /// prompting for missing values.
    ///
    /// Examples:
    ///   # Interactive mode (prompts for values)
    ///   controlpath flag add
    ///
    ///   # Add with all options
    ///   controlpath flag add --name my_feature --type boolean --default false --description "My feature flag"
    ///
    ///   # Add and sync environment rules
    ///   controlpath flag add --name my_feature --sync
    Add {
        /// Flag name (required, snake_case format)
        #[arg(long)]
        name: Option<String>,
        /// Flag type (boolean only)
        #[arg(long)]
        r#type: Option<String>,
        /// Default value (true or false)
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
        /// Sync environment rules for the new flag (default: prompts)
        #[arg(long)]
        sync: bool,
        /// Disable interactive mode
        ///
        /// When set, disables interactive prompts. All required values must be
        /// provided via command-line flags.
        #[arg(long)]
        no_interactive: bool,
    },
    /// List flags from the catalog or a specific environment
    ///
    /// Examples:
    ///   # List all flags (default)
    ///   controlpath flag list
    ///
    ///   # List rules for a specific environment
    ///   controlpath flag list --deployment production
    List {
        /// List from control-path.yaml catalog (default)
        #[arg(long)]
        definitions: bool,
        /// List rollout rules for an environment
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
    /// Examples:
    ///   controlpath flag show --name my_feature
    ///   controlpath flag show --name my_feature --deployment production
    Show {
        #[arg(long)]
        name: String,
        /// Show rollout rules for an environment
        #[arg(long)]
        deployment: Option<String>,
        /// Output format (table, json, yaml)
        ///
        /// The output format. Defaults to 'table' for TTY output, 'json' for piped output.
        #[arg(long)]
        format: Option<String>,
    },
    /// Remove a flag from the catalog and all environments
    ///
    /// Removes a flag from control-path.yaml.
    /// Examples:
    ///   controlpath flag remove --name my_feature
    ///   controlpath flag remove --name my_feature --env staging
    Remove {
        /// Flag name
        ///
        /// The name of the flag to remove.
        #[arg(long)]
        name: String,
        /// Remove environment rules for this flag in a specific environment only
        #[arg(long)]
        env: Option<String>,
    },
    /// Enable a flag in one or more environments
    Enable {
        /// Flag name (required)
        #[arg(value_name = "NAME")]
        name: String,
        /// Environment(s) to update.
        ///
        /// Repeat `--env` for multiple values, or pass comma-separated values.
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        env: Vec<String>,
        /// Rule expression (e.g., "user.role == 'admin'")
        #[arg(long)]
        rule: Option<String>,
        /// Enable for all users (no rule, just serve default)
        #[arg(long)]
        all: bool,
        /// Value to serve (true/false for boolean flags)
        #[arg(long)]
        value: Option<String>,
        /// Interactive rule builder
        #[arg(long)]
        interactive: bool,
        /// Skip automatic compilation of ASTs after updating deployments
        #[arg(long)]
        no_compile: bool,
        /// Continue even if follow-up compile step fails
        #[arg(long)]
        best_effort: bool,
        /// Allow rule changes on deprecated flags
        #[arg(long)]
        force: bool,
    },
    /// Mark a flag as deprecated (blocks new rule changes unless forced)
    Deprecate {
        #[arg(long)]
        name: String,
    },
    /// Report flag lifecycle and rot signals (SaaS telemetry is read-only)
    Report,
}

#[derive(Subcommand)]
enum OverrideSubcommand {
    /// Set a kill switch for a flag
    ///
    /// Sets a runtime boolean kill switch. Stored in `.controlpath/<env>.kill-switches.json`.
    ///
    /// Examples:
    ///   controlpath kill-switch set new_dashboard false --env production
    ///   controlpath kill-switch set new_dashboard true --env production --reason "Emergency rollback"
    Set {
        #[arg(value_name = "FLAG")]
        flag: String,
        /// Boolean value (true/false, or ON/OFF)
        #[arg(value_name = "VALUE")]
        value: String,
        /// Reason (not persisted in kill switch files; shown for compatibility)
        #[arg(long)]
        reason: Option<String>,
        /// Operator (not persisted; shown for compatibility)
        #[arg(long)]
        operator: Option<String>,
        /// Deprecated: ignored; kill switches use `.controlpath/<env>.kill-switches.json`
        #[arg(long, hide = true)]
        file: Option<String>,
        /// Deprecated: ignored
        #[arg(long)]
        definitions: Option<String>,
        /// Environment (default: defaultEnv or first environment)
        #[arg(long)]
        env: Option<String>,
    },
    /// Clear a kill switch for a flag
    Clear {
        #[arg(value_name = "FLAG")]
        flag: String,
        /// Deprecated: ignored
        #[arg(long, hide = true)]
        file: Option<String>,
        #[arg(long)]
        env: Option<String>,
    },
    /// List kill switches for an environment
    List {
        /// Deprecated: ignored
        #[arg(long, hide = true)]
        file: Option<String>,
        #[arg(long)]
        env: Option<String>,
    },
    /// Show current kill switch state (alias for list; no audit history is stored)
    History {
        #[arg(value_name = "FLAG")]
        flag: Option<String>,
        /// Deprecated: ignored
        #[arg(long, hide = true)]
        file: Option<String>,
        #[arg(long)]
        env: Option<String>,
    },
}

#[derive(Subcommand)]
enum EnvSubcommand {
    /// Add a new environment
    ///
    /// Adds a new environment (flags can be enabled in this environment via control-path.yaml)
    /// file. Can optionally copy flags from a template environment.
    ///
    /// Examples:
    ///   # Add new environment (interactive)
    ///   controlpath env add
    ///
    ///   # Add with name
    ///   controlpath env add --name staging
    ///
    Add {
        /// Environment name
        ///
        /// The name of the environment to create. If not provided, will prompt
        /// in interactive mode.
        #[arg(long)]
        name: Option<String>,
        /// Interactive mode (prompts for missing values)
        ///
        /// When set, prompts for missing values. This is the default behavior
        /// when name is not provided.
        #[arg(long)]
        interactive: bool,
    },
    /// Sync catalog flags across environments in control-path.yaml
    ///
    /// Validates that each environment's rules reference defined flags. In v2 catalogs,
    /// environment rules live in control-path.yaml (not separate deployment files).
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
        /// all environments defined in control-path.yaml.
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
    /// List all environments defined in control-path.yaml
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
    /// Removes an environment (removes all rules for that environment from control-path.yaml)
    /// Examples:
    ///   controlpath env remove --name staging
    Remove {
        /// Environment name
        ///
        /// The name of the environment to remove.
        #[arg(long)]
        name: String,
    },
}

/// Get the CLI command structure for completion generation
pub fn get_cli_command() -> clap::Command {
    Cli::command()
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = init_runtime_options(RuntimeOptions {
        json_output: cli.json,
        non_interactive: cli.non_interactive,
        verbose: cli.verbose,
        quiet: cli.quiet,
    }) {
        eprintln!("✗ Failed to initialize runtime options");
        eprintln!("  Error: {e}");
        std::process::exit(1);
    }

    let exit_code = match cli.command {
        Commands::Validate { env, all } => {
            let opts = validate::Options { env, all };
            validate::run(&opts)
        }
        Commands::Compile { env, output } => {
            let opts = compile::Options { env, output };
            compile::run(&opts)
        }
        Commands::Setup {
            lang,
            skip_install,
            no_examples,
        } => {
            let opts = setup::Options {
                lang: lang.clone(),
                skip_install,
                no_examples,
            };
            setup::run(&opts)
        }
        Commands::Init {
            monorepo,
            no_monorepo,
            namespace,
            service_id,
        } => {
            let monorepo_choice = if monorepo {
                Some(true)
            } else if no_monorepo {
                Some(false)
            } else {
                None
            };
            init::run(&init::Options {
                monorepo: monorepo_choice,
                namespace,
                service_id,
            })
        }
        Commands::GenerateSdk { lang, output } => {
            let opts = generate_sdk::Options { lang, output };
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
        Commands::Dev { lang } => {
            let opts = dev::Options { lang };
            dev::run(&opts)
        }
        Commands::Ci {
            env,
            no_sdk,
            no_validate,
        } => {
            let envs = if env.is_empty() { None } else { Some(env) };
            let opts = ci::Options {
                envs,
                no_sdk,
                no_validate,
            };
            ci::run(&opts)
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
        Commands::Flag { subcommand } => match subcommand {
            FlagSubcommand::Enable {
                name,
                env,
                rule,
                all,
                value,
                interactive,
                no_compile,
                best_effort,
                force,
            } => {
                let env = if env.is_empty() {
                    None
                } else {
                    Some(env.join(","))
                };
                let opts = workflow::EnableOptions {
                    name,
                    env,
                    rule,
                    all,
                    value,
                    interactive,
                    no_compile,
                    best_effort,
                    force,
                };
                workflow::run_enable(&opts)
            }
            _ => {
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
                        interactive: !no_interactive && !cli.non_interactive,
                    },
                    FlagSubcommand::List {
                        definitions,
                        deployment,
                        format,
                    } => {
                        let format_str =
                            if cli.json || (format == "table" && !atty::is(atty::Stream::Stdout)) {
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
                            .or(if cli.json {
                                Some(flag::OutputFormat::Json)
                            } else {
                                None
                            })
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
                    FlagSubcommand::Remove { name, env } => {
                        flag::FlagSubcommand::Remove { name, env }
                    }
                    FlagSubcommand::Deprecate { name } => flag::FlagSubcommand::Deprecate { name },
                    FlagSubcommand::Report => flag::FlagSubcommand::Report,
                    FlagSubcommand::Enable { .. } => unreachable!(),
                };

                let opts = flag::Options {
                    subcommand: flag_subcommand,
                };
                flag::run(&opts)
            }
        },
        Commands::Env { subcommand } => {
            let env_subcommand = match subcommand {
                EnvSubcommand::Add { name, interactive } => env::EnvSubcommand::Add {
                    name: name.clone(),
                    interactive: (interactive || name.is_none()) && !cli.non_interactive,
                },
                EnvSubcommand::Sync { env, dry_run } => env::EnvSubcommand::Sync {
                    env: env.clone(),
                    dry_run,
                },
                EnvSubcommand::List { format } => {
                    let format_str =
                        if cli.json || (format == "table" && !atty::is(atty::Stream::Stdout)) {
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
                EnvSubcommand::Remove { name } => env::EnvSubcommand::Remove { name: name.clone() },
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
            skip_sdk,
            best_effort,
        } => {
            let enable_in = if enable_in.is_empty() {
                None
            } else {
                Some(enable_in.join(","))
            };
            let opts = workflow::NewFlagOptions {
                name,
                flag_type: r#type,
                default,
                description,
                enable_in,
                skip_sdk,
                best_effort,
            };
            workflow::run_new_flag(&opts)
        }
        Commands::Deploy {
            env,
            dry_run,
            skip_validation,
        } => {
            let env = if env.is_empty() {
                None
            } else {
                Some(env.join(","))
            };
            let opts = workflow::DeployOptions {
                env,
                dry_run,
                skip_validation,
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
                    env,
                } => override_cmd::OverrideSubcommand::Set {
                    flag,
                    value,
                    reason,
                    operator,
                    file: file.map(PathBuf::from),
                    definitions: definitions.map(PathBuf::from),
                    env,
                },
                OverrideSubcommand::Clear { flag, file, env } => {
                    override_cmd::OverrideSubcommand::Clear {
                        flag,
                        file: file.map(PathBuf::from),
                        env,
                    }
                }
                OverrideSubcommand::List { file, env } => override_cmd::OverrideSubcommand::List {
                    file: file.map(PathBuf::from),
                    env,
                },
                OverrideSubcommand::History { flag, file, env } => {
                    override_cmd::OverrideSubcommand::History {
                        flag,
                        file: file.map(PathBuf::from),
                        env,
                    }
                }
            };

            let opts = override_cmd::Options {
                subcommand: override_subcommand,
            };
            override_cmd::run(&opts)
        }
        Commands::Completion { shell } => {
            let opts = completion::Options { shell };
            completion::run(&opts)
        }
    };

    std::process::exit(exit_code);
}
