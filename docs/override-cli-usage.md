# CLI Usage for Overrides

This guide explains how to use the Control Path CLI to manage override files (kill switches).

## Overview

The CLI manages override files **locally**. After editing, you upload the file to your preferred storage (CDN, S3, web server, etc.). The SDK then loads the override file from the URL you provide.

**Workflow:**
1. Edit override file locally using CLI
2. Upload to your storage (manual step)
3. SDK automatically loads from URL

## Commands

### Set Override

Set a flag override value:

```bash
controlpath override set <flag> <value> --file <path> [options]
```

**Examples:**

```bash
# Boolean flag: turn OFF
controlpath override set new_dashboard OFF \
  --file overrides.json \
  --reason "Emergency kill switch - bug causing crashes"

# Boolean flag: turn ON
controlpath override set new_dashboard ON \
  --file overrides.json \
  --reason "Re-enable after bug fix"

# Multivariate flag: set variation
controlpath override set api_version V1 \
  --file overrides.json \
  --reason "Rollback to v1 due to performance issues"

# With operator (for audit trail)
controlpath override set new_dashboard OFF \
  --file overrides.json \
  --reason "Emergency kill switch" \
  --operator "alice@example.com"
```

**Options:**
- `--file <path>`: Path to override file (required)
- `--reason <text>`: Reason for override (recommended for audit trail)
- `--operator <name>`: Operator identifier (e.g., email, username)
- `--definitions <path>`: Path to flag definitions file (optional, for validation)

**Boolean Values:**
- `ON`, `OFF` (case-insensitive)
- `true`, `false` (case-insensitive)
- `1`, `0`
- `yes`, `no` (case-insensitive)

All boolean values are normalized to `ON`/`OFF` format in the override file.

**Validation:**
- Validates against `override-file.schema.v1.json` schema
- Validates flag exists in definitions (if `--definitions` provided)
- Validates value is valid for flag type (if definitions provided)
- Warns if `--reason` is not provided (recommended for audit trail)

### Clear Override

Remove a flag from override file:

```bash
controlpath override clear <flag> --file <path>
```

**Examples:**

```bash
# Clear override (flag will use AST evaluation)
controlpath override clear new_dashboard --file overrides.json

# Clear override for multivariate flag
controlpath override clear api_version --file overrides.json
```

**Options:**
- `--file <path>`: Path to override file (required)

**Note:** If the override doesn't exist, the command shows an info message (doesn't error).

### List Overrides

Display all current overrides:

```bash
controlpath override list --file <path>
```

**Examples:**

```bash
# List all overrides
controlpath override list --file overrides.json

# Output:
# Overrides:
# ────────────────────────────────────────────────────────────────
# Flag              Value    Timestamp              Operator
# ────────────────────────────────────────────────────────────────
# new_dashboard     OFF      2025-01-15T10:30:00Z   alice@example.com
# api_version       V1       2025-01-15T10:30:00Z   bob@example.com
```

**Options:**
- `--file <path>`: Path to override file (required)

**Output:**
- Shows flag name, value, timestamp, operator, and reason (if available)
- Validates file format (warns if invalid)
- Handles empty override files gracefully

### View History

View override history (current overrides with audit trail):

```bash
controlpath override history [<flag>] --file <path>
```

**Examples:**

```bash
# View all override history
controlpath override history --file overrides.json

# Filter by flag name
controlpath override history new_dashboard --file overrides.json
```

**Options:**
- `--file <path>`: Path to override file (required)
- `<flag>`: Optional flag name to filter by

**Output:**
- Shows timestamp, flag, value, operator, and reason for each override
- Filters by flag name if provided
- Handles empty override files gracefully

**Note:** The override file itself serves as the audit trail. No separate history file is needed.

## File Format

The CLI automatically uses the **full format** (with timestamp, reason, operator) when setting overrides:

```json
{
  "version": "1.0",
  "overrides": {
    "new_dashboard": {
      "value": "OFF",
      "timestamp": "2025-01-15T10:30:00Z",
      "reason": "Emergency kill switch - bug causing crashes",
      "operator": "alice@example.com"
    }
  }
}
```

**Simple format** (for manual edits in emergencies):

```json
{
  "version": "1.0",
  "overrides": {
    "new_dashboard": "OFF"
  }
}
```

Both formats are supported. The CLI always uses the full format for audit trail.

## Upload Workflow

After editing override files locally, upload to your storage:

### CDN/Web Server

```bash
# Edit locally
controlpath override set new_dashboard OFF \
  --file overrides.json \
  --reason "Emergency kill switch"

# Upload to web server
scp overrides.json web-server:/var/www/flags/overrides.json

# Or use rsync
rsync -avz overrides.json web-server:/var/www/flags/
```

### S3

