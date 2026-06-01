//! Flag lifecycle and rot reporting (declared Git metadata + read-only SaaS telemetry).

use std::collections::BTreeMap;

use controlpath_compiler::{FlagLifecycle, SdkCatalog, SdkFlag};

use crate::error::{CliError, CliResult};
use crate::saas::client::{FetchFlagTelemetryRequest, FlagTelemetry, SaasClient};
use crate::utils::runtime;
use controlpath_compiler::EffectiveCatalogId;

/// One row in a flag rot report.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct FlagRotReportEntry {
    pub flag_key: String,
    pub lifecycle: String,
    pub last_evaluated: Option<String>,
    pub evaluation_count: Option<u64>,
    pub rot_suggestion: Option<String>,
    pub imported: bool,
}

/// Build a rot report from the merged SDK catalog and optional SaaS telemetry.
///
/// Uses [`SdkCatalog`] so imported flags (issue 07) appear with qualified names
/// such as `platform.emergency_kill_switch`.
pub fn build_flag_rot_report(
    sdk_catalog: &SdkCatalog,
    telemetry: &[FlagTelemetry],
) -> Vec<FlagRotReportEntry> {
    let telemetry_by_key: BTreeMap<&str, &FlagTelemetry> =
        telemetry.iter().map(|t| (t.flag_key.as_str(), t)).collect();

    let mut entries: Vec<FlagRotReportEntry> = sdk_catalog
        .flags
        .iter()
        .map(|flag| entry_for_sdk_flag(flag, &telemetry_by_key))
        .collect();

    entries.sort_by(|a, b| a.flag_key.cmp(&b.flag_key));
    entries
}

fn entry_for_sdk_flag(
    flag: &SdkFlag,
    telemetry_by_key: &BTreeMap<&str, &FlagTelemetry>,
) -> FlagRotReportEntry {
    let tel = telemetry_for_flag(telemetry_by_key, flag);
    FlagRotReportEntry {
        flag_key: flag.qualified_name.clone(),
        lifecycle: lifecycle_label(flag.lifecycle).to_string(),
        last_evaluated: tel.and_then(|t| t.last_evaluated.clone()),
        evaluation_count: tel.map(|t| t.evaluation_count),
        rot_suggestion: tel.and_then(|t| t.rot_suggestion.clone()),
        imported: flag.is_imported,
    }
}

fn lifecycle_label(lifecycle: FlagLifecycle) -> &'static str {
    match lifecycle {
        FlagLifecycle::Active => "active",
        FlagLifecycle::Deprecated => "deprecated",
    }
}

fn telemetry_for_flag<'a>(
    by_key: &BTreeMap<&str, &'a FlagTelemetry>,
    flag: &SdkFlag,
) -> Option<&'a FlagTelemetry> {
    by_key
        .get(flag.qualified_name.as_str())
        .or_else(|| {
            flag.qualified_name
                .rsplit('.')
                .next()
                .and_then(|short| by_key.get(short))
        })
        .copied()
}

/// Fetch SaaS telemetry for a project (read-only).
pub fn fetch_saas_telemetry(
    client: &dyn SaasClient,
    catalog_id: &EffectiveCatalogId,
    project: &str,
) -> CliResult<Vec<FlagTelemetry>> {
    client.fetch_flag_telemetry(&FetchFlagTelemetryRequest {
        catalog_id: catalog_id.clone(),
        project: project.to_string(),
    })
}

/// Print a rot report as a table or JSON (`--json` / piped stdout).
pub fn print_flag_rot_report(entries: &[FlagRotReportEntry]) -> CliResult<()> {
    if runtime::is_json_output() {
        let json = serde_json::json!({
            "command": "flag report",
            "flags": entries,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&json)
                .map_err(|e| CliError::Message(format!("Failed to serialize report: {e}")))?
        );
        return Ok(());
    }

    if entries.is_empty() {
        println!("No flags in catalog.");
        return Ok(());
    }

    println!("Flag rot report:");
    println!("{:-<96}", "");
    println!(
        "{:<32} {:<12} {:<16} {:<12} Rot suggestion",
        "Flag", "Lifecycle", "Last evaluated", "Evaluations"
    );
    println!("{:-<96}", "");

    for entry in entries {
        let last = entry.last_evaluated.as_deref().unwrap_or("-");
        let evals = entry
            .evaluation_count
            .map(|c| c.to_string())
            .unwrap_or_else(|| "-".to_string());
        let rot = entry.rot_suggestion.as_deref().unwrap_or("-");
        println!(
            "{:<32} {:<12} {:<16} {:<12} {rot}",
            entry.flag_key, entry.lifecycle, last, evals
        );
    }
    Ok(())
}

