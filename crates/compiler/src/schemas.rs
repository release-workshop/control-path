/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */
use serde_json::Value;

/// Embed v2 boolean catalog schema at compile time
const CATALOG_SCHEMA_V2_JSON: &str = include_str!("../../../schemas/control-path.schema.v2.json");

/// Embed workspace schema at compile time
const WORKSPACE_SCHEMA_JSON: &str =
    include_str!("../../../schemas/control-path.workspace.schema.v1.json");

/// Load the v2 boolean catalog schema.
#[must_use]
pub fn load_catalog_schema() -> Value {
    serde_json::from_str(CATALOG_SCHEMA_V2_JSON)
        .expect("Failed to parse embedded catalog v2 schema - this should never happen")
}

/// Load the monorepo workspace schema.
#[must_use]
pub fn load_workspace_schema() -> Value {
    serde_json::from_str(WORKSPACE_SCHEMA_JSON)
        .expect("Failed to parse embedded workspace schema - this should never happen")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_catalog_schema_v2() {
        let schema = load_catalog_schema();
        assert!(schema.is_object());
        assert!(schema["required"]
            .as_array()
            .is_some_and(|r| r.iter().any(|v| v == "catalog")));
    }

    #[test]
    fn test_load_workspace_schema() {
        let schema = load_workspace_schema();
        assert!(schema.is_object());
        assert!(schema["required"]
            .as_array()
            .is_some_and(|r| r.iter().any(|v| v == "namespace")));
    }
}
