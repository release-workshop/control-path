/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 *
 * v2 boolean catalog parsing, validation, and typed models.
 */

mod cdn;
mod compile;
mod explain;
mod saas_environment;
mod sdk;

pub mod model;
pub mod namespace;
pub mod parse;
pub mod validate;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod validation_mode_tests;

pub use cdn::{
    build_saas_runtime_url_maps, build_saas_runtime_urls, saas_cdn_base_url, SaasRuntimeUrlMaps,
    SaasRuntimeUrls, DEFAULT_SAAS_CDN_BASE,
};
pub use compile::{
    compile_catalog, compile_catalog_with_imports, load_validate_and_compile_catalog,
    validate_and_compile_catalog,
};
pub use explain::{
    explain_flag, is_compiled_catalog_default, ExplainError, ExplainLayer, ExplainRequest,
    ExplainRuleTrace, ExplainTrace, KillSwitchOverrides,
};
pub use model::*;
pub use namespace::{effective_catalog_id, resolve_namespace};
pub use parse::{parse_catalog, parse_catalog_value, parse_workspace, parse_workspace_value};
pub use saas_environment::{environment_from_ast_path, is_valid_saas_environment_name};
pub use sdk::{build_sdk_catalog, SdkCatalog, SdkFlag};
pub use validate::{
    imported_flag_keys_from_imports, load_and_validate_catalog, load_and_validate_workspace,
    validate_catalog, validate_catalog_value, validate_workspace_value, CatalogValidationContext,
    CatalogValidationResult, ValidationMode,
};
