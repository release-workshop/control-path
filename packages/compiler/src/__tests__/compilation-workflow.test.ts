/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { Validator } from '../validator';
import { parseDefinitionsFromString, parseDeploymentFromString } from '../parser';
import { compile, serialize, compileAndSerialize } from '../compiler';
import { unpack } from 'msgpackr';
import { Artifact, RuleType, isArtifact } from '../ast';
import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import { tmpdir } from 'node:os';

/**
 * Integration tests for the complete compilation workflow:
 * 1. Parse definitions
 * 2. Validate definitions
 * 3. Parse deployment
 * 4. Validate deployment
 * 5. Compile deployment to AST
 * 6. Serialize AST to MessagePack
 * 7. Verify AST can be deserialized and is valid
 */

describe('Integration: Compilation Workflow', () => {
  let validator: Validator;

  beforeEach(() => {
    validator = new Validator();
  });

  describe('Complete workflow: Validate → Compile → Serialize', () => {
    it('should complete full workflow for simple flag with no rules', () => {
      // Given: Valid flag definitions and deployment
      const definitionsYaml = `
flags:
  - name: new_dashboard
    type: boolean
    defaultValue: OFF
`;
      const definitions = parseDefinitionsFromString(definitionsYaml, 'test-definitions.yaml');

      const deploymentYaml = `
environment: production
rules:
  new_dashboard: {}
`;
      const deployment = parseDeploymentFromString(deploymentYaml, 'test-deployment.yaml');

      // When: We validate and compile the deployment
      const definitionsValidation = validator.validateDefinitions(
        'test-definitions.yaml',
        definitions
      );
      const deploymentValidation = validator.validateDeployment('test-deployment.yaml', deployment);
      const artifact = compile(deployment, definitions);
      const serialized = serialize(artifact);
      const deserialized = unpack(serialized) as Artifact;

      // Then: Validation should pass and AST should be correct
      expect(definitionsValidation.valid).toBe(true);
      expect(definitionsValidation.errors).toHaveLength(0);
      expect(deploymentValidation.valid).toBe(true);
      expect(deploymentValidation.errors).toHaveLength(0);
      expect(isArtifact(deserialized)).toBe(true);
      expect(deserialized.v).toBe('1.0');
      expect(deserialized.env).toBe('production');
      expect(deserialized.flags).toHaveLength(1);
      expect(deserialized.flags[0]).toHaveLength(1); // default rule
    });

    it('should complete full workflow for flag with simple rule', () => {
      // Given: Flag definitions and deployment with a serve rule
      const definitionsYaml = `
flags:
  - name: new_dashboard
    type: boolean
    defaultValue: OFF
`;
      const definitions = parseDefinitionsFromString(definitionsYaml, 'test-definitions.yaml');

      const deploymentYaml = `
environment: production
rules:
  new_dashboard:
    rules:
      - serve: ON
`;
      const deployment = parseDeploymentFromString(deploymentYaml, 'test-deployment.yaml');

      // When: We validate, compile, and serialize
      const definitionsValidation = validator.validateDefinitions(
        'test-definitions.yaml',
        definitions
      );
      const deploymentValidation = validator.validateDeployment('test-deployment.yaml', deployment);
      const artifact = compile(deployment, definitions);
      const serialized = serialize(artifact);
      const deserialized = unpack(serialized) as Artifact;

      // Then: Validation should pass and AST should contain the serve rule
      expect(definitionsValidation.valid).toBe(true);
      expect(deploymentValidation.valid).toBe(true);
      expect(isArtifact(deserialized)).toBe(true);
      expect(deserialized.v).toBe('1.0');
      expect(deserialized.env).toBe('production');
      expect(deserialized.flags).toHaveLength(1);
      expect(deserialized.flags[0]).toHaveLength(2); // rule + default

      const rule = deserialized.flags[0][0];
      expect(rule[0]).toBe(RuleType.SERVE);
      expect(rule[1]).toBeUndefined(); // no expression
      expect(deserialized.strs[rule[2] as number]).toBe('ON');
    });

    it('should complete full workflow for flag with expression rule', () => {
      // Given: Flag definitions and deployment with a conditional rule
      const definitionsYaml = `
flags:
  - name: new_dashboard
    type: boolean
    defaultValue: OFF
`;
      const definitions = parseDefinitionsFromString(definitionsYaml, 'test-definitions.yaml');

      const deploymentYaml = `
environment: production
rules:
  new_dashboard:
    rules:
      - serve: ON
        when: user.role == "admin"
`;
      const deployment = parseDeploymentFromString(deploymentYaml, 'test-deployment.yaml');

      // When: We validate, compile, and serialize
      const definitionsValidation = validator.validateDefinitions(
        'test-definitions.yaml',
        definitions
      );
      const deploymentValidation = validator.validateDeployment('test-deployment.yaml', deployment);
      const artifact = compile(deployment, definitions);
      const serialized = serialize(artifact);
      const deserialized = unpack(serialized) as Artifact;

      // Then: Validation should pass and AST should contain the expression
      expect(definitionsValidation.valid).toBe(true);
      expect(deploymentValidation.valid).toBe(true);
      expect(isArtifact(deserialized)).toBe(true);
      expect(deserialized.flags[0]).toHaveLength(2); // rule + default

      const rule = deserialized.flags[0][0];
      expect(rule[0]).toBe(RuleType.SERVE);
      expect(rule[1]).toBeDefined(); // expression should be present
      expect(deserialized.strs[rule[2] as number]).toBe('ON');
    });

    it('should complete full workflow for multiple flags', () => {
      // Given: Multiple flag definitions and deployment
      const definitionsYaml = `
flags:
  - name: feature_a
    type: boolean
    defaultValue: OFF
  - name: feature_b
    type: boolean
    defaultValue: OFF
  - name: feature_c
    type: boolean
    defaultValue: OFF
`;
      const definitions = parseDefinitionsFromString(definitionsYaml, 'test-definitions.yaml');

      const deploymentYaml = `
environment: production
rules:
  feature_a: {}
  feature_b:
    rules:
      - serve: ON
  feature_c: {}
`;
      const deployment = parseDeploymentFromString(deploymentYaml, 'test-deployment.yaml');

      // When: We validate, compile, and serialize
      const definitionsValidation = validator.validateDefinitions(
        'test-definitions.yaml',
        definitions
      );
      const deploymentValidation = validator.validateDeployment('test-deployment.yaml', deployment);
      const artifact = compile(deployment, definitions);
      const serialized = serialize(artifact);
      const deserialized = unpack(serialized) as Artifact;

      // Then: All flags should be compiled correctly
      expect(definitionsValidation.valid).toBe(true);
      expect(deploymentValidation.valid).toBe(true);
      expect(isArtifact(deserialized)).toBe(true);
      expect(deserialized.flags).toHaveLength(3);
      expect(deserialized.flags[0]).toHaveLength(1); // feature_a: default only
      expect(deserialized.flags[1]).toHaveLength(2); // feature_b: rule + default
      expect(deserialized.flags[2]).toHaveLength(1); // feature_c: default only
    });
  });

  describe('compileAndSerialize convenience function', () => {
    it('should compile and serialize in one step', () => {
      // Given: Valid flag definitions and deployment
      const definitionsYaml = `
flags:
  - name: new_dashboard
    type: boolean
    defaultValue: OFF
`;
      const definitions = parseDefinitionsFromString(definitionsYaml, 'test-definitions.yaml');

      const deploymentYaml = `
environment: production
rules:
  new_dashboard:
    rules:
      - serve: ON
`;
      const deployment = parseDeploymentFromString(deploymentYaml, 'test-deployment.yaml');

      // When: We validate and use compileAndSerialize
      const definitionsValidation = validator.validateDefinitions(
        'test-definitions.yaml',
        definitions
      );
      const deploymentValidation = validator.validateDeployment('test-deployment.yaml', deployment);
      const serialized = compileAndSerialize(deployment, definitions);
      const deserialized = unpack(serialized) as Artifact;

      // Then: Validation should pass and AST should be correct
      expect(definitionsValidation.valid).toBe(true);
      expect(deploymentValidation.valid).toBe(true);
      expect(isArtifact(deserialized)).toBe(true);
      expect(deserialized.v).toBe('1.0');
      expect(deserialized.env).toBe('production');
      expect(deserialized.flags).toHaveLength(1);
    });
  });

  describe('Error handling in workflow', () => {
    it('should reject invalid definitions', () => {
      // Given: Invalid flag definitions with missing required fields
      const definitionsYaml = `
flags:
  - name: invalid_flag
    # missing required fields
`;
      const definitions = parseDefinitionsFromString(definitionsYaml, 'test-definitions.yaml');

      // When: We validate the definitions
      const validation = validator.validateDefinitions('test-definitions.yaml', definitions);

      // Then: Validation should fail with errors
      expect(validation.valid).toBe(false);
      expect(validation.errors.length).toBeGreaterThan(0);
    });

    it('should reject invalid deployment structure', () => {
      // Given: Invalid deployment structure with invalid fields
      const deploymentYaml = `
environment: production
rules:
  new_dashboard:
    rules:
      - serve: ON
        invalid_field: value
`;
      const deployment = parseDeploymentFromString(deploymentYaml, 'test-deployment.yaml');

      // When: We validate the deployment
      const validation = validator.validateDeployment('test-deployment.yaml', deployment);

      // Then: Validation should fail with errors
      expect(validation.valid).toBe(false);
      expect(validation.errors.length).toBeGreaterThan(0);
    });

    it('should throw error when compiling invalid deployment', () => {
      // Given: Definitions and deployment referencing a nonexistent flag
      const definitionsYaml = `
flags:
  - name: new_dashboard
    type: boolean
    defaultValue: OFF
`;
      const definitions = parseDefinitionsFromString(definitionsYaml, 'test-definitions.yaml');

      const deploymentYaml = `
environment: production
rules:
  nonexistent_flag: {}
`;
      const deployment = parseDeploymentFromString(deploymentYaml, 'test-deployment.yaml');

      // When/Then: Compilation should throw an error
      expect(() => {
        compile(deployment, definitions);
      }).toThrow('Flag "nonexistent_flag" not found in flag definitions');
    });
  });

  describe('File I/O workflow', () => {
    it('should compile and write AST file', async () => {
      // Given: Valid definitions, deployment, and a temporary directory
      const tmpDir = await fs.mkdtemp(path.join(tmpdir(), 'controlpath-test-'));
      const outputPath = path.join(tmpDir, 'output.ast');

      try {
        const definitionsYaml = `
flags:
  - name: new_dashboard
    type: boolean
    defaultValue: OFF
`;
        const definitions = parseDefinitionsFromString(definitionsYaml, 'test-definitions.yaml');

        const deploymentYaml = `
environment: production
rules:
  new_dashboard:
    rules:
      - serve: ON
`;
        const deployment = parseDeploymentFromString(deploymentYaml, 'test-deployment.yaml');

        // When: We validate, compile, serialize, and write to file
        const definitionsValidation = validator.validateDefinitions(
          'test-definitions.yaml',
          definitions
        );
        const deploymentValidation = validator.validateDeployment(
          'test-deployment.yaml',
          deployment
        );
        const serialized = compileAndSerialize(deployment, definitions);
        await fs.writeFile(outputPath, serialized);
        const fileContent = await fs.readFile(outputPath);
        const deserialized = unpack(fileContent) as Artifact;

        // Then: Validation should pass, file should exist, and AST should be correct
        expect(definitionsValidation.valid).toBe(true);
        expect(deploymentValidation.valid).toBe(true);
        const stats = await fs.stat(outputPath);
        expect(stats.isFile()).toBe(true);
        expect(stats.size).toBeGreaterThan(0);
        expect(isArtifact(deserialized)).toBe(true);
        expect(deserialized.v).toBe('1.0');
        expect(deserialized.env).toBe('production');
      } finally {
        await fs.rm(tmpDir, { recursive: true, force: true });
      }
    });
  });
});
