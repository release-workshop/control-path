# Git Hooks

This directory contains git hooks for the Control Path repository. These hooks replace the previous Husky-based setup with standard git hooks that work with Rust tooling.

## Hooks

- **pre-commit**: Runs Rust checks (cargo check, clippy, fmt) and TypeScript build before commits
- **commit-msg**: Validates commit messages follow Conventional Commits format
- **pre-push**: Blocks direct pushes to main branch (use `git pushmain` instead)

## Installation

Run the setup script to install these hooks and configure git aliases:

```bash
bash scripts/setup-git-aliases.sh
```

This will:
- Install all git hooks (pre-commit, commit-msg, pre-push)
- Configure the `git pushmain` alias for trunk-based development

### Manual Installation

If you only want to install hooks without aliases:

```bash
cp .githooks/* .git/hooks/
chmod +x .git/hooks/*
```

## Manual Installation

If you prefer to install hooks manually:

```bash
# Copy hooks to .git/hooks
cp .githooks/pre-commit .git/hooks/
cp .githooks/commit-msg .git/hooks/
cp .githooks/pre-push .git/hooks/

# Make them executable
chmod +x .git/hooks/pre-commit
chmod +x .git/hooks/commit-msg
chmod +x .git/hooks/pre-push
```

## Commit Message Format

Commit messages must follow [Conventional Commits](https://www.conventionalcommits.org/) format:

```
type(scope): description
```

**Types**: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`

**Scopes** (optional): `compiler`, `cli`, `runtime`, `repo`, `ci`, `docs`, `deps`

**Examples**:
- `feat(compiler): add support for semver comparison`
- `fix(cli): handle missing flag definitions gracefully`
- `chore(ci): update GitHub Actions workflows`

