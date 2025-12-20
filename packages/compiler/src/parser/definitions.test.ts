/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import { parseDefinitions, parseDefinitionsFromString } from './definitions';
import { ParseError } from './parse-error';

describe('parseDefinitions', () => {
  it('should parse valid YAML flag definitions', () => {
    const yaml = `
flags:
  - name: new_dashboard
    type: boolean
    defaultValue: OFF
    description: "New dashboard UI feature"
  
  - name: enable_analytics
    type: boolean
    defaultValue: false
    description: "Enable analytics tracking"
`;

    const result = parseDefinitionsFromString(yaml, 'test.yaml');

    expect(result).toBeDefined();
    expect(result.flags).toHaveLength(2);
    expect(result.flags[0].name).toBe('new_dashboard');
    expect(result.flags[0].type).toBe('boolean');
    expect(result.flags[0].defaultValue).toBe('OFF');
    expect(result.flags[1].name).toBe('enable_analytics');
    expect(result.flags[1].type).toBe('boolean');
    expect(result.flags[1].defaultValue).toBe(false);
  });

  it('should parse valid JSON flag definitions', () => {
    const json = JSON.stringify({
      flags: [
        {
          name: 'test_flag',
          type: 'boolean',
          defaultValue: true,
          description: 'Test flag',
        },
      ],
    });

    const result = parseDefinitionsFromString(json, 'test.json');

    expect(result).toBeDefined();
    expect(result.flags).toHaveLength(1);
    expect(result.flags[0].name).toBe('test_flag');
    expect(result.flags[0].type).toBe('boolean');
    expect(result.flags[0].defaultValue).toBe(true);
  });

  it('should parse multivariate flag definitions', () => {
    const yaml = `
flags:
  - name: theme
    type: multivariate
    defaultValue: light
    variations:
      - name: LIGHT
        value: light
      - name: DARK
        value: dark
      - name: AUTO
        value: auto
`;

    const result = parseDefinitionsFromString(yaml, 'test.yaml');

    expect(result).toBeDefined();
    expect(result.flags).toHaveLength(1);
    expect(result.flags[0].type).toBe('multivariate');
    expect(result.flags[0].variations).toHaveLength(3);
    expect(result.flags[0].variations?.[0].name).toBe('LIGHT');
    expect(result.flags[0].variations?.[0].value).toBe('light');
  });

  it('should parse flag definitions with context schema', () => {
    const yaml = `
context:
  user:
    age: number
    department: string
flags:
  - name: test_flag
    type: boolean
    defaultValue: false
`;

    const result = parseDefinitionsFromString(yaml, 'test.yaml');

    expect(result).toBeDefined();
    expect(result.context).toBeDefined();
    expect(result.context?.user).toBeDefined();
  });

  it('should throw ParseError for invalid YAML', () => {
    const invalidYaml = `
flags:
  - name: test
    type: boolean
    invalid: [unclosed
`;

    expect(() => {
      parseDefinitionsFromString(invalidYaml, 'test.yaml');
    }).toThrow(ParseError);
  });

  it('should throw ParseError for invalid JSON', () => {
    const invalidJson = '{"flags": [{"name": "test"';

    expect(() => {
      parseDefinitionsFromString(invalidJson, 'test.json');
    }).toThrow(ParseError);
  });

  it('should throw ParseError when flags field is missing', () => {
    const yaml = `
other_field: value
`;

    expect(() => {
      parseDefinitionsFromString(yaml, 'test.yaml');
    }).toThrow(ParseError);
  });

  it('should throw ParseError when flags is not an array', () => {
    const yaml = `
flags: not_an_array
`;

    expect(() => {
      parseDefinitionsFromString(yaml, 'test.yaml');
    }).toThrow(ParseError);
  });

  it('should parse from file', () => {
    // Create a temporary file
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'controlpath-test-'));
    const testFile = path.join(tmpDir, 'test.yaml');

    const yaml = `
flags:
  - name: test_flag
    type: boolean
    defaultValue: false
`;

    fs.writeFileSync(testFile, yaml);

    try {
      const result = parseDefinitions(testFile);
      expect(result).toBeDefined();
      expect(result.flags).toHaveLength(1);
      expect(result.flags[0].name).toBe('test_flag');
    } finally {
      fs.unlinkSync(testFile);
      fs.rmdirSync(tmpDir);
    }
  });

  it('should throw ParseError for non-existent file', () => {
    expect(() => {
      parseDefinitions('/nonexistent/file.yaml');
    }).toThrow(ParseError);
  });
});
