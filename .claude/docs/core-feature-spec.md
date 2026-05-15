
## 1. Overview

A dependency-aware knowledge graph application where nodes represent atomic pieces of knowledge and directed edges represent dependency relationships between them. The system propagates staleness when upstream knowledge changes, enabling users to trace, validate, and maintain chains of reasoning.

The data layer is git-native: every node is a markdown file, the graph structure is encoded in frontmatter. Collaboration and version controlling can happen through standard git workflows (branches, PRs, merges).

---

## 2. Core Concepts

### 2.1 Nodes

A node is an atomic unit of knowledge. Each node is a single markdown file stored in `nodes/<node-id>.md`.

Every node has:

- **ID**: A short, stable, randomly generated identifier prefixed with `n-` (e.g., `n-4a7b2c`). The ID is also the filename. IDs never change once assigned.
- **Title**: A human-readable display name. Can be changed freely.
- **Type**: One of two values — `axiom` or `deduction`. Determines structural constraints (see 2.1.1).
- **Tags**: Zero or more user-defined freeform tags. Tags are defined at the project level. A node can carry multiple tags. Tags are used for filtering and visual grouping (e.g., "well-established", "subjective", "speculative", "physics", "economics").
- **Status**: One of `current` or `stale`. Governs the staleness lifecycle (see §3).
- **Content**: pure markdown

#### 2.1.1 Node Types

**Axiom**

- Represents accepted foundational knowledge (facts, assumptions, premises).
- Must have zero dependencies (no incoming knowledge edges).
- Must not have a `relation` expression.
- Cannot become stale from upstream changes (it has no upstream).

**Deduction**

- Represents derived knowledge — conclusions, inferences, results.
- Must have at least one dependency.
- Must have a `relation` expression describing the logical relationship among its dependencies.
- Can become stale when any dependency changes or becomes stale.

**Type conversion**: Changing a node's type is a constrained operation. Converting axiom → deduction requires adding at least one dependency and a valid relation expression. Converting deduction → axiom requires removing all dependencies and the relation expression. The GUI will enforce this as a guided multi-step process.

### 2.2 Edges

A directed edge represents a dependency relationship: if node A has an edge from node B, then "A relies on B".

Edges are stored in the frontmatter of the dependent node (the node that relies on the other). Each edge entry contains the **target node ID** — the node being depended upon.

**Directionality convention**: An edge is "from" the dependency "to" the dependent. In the frontmatter of node A, listing node B as a dependency means "there is an edge from B to A." In the graph, the arrow points from B → A (B feeds into A).

**Constraints**: Many-to-one is allowed (a node can depend on many others). One-to-many is allowed (a node can be depended upon by many others). Cycles are forbidden — the graph must be a DAG at all times. Cycle detection is enforced at edge creation time and by the validation system.

### 2.3 Relation Expressions

Each deduction node carries a `relation` field: a propositional logic expression describing how its dependencies combine to support it.

**Syntax**:

- Operands: Node IDs from the node's dependency list.
- Operators: `AND`, `OR`, `NOT`, `IMPLIES`, `IFF`.
- Parentheses for grouping.
- Example: `(Node-1 AND Node-2) IMPLIES Node-3`

**Constraints**: Every node ID in the expression must appear in the node's dependency list. Every node in the dependency list should appear in the expression. If a dependency is removed, the expression may become invalid — this is flagged as a broken relation (see §3.3).

**UI representation**: The relation is displayed as a small symbiotic node attached to the incoming-edge side of the parent node, approximately 1/10th the size of the main node — rendered as a plain unlabeled circle. Clicking it opens the relation editor panel (see §6.5), where the user can view and modify the expression. Inside the editor, node IDs are dynamically substituted with their human-readable titles; the canvas circle itself carries no text.

---

## 3. Staleness Propagation

### 3.1 Lifecycle

Every node has a `status` field with one of two values:

| Status    | Meaning                                                                                 |
| --------- | --------------------------------------------------------------------------------------- |
| `current` | The node's content is up to date with respect to all its dependencies.                  |
| `stale`   | One or more upstream dependencies have changed or become stale. This node needs review. |

