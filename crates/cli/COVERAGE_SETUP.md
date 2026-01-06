# Coverage Setup Guide

## Quick Start

### macOS

If you encounter OpenSSL errors when installing `cargo-tarpaulin`:

```bash
# Install required dependencies
brew install pkg-config openssl

# Set OpenSSL directory
export OPENSSL_DIR=$(brew --prefix openssl)

# Install cargo-tarpaulin
cargo install cargo-tarpaulin
```

### Linux (Ubuntu/Debian)

```bash
sudo apt-get update
sudo apt-get install -y libssl-dev pkg-config
cargo install cargo-tarpaulin
```

### Linux (Fedora/RHEL)

```bash
sudo yum install openssl-devel pkg-config
cargo install cargo-tarpaulin
```

## Alternative: cargo-llvm-cov

If you prefer not to deal with OpenSSL dependencies, use `cargo-llvm-cov`:

```bash
cargo install cargo-llvm-cov
cargo llvm-cov --all-features --workspace
```

## Troubleshooting

### OpenSSL Not Found

**Error**: `Could not find directory of OpenSSL installation`

**Solution**:
1. Install OpenSSL via your package manager
2. Set `OPENSSL_DIR` environment variable:
   ```bash
   export OPENSSL_DIR=$(brew --prefix openssl)  # macOS
   export OPENSSL_DIR=/usr  # Linux (if installed system-wide)
   ```

### pkg-config Not Found

**Error**: `The pkg-config command could not be found`

**Solution**:
```bash
# macOS
brew install pkg-config

# Linux
sudo apt-get install pkg-config  # Ubuntu/Debian
sudo yum install pkg-config      # Fedora/RHEL
```

## CI/CD

Coverage is automatically calculated in GitHub Actions - no local setup required. See `.github/workflows/coverage.yml` for details.

