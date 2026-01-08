module.exports = {
  extends: ['@commitlint/config-conventional'],
  rules: {
    // Allow empty scope for all commits (chore commits from release-please don't have scopes)
    'scope-empty': [0],
    // Validate scope enum when scope is present
    'scope-enum': [
      2,
      'always',
      [
        'compiler', // Rust compiler crate
        'cli', // Rust CLI crate
        'runtime', // TypeScript runtime SDK
        'repo', // Root-level config, workflows, docs
        'ci', // CI/CD changes
        'docs', // Documentation changes
        'deps', // Dependency updates
      ],
    ],
  },
};

