# MassiveCityScale World Renderer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Bevy/Rust app that renders a procedurally generated world you can zoom continuously from street level to solar system, with an isometric camera, smooth opacity-based level transitions, and a swappable theme system.

**Architecture:** Eight nested coordinate spaces (one per zoom level) live in a Bevy transform hierarchy. A `ZoomLevel` f32 resource (0.0 = street, 1.0 = solar system) drives camera scale and the opacity of each space. All world content is generated once at startup from a fixed seed into pure data structs, then spawned into Bevy entities.

**Tech Stack:** Rust, Bevy (latest stable, currently 0.15), `noise` 0.9 (simplex), `rand` 0.8 + `rand_chacha` 0.3 (seeded RNG)

---

## File Map

| File | Responsibility |
|---|---|
| `Cargo.toml` | Dependencies |
| `src/main.rs` | App setup, plugin registration |
| `src/world_gen/mod.rs` | `WorldGenPlugin`, `WorldData` resource, seed config |
| `src/world_gen/planet.rs` | `PlanetData` — continent outlines via simplex noise |
| `src/world_gen/city.rs` | `CityData`, `DistrictData`, `BlockData` — Poisson disk + grid |
| `src/world_gen/street.rs` | `BuildingData`, `RoadData` — per-block contents |
| `src/theme/mod.rs` | `Theme` resource, `ThemePlugin`, color lookup helpers |
| `src/theme/themes.rs` | `dark_neon()`, `clean_pastel()`, `earthy()` theme constructors |
| `src/camera/mod.rs` | `CameraPlugin` |
| `src/camera/zoom.rs` | `ZoomLevel` resource, scroll input system, pan system |
| `src/scale/mod.rs` | `ScalePlugin`, `ScaleSpace` enum, `SpaceTag` component |
| `src/scale/spaces.rs` | Spawn 8 parent entities in transform hierarchy |
| `src/scale/visibility.rs` | Per-frame opacity update system |
| `src/renderer/mod.rs` | `RendererPlugin` |
| `src/renderer/buildings.rs` | Box meshes for buildings and block-level boxes |
| `src/renderer/terrain.rs` | Ground quads, water planes, world flat plane, planet sphere |
| `src/renderer/solar.rs` | Sun sphere, orbital ring, planet dot |

---

## Task 1: Bootstrap the Rust/Bevy project

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `assets/.gitkeep`

- [ ] **Step 1: Initialise the Cargo project**

```bash
cd /Users/jamesbaxter/git/massivecityscale
cargo init --name massivecityscale
```

Expected: `src/main.rs` created with `fn main() { println!("Hello, world!"); }`

- [ ] **Step 2: Replace Cargo.toml with project dependencies**

Replace the entire `Cargo.toml` with:

```toml
[package]
name = "massivecityscale"
version = "0.1.0"
edition = "2021"

[dependencies]
bevy = { version = "0.15", features = ["default"] }
noise = "0.9"
rand = "0.8"
rand_chacha = "0.3"

[profile.dev]
opt-level = 1

[profile.dev.package."*"]
opt-level = 3
```

The `opt-level` overrides keep Bevy dev builds fast without full release mode.

- [ ] **Step 3: Replace src/main.rs with a minimal Bevy app**

```rust
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .run();
}
```

- [ ] **Step 4: Create assets directory**

```bash
mkdir -p assets && touch assets/.gitkeep
```

- [ ] **Step 5: Verify it compiles and opens a window**

```bash
cargo run
```

Expected: a blank Bevy window opens (dark grey). Press Escape or close the window to exit. Compilation will be slow the first time (~2 min). Subsequent runs are fast.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/main.rs assets/.gitkeep
git commit -m "feat: bootstrap bevy project"
```

---

## Task 2: World gen data types

**Files:**
- Create: `src/world_gen/mod.rs`
- Create: `src/world_gen/planet.rs`
- Create: `src/world_gen/city.rs`
- Create: `src/world_gen/street.rs`

These are pure data structs — no Bevy, no rendering.

- [ ] **Step 1: Write the failing test**

Add to the bottom of `src/world_gen/mod.rs` (create the file):

```rust
pub mod planet;
pub mod city;
pub mod street;

pub const WORLD_SEED: u64 = 42;

#[derive(Debug)]
pub struct WorldData {
    pub planet: planet::PlanetData,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_data_holds_planet() {
        let planet = planet::PlanetData { continents: vec![] };
        let world = WorldData { planet };
        assert_eq!(world.planet.continents.len(), 0);
    }
}
```

Create `src/world_gen/planet.rs`:

```rust
#[derive(Debug, Clone)]
pub struct ContinentData {
    pub cells: Vec<(i32, i32)>,
}

#[derive(Debug)]
pub struct PlanetData {
    pub continents: Vec<ContinentData>,
}
```

Create `src/world_gen/city.rs`:

```rust
#[derive(Debug, Clone)]
pub struct BuildingData {
    pub x: f32,
    pub z: f32,
    pub width: f32,
    pub depth: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct RoadData {
    pub x: f32,
    pub z: f32,
    pub length: f32,
    pub horizontal: bool,
}

#[derive(Debug, Clone)]
pub struct BlockData {
    pub x: f32,
    pub z: f32,
    pub buildings: Vec<BuildingData>,
    pub roads: Vec<RoadData>,
}

#[derive(Debug, Clone)]
pub struct DistrictData {
    pub x: f32,
    pub z: f32,
    pub blocks: Vec<BlockData>,
}

#[derive(Debug, Clone)]
pub struct CityData {
    pub lat: f32,
    pub lon: f32,
    pub districts: Vec<DistrictData>,
}
```

Create `src/world_gen/street.rs`:

```rust
// Re-exports used by the renderer
pub use crate::world_gen::city::{BuildingData, RoadData, BlockData};
```

- [ ] **Step 2: Run the test to verify it fails (or passes at struct level)**

```bash
cargo test world_data_holds_planet
```

Expected: PASS (this tests struct construction, which should work once files exist).

- [ ] **Step 3: Add module to main.rs**

Add to `src/main.rs`:

```rust
mod world_gen;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .run();
}
```

- [ ] **Step 4: Verify all tests pass**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/world_gen/ src/main.rs
git commit -m "feat: world gen data types"
```

---

## Task 3: Planet generation — land/water mask

**Files:**
- Modify: `src/world_gen/planet.rs`

