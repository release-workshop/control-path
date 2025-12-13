/**
 * Copyright 2024-2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

// Control Path CLI
// Main entry point for the CLI tool (Deno runtime)

import { Validator } from '@controlpath/compiler';
// Import embedded schemas (bundled with CLI, no disk access needed)
// These JSON files are bundled into the CLI binary at compile time
// Using Deno-compatible import with JSON import assertions
import definitionsSchema from '../../compiler/src/schemas/flag-definitions.schema.v1.json' with { type: 'json' };
import deploymentSchema from '../../compiler/src/schemas/flag-deployment.schema.v1.json' with { type: 'json' };

// Create validator with embedded schemas (bundled, no disk access)
const validator = new Validator({
  definitions: definitionsSchema,
  deployment: deploymentSchema,
});

console.log('Control Path CLI');
