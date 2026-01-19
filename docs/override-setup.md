# Override File Storage Setup

This guide explains how to set up storage for override files (kill switches) in Control Path. Override files allow you to change flag values at runtime without redeploying code.

## Overview

Control Path supports override files from **any URL-accessible location**. You can use:
- **CDN/web server** (recommended for production)
- **S3 public URLs** (or any object storage with public HTTP access)
- **Local files** (for development)
- **GitHub raw URLs** (for simple setups)

The SDK automatically loads and polls override files from the URL you provide.

## Storage Options

### Option 1: CDN/Web Server (Recommended for Production)

Use a CDN or web server to host your override files. This provides:
- Fast global access
- HTTPS support
- Easy updates (just upload the file)
- No vendor lock-in

**Example Setup:**

1. **Upload override file to your web server:**
   ```bash
   # Create override file locally
   controlpath override set new_dashboard OFF \
     --file overrides.json \
     --reason "Emergency kill switch"
   
   # Upload to web server
   scp overrides.json web-server:/var/www/flags/overrides.json
   ```

2. **Configure SDK with URL:**
   ```typescript
   const provider = new Provider({
     overrideUrl: 'https://flags.example.com/overrides.json',
   });
   ```

**Popular CDN Options:**
- Cloudflare (free tier available)
- AWS CloudFront
- Fastly
- Any web server with HTTPS

### Option 2: S3 Public URLs

Use AWS S3 (or compatible object storage) with public read access:

**Setup Steps:**

1. **Create S3 bucket** (or use existing):
   ```bash
   aws s3 mb s3://my-flags-overrides
   ```

2. **Set bucket policy for public read:**
   ```json
   {
     "Version": "2012-10-17",
     "Statement": [
       {
         "Sid": "PublicReadGetObject",
         "Effect": "Allow",
         "Principal": "*",
         "Action": "s3:GetObject",
         "Resource": "arn:aws:s3:::my-flags-overrides/*"
       }
     ]
   }
   ```

   **Security Note:** For production, consider restricting access:
   - Use CloudFront in front of S3 (recommended)
   - Use VPC endpoints to restrict S3 access to your VPC
   - Use S3 bucket policies with IP restrictions (if you have static IPs)

3. **Upload override file:**
   ```bash
   # Create override file locally
   controlpath override set new_dashboard OFF \
     --file overrides.json \
     --reason "Emergency kill switch"
   
   # Upload to S3
   aws s3 cp overrides.json s3://my-flags-overrides/overrides.json \
     --content-type application/json
   ```

4. **Configure SDK with S3 URL:**
   ```typescript
   const provider = new Provider({
     overrideUrl: 'https://my-flags-overrides.s3.amazonaws.com/overrides.json',
   });
   ```

**Note:** For production, use CloudFront in front of S3 for better performance, HTTPS, and additional security options (IP restrictions, WAF rules).

### Option 3: Local Files (Development)

For local development, use file system paths:

**Setup:**

```typescript
// Node.js - direct file path
const provider = new Provider({
  overrideUrl: './overrides.json', // Relative to current working directory
});

// Or use file:// URL
const provider = new Provider({
  overrideUrl: 'file:///absolute/path/to/overrides.json',
});
```

**Note:** Local files are **not polled** (polling only works for HTTP/HTTPS URLs). To see changes, reload the artifact or restart your application.

### Option 4: GitHub Raw URLs

For simple setups, you can host override files in a GitHub repository:

**Setup:**

1. **Create override file in your repo:**
   ```bash
   controlpath override set new_dashboard OFF \
     --file overrides.json \
     --reason "Emergency kill switch"
   
   git add overrides.json
   git commit -m "Add override file"
   git push
   ```

2. **Configure SDK with GitHub raw URL:**
   ```typescript
   const provider = new Provider({
     overrideUrl: 'https://raw.githubusercontent.com/your-org/your-repo/main/overrides.json',
   });
   ```

**Note:** GitHub raw URLs work, but updates require a new commit and push. For production, consider a CDN or web server for faster updates.

## Upload Workflow

The typical workflow is:

1. **Edit override file locally** using CLI:
   ```bash
   controlpath override set <flag> <value> \
     --file overrides.json \
     --reason "Reason for override"
   ```

2. **Upload to your storage** (manual step):
   ```bash
   # CDN/Web server
   scp overrides.json web-server:/path/to/overrides.json
   
   # S3
   aws s3 cp overrides.json s3://bucket/overrides.json
   
   # Or use your preferred upload method
   ```

3. **SDK automatically loads** from URL (polling starts automatically)

## Security Considerations

### Access Control

**Important:** The Control Path SDK performs a simple HTTP GET request with **no authentication support**. Security must be handled at the storage/CDN level, not in the SDK.

**Security Options:**

1. **Public URLs with IP/Network Restrictions**
   - Use your CDN or web server's IP whitelisting
   - Restrict access to your application's IP addresses
   - Use VPC endpoints for AWS S3 (restrict to your VPC)

2. **CloudFront with Origin Access Control**
   - Use CloudFront in front of private S3 buckets
   - CloudFront URL is public, but S3 bucket remains private
   - Restrict CloudFront distribution to specific IPs if needed

3. **CDN/WAF Rules**
   - Use Cloudflare, AWS WAF, or similar to restrict access
   - Block requests from unauthorized IPs
   - Rate limiting to prevent abuse

4. **HTTPS Only**
   - Always use HTTPS in production (SDK supports both HTTP and HTTPS)
   - Prevents man-in-the-middle attacks
   - Use TLS certificates from trusted CAs

**Not Supported:**
- ❌ HTTP Basic Auth (SDK doesn't send credentials)
- ❌ API keys in query parameters (SDK doesn't add them)
- ❌ Signed URLs (expire and break polling)
- ❌ Custom headers for authentication (SDK doesn't add them)

**Best Practice:** Use public HTTPS URLs with network-level restrictions (IP whitelisting, VPC endpoints, WAF rules) rather than URL-based authentication.

## Best Practices

1. **Production**: Use CDN or web server with HTTPS
2. **Development**: Use local files (`file://` or direct paths)
3. **Staging**: Use same setup as production (different URL)
4. **Updates**: Upload override files manually (CLI edits locally)
5. **Monitoring**: Monitor override file access (CDN logs, S3 access logs)
6. **Backup**: Keep override files in version control (Git) for audit trail

## Multi-Environment Setup

For multiple environments, use different override files:

```typescript
// Production
const prodProvider = new Provider({
  overrideUrl: 'https://flags.example.com/production/overrides.json',
});

// Staging
const stagingProvider = new Provider({
  overrideUrl: 'https://flags.example.com/staging/overrides.json',
});
```

Or use environment variables:

```typescript
const provider = new Provider({
  overrideUrl: process.env.OVERRIDE_URL || './overrides.json',
});
```

## Troubleshooting

### Override file not loading

- **Check URL**: Verify the URL is accessible (try in browser)
- **Check CORS**: If loading from different domain, ensure CORS headers are set
- **Check format**: Verify JSON is valid (use `controlpath override list --file overrides.json`)
- **Check logs**: SDK logs warnings for loading errors (doesn't throw)

### Polling not working

- **HTTP/HTTPS only**: Polling only works for HTTP/HTTPS URLs (not `file://` or direct paths)
- **Check interval**: Default is 3 seconds (configurable via `pollingInterval`)
- **Check ETag**: SDK uses ETag for efficient updates (304 Not Modified)

### Override not taking effect

- **Check priority**: Override → AST → Default (override should take precedence)
- **Check flag name**: Ensure flag name matches exactly (case-sensitive)
- **Check value format**: Boolean flags use `"ON"`/`"OFF"`, multivariate use variation name
- **Check logs**: SDK logs warnings for invalid overrides (falls back to AST)

## Next Steps

- [SDK Configuration Guide](./override-sdk-config.md) - Configure override URL in SDK
- [CLI Usage Guide](./override-cli-usage.md) - Manage override files with CLI
- [Use Cases and Examples](./override-examples.md) - Real-world scenarios
