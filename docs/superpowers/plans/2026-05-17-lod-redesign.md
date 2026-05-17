# MassiveCityScale LOD Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the 8-parallel-spaces architecture with a single coordinate system + LOD (Level of Detail) system so zooming continuously reveals a coherent planet-scale world — from street-level buildings at close zoom to a solar system view at max zoom-out.

**Architecture:** All geometry lives in one coordinate space where 1 unit = 1 km, centered at the origin. The flat planet surface spans ±320 km. A `LodRange` component on each entity specifies the ortho_scale window in which it is visible; `LodPlugin` updates `Visibility` every frame. Camera ortho_scale runs from 0.001 (street, ~1 km view, buildings visible) to ~16 (solar system, ~20 000 km view) via `0.001 × 10^(4.2×zoom_value)`. The entire `src/scale/` module is deleted.

**Tech Stack:** Rust, Bevy 0.15; `src/world_gen/` is untouched — all scaling is applied in the renderer.

---

## Scale Constants Reference

Copy this block into any task that references the formulas.

```
1 unit = 1 km

CELL_KM         = 10.0    1 planet grid unit → 10 km
WORLD_HALF_KM   = 320.0   half of (32 cells × 10 km); used to center grid at origin
DIST_SCALE      = 2.0     district offset: raw Δ / 3.0 × DIST_SCALE km per district step
BLOCK_SCALE     = 0.25    block offset:    raw Δ × BLOCK_SCALE km (250 m between blocks)
BLDG_POS_SCALE  = 0.10    building offset within block: raw 0..0.8 → 0..0.08 km
BLDG_W_SCALE    = 0.20    building width/depth: raw 0.1..0.25 → 20..50 m
BLDG_H_SCALE    = 0.05    building height:      raw 0.2..3.0  → 10..150 m
```

Building world position formula:
```
cx = city.lat × CELL_KM − WORLD_HALF_KM
cz = city.lon × CELL_KM − WORLD_HALF_KM
dx = (district.x − city.lat) / 3.0 × DIST_SCALE
dz = (district.z − city.lon) / 3.0 × DIST_SCALE
bx = (block.x − district.x) × BLOCK_SCALE
bz = (block.z − district.z) × BLOCK_SCALE
ox = (building.x − block.x) × BLDG_POS_SCALE
oz = (building.z − block.z) × BLDG_POS_SCALE
h  = building.height × BLDG_H_SCALE
visual = Vec3(cx+dx+bx+ox, h/2.0, cz+dz+bz+oz)
```

LOD ranges (min_scale, max_scale) — entity is visible when `min_scale ≤ ortho_scale ≤ max_scale`:
```
LOD_BUILDINGS  = (0.0,  0.030)   individual buildings visible
LOD_BLOCKS     = (0.005, 0.20)   block-level boxes visible
LOD_CITIES     = (0.05, 1.2)     city-blob slabs visible
LOD_OCEAN      = (0.0,  4.0)     ocean/ground plane visible
LOD_CONTINENTS = (0.04, 4.0)     continent patches visible
LOD_PLANET     = (0.8,  20.0)    planet sphere visible
LOD_SOLAR      = (3.0,  20.0)    solar system visible
```

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `src/renderer/scale_consts.rs` | Create | Shared scale constants for all renderers |
| `src/lod/mod.rs` | Create | `LodRange` component + `LodPlugin` visibility system |
| `src/camera/zoom.rs` | Modify | New ortho_scale formula, new default zoom |
| `src/renderer/buildings.rs` | Rewrite | Buildings, block boxes, city blobs at real world positions |
| `src/renderer/terrain.rs` | Rewrite | Ocean plane, continent patches, planet sphere |
| `src/renderer/solar.rs` | Rewrite | Solar system at km scale |
| `src/renderer/mod.rs` | Rewrite | RendererPlugin using `LodRange`; no SpaceTags |
| `src/main.rs` | Modify | Remove `ScalePlugin`, add `LodPlugin` |
| `src/scale/` | Delete | Entire module removed |

---

## Task 1: Scale constants module

**Files:**
- Create: `src/renderer/scale_consts.rs`
- Modify: `src/renderer/mod.rs` (add one `pub mod scale_consts;` line)

- [ ] **Step 1: Create `src/renderer/scale_consts.rs`**

