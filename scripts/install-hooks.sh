#!/usr/bin/env sh
#
# Install git hooks for selkie development
#
# Usage: ./scripts/install-hooks.sh

set -e

REPO_ROOT="$(git rev-parse --show-toplevel)"
HOOKS_DIR="$REPO_ROOT/.git/hooks"

echo "Installing git hooks..."

# Install pre-commit hook
cat > "$HOOKS_DIR/pre-commit" << 'EOF'
#!/usr/bin/env sh
#
# Combined pre-commit hook: cargo checks + mb (microbeads) sync
#
# Runs cargo fmt and clippy to match CI, then syncs mb issues.

set -e

echo "Running pre-commit checks..."

# 1. Check formatting (matches CI: cargo fmt --check)
echo "  Checking formatting..."
if ! cargo fmt --check; then
    echo ""
    echo "ERROR: Code is not formatted. Run 'cargo fmt' to fix."
    exit 1
fi

# 2. Run clippy (matches CI: cargo clippy --features all-formats -- -D warnings)
echo "  Running clippy..."
if ! cargo clippy --features all-formats -- -D warnings; then
    echo ""
    echo "ERROR: Clippy found warnings/errors. Fix them before committing."
    exit 1
fi

# 3. Run mb hooks for issue tracking
if command -v mb >/dev/null 2>&1; then
    echo "  Syncing mb issues..."
    mb hooks run pre-commit "$@"
else
    echo "  Warning: mb not found, skipping issue sync"
fi

echo "Pre-commit checks passed!"
EOF

chmod +x "$HOOKS_DIR/pre-commit"

echo "Done! Pre-commit hook installed."
echo ""
echo "The hook will run on each commit:"
echo "  - cargo fmt --check"
echo "  - cargo clippy --features all-formats -- -D warnings"
echo "  - mb sync (if mb is installed)"