### 3.2 Propagation Rules

Staleness propagation is **conservative**: if any dependency of a node changes or becomes stale, the node itself becomes stale, regardless of the logical relation expression. This forces users to review the relation expression as well.

**Trigger**: When a node is edited (content or metadata change), all nodes that directly depend on it become `stale`. This cascades transitively — if X becomes stale and Y depends on X, Y also becomes stale.

**Stale source tracking**: When a node becomes stale, the system records which dependency triggered the staleness and when, in a `stale_sources` field. Multiple sources can accumulate if several dependencies change.

**Resolution**: A user opens a stale node, reviews it, and either:

1. **Confirms it as current** (the upstream change doesn't affect this node's validity): status returns to `current`. Propagation stops at this node — its dependents are not further affected.
2. **Edits the node**: status becomes `current` after save, but the edit triggers a new round of staleness propagation to this node's own dependents.

**Axiom behavior**: Axioms have no upstream, so they never become stale from propagation. They can only change via direct user edit, which then propagates staleness downstream.

### 3.3 Broken Dependencies

If a node references a dependency that no longer exists (e.g., the file was removed outside the app, or a merge introduced an inconsistency), the node enters a `broken dependency` state. This is distinct from staleness — it means the graph structure itself is invalid. The validation system (§7) catches this. In the GUI, broken dependencies are rendered with a red error indicator.

---

## 4. Node Deletion

A node can only be deleted if no other nodes depend on it (no edges are coming out of it to other nodes). If a user wants to delete a node that has dependents, they must first manually remove all edges from dependent nodes. This ensures the user explicitly considers the impact on each dependent.

The GUI should clearly communicate why deletion is blocked and list the nodes that still depend on the target.

---

## 5. Project Structure & File Format

### 5.1 Directory Layout

```
my-knowledge-base/
├── .knowledgebase/
│   ├── config.yaml              # Project-level configuration
│   └── hooks/                   # (optional) Git pre-commit hook
│       └── validate.sh
├── nodes/
│   ├── n-4a7b2c.md
│   ├── n-3f8a1d.md
│   └── ...
├── assets/
│   ├── n-4a7b2c/
│   │   ├── diagram.png
│   │   └── photo.jpg
│   ├── n-3f8a1d/
│   │   └── chart.svg
│   └── ...
├── .github/                     # (optional) GitHub Actions workflow
│   └── workflows/
│       └── validate.yaml
└── README.md
```

The `hooks/` directory and `.github/` directory are **optional** — they are only created during project initialization if the user opts in via the "New Project" dialog. Users who don't use git or GitHub can safely omit them.

### 5.2 Project Configuration

`.knowledgebase/config.yaml` defines project-scoped settings:

```yaml
display:
  relation_nodes: true
  canvas_background: "dots"  # one of: "dots", "grid", "arcane"

tag_definitions:
  - name: "well-established"
  - name: "tentative"
  - name: "speculative"
```

Tag definitions are user-editable. Users can add and remove variants through the GUI settings panel or by editing the config file directly.

The `relation_nodes` toggle controls visibility of symbiotic relation nodes, allowing the user to declutter the graph view. The `canvas_background` setting chooses the canvas background style: `dots` and `grid` use Svelte Flow's built-in clean backgrounds (default `dots`); `arcane` uses a dark canvas with a low-opacity tribal/arcane linework pattern for users who prefer the original mood-board aesthetic.

### 5.3 Node File Format

Each node file uses YAML frontmatter followed by markdown content:

```yaml
---
id: "n-4a7b2c"
title: "Conservation of Energy"
type: "axiom"
tags:
  - "physics"
  - "well-established"
status: "current"
status_updated_at: "2026-03-04T14:30:00Z"
status_updated_by: "alice"
created_at: "2026-02-15T10:00:00Z"
created_by: "alice"
dependencies: []
---

# Conservation of Energy

Energy cannot be created or destroyed in an isolated system...
```

