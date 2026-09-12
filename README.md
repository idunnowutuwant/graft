# graft

AST-aware merge driver for Git.

Standard Git merges line-by-line. If two branches add adjacent imports, update config arrays, or append functions, Git reports a conflict even when the changes are syntactically compatible.

`graft` parses files using Tree-sitter to resolve these conflicts automatically while enforcing architectural boundaries, scanning for secret leaks, and tracking merge provenance.

## Supported Languages

- TypeScript / JavaScript (`.ts`, `.tsx`, `.js`, `.jsx`)
- Rust (`.rs`)
- Python (`.py`)
- Go (`.go`)
- Java (`.java`)
- C / C++ (`.c`, `.cc`, `.cpp`, `.h`, `.hpp`)
- JSON (`package.json`, `.json`)

## Features

- **AST 3-way merge**: Merges imports and top-level declarations without line-order conflicts.
- **Merge summary & provenance**: Prints kept declarations and calculates line contributions.
- **Security & leak scanning**: Blocks merges that introduce high-entropy API keys or credentials.
- **Time-travel rollback (`graft undo`)**: Restores files to their pre-merge state from local journals.
- **Conflict fixture generator (`--repro`)**: Automatically generates standalone reproduction fixtures when a conflict occurs.
- **Policy enforcement**: Enforces rules defined in `graft.policy.toml` (e.g. cross-import bans, function length limits).
- **Pre-merge check (`graft radar`)**: Inspects repository state against target branch for overlapping hotspots.
- **Interactive resolver (`graft mergetool`)**: Terminal interface for unresolvable conflicts.
- **Local LLM fallback (`--ai`)**: Context-sliced fallback using a local Ollama/OpenAI endpoint for divergent functions.

## Install

```bash
cargo install --git https://github.com/idunnowutuwant/graft
```

## Setup

Set `graft` as your merge driver:

```bash
# Current repository
graft init

# Global configuration
graft init --global
```

Check configuration:

```bash
graft doctor
```

## Commands

```bash
# Check potential conflicts with target branch
graft radar main

# Revert file to pre-merge state
graft undo <path>

# View merge journal history
graft log

# Launch interactive resolver
git mergetool -t graft

# Merge with reproduction fixture generation on conflict
graft <base> <ours> <theirs> -p <path> --repro

# Merge with local AI fallback enabled
graft <base> <ours> <theirs> -p <path> --ai
```

## Configuration (`graft.policy.toml`)

Optional policy file placed in the repository root:

```toml
[rules]
max_function_lines = 120
block_leaked_secrets = true

[[rules.forbid_cross_import]]
from = "src/domain"
to = "src/infra"
```

## Ecosystem

`graft` integrates subsystems inspired by:

- [chronicle](https://github.com/idunnowutuwant/chronicle) - Time-travel journal and storage architecture
- [leakguard](https://github.com/idunnowutuwant/leakguard) - AST-guided secret and entropy scanner
- [repro](https://github.com/idunnowutuwant/repro) - Automated conflict reproduction generator
- [trace-ctx](https://github.com/idunnowutuwant/trace-ctx) - Context window slicing and token scrubbing
- [git-state](https://github.com/idunnowutuwant/git-state) - Fast repository state inspection