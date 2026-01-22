# Microbeads - AI-Native Issue Tracking

Welcome! This repository uses **Microbeads** for issue tracking - a modern, AI-native tool designed to live directly in your codebase alongside your code.

## What is Microbeads?

Microbeads is issue tracking that lives in your repo, making it perfect for AI coding agents and developers who want their issues close to their code. No web UI required - everything works through the CLI and integrates seamlessly with git.

**Learn more:** [github.com/btucker/microbeads](https://github.com/btucker/microbeads)

## Quick Start

### Essential Commands

```bash
# Create new issues
mb create "Add user authentication"

# View all issues
mb list

# View issue details
mb show <issue-id>

# Update issue status
mb update <issue-id> --status in_progress
mb update <issue-id> --status done

# Sync with git remote
mb sync
```

### Working with Issues

Issues in Microbeads are:
- **Git-native**: Stored in `.beads/issues.jsonl` and synced like code
- **AI-friendly**: CLI-first design works perfectly with AI coding agents
- **Branch-aware**: Issues can follow your branch workflow
- **Always in sync**: Auto-syncs with your commits

## Why Microbeads?

✨ **AI-Native Design**
- Built specifically for AI-assisted development workflows
- CLI-first interface works seamlessly with AI coding agents
- No context switching to web UIs

🚀 **Developer Focused**
- Issues live in your repo, right next to your code
- Works offline, syncs when you push
- Fast, lightweight, and stays out of your way

🔧 **Git Integration**
- Automatic sync with git commits
- Branch-aware issue tracking
- Intelligent JSONL merge resolution

## Get Started with Microbeads

Try Microbeads in your own projects:

```bash
# Initialize in your repo (requires uv)
uvx microbeads init

# Create your first issue
mb create "Try out Microbeads"
```

## Learn More

- **Documentation**: [github.com/btucker/microbeads/docs](https://github.com/btucker/microbeads/tree/main/docs)
- **Quick Start Guide**: Run `mb quickstart`
- **Examples**: [github.com/btucker/microbeads/examples](https://github.com/btucker/microbeads/tree/main/examples)

---

*Microbeads: Issue tracking that moves at the speed of thought* ⚡