/// Warning messages for deprecated lifecycle and SaaS rot suggestions.
pub fn rot_warning_messages(entries: &[FlagRotReportEntry]) -> Vec<String> {
    let mut messages = Vec::new();
    for entry in entries {
        if entry.lifecycle == "deprecated" {
            messages.push(format!(
                "Flag '{}' is deprecated in the catalog.",
                entry.flag_key
            ));
        }
        if let Some(suggestion) = &entry.rot_suggestion {
            messages.push(format!(
                "Flag '{}' — SaaS rot suggestion: {suggestion}",
                entry.flag_key
            ));
        }
    }
    messages
}

/// Emit stderr warnings for flags with rot suggestions or deprecated lifecycle.
pub fn warn_on_rot_findings(entries: &[FlagRotReportEntry]) {
    for message in rot_warning_messages(entries) {
        eprintln!("⚠ Warning: {message}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use controlpath_compiler::catalog::FlagKind;

    fn sdk_flag(qualified_name: &str, lifecycle: FlagLifecycle, imported: bool) -> SdkFlag {
        SdkFlag {
            qualified_name: qualified_name.to_string(),
            sdk_method_name: qualified_name.to_string(),
            default: false,
            kind: FlagKind::Release,
            lifecycle,
            description: None,
            is_imported: imported,
        }
    }

    #[test]
    fn merges_telemetry_into_declared_catalog_rows() {
        let sdk_catalog = SdkCatalog {
            flags: vec![sdk_flag("stale_feature", FlagLifecycle::Active, false)],
            kill_switch_urls: BTreeMap::new(),
            artifact_urls: BTreeMap::new(),
        };
        let telemetry = vec![FlagTelemetry {
            flag_key: "stale_feature".to_string(),
            last_evaluated: Some("2026-01-15".to_string()),
            evaluation_count: 0,
            rot_suggestion: Some("unused".to_string()),
        }];

        let report = build_flag_rot_report(&sdk_catalog, &telemetry);
        assert_eq!(report.len(), 1);
        assert_eq!(report[0].rot_suggestion.as_deref(), Some("unused"));
        assert_eq!(report[0].evaluation_count, Some(0));
    }

    #[test]
    fn includes_imported_flags_with_qualified_names() {
        let sdk_catalog = SdkCatalog {
            flags: vec![
                sdk_flag("new_dashboard", FlagLifecycle::Active, false),
                sdk_flag(
                    "platform.emergency_kill_switch",
                    FlagLifecycle::Active,
                    true,
                ),
            ],
            kill_switch_urls: BTreeMap::new(),
            artifact_urls: BTreeMap::new(),
        };
        let telemetry = vec![FlagTelemetry {
            flag_key: "platform.emergency_kill_switch".to_string(),
            last_evaluated: Some("2026-05-01".to_string()),
            evaluation_count: 42,
            rot_suggestion: Some("review".to_string()),
        }];

        let report = build_flag_rot_report(&sdk_catalog, &telemetry);
        assert_eq!(report.len(), 2);
        let imported = report
            .iter()
            .find(|e| e.flag_key == "platform.emergency_kill_switch")
            .unwrap();
        assert!(imported.imported);
        assert_eq!(imported.rot_suggestion.as_deref(), Some("review"));
    }

    #[test]
    fn rot_warning_messages_cover_deprecated_and_rot_suggestions() {
        let entries = vec![
            FlagRotReportEntry {
                flag_key: "old_flow".to_string(),
                lifecycle: "deprecated".to_string(),
                last_evaluated: None,
                evaluation_count: None,
                rot_suggestion: None,
                imported: false,
            },
            FlagRotReportEntry {
                flag_key: "stale".to_string(),
                lifecycle: "active".to_string(),
                last_evaluated: Some("2026-01-01".to_string()),
                evaluation_count: Some(0),
                rot_suggestion: Some("unused".to_string()),
                imported: false,
            },
        ];

        let messages = rot_warning_messages(&entries);
        assert_eq!(messages.len(), 2);
        assert!(messages[0].contains("old_flow") && messages[0].contains("deprecated"));
        assert!(messages[1].contains("stale") && messages[1].contains("unused"));
    }
}
