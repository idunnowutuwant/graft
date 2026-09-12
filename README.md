# graft

[![CI](https://github.com/idunnowutuwant/graft/actions/workflows/ci.yml/badge.svg)](https://github.com/idunnowutuwant/graft/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2021-orange.svg)](Cargo.toml)

AST-aware merge driver for Git.

Standard Git merges line-by-line. If two branches add adjacent imports, update dependency arrays, or append functions at the bottom of a file, Git reports a conflict even when the changes are syntactically compatible.

`graft` parses files using Tree-sitter to resolve these conflicts automatically while enforcing architectural rules, scanning for secret leaks, and tracking merge provenance.

## Benchmark (100 Concurrent Edge Cases)

| Engine | Conflicts Encountered | Auto-Resolved | Average Latency |
| :--- | :---: | :---: | :---: |
| **Standard Git (diff3)** | 100 / 100 (100%) | 0 / 100 (0%) | - |
| **Graft Engine** | **0 / 100 (0%)** | **100 / 100 (100%)** | **0.882 ms** |

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
- **Merge summary & provenance**: Reports kept declarations and line contribution ratios.
- **Secret leak scanning**: Blocks merges that introduce high-entropy API keys or credentials.
- **Time-travel rollback (`graft undo`)**: Reverts merged files to their pre-merge state from local journals.
- **Conflict fixture generator (`--repro`)**: Generates standalone reproduction fixtures when a conflict occurs.
- **Policy enforcement**: Validates rules in `graft.policy.toml` (e.g. cross-import bans, function length limits).
- **Pre-merge radar (`graft radar`)**: Inspects repository state against target branch for overlapping files.
- **Blast radius analysis (`graft impact`)**: Calculates downstream files affected by changes to a file.
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
# Run local 100-scenario benchmark suite
graft bench

# Check potential conflicts with target branch
graft radar main

# Calculate downstream blast radius of a file
graft impact src/index.ts

# Install git lifecycle hooks
graft hook install

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