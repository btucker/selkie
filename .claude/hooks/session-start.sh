#!/bin/bash

# .claude/hooks/session-start.sh
echo "Setting up mb (microbeads issue tracker)..."

# Install microbeads via uv if not already available
if ! command -v mb &> /dev/null; then
    if command -v uv &> /dev/null; then
        uv tool install microbeads --quiet
        echo "Installed via uv"
    else
        echo "Error: uv is required to install microbeads"
        exit 1
    fi
fi

# Verify and show version
mb --version
