module.exports = {
  extends: ['@commitlint/config-conventional'],
  rules: {
    // Validate scope enum when scope is present
    'scope-enum': [
      2,
      'always',
      (parsed) => {
        // Only validate scope enum if scope is present
        if (!parsed.scope) {
          return [true];
        }
        const allowedScopes = [
          'compiler', // Rust compiler crate
          'cli', // Rust CLI crate
          'runtime', // TypeScript runtime SDK
          'repo', // Root-level config, workflows, docs
          'ci', // CI/CD changes
          'docs', // Documentation changes
          'deps', // Dependency updates
        ];
        return [
          allowedScopes.includes(parsed.scope),
          `scope must be one of: ${allowedScopes.join(', ')}`,
        ];
      },
    ],
    // Allow empty scope for chore commits (used by release-please: "chore: release X.X.X")
    // For other types, scope is required
    'scope-empty': [
      2,
      'never',
      (parsed) => {
        // Allow empty scope for chore type (release-please uses "chore: release X.X.X")
        if (parsed.type === 'chore' && !parsed.scope) {
          return [true];
        }
        // For other types, scope is required
        return [parsed.scope ? true : false, 'scope must be provided'];
      },
    ],
  },
};

