# graft

AST-aware merge driver for Git.

Standard Git merges line-by-line. If two branches add adjacent imports or append functions at the bottom of a file, Git reports a conflict even when the changes are syntactically compatible.

`graft` parses files using Tree-sitter to resolve these conflicts automatically.

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
- **Merge summary**: Prints which declarations were kept and basic line provenance.
- **Policy check**: Optional rule enforcement via `graft.policy.toml` (e.g. cross-import bans, line limits).
- **Pre-merge check (`graft radar`)**: Checks working tree diff against target branch for overlapping files.
- **Interactive resolver (`graft mergetool`)**: Terminal interface for unresolvable conflicts.
- **Local LLM fallback (`--ai`)**: Optional fallback using a local Ollama/OpenAI endpoint for divergent functions.

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

# Launch interactive resolver
git mergetool -t graft

# Merge with local AI fallback enabled
graft <base> <ours> <theirs> -p <path> --ai
```

## Configuration (`graft.policy.toml`)

Optional policy file placed in the repository root:

```toml
[rules]
max_function_lines = 120

[[rules.forbid_cross_import]]
from = "src/domain"
to = "src/infra"
```