Generate a grid of land/water cells using simplex noise, then group contiguous land cells into continents.

- [ ] **Step 1: Write failing tests**

Replace `src/world_gen/planet.rs` with (the structs from Task 2 are included here, plus the full implementation):

```rust
use noise::{NoiseFn, Simplex};

pub const GRID_SIZE: i32 = 64;
pub const LAND_THRESHOLD: f64 = 0.0;

#[derive(Debug, Clone)]
pub struct ContinentData {
    pub cells: Vec<(i32, i32)>,
}

#[derive(Debug)]
pub struct PlanetData {
    pub continents: Vec<ContinentData>,
}

pub fn generate(seed: u64) -> PlanetData {
    let noise = Simplex::new(seed as u32);
    let mut land: Vec<Vec<bool>> = vec![vec![false; GRID_SIZE as usize]; GRID_SIZE as usize];

    for x in 0..GRID_SIZE {
        for z in 0..GRID_SIZE {
            let nx = x as f64 / GRID_SIZE as f64 * 4.0;
            let nz = z as f64 / GRID_SIZE as f64 * 4.0;
            land[x as usize][z as usize] = noise.get([nx, nz]) > LAND_THRESHOLD;
        }
    }

    let continents = flood_fill_continents(&land);
    PlanetData { continents }
}

fn flood_fill_continents(land: &Vec<Vec<bool>>) -> Vec<ContinentData> {
    let size = land.len();
    let mut visited = vec![vec![false; size]; size];
    let mut continents = Vec::new();

    for x in 0..size {
        for z in 0..size {
            if land[x][z] && !visited[x][z] {
                let cells = bfs(land, &mut visited, x, z, size);
                if cells.len() >= 4 {
                    continents.push(ContinentData { cells });
                }
            }
        }
    }
    continents
}

fn bfs(land: &Vec<Vec<bool>>, visited: &mut Vec<Vec<bool>>, sx: usize, sz: usize, size: usize) -> Vec<(i32, i32)> {
    let mut queue = std::collections::VecDeque::new();
    let mut cells = Vec::new();
    queue.push_back((sx, sz));
    visited[sx][sz] = true;

    while let Some((x, z)) = queue.pop_front() {
        cells.push((x as i32, z as i32));
        for (dx, dz) in [(-1i32, 0), (1, 0), (0, -1i32), (0, 1)] {
            let nx = x as i32 + dx;
            let nz = z as i32 + dz;
            if nx >= 0 && nx < size as i32 && nz >= 0 && nz < size as i32 {
                let (nx, nz) = (nx as usize, nz as usize);
                if land[nx][nz] && !visited[nx][nz] {
                    visited[nx][nz] = true;
                    queue.push_back((nx, nz));
                }
            }
        }
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planet_has_continents() {
        let planet = generate(42);
        assert!(!planet.continents.is_empty(), "seed 42 should produce at least one continent");
    }

    #[test]
    fn continents_have_minimum_size() {
        let planet = generate(42);
        for c in &planet.continents {
            assert!(c.cells.len() >= 4, "all continents must have at least 4 cells");
        }
    }

    #[test]
    fn planet_generation_is_deterministic() {
        let a = generate(42);
        let b = generate(42);
        assert_eq!(a.continents.len(), b.continents.len());
        assert_eq!(a.continents[0].cells.len(), b.continents[0].cells.len());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test planet
```

Expected: compilation error — `generate` not defined yet. (We wrote the implementation above directly, so it should compile and pass on first run.)

- [ ] **Step 3: Run tests**

```bash
cargo test planet
```

Expected: all 3 planet tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src/world_gen/planet.rs
git commit -m "feat: planet generation with simplex noise and continent detection"
```

---

## Task 4: City placement via Poisson disk sampling

**Files:**
- Modify: `src/world_gen/city.rs`
- Modify: `src/world_gen/mod.rs`

Place cities on land cells, minimum distance apart, using Poisson disk sampling. Generate empty district/block structure per city (buildings populated in Task 5).

- [ ] **Step 1: Write failing tests**

Add to the bottom of `src/world_gen/city.rs`:

```rust
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use crate::world_gen::planet::PlanetData;

pub const CITY_COUNT_TARGET: usize = 8;
pub const CITY_MIN_DISTANCE: f32 = 6.0;
pub const DISTRICTS_PER_CITY: usize = 4;
pub const BLOCKS_PER_DISTRICT: usize = 9;

pub fn generate(planet: &PlanetData, seed: u64) -> Vec<CityData> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed.wrapping_add(1));
    let land_cells: Vec<(f32, f32)> = planet.continents.iter()
        .flat_map(|c| c.cells.iter().map(|(x, z)| (*x as f32, *z as f32)))
        .collect();

    if land_cells.is_empty() {
        return vec![];
    }

    let mut cities: Vec<CityData> = Vec::new();
    let mut attempts = 0;

    while cities.len() < CITY_COUNT_TARGET && attempts < 1000 {
        attempts += 1;
        let idx = rng.gen_range(0..land_cells.len());
        let (lat, lon) = land_cells[idx];

        let too_close = cities.iter().any(|c| {
            let dx = c.lat - lat;
            let dz = c.lon - lon;
            (dx * dx + dz * dz).sqrt() < CITY_MIN_DISTANCE
        });

        if !too_close {
            let districts = generate_districts(&mut rng, lat, lon);
            cities.push(CityData { lat, lon, districts });
        }
    }

    cities
}

fn generate_districts(rng: &mut ChaCha8Rng, city_lat: f32, city_lon: f32) -> Vec<DistrictData> {
    let mut districts = Vec::new();
    let side = (DISTRICTS_PER_CITY as f32).sqrt() as i32;
    for dx in 0..side {
        for dz in 0..side {
            let x = city_lat + dx as f32 * 3.0;
            let z = city_lon + dz as f32 * 3.0;
            let blocks = generate_blocks(rng, x, z);
            districts.push(DistrictData { x, z, blocks });
        }
    }
    districts
}