```rust
/// All world positions are in km. The flat planet surface spans ±320 km from origin.

pub const CELL_KM: f32 = 10.0;
pub const WORLD_HALF_KM: f32 = 320.0;

pub const DIST_SCALE: f32 = 2.0;
pub const BLOCK_SCALE: f32 = 0.25;
pub const BLDG_POS_SCALE: f32 = 0.10;
pub const BLDG_W_SCALE: f32 = 0.20;
pub const BLDG_H_SCALE: f32 = 0.05;

pub const LOD_BUILDINGS: (f32, f32) = (0.0, 0.030);
pub const LOD_BLOCKS: (f32, f32) = (0.005, 0.20);
pub const LOD_CITIES: (f32, f32) = (0.05, 1.2);
pub const LOD_OCEAN: (f32, f32) = (0.0, 4.0);
pub const LOD_CONTINENTS: (f32, f32) = (0.04, 4.0);
pub const LOD_PLANET: (f32, f32) = (0.8, 20.0);
pub const LOD_SOLAR: (f32, f32) = (3.0, 20.0);
```

- [ ] **Step 2: Add `pub mod scale_consts;` to `src/renderer/mod.rs`**

Open `src/renderer/mod.rs`. Add this line at the top of the file, before the existing `pub mod buildings;` line:

```rust
pub mod scale_consts;
pub mod buildings;
pub mod terrain;
pub mod solar;
```

- [ ] **Step 3: Run tests**

```bash
cd /Users/jamesbaxter/git/massivecityscale && cargo test
```

Expected: all existing tests still pass.

- [ ] **Step 4: Commit**

```bash
git add src/renderer/scale_consts.rs src/renderer/mod.rs
git commit -m "feat: renderer scale constants for km coordinate system"
```

---

## Task 2: LOD module

**Files:**
- Create: `src/lod/mod.rs`

This replaces the entire `src/scale/` module. No more opacity fading — just binary Visible/Hidden based on camera ortho_scale.

- [ ] **Step 1: Write failing tests**

Create `src/lod/mod.rs`:

```rust
use bevy::prelude::*;
use crate::camera::ZoomLevel;

#[derive(Component, Debug, Clone, Copy)]
pub struct LodRange {
    pub min_scale: f32,
    pub max_scale: f32,
}

pub struct LodPlugin;

impl Plugin for LodPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_lod_visibility);
    }
}

fn update_lod_visibility(
    zoom: Res<ZoomLevel>,
    mut query: Query<(&LodRange, &mut Visibility)>,
) {
    let scale = zoom.to_ortho_scale();
    for (lod, mut vis) in query.iter_mut() {
        *vis = if scale >= lod.min_scale && scale <= lod.max_scale {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;
    use crate::camera::ZoomLevel;

    #[test]
    fn entity_visible_within_lod_range() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
           .insert_resource(ZoomLevel { value: 0.0 }) // scale = 0.001
           .add_plugins(LodPlugin);

        let entity = app.world_mut().spawn((
            LodRange { min_scale: 0.0, max_scale: 0.05 },
            Visibility::Hidden,
        )).id();

        app.update();

        let vis = app.world().get::<Visibility>(entity).unwrap();
        assert_eq!(*vis, Visibility::Visible);
    }

    #[test]
    fn entity_hidden_above_lod_range() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
           .insert_resource(ZoomLevel { value: 1.0 }) // scale = ~16
           .add_plugins(LodPlugin);

        let entity = app.world_mut().spawn((
            LodRange { min_scale: 0.0, max_scale: 0.03 }, // buildings range
            Visibility::Visible,
        )).id();

        app.update();

        let vis = app.world().get::<Visibility>(entity).unwrap();
        assert_eq!(*vis, Visibility::Hidden);
    }

    #[test]
    fn entity_hidden_below_lod_range() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
           .insert_resource(ZoomLevel { value: 0.0 }) // scale = 0.001
           .add_plugins(LodPlugin);

        let entity = app.world_mut().spawn((
            LodRange { min_scale: 3.0, max_scale: 20.0 }, // solar range
            Visibility::Visible,
        )).id();

        app.update();

        let vis = app.world().get::<Visibility>(entity).unwrap();
        assert_eq!(*vis, Visibility::Hidden);
    }
}
```

- [ ] **Step 2: Declare `mod lod` in `src/main.rs`**

Open `src/main.rs`. Add `mod lod;` alongside the other `mod` declarations. Do NOT add `LodPlugin` yet — that is Task 8. The module just needs to compile.

