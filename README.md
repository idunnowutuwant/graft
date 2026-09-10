# graft

AST-aware syntactic merge driver for Git (TypeScript / JavaScript).

Standard line-based Git merge triggers unnecessary conflicts when branches modify adjacent imports or append functions. `graft` resolves these semantically using Tree-sitter.

## Visual Comparison

### 1. Concurrent Imports

Standard Git:
```diff
<<<<<<< HEAD
import { useState, useEffect } from "react";
=======
import { useState, useMemo } from "react";
>>>>>>> feature-b
```

With graft:
```typescript
import { useEffect, useMemo, useState } from "react";
```

---

### 2. Appending Top-Level Functions

Standard Git:
```diff
<<<<<<< HEAD
export function UserProfile() { ... }
=======
export function SettingsModal() { ... }
>>>>>>> feature-b
```

With graft:
```typescript
export function UserProfile() { ... }

export function SettingsModal() { ... }
```

## Install

```bash
cargo install --git https://github.com/idunnowutuwant/graft
```

## Setup

```bash
graft init          # Current repository
graft init --global # Global
```
