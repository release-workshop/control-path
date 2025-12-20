/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { Validator } from '../validator';
import { parseDefinitionsFromString, parseDeploymentFromString } from '../parser';
import { compileAndSerialize } from '../compiler';
import { unpack } from 'msgpackr';
import { Artifact, isArtifact } from '../ast';

/**
 * Tests using example files from the examples/ directory.
 * These tests verify that the compiler can handle real-world configurations.
 */

/**
 * Find examples directory, trying multiple possible locations.
 * Returns null if not found (tests will be skipped).
 */
function findExamplesDir(): string | null {
  const possiblePaths = [
    // From compiled output (dist/)
    join(__dirname, '../../../../../control-path-next/examples'),
    // From source (src/)
    join(__dirname, '../../../../../../control-path-next/examples'),
    // Relative to package root
    join(__dirname, '../../../../control-path-next/examples'),
    // Relative to monorepo root
    join(__dirname, '../../../../../examples'),
  ];

  for (const path of possiblePaths) {
    try {
      const testFile = join(path, 'simple/flags.definitions.yaml');
      readFileSync(testFile, 'utf-8');
      return path;
    } catch {
      // Path doesn't exist, try next
    }
  }

  return null;
}

const EXAMPLES_DIR = findExamplesDir();

// Example content embedded for CI compatibility
const SIMPLE_FLAGS_DEFINITIONS = `flags:
  - name: new_dashboard
    type: boolean
    defaultValue: OFF
    description: "New dashboard UI feature"
  
  - name: enable_analytics
    type: boolean
    defaultValue: false
    description: "Enable analytics tracking"
`;

const COMPLEX_FLAGS_DEFINITIONS = `context:
  user:
    age?: 'number'
    department: 'string'

flags:
  - name: new_dashboard
    type: boolean
    defaultValue: OFF
    description: "New dashboard UI feature"
    metadata:
      owner: "frontend-team"
      ticket: "FE-1234"
  
  - name: theme_color
    type: multivariate
    defaultValue: blue
    description: "Application theme color"
    variations:
      - name: BLUE
        value: "blue"
        description: "Default blue theme"
      - name: GREEN
        value: "green"
        description: "Green theme"
      - name: DARK
        value: "dark"
        description: "Dark theme"
  
  - name: api_version
    type: multivariate
    defaultValue: v1
    description: "API version to use"
    variations:
      - name: V1
        value: "v1"
      - name: V2
        value: "v2"
      - name: V3
        value: "v3"
`;

describe('Examples: Simple Configuration', () => {
  it('should compile simple flags.definitions.yaml', () => {
    // Given: Simple flag definitions from examples
    const definitionsContent = EXAMPLES_DIR
      ? readFileSync(join(EXAMPLES_DIR, 'simple/flags.definitions.yaml'), 'utf-8')
      : SIMPLE_FLAGS_DEFINITIONS;
    const definitionsPath = 'examples/simple/flags.definitions.yaml';
    const definitions = parseDefinitionsFromString(definitionsContent, definitionsPath);

    // When: We validate the definitions
    const validator = new Validator();
    const validation = validator.validateDefinitions(definitionsPath, definitions);

    // Then: Validation should pass
    expect(validation.valid).toBe(true);
    expect(validation.errors).toHaveLength(0);
    expect(definitions.flags).toHaveLength(2);
    expect(definitions.flags[0].name).toBe('new_dashboard');
    expect(definitions.flags[1].name).toBe('enable_analytics');
  });

  it('should compile simple deployment with default rules', () => {
    // Given: Simple flag definitions and a basic deployment
    const definitionsContent = EXAMPLES_DIR
      ? readFileSync(join(EXAMPLES_DIR, 'simple/flags.definitions.yaml'), 'utf-8')
      : SIMPLE_FLAGS_DEFINITIONS;
    const definitionsPath = 'examples/simple/flags.definitions.yaml';
    const definitions = parseDefinitionsFromString(definitionsContent, definitionsPath);

    const deploymentYaml = `
environment: production
rules:
  new_dashboard: {}
  enable_analytics: {}
`;
    const deployment = parseDeploymentFromString(deploymentYaml, 'test-deployment.yaml');

    // When: We validate and compile
    const validator = new Validator();
    const definitionsValidation = validator.validateDefinitions(definitionsPath, definitions);
    const deploymentValidation = validator.validateDeployment('test-deployment.yaml', deployment);
    const serialized = compileAndSerialize(deployment, definitions);
    const deserialized = unpack(serialized) as Artifact;

    // Then: Everything should work correctly
    expect(definitionsValidation.valid).toBe(true);
    expect(deploymentValidation.valid).toBe(true);
    expect(isArtifact(deserialized)).toBe(true);
    expect(deserialized.v).toBe('1.0');
    expect(deserialized.env).toBe('production');
    expect(deserialized.flags).toHaveLength(2);
    // Each flag should have a default rule
    expect(deserialized.flags[0]).toHaveLength(1);
    expect(deserialized.flags[1]).toHaveLength(1);
  });

  it('should compile simple deployment with serve rules', () => {
    // Given: Simple flag definitions and deployment with serve rules
    const definitionsContent = EXAMPLES_DIR
      ? readFileSync(join(EXAMPLES_DIR, 'simple/flags.definitions.yaml'), 'utf-8')
      : SIMPLE_FLAGS_DEFINITIONS;
    const definitionsPath = 'examples/simple/flags.definitions.yaml';
    const definitions = parseDefinitionsFromString(definitionsContent, definitionsPath);

    const deploymentYaml = `
environment: production
rules:
  new_dashboard:
    rules:
      - serve: ON
  enable_analytics:
    rules:
      - serve: true
`;
    const deployment = parseDeploymentFromString(deploymentYaml, 'test-deployment.yaml');

    // When: We validate and compile
    const validator = new Validator();
    const definitionsValidation = validator.validateDefinitions(definitionsPath, definitions);
    const deploymentValidation = validator.validateDeployment('test-deployment.yaml', deployment);
    const serialized = compileAndSerialize(deployment, definitions);
    const deserialized = unpack(serialized) as Artifact;

    // Then: Everything should work correctly
    expect(definitionsValidation.valid).toBe(true);
    expect(deploymentValidation.valid).toBe(true);
    expect(isArtifact(deserialized)).toBe(true);
    expect(deserialized.flags).toHaveLength(2);
    // Each flag should have a rule + default
    expect(deserialized.flags[0]).toHaveLength(2);
    expect(deserialized.flags[1]).toHaveLength(2);
  });
});

