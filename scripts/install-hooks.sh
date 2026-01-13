#!/bin/sh
# Install git hooks for this repository

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Find the git hooks directory (handles worktrees)
GIT_DIR=$(git -C "$REPO_ROOT" rev-parse --git-dir)
HOOKS_DIR="$GIT_DIR/hooks"

echo "Installing pre-commit hook to $HOOKS_DIR..."

mkdir -p "$HOOKS_DIR"
cp "$SCRIPT_DIR/pre-commit" "$HOOKS_DIR/pre-commit"
chmod +x "$HOOKS_DIR/pre-commit"

echo "Done! Pre-commit hook installed."
echo "The hook will run 'cargo fmt --check' and 'cargo clippy' before each commit."