```YML
---
id: "n-7c1d3e"
title: "Orbital Mechanics Follow from Newton's Laws"
type: "deduction"
tags:
  - "physics"
  - "well-established"
status: "stale"
status_updated_at: "2026-03-04T15:00:00Z"
status_updated_by: "system"
stale_sources:
  - node_id: "n-4a7b2c"
    changed_at: "2026-03-04T14:30:00Z"
created_at: "2026-02-20T09:00:00Z"
created_by: "alice"
dependencies:
  - node_id: "n-4a7b2c"
  - node_id: "n-3f8a1d"
relation: "(n-4a7b2c AND n-3f8a1d)"
---

# Orbital Mechanics Follow from Newton's Laws

Given conservation of energy and Newton's gravitational law...
```

**Type constraints on the file format**:

- If `type` is `axiom`: `dependencies` must be an empty list, `relation` must be absent, `stale_sources` must be absent.
- If `type` is `deduction`: `dependencies` must be non-empty, `relation` must be present and valid.

---

## 6. GUI Interaction Model

### 6.1 Layout & Visual Philosophy

The main view is a **graph canvas** with a **side panel**. Main knowledge nodes are rendered as compact cards (Liam ERD-inspired) carrying their title and type icon directly on the card — so a user can read the graph at a glance without opening every node. Symbiotic relation nodes (see §6.5) are an exception: they are rendered as small unlabeled circles, intentionally text-free to keep the canvas uncluttered.

The **side panel** is the primary navigation and discovery surface (search, filters, broken-deps view, full node list). It is open by default when the app launches and can be toggled off/hidden/collapsed via a button or keyboard shortcut.

