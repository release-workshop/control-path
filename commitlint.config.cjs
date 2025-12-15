module.exports = {
  extends: ['@commitlint/config-conventional'],
  rules: {
    'scope-enum': [
      2,
      'always',
      [
        'compiler',
        'cli',
        'repo',
        'ci',
        'docs',
        'deps',
      ],
    ],
  },
};


