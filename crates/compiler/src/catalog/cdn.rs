/*!
 * Platform CDN path contract for SaaS-mode runtime URLs.
 *
 * At `generate-sdk`, artifact and kill switch URLs are embedded per environment
 * that has a `.controlpath/<env>.ast` file present. SaaS sync (`write_remote_asts`)
 * prunes `*.ast` for environments no longer returned by the platform; manually copied
 * or leftover files are still embedded until deleted or the next sync runs.
 *
 * Path shape (each segment percent-encoded when needed):
 * `{cdn_base}/v2/runtime/projects/{saas.project}/catalogs/{effective_catalog_id}/environments/{env}/rules.ast`
 * `{cdn_base}/v2/runtime/projects/{saas.project}/catalogs/{effective_catalog_id}/environments/{env}/kill-switches.json`
 *
 * `cdn_base` is `saas.cdn_url` when set (self-hosted CDN), otherwise [`DEFAULT_SAAS_CDN_BASE`].
 * `saas.api_url` is for catalog sync only and must not be used here.
 */

use std::collections::BTreeMap;

use crate::catalog::model::EffectiveCatalogId;

/// Default platform CDN origin when `saas.cdn_url` is omitted.
pub const DEFAULT_SAAS_CDN_BASE: &str = "https://cdn.controlpath.dev";

/// Runtime poll endpoints for one SaaS environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaasRuntimeUrls {
    pub artifact_url: String,
    pub kill_switch_url: String,
}

/// Resolve the CDN origin from optional `saas.cdn_url` (trailing slash stripped).
#[must_use]
pub fn saas_cdn_base_url(cdn_url: Option<&str>) -> &str {
    match cdn_url {
        Some(url) if !url.trim().is_empty() => url.trim().trim_end_matches('/'),
        _ => DEFAULT_SAAS_CDN_BASE,
    }
}

/// Build artifact and kill switch CDN URLs for a SaaS project environment.
#[must_use]
pub fn build_saas_runtime_urls(
    cdn_base: &str,
    project: &str,
    catalog_id: &EffectiveCatalogId,
    environment: &str,
) -> SaasRuntimeUrls {
    let base = cdn_base.trim().trim_end_matches('/');
    let project_path = encode_project_path(project);
    let catalog_segment = encode_path_segment(&catalog_id.as_str());
    let env_segment = encode_path_segment(environment);
    let prefix = format!(
        "{base}/v2/runtime/projects/{project_path}/catalogs/{catalog_segment}/environments/{env_segment}"
    );
    SaasRuntimeUrls {
        artifact_url: format!("{prefix}/rules.ast"),
        kill_switch_url: format!("{prefix}/kill-switches.json"),
    }
}

/// Per-environment artifact and kill switch CDN URLs for SaaS SDK embedding.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SaasRuntimeUrlMaps {
    pub artifact_urls: BTreeMap<String, String>,
    pub kill_switch_urls: BTreeMap<String, String>,
}

/// Build artifact and kill switch CDN URLs for each SaaS environment name.
///
/// Callers must pass a non-empty `environments` slice (typically from disk discovery
/// after SaaS sync). An empty slice yields empty maps with no error.
#[must_use]
pub fn build_saas_runtime_url_maps(
    cdn_base: &str,
    project: &str,
    catalog_id: &EffectiveCatalogId,
    environments: &[impl AsRef<str>],
) -> SaasRuntimeUrlMaps {
    let mut artifact_urls = BTreeMap::new();
    let mut kill_switch_urls = BTreeMap::new();

    for environment in environments {
        let env = environment.as_ref();
        let urls = build_saas_runtime_urls(cdn_base, project, catalog_id, env);
        artifact_urls.insert(env.to_string(), urls.artifact_url);
        kill_switch_urls.insert(env.to_string(), urls.kill_switch_url);
    }

    SaasRuntimeUrlMaps {
        artifact_urls,
        kill_switch_urls,
    }
}

fn encode_project_path(project: &str) -> String {
    project
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(encode_path_segment)
        .collect::<Vec<_>>()
        .join("/")
}

fn encode_path_segment(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    for byte in segment.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => {
                out.push(byte as char);
            }
            _ => {
                use std::fmt::Write as _;
                let _ = write!(out, "%{byte:02X}");
            }
        }
    }
    out
}

#[cfg(test)]
#[path = "cdn_tests.rs"]
mod tests;
