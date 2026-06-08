//! Resolve SaaS API and CDN endpoints from catalog + workspace walk-up.

use controlpath_compiler::catalog::{CatalogDocument, CatalogScope};
use controlpath_compiler::{saas_cdn_base_url, WorkspaceDocument};

use crate::error::{CliError, CliResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSaasEndpoints {
    pub api_url: Option<String>,
    pub cdn_url: String,
    pub catalog_scope: String,
}

pub fn resolve_saas_endpoints(
    catalog: &CatalogDocument,
    workspace: Option<&WorkspaceDocument>,
) -> CliResult<ResolvedSaasEndpoints> {
    let saas = catalog.saas.as_ref().ok_or_else(|| {
        CliError::Message("SaaS mode requires saas.project in control-path.yaml".to_string())
    })?;

    let workspace_saas = workspace.and_then(|w| w.saas.as_ref());

    let api_url = saas
        .api_url
        .as_deref()
        .or_else(|| workspace_saas.and_then(|config| config.api_url.as_deref()))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    let cdn_url = saas
        .cdn_url
        .as_deref()
        .or_else(|| workspace_saas.and_then(|config| config.cdn_url.as_deref()))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| saas_cdn_base_url(None).to_string());

    let catalog_scope = match catalog.catalog.scope {
        CatalogScope::Org => "org",
        CatalogScope::Service => "service",
    };

    Ok(ResolvedSaasEndpoints {
        api_url,
        cdn_url,
        catalog_scope: catalog_scope.to_string(),
    })
}
