# Kill Switch Storage Setup

This guide explains how to host kill switch files for Control Path runtime use.

## Overview

Kill switch files (`.controlpath/<env>.kill-switches.json`) allow boolean runtime overrides without redeploying code. The CLI edits files locally; the SDK polls a URL you provide.

Supported hosting: any URL-accessible location (CDN, S3, GitHub raw, web server).

## Basic Workflow

1. **Set kill switches locally:**

   ```bash
   controlpath kill-switch set new_dashboard false --env production
   ```

2. **Upload the file** from `.controlpath/production.kill-switches.json` to your storage.

3. **Configure the SDK** with the public URL (see [SDK Configuration](./override-sdk-config.md)).

## Multi-Environment Setup

Use one kill switch file per environment:

```
.controlpath/production.kill-switches.json
.controlpath/staging.kill-switches.json
```

Upload each to environment-specific URLs and configure the SDK accordingly.

## Recommendations

- Keep kill switch files in version control or an audit log if your process requires traceability
- Use `--reason` only as a local operator note today; it is not stored in v2 kill switch files
- Monitor access to hosted kill switch URLs (CDN/S3 logs)

## See Also

- [CLI Usage Guide](./override-cli-usage.md)
- [SDK Configuration](./override-sdk-config.md)
