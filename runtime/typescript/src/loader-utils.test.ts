/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  buildConditionalGetHeaders,
  fetchWithRedirects,
  validateFilePath,
  validateHttpUrl,
} from './loader-utils';

describe('loader-utils', () => {
  describe('validateFilePath', () => {
    it('rejects empty paths', () => {
      expect(() => validateFilePath('')).toThrow('File path is required');
    });

    it('rejects path traversal', () => {
      expect(() => validateFilePath('../secrets.ast')).toThrow('Path traversal detected');
    });
  });

  describe('validateHttpUrl', () => {
    it('rejects unsupported protocols', () => {
      expect(() => validateHttpUrl('file:///tmp/rules.ast')).toThrow('Unsupported URL protocol');
    });

    it('accepts https URLs', () => {
      expect(() => validateHttpUrl('https://flags.example.com/rules.ast')).not.toThrow();
    });
  });

  describe('buildConditionalGetHeaders', () => {
    it('returns empty headers when etag is omitted', () => {
      expect(buildConditionalGetHeaders()).toEqual({});
    });

    it('sends If-None-Match when etag is provided', () => {
      expect(buildConditionalGetHeaders('"v1"')).toEqual({
        'If-None-Match': '"v1"',
      });
    });
  });

  describe('fetchWithRedirects', () => {
    const originalFetch = globalThis.fetch;

    beforeEach(() => {
      vi.stubGlobal('fetch', vi.fn());
    });

    afterEach(() => {
      vi.stubGlobal('fetch', originalFetch);
    });

    it('follows redirects manually and returns the final response', async () => {
      const fetchMock = vi.mocked(globalThis.fetch);
      fetchMock
        .mockResolvedValueOnce(
          new Response(null, {
            status: 302,
            headers: { location: 'https://cdn.example.com/rules.ast' },
          })
        )
        .mockResolvedValueOnce(new Response('ok', { status: 200 }));

      const response = await fetchWithRedirects({
        url: 'https://flags.example.com/rules.ast',
        effectiveTimeoutMs: 5000,
        maxRedirects: 5,
        headers: buildConditionalGetHeaders('"v1"'),
      });

      expect(response.status).toBe(200);
      expect(fetchMock).toHaveBeenNthCalledWith(
        2,
        'https://cdn.example.com/rules.ast',
        expect.objectContaining({
          redirect: 'manual',
          headers: { 'If-None-Match': '"v1"' },
        })
      );
    });

    it('rejects redirect targets that use an unsupported protocol', async () => {
      const fetchMock = vi.mocked(globalThis.fetch);
      fetchMock.mockResolvedValueOnce(
        new Response(null, {
          status: 302,
          headers: { location: 'file:///tmp/rules.ast' },
        })
      );

      await expect(
        fetchWithRedirects({
          url: 'https://flags.example.com/rules.ast',
          effectiveTimeoutMs: 5000,
          maxRedirects: 5,
        })
      ).rejects.toThrow('Unsupported URL protocol');
    });
  });
});
