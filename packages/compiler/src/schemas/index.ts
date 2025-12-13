/**
 * Copyright 2024-2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

// Export embedded schemas for bundling with CLI
// These schemas are bundled at build time and don't require disk access
// Note: For Node.js builds, use require() since import assertions aren't supported in CommonJS
// For Deno, use the deno.ts file which has import assertions

// Use require for Node.js compatibility (CommonJS)
// eslint-disable-next-line @typescript-eslint/no-require-imports
const definitionsSchema = require('./flag-definitions.schema.v1.json');
// eslint-disable-next-line @typescript-eslint/no-require-imports
const deploymentSchema = require('./flag-deployment.schema.v1.json');

export { definitionsSchema, deploymentSchema };

