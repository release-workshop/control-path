/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

use serde_json::Value;

/// Type guard to check if value is a record/object.
pub fn is_record(value: &Value) -> bool {
    value.is_object()
}

/// Type guard to check if value has a name property.
pub fn has_name(value: &Value) -> Option<&str> {
    if let Some(obj) = value.as_object() {
        if let Some(name) = obj.get("name") {
            if let Some(name_str) = name.as_str() {
                return Some(name_str);
            }
        }
    }
    None
}

/// Type guard to check if value is a flag definition object.
pub fn is_flag_definition(value: &Value) -> bool {
    is_record(value)
}

/// Type guard to check if value is a variation object.
pub fn is_variation(value: &Value) -> bool {
    is_record(value)
}

/// Type guard to check if value is a rollout object.
pub fn is_rollout(value: &Value) -> bool {
    is_record(value)
}

/// Type guard to check if value is a flag definitions object with flags array.
pub fn is_flag_definitions(value: &Value) -> bool {
    if let Some(obj) = value.as_object() {
        if let Some(flags) = obj.get("flags") {
            return flags.is_array();
        }
    }
    false
}

/// Type guard to check if value is a deployment object with rules.
pub fn is_deployment(value: &Value) -> bool {
    if let Some(obj) = value.as_object() {
        if let Some(rules) = obj.get("rules") {
            return is_record(rules);
        }
    }
    false
}
