# Ex Grapha

## Overview

A dependency-aware knowledge graph desktop app (Tauri) where nodes are markdown files and directed edges encode dependency relationships. Staleness propagates when upstream knowledge changes, forcing users to review and maintain chains of reasoning. The data layer is git-native (frontmatter + markdown, no built-in git integration).

## Tech Stack

- **Shell**: Tauri (Rust backend + webview frontend)
- **Backend (Rust)**: Data model, frontmatter parsing (serde_yaml), cycle detection, staleness propagation, validation engine, file watching (notify crate)
- **Frontend**: Svelte 5 + Vite + TypeScript, Svelte Flow (`@xyflow/svelte`) for the graph canvas. Visual reference: Liam ERD (port aesthetics only — Liam ERD itself is React-Flow-based)
- **Editor**: Milkdown (WYSIWYG) + CodeMirror 6 (raw markdown), with toggle switch

## Key Concepts

- **Nodes**: Atomic knowledge units stored as `nodes/n-XXXXXX.md` with YAML frontmatter. Two types: `axiom` (no deps) and `deduction` (has deps + relation expression).
- **Edges**: Directed dependency relationships stored in the dependent node's frontmatter.
- **Relation expressions**: Propositional logic expressions (`AND`, `OR`, `NOT`, `IMPLIES`, `IFF`) on deduction nodes describing how dependencies combine.
- **Staleness propagation**: Editing a node marks all downstream dependents as `stale` (transitively). Reviewing stops propagation; editing re-propagates.
- **Validation**: DAG enforcement, dangling ref detection, relation integrity, type constraints. Can optionally run as a git pre-commit hook and/or GitHub Actions workflow.

## Project Structure (Knowledge Base)

```
my-knowledge-base/
├── .knowledgebase/
│   ├── config.yaml              # Tags, display toggles
│   └── hooks/validate.sh       # (optional) Git pre-commit hook
├── nodes/                       # One .md file per node
├── assets/                      # Per-node asset directories
├── .github/workflows/validate.yaml  # (optional) GitHub Actions
└── README.md
```

## Current Milestone: Core Knowledge Graph

Precondition: Tauri app shell with embedded Milkdown/CodeMirror is already working.

### Issues

| #   | Title                                             | Labels                 | Status |
| --- | ------------------------------------------------- | ---------------------- | ------ |
| 1   | Data Model: Frontmatter Parsing & Serialization   | backend, foundation    | todo   |
| 2   | Project Scaffolding: Init, Open, Directory Layout | backend, foundation    | todo   |
| 3   | Node CRUD Operations                              | backend, core          | todo   |
| 4   | Edge CRUD Operations                              | backend, core          | todo   |
| 5   | Validation Engine                                 | backend, core          | todo   |
| 6   | Relation Expression Parser                        | backend, core          | todo   |
| 7   | Staleness Propagation Engine                      | backend, core          | todo   |
| 8   | File Watcher                                      | backend, core          | todo   |
| 9   | Graph Canvas: Rendering Nodes & Edges             | frontend, core         | todo   |
| 10  | Symbiotic Relation Nodes                          | frontend, core         | todo   |
| 11  | Node Interactions                                 | frontend, interaction  | todo   |
| 12  | Edge Interactions                                 | frontend, interaction  | todo   |
| 13  | Sidebar Editor Panel                              | frontend, core         | todo   |
| 14  | Relation Expression Editor                        | frontend, interaction  | todo   |
| 15  | Tracing: Upstream & Downstream Highlighting       | frontend, interaction  | todo   |
| 16  | Project Settings Panel                            | frontend, settings     | todo   |
| 17  | Undo/Redo Stack                                   | frontend, core         | todo   |
| 18  | Broken Dependency Detection & UI                  | backend+frontend, core | todo   |

### Issue Dependency Chain

```
#1 → #2 → #3 → #4 → #6 → #5
                #3 → #7
                #3 → #8
#9 (after #1-#4) → #10, #11 → #13, #12 → #14, #15, #18
#16 (after #2)
#17 (after #3+#4)
```

Backend #1-#8 and frontend #9-#18 can proceed in parallel once the data model (#1) is solid.

## Full Reference Docs

- **Core Feature Spec**: `.claude/docs/core-feature-spec.md`
- **Milestone Details**: `.claude/docs/milestone.md`

Read these when you need detailed acceptance criteria, file format specs, or interaction details.

## Conventions

- Node IDs: `n-XXXXXX` (6 hex chars, randomly generated, prefixed with `n-`)
- All mutations write to disk immediately (no in-memory-only state)
- The app does NOT interact with git — users manage version control externally
- Node positions are local-only (gitignored)
- Graph must always be a DAG — cycles are rejected at edge creation time
- **Tests live in dedicated files**, not inline with source code. Use `crates/<crate>/tests/` for integration tests with shared fixtures in `tests/common/mod.rs`. Never use `#[cfg(test)] mod tests` inside source files.
