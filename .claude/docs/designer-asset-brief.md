# Ex Grapha — Designer Asset Brief

## Project Context

Ex Grapha is a desktop knowledge graph app. The main view is a dark canvas where nodes (spheres) are connected by directed edges. The aesthetic is "obsidian UI meets alchemist's workshop" — dark, polished surfaces with subtle light effects. Clean and minimalist enough for professional use, but with a distinctive material language.

**Rendering engine**: Pixi.js (WebGL). All assets are rendered as sprites on a GPU-accelerated canvas. The engine handles glow, blur, tinting, scaling, and animation at runtime — these should NOT be baked into the assets.

---

## Assets Needed

### 1. Node Spheres (4 sprites)

Each node in the graph is a sphere. There are 4 distinct states:

| Asset filename    | Description |
|-------------------|-------------|
| `node-axiom.png`  | Solid, opaque polished sphere. Dark glass with a faint inner luminance. Should feel grounded and foundational — this represents accepted facts/premises. |
| `node-deduction.png` | Similar sphere but slightly more translucent with a subtle luminous core. Should feel "derived" — this represents conclusions drawn from other nodes. |
| `node-stale.png`  | Sphere with **hairline cracks** on the surface and **warm amber/gold light leaking through** the cracks. Represents a node that needs review because an upstream dependency changed. Should look "under stress" but not destroyed. |
| `node-broken.png` | **Full shattered glass effect**. The sphere fragments are still roughly held in shape but clearly fractured. **Red/crimson light bleeds through** the cracks and gaps. Represents a structurally broken node (a dependency no longer exists). Should look more severe than stale. |

**Format requirements:**
- PNG with **transparent background** (alpha channel required)
- **512x512 px** minimum (1024x1024 preferred for retina displays). One size only — the engine scales down.
- The sphere should fill ~80-90% of the canvas, leaving ~10-20% transparent padding around it. This padding is where the engine renders glow/blur effects without clipping.
- **Do NOT bake in**: glow, halo, bloom, shadow, blur, or any outer effects. The engine adds these dynamically per-node at runtime.
- **Do NOT provide** size variants, color variants, or selected-state variants. The engine handles scaling, tinting, and selection highlight.

**Style reference:** Think polished obsidian, dark crystal, or dark glass with subtle internal light. Not cartoonish or flat — should have realistic light reflections and depth.

---

### 2. Relation Node (2 sprites)

Relation nodes are small symbols attached to certain nodes, representing a logical expression. They are ~1/10th the visual size of a main node sphere.

| Asset filename       | Description |
|----------------------|-------------|
| `relation-valid.png`   | Small geometric shape — faceted gem, diamond, or teardrop. Neutral/healthy appearance. Should use the same material language as the spheres (dark glass/crystal). |
| `relation-broken.png`  | Same shape but with a red/error visual treatment (red tint, cracks, or red inner glow). Represents an invalid logical expression. |

**Format requirements:**
- PNG with transparent background
- **256x256 px** minimum
- Same padding and "no baked effects" rules as node spheres

---

### 3. Background Tile (1 image)

A subtle background texture for the dark canvas.

| Asset filename | Description |
|----------------|-------------|
| `bg-tile.png`  | Seamless tileable pattern with tribal/arcane/celtic linework. Think circuit-board geometry meets celtic knots — intricate but not busy. |

**Format requirements:**
- PNG with **transparent background** — provide the linework as **white or light gray on transparent**. The app controls opacity (will be displayed at 5-10% opacity) and can tint the color in code.
- **Must tile seamlessly** in both X and Y directions
- **512x512 px** or **1024x1024 px**
- The pattern should be visible as subtle texture when zoomed out, but fade into "just a dark background" at normal working zoom

---

### 4. Edge Connector (1 sprite, optional)

If edges should have a decorative element where they meet nodes (like a teardrop/droplet shape), provide it here. If edges are just luminous lines, skip this.

| Asset filename       | Description |
|----------------------|-------------|
| `edge-connector.png`  | Teardrop or droplet shape where an edge meets a node. Same material language as the spheres. |

**Format requirements:**
- PNG with transparent background
- **128x128 px**
- Oriented pointing **downward** — the engine rotates it to match edge direction at runtime
- No baked glow/effects

---

## What the Engine Handles (NOT your job)

These effects are applied dynamically in code. Do not include them in the assets:

- **Glow / halo** around nodes (selection highlight, pulsating effects)
- **Blur / shadow** behind nodes (used for distance-based visual encoding)
- **Color tinting** (the engine can shift hue/saturation on any sprite)
- **Scaling** (one size per asset, engine scales to any display size)
- **Edge lines** (drawn programmatically — curves, gradients, luminous lines)
- **Animation** (glow pulsing, highlight transitions)
- **Pan / zoom** camera behavior

---

## Summary Checklist

| # | Asset | Size | Format | Required? |
|---|-------|------|--------|-----------|
| 1 | `node-axiom.png` | 512-1024px | PNG, transparent bg | Yes |
| 2 | `node-deduction.png` | 512-1024px | PNG, transparent bg | Yes |
| 3 | `node-stale.png` | 512-1024px | PNG, transparent bg | Yes |
| 4 | `node-broken.png` | 512-1024px | PNG, transparent bg | Yes |
| 5 | `relation-valid.png` | 256px | PNG, transparent bg | Yes |
| 6 | `relation-broken.png` | 256px | PNG, transparent bg | Yes |
| 7 | `bg-tile.png` | 512-1024px | PNG, transparent bg, seamless tile | Yes |
| 8 | `edge-connector.png` | 128px | PNG, transparent bg | Optional |

**Total: 7 required assets + 1 optional**