Current `src/main.rs` starts with:
```rust
mod world_gen;
mod theme;
mod camera;
mod scale;
mod renderer;
```

Change to:
```rust
mod world_gen;
mod theme;
mod camera;
mod scale;
mod lod;
mod renderer;
```

- [ ] **Step 3: Run tests**

```bash
cd /Users/jamesbaxter/git/massivecityscale && cargo test lod
```

Expected: all 3 LOD tests PASS.

- [ ] **Step 4: Run all tests**

```bash
cd /Users/jamesbaxter/git/massivecityscale && cargo test
```

Expected: all tests pass (existing tests unaffected; LOD module doesn't affect scale module yet).

- [ ] **Step 5: Commit**

```bash
git add src/lod/mod.rs src/main.rs
git commit -m "feat: LodPlugin with LodRange component for distance-based visibility"
```

---

## Task 3: Camera zoom formula

**Files:**
- Modify: `src/camera/zoom.rs`

The new formula maps zoom value [0,1] to ortho_scale [0.001, ~16]:
- `0.001` at zoom 0.0 → camera sees 1.28 km total → buildings (25–50 m) appear as 25–50 px ✓
- `~16` at zoom 1.0 → camera sees ~20 000 km → solar system (±5000 km) fills screen ✓

Default zoom changes from 0.1 to 0.45, which maps to scale ≈ 0.078 (camera sees ~100 km, showing city blobs and block boxes).

- [ ] **Step 1: Update `to_ortho_scale` and default zoom**

Read `src/camera/zoom.rs`, then replace the `to_ortho_scale` implementation and `Default` impl:

```rust
impl ZoomLevel {
    pub fn to_ortho_scale(&self) -> f32 {
        // 0.001 at street (zoom 0) → ~16 at solar system (zoom 1)
        0.001 * (10.0f32).powf(self.value * 4.2)
    }
}

impl Default for ZoomLevel {
    fn default() -> Self {
        // zoom 0.45 → scale ≈ 0.078 → see ~100 km → city blobs visible on startup
        Self { value: 0.45 }
    }
}
```

Also update the three existing tests at the bottom of the file to match the new formula:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_clamps_to_range() {
        let mut zoom = ZoomLevel { value: 0.0 };
        zoom.value = (zoom.value - 100.0 * ZOOM_SPEED).clamp(ZOOM_MIN, ZOOM_MAX);
        assert_eq!(zoom.value, ZOOM_MIN);
        zoom.value = (zoom.value + 100.0 * ZOOM_SPEED).clamp(ZOOM_MIN, ZOOM_MAX);
        assert_eq!(zoom.value, ZOOM_MAX);
    }

    #[test]
    fn ortho_scale_increases_with_zoom() {
        let street = ZoomLevel { value: 0.0 };
        let solar = ZoomLevel { value: 1.0 };
        assert!(street.to_ortho_scale() < solar.to_ortho_scale());
    }

    #[test]
    fn street_scale_shows_buildings() {
        // At zoom 0, scale = 0.001, total view = 1.28 km.
        // Buildings at 25–50 m (0.025–0.050 km) → 25–50 px wide. Visible.
        let street = ZoomLevel { value: 0.0 };
        assert!(street.to_ortho_scale() < 0.01, "street ortho_scale should be < 0.01 to show buildings");
    }

    #[test]
    fn solar_scale_sees_solar_system() {
        // At zoom 1, scale ≈ 16, total view ≈ 20 000 km. Solar system (±5000 km) fits.
        let solar = ZoomLevel { value: 1.0 };
        assert!(solar.to_ortho_scale() > 5.0, "solar ortho_scale should be > 5.0 to see solar system");
    }

    #[test]
    fn default_zoom_is_mid_range() {
        let z = ZoomLevel::default();
        assert!(z.value > ZOOM_MIN && z.value < ZOOM_MAX);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cd /Users/jamesbaxter/git/massivecityscale && cargo test zoom
```

Expected: all 5 zoom tests PASS.

- [ ] **Step 3: Run all tests**

```bash
cd /Users/jamesbaxter/git/massivecityscale && cargo test
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/camera/zoom.rs
git commit -m "feat: recalibrate zoom formula 0.001..16 and default zoom 0.45"
```

---

## Task 4: Buildings renderer

**Files:**
- Rewrite: `src/renderer/buildings.rs`

Replaces the old buildings.rs completely. Three systems:
- `spawn_street_buildings` — individual buildings, real world positions
- `spawn_block_level` — one box per city block
- `spawn_city_level` — one slab per city

No more `Query<(Entity, &SpaceTag)>` — entities spawn directly at world coordinates with `LodRange`.

- [ ] **Step 1: Replace `src/renderer/buildings.rs` entirely**

```rust
use bevy::prelude::*;
use crate::world_gen::{WorldData, city::{CityData, DistrictData, BlockData}};
use crate::world_gen::city::BuildingData;
use crate::theme::Theme;
use crate::lod::LodRange;
use crate::renderer::scale_consts::*;

fn city_km(city: &CityData) -> (f32, f32) {
    (
        city.lat * CELL_KM - WORLD_HALF_KM,
        city.lon * CELL_KM - WORLD_HALF_KM,
    )
}

fn building_world_pos(
    city: &CityData,
    district: &DistrictData,
    block: &BlockData,
    building: &BuildingData,
) -> Vec3 {
    let (cx, cz) = city_km(city);
    let dx = (district.x - city.lat) / 3.0 * DIST_SCALE;
    let dz = (district.z - city.lon) / 3.0 * DIST_SCALE;
    let bx = (block.x - district.x) * BLOCK_SCALE;
    let bz = (block.z - district.z) * BLOCK_SCALE;
    let ox = (building.x - block.x) * BLDG_POS_SCALE;
    let oz = (building.z - block.z) * BLDG_POS_SCALE;
    let h = building.height * BLDG_H_SCALE;
    Vec3::new(cx + dx + bx + ox, h / 2.0, cz + dz + bz + oz)
}

fn block_world_pos(city: &CityData, district: &DistrictData, block: &BlockData) -> Vec3 {
    let (cx, cz) = city_km(city);
    let dx = (district.x - city.lat) / 3.0 * DIST_SCALE;
    let dz = (district.z - city.lon) / 3.0 * DIST_SCALE;
    let bx = (block.x - district.x) * BLOCK_SCALE;
    let bz = (block.z - district.z) * BLOCK_SCALE;
    Vec3::new(cx + dx + bx, 0.0, cz + dz + bz)
}

pub fn spawn_street_buildings(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    world: Res<WorldData>,
    theme: Res<Theme>,
) {
    for city in &world.cities {
        for district in &city.districts {
            for block in &district.blocks {
                for building in &block.buildings {
                    let color = if building.height > 1.5 {
                        theme.buildings.commercial
                    } else {
                        theme.buildings.residential
                    };
                    let w = building.width * BLDG_W_SCALE;
                    let h = building.height * BLDG_H_SCALE;
                    let d = building.depth * BLDG_W_SCALE;
                    let mesh = meshes.add(Cuboid::new(w, h, d));
                    let material = materials.add(StandardMaterial { base_color: color, ..default() });
                    let pos = building_world_pos(city, district, block, building);
                    commands.spawn((
                        Mesh3d(mesh),
                        MeshMaterial3d(material),
                        Transform::from_translation(pos),
                        LodRange { min_scale: LOD_BUILDINGS.0, max_scale: LOD_BUILDINGS.1 },
                        Visibility::Hidden,
                    ));
                }
            }
        }
    }
}

pub fn spawn_block_level(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    world: Res<WorldData>,
    theme: Res<Theme>,
) {
    for city in &world.cities {
        for district in &city.districts {
            for block in &district.blocks {
                let avg_height: f32 = if block.buildings.is_empty() {
                    0.5
                } else {
                    block.buildings.iter().map(|b| b.height).sum::<f32>()
                        / block.buildings.len() as f32
                };
                let box_height = avg_height * BLDG_H_SCALE;
                let mut pos = block_world_pos(city, district, block);
                pos.y = box_height / 2.0;
                let mesh = meshes.add(Cuboid::new(BLOCK_SCALE * 0.85, box_height, BLOCK_SCALE * 0.85));
                let material = materials.add(StandardMaterial {
                    // Block level uses uniform color; individual height variation averages out
                    base_color: theme.buildings.residential,
                    ..default()
                });
                commands.spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    Transform::from_translation(pos),
                    LodRange { min_scale: LOD_BLOCKS.0, max_scale: LOD_BLOCKS.1 },
                    Visibility::Hidden,
                ));
            }
        }
    }
}

