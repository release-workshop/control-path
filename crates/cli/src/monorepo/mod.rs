//! Monorepo support for Control Path CLI
//!
//! This module provides functionality for detecting monorepo structures,
//! resolving service contexts, and managing service-scoped file paths.

mod detection;
mod paths;
mod service;

// Public API
// Note: Some exports are for future use (e.g., discover_services for bulk operations in section 1.2.2)
#[allow(unused_imports)] // These will be used when more commands are integrated
pub use detection::{detect_workspace_root, discover_services, is_monorepo};
#[allow(unused_imports)] // resolve_service_path will be used when more commands are integrated
pub use paths::{resolve_service_path, ServicePathResolver};
#[allow(unused_imports)] // Service will be used in services list/status commands
pub use service::{resolve_service_context, Service, ServiceContext};
