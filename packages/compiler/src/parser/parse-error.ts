/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * Error thrown when parsing fails.
 * Includes file path and optional cause for better error reporting.
 */
export class ParseError extends Error {
  /**
   * Create a new ParseError.
   *
   * @param message - Error message
   * @param filePath - Path to the file that caused the error
   * @param cause - Optional underlying error
   */
  constructor(
    message: string,
    public readonly filePath: string,
    public readonly cause?: Error
  ) {
    super(message);
    this.name = 'ParseError';
  }
}