```bash
# Edit locally
controlpath override set new_dashboard OFF \
  --file overrides.json \
  --reason "Emergency kill switch"

# Upload to S3
aws s3 cp overrides.json s3://my-flags-overrides/overrides.json \
  --content-type application/json

# Or use AWS CLI with versioning
aws s3 cp overrides.json s3://my-flags-overrides/overrides.json \
  --content-type application/json \
  --metadata "reason=Emergency kill switch"
```

### GitHub

```bash
# Edit locally
controlpath override set new_dashboard OFF \
  --file overrides.json \
  --reason "Emergency kill switch"

# Commit and push
git add overrides.json
git commit -m "Emergency kill switch for new_dashboard"
git push
```

## Validation

The CLI validates override files against the schema:

```bash
# Set override (validates automatically)
controlpath override set new_dashboard OFF \
  --file overrides.json \
  --reason "Emergency kill switch"

# List overrides (validates file format)
controlpath override list --file overrides.json
```

**Validation checks:**
- Schema validation (structure, types, required fields)
- Flag existence (if `--definitions` provided)
- Value validity (if definitions provided)
- Format validation (JSON syntax, version compatibility)

**Error handling:**
- Clear error messages for validation failures
- Prevents invalid overrides from being written
- Warns if reason is not provided (recommended for audit trail)

## Best Practices

### 1. Always Include Reason

```bash
# Good: Includes reason for audit trail
controlpath override set new_dashboard OFF \
  --file overrides.json \
  --reason "Emergency kill switch - bug causing crashes"

# Warning: Missing reason (CLI warns)
controlpath override set new_dashboard OFF --file overrides.json
```

### 2. Use Operator for Audit Trail

```bash
# Good: Includes operator
controlpath override set new_dashboard OFF \
  --file overrides.json \
  --reason "Emergency kill switch" \
  --operator "alice@example.com"
```

### 3. Validate Before Upload

```bash
# List overrides to verify
controlpath override list --file overrides.json

# Check history
controlpath override history --file overrides.json

# Then upload
aws s3 cp overrides.json s3://bucket/overrides.json
```

### 4. Use Version Control

```bash
# Commit override files to Git for audit trail
git add overrides.json
git commit -m "Emergency kill switch for new_dashboard"
git push
```

### 5. Clear Overrides When Done

```bash
# Clear override when issue is resolved
controlpath override clear new_dashboard --file overrides.json

# Upload cleared file
aws s3 cp overrides.json s3://bucket/overrides.json
```

## Troubleshooting

### Override file not found

```bash
# Create empty override file if it doesn't exist
controlpath override set new_dashboard OFF \
  --file overrides.json \
  --reason "Emergency kill switch"
# File is created automatically if it doesn't exist
```

### Invalid override value

```bash
# CLI validates values if definitions file provided
controlpath override set new_dashboard INVALID \
  --file overrides.json \
  --definitions flags.definitions.yaml
# Error: Invalid value for boolean flag
```

### Schema validation error

```bash
# CLI validates against schema
controlpath override set new_dashboard OFF --file overrides.json
# Error: Schema validation failed (if file is malformed)
```

## Examples

### Emergency Kill Switch

```bash
# 1. Set override
controlpath override set new_dashboard OFF \
  --file overrides.json \
  --reason "Emergency kill switch - bug causing crashes" \
  --operator "alice@example.com"

# 2. Verify
controlpath override list --file overrides.json

# 3. Upload
aws s3 cp overrides.json s3://bucket/overrides.json

# 4. SDK automatically loads and applies override
```

### Gradual Rollout

```bash
# 1. Set override for gradual rollout
controlpath override set api_version V2 \
  --file overrides.json \
  --reason "Gradual rollout - 10% of users" \
  --operator "bob@example.com"

# 2. Upload
aws s3 cp overrides.json s3://bucket/overrides.json

# 3. Monitor, then increase rollout
# 4. Clear override when fully rolled out
controlpath override clear api_version --file overrides.json
aws s3 cp overrides.json s3://bucket/overrides.json
```

### Multi-Environment

```bash
# Production overrides
controlpath override set new_dashboard OFF \
  --file production-overrides.json \
  --reason "Production kill switch"

# Staging overrides
controlpath override set new_dashboard ON \
  --file staging-overrides.json \
  --reason "Staging test"

# Upload to different URLs
aws s3 cp production-overrides.json s3://bucket/production/overrides.json
aws s3 cp staging-overrides.json s3://bucket/staging/overrides.json
```

## Next Steps

- [Storage Setup Guide](./override-setup.md) - Set up override file storage
- [SDK Configuration Guide](./override-sdk-config.md) - Configure override URL in SDK
- [Use Cases and Examples](./override-examples.md) - Real-world scenarios