pub fn spawn_city_level(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    world: Res<WorldData>,
    theme: Res<Theme>,
) {
    // City extent: 2 districts × DIST_SCALE + some block extent ≈ 4.5 km
    let city_size = 2.0 * DIST_SCALE + BLOCK_SCALE * 2.0;

    for city in &world.cities {
        let (cx, cz) = city_km(city);
        let mesh = meshes.add(Cuboid::new(city_size, 0.05, city_size));
        let material = materials.add(StandardMaterial {
            base_color: theme.buildings.commercial,
            ..default()
        });
        // Centre the blob over the city's content area
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_xyz(cx + city_size / 2.0, 0.025, cz + city_size / 2.0),
            LodRange { min_scale: LOD_CITIES.0, max_scale: LOD_CITIES.1 },
            Visibility::Hidden,
        ));
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cd /Users/jamesbaxter/git/massivecityscale && cargo test
```

The old scale tests will fail because `src/scale/` still exists and its tests reference `SpaceTag`. The renderer test may fail because it still registers `ScalePlugin`. **Ignore these for now** — they are fixed in Tasks 7 and 8. Focus only on whether `cargo build` succeeds:

```bash
cd /Users/jamesbaxter/git/massivecityscale && cargo build 2>&1 | grep "^error"
```

Expected: no `error` lines. Warnings are OK.

- [ ] **Step 3: Commit**

```bash
git add src/renderer/buildings.rs
git commit -m "feat: buildings/blocks/cities renderer in km coordinate system with LodRange"
```

---

## Task 5: Terrain renderer

**Files:**
- Rewrite: `src/renderer/terrain.rs`

Three systems:
- `spawn_ocean` — large flat plane covering the whole world surface
- `spawn_continent_patches` — one 10×10 km tile per land cell
- `spawn_planet_sphere` — sphere at origin visible at extreme zoom-out

- [ ] **Step 1: Replace `src/renderer/terrain.rs` entirely**

```rust
use bevy::prelude::*;
use crate::world_gen::{WorldData, planet::GRID_SIZE};
use crate::theme::Theme;
use crate::lod::LodRange;
use crate::renderer::scale_consts::*;

