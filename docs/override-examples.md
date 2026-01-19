# Override Use Cases and Examples

This guide provides real-world examples and use cases for override files (kill switches) in Control Path.

## Use Cases

### 1. Emergency Kill Switch

**Scenario:** A new feature is causing crashes in production. You need to disable it immediately without redeploying.

**Solution:**

```bash
# 1. Set override to disable feature
controlpath override set new_dashboard OFF \
  --file overrides.json \
  --reason "Emergency kill switch - bug causing crashes" \
  --operator "alice@example.com"

# 2. Upload to CDN
aws s3 cp overrides.json s3://flags-bucket/overrides.json

# 3. SDK automatically loads and applies (within 3 seconds)
```

**SDK Configuration:**

```typescript
const provider = new Provider({
  overrideUrl: 'https://flags.example.com/overrides.json',
  // Polling starts automatically, checks every 3 seconds
});

await provider.loadArtifact('./flags/production.ast');

// Override takes effect immediately (within polling interval)
const result = provider.resolveBooleanEvaluation('new_dashboard', false, context);
// result.value = false (override wins)
```

**Benefits:**
- No code deployment needed
- Takes effect within seconds (polling interval)
- Complete audit trail (timestamp, reason, operator)

### 2. Gradual Rollout with Overrides

**Scenario:** You want to gradually roll out a new API version, but the rollout rules in the deployment file are too complex. Use overrides for quick adjustments.

**Solution:**

```bash
# 1. Start with small percentage (via deployment file)
# 2. Use override to quickly adjust rollout

# Increase rollout to 50%
controlpath override set api_version V2 \
  --file overrides.json \
  --reason "Increase rollout to 50%" \
  --operator "bob@example.com"

# Upload
aws s3 cp overrides.json s3://flags-bucket/overrides.json

# Later, increase to 100%
# (Update override file or clear to use deployment rules)
```

**SDK Configuration:**

```typescript
const provider = new Provider({
  overrideUrl: 'https://flags.example.com/overrides.json',
});

await provider.loadArtifact('./flags/production.ast');

// Override forces V2 for all users (bypasses rollout rules)
const result = provider.resolveStringEvaluation('api_version', 'V1', context);
// result.value = 'V2' (override wins)
```

**Benefits:**
- Quick adjustments without redeploying
- Bypass complex rollout rules when needed
- Easy to revert (clear override)

### 3. Multi-Environment Override Management

**Scenario:** Different environments need different override files (production, staging, development).

**Solution:**

```bash
# Production overrides
controlpath override set new_dashboard OFF \
  --file production-overrides.json \
  --reason "Production kill switch"

# Staging overrides
controlpath override set new_dashboard ON \
  --file staging-overrides.json \
  --reason "Staging test"

# Development (local file)
controlpath override set new_dashboard ON \
  --file dev-overrides.json \
  --reason "Local development"
```

**SDK Configuration:**

```typescript
// Production
const prodProvider = new Provider({
  overrideUrl: 'https://flags.example.com/production/overrides.json',
});

// Staging
const stagingProvider = new Provider({
  overrideUrl: 'https://flags.example.com/staging/overrides.json',
});

// Development
const devProvider = new Provider({
  overrideUrl: './dev-overrides.json', // Local file
});
```

**Benefits:**
- Separate override files per environment
- Different kill switches per environment
- Easy to manage per-environment overrides

### 4. Database Connection Switching

**Scenario:** Need to quickly switch database connections (PRIMARY/REPLICA) during maintenance or incidents.

**Solution:**

```bash
# Switch to replica
controlpath override set database_connection REPLICA \
  --file overrides.json \
  --reason "Maintenance on primary database" \
  --operator "charlie@example.com"

# Upload
aws s3 cp overrides.json s3://flags-bucket/overrides.json

# Later, switch back to primary
controlpath override set database_connection PRIMARY \
  --file overrides.json \
  --reason "Maintenance complete" \
  --operator "charlie@example.com"

aws s3 cp overrides.json s3://flags-bucket/overrides.json
```

