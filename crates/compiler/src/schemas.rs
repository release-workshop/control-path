/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */
use serde_json::Value;

/// Embed flag definitions schema at compile time
/// This avoids file I/O which is not available in WASM environments
const DEFINITIONS_SCHEMA_JSON: &str =
    include_str!("../../../schemas/flag-definitions.schema.v1.json");

/// Embed deployment schema at compile time
/// This avoids file I/O which is not available in WASM environments
const DEPLOYMENT_SCHEMA_JSON: &str =
    include_str!("../../../schemas/flag-deployment.schema.v1.json");

/// Load the flag definitions schema
///
/// Returns the parsed JSON schema as a `serde_json::Value`.
/// This function never fails at runtime since the schema is embedded at compile time.
///
/// # Panics
///
/// Panics if the embedded schema JSON is invalid (this should never happen).
#[must_use]
pub fn load_definitions_schema() -> Value {
    serde_json::from_str(DEFINITIONS_SCHEMA_JSON)
        .expect("Failed to parse embedded definitions schema - this should never happen")
}

/// Load the deployment schema
///
/// Returns the parsed JSON schema as a `serde_json::Value`.
/// This function never fails at runtime since the schema is embedded at compile time.
///
/// # Panics
///
/// Panics if the embedded schema JSON is invalid (this should never happen).
#[must_use]
pub fn load_deployment_schema() -> Value {
    serde_json::from_str(DEPLOYMENT_SCHEMA_JSON)
        .expect("Failed to parse embedded deployment schema - this should never happen")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_definitions_schema() {
        let schema = load_definitions_schema();
        assert!(schema.is_object());
        assert_eq!(schema["$schema"], "http://json-schema.org/draft-07/schema#");
    }

    #[test]
    fn test_load_deployment_schema() {
        let schema = load_deployment_schema();
        assert!(schema.is_object());
        assert_eq!(schema["$schema"], "http://json-schema.org/draft-07/schema#");
    }
}
