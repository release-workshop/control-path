use super::*;
use crate::catalog::model::EffectiveCatalogId;

fn catalog_id(namespace: Option<&str>, id: &str) -> EffectiveCatalogId {
    EffectiveCatalogId {
        namespace: namespace.map(str::to_string),
        id: id.to_string(),
    }
}

#[test]
fn build_saas_runtime_urls_uses_default_cdn_and_path_shape() {
    let urls = build_saas_runtime_urls(
        saas_cdn_base_url(None),
        "acme/checkout",
        &catalog_id(Some("acme"), "checkout-service"),
        "production",
    );

    assert_eq!(
        urls.artifact_url,
        "https://cdn.controlpath.dev/v2/runtime/projects/acme/checkout/catalogs/acme.checkout-service/environments/production/rules.ast"
    );
    assert_eq!(
        urls.kill_switch_url,
        "https://cdn.controlpath.dev/v2/runtime/projects/acme/checkout/catalogs/acme.checkout-service/environments/production/kill-switches.json"
    );
}

#[test]
fn build_saas_runtime_urls_honors_custom_cdn_base() {
    let urls = build_saas_runtime_urls(
        saas_cdn_base_url(Some("https://cdn.example.com/")),
        "org/app",
        &catalog_id(None, "app"),
        "staging",
    );

    assert!(urls
        .artifact_url
        .starts_with("https://cdn.example.com/v2/runtime/"));
    assert!(urls
        .kill_switch_url
        .ends_with("/staging/kill-switches.json"));
}

#[test]
fn saas_config_deserializes_legacy_url_as_api_url_not_cdn() {
    use crate::catalog::model::SaasConfig;

    let config: SaasConfig =
        serde_json::from_str(r#"{"project":"acme/svc","url":"https://api.example.com"}"#).unwrap();
    assert_eq!(config.api_url.as_deref(), Some("https://api.example.com"));
    assert!(config.cdn_url.is_none());
    assert_eq!(
        saas_cdn_base_url(config.cdn_url.as_deref()),
        DEFAULT_SAAS_CDN_BASE
    );
}

#[test]
fn build_saas_runtime_url_maps_embeds_all_environments() {
    let catalog_id = catalog_id(Some("acme"), "checkout-service");
    let maps = build_saas_runtime_url_maps(
        saas_cdn_base_url(None),
        "acme/checkout",
        &catalog_id,
        &["production", "staging"],
    );

    let production = build_saas_runtime_urls(
        saas_cdn_base_url(None),
        "acme/checkout",
        &catalog_id,
        "production",
    );
    let staging = build_saas_runtime_urls(
        saas_cdn_base_url(None),
        "acme/checkout",
        &catalog_id,
        "staging",
    );

    assert_eq!(
        maps.artifact_urls.get("production"),
        Some(&production.artifact_url)
    );
    assert_eq!(
        maps.kill_switch_urls.get("production"),
        Some(&production.kill_switch_url)
    );
    assert_eq!(
        maps.artifact_urls.get("staging"),
        Some(&staging.artifact_url)
    );
    assert_eq!(
        maps.kill_switch_urls.get("staging"),
        Some(&staging.kill_switch_url)
    );
}

#[test]
fn build_saas_runtime_url_maps_with_empty_environments_returns_empty_maps() {
    let catalog_id = catalog_id(Some("acme"), "checkout-service");
    let maps = build_saas_runtime_url_maps(
        saas_cdn_base_url(None),
        "acme/checkout",
        &catalog_id,
        &[] as &[&str],
    );
    assert!(maps.artifact_urls.is_empty());
    assert!(maps.kill_switch_urls.is_empty());
}

#[test]
fn build_saas_runtime_urls_percent_encodes_special_characters() {
    let urls = build_saas_runtime_urls(
        DEFAULT_SAAS_CDN_BASE,
        "acme/checkout",
        &catalog_id(Some("acme"), "checkout service"),
        "prod west",
    );

    assert!(urls
        .artifact_url
        .contains("/catalogs/acme.checkout%20service/"));
    assert!(urls
        .artifact_url
        .contains("/environments/prod%20west/rules.ast"));
}