pub fn spawn_ocean(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    theme: Res<Theme>,
) {
    // Large flat plane covering the whole planet surface (640 km) plus margin
    let mesh = meshes.add(Plane3d::default().mesh().size(700.0, 700.0));
    let material = materials.add(StandardMaterial {
        base_color: theme.terrain.water,
        ..default()
    });
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, -0.01, 0.0),
        LodRange { min_scale: LOD_OCEAN.0, max_scale: LOD_OCEAN.1 },
        Visibility::Visible,
    ));
}

pub fn spawn_continent_patches(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    world: Res<WorldData>,
    theme: Res<Theme>,
) {
    for continent in &world.planet.continents {
        for (cx, cz) in &continent.cells {
            // Each grid cell is CELL_KM × CELL_KM km; offset to centre at origin
            let x = *cx as f32 * CELL_KM - WORLD_HALF_KM + CELL_KM / 2.0;
            let z = *cz as f32 * CELL_KM - WORLD_HALF_KM + CELL_KM / 2.0;
            let mesh = meshes.add(Cuboid::new(CELL_KM, 0.02, CELL_KM));
            let material = materials.add(StandardMaterial {
                base_color: theme.terrain.land_continent,
                ..default()
            });
            commands.spawn((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::from_xyz(x, 0.01, z),
                LodRange { min_scale: LOD_CONTINENTS.0, max_scale: LOD_CONTINENTS.1 },
                Visibility::Hidden,
            ));
        }
    }
}

pub fn spawn_planet_sphere(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    theme: Res<Theme>,
) {
    // Sphere radius slightly larger than WORLD_HALF_KM so it peeks around the flat map
    let mesh = meshes.add(Sphere::new(350.0).mesh().uv(32, 18));
    let material = materials.add(StandardMaterial {
        base_color: theme.solar.planet,
        ..default()
    });
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, -30.0, 0.0),
        LodRange { min_scale: LOD_PLANET.0, max_scale: LOD_PLANET.1 },
        Visibility::Hidden,
    ));
}
```

- [ ] **Step 2: Build check**

```bash
cd /Users/jamesbaxter/git/massivecityscale && cargo build 2>&1 | grep "^error"
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/renderer/terrain.rs
git commit -m "feat: ocean, continent patches, planet sphere in km coordinate system"
```

---

## Task 6: Solar system renderer

**Files:**
- Rewrite: `src/renderer/solar.rs`

Geometry at km scale:
- Orbital ring centred at origin, major_radius = 3 000 km
- Sun at (−5000, 0, 0), radius = 800 km
- Planet dot at the ring's top (0, 0, 3 000), radius = 250 km

At zoom 1.0 (ortho_scale ≈ 16, total view ≈ 20 480 km), the ring (±3 000 km) occupies ~37% of screen width and the sun (at −5000) is visible to the left. ✓

- [ ] **Step 1: Replace `src/renderer/solar.rs` entirely**

```rust
use bevy::prelude::*;
use crate::theme::Theme;
use crate::lod::LodRange;
use crate::renderer::scale_consts::LOD_SOLAR;

