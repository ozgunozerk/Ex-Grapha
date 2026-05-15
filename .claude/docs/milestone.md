# Milestone: Core Knowledge Graph

> **Goal**: From a working Tauri + Milkdown/CodeMirror shell to a fully functional dependency-aware knowledge graph application.
> 
> **Precondition**: Tauri app shell with embedded Milkdown/CodeMirror markdown editor is already working.

---

## Issue #1 — Data Model: Frontmatter Parsing & Serialization (Rust)

**Labels**: `backend`, `foundation`

Implement the core Rust data structures and frontmatter parsing layer.

**Acceptance criteria**:

- Define Rust structs for `Node`, `Edge`, `StaleSource`, and `ProjectConfig`
- Parse a `.md` file with YAML frontmatter into a `Node` struct (using `serde_yaml` or similar)
- Serialize a `Node` struct back to a `.md` file preserving content after the frontmatter
- Handle both `axiom` and `deduction` variants with their respective constraints (axiom: empty deps/no relation, deduction: non-empty deps/valid relation)
- Parse `.knowledgebase/config.yaml` into a `ProjectConfig` struct
- Unit tests covering round-trip parse → serialize for both node types, edge cases (missing fields, malformed YAML)

---

## Issue #2 — Project Scaffolding: Init, Open, Directory Layout

**Labels**: `backend`, `foundation`

Support creating new knowledge base projects and opening existing ones.

**Acceptance criteria**:

- "New Project" command that scaffolds the directory layout: `nodes/`, `assets/`, `.knowledgebase/config.yaml` with sensible defaults, `README.md`, `.gitignore` (ignoring node position data). Optionally includes `.knowledgebase/hooks/validate.sh` (git pre-commit hook) and/or `.github/workflows/validate.yaml` (GitHub Actions) based on user choice
- "Open Project" command that reads an existing directory, validates the structure, and loads all nodes into memory
- Build the in-memory graph representation: a `HashMap<NodeId, Node>` plus an adjacency structure for edges
- Expose Tauri commands: `init_project(path)`, `open_project(path)`
- Error handling for malformed/missing files with clear error messages

---

## Issue #3 — Node CRUD Operations (Rust Backend)

**Labels**: `backend`, `core`

Implement create, read, update, and delete for nodes, writing to disk on every mutation.

**Acceptance criteria**:

- **Create**: Generate a random `n-XXXXXX` ID, create the `.md` file in `nodes/`, add to in-memory graph
- **Read**: Return full node data (frontmatter + content) by ID
- **Update**: Modify any frontmatter field or content body, write to disk immediately
- **Delete**: Remove a node file only if no other nodes depend on it; return an error listing dependents if deletion is blocked
- Type conversion support: axiom → deduction (require deps + relation to be provided), deduction → axiom (require deps + relation to be cleared)
- Expose Tauri commands: `create_node`, `get_node`, `update_node`, `delete_node`
- Unit tests for each operation, including deletion-blocked scenario

---

## Issue #4 — Edge CRUD Operations (Rust Backend)

**Labels**: `backend`, `core`

Implement edge creation, modification, and deletion, stored in the dependent node's frontmatter.

**Acceptance criteria**:

- **Create edge**: Add a dependency entry to the dependent node's frontmatter
- **Delete edge**: Remove a dependency entry; flag if this leaves a deduction node with zero dependencies
- **Cycle detection**: Before creating an edge, run a DAG check (topological sort or DFS). Reject the edge with a clear error if it would create a cycle
- Expose Tauri commands: `create_edge`, `delete_edge`
- Unit tests: basic CRUD, cycle rejection (direct cycle, transitive cycle)

---

## Issue #5 — Validation Engine (Rust)

**Labels**: `backend`, `core`

Implement the full validation rule set from §7 of the spec, runnable both in-app and as a CLI.

**Acceptance criteria**:

- Implement all validation rules:
	- No cycles (report full cycle path)
	- No dangling references (identify referencing node + missing target)
	- Relation integrity (all dep IDs in expression, all expression IDs in deps)
	- Relation parsability (syntactically valid propositional logic)
	- Type constraints (axiom/deduction invariants)
	- No duplicate IDs
	- Required fields present and correctly typed
	- Tag reference warnings (not errors)
