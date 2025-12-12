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

### Making Changes

1. Create a new branch from `main`:
   ```bash
   git checkout -b feature/your-feature-name
   ```
2. Make your changes
3. Ensure code quality:
   ```bash
   turbo run lint
   turbo run format:check
   ```
4. Run tests:
   ```bash
   turbo run test
   ```
5. Build to verify:
   ```bash
   turbo run build
   ```

### Code Style

- Follow the existing code style
- Use TypeScript strict mode
- Run `pnpm format` to format code before committing
- Ensure all lint checks pass

### Commit Messages

Write clear, descriptive commit messages:

- Use the present tense ("Add feature" not "Added feature")
- Use the imperative mood ("Move cursor to..." not "Moves cursor to...")
- Limit the first line to 72 characters or less
- Reference issues and pull requests liberally after the first line

Example:
```
Add support for semver comparison in expressions

Implements SEMVER_EQ, SEMVER_GT, SEMVER_GTE, SEMVER_LT, and SEMVER_LTE
functions in the expression engine.

Fixes #123
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