describe('Examples: Complex Configuration', () => {
  it('should compile complex flags.definitions.yaml with multivariate flags', () => {
    // Given: Complex flag definitions from examples
    const definitionsContent = EXAMPLES_DIR
      ? readFileSync(join(EXAMPLES_DIR, 'complex/flags.definitions.yaml'), 'utf-8')
      : COMPLEX_FLAGS_DEFINITIONS;
    const definitionsPath = 'examples/complex/flags.definitions.yaml';
    const definitions = parseDefinitionsFromString(definitionsContent, definitionsPath);

    // When: We validate the definitions
    const validator = new Validator();
    const validation = validator.validateDefinitions(definitionsPath, definitions);

    // Then: Validation should pass
    expect(validation.valid).toBe(true);
    expect(validation.errors).toHaveLength(0);
    expect(definitions.flags).toHaveLength(3);
    expect(definitions.flags[0].name).toBe('new_dashboard');
    expect(definitions.flags[0].type).toBe('boolean');
    expect(definitions.flags[1].name).toBe('theme_color');
    expect(definitions.flags[1].type).toBe('multivariate');
    expect(definitions.flags[1].variations).toHaveLength(3);
    expect(definitions.flags[2].name).toBe('api_version');
    expect(definitions.flags[2].type).toBe('multivariate');
    expect(definitions.flags[2].variations).toHaveLength(3);
  });

  it('should compile complex deployment with multivariate flags', () => {
    // Given: Complex flag definitions and deployment with variations
    const definitionsContent = EXAMPLES_DIR
      ? readFileSync(join(EXAMPLES_DIR, 'complex/flags.definitions.yaml'), 'utf-8')
      : COMPLEX_FLAGS_DEFINITIONS;
    const definitionsPath = 'examples/complex/flags.definitions.yaml';
    const definitions = parseDefinitionsFromString(definitionsContent, definitionsPath);

    const deploymentYaml = `
environment: production
rules:
  new_dashboard:
    rules:
      - serve: ON
  theme_color:
    rules:
      - variations:
          - variation: BLUE
            weight: 50
          - variation: GREEN
            weight: 30
          - variation: DARK
            weight: 20
  api_version:
    rules:
      - variations:
          - variation: V1
            weight: 70
          - variation: V2
            weight: 20
          - variation: V3
            weight: 10
`;
    const deployment = parseDeploymentFromString(deploymentYaml, 'test-deployment.yaml');

    // When: We validate and compile
    const validator = new Validator();
    const definitionsValidation = validator.validateDefinitions(definitionsPath, definitions);
    const deploymentValidation = validator.validateDeployment('test-deployment.yaml', deployment);
    const serialized = compileAndSerialize(deployment, definitions);
    const deserialized = unpack(serialized) as Artifact;

    // Then: Everything should work correctly
    expect(definitionsValidation.valid).toBe(true);
    expect(deploymentValidation.valid).toBe(true);
    expect(isArtifact(deserialized)).toBe(true);
    expect(deserialized.v).toBe('1.0');
    expect(deserialized.env).toBe('production');
    expect(deserialized.flags).toHaveLength(3);
  });

  it('should compile complex deployment with expressions', () => {
    // Given: Complex flag definitions and deployment with conditional rules
    const definitionsContent = EXAMPLES_DIR
      ? readFileSync(join(EXAMPLES_DIR, 'complex/flags.definitions.yaml'), 'utf-8')
      : COMPLEX_FLAGS_DEFINITIONS;
    const definitionsPath = 'examples/complex/flags.definitions.yaml';
    const definitions = parseDefinitionsFromString(definitionsContent, definitionsPath);

    const deploymentYaml = `
environment: production
rules:
  new_dashboard:
    rules:
      - serve: ON
        when: user.role == "admin"
  theme_color:
    rules:
      - serve: DARK
        when: user.department == "engineering"
      - variations:
          - variation: BLUE
            weight: 50
          - variation: GREEN
            weight: 50
  api_version:
    rules:
      - serve: V2
        when: user.age >= 18
`;
    const deployment = parseDeploymentFromString(deploymentYaml, 'test-deployment.yaml');

    // When: We validate and compile
    const validator = new Validator();
    const definitionsValidation = validator.validateDefinitions(definitionsPath, definitions);
    const deploymentValidation = validator.validateDeployment('test-deployment.yaml', deployment);
    const serialized = compileAndSerialize(deployment, definitions);
    const deserialized = unpack(serialized) as Artifact;

    // Then: Everything should work correctly
    expect(definitionsValidation.valid).toBe(true);
    expect(deploymentValidation.valid).toBe(true);
    expect(isArtifact(deserialized)).toBe(true);
    expect(deserialized.flags).toHaveLength(3);
    // Flags with expressions should have when clauses
    expect(deserialized.flags[0][0][1]).toBeDefined(); // new_dashboard has expression
    expect(deserialized.flags[1][0][1]).toBeDefined(); // theme_color has expression
    expect(deserialized.flags[2][0][1]).toBeDefined(); // api_version has expression
  });

  it('should compile complex deployment with segments', () => {
    // Given: Complex flag definitions and deployment with segments
    const definitionsContent = EXAMPLES_DIR
      ? readFileSync(join(EXAMPLES_DIR, 'complex/flags.definitions.yaml'), 'utf-8')
      : COMPLEX_FLAGS_DEFINITIONS;
    const definitionsPath = 'examples/complex/flags.definitions.yaml';
    const definitions = parseDefinitionsFromString(definitionsContent, definitionsPath);

    const deploymentYaml = `
environment: production
segments:
  beta_users:
    when: user.role == "beta"
  engineering_team:
    when: user.department == "engineering"
rules:
  new_dashboard:
    rules:
      - serve: ON
  theme_color:
    rules:
      - serve: DARK
  api_version:
    rules:
      - serve: V2
`;
    const deployment = parseDeploymentFromString(deploymentYaml, 'test-deployment.yaml');

    // When: We validate and compile
    const validator = new Validator();
    const definitionsValidation = validator.validateDefinitions(definitionsPath, definitions);
    const deploymentValidation = validator.validateDeployment('test-deployment.yaml', deployment);
    const serialized = compileAndSerialize(deployment, definitions);
    const deserialized = unpack(serialized) as Artifact;

    // Then: Everything should work correctly
    expect(definitionsValidation.valid).toBe(true);
    expect(deploymentValidation.valid).toBe(true);
    expect(isArtifact(deserialized)).toBe(true);
    expect(deserialized.segments).toBeDefined();
    expect(deserialized.segments!).toHaveLength(2);
    // Verify segment names are in string table
    const segmentNames = deserialized.segments!.map(([nameIndex]) => deserialized.strs[nameIndex]);
    expect(segmentNames).toContain('beta_users');
    expect(segmentNames).toContain('engineering_team');
  });

  it('should compile complex deployment with rollout rules', () => {
    // Given: Complex flag definitions and deployment with rollout
    const definitionsContent = EXAMPLES_DIR
      ? readFileSync(join(EXAMPLES_DIR, 'complex/flags.definitions.yaml'), 'utf-8')
      : COMPLEX_FLAGS_DEFINITIONS;
    const definitionsPath = 'examples/complex/flags.definitions.yaml';
    const definitions = parseDefinitionsFromString(definitionsContent, definitionsPath);

    const deploymentYaml = `
environment: production
rules:
  new_dashboard:
    rules:
      - rollout:
          variation: ON
          percentage: 25
  theme_color:
    rules:
      - rollout:
          variation: DARK
          percentage: 50
  api_version:
    rules:
      - rollout:
          variation: V3
          percentage: 10
`;
    const deployment = parseDeploymentFromString(deploymentYaml, 'test-deployment.yaml');

    // When: We validate and compile
    const validator = new Validator();
    const definitionsValidation = validator.validateDefinitions(definitionsPath, definitions);
    const deploymentValidation = validator.validateDeployment('test-deployment.yaml', deployment);
    const serialized = compileAndSerialize(deployment, definitions);
    const deserialized = unpack(serialized) as Artifact;

    // Then: Everything should work correctly
    expect(definitionsValidation.valid).toBe(true);
    expect(deploymentValidation.valid).toBe(true);
    expect(isArtifact(deserialized)).toBe(true);
    expect(deserialized.flags).toHaveLength(3);
    // All flags should have rollout rules
    expect(deserialized.flags[0][0][0]).toBe(2); // RuleType.ROLLOUT
    expect(deserialized.flags[1][0][0]).toBe(2); // RuleType.ROLLOUT
    expect(deserialized.flags[2][0][0]).toBe(2); // RuleType.ROLLOUT
  });
});
