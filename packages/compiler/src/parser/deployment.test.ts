/**
 * Copyright 2024-2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import { parseDeployment, parseDeploymentFromString, ParseError } from './deployment';

describe('parseDeployment', () => {
  it('should parse valid YAML deployment', () => {
    const yaml = `
environment: production
rules:
  new_dashboard:
    default: OFF
    rules:
      - name: "Enable for admins"
        when: "user.role == 'admin'"
        serve: ON
      - name: "10% rollout"
        when: "true"
        rollout:
          variation: ON
          percentage: 10
`;

    const result = parseDeploymentFromString(yaml, 'test.yaml');

    expect(result).toBeDefined();
    expect(result.environment).toBe('production');
    expect(result.rules).toBeDefined();
    expect(result.rules.new_dashboard).toBeDefined();
    expect(result.rules.new_dashboard.default).toBe('OFF');
    expect(result.rules.new_dashboard.rules).toHaveLength(2);
    expect(result.rules.new_dashboard.rules?.[0].name).toBe('Enable for admins');
    expect(result.rules.new_dashboard.rules?.[0].serve).toBe('ON');
    expect(result.rules.new_dashboard.rules?.[1].rollout?.percentage).toBe(10);
  });

  it('should parse valid JSON deployment', () => {
    const json = JSON.stringify({
      environment: 'staging',
      rules: {
        test_flag: {
          default: false,
        },
      },
    });

    const result = parseDeploymentFromString(json, 'test.json');

    expect(result).toBeDefined();
    expect(result.environment).toBe('staging');
    expect(result.rules.test_flag).toBeDefined();
    expect(result.rules.test_flag.default).toBe(false);
  });

  it('should parse deployment with variations', () => {
    const yaml = `
environment: production
rules:
  theme:
    default: light
    rules:
      - name: "Theme distribution"
        variations:
          - variation: LIGHT
            weight: 50
          - variation: DARK
            weight: 30
          - variation: AUTO
            weight: 20
`;

    const result = parseDeploymentFromString(yaml, 'test.yaml');

    expect(result).toBeDefined();
    expect(result.rules.theme.rules?.[0].variations).toHaveLength(3);
    expect(result.rules.theme.rules?.[0].variations?.[0].variation).toBe('LIGHT');
    expect(result.rules.theme.rules?.[0].variations?.[0].weight).toBe(50);
  });

  it('should parse deployment with segments', () => {
    const yaml = `
environment: production
rules:
  test_flag:
    default: false
segments:
  beta_users:
    when: "user.role == 'beta'"
  premium_customers:
    when: "user.subscription_tier == 'premium'"
`;

    const result = parseDeploymentFromString(yaml, 'test.yaml');

    expect(result).toBeDefined();
    expect(result.segments).toBeDefined();
    expect(result.segments?.beta_users).toBeDefined();
    expect(result.segments?.beta_users.when).toBe("user.role == 'beta'");
  });

  it('should throw ParseError for invalid YAML', () => {
    const invalidYaml = `
environment: production
rules:
  test: [unclosed
`;

    expect(() => {
      parseDeploymentFromString(invalidYaml, 'test.yaml');
    }).toThrow(ParseError);
  });

  it('should throw ParseError for invalid JSON', () => {
    const invalidJson = '{"environment": "production"';

    expect(() => {
      parseDeploymentFromString(invalidJson, 'test.json');
    }).toThrow(ParseError);
  });

  it('should throw ParseError when environment field is missing', () => {
    const yaml = `
rules:
  test_flag:
    default: false
`;

    expect(() => {
      parseDeploymentFromString(yaml, 'test.yaml');
    }).toThrow(ParseError);
  });

  it('should throw ParseError when rules field is missing', () => {
    const yaml = `
environment: production
`;

    expect(() => {
      parseDeploymentFromString(yaml, 'test.yaml');
    }).toThrow(ParseError);
  });

  it('should throw ParseError when environment is not a string', () => {
    const yaml = `
environment: 123
rules:
  test_flag:
    default: false
`;

    expect(() => {
      parseDeploymentFromString(yaml, 'test.yaml');
    }).toThrow(ParseError);
  });

  it('should throw ParseError when rules is not an object', () => {
    const yaml = `
environment: production
rules: not_an_object
`;

    expect(() => {
      parseDeploymentFromString(yaml, 'test.yaml');
    }).toThrow(ParseError);
  });

  it('should parse from file', () => {
    // Create a temporary file
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'controlpath-test-'));
    const testFile = path.join(tmpDir, 'test.yaml');

    const yaml = `
environment: production
rules:
  test_flag:
    default: false
`;

    fs.writeFileSync(testFile, yaml);

    try {
      const result = parseDeployment(testFile);
      expect(result).toBeDefined();
      expect(result.environment).toBe('production');
      expect(result.rules.test_flag).toBeDefined();
    } finally {
      fs.unlinkSync(testFile);
      fs.rmdirSync(tmpDir);
    }
  });

  it('should throw ParseError for non-existent file', () => {
    expect(() => {
      parseDeployment('/nonexistent/file.yaml');
    }).toThrow(ParseError);
  });
});
