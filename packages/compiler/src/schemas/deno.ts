/**
 * Copyright 2024-2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

// Deno-compatible schema exports
// These schemas are bundled with the CLI binary and don't require disk access

import definitionsSchema from './flag-definitions.schema.v1.json' with { type: 'json' };
import deploymentSchema from './flag-deployment.schema.v1.json' with { type: 'json' };

export { definitionsSchema, deploymentSchema };

