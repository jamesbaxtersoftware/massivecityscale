# MassiveCityScale — World Renderer Design

**Date:** 2026-05-16
**Scope:** World renderer only — no gameplay. Zoom from street level to solar system using simple geometry.

---

## Overview

A Bevy/Rust application that renders a procedurally generated world across 8 zoom levels — from a single road all the way out to a solar system. No gameplay; pure world visualisation. The camera is isometric. Zoom is continuous and smooth (no snapping). The color system is themeable and swappable at runtime.

---

## Scale Levels

Eight zoom levels, each a nested coordinate space. A single `ZoomLevel` f32 value (0.0 = street, 1.0 = solar system) drives camera scale and per-level opacity.

| Level | What you see | Geometry |
|---|---|---|
| **Street** | Individual roads, buildings, kerbs | Box meshes, road quads |
| **Block** | City blocks, major roads | Larger boxes grouped per block |
| **Town** | Several blocks, green space | Block groups, park patches |
| **City** | Many districts, river/coast | District blobs, road network outline |
| **Country** | Multiple cities, terrain | City dots on terrain heightmap |
| **World** | Continents, oceans | Continent shapes on a flat plane |
| **Planet** | A sphere with continents | Sphere mesh with continent patches |
| **Solar System** | Planet orbiting a star | Planet dot, sun, orbital ring |

As the user scrolls out, geometry shrinks continuously — street boxes become dots, dots merge into district blobs, blobs become city dots on a planet sphere. Each scale space fades out via opacity as the camera moves away from its range.

---

## Architecture

### `world_gen` — Pure Data Generation

Runs at startup. Seed `42` (configurable) drives a deterministic procedural generator that builds the full world hierarchy as data structs — no Bevy entities. Fast, runs once.

- **`planet.rs`** — Simplex noise on a sphere produces a land/water mask. Continent outlines derived from mask edges.
- **`city.rs`** — Poisson disk sampling places cities on land. Each city is subdivided into districts via a road grid. Districts contain blocks.
- **`street.rs`** — Each block cell gets randomly sized buildings (heights from noise), road quads between blocks.

All parameters (city count, block density, building height range) are deterministic from the seed.

### `scale` — Nested Coordinate Spaces

Eight Bevy parent entities form a transform hierarchy: street → block → town → city → country → world → planet → solar system. Each level's entities are children of that parent. The camera lives in whichever space is currently active.

- **`spaces.rs`** — Spawns the 8 parent transforms and populates children from world gen data.
- **`visibility.rs`** — Each frame, computes opacity for each space based on distance from `ZoomLevel` to that space's threshold range. Spaces far from current zoom fade toward transparent.

This approach keeps all coordinate values small and precise — a road at (4, 2) in street space never needs to know it's 200 units from the sun in solar space. Bevy's transform system composes the hierarchy automatically, solving floating-point precision issues that would arise from a single flat coordinate space spanning 13 orders of magnitude.

### `renderer` — Geometry Spawning

Simple procedural meshes only — no external assets.

- **`buildings.rs`** — Box meshes per building, sized and positioned from world gen data.
- **`terrain.rs`** — Ground quads, water planes, planet sphere mesh.
- **`solar.rs`** — Sun sphere, planet dot, orbital ring.

### `theme` — Swappable Color Palettes

A `Theme` resource holds a color palette mapping building types and terrain types to colors. Replacing the resource re-colors all meshes. Three starter themes ship:

- `dark_neon` — dark background, purple/green/pink accent buildings
- `clean_pastel` — light background, soft pastel blocks
- `earthy` — dark background, warm stone/moss tones

### `camera` — Isometric Zoom + Pan

- Isometric projection (fixed 45° angle, no perspective distortion)
- Mouse wheel → updates `ZoomLevel` resource
- Click + drag → pan
- `ZoomLevel` drives both camera scale and space opacity each frame

---

## Project Structure

```
massivecityscale/
├── src/
│   ├── main.rs
│   ├── world_gen/
│   │   ├── mod.rs
│   │   ├── planet.rs
│   │   ├── city.rs
│   │   └── street.rs
│   ├── scale/
│   │   ├── mod.rs
│   │   ├── spaces.rs
│   │   └── visibility.rs
│   ├── renderer/
│   │   ├── mod.rs
│   │   ├── buildings.rs
│   │   ├── terrain.rs
│   │   └── solar.rs
│   ├── theme/
│   │   ├── mod.rs
│   │   └── themes.rs
│   └── camera/
│       ├── mod.rs
│       └── zoom.rs
├── Cargo.toml
└── assets/
```

---

## Dependencies

- `bevy` (latest stable) — engine, ECS, rendering
- `noise` — simplex noise for terrain and building heights
- `rand` — seeded RNG for placement (Poisson disk, block layouts)

No external assets. Everything is procedural geometry and flat colors.

---

## Out of Scope (this phase)

- Gameplay mechanics of any kind
- User-placed buildings
- Simulation (traffic, population, economy)
- Networking
- Save/load
- UI beyond zoom/pan controls
