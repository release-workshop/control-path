# Contributing to Control Path

Thank you for your interest in contributing to Control Path! This document provides guidelines and instructions for contributing.

## Code of Conduct

This project adheres to a Code of Conduct that all contributors are expected to follow. Please read [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) before contributing.

## Contributor License Agreement

**Important**: By contributing to Control Path, you agree that your contributions will be licensed under the same terms as the project, and you grant Release Workshop Ltd ownership of your contributions.

Please read and agree to our [Contributor License Agreement](CONTRIBUTOR_LICENSE_AGREEMENT.md) before submitting contributions.

### How to Agree to the CLA

When you submit a pull request, you are agreeing to the terms of the CLA. For your first contribution, you may be asked to explicitly confirm your agreement by:

- Adding a comment to your pull request stating "I agree to the Contributor License Agreement"
- Or signing the CLA through an automated service (if implemented)

## Getting Started

### Prerequisites

Before contributing, ensure you have:

- **Node.js 24 LTS** or higher
- **Deno** installed
- **pnpm 8+** installed
- Git configured

See the [README.md](README.md) for detailed setup instructions.

### Development Setup

1. Fork the repository
2. Clone your fork:
   ```bash
   git clone https://github.com/your-username/control-path.git
   cd control-path
   ```
3. Install dependencies:
   ```bash
   pnpm install
   ```
4. Build the project:
   ```bash
   turbo run build
   ```
5. Run tests:
   ```bash
   turbo run test
   ```

## Development Workflow

### Trunk-Based Development (Maintainers/Trusted Users)

For maintainers and trusted contributors, Control Path supports **trunk-based development**:

- Work directly on `main` locally
- Use `git pushmain` to push changes (appears as direct push to `main`, but goes through validation)
- CI automatically validates and merges into `main` on success
- This ensures `main` is always in a releasable state while maintaining fast iteration

**Setup**:

```bash
bash scripts/setup-git-aliases.sh
```

**Workflow**:

```bash
git checkout main
git pull --ff-only  # Stay up to date
# ... make changes and commit directly on main ...
git commit -m "feat(compiler): add new feature"
git pushmain  # Validates and auto-merges into main on success
```

The `pushmain` command:

- Syncs your local `main` with `origin/main`
- Pushes to a temporary `validation/*` branch
- CI runs full validation (TIA, 100% diff coverage, lint, typecheck)
- On success, automatically merges into `main` (appears as if you pushed directly)

### Pull Request Workflow (Contributors)

For external contributors, use the standard **Pull Request** workflow:

1. Fork the repository and clone your fork
2. Create a feature branch:
   ```bash
   git checkout -b feature/your-feature-name
   ```
3. Make your changes and commit using [Conventional Commits](#commit-messages):
   ```bash
   git add .
   git commit -m "feat(compiler): add new feature"
   ```
4. Push your branch and create a PR:
   ```bash
   git push origin feature/your-feature-name
   # Then create a PR via GitHub UI
   ```
5. Once CI passes, a maintainer will review and merge your PR

**Note**: Direct commits to `main` are not allowed (enforced by branch protection). Only maintainers can use `pushmain` for trunk-based development.

### Code Style

- Follow the existing code style
- Use TypeScript strict mode
- Run `pnpm format` to format code before committing
- Ensure all lint checks pass

### Commit Messages

Control Path uses **Conventional Commits** format. Commit messages are enforced by `commitlint` (both locally via Husky and in CI).

**Format**: `type(scope): summary`

**Types**:

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `refactor`: Code refactoring
- `test`: Test changes
- `chore`: Maintenance tasks (CI, build, deps)
- `build`: Build system changes
- `ci`: CI configuration changes

**Scopes** (optional but recommended):

- `compiler`: Changes to `crates/compiler`
- `cli`: Changes to `crates/cli`
- `runtime`: Changes to `runtime/` packages
- `repo`: Root-level config, workflows, docs
- `ci`: CI/CD changes
- `docs`: Documentation changes
- `deps`: Dependency updates

**Examples**:

```
feat(compiler): add support for semver comparison in expressions
feat(runtime): add support for hot reloading

Implements SEMVER_EQ, SEMVER_GT, SEMVER_GTE, SEMVER_LT, and SEMVER_LTE
functions in the expression engine.

Fixes #123
```

```
fix(cli): handle missing flag definitions gracefully

Previously, the CLI would crash when encountering missing flag definitions.
Now it logs a warning and continues processing.

Closes #456
```

```
chore(ci): update GitHub Actions workflows for merge queues
```

## Submitting Changes

### Pull Request Process

1. Update the README.md with details of changes if applicable
2. Update documentation if you're changing functionality
3. Ensure your code follows the project's code style
4. Ensure all tests pass
5. Add tests for new functionality
6. Create a pull request with a clear description

### Pull Request Guidelines

- **Title**: Clear, descriptive title
- **Description**: Explain what changes you made and why
- **Tests**: Include tests for new features or bug fixes
- **Documentation**: Update relevant documentation
- **Breaking Changes**: Clearly mark any breaking changes

### Review Process

- All pull requests require review before merging
- Address review comments promptly
- Be open to feedback and suggestions
- Maintain a professional and respectful tone

## Project Structure

```
control-path/
├── packages/
│   ├── cli/          # CLI tool (Deno runtime)
│   ├── compiler/     # AST compiler (Node.js runtime)
│   └── runtime/      # Runtime SDKs (future)
├── schemas/          # JSON schemas
└── scripts/          # Build and utility scripts
```

## Licensing

For information about licensing, see:

- [LICENSE](LICENSE) - Elastic License 2.0 (applies to entire repository)
- [LICENSE-LAYER1.md](LICENSE-LAYER1.md) - Additional licensing information for Layer 1 Runtime SDK

## Areas for Contribution

We welcome contributions in many areas:

- **Bug fixes**: Fix issues reported in GitHub Issues
- **Features**: Implement features from the roadmap
- **Documentation**: Improve documentation and examples
- **Tests**: Add test coverage
- **Performance**: Optimize code and improve performance
- **Examples**: Add example configurations and use cases

## Questions?

If you have questions about contributing:

- Check existing [GitHub Issues](https://github.com/your-org/control-path/issues)
- Review the [Architecture documentation](../control-path-next/ARCHITECTURE.md)
- Ask in discussions or create an issue

## Security Issues

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, please report them via our [Security Policy](SECURITY.md).

---

Thank you for contributing to Control Path!