pub fn spawn_solar_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    theme: Res<Theme>,
) {
    // Sun — large emissive sphere to one side of the orbital ring
    let sun_mesh = meshes.add(Sphere::new(800.0).mesh().uv(32, 18));
    let sun_mat = materials.add(StandardMaterial {
        base_color: theme.solar.sun,
        emissive: LinearRgba::from(theme.solar.sun) * 2.0,
        ..default()
    });
    commands.spawn((
        Mesh3d(sun_mesh),
        MeshMaterial3d(sun_mat),
        Transform::from_xyz(-5000.0, 0.0, 0.0),
        LodRange { min_scale: LOD_SOLAR.0, max_scale: LOD_SOLAR.1 },
        Visibility::Hidden,
    ));

    // Orbital ring centred at world origin
    let ring_mesh = meshes.add(
        Torus { minor_radius: 50.0, major_radius: 3000.0 }
            .mesh()
            .minor_resolution(4)
            .major_resolution(64),
    );
    let ring_mat = materials.add(StandardMaterial {
        base_color: theme.solar.orbital_ring,
        ..default()
    });
    commands.spawn((
        Mesh3d(ring_mesh),
        MeshMaterial3d(ring_mat),
        Transform::IDENTITY,
        LodRange { min_scale: LOD_SOLAR.0, max_scale: LOD_SOLAR.1 },
        Visibility::Hidden,
    ));

    // Planet dot on the orbital ring
    let planet_mesh = meshes.add(Sphere::new(250.0).mesh().uv(16, 12));
    let planet_mat = materials.add(StandardMaterial {
        base_color: theme.solar.planet,
        ..default()
    });
    commands.spawn((
        Mesh3d(planet_mesh),
        MeshMaterial3d(planet_mat),
        Transform::from_xyz(0.0, 0.0, 3000.0),
        LodRange { min_scale: LOD_SOLAR.0, max_scale: LOD_SOLAR.1 },
        Visibility::Hidden,
    ));
}
```

- [ ] **Step 2: Build check**

```bash
cd /Users/jamesbaxter/git/massivecityscale && cargo build 2>&1 | grep "^error"
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/renderer/solar.rs
git commit -m "feat: solar system at km scale (ring 3000 km, sun at -5000 km)"
```

---

## Task 7: Renderer plugin + delete scale module

**Files:**
- Rewrite: `src/renderer/mod.rs`
- Delete: `src/scale/mod.rs`, `src/scale/spaces.rs`, `src/scale/visibility.rs`
- Modify: `src/main.rs`

The RendererPlugin no longer needs SpaceTags. The test no longer imports `ScalePlugin`. The `src/scale/` directory and its module declaration in `src/main.rs` are removed.

- [ ] **Step 1: Replace `src/renderer/mod.rs`**

```rust
pub mod scale_consts;
pub mod buildings;
pub mod terrain;
pub mod solar;

use bevy::prelude::*;

pub struct RendererPlugin;

