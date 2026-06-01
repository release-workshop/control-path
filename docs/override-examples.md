# Kill Switch Examples

Real-world patterns for boolean kill switches in Control Path.

## Emergency Rollback

```bash
# Disable feature immediately in production
controlpath kill-switch set new_checkout false --env production

# Upload .controlpath/production.kill-switches.json to CDN

# After fix, clear kill switch to restore AST rules
controlpath kill-switch clear new_checkout --env production
```

## Staging Verification

```bash
# Force-enable on staging for QA
controlpath kill-switch set beta_feature true --env staging

# List current state
controlpath kill-switch list --env staging
```

## Per-Environment Files

| Environment | Local file | Typical URL |
|-------------|------------|-------------|
| production | `.controlpath/production.kill-switches.json` | `https://flags.example.com/production.kill-switches.json` |
| staging | `.controlpath/staging.kill-switches.json` | `https://flags.example.com/staging.kill-switches.json` |

## CI/CD

Validate catalog and compile ASTs in CI; manage kill switches as a separate operational step:

```bash
controlpath ci --env production
# Kill switches are not modified by ci — set them explicitly when needed
controlpath kill-switch set risky_feature false --env production
```

## See Also

- [CLI Usage Guide](./override-cli-usage.md)
- [Storage Setup Guide](./override-setup.md)
- [SDK Configuration](./override-sdk-config.md)
