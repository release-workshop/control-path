/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect } from 'vitest';
import { ParseError } from './parse-error';

describe('ParseError', () => {
  it('should create a ParseError with message and filePath', () => {
    const error = new ParseError('Test error message', 'test.yaml');

    expect(error).toBeInstanceOf(Error);
    expect(error).toBeInstanceOf(ParseError);
    expect(error.message).toBe('Test error message');
    expect(error.filePath).toBe('test.yaml');
    expect(error.name).toBe('ParseError');
    expect(error.cause).toBeUndefined();
  });

  it('should create a ParseError with cause', () => {
    const cause = new Error('Original error');
    const error = new ParseError('Wrapped error', 'test.yaml', cause);

    expect(error.message).toBe('Wrapped error');
    expect(error.filePath).toBe('test.yaml');
    expect(error.cause).toBe(cause);
  });

  it('should be throwable and catchable', () => {
    expect(() => {
      throw new ParseError('Test error', 'test.yaml');
    }).toThrow(ParseError);

    try {
      throw new ParseError('Test error', 'test.yaml');
    } catch (error) {
      expect(error).toBeInstanceOf(ParseError);
      if (error instanceof ParseError) {
        expect(error.message).toBe('Test error');
        expect(error.filePath).toBe('test.yaml');
      }
    }
  });

  it('should preserve error properties when thrown', () => {
    const cause = new Error('Original error');
    let caughtError: ParseError | undefined;

    try {
      throw new ParseError('Wrapped error', 'test.yaml', cause);
    } catch (error) {
      caughtError = error as ParseError;
    }

    expect(caughtError).toBeDefined();
    expect(caughtError?.message).toBe('Wrapped error');
    expect(caughtError?.filePath).toBe('test.yaml');
    expect(caughtError?.cause).toBe(cause);
  });

  it('should have filePath and cause properties accessible', () => {
    const error = new ParseError('Test', 'test.yaml');

    // Verify the properties exist and are accessible
    expect(error.filePath).toBe('test.yaml');
    expect(error.cause).toBeUndefined();

    const errorWithCause = new ParseError('Test', 'test.yaml', new Error('Cause'));
    expect(errorWithCause.cause).toBeInstanceOf(Error);
    expect(errorWithCause.cause?.message).toBe('Cause');
  });
});
