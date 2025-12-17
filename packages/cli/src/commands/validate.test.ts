/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { assertEquals } from 'https://deno.land/std@0.208.0/assert/mod.ts';
import { validateCommand } from './validate.ts';
import { createTestFile, setupTestEnvironment } from '../test-utils/test-helpers.ts';

Deno.test('validate: should validate a valid definitions file', async () => {
  const { cleanup } = await setupTestEnvironment('-validate');

  try {
    const validDefinitions = `flags:
  - name: test_flag
    type: boolean
    defaultValue: false
    description: Test flag
`;

    await createTestFile('flags.definitions.yaml', validDefinitions);

    const exitCode = await validateCommand({
      definitions: 'flags.definitions.yaml',
    });

    assertEquals(exitCode, 0);
  } finally {
    await cleanup();
  }
});

Deno.test('validate: should reject invalid definitions file', async () => {
  const { cleanup } = await setupTestEnvironment('-validate');

  try {
    const invalidDefinitions = `flags:
  - name: test_flag
    # missing type and defaultValue
`;

    await createTestFile('flags.definitions.yaml', invalidDefinitions);

    const exitCode = await validateCommand({
      definitions: 'flags.definitions.yaml',
    });

    assertEquals(exitCode, 1);
  } finally {
    await cleanup();
  }
});

Deno.test('validate: should validate a valid deployment file', async () => {
  const { cleanup } = await setupTestEnvironment('-validate');

  try {
    const validDeployment = `environment: production
rules:
  test_flag:
    rules:
      - serve: false
`;

    await createTestFile('.controlpath/production.deployment.yaml', validDeployment);

    const exitCode = await validateCommand({
      deployment: '.controlpath/production.deployment.yaml',
    });

    assertEquals(exitCode, 0);
  } finally {
    await cleanup();
  }
});

Deno.test('validate: should reject invalid deployment file', async () => {
  const { cleanup } = await setupTestEnvironment('-validate');

  try {
    const invalidDeployment = `environment: production
# missing rules
`;

    await createTestFile('.controlpath/production.deployment.yaml', invalidDeployment);

    const exitCode = await validateCommand({
      deployment: '.controlpath/production.deployment.yaml',
    });

    assertEquals(exitCode, 1);
  } finally {
    await cleanup();
  }
});

Deno.test('validate: should auto-detect files when no flags provided', async () => {
  const { cleanup } = await setupTestEnvironment('-validate');

  try {
    const validDefinitions = `flags:
  - name: test_flag
    type: boolean
    defaultValue: false
    description: Test flag
`;

    await createTestFile('flags.definitions.yaml', validDefinitions);

    const exitCode = await validateCommand({});

    assertEquals(exitCode, 0);
  } finally {
    await cleanup();
  }
});

Deno.test('validate: should return error when no files found', async () => {
  const { cleanup } = await setupTestEnvironment('-validate');

  try {
    const exitCode = await validateCommand({});

    assertEquals(exitCode, 1);
  } finally {
    await cleanup();
  }
});

Deno.test('validate: should validate multiple files with --all flag', async () => {
  const { cleanup } = await setupTestEnvironment('-validate');

  try {
    const validDefinitions = `flags:
  - name: test_flag
    type: boolean
    defaultValue: false
    description: Test flag
`;

    const validDeployment = `environment: production
rules:
  test_flag:
    rules:
      - serve: false
`;

    await createTestFile('flags.definitions.yaml', validDefinitions);
    await createTestFile('.controlpath/production.deployment.yaml', validDeployment);

    const exitCode = await validateCommand({
      all: true,
    });

    assertEquals(exitCode, 0);
  } finally {
    await cleanup();
  }
});

Deno.test('validate: should validate deployment for specific environment', async () => {
  const { cleanup } = await setupTestEnvironment('-validate');

  try {
    const validDefinitions = `flags:
  - name: test_flag
    type: boolean
    defaultValue: false
    description: Test flag
`;

    const validDeployment = `environment: staging
rules:
  test_flag:
    rules:
      - serve: false
`;

    await createTestFile('flags.definitions.yaml', validDefinitions);
    await createTestFile('.controlpath/staging.deployment.yaml', validDeployment);

    const exitCode = await validateCommand({
      env: 'staging',
    });

    assertEquals(exitCode, 0);
  } finally {
    await cleanup();
  }
});

Deno.test('validate: should handle file not found gracefully', async () => {
  const { cleanup } = await setupTestEnvironment('-validate');

  try {
    const exitCode = await validateCommand({
      definitions: 'nonexistent.yaml',
    });

    assertEquals(exitCode, 1);
  } finally {
    await cleanup();
  }
});
