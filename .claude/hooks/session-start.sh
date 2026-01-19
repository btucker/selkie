#!/bin/bash
# .claude/hooks/session-start.sh

# Install bd globally (only takes a few seconds)
echo "Installing bd (beads issue tracker)..."
npm install -g @beads/bd

# Initialize bd in the project (if not already initialized)
if [ ! -d .beads ]; then
  bd init --quiet
fi

echo "✓ bd is ready! Use 'bd ready' to see available work."
