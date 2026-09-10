# graft

AST-aware syntactic merge driver for Git (TypeScript / JavaScript).

Resolves import conflicts and top-level function/class additions semantically using Tree-sitter. Falls back to standard diff3 if ambiguous.

## Install

```bash
cargo install --git https://github.com/idunnowutuwant/graft
```

## Setup

```bash
graft init          # Current repo
graft init --global # All repos
```
