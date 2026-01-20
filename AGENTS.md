# Agent Instructions

This project uses **bd** (beads) for issue tracking. Run `bd onboard` to get started.

## Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --status in_progress  # Claim work
bd close <id>         # Complete work
bd sync               # Sync with git
```

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds:

   ```bash
   cargo fmt                                        # Fix formatting
   cargo clippy --features all-formats -- -D warnings  # Lint (warnings = errors)
   cargo test --features all-formats               # Run tests
   ```

3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:

   ```bash
   git pull --rebase
   bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```

5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**

- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds

## IMPORTANT Development Rules

### The Five Rules

Rule 1: If the user ever points out a bug that was not caught by eval, before you fix it, first make updates to eval such that it detects the bug.

Rule 2: Always follow the instructions from eval output

Rule 3: Always follow TDD

Rule 4: Always review the relevant implementation code from the reference-implementations before starting work.

Rule 5: Always prefer the implementation approach of the reference-implementations.

### The process that MUST be followed

1. Use `cargo run --features eval --bin selkie -- eval --type <diagram_type>` evaluate where our implementation is relative to the reference.
2. Follow all instructions from its output & confirm our changes are increasing scores
3. Log new issues to log in bd & resolve completed ones
4. When you resolve a rendering issue, update the svg in docs/images
5. Follow TDD, run `cargo fmt && cargo clippy --features all-formats -- -D warnings` before committing, commit when tests pass
6. Explore Reference implementations available as git submodules in reference-implementations:
    - mermaid
    - dagre
    - elkjs
7. Return to step 1