impl Plugin for RendererPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (
            terrain::spawn_ocean,
            buildings::spawn_street_buildings,
            buildings::spawn_block_level,
            buildings::spawn_city_level,
            terrain::spawn_continent_patches,
            terrain::spawn_planet_sphere,
            solar::spawn_solar_system,
        ).chain());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;
    use crate::{world_gen::WorldGenPlugin, theme::ThemePlugin, camera::CameraPlugin, lod::LodPlugin};

    #[test]
    fn renderer_plugin_spawns_mesh_entities() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
           .add_plugins(bevy::input::InputPlugin)
           .add_plugins(AssetPlugin::default())
           .init_asset::<Mesh>()
           .init_asset::<StandardMaterial>()
           .insert_resource(ClearColor(Color::BLACK))
           .add_plugins(WorldGenPlugin)
           .add_plugins(ThemePlugin)
           .add_plugins(CameraPlugin)
           .add_plugins(LodPlugin)
           .add_plugins(RendererPlugin);
        app.update();
        let mesh_count = app.world_mut()
            .query::<&Mesh3d>()
            .iter(app.world())
            .count();
        assert!(mesh_count > 0, "no mesh entities were spawned");
    }
}
```

- [ ] **Step 2: Delete the scale module files**

```bash
rm /Users/jamesbaxter/git/massivecityscale/src/scale/mod.rs
rm /Users/jamesbaxter/git/massivecityscale/src/scale/spaces.rs
rm /Users/jamesbaxter/git/massivecityscale/src/scale/visibility.rs
rmdir /Users/jamesbaxter/git/massivecityscale/src/scale
```

- [ ] **Step 3: Update `src/main.rs`**

Replace the entire file with:

```rust
mod world_gen;
mod theme;
mod camera;
mod lod;
mod renderer;

use bevy::prelude::*;
use world_gen::WorldGenPlugin;
use theme::ThemePlugin;
use camera::CameraPlugin;
use lod::LodPlugin;
use renderer::RendererPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "MassiveCityScale".into(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(theme::themes::dark_neon().background))
        .add_plugins(WorldGenPlugin)
        .add_plugins(ThemePlugin)
        .add_plugins(CameraPlugin)
        .add_plugins(LodPlugin)
        .add_plugins(RendererPlugin)
        .add_systems(Startup, spawn_light)
        .add_systems(Update, quit_on_escape)
        .run();
}

fn spawn_light(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(1.0, 2.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 3_000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(-1.0, 0.5, -1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn quit_on_escape(
    keys: Res<ButtonInput<KeyCode>>,
    mut exit: EventWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.send(AppExit::Success);
    }
}
```

- [ ] **Step 4: Run all tests**

```bash
cd /Users/jamesbaxter/git/massivecityscale && cargo test
```

Expected: tests from scale module are gone (that module is deleted). The remaining tests all pass. The total count will be lower (was 22, losing 4 scale tests, gaining 3 LOD tests, so expect ~21 tests).

- [ ] **Step 5: Build check — no warnings about unused imports**

```bash
cd /Users/jamesbaxter/git/massivecityscale && cargo build 2>&1
```

Expected: 0 errors, minimal warnings (the `#[allow(dead_code)]` annotations on RoadData etc. handle the known ones).

- [ ] **Step 6: Commit**

```bash
git add src/renderer/mod.rs src/main.rs
git rm src/scale/mod.rs src/scale/spaces.rs src/scale/visibility.rs
git commit -m "feat: unified LOD-based renderer; delete parallel-spaces scale module"
```

---

## Self-Review

### Spec coverage

- [x] Single coordinate system (1 unit = 1 km) — all renderers use scale_consts
- [x] LOD visibility by ortho_scale — `LodPlugin` + `LodRange` on every entity
- [x] Street buildings visible when zoomed in — LOD_BUILDINGS (0.0, 0.03), scale 0.001 at zoom 0
- [x] Block-level boxes — LOD_BLOCKS (0.005, 0.20)
- [x] City blobs — LOD_CITIES (0.05, 1.2)
- [x] Ocean/ground — LOD_OCEAN (0.0, 4.0), always visible
- [x] Continent patches — LOD_CONTINENTS (0.04, 4.0)
- [x] Planet sphere — LOD_PLANET (0.8, 20.0)
- [x] Solar system — LOD_SOLAR (3.0, 20.0)
- [x] Camera zoom formula calibrated — Task 3
- [x] Scale module deleted — Task 7
- [x] world_gen untouched — confirmed, no world_gen changes in any task

### Placeholder scan

None found.

### Type consistency

- `LodRange` defined in Task 2, used in Tasks 4–6 via `crate::lod::LodRange` ✓
- `LOD_*` constants defined in Task 1 (`scale_consts`), used in Tasks 4–6 via `use crate::renderer::scale_consts::*` ✓
- `city_km()`, `building_world_pos()`, `block_world_pos()` — all defined and used within Task 4 ✓
- `spawn_ocean` used in Task 7's `RendererPlugin` (renamed from old `spawn_ground` + `spawn_water_plane`) ✓
- Renderer test in Task 7 uses `LodPlugin` not `ScalePlugin` ✓