fn generate_blocks(rng: &mut ChaCha8Rng, district_x: f32, district_z: f32) -> Vec<BlockData> {
    let mut blocks = Vec::new();
    let side = (BLOCKS_PER_DISTRICT as f32).sqrt() as i32;
    for bx in 0..side {
        for bz in 0..side {
            let x = district_x + bx as f32;
            let z = district_z + bz as f32;
            blocks.push(BlockData { x, z, buildings: vec![], roads: vec![] });
        }
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_gen::planet;

    fn test_planet() -> PlanetData {
        planet::generate(42)
    }

    #[test]
    fn cities_placed_on_land() {
        let planet = test_planet();
        let land_set: std::collections::HashSet<(i32, i32)> = planet.continents.iter()
            .flat_map(|c| c.cells.iter().cloned())
            .collect();
        let cities = generate(&planet, 42);
        for city in &cities {
            assert!(
                land_set.contains(&(city.lat as i32, city.lon as i32)),
                "city at ({}, {}) is not on land", city.lat, city.lon
            );
        }
    }

    #[test]
    fn cities_are_minimum_distance_apart() {
        let planet = test_planet();
        let cities = generate(&planet, 42);
        for i in 0..cities.len() {
            for j in (i + 1)..cities.len() {
                let dx = cities[i].lat - cities[j].lat;
                let dz = cities[i].lon - cities[j].lon;
                let dist = (dx * dx + dz * dz).sqrt();
                assert!(dist >= CITY_MIN_DISTANCE, "cities {} and {} are too close: {}", i, j, dist);
            }
        }
    }

    #[test]
    fn city_generation_is_deterministic() {
        let planet = test_planet();
        let a = generate(&planet, 42);
        let b = generate(&planet, 42);
        assert_eq!(a.len(), b.len());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test city
```

Expected: compilation errors until implementation is added. (We wrote implementation above directly.)

- [ ] **Step 3: Run tests**

```bash
cargo test city
```

Expected: all 3 city tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src/world_gen/city.rs
git commit -m "feat: city placement via poisson disk sampling"
```

---

## Task 5: Building and road generation per block

**Files:**
- Modify: `src/world_gen/city.rs`
- Modify: `src/world_gen/street.rs`

Populate each `BlockData` with buildings (noise-driven heights) and road quads.

- [ ] **Step 1: Write failing tests**

Replace `src/world_gen/street.rs` with:

```rust
use noise::{NoiseFn, Simplex};
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use crate::world_gen::city::BlockData;

pub use crate::world_gen::city::{BuildingData, RoadData};

pub const MIN_BUILDING_HEIGHT: f32 = 0.2;
pub const MAX_BUILDING_HEIGHT: f32 = 3.0;
pub const BUILDINGS_PER_BLOCK: usize = 4;

pub fn populate_block(block: &mut BlockData, seed: u64) {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let height_noise = Simplex::new(seed as u32);

    for i in 0..BUILDINGS_PER_BLOCK {
        let bx = block.x + rng.gen::<f32>() * 0.8;
        let bz = block.z + rng.gen::<f32>() * 0.8;
        let width = 0.1 + rng.gen::<f32>() * 0.15;
        let depth = 0.1 + rng.gen::<f32>() * 0.15;
        let noise_val = height_noise.get([bx as f64, bz as f64]);
        let t = (noise_val + 1.0) / 2.0;
        let height = MIN_BUILDING_HEIGHT + t as f32 * (MAX_BUILDING_HEIGHT - MIN_BUILDING_HEIGHT);

        block.buildings.push(BuildingData { x: bx, z: bz, width, depth, height });

        // Road along x axis for each row
        if i == 0 {
            block.roads.push(RoadData {
                x: block.x,
                z: block.z - 0.05,
                length: 1.0,
                horizontal: true,
            });
            block.roads.push(RoadData {
                x: block.x - 0.05,
                z: block.z,
                length: 1.0,
                horizontal: false,
            });
        }
    }
}

pub fn populate_all_blocks(cities: &mut Vec<crate::world_gen::city::CityData>, seed: u64) {
    for city in cities.iter_mut() {
        for district in city.districts.iter_mut() {
            for block in district.blocks.iter_mut() {
                let block_seed = seed
                    .wrapping_add((block.x * 1000.0) as u64)
                    .wrapping_add((block.z * 997.0) as u64);
                populate_block(block, block_seed);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_gen::city::{BlockData};

    fn empty_block() -> BlockData {
        BlockData { x: 1.0, z: 1.0, buildings: vec![], roads: vec![] }
    }

    #[test]
    fn block_gets_correct_building_count() {
        let mut block = empty_block();
        populate_block(&mut block, 42);
        assert_eq!(block.buildings.len(), BUILDINGS_PER_BLOCK);
    }

    #[test]
    fn building_heights_in_range() {
        let mut block = empty_block();
        populate_block(&mut block, 42);
        for b in &block.buildings {
            assert!(b.height >= MIN_BUILDING_HEIGHT, "height {} below min", b.height);
            assert!(b.height <= MAX_BUILDING_HEIGHT, "height {} above max", b.height);
        }
    }

    #[test]
    fn block_gets_roads() {
        let mut block = empty_block();
        populate_block(&mut block, 42);
        assert!(!block.roads.is_empty());
    }

    #[test]
    fn block_population_is_deterministic() {
        let mut a = empty_block();
        let mut b = empty_block();
        populate_block(&mut a, 42);
        populate_block(&mut b, 42);
        assert_eq!(a.buildings.len(), b.buildings.len());
        assert!((a.buildings[0].height - b.buildings[0].height).abs() < f32::EPSILON);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test street
```

Expected: all 4 tests PASS.

- [ ] **Step 3: Commit**

```bash
git add src/world_gen/street.rs
git commit -m "feat: building and road generation per block"
```

---

## Task 6: WorldData resource and WorldGenPlugin

**Files:**
- Modify: `src/world_gen/mod.rs`
- Modify: `src/main.rs`

Wire the generators into a Bevy `Resource` that gets created at startup.

- [ ] **Step 1: Write failing test**

Replace `src/world_gen/mod.rs` with:

```rust
pub mod planet;
pub mod city;
pub mod street;

use bevy::prelude::*;

pub const WORLD_SEED: u64 = 42;

#[derive(Resource)]
pub struct WorldData {
    pub planet: planet::PlanetData,
    pub cities: Vec<city::CityData>,
}

pub struct WorldGenPlugin;

impl Plugin for WorldGenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, generate_world);
    }
}

fn generate_world(mut commands: Commands) {
    let planet = planet::generate(WORLD_SEED);
    let mut cities = city::generate(&planet, WORLD_SEED);
    street::populate_all_blocks(&mut cities, WORLD_SEED);
    commands.insert_resource(WorldData { planet, cities });
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    #[test]
    fn world_gen_plugin_inserts_resource() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
           .add_plugins(WorldGenPlugin);
        app.update();
        assert!(app.world().get_resource::<WorldData>().is_some());
    }

    #[test]
    fn generated_world_has_cities_with_buildings() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
           .add_plugins(WorldGenPlugin);
        app.update();
        let world = app.world().resource::<WorldData>();
        assert!(!world.cities.is_empty());
        let first_city = &world.cities[0];
        let first_block = &first_city.districts[0].blocks[0];
        assert!(!first_block.buildings.is_empty());
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test world_gen
```

Expected: both tests PASS.

- [ ] **Step 3: Register the plugin in main.rs**

```rust
mod world_gen;

use bevy::prelude::*;
use world_gen::WorldGenPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WorldGenPlugin)
        .run();
}
```

- [ ] **Step 4: Verify the app still runs**

```bash
cargo run
```

Expected: window opens, no panics.

- [ ] **Step 5: Commit**

```bash
git add src/world_gen/mod.rs src/main.rs
git commit -m "feat: WorldGenPlugin creates WorldData resource at startup"
```

---

## Task 7: Theme system

**Files:**
- Create: `src/theme/mod.rs`
- Create: `src/theme/themes.rs`

- [ ] **Step 1: Write failing tests**

Create `src/theme/mod.rs`:

```rust
pub mod themes;

use bevy::prelude::*;

#[derive(Clone, Debug)]
pub struct BuildingColors {
    pub residential: Color,
    pub commercial: Color,
    pub park: Color,
}

#[derive(Clone, Debug)]
pub struct TerrainColors {
    pub ground: Color,
    pub water: Color,
    pub land_continent: Color,
}

#[derive(Clone, Debug)]
pub struct SolarColors {
    pub sun: Color,
    pub planet: Color,
    pub orbital_ring: Color,
}

#[derive(Resource, Clone, Debug)]
pub struct Theme {
    pub name: &'static str,
    pub background: Color,
    pub buildings: BuildingColors,
    pub terrain: TerrainColors,
    pub solar: SolarColors,
}

pub struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(themes::dark_neon())
           .add_systems(Update, cycle_theme_on_t);
    }
}

fn cycle_theme_on_t(
    keys: Res<ButtonInput<KeyCode>>,
    mut theme: ResMut<Theme>,
    mut clear_color: ResMut<ClearColor>,
) {
    if keys.just_pressed(KeyCode::KeyT) {
        let next = match theme.name {
            "dark_neon" => themes::clean_pastel(),
            "clean_pastel" => themes::earthy(),
            _ => themes::dark_neon(),
        };
        clear_color.0 = next.background;
        *theme = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    #[test]
    fn theme_plugin_inserts_default_theme() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
           .add_plugins(ThemePlugin);
        app.update();
        let theme = app.world().resource::<Theme>();
        assert_eq!(theme.name, "dark_neon");
    }

    #[test]
    fn all_three_themes_construct_without_panic() {
        let _ = themes::dark_neon();
        let _ = themes::clean_pastel();
        let _ = themes::earthy();
    }
}
```

Create `src/theme/themes.rs`:

```rust
use bevy::prelude::*;
use super::{Theme, BuildingColors, TerrainColors, SolarColors};

pub fn dark_neon() -> Theme {
    Theme {
        name: "dark_neon",
        background: Color::srgb(0.03, 0.05, 0.06),
        buildings: BuildingColors {
            residential: Color::srgb(0.48, 0.33, 0.58),
            commercial: Color::srgb(0.13, 0.47, 0.24),
            park: Color::srgb(0.10, 0.35, 0.15),
        },
        terrain: TerrainColors {
            ground: Color::srgb(0.06, 0.10, 0.06),
            water: Color::srgb(0.04, 0.08, 0.18),
            land_continent: Color::srgb(0.10, 0.20, 0.10),
        },
        solar: SolarColors {
            sun: Color::srgb(0.96, 0.62, 0.04),
            planet: Color::srgb(0.10, 0.20, 0.30),
            orbital_ring: Color::srgb(0.16, 0.16, 0.28),
        },
    }
}

pub fn clean_pastel() -> Theme {
    Theme {
        name: "clean_pastel",
        background: Color::srgb(0.94, 0.96, 0.98),
        buildings: BuildingColors {
            residential: Color::srgb(0.75, 0.86, 0.99),
            commercial: Color::srgb(0.73, 0.97, 0.81),
            park: Color::srgb(0.86, 0.99, 0.73),
        },
        terrain: TerrainColors {
            ground: Color::srgb(0.88, 0.91, 0.85),
            water: Color::srgb(0.68, 0.85, 0.97),
            land_continent: Color::srgb(0.78, 0.88, 0.72),
        },
        solar: SolarColors {
            sun: Color::srgb(1.0, 0.87, 0.40),
            planet: Color::srgb(0.60, 0.78, 0.95),
            orbital_ring: Color::srgb(0.80, 0.80, 0.90),
        },
    }
}

pub fn earthy() -> Theme {
    Theme {
        name: "earthy",
        background: Color::srgb(0.11, 0.10, 0.08),
        buildings: BuildingColors {
            residential: Color::srgb(0.49, 0.40, 0.27),
            commercial: Color::srgb(0.29, 0.42, 0.23),
            park: Color::srgb(0.22, 0.34, 0.16),
        },
        terrain: TerrainColors {
            ground: Color::srgb(0.18, 0.15, 0.10),
            water: Color::srgb(0.08, 0.14, 0.22),
            land_continent: Color::srgb(0.23, 0.20, 0.14),
        },
        solar: SolarColors {
            sun: Color::srgb(0.95, 0.75, 0.30),
            planet: Color::srgb(0.25, 0.32, 0.20),
            orbital_ring: Color::srgb(0.22, 0.20, 0.16),
        },
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test theme
```

Expected: both theme tests PASS.

- [ ] **Step 3: Add ThemePlugin to main.rs**

```rust
mod world_gen;
mod theme;

use bevy::prelude::*;
use world_gen::WorldGenPlugin;
use theme::ThemePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WorldGenPlugin)
        .add_plugins(ThemePlugin)
        .run();
}
```

- [ ] **Step 4: Run app and verify T key cycles theme (background color changes)**

```bash
cargo run
```

Press **T** — the background color should cycle between dark, light, and warm-dark. Press Escape to exit.

- [ ] **Step 5: Commit**

```bash
git add src/theme/ src/main.rs
git commit -m "feat: theme system with dark_neon, clean_pastel, earthy; T key cycles"
```

---

## Task 8: ZoomLevel resource and isometric camera

**Files:**
- Create: `src/camera/mod.rs`
- Create: `src/camera/zoom.rs`

- [ ] **Step 1: Write failing tests**

Create `src/camera/mod.rs`:

```rust
pub mod zoom;

use bevy::prelude::*;
pub use zoom::ZoomLevel;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ZoomLevel::default())
           .add_systems(Startup, spawn_camera)
           .add_systems(Update, (zoom::handle_scroll, zoom::handle_pan));
    }
}

fn spawn_camera(mut commands: Commands) {
    let scale = ZoomLevel::default().to_ortho_scale();
    commands.spawn((
        Camera3d::default(),
        Projection::Orthographic(OrthographicProjection {
            scale,
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_xyz(50.0, 50.0, 50.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
```

Create `src/camera/zoom.rs`:

```rust
use bevy::prelude::*;
use bevy::input::mouse::{MouseWheel, MouseMotion};

pub const ZOOM_SPEED: f32 = 0.05;
pub const ZOOM_MIN: f32 = 0.0;
pub const ZOOM_MAX: f32 = 1.0;
pub const PAN_SPEED: f32 = 0.5;

#[derive(Resource, Debug, Clone)]
pub struct ZoomLevel {
    pub value: f32,
}

impl Default for ZoomLevel {
    fn default() -> Self {
        Self { value: 0.4 }
    }
}

impl ZoomLevel {
    pub fn to_ortho_scale(&self) -> f32 {
        // Exponential: 1.0 at street (0.0) to 100_000.0 at solar (1.0)
        (10.0f32).powf(self.value * 5.0)
    }
}

pub fn handle_scroll(
    mut scroll: EventReader<MouseWheel>,
    mut zoom: ResMut<ZoomLevel>,
    mut proj_query: Query<&mut Projection, With<Camera3d>>,
) {
    for ev in scroll.read() {
        zoom.value = (zoom.value - ev.y * ZOOM_SPEED).clamp(ZOOM_MIN, ZOOM_MAX);
    }
    if let Ok(mut proj) = proj_query.get_single_mut() {
        if let Projection::Orthographic(ref mut ortho) = *proj {
            ortho.scale = zoom.to_ortho_scale();
        }
    }
}

pub fn handle_pan(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut motion: EventReader<MouseMotion>,
    mut cam_query: Query<&mut Transform, With<Camera3d>>,
    zoom: Res<ZoomLevel>,
) {
    if !mouse_buttons.pressed(MouseButton::Left) {
        motion.clear();
        return;
    }
    let scale = zoom.to_ortho_scale();
    for ev in motion.read() {
        if let Ok(mut transform) = cam_query.get_single_mut() {
            let right = transform.right().as_vec3();
            let up = transform.up().as_vec3();
            transform.translation -= right * ev.delta.x * PAN_SPEED * scale * 0.001;
            transform.translation += up * ev.delta.y * PAN_SPEED * scale * 0.001;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_clamps_to_range() {
        let mut zoom = ZoomLevel { value: 0.0 };
        // simulate scrolling way out
        zoom.value = (zoom.value - 100.0 * ZOOM_SPEED).clamp(ZOOM_MIN, ZOOM_MAX);
        assert_eq!(zoom.value, ZOOM_MIN);
        // simulate scrolling way in
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
    fn default_zoom_is_mid_range() {
        let z = ZoomLevel::default();
        assert!(z.value > ZOOM_MIN && z.value < ZOOM_MAX);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test zoom
```

Expected: all 3 zoom tests PASS.

- [ ] **Step 3: Add CameraPlugin to main.rs**

```rust
mod world_gen;
mod theme;
mod camera;

use bevy::prelude::*;
use world_gen::WorldGenPlugin;
use theme::ThemePlugin;
use camera::CameraPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WorldGenPlugin)
        .add_plugins(ThemePlugin)
        .add_plugins(CameraPlugin)
        .run();
}
```

- [ ] **Step 4: Verify camera in app**

```bash
cargo run
```

Expected: blank window from isometric angle (camera at 50, 50, 50 looking at origin). Scroll wheel should change the ortho scale (nothing visible yet to confirm zoom, but no panic).

- [ ] **Step 5: Commit**

```bash
git add src/camera/ src/main.rs
git commit -m "feat: isometric camera with scroll zoom and click-drag pan"
```

---

## Task 9: Scale space entities

**Files:**
- Create: `src/scale/mod.rs`
- Create: `src/scale/spaces.rs`
- Create: `src/scale/visibility.rs`

- [ ] **Step 1: Write failing tests**

Create `src/scale/mod.rs`:

```rust
pub mod spaces;
pub mod visibility;

use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpaceTag {
    Street,
    Block,
    Town,
    City,
    Country,
    World,
    Planet,
    SolarSystem,
}

impl SpaceTag {
    pub const ALL: [SpaceTag; 8] = [
        SpaceTag::Street,
        SpaceTag::Block,
        SpaceTag::Town,
        SpaceTag::City,
        SpaceTag::Country,
        SpaceTag::World,
        SpaceTag::Planet,
        SpaceTag::SolarSystem,
    ];

    pub fn zoom_center(&self) -> f32 {
        match self {
            SpaceTag::Street     => 0.0 / 7.0,
            SpaceTag::Block      => 1.0 / 7.0,
            SpaceTag::Town       => 2.0 / 7.0,
            SpaceTag::City       => 3.0 / 7.0,
            SpaceTag::Country    => 4.0 / 7.0,
            SpaceTag::World      => 5.0 / 7.0,
            SpaceTag::Planet     => 6.0 / 7.0,
            SpaceTag::SolarSystem => 7.0 / 7.0,
        }
    }

    pub fn opacity_for_zoom(&self, zoom: f32) -> f32 {
        let dist = (zoom - self.zoom_center()).abs();
        let fade_range = 1.5 / 7.0;
        (1.0 - dist / fade_range).clamp(0.0, 1.0)
    }
}

pub struct ScalePlugin;

impl Plugin for ScalePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spaces::spawn_spaces)
           .add_systems(Update, visibility::update_visibility);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;
    use crate::camera::ZoomLevel;

    #[test]
    fn street_is_fully_opaque_at_zoom_zero() {
        let opacity = SpaceTag::Street.opacity_for_zoom(0.0);
        assert!(opacity > 0.99, "street opacity at zoom 0 was {}", opacity);
    }

    #[test]
    fn solar_is_transparent_at_zoom_zero() {
        let opacity = SpaceTag::SolarSystem.opacity_for_zoom(0.0);
        assert!(opacity < 0.01, "solar opacity at zoom 0 was {}", opacity);
    }

    #[test]
    fn solar_is_fully_opaque_at_zoom_one() {
        let opacity = SpaceTag::SolarSystem.opacity_for_zoom(1.0);
        assert!(opacity > 0.99, "solar opacity at zoom 1 was {}", opacity);
    }

    #[test]
    fn eight_space_entities_spawned() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
           .insert_resource(ZoomLevel::default())
           .add_plugins(ScalePlugin);
        app.update();
        let count = app.world().query::<&SpaceTag>().iter(app.world()).count();
        assert_eq!(count, 8);
    }
}
```

Create `src/scale/spaces.rs`:

```rust
use bevy::prelude::*;
use super::SpaceTag;

pub fn spawn_spaces(mut commands: Commands) {
    for tag in SpaceTag::ALL {
        commands.spawn((
            tag,
            Transform::default(),
            Visibility::default(),
        ));
    }
}
```

Create `src/scale/visibility.rs`:

```rust
use bevy::prelude::*;
use super::SpaceTag;
use crate::camera::ZoomLevel;

pub fn update_visibility(
    zoom: Res<ZoomLevel>,
    mut query: Query<(&SpaceTag, &mut Visibility)>,
) {
    if !zoom.is_changed() {
        return;
    }
    for (tag, mut vis) in query.iter_mut() {
        let opacity = tag.opacity_for_zoom(zoom.value);
        *vis = if opacity > 0.01 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test scale
```

Expected: all 4 scale tests PASS.

- [ ] **Step 3: Add ScalePlugin to main.rs**

```rust
mod world_gen;
mod theme;
mod camera;
mod scale;

use bevy::prelude::*;
use world_gen::WorldGenPlugin;
use theme::ThemePlugin;
use camera::CameraPlugin;
use scale::ScalePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WorldGenPlugin)
        .add_plugins(ThemePlugin)
        .add_plugins(CameraPlugin)
        .add_plugins(ScalePlugin)
        .run();
}
```

- [ ] **Step 4: Verify no panics**

```bash
cargo run
```

Expected: window opens, no panics.

- [ ] **Step 5: Commit**

```bash
git add src/scale/ src/main.rs
git commit -m "feat: 8 scale space entities with zoom-driven visibility"
```

---

## Task 10: Street-level renderer — buildings and ground

**Files:**
- Create: `src/renderer/mod.rs`
- Create: `src/renderer/buildings.rs`
- Create: `src/renderer/terrain.rs`

- [ ] **Step 1: Write failing test**

Create `src/renderer/mod.rs`:

```rust
pub mod buildings;
pub mod terrain;
pub mod solar;

use bevy::prelude::*;

pub struct RendererPlugin;

impl Plugin for RendererPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (
            terrain::spawn_ground,
            buildings::spawn_street_buildings,
            terrain::spawn_water_plane,
        ).chain());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;
    use crate::{world_gen::WorldGenPlugin, theme::ThemePlugin, camera::CameraPlugin, scale::ScalePlugin};

    #[test]
    fn renderer_plugin_spawns_mesh_entities() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
           .add_plugins(AssetPlugin::default())
           .add_plugins(WorldGenPlugin)
           .add_plugins(ThemePlugin)
           .add_plugins(CameraPlugin)
           .add_plugins(ScalePlugin)
           .add_plugins(RendererPlugin);
        app.update();
        let mesh_count = app.world()
            .query::<&Mesh3d>()
            .iter(app.world())
            .count();
        assert!(mesh_count > 0, "no mesh entities were spawned");
    }
}
```

Create `src/renderer/buildings.rs`:

```rust
use bevy::prelude::*;
use crate::world_gen::WorldData;
use crate::theme::Theme;
use crate::scale::SpaceTag;

pub fn spawn_street_buildings(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    world: Res<WorldData>,
    theme: Res<Theme>,
    space_query: Query<(Entity, &SpaceTag)>,
) {
    let street_entity = space_query.iter()
        .find(|(_, tag)| **tag == SpaceTag::Street)
        .map(|(e, _)| e);

    let Some(parent) = street_entity else { return };

    for city in &world.cities {
        for district in &city.districts {
            for block in &district.blocks {
                for building in &block.buildings {
                    let color = if building.height > 1.5 {
                        theme.buildings.commercial
                    } else {
                        theme.buildings.residential
                    };

                    let mesh = meshes.add(Cuboid::new(building.width, building.height, building.depth));
                    let material = materials.add(StandardMaterial {
                        base_color: color,
                        ..default()
                    });

                    commands.spawn((
                        Mesh3d(mesh),
                        MeshMaterial3d(material),
                        Transform::from_xyz(
                            building.x,
                            building.height / 2.0,
                            building.z,
                        ),
                    )).set_parent(parent);
                }
            }
        }
    }
}
```

Create `src/renderer/terrain.rs`:

```rust
use bevy::prelude::*;
use crate::theme::Theme;
use crate::scale::SpaceTag;

pub fn spawn_ground(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    theme: Res<Theme>,
    space_query: Query<(Entity, &SpaceTag)>,
) {
    let street_entity = space_query.iter()
        .find(|(_, tag)| **tag == SpaceTag::Street)
        .map(|(e, _)| e);

    let Some(parent) = street_entity else { return };

    let mesh = meshes.add(Plane3d::default().mesh().size(200.0, 200.0));
    let material = materials.add(StandardMaterial {
        base_color: theme.terrain.ground,
        ..default()
    });

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, 0.0, 0.0),
    )).set_parent(parent);
}

pub fn spawn_water_plane(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    theme: Res<Theme>,
    space_query: Query<(Entity, &SpaceTag)>,
) {
    let world_entity = space_query.iter()
        .find(|(_, tag)| **tag == SpaceTag::World)
        .map(|(e, _)| e);

    let Some(parent) = world_entity else { return };

    let mesh = meshes.add(Plane3d::default().mesh().size(300.0, 300.0));
    let material = materials.add(StandardMaterial {
        base_color: theme.terrain.water,
        ..default()
    });

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, -0.5, 0.0),
    )).set_parent(parent);
}
```

Create `src/renderer/solar.rs` (stub for now, filled in Task 12):

```rust
// Solar system renderer — see Task 12
```

- [ ] **Step 2: Run tests**

```bash
cargo test renderer
```

Expected: `renderer_plugin_spawns_mesh_entities` PASS.

- [ ] **Step 3: Add RendererPlugin + lighting to main.rs**

```rust
mod world_gen;
mod theme;
mod camera;
mod scale;
mod renderer;

use bevy::prelude::*;
use world_gen::WorldGenPlugin;
use theme::ThemePlugin;
use camera::CameraPlugin;
use scale::ScalePlugin;
use renderer::RendererPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WorldGenPlugin)
        .add_plugins(ThemePlugin)
        .add_plugins(CameraPlugin)
        .add_plugins(ScalePlugin)
        .add_plugins(RendererPlugin)
        .add_systems(Startup, spawn_light)
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
}
```

- [ ] **Step 4: Visual check — run and verify buildings appear**

```bash
cargo run
```

Expected: isometric view showing coloured box buildings on a dark ground plane. Scroll in/out to see zoom working. The buildings may appear very small at the default zoom level — scroll in (down) to zoom in.

- [ ] **Step 5: Commit**

```bash
git add src/renderer/ src/main.rs
git commit -m "feat: street-level building renderer and ground/water planes"
```

---

## Task 11: World-level renderer — continents and planet sphere

**Files:**
- Modify: `src/renderer/terrain.rs`

Add continent patches at world level and a planet sphere at planet level.

- [ ] **Step 1: Add continent and planet spawning to terrain.rs**

Add this import at the top of `src/renderer/terrain.rs` (after the existing `use bevy::prelude::*;` line):

```rust
use crate::world_gen::{WorldData, planet::GRID_SIZE};
```

Then add these two functions at the bottom of `src/renderer/terrain.rs`:

```rust

pub fn spawn_continent_patches(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    world: Res<WorldData>,
    theme: Res<Theme>,
    space_query: Query<(Entity, &SpaceTag)>,
) {
    let world_entity = space_query.iter()
        .find(|(_, tag)| **tag == SpaceTag::World)
        .map(|(e, _)| e);

    let Some(parent) = world_entity else { return };

    let cell_size = 2.0;
    let offset = GRID_SIZE as f32 * cell_size / 2.0;

    for continent in &world.planet.continents {
        for (cx, cz) in &continent.cells {
            let mesh = meshes.add(Cuboid::new(cell_size, 0.1, cell_size));
            let material = materials.add(StandardMaterial {
                base_color: theme.terrain.land_continent,
                ..default()
            });
            commands.spawn((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::from_xyz(
                    *cx as f32 * cell_size - offset,
                    0.05,
                    *cz as f32 * cell_size - offset,
                ),
            )).set_parent(parent);
        }
    }
}

pub fn spawn_planet_sphere(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    theme: Res<Theme>,
    space_query: Query<(Entity, &SpaceTag)>,
) {
    let planet_entity = space_query.iter()
        .find(|(_, tag)| **tag == SpaceTag::Planet)
        .map(|(e, _)| e);

    let Some(parent) = planet_entity else { return };

    let mesh = meshes.add(Sphere::new(40.0).mesh().uv(32, 18));
    let material = materials.add(StandardMaterial {
        base_color: theme.terrain.water,
        ..default()
    });
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, 0.0, 0.0),
    )).set_parent(parent);
}
```

- [ ] **Step 2: Register new systems in RendererPlugin**

Update `src/renderer/mod.rs`:

```rust
pub mod buildings;
pub mod terrain;
pub mod solar;

use bevy::prelude::*;

pub struct RendererPlugin;

impl Plugin for RendererPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (
            terrain::spawn_ground,
            buildings::spawn_street_buildings,
            terrain::spawn_water_plane,
            terrain::spawn_continent_patches,
            terrain::spawn_planet_sphere,
        ).chain());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;
    use crate::{world_gen::WorldGenPlugin, theme::ThemePlugin, camera::CameraPlugin, scale::ScalePlugin};

    #[test]
    fn renderer_plugin_spawns_mesh_entities() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
           .add_plugins(AssetPlugin::default())
           .add_plugins(WorldGenPlugin)
           .add_plugins(ThemePlugin)
           .add_plugins(CameraPlugin)
           .add_plugins(ScalePlugin)
           .add_plugins(RendererPlugin);
        app.update();
        let mesh_count = app.world()
            .query::<&Mesh3d>()
            .iter(app.world())
            .count();
        assert!(mesh_count > 0, "no mesh entities were spawned");
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test
```

Expected: all tests PASS.

- [ ] **Step 4: Visual check**

```bash
cargo run
```

Scroll all the way out (zoom to ~0.75-1.0). Expected: continent patches visible as flat coloured tiles, planet sphere visible at high zoom.

- [ ] **Step 5: Commit**

```bash
git add src/renderer/terrain.rs src/renderer/mod.rs
git commit -m "feat: continent patches and planet sphere renderer"
```

---

## Task 12: Solar system renderer

**Files:**
- Modify: `src/renderer/solar.rs`
- Modify: `src/renderer/mod.rs`

- [ ] **Step 1: Implement solar.rs**

Replace the stub `src/renderer/solar.rs` with:

```rust
use bevy::prelude::*;
use crate::theme::Theme;
use crate::scale::SpaceTag;

pub fn spawn_solar_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    theme: Res<Theme>,
    space_query: Query<(Entity, &SpaceTag)>,
) {
    let solar_entity = space_query.iter()
        .find(|(_, tag)| **tag == SpaceTag::SolarSystem)
        .map(|(e, _)| e);

    let Some(parent) = solar_entity else { return };

    // Sun
    let sun_mesh = meshes.add(Sphere::new(80.0).mesh().uv(32, 18));
    let sun_mat = materials.add(StandardMaterial {
        base_color: theme.solar.sun,
        emissive: LinearRgba::from(theme.solar.sun) * 2.0,
        ..default()
    });
    commands.spawn((
        Mesh3d(sun_mesh),
        MeshMaterial3d(sun_mat),
        Transform::from_xyz(-800.0, 0.0, 0.0),
    )).set_parent(parent);

    // Orbital ring — thin torus: major_radius = distance from center, minor_radius = tube thickness
    let ring_mesh = meshes.add(
        Torus { minor_radius: 5.0, major_radius: 490.0 }.mesh().minor_resolution(4).major_resolution(64)
    );
    let ring_mat = materials.add(StandardMaterial {
        base_color: theme.solar.orbital_ring,
        ..default()
    });
    commands.spawn((
        Mesh3d(ring_mesh),
        MeshMaterial3d(ring_mat),
        Transform::from_xyz(-800.0, 0.0, 0.0),
    )).set_parent(parent);

    // Planet dot at orbit position
    let planet_mesh = meshes.add(Sphere::new(20.0).mesh().uv(16, 12));
    let planet_mat = materials.add(StandardMaterial {
        base_color: theme.solar.planet,
        ..default()
    });
    commands.spawn((
        Mesh3d(planet_mesh),
        MeshMaterial3d(planet_mat),
        Transform::from_xyz(-320.0, 0.0, 0.0),
    )).set_parent(parent);
}
```

- [ ] **Step 2: Register in RendererPlugin**

Update the `.add_systems(Startup, ...)` chain in `src/renderer/mod.rs` to add `solar::spawn_solar_system`:

```rust
app.add_systems(Startup, (
    terrain::spawn_ground,
    buildings::spawn_street_buildings,
    terrain::spawn_water_plane,
    terrain::spawn_continent_patches,
    terrain::spawn_planet_sphere,
    solar::spawn_solar_system,
).chain());
```

- [ ] **Step 3: Run tests**

```bash
cargo test
```

Expected: all tests PASS.

- [ ] **Step 4: Visual check**

```bash
cargo run
```

Scroll all the way out to zoom 1.0. Expected: sun sphere and orbital ring visible, planet dot on the ring.

- [ ] **Step 5: Commit**

```bash
git add src/renderer/solar.rs src/renderer/mod.rs
git commit -m "feat: solar system renderer with sun, orbital ring, and planet dot"
```

---

## Task 13: Block and city-level renderer

**Files:**
- Modify: `src/renderer/buildings.rs`

Add coarser representations for block, town, and city zoom levels.

- [ ] **Step 1: Add block-level boxes to buildings.rs**

Add these functions to `src/renderer/buildings.rs`:

```rust
pub fn spawn_block_level(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    world: Res<WorldData>,
    theme: Res<Theme>,
    space_query: Query<(Entity, &SpaceTag)>,
) {
    let block_entity = space_query.iter()
        .find(|(_, tag)| **tag == SpaceTag::Block)
        .map(|(e, _)| e);

    let Some(parent) = block_entity else { return };

    for city in &world.cities {
        for district in &city.districts {
            for block in &district.blocks {
                let avg_height: f32 = if block.buildings.is_empty() {
                    0.5
                } else {
                    block.buildings.iter().map(|b| b.height).sum::<f32>() / block.buildings.len() as f32
                };

                let mesh = meshes.add(Cuboid::new(0.7, avg_height * 0.3, 0.7));
                let material = materials.add(StandardMaterial {
                    base_color: theme.buildings.residential,
                    ..default()
                });
                commands.spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    Transform::from_xyz(block.x, avg_height * 0.15, block.z),
                )).set_parent(parent);
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
    space_query: Query<(Entity, &SpaceTag)>,
) {
    let city_entity = space_query.iter()
        .find(|(_, tag)| **tag == SpaceTag::City)
        .map(|(e, _)| e);

    let Some(parent) = city_entity else { return };

    for city in &world.cities {
        let scale = city.districts.len() as f32 * 0.3 + 1.0;
        let mesh = meshes.add(Cuboid::new(scale, scale * 0.1, scale));
        let material = materials.add(StandardMaterial {
            base_color: theme.buildings.commercial,
            ..default()
        });
        // Place at scaled-down world coords (cities use planet lat/lon ~0-64)
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_xyz(city.lat * 0.5 - 16.0, 0.0, city.lon * 0.5 - 16.0),
        )).set_parent(parent);
    }
}
```

- [ ] **Step 2: Register new systems in RendererPlugin**

Update `src/renderer/mod.rs` startup chain:

```rust
app.add_systems(Startup, (
    terrain::spawn_ground,
    buildings::spawn_street_buildings,
    buildings::spawn_block_level,
    buildings::spawn_city_level,
    terrain::spawn_water_plane,
    terrain::spawn_continent_patches,
    terrain::spawn_planet_sphere,
    solar::spawn_solar_system,
).chain());
```

- [ ] **Step 3: Run tests**

```bash
cargo test
```

Expected: all tests PASS.

- [ ] **Step 4: Visual check at all zoom levels**

```bash
cargo run
```

Scroll slowly from 0 to 1. Verify:
- Zoom ~0.0: individual buildings visible
- Zoom ~0.15: block-level boxes appear
- Zoom ~0.43: city blobs appear
- Zoom ~0.75: continent patches / planet sphere
- Zoom ~1.0: solar system

- [ ] **Step 5: Commit**

```bash
git add src/renderer/buildings.rs src/renderer/mod.rs
git commit -m "feat: block and city level renderers"
```

---

## Task 14: Final wiring and polish

**Files:**
- Modify: `src/main.rs`

Add window title, set initial clear color from theme, and add an Escape key quit handler.

- [ ] **Step 1: Update main.rs**

```rust
mod world_gen;
mod theme;
mod camera;
mod scale;
mod renderer;

use bevy::prelude::*;
use world_gen::WorldGenPlugin;
use theme::ThemePlugin;
use camera::CameraPlugin;
use scale::ScalePlugin;
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
        .add_plugins(ScalePlugin)
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

- [ ] **Step 2: Run all tests**

```bash
cargo test
```

Expected: all tests PASS.

- [ ] **Step 3: Full visual run — test all controls**

```bash
cargo run
```

Verify:
- Window titled "MassiveCityScale" at 1280×720
- Buildings visible when zoomed in (scroll down)
- Smooth zoom from street → solar system (scroll up)
- Click + drag pans the view
- T key cycles through dark/pastel/earthy themes (background and geometry colors change)
- Escape quits

- [ ] **Step 4: Final commit**

```bash
git add src/main.rs
git commit -m "feat: window title, clear color, dual lighting, escape to quit"
```