**SDK Configuration:**

```typescript
const provider = new Provider({
  overrideUrl: 'https://flags.example.com/overrides.json',
});

await provider.loadArtifact('./flags/production.ast');

const result = provider.resolveStringEvaluation('database_connection', 'PRIMARY', context);
// result.value = 'REPLICA' (override wins)
```

**Benefits:**
- Quick database switching
- No application restart needed
- Complete audit trail

### 5. Feature Flag Testing

**Scenario:** QA team needs to test different flag combinations without changing deployment files.

**Solution:**

```bash
# Test scenario 1: New dashboard ON, API v2
controlpath override set new_dashboard ON \
  --file test-scenario-1.json \
  --reason "QA test scenario 1"
controlpath override set api_version V2 \
  --file test-scenario-1.json \
  --reason "QA test scenario 1"

# Test scenario 2: New dashboard OFF, API v1
controlpath override set new_dashboard OFF \
  --file test-scenario-2.json \
  --reason "QA test scenario 2"
controlpath override set api_version V1 \
  --file test-scenario-2.json \
  --reason "QA test scenario 2"
```

**SDK Configuration:**

```typescript
// Load different override files for different test scenarios
const testProvider = new Provider({
  overrideUrl: process.env.TEST_SCENARIO === '1' 
    ? 'https://flags.example.com/test-scenario-1.json'
    : 'https://flags.example.com/test-scenario-2.json',
});
```

**Benefits:**
- Quick test scenario switching
- No deployment file changes
- Easy to revert

## Storage Backend Examples

### Example 1: AWS S3 with CloudFront

**Setup:**

```bash
# 1. Create override file locally
controlpath override set new_dashboard OFF \
  --file overrides.json \
  --reason "Emergency kill switch"

# 2. Upload to S3
aws s3 cp overrides.json s3://flags-bucket/overrides.json \
  --content-type application/json \
  --cache-control "no-cache"

# 3. CloudFront automatically serves from S3
# URL: https://d1234abcd.cloudfront.net/overrides.json
```

**SDK Configuration:**

```typescript
const provider = new Provider({
  overrideUrl: 'https://d1234abcd.cloudfront.net/overrides.json',
  pollingInterval: 3000, // 3 seconds
});
```

**Benefits:**
- Fast global access (CloudFront CDN)
- HTTPS support
- S3 for storage, CloudFront for delivery

### Example 2: GitHub Raw URLs

**Setup:**

```bash
# 1. Create override file
controlpath override set new_dashboard OFF \
  --file overrides.json \
  --reason "Emergency kill switch"

# 2. Commit to Git
git add overrides.json
git commit -m "Emergency kill switch"
git push

# 3. Use GitHub raw URL
# URL: https://raw.githubusercontent.com/org/repo/main/overrides.json
```

**SDK Configuration:**

```typescript
const provider = new Provider({
  overrideUrl: 'https://raw.githubusercontent.com/org/repo/main/overrides.json',
  pollingInterval: 5000, // 5 seconds (GitHub may have rate limits)
});
```

**Benefits:**
- Simple setup (just Git)
- Version control built-in
- Free for public repos

**Limitations:**
- Updates require commit + push
- Rate limits on GitHub
- Not ideal for high-frequency updates

### Example 3: Local Files (Development)

**Setup:**

```bash
# 1. Create override file locally
controlpath override set new_dashboard ON \
  --file ./overrides.json \
  --reason "Local development"
```

**SDK Configuration:**

```typescript
const provider = new Provider({
  overrideUrl: './overrides.json', // Local file path
  // No polling for local files
});

// To see changes, reload or restart application
await provider.reloadOverrideFile();
```

**Benefits:**
- Simple for local development
- No network needed
- Fast updates (just edit file)

