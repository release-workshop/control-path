/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { assertEquals } from 'https://deno.land/std@0.208.0/assert/mod.ts';
import { compileCommand } from './compile.ts';
import { createTestFile, pathExists, setupTestEnvironment } from '../test-utils/test-helpers.ts';

Deno.test('compile: should compile valid deployment file', async () => {
  const { cleanup } = await setupTestEnvironment('-compile');

  try {
    const definitions = `flags:
  - name: test_flag
    type: boolean
    defaultValue: false
    description: Test flag
`;

    const deployment = `environment: production
rules:
  test_flag:
    rules:
      - serve: false
`;

    await createTestFile('flags.definitions.yaml', definitions);
    await createTestFile('.controlpath/production.deployment.yaml', deployment);

    const exitCode = await compileCommand({
      deployment: '.controlpath/production.deployment.yaml',
      output: '.controlpath/production.ast',
    });

    assertEquals(exitCode, 0);

    // Verify AST file was created
    const astExists = await pathExists('.controlpath/production.ast');

    assertEquals(astExists, true);
  } finally {
    await cleanup();
  }
});

Deno.test('compile: should infer output path from deployment file', async () => {
  const { cleanup } = await setupTestEnvironment('-compile');

  try {
    const definitions = `flags:
  - name: test_flag
    type: boolean
    defaultValue: false
    description: Test flag
`;

    const deployment = `environment: staging
rules:
  test_flag:
    rules:
      - serve: false
`;

    await createTestFile('flags.definitions.yaml', definitions);
    await createTestFile('.controlpath/staging.deployment.yaml', deployment);

    const exitCode = await compileCommand({
      deployment: '.controlpath/staging.deployment.yaml',
    });

    assertEquals(exitCode, 0);

    // Verify AST file was created with inferred name
    const astExists = await pathExists('.controlpath/staging.ast');

    assertEquals(astExists, true);
  } finally {
    await cleanup();
  }
});

Deno.test('compile: should use --env flag to infer paths', async () => {
  const { cleanup } = await setupTestEnvironment('-compile');

  try {
    const definitions = `flags:
  - name: test_flag
    type: boolean
    defaultValue: false
    description: Test flag
`;

    const deployment = `environment: production
rules:
  test_flag:
    rules:
      - serve: false
`;

    await createTestFile('flags.definitions.yaml', definitions);
    await createTestFile('.controlpath/production.deployment.yaml', deployment);

    const exitCode = await compileCommand({
      env: 'production',
    });

    assertEquals(exitCode, 0);

    // Verify AST file was created
    const astExists = await pathExists('.controlpath/production.ast');

    assertEquals(astExists, true);
  } finally {
    await cleanup();
  }
});

Deno.test('compile: should fail with invalid deployment file', async () => {
  const { cleanup } = await setupTestEnvironment('-compile');

  try {
    const definitions = `flags:
  - name: test_flag
    type: boolean
    defaultValue: false
    description: Test flag
`;

    const invalidDeployment = `environment: production
# missing rules
`;

    await createTestFile('flags.definitions.yaml', definitions);
    await createTestFile('.controlpath/production.deployment.yaml', invalidDeployment);

    const exitCode = await compileCommand({
      env: 'production',
    });

    assertEquals(exitCode, 1);
  } finally {
    await cleanup();
  }
});

Deno.test('compile: should fail when deployment file not found', async () => {
  const { cleanup } = await setupTestEnvironment('-compile');

  try {
    const exitCode = await compileCommand({
      deployment: 'nonexistent.yaml',
      output: 'output.ast',
    });

    assertEquals(exitCode, 1);
  } finally {
    await cleanup();
  }
});

Deno.test('compile: should fail when definitions file not found', async () => {
  const { cleanup } = await setupTestEnvironment('-compile');

  try {
    const deployment = `environment: production
rules:
  test_flag:
    rules:
      - serve: false
`;

    await createTestFile('.controlpath/production.deployment.yaml', deployment);

    const exitCode = await compileCommand({
      env: 'production',
      definitions: 'nonexistent.yaml',
    });

    assertEquals(exitCode, 1);
  } finally {
    await cleanup();
  }
});

Deno.test('compile: should create output directory if it does not exist', async () => {
  const { cleanup } = await setupTestEnvironment('-compile');

  try {
    const definitions = `flags:
  - name: test_flag
    type: boolean
    defaultValue: false
    description: Test flag
`;

    const deployment = `environment: production
rules:
  test_flag:
    rules:
      - serve: false
`;

    await createTestFile('flags.definitions.yaml', definitions);
    await createTestFile('.controlpath/production.deployment.yaml', deployment);

    const exitCode = await compileCommand({
      env: 'production',
      output: 'custom-output/production.ast',
    });

    assertEquals(exitCode, 0);

    // Verify AST file was created in custom directory
    const astExists = await pathExists('custom-output/production.ast');

    assertEquals(astExists, true);
  } finally {
    await cleanup();
  }
});

Deno.test('compile: should fail when required flags missing', async () => {
  const { cleanup } = await setupTestEnvironment('-compile');

  try {
    const exitCode = await compileCommand({});

    assertEquals(exitCode, 1);
  } finally {
    await cleanup();
  }
});