**Background**: Defaults to a clean dot pattern (Svelte Flow's built-in `<Background pattern="dots" />`), which works well on either a light or dark canvas. A grid pattern and an "arcane" pattern — dark canvas (#1a1a2e or similar) with a subtle tileable tribal/arcane linework at very low opacity (5–10%) — are alternative styles selectable by the user via project settings (`display.canvas_background`). All three render as backgrounds behind the same node/edge components.

Node positions on the canvas follow a DAG-aware hierarchical auto-layout algorithm (engine candidates and trade-offs in §9.2) whose `(x, y)` output is fed to Svelte Flow as initial node positions. Users can manually drag-reposition nodes afterward; positions are persisted locally and gitignored, as they are a personal display preference, not shared knowledge.

### 6.2 Side Panel

The side panel is the primary interface for navigating, filtering, and understanding the graph.

#### 6.2.1 Default View: Broken Dependencies

When the app opens, the side panel defaults to the **broken dependencies / needs review** view — showing nodes that need attention after upstream changes. This makes the default state actionable ("here's what needs your attention"). If no nodes are broken or stale, the panel falls back to displaying the full list of all node titles.

#### 6.2.2 Broken Dependencies View

In this view, the side panel displays a **toggle list** of all changed (source) nodes. Clicking a changed node expands it to reveal all downstream nodes that depend on it and need review.

Each title in the expanded list has a **small circle indicator** positioned to the right of the title. These circles use a **distance-based opacity gradient**: direct children of the changed node have ~90% opaque white, nodes 2 hops away have less, and so on — nodes 10 hops away have ~10% opaque white. This gradient is also reflected on the canvas as varying **shadow/blur intensity** behind the corresponding nodes, providing a rough visual mapping between panel titles and canvas nodes based on distance from the source of change.

When a changed node is clicked and expanded, the canvas **auto-zooms** to the relevant subgraph, highlighting the selected node and its affected dependents.

#### 6.2.3 Full Node List View

When no filter is active (or explicitly toggled), the panel shows all node titles. A **fuzzy search bar** at the top of the panel is mandatory for scaling — a 500-node list must be searchable. Grouping by tags or other criteria is deferred to v2.

#### 6.2.4 Panel ↔ Canvas Bidirectional Interaction

**Panel → Canvas:**
- **Hover** on a title in the panel → the corresponding node on the canvas receives a **pulsating highlight** (glow animation).
- **Click** on a title → the canvas auto-zooms to the node and its neighborhood. In the broken deps view, clicking filters the panel to show only connected nodes.

**Canvas → Panel:**
- **Click** a node on the canvas → the side panel syncs: shows the node's details and filters the list to its neighborhood. This bidirectional linkage keeps both views in sync at all times.

### 6.3 Node Interactions

| Action                | Behavior                                                                                                          |
| --------------------- | ----------------------------------------------------------------------------------------------------------------- |
| **Click node**        | Select the node. Highlight the node and all its directly relevant edges (both incoming and outgoing).             |
| **Double-click node** | Open the sidebar editor panel for viewing and editing the node's content, metadata, type, tags, and dependencies. |
| **Right-click node**  | Context menu: delete (if allowed), change type (guided), mark as reviewed, trace upstream, trace downstream.      |
| **Drag node**         | Reposition the node on the canvas.                                                                                |

### 6.4 Edge Interactions

| Action                     | Behavior                                                                                                                     |
| -------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| **Drag from node to node** | Create a new edge. Opens a dialog to confirm the direction of the dependency.                              |
| **Click edge**             | Select the edge. Offer option to delete the edge.                                                         |

### 6.5 Relation Node Interactions

| Action                            | Behavior                                                                                                                                               |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Click symbiotic relation node** | Open the relation expression editor panel. This panel should highlight all the possible logical operators (AND, OR, NOT, IMPLIES, IFF) |

The symbiotic relation node is rendered as a small unlabeled circle, approximately 1/10th the size of the parent node, attached to the incoming-edge side. Its visual state reflects validity — neutral fill when valid, red fill when the expression is broken or invalid. The expression itself is shown only inside the relation editor panel (with node titles substituted for IDs), never on the canvas.

### 6.6 Tracing

- **Trace upstream**: From a selected node, highlight all nodes it depends on, recursively, and the edges connecting them. All other nodes and edges dim.
- **Trace downstream**: From a selected node, highlight all nodes that depend on it, recursively, and the edges connecting them. All other nodes and edges dim.

Pressing Escape or clicking empty canvas space clears tracing highlights.

### 6.7 Visual States

Main nodes are rendered as **Liam ERD-style cards**: a clean rectangular card with a header strip carrying the type icon and the node's title. Edges attach to handles on the card's left/right sides. State is communicated through card-level CSS treatments — color accents and selection rings — rather than per-state bespoke artwork.

**State styling — three candidate approaches** (decision deferred to implementation; the spec lists all three so trade-offs can be evaluated against real card mockups):

1. **Left-border accent.** A vertical color strip on the left edge of the card. Light visual weight; common in notification/callout UIs (Bootstrap, Material, GitHub).
2. **Header-strip tint.** The header strip's background color shifts to communicate state. Matches Liam ERD's actual pattern — a single visual element does the state work and integrates with the type icon already in the header.
3. **Full border tint.** Color the entire card's border. Loudest, hardest to miss, but visually heavier and may compete with the selection ring.

Whichever option is chosen, the state-to-color mapping is consistent:

| State                  | Color                  |
| ---------------------- | ---------------------- |
| Axiom (current)        | Neutral — no accent    |
| Deduction (current)    | Neutral — no accent    |
| Stale (needs review)   | Warm amber             |
| Broken dependency      | Red                    |

**Selected / Active** is independent of the state styling above: an accent-colored ring/outline around the entire card, with a subtle pulse animation when triggered via panel hover. The selection ring is orthogonal to all three candidates, so each one supports it cleanly.

**Edges** use Svelte Flow's edge components, styled to thin lines in a muted accent color with arrow markers pointing from dependency → dependent. Stale edges (where the source node is stale) render with a dashed style. Selected/traced edges render at full opacity with the accent color; non-selected edges dim during trace mode.

**Symbiotic relation nodes** (see §6.5) are small unlabeled circles rendered as a separate custom Svelte Flow node type. They share the accent vocabulary — neutral fill when valid, red fill when broken.

**Distance-based shadow**: When the broken-dependency filter is active in the side panel, cards on the canvas receive a CSS `box-shadow` whose intensity is proportional to their distance from the source of change — matching the opacity gradient shown next to titles in the side panel's circle indicators.

### 6.8 Undo/Redo

All graph mutations (node create/edit/delete, edge create/delete/modify, relation edits, status changes) are undoable. The undo stack is session-scoped (not persisted across app restarts).

---

## 7. Validation

A validation system enforces graph integrity. It always runs in-app and can optionally be configured for CI contexts:

1. **Pre-commit hook** (`.knowledgebase/hooks/validate.sh`): Optional. Runs locally before each git commit if the user opted in during project creation.
2. **GitHub Action** (`.github/workflows/validate.yaml`): Optional. Runs on every PR if the user opted in during project creation and uses GitHub.

### 7.1 Validation Rules

| Rule                              | Description                                                                                                                                      |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| **No cycles**                     | The dependency graph must be a DAG. Report the cycle path if found.                                                                              |
| **No dangling references**        | Every `node_id` in a `dependencies` list must correspond to an existing node file.                                                               |
| **Relation integrity**            | Every node ID in a `relation` expression must exist in that node's `dependencies` list. Every dependency must appear in the relation expression. |
| **Relation parsability**          | The `relation` expression must be syntactically valid propositional logic.                                                                       |
| **Type constraints**              | Axioms must have empty dependencies and no relation. Deductions must have non-empty dependencies and a valid relation.                           |
| **No duplicate IDs**              | No two node files may share the same `id` value.                                                                                                 |
| **Required fields**               | All mandatory frontmatter fields are present and correctly typed.                                                                                |
| **Tag references**                | Tags used by nodes should be defined in `config.yaml`. Warn (not error) on undefined tags.                                                       |

### 7.2 Error Reporting

Validation errors are reported with the file path, the specific rule violated, and a human-readable explanation. For cycle detection, the full cycle path is printed. For dangling references, both the referencing node and the missing target are identified.

---

## 8. Collaboration Model

The data format is git-native, so collaboration can happen through standard git workflows, but git is **not required**:

- The knowledge base directory is designed to work well as a git repository, but users may also use it purely locally or with any other VCS.
- When using git, contributors clone the repo, make changes (add/edit/remove nodes, modify edges), and push branches. Changes can be proposed via pull requests or merge requests on any hosting platform (GitHub, GitLab, Bitbucket, etc.).
- If the optional GitHub Actions workflow was included during project creation, it runs on each PR, blocking merges that violate graph integrity. Similarly, the optional pre-commit hook runs validation before each commit.
- Merge conflicts in node files are resolved using standard merge tooling. Since each node is its own file, conflicts are minimized.
- Node positions are gitignored (local display preference), so they never conflict.

---

## 9. Technology Stack

### 9.1 Application Shell

**Tauri** serves as the application shell. The backend (Rust) handles all data model logic: parsing and writing frontmatter, cycle detection, staleness propagation, file watching, validation, and file I/O. The frontend (web technologies via webview) handles all UI rendering: the graph canvas, the editor panel, node interactions, and visual state.

### 9.2 Frontend

The frontend runs inside Tauri's webview layer using **Svelte 5** + **Vite** + **TypeScript**. The graph canvas is rendered with **Svelte Flow** (`@xyflow/svelte`), a DOM-based graph rendering library — the Svelte port of React Flow, both maintained under the `xyflow` umbrella. Visual reference: [Liam ERD](https://liambx.com/) — clean card-style nodes, handle-based edge endpoints, polished pan/zoom interactions. Only the visual language is borrowed; no code is lifted (Liam ERD itself is React Flow-based; the libraries are API-similar but separate packages).

- **Graph canvas (Svelte Flow)**: A custom Svelte Flow node component for main knowledge nodes (Liam-ERD-style card with a header strip carrying type icon + title; state styling per §6.7). A separate custom node type for symbiotic relation nodes (small unlabeled circle). Customized Svelte Flow edge components with arrow markers and accent-colored styling. Pan/zoom/drag/selection/minimap come built-in.
- **App UI (side panel, editor, settings, dialogs)**: Standard Svelte 5 components (HTML/CSS). Handles all text-based chrome, forms, search, filters, and the markdown editor panel.

**Why Svelte Flow (not Pixi.js)**: An earlier draft of this spec specified Pixi.js + WebGL with custom sphere sprites. After surveying showcase projects — Liam ERD in particular — the DOM-based card aesthetic is a better fit for a knowledge-graph tool than bespoke artwork: readable nodes at any zoom level, real accessibility and copy-paste, and CSS-driven state styling instead of swapping pre-rendered PNG variants. The expected node count is 10–1000 (typically ~100), well within Svelte Flow's comfort zone, so WebGL's performance ceiling is unnecessary.

**Communication**: A Svelte store holds the graph state. The side panel reads/writes it. Svelte Flow observes it (via reactive `$nodes` / `$edges` stores) and renders accordingly. Node and edge interactions dispatch events back to the store, keeping panel and canvas in sync.

**Layout engine**: Initial node positioning uses a DAG-aware hierarchical layout algorithm whose `(x, y)` output is fed into Svelte Flow as initial node positions. Two strong candidates (decision at implementation time):

- **elkjs** — the JavaScript port of Eclipse Layout Kernel. Liam ERD uses elkjs (specifically the `layered` algorithm — verified in their repo at `frontend/packages/erd-core/src/features/erd/utils/computeAutoLayout/`), so this is the closest path to matching Liam's behavior on canvas.
- **dagre** — simpler API, smaller bundle, widely used across the React/Svelte Flow ecosystems. Often sufficient for hierarchical DAG layouts and easier to drop in.

Users can manually drag-reposition nodes after auto-layout. Positions are persisted locally and gitignored.

### 9.3 Markdown Editor

The application provides two modes for editing node content:

**In-app editor**: Milkdown, A WYSIWYG markdown editor embedded in the sidebar panel. Since the app uses a webview frontend, a JavaScript-based markdown editor component is embedded directly. **CodeMirror 6** (raw markdown with syntax highlighting) will be used for pure markdown mode (there will be a switch toggle for WYSIWYG and pure markdown).

**"Open with" button**: For users who prefer their own editor, a button opens the node's markdown file in an external application (e.g., VSCode, Obsidian, Notepad). This complements the in-app editor — the in-app editor handles quick edits, and "Open with" handles power-user workflows.

### 9.4 Persistence & File Watching

**Local save only.** The application saves all changes directly to disk. Every mutation (node create/edit/delete, edge changes, status changes) is written to the corresponding markdown file immediately. There is no built-in git integration — the app does not commit, push, pull, or manage branches.

**File watching.** The application watches the `nodes/` directory and `.knowledgebase/config.yaml` for external changes (via the `notify` crate in Rust). When a file is modified, created, or deleted outside the app (e.g., by an external editor via "Open with", by a `git pull`, or by manual file editing), the app detects the change, reloads the affected node(s), re-evaluates graph integrity, and updates the UI accordingly.

**Version control is the user's responsibility.** The project directory is designed to be a git repository, but the app does not interact with git. Users manage commits, branches, pushes, and PRs using their preferred git tooling (CLI, GitHub Desktop, VSCode git panel, etc.).

---

## 10. Deferred / Future Considerations

The following items have been identified but are explicitly out of scope for the core feature set. They are recorded here for future planning.

- **Side panel grouping/tags**: Grouping the full node list by tags or other criteria (v2 — fuzzy search is sufficient for v1).
- Focus mode (isolated view of a single node's full dependency tree).
- Logic-aware staleness propagation (using relation expressions to determine if staleness is relevant).
- Visual relation builder (drag-and-drop logic tree editor as an alternative to raw text).
- Bulk operations (mass status changes, subgraph tagging, subtree deletion).
- Keyboard shortcuts (beyond side panel toggle).
- Confidence/strength as a first-class computed property (derived from tag composition across a dependency chain).
- Structural queries ("all deductions whose dependencies are all axioms", "all stale nodes", etc.).
- Minimap for large graphs.
