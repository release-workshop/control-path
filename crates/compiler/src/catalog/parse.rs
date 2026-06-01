/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

use crate::catalog::model::{CatalogDocument, WorkspaceDocument};
use crate::parser::error::ParseError;
use crate::parser::utils::parse_yaml_or_json;
use serde::de::DeserializeOwned;
use serde_json::Value;

/// Parse a v2 `control-path.yaml` from YAML/JSON into a typed document.
///
/// **Do not use on untrusted input without prior validation:** serde drops unknown flag
/// fields (e.g. v1 `type`, `defaultValue`) during deserialization, so a later
/// [`super::validate_catalog`] will not see them. Prefer [`load_and_validate_catalog`] or
/// [`parse_catalog_value`] + [`super::validate_catalog_value`] on the raw `Value`.
pub fn parse_catalog(
    content: &str,
    file_path: Option<&str>,
) -> Result<CatalogDocument, ParseError> {
    let value = parse_catalog_value(content, file_path)?;
    value_to_document::<CatalogDocument>(&value, "catalog").map_err(ParseError::InvalidFieldType)
}

/// Parse catalog content to `serde_json::Value` (no typing).
///
/// Use with [`super::validate_catalog_value`] before deserializing to a [`CatalogDocument`].
pub fn parse_catalog_value(content: &str, file_path: Option<&str>) -> Result<Value, ParseError> {
    let parsed = parse_yaml_or_json(content, file_path)?;

    if !parsed.is_object() {
        return Err(ParseError::InvalidFieldType(
            "Invalid catalog: expected an object".to_string(),
        ));
    }

    let obj = parsed.as_object().unwrap();
    if !obj.contains_key("catalog") {
        return Err(ParseError::MissingField(
            "Invalid catalog: missing required field \"catalog\"".to_string(),
        ));
    }
    if !obj.contains_key("flags") {
        return Err(ParseError::MissingField(
            "Invalid catalog: missing required field \"flags\"".to_string(),
        ));
    }

    let flags = &obj["flags"];
    if flags.is_array() {
        return Err(ParseError::InvalidFieldType(
            "Invalid catalog: v1 array \"flags\" is not supported; use map-keyed flags".to_string(),
        ));
    }
    if !flags.is_object() {
        return Err(ParseError::InvalidFieldType(
            "Invalid catalog: \"flags\" must be an object".to_string(),
        ));
    }

    Ok(parsed)
}

/// Parse `control-path.workspace.yaml` into a typed document.
pub fn parse_workspace(
    content: &str,
    file_path: Option<&str>,
) -> Result<WorkspaceDocument, ParseError> {
    let value = parse_workspace_value(content, file_path)?;
    value_to_document::<WorkspaceDocument>(&value, "workspace")
        .map_err(ParseError::InvalidFieldType)
}

/// Parse workspace content to `serde_json::Value`.
pub fn parse_workspace_value(content: &str, file_path: Option<&str>) -> Result<Value, ParseError> {
    let parsed = parse_yaml_or_json(content, file_path)?;

    if !parsed.is_object() {
        return Err(ParseError::InvalidFieldType(
            "Invalid workspace: expected an object".to_string(),
        ));
    }

    if !parsed.as_object().unwrap().contains_key("namespace") {
        return Err(ParseError::MissingField(
            "Invalid workspace: missing required field \"namespace\"".to_string(),
        ));
    }

    Ok(parsed)
}

fn value_to_document<T: DeserializeOwned>(value: &Value, label: &str) -> Result<T, String> {
    serde_json::from_value(value.clone()).map_err(|e| format!("Failed to deserialize {label}: {e}"))
}
