//! SaaS-mode catalog sync and remote AST download boundary.

pub mod ast;
pub mod ast_cache;
pub mod client;
pub mod fake;
pub mod report;
pub mod sync;

pub use ast::remote_ast_options_from_catalog;
pub use ast_cache::FilesystemAstCache;
pub use fake::FakeSaasClient;
pub use report::{
    build_flag_rot_report, fetch_saas_telemetry, print_flag_rot_report, warn_on_rot_findings,
};
pub use sync::{
    load_saas_catalog_for_ci, parse_saas_catalog_document, sync_saas_catalog_with_catalog,
};

// Public API for callers that load + sync in one step (used by unit tests).
#[allow(unused_imports)]
pub use sync::{sync_saas_catalog, SaasSyncOutcome};
