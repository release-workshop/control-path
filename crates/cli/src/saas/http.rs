//! HTTP SaaS client for catalog sync against a deployed Control Path API.

use std::collections::BTreeMap;

use controlpath_compiler::catalog::FlagDefinition;
use controlpath_compiler::EffectiveCatalogId;
use serde::{Deserialize, Serialize};
use ureq::Agent;

use crate::error::{CliError, CliResult};
use crate::saas::client::{
    CatalogSyncPayload, CatalogSyncResult, DownloadCompiledAstsRequest, FetchFlagTelemetryRequest,
    FlagTelemetry, ListActiveFlagsRequest, RemoteAstArtifact, RetireFlagsRequest, SaasClient,
};

#[derive(Debug, Clone)]
pub struct HttpSaasClient {
    agent: Agent,
    base_url: String,
    token: String,
    cdn_url: String,
    catalog_scope: String,
}

impl HttpSaasClient {
    pub fn new(
        base_url: impl Into<String>,
        token: impl Into<String>,
        cdn_url: impl Into<String>,
        catalog_scope: impl Into<String>,
    ) -> Self {
        Self {
            agent: Agent::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            token: token.into(),
            cdn_url: cdn_url.into(),
            catalog_scope: catalog_scope.into(),
        }
    }

    fn encode_path_segment(segment: &str) -> String {
        let mut out = String::with_capacity(segment.len());
        for byte in segment.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
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

    fn project_path(project: &str) -> String {
        project
            .split('/')
            .filter(|segment| !segment.is_empty())
            .map(Self::encode_path_segment)
            .collect::<Vec<_>>()
            .join("/")
    }

    fn catalog_id_query(catalog_id: &EffectiveCatalogId) -> String {
        catalog_id.as_str()
    }

    fn request(&self, method: &str, path: &str) -> ureq::Request {
        self.agent
            .request(method, &format!("{}{path}", self.base_url))
            .set("Authorization", &format!("Bearer {}", self.token))
    }

    fn map_http_error(status: u16, body: &str) -> CliError {
        CliError::Message(format!("SaaS API error ({status}): {body}"))
    }

    fn send_json<T: Serialize>(
        &self,
        method: &str,
        path: &str,
        body: &T,
        include_cdn_url: bool,
    ) -> CliResult<String> {
        let mut request = self
            .request(method, path)
            .set("Content-Type", "application/json");
        if include_cdn_url {
            request = request.set("X-Control-Path-Cdn-Url", &self.cdn_url);
        }

        let response = request
            .send_json(body)
            .map_err(|e| CliError::Message(format!("SaaS API request failed: {e}")))?;

        let status = response.status();
        let response_body = response
            .into_string()
            .map_err(|e| CliError::Message(format!("Failed to read SaaS API response: {e}")))?;

        if (200..300).contains(&status) {
            Ok(response_body)
        } else {
            Err(Self::map_http_error(status, &response_body))
        }
    }

    fn get_json(&self, path: &str) -> CliResult<String> {
        let response = self
            .request("GET", path)
            .call()
            .map_err(|e| CliError::Message(format!("SaaS API request failed: {e}")))?;

        let status = response.status();
        let body = response
            .into_string()
            .map_err(|e| CliError::Message(format!("Failed to read SaaS API response: {e}")))?;

        if (200..300).contains(&status) {
            Ok(body)
        } else {
            Err(Self::map_http_error(status, &body))
        }
    }
}

#[derive(Debug, Deserialize)]
struct ListFlagsResponse {
    flags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SyncCatalogResponse {
    upserted_flags: Vec<String>,
}

#[derive(Debug, Serialize)]
struct HttpCatalogSyncBody<'a> {
    catalog_id: &'a EffectiveCatalogId,
    project: &'a str,
    flags: &'a BTreeMap<String, FlagDefinition>,
    catalog_scope: &'a str,
}

#[derive(Debug, Serialize)]
struct HttpRetireFlagsBody<'a> {
    catalog_id: &'a EffectiveCatalogId,
    project: &'a str,
    flag_keys: &'a [String],
}

#[derive(Debug, Deserialize)]
struct DownloadArtifactsResponse {
    artifacts: Vec<RemoteAstArtifactWire>,
}

#[derive(Debug, Deserialize)]
struct RemoteAstArtifactWire {
    environment: String,
    bytes: String,
}

impl SaasClient for HttpSaasClient {
    fn list_active_flags(&self, request: &ListActiveFlagsRequest) -> CliResult<Vec<String>> {
        let path = format!(
            "/v1/sync/projects/{}/flags?catalog_id={}",
            Self::project_path(&request.project),
            Self::catalog_id_query(&request.catalog_id)
        );
        let body = self.get_json(&path)?;
        let parsed: ListFlagsResponse = serde_json::from_str(&body)
            .map_err(|e| CliError::Message(format!("Invalid SaaS list flags response: {e}")))?;
        Ok(parsed.flags)
    }

    fn sync_catalog(&mut self, payload: &CatalogSyncPayload) -> CliResult<CatalogSyncResult> {
        let path = format!(
            "/v1/sync/projects/{}/catalog",
            Self::project_path(&payload.project)
        );
        let body = HttpCatalogSyncBody {
            catalog_id: &payload.catalog_id,
            project: &payload.project,
            flags: &payload.flags,
            catalog_scope: &self.catalog_scope,
        };
        let response_body = self.send_json("POST", &path, &body, true)?;
        let parsed: SyncCatalogResponse = serde_json::from_str(&response_body)
            .map_err(|e| CliError::Message(format!("Invalid SaaS sync response: {e}")))?;
        Ok(CatalogSyncResult {
            upserted_flags: parsed.upserted_flags,
        })
    }

    fn retire_flags(&mut self, request: &RetireFlagsRequest) -> CliResult<()> {
        let path = format!(
            "/v1/sync/projects/{}/flags/retire",
            Self::project_path(&request.project)
        );
        let body = HttpRetireFlagsBody {
            catalog_id: &request.catalog_id,
            project: &request.project,
            flag_keys: &request.flag_keys,
        };
        self.send_json("POST", &path, &body, false)?;
        Ok(())
    }

    fn download_compiled_asts(
        &self,
        request: &DownloadCompiledAstsRequest,
    ) -> CliResult<Vec<RemoteAstArtifact>> {
        let path = format!(
            "/v1/sync/projects/{}/compiled-asts?catalog_id={}",
            Self::project_path(&request.project),
            Self::catalog_id_query(&request.catalog_id)
        );
        let body = self.get_json(&path)?;
        let parsed: DownloadArtifactsResponse = serde_json::from_str(&body)
            .map_err(|e| CliError::Message(format!("Invalid SaaS compiled AST response: {e}")))?;

        use base64::Engine;
        let mut artifacts = Vec::new();
        for entry in parsed.artifacts {
            if entry.bytes.is_empty() {
                continue;
            }
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(entry.bytes.trim())
                .map_err(|e| {
                    CliError::Message(format!(
                        "Invalid base64 AST bytes for {}: {e}",
                        entry.environment
                    ))
                })?;
            artifacts.push(RemoteAstArtifact {
                environment: entry.environment,
                bytes,
            });
        }

        Ok(artifacts)
    }

    fn fetch_flag_telemetry(
        &self,
        _request: &FetchFlagTelemetryRequest,
    ) -> CliResult<Vec<FlagTelemetry>> {
        Ok(Vec::new())
    }
}
