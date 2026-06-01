/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

use crate::catalog::model::{CatalogIdentity, EffectiveCatalogId, WorkspaceDocument};

/// Resolve namespace: `catalog.namespace` → workspace → none.
#[must_use]
pub fn resolve_namespace(
    catalog: &CatalogIdentity,
    workspace: Option<&WorkspaceDocument>,
) -> Option<String> {
    if let Some(ns) = &catalog.namespace {
        return Some(ns.clone());
    }
    workspace.map(|w| w.namespace.clone())
}

/// Build effective catalog id after namespace resolution.
#[must_use]
pub fn effective_catalog_id(
    catalog: &CatalogIdentity,
    workspace: Option<&WorkspaceDocument>,
) -> EffectiveCatalogId {
    EffectiveCatalogId {
        namespace: resolve_namespace(catalog, workspace),
        id: catalog.id.clone(),
    }
}
