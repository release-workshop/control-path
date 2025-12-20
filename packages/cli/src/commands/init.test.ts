/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { assertEquals } from 'https://deno.land/std@0.208.0/assert/mod.ts';
import { initCommand } from './init.ts';
import { pathExists, readTestFile, setupTestEnvironment } from '../test-utils/test-helpers.ts';

Deno.test('init: should create project files in empty directory', async () => {
  const { cleanup } = await setupTestEnvironment('-init');

  try {
    const exitCode = await initCommand({});

    assertEquals(exitCode, 0);

    // Verify files were created
    const definitionsExists = await pathExists('flags.definitions.yaml');
    const deploymentExists = await pathExists('.controlpath/production.deployment.yaml');

    assertEquals(definitionsExists, true);
    assertEquals(deploymentExists, true);
  } finally {
    await cleanup();
  }
});

Deno.test('init: should create definitions file with example flag', async () => {
  const { cleanup } = await setupTestEnvironment('-init');

  try {
    const exitCode = await initCommand({});

    assertEquals(exitCode, 0);

    const content = await readTestFile('flags.definitions.yaml');
    assertEquals(content.includes('example_flag'), true);
    assertEquals(content.includes('type: boolean'), true);
  } finally {
    await cleanup();
  }
});

Deno.test('init: should not create definitions file with --no-examples', async () => {
  const { cleanup } = await setupTestEnvironment('-init');

  try {
    const exitCode = await initCommand({
      noExamples: true,
    });

    assertEquals(exitCode, 0);

    const definitionsExists = await pathExists('flags.definitions.yaml');

    assertEquals(definitionsExists, false);
  } finally {
    await cleanup();
  }
});

Deno.test(
  'init: should create definitions file with --example-flags even if --no-examples',
  async () => {
    const { cleanup } = await setupTestEnvironment('-init');

    try {
      const exitCode = await initCommand({
        noExamples: true,
        exampleFlags: true,
      });

      assertEquals(exitCode, 0);

      const definitionsExists = await pathExists('flags.definitions.yaml');

      assertEquals(definitionsExists, true);
    } finally {
      await cleanup();
    }
  },
);

Deno.test('init: should fail when files exist without --force', async () => {
  const { cleanup } = await setupTestEnvironment('-init');

  try {
    // Create existing file
    await Deno.writeTextFile('flags.definitions.yaml', 'existing content');

    const exitCode = await initCommand({});

    assertEquals(exitCode, 1);
  } finally {
    await cleanup();
  }
});

Deno.test('init: should succeed with --force when files exist', async () => {
  const { cleanup } = await setupTestEnvironment('-init');

  try {
    // Create existing file
    await Deno.writeTextFile('flags.definitions.yaml', 'existing content');

    const exitCode = await initCommand({
      force: true,
    });

    assertEquals(exitCode, 0);

    // Verify new content was written
    const content = await readTestFile('flags.definitions.yaml');
    assertEquals(content.includes('example_flag'), true);
  } finally {
    await cleanup();
  }
});

Deno.test('init: should create .controlpath directory', async () => {
  const { cleanup } = await setupTestEnvironment('-init');

  try {
    const exitCode = await initCommand({});

    assertEquals(exitCode, 0);

    const controlpathExists = await pathExists('.controlpath');

    assertEquals(controlpathExists, true);
  } finally {
    await cleanup();
  }
});

Deno.test('init: should create production deployment file', async () => {
  const { cleanup } = await setupTestEnvironment('-init');

  try {
    const exitCode = await initCommand({});

    assertEquals(exitCode, 0);

    const content = await readTestFile('.controlpath/production.deployment.yaml');
    assertEquals(content.includes('environment: production'), true);
    assertEquals(content.includes('rules:'), true);
  } finally {
    await cleanup();
  }
});
