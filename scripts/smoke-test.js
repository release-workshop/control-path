#!/usr/bin/env node

/**
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

/**
 * Simple production smoke test script.
 *
 * Expects:
 *   - PROD_BASE_URL: base URL of the production environment (e.g. https://app.example.com)
 *   - Optional: PROD_HEALTH_PATH: health endpoint path (default: /healthz)
 *
 * The script:
 *   - Performs a GET request to `${PROD_BASE_URL}${PROD_HEALTH_PATH}`
 *   - Fails (exit code 1) on network error or non-2xx status
 */

async function main() {
  const baseUrl = process.env.PROD_BASE_URL;
  const healthPath = process.env.PROD_HEALTH_PATH || '/healthz';

  if (!baseUrl) {
    console.log(
      'PROD_BASE_URL is not set; skipping smoke test. Configure PROD_BASE_URL and PROD_HEALTH_PATH to enable.'
    );
    return;
  }

  const url = new URL(healthPath, baseUrl).toString();
  console.log(`Running production smoke test against ${url}`);

  try {
    // Node 18+ provides a global fetch implementation
    const response = await fetch(url, { method: 'GET' });
    if (!response.ok) {
      console.error(`❌ Smoke test failed: HTTP ${response.status} ${response.statusText}`);
      process.exit(1);
    }

    console.log('✅ Smoke test passed.');
  } catch (err) {
    console.error('❌ Smoke test failed with error:', err);
    process.exit(1);
  }
}

if (require.main === module) {
  // eslint-disable-next-line unicorn/prefer-top-level-await
  main();
}


