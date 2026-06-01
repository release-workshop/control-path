//! Narrow library surface for integration tests and programmatic hooks.

mod error;

#[path = "saas/ast_cache.rs"]
pub mod ast_cache;

pub use ast_cache::discover_environments_in_dir;