- Return structured validation results: `Vec<ValidationError>` with file path, rule name, human-readable message
- Expose as a Tauri command: `validate_project()`
- Also compilable as a standalone CLI binary for use in pre-commit hooks and CI workflows
- The pre-commit hook script (`validate.sh`) and GitHub Action workflow (`validate.yaml`) are generated during project scaffolding (Issue #2) only if the user opts in

---

## Issue #6 — Relation Expression Parser (Rust)

**Labels**: `backend`, `core`

Build a parser for propositional logic relation expressions.

**Acceptance criteria**:

- Parse expressions with operands (node IDs like `n-XXXXXX`), operators (`AND`, `OR`, `NOT`, `IMPLIES`, `IFF`), and parentheses
- Return an AST representation
- Validate that all operands exist in the node's dependency list and vice versa
- Provide clear parse error messages with position info (e.g., "unexpected token at position 14")
- Expose a `parse_relation(expression, dependencies) -> Result<AST, ParseError>` function
- Unit tests: valid expressions, operator precedence, missing operands, extra operands, syntax errors

---

## Issue #7 — Staleness Propagation Engine (Rust)

**Labels**: `backend`, `core`

Implement the staleness lifecycle and propagation logic from §3.

**Acceptance criteria**:

- When a node is edited, mark all direct dependents as `stale` with `stale_sources` recording which node triggered it and when
- Cascade transitively: if X becomes stale, all of X's dependents also become stale
- "Mark as reviewed" action: sets status back to `current`, clears `stale_sources`, does NOT propagate further
- "Edit and save" on a stale node: sets status to `current` but triggers a new propagation wave to its own dependents
- Axioms never become stale from propagation, only from direct edit
- Write updated status fields to disk for every affected node
- Expose Tauri commands: `mark_node_reviewed(node_id)`, and ensure `update_node` triggers propagation automatically
- Unit tests: single-hop propagation, transitive cascade, review-stops-propagation, edit-re-propagates

---

## Issue #8 — File Watcher (Rust)

**Labels**: `backend`, `core`

Watch the project directory for external changes and sync in-memory state.

**Acceptance criteria**:

- Use the `notify` crate to watch `nodes/` and `.knowledgebase/config.yaml`
- On file create: parse new node, add to in-memory graph, re-validate
- On file modify: re-parse node, diff against in-memory state, trigger staleness propagation if content changed
- On file delete: remove from in-memory graph, flag any nodes that now have broken dependencies
- On config change: reload project config (tags, display settings)
- Debounce rapid changes (e.g., editor save-then-format)
- Emit events to the frontend via Tauri's event system so the UI can react
- Integration tests simulating external file changes

---

## Issue #9 — Graph Canvas: Rendering Nodes & Edges (Frontend)

**Labels**: `frontend`, `core`

Render the knowledge graph on an interactive canvas.

**Acceptance criteria**:

- Integrate **Svelte Flow** (`@xyflow/svelte`) as the graph rendering library (decision rationale in `core-feature-spec.md` §9.2 — DOM-based, Liam ERD-style cards, comfortable up to ~1000 nodes)
- Build a custom Svelte Flow node component for main nodes: Liam-ERD-style card with a header strip carrying the type icon (axiom vs deduction) and title. Edge handles on the left/right sides. State styling — left-border accent / header-strip tint / full border tint — picked at implementation time from the three candidates documented in §6.7
- Render directed edges with arrows pointing from dependency → dependent (customized Svelte Flow edge component, accent-colored thin lines)
- Auto-layout: DAG-aware hierarchical layout run on first load. Pick between **elkjs** (Liam ERD's choice — `layered` algorithm, closest match to Liam's on-canvas behavior) and **dagre** (simpler API, smaller bundle). Feed the resulting `(x, y)` coordinates into Svelte Flow as initial node positions
- Manual drag-to-reposition (built-in to Svelte Flow); persist updated positions to a local (gitignored) file
- Canvas pan and zoom (built-in to Svelte Flow), plus the `<Background pattern="dots" />` component as the default canvas background

---

## Issue #10 — Symbiotic Relation Nodes (Frontend)

**Labels**: `frontend`, `core`

Render the relation expression as a small attached node on each deduction.

**Acceptance criteria**:

- For each deduction node, render a small **unlabeled circle** (~1/10th size of a main card) attached to the incoming-edge side, as a separate custom Svelte Flow node type
- Visual state: neutral fill when the expression is valid, red fill when broken/invalid (no text on the circle itself)
- Clicking the relation node opens the relation editor panel (Issue #14), which displays the expression with node titles substituted for IDs
- Visibility toggled by the `display.relation_nodes` config flag

---

## Issue #11 — Node Interactions (Frontend)

**Labels**: `frontend`, `interaction`

Implement the click/double-click/right-click/drag interactions from §6.2.

**Acceptance criteria**:

- **Click**: Select node, highlight it and all directly connected edges (incoming + outgoing)
- **Double-click**: Open the sidebar editor panel (Issue #13)
- **Right-click**: Context menu with: Delete (greyed out + tooltip if dependents exist), Change type (guided flow), Mark as reviewed, Trace upstream, Trace downstream
- **Drag**: Reposition node on canvas, persist new position locally
- Clicking empty canvas deselects everything

---

## Issue #12 — Edge Interactions (Frontend)

**Labels**: `frontend`, `interaction`

Implement edge creation and management from §6.3.

**Acceptance criteria**:

- **Drag from node to node**: Visual drag indicator (line following cursor), on drop open a dialog to confirm direction
- **Click edge**: Select it, offer option to delete
- Enforce cycle prevention: if the backend rejects the edge, show a clear error in the UI explaining the cycle
- Edge creation updates the relation expression prompt (warn user if new dep isn't in the expression yet)

---

## Issue #13 — Sidebar Editor Panel (Frontend)

**Labels**: `frontend`, `core`

Build the sidebar panel for viewing and editing node content and metadata.

**Acceptance criteria**:

- Opens on double-click of a node
- Displays and allows editing of: title, type (with guided conversion flow), tags (multi-select from project tags), status (read-only display + "mark as reviewed" button for stale nodes)
- Dependency list: shows current dependencies, allows removal
- `stale_sources` display: when stale, show which dependencies triggered it and when
- Embeds the existing Milkdown/CodeMirror editor for the node's markdown content, with the WYSIWYG/raw toggle
- "Open with" button: opens the `.md` file in the system default or user-configured external editor
- Save triggers backend `update_node` (which handles disk write + staleness propagation)

---

## Issue #14 — Relation Expression Editor (Frontend)

**Labels**: `frontend`, `interaction`

Build the relation expression editor panel.

**Acceptance criteria**:

- Opens when clicking a symbiotic relation node
- Shows the current relation expression with node IDs replaced by human-readable titles
- Text input for editing the raw expression
- Highlight available operators: `AND`, `OR`, `NOT`, `IMPLIES`, `IFF` as clickable chips/buttons
- Live validation: as the user types, parse the expression and show errors inline
- On save: validate via backend, update the node's `relation` field, write to disk

---

## Issue #15 — Tracing: Upstream & Downstream Highlighting (Frontend)

**Labels**: `frontend`, `interaction`

Implement the trace-upstream and trace-downstream features from §6.5.

**Acceptance criteria**:

- **Trace upstream**: From a selected node, recursively highlight all dependencies and connecting edges; dim everything else
- **Trace downstream**: From a selected node, recursively highlight all dependents and connecting edges; dim everything else
- Accessible via right-click context menu (Issue #11)
- Escape key or clicking empty canvas clears the trace and restores normal view
- Smooth transition when entering/exiting trace mode

---

## Issue #16 — Project Settings Panel (Frontend)

**Labels**: `frontend`, `settings`

Build the settings UI for managing project-level configuration.

**Acceptance criteria**:

- Edit tag definitions: add/remove tags
- Display toggles: relation nodes visibility
- Changes write to `.knowledgebase/config.yaml` via backend
- Validate that removing a tag in active use shows a warning listing affected nodes

---

## Issue #17 — Undo/Redo Stack

**Labels**: `frontend`, `core`

Implement session-scoped undo/redo for all graph mutations.

**Acceptance criteria**:

- Track all mutations: node create/edit/delete, edge create/delete/modify, relation edits, status changes
- Undo reverses the last mutation and writes the reverted state to disk
- Redo re-applies the undone mutation
- Stack is session-scoped (cleared on app restart)
- Standard keyboard shortcuts: Ctrl+Z / Ctrl+Shift+Z (or Cmd on macOS)
- UI indicators showing undo/redo availability

---

## Issue #18 — Broken Dependency Detection & UI

**Labels**: `backend`, `frontend`, `core`

Handle broken dependencies gracefully in both backend and UI.

**Acceptance criteria**:

- Backend detects when a node references a dependency that doesn't exist (on load, after file watcher events, after deletions)
- Affected nodes flagged with a `broken_dependency` state alongside their normal status
- Frontend renders broken dependency nodes with a red border and error badge
- Clicking a broken-dependency node shows which references are broken and suggests resolution (remove the edge or recreate the missing node)

---

## Suggested Issue Order

This isn't strictly linear — some issues can be parallelized — but the dependency chain is:

```
#1 (Data Model)
 └─► #2 (Project Scaffold)
      └─► #3 (Node CRUD)
           ├─► #4 (Edge CRUD)
           │    └─► #6 (Relation Parser)
           │         └─► #5 (Validation Engine)
           ├─► #7 (Staleness Propagation)
           └─► #8 (File Watcher)

#9 (Graph Canvas)  ← can start once #1-#4 are done
 ├─► #10 (Relation Nodes)
 ├─► #11 (Node Interactions)
 │    └─► #13 (Sidebar Editor)
 ├─► #12 (Edge Interactions)
 │    └─► #14 (Relation Editor)
 ├─► #15 (Tracing)
 └─► #18 (Broken Deps UI)

#16 (Settings Panel)  ← can start once #2 is done
#17 (Undo/Redo)       ← can start once #3 + #4 are done
```

Backend issues #1–#8 and frontend issues #9–#18 can largely proceed in parallel once the data model (#1) is solid, with the frontend consuming the Tauri commands exposed by the backend.