**Limitations:**
- No polling (manual reload needed)
- Not suitable for production

### Example 4: Custom Web Server

**Setup:**

```bash
# 1. Create override file
controlpath override set new_dashboard OFF \
  --file overrides.json \
  --reason "Emergency kill switch"

# 2. Upload to web server
scp overrides.json web-server:/var/www/flags/overrides.json

# 3. Web server serves at:
# URL: https://flags.example.com/overrides.json
```

**SDK Configuration:**

```typescript
const provider = new Provider({
  overrideUrl: 'https://flags.example.com/overrides.json',
  pollingInterval: 3000,
});
```

**Benefits:**
- Full control over server
- Network-level security (IP whitelisting, WAF rules)
- Easy to integrate with existing infrastructure

## Complete Workflow Example

**Scenario:** Emergency kill switch for a new feature causing issues in production.

**Step 1: Set Override Locally**

```bash
controlpath override set new_dashboard OFF \
  --file overrides.json \
  --reason "Emergency kill switch - bug causing crashes" \
  --operator "alice@example.com"
```

**Step 2: Verify Override**

```bash
controlpath override list --file overrides.json

# Output:
# Overrides:
# ────────────────────────────────────────────────────────────────
# Flag              Value    Timestamp              Operator
# ────────────────────────────────────────────────────────────────
# new_dashboard     OFF      2025-01-15T10:30:00Z   alice@example.com
```

**Step 3: Upload to Storage**

```bash
# Upload to S3
aws s3 cp overrides.json s3://flags-bucket/overrides.json \
  --content-type application/json

# Or upload to CDN
scp overrides.json cdn-server:/var/www/flags/overrides.json
```

**Step 4: SDK Automatically Loads**

```typescript
// SDK is already configured with override URL
const provider = new Provider({
  overrideUrl: 'https://flags.example.com/overrides.json',
});

await provider.loadArtifact('./flags/production.ast');

// Override takes effect within 3 seconds (polling interval)
const result = provider.resolveBooleanEvaluation('new_dashboard', false, context);
// result.value = false (override wins, feature disabled)
```

**Step 5: Monitor and Resolve**

```bash
# After fixing the bug, clear the override
controlpath override clear new_dashboard --file overrides.json

# Upload cleared file
aws s3 cp overrides.json s3://flags-bucket/overrides.json

# Feature re-enables automatically (uses AST evaluation)
```

## Best Practices

1. **Always include reason**: Helps with troubleshooting and audit trail
2. **Use operator**: Track who made the change
3. **Clear when done**: Remove overrides after resolving issues
4. **Version control**: Commit override files to Git for audit trail
5. **Monitor**: Watch override file access (CDN logs, S3 access logs)
6. **Test locally**: Test override changes locally before uploading
7. **Use HTTPS**: Always use HTTPS in production
8. **Set appropriate polling**: Balance between responsiveness and bandwidth

## Troubleshooting

### Override not taking effect

1. **Check URL**: Verify URL is accessible (try in browser)
2. **Check polling**: Ensure polling is enabled (HTTP/HTTPS URLs only)
3. **Check logs**: SDK logs warnings for loading errors
4. **Check flag name**: Ensure flag name matches exactly (case-sensitive)
5. **Check value format**: Boolean flags use `"ON"`/`"OFF"`, multivariate use variation name

### Override file not loading

1. **Check CORS**: If loading from different domain, ensure CORS headers are set
2. **Check format**: Verify JSON is valid (use `controlpath override list`)
3. **Check network**: Verify URL is accessible from your application
4. **Check logs**: SDK logs warnings for network errors (doesn't throw)

## Next Steps

- [Storage Setup Guide](./override-setup.md) - Set up override file storage
- [SDK Configuration Guide](./override-sdk-config.md) - Configure override URL in SDK
- [CLI Usage Guide](./override-cli-usage.md) - Manage override files with CLI
