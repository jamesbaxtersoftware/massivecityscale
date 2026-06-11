# Creature Battles Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Pokémon-style creature-battling layer to the solar-system renderer: typed planets host wild monsters; the player picks a starter and fights or traps wild monsters in an in-world turn-based battle.

**Architecture:** A new `src/creatures/` Bevy plugin. Pure game logic (type matchup, damage, XP/leveling, trap odds, planet typing) lives in testable free functions. A `GameState` state machine (`StarterSelect → Exploring → Battle`) gates systems. Battle is rendered in-world (approach B): real creature meshes face off, billboarded HP bars float above, keyboard drives turns. The orthographic picking ray is extracted into a shared helper used by both planet and monster pickers.

**Tech Stack:** Rust, Bevy 0.15, `rand` / `rand_chacha` (seeded RNG, already a dependency).

---

## Conventions

- Run all tests with: `cargo test`
- Run a single test: `cargo test <test_name> -- --exact`
- Build/verify it compiles: `cargo build`
- Manual run (visual checks): `cargo run`
- The crate already has headless-app tests (see `src/renderer/mod.rs` and `src/lod/mod.rs`) — follow that pattern for ECS tests using `MinimalPlugins`.
- Commit after every green step.

## File Structure

- Create `src/creatures/mod.rs` — `CreaturesPlugin`, shared types (`CreatureType`, `Species`, `Move`, `Creature`, `PlanetType`, `ClickSphere`, `WildMonster`), resources (`PlayerCreature`, `Collection`, `Inventory`, `BattleSession`), `GameState`, and pure logic functions (`type_multiplier`, `type_from_color`, `damage`, `xp_reward`, `xp_to_next`, `apply_xp`, `trap_chance`).
- Create `src/creatures/spawn.rs` — creature mesh builders; `spawn_wild_monsters` startup system.
- Create `src/creatures/pick.rs` — shared `ortho_pick_ray` helper; `pick_wild_monster` system; refactor `solar::pick_planet` to use the helper.
- Create `src/creatures/starter.rs` — starter-select state UI/spawn + click handling.
- Create `src/creatures/battle.rs` — battle setup/teardown, turn resolution, win/lose/trap.
- Create `src/creatures/battle_view.rs` — camera framing, billboard HP bars, hit flash, input.
- Modify `src/main.rs` — register `CreaturesPlugin`.
- Modify `src/renderer/solar.rs` — add `PlanetType` component to planets; use shared ray helper.

---

## Phase 1 — Foundation (types, logic, spawning, picking)

### Task 1: Core types and module skeleton

**Files:**
- Create: `src/creatures/mod.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create the module with core types and an empty plugin**

`src/creatures/mod.rs`:

```rust
pub mod spawn;
pub mod pick;
pub mod starter;
pub mod battle;
pub mod battle_view;

use bevy::prelude::*;

// ── Types ──────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CreatureType { Fire, Water }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Species { Emberling, Tideling }

impl Species {
    pub fn creature_type(self) -> CreatureType {
        match self {
            Species::Emberling => CreatureType::Fire,
            Species::Tideling => CreatureType::Water,
        }
    }
    pub fn name(self) -> &'static str {
        match self { Species::Emberling => "Emberling", Species::Tideling => "Tideling" }
    }
    /// Two moves per species: [weak, strong].
    pub fn moves(self) -> [Move; 2] {
        match self {
            Species::Emberling => [
                Move { name: "Ember", power: 8 },
                Move { name: "Flare", power: 14 },
            ],
            Species::Tideling => [
                Move { name: "Splash", power: 8 },
                Move { name: "Torrent", power: 14 },
            ],
        }
    }
    pub fn base_max_hp(self) -> i32 { 30 }
    pub fn base_attack(self) -> i32 { 10 }
    pub fn of_type(t: CreatureType) -> Species {
        match t { CreatureType::Fire => Species::Emberling, CreatureType::Water => Species::Tideling }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Move { pub name: &'static str, pub power: i32 }

#[derive(Component, Clone, Debug)]
pub struct Creature {
    pub species: Species,
    pub level:   u32,
    pub xp:      u32,
    pub hp:      i32,
    pub max_hp:  i32,
    pub attack:  i32,
}

impl Creature {
    pub fn new(species: Species, level: u32) -> Self {
        // +5 max_hp and +2 attack per level above 1.
        let lv = level.max(1);
        let max_hp = species.base_max_hp() + (lv as i32 - 1) * 5;
        let attack = species.base_attack() + (lv as i32 - 1) * 2;
        Self { species, level: lv, xp: 0, hp: max_hp, max_hp, attack }
    }
    pub fn creature_type(&self) -> CreatureType { self.species.creature_type() }
    pub fn heal_full(&mut self) { self.hp = self.max_hp; }
    pub fn is_fainted(&self) -> bool { self.hp <= 0 }
}

// ── Components ──────────────────────────────────────────────────────────────────

/// Element a planet is themed as; added to each planet entity at spawn.
#[derive(Component, Clone, Copy, Debug)]
pub struct PlanetType(pub CreatureType);

/// Marks a wild (clickable, battleable) monster entity.
#[derive(Component)]
pub struct WildMonster;

/// Sphere used for click raycasting against non-planet entities.
#[derive(Component)]
pub struct ClickSphere { pub radius: f32 }

// ── Resources ───────────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct PlayerCreature(pub Creature);

#[derive(Resource, Default)]
pub struct Collection(pub Vec<Creature>);

#[derive(Resource)]
pub struct Inventory { pub force_fields: u32 }

impl Default for Inventory {
    fn default() -> Self { Self { force_fields: 3 } }
}

// ── State ────────────────────────────────────────────────────────────────────────

#[derive(States, Default, Clone, Eq, PartialEq, Hash, Debug)]
pub enum GameState {
    #[default]
    StarterSelect,
    Exploring,
    Battle,
}

// ── Plugin ───────────────────────────────────────────────────────────────────────

pub struct CreaturesPlugin;

impl Plugin for CreaturesPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
           .init_resource::<Collection>()
           .init_resource::<Inventory>();
    }
}
```

- [ ] **Step 2: Register the plugin in `main.rs`**

Add `mod creatures;` near the other `mod` lines, `use creatures::CreaturesPlugin;`, and `.add_plugins(CreaturesPlugin)` after `.add_plugins(RendererPlugin)`.

- [ ] **Step 3: Verify it compiles**

Run: `cargo build`
Expected: compiles (warnings about unused items are fine for now).

- [ ] **Step 4: Commit**

```bash
git add src/creatures/mod.rs src/main.rs
git commit -m "feat(creatures): module skeleton, core types, GameState"
```

> Note: Steps 1 declares submodules (`spawn`, `pick`, etc.) that don't exist yet — create empty placeholder files so `cargo build` passes:
> ```bash
> printf '' > src/creatures/spawn.rs src/creatures/pick.rs src/creatures/starter.rs src/creatures/battle.rs src/creatures/battle_view.rs
> ```
> Do this before Step 3.

---

### Task 2: Type matchup logic (TDD)

**Files:**
- Modify: `src/creatures/mod.rs`

- [ ] **Step 1: Write the failing test**

Add to a `#[cfg(test)] mod tests` block at the bottom of `mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn water_beats_fire_fire_weak_to_water() {
        assert_eq!(type_multiplier(CreatureType::Water, CreatureType::Fire), 2.0);
        assert_eq!(type_multiplier(CreatureType::Fire, CreatureType::Water), 0.5);
    }

    #[test]
    fn same_type_is_neutral() {
        assert_eq!(type_multiplier(CreatureType::Fire, CreatureType::Fire), 1.0);
        assert_eq!(type_multiplier(CreatureType::Water, CreatureType::Water), 1.0);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test type_multiplier -- --exact` (or `cargo test water_beats_fire`)
Expected: FAIL — `cannot find function type_multiplier`.

- [ ] **Step 3: Implement `type_multiplier`**

Add to `mod.rs` (above the tests module):

```rust
pub fn type_multiplier(attacker: CreatureType, defender: CreatureType) -> f32 {
    use CreatureType::*;
    match (attacker, defender) {
        (Water, Fire) => 2.0,
        (Fire, Water) => 0.5,
        _ => 1.0,
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test same_type_is_neutral water_beats_fire`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/creatures/mod.rs
git commit -m "feat(creatures): type matchup multiplier"
```

---

### Task 3: Planet typing from color (TDD)

**Files:**
- Modify: `src/creatures/mod.rs`

- [ ] **Step 1: Write the failing test**

Add to the `tests` module:

```rust
#[test]
fn warm_colors_are_fire_cool_are_water() {
    // lava orange
    assert_eq!(type_from_color(Color::srgb(0.95, 0.28, 0.05)), CreatureType::Fire);
    // ocean blue
    assert_eq!(type_from_color(Color::srgb(0.10, 0.38, 0.90)), CreatureType::Water);
    // ice cyan (blue dominant)
    assert_eq!(type_from_color(Color::srgb(0.35, 0.82, 0.88)), CreatureType::Water);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test warm_colors_are_fire -- --exact`
Expected: FAIL — `cannot find function type_from_color`.

- [ ] **Step 3: Implement `type_from_color`**

```rust
/// Classify a planet color: red channel ≥ blue channel → Fire, else Water.
pub fn type_from_color(c: Color) -> CreatureType {
    let s = c.to_srgba();
    if s.red >= s.blue { CreatureType::Fire } else { CreatureType::Water }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test warm_colors_are_fire -- --exact`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/creatures/mod.rs
git commit -m "feat(creatures): derive planet type from color"
```

---

### Task 4: Tag planets with their type

**Files:**
- Modify: `src/renderer/solar.rs:162-181` (the planet spawn loop)

- [ ] **Step 1: Add `PlanetType` when spawning each planet**

In `spawn_solar_system`, inside the `for p in &planets` loop, change the planet `commands.spawn((...))` tuple to also insert the planet type. Add the import at the top of `solar.rs`:

```rust
use crate::creatures::{PlanetType, type_from_color};
```

Then in the spawn tuple (the one with `Mesh3d(mesh)` … `Visibility::Hidden`), add a line:

```rust
            PlanetType(type_from_color(p.color)),
```

so the planet entity now carries `PlanetType`. (Leave the ring entity unchanged.)

- [ ] **Step 2: Verify it compiles**

Run: `cargo build`
Expected: compiles.

- [ ] **Step 3: Commit**

```bash
git add src/renderer/solar.rs
git commit -m "feat(creatures): tag planets with elemental type"
```

---

### Task 5: Creature mesh builders

**Files:**
- Modify: `src/creatures/spawn.rs`

- [ ] **Step 1: Implement two mesh/material builders**

`src/creatures/spawn.rs` (start of file):

```rust
use bevy::prelude::*;
use super::{CreatureType, Species};

/// Returns (body mesh, material, accent mesh, accent material, accent local transform)
/// for a creature of the given type. Accent is the flame spike (Fire) or fin (Water).
pub struct CreatureVisual {
    pub body_mesh:    Handle<Mesh>,
    pub body_mat:     Handle<StandardMaterial>,
    pub accent_mesh:  Handle<Mesh>,
    pub accent_mat:   Handle<StandardMaterial>,
    pub accent_xform: Transform,
    /// Approximate bounding radius for click spheres and HP-bar placement.
    pub radius: f32,
}

pub fn build_visual(
    t: CreatureType,
    scale: f32,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> CreatureVisual {
    let radius = scale;
    match t {
        CreatureType::Fire => CreatureVisual {
            body_mesh: meshes.add(Sphere::new(scale).mesh().uv(16, 12)),
            body_mat: materials.add(StandardMaterial {
                base_color: Color::srgb(0.95, 0.35, 0.10),
                emissive: LinearRgba::rgb(0.6, 0.2, 0.0),
                ..default()
            }),
            accent_mesh: meshes.add(Cone { radius: scale * 0.5, height: scale * 1.4 }.mesh()),
            accent_mat: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.8, 0.2),
                emissive: LinearRgba::rgb(0.8, 0.5, 0.0),
                unlit: true,
                ..default()
            }),
            accent_xform: Transform::from_xyz(0.0, scale * 1.1, 0.0),
            radius,
        },
        CreatureType::Water => CreatureVisual {
            body_mesh: meshes.add(Sphere::new(scale).mesh().uv(16, 12)),
            body_mat: materials.add(StandardMaterial {
                base_color: Color::srgb(0.15, 0.45, 0.9),
                perceptual_roughness: 0.3,
                ..default()
            }),
            accent_mesh: meshes.add(Cuboid::new(scale * 0.2, scale * 1.2, scale * 0.8)),
            accent_mat: materials.add(StandardMaterial {
                base_color: Color::srgb(0.5, 0.8, 1.0),
                perceptual_roughness: 0.2,
                ..default()
            }),
            accent_xform: Transform::from_xyz(0.0, scale * 0.9, -scale * 0.3),
            radius,
        },
    }
}

/// Spawn a creature's body+accent as children of `parent_entity`, at `local`.
/// Returns nothing; used by both wild spawning and battle setup.
pub fn spawn_creature_visual(
    commands: &mut Commands,
    parent: Entity,
    local: Transform,
    vis: &CreatureVisual,
) {
    commands.entity(parent).with_children(|c| {
        c.spawn((
            Mesh3d(vis.body_mesh.clone()),
            MeshMaterial3d(vis.body_mat.clone()),
            local,
        )).with_children(|b| {
            b.spawn((
                Mesh3d(vis.accent_mesh.clone()),
                MeshMaterial3d(vis.accent_mat.clone()),
                vis.accent_xform,
            ));
        });
    });
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build`
Expected: compiles (`spawn_creature_visual` may warn as unused — fine).

- [ ] **Step 3: Commit**

```bash
git add src/creatures/spawn.rs
git commit -m "feat(creatures): creature mesh/material builders"
```

---

### Task 6: Spawn wild monsters on planets (headless test + system)

**Files:**
- Modify: `src/creatures/spawn.rs`, `src/creatures/mod.rs`

- [ ] **Step 1: Write a failing headless test**

Add at the bottom of `spawn.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::creatures::{CreaturesPlugin, PlanetType, WildMonster, CreatureType};
    use crate::renderer::solar::CelestialBody;

    #[test]
    fn wild_monsters_spawn_as_children_of_a_planet() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
           .add_plugins(AssetPlugin::default())
           .init_asset::<Mesh>()
           .init_asset::<StandardMaterial>()
           .add_plugins(CreaturesPlugin);

        // a fake planet
        let planet = app.world_mut().spawn((
            Transform::from_xyz(1000.0, 0.0, 0.0),
            GlobalTransform::default(),
            PlanetType(CreatureType::Fire),
            CelestialBody { radius: 200.0, pivot_offset: Vec3::ZERO },
        )).id();

        app.world_mut().run_system_once(spawn_wild_monsters).unwrap();

        let count = app.world_mut()
            .query::<(&WildMonster, &Parent)>()
            .iter(app.world())
            .filter(|(_, p)| p.get() == planet)
            .count();
        assert!(count > 0, "expected wild monsters parented to the planet");
    }
}
```

(Imports needed at top of the test or via `use bevy::ecs::system::RunSystemOnce;`.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test wild_monsters_spawn -- --exact`
Expected: FAIL — `cannot find function spawn_wild_monsters`.

- [ ] **Step 3: Implement `spawn_wild_monsters`**

Add to `spawn.rs`:

```rust
use bevy::ecs::system::RunSystemOnce; // only needed for tests; keep behind cfg if it warns
use rand::SeedableRng;
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use crate::creatures::{Creature, Species, PlanetType, WildMonster, ClickSphere};
use crate::renderer::solar::CelestialBody;
use crate::lod::LodRange;
use crate::renderer::scale_consts::LOD_SOLAR;

const MONSTERS_PER_PLANET: usize = 3;
const MONSTER_SCALE: f32 = 28.0; // km

pub fn spawn_wild_monsters(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    planets: Query<(Entity, &PlanetType, &CelestialBody)>,
) {
    let mut rng = ChaCha8Rng::seed_from_u64(0x_C0FF_EE15_600D);
    for (planet, ptype, body) in &planets {
        let species = Species::of_type(ptype.0);
        let vis = build_visual(ptype.0, MONSTER_SCALE, &mut meshes, &mut materials);
        for _ in 0..MONSTERS_PER_PLANET {
            // random point on the planet surface
            let dir = Vec3::new(
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
            ).normalize_or_zero();
            let local = Transform::from_translation(dir * (body.radius + MONSTER_SCALE));
            let level = rng.gen_range(2..=5);
            let monster = commands.spawn((
                Creature::new(species, level),
                WildMonster,
                ClickSphere { radius: MONSTER_SCALE * 1.5 },
                local,
                GlobalTransform::default(),
                Visibility::Inherited,
                LodRange { min_scale: 0.0, max_scale: LOD_SOLAR.1 },
            )).id();
            commands.entity(planet).add_child(monster);
            spawn_creature_visual(&mut commands, monster, Transform::IDENTITY, &vis);
        }
    }
}
```

> Note: the visual children attach to the `monster` entity (which holds the click sphere + LOD). The monster entity's own `Transform` is the surface position; the body mesh sits at its local origin.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test wild_monsters_spawn -- --exact`
Expected: PASS.

- [ ] **Step 5: Register the spawn system**

In `mod.rs` `CreaturesPlugin::build`, add:

```rust
           .add_systems(Startup, spawn::spawn_wild_monsters.after(crate::renderer::solar::spawn_solar_system))
```

(Confirm `spawn_solar_system` is `pub` — it is. The `.after` ordering guarantees `PlanetType` exists.)

- [ ] **Step 6: Verify build + run**

Run: `cargo build` then `cargo run`
Expected: compiles; zoom into the solar system — small glowing/blue shapes sit on planet surfaces matching planet color.

- [ ] **Step 7: Commit**

```bash
git add src/creatures/spawn.rs src/creatures/mod.rs
git commit -m "feat(creatures): spawn wild monsters on typed planets"
```

---

### Task 7: Extract shared picking ray + monster picker

**Files:**
- Modify: `src/creatures/pick.rs`, `src/renderer/solar.rs`, `src/creatures/mod.rs`

- [ ] **Step 1: Write the shared ray helper**

`src/creatures/pick.rs`:

```rust
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::input::mouse::MouseMotion;
use crate::camera::zoom::OrbitState;
use super::{WildMonster, ClickSphere, Creature, GameState, BattleSession};

/// Build an orthographic picking ray from the cursor. Returns (origin, forward dir).
pub fn ortho_pick_ray(
    window: &Window,
    ortho_scale: f32,
    orbit: &OrbitState,
) -> Option<(Vec3, Vec3)> {
    let cursor = window.cursor_position()?;
    let cam_pos = orbit.camera_pos();
    let forward = (orbit.pivot - cam_pos).normalize();
    let right   = forward.cross(Vec3::Y).normalize();
    let up      = right.cross(forward);

    let win = Vec2::new(window.width(), window.height());
    let ndc = Vec2::new(cursor.x / win.x * 2.0 - 1.0, 1.0 - cursor.y / win.y * 2.0);
    let half_w = ortho_scale * win.x / 2.0;
    let half_h = ortho_scale * win.y / 2.0;
    let origin = cam_pos + ndc.x * half_w * right + ndc.y * half_h * up;
    Some((origin, forward))
}

/// Nearest ray-sphere hit `t` (front face), or None.
pub fn ray_sphere_t(origin: Vec3, dir: Vec3, center: Vec3, radius: f32) -> Option<f32> {
    let l = origin - center;
    let b = l.dot(dir);
    let c = l.dot(l) - radius * radius;
    let d = b * b - c;
    if d < 0.0 { return None; }
    let t = -b - d.sqrt();
    if t < 0.0 { None } else { Some(t) }
}
```

- [ ] **Step 2: Refactor `solar::pick_planet` to use the helper**

In `src/renderer/solar.rs`, replace the inline ray construction (lines ~314-345, from building the ortho ray through the ray-sphere loop) so it calls `crate::creatures::pick::ortho_pick_ray` and `ray_sphere_t`. Concretely, replace the block that computes `ray_o` and the per-body loop with:

```rust
    let Ok(window) = windows.get_single() else { return };
    let Some((ray_o, forward)) = crate::creatures::pick::ortho_pick_ray(window, ortho.scale, &orbit) else { return };

    let mut best: Option<(Entity, Vec3, f32)> = None;
    for (entity, gtransform, body) in &bodies {
        let center = gtransform.translation() + body.pivot_offset;
        if let Some(t) = crate::creatures::pick::ray_sphere_t(ray_o, forward, center, body.radius) {
            if best.map_or(true, |(_, _, bt)| t < bt) {
                best = Some((entity, center, t));
            }
        }
    }
```

Delete the now-unused `cam_pos`/`right`/`up`/`ndc`/`half_w`/`half_h` locals and the manual discriminant math. Keep the `proj`/`ortho` extraction above it (the helper needs `ortho.scale`).

- [ ] **Step 3: Verify planet picking still works**

Run: `cargo build` then `cargo run`
Expected: compiles; clicking a planet still re-targets the camera exactly as before.

- [ ] **Step 4: Commit the refactor**

```bash
git add src/creatures/pick.rs src/renderer/solar.rs
git commit -m "refactor: extract shared orthographic picking ray helper"
```

- [ ] **Step 5: Add the wild-monster picker (starts a battle)**

Append to `pick.rs`:

```rust
/// In Exploring, a non-drag left click on a wild monster opens a battle.
pub fn pick_wild_monster(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut motion: EventReader<MouseMotion>,
    windows: Query<&Window, With<PrimaryWindow>>,
    proj_query: Query<&Projection, With<Camera3d>>,
    orbit: Res<OrbitState>,
    monsters: Query<(Entity, &GlobalTransform, &ClickSphere), With<WildMonster>>,
    mut tracker: ResMut<crate::renderer::solar::ClickTracker>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
) {
    // Mirror the planet picker's drag tracking so a drag isn't a click.
    if mouse_buttons.just_pressed(MouseButton::Left) {
        // ClickTracker drag_sq is reset by pick_planet; we only read just_released below.
    }
    if !mouse_buttons.just_released(MouseButton::Left) { motion.clear(); return; }
    if tracker.drag_sq() > 25.0 { return; }

    let Ok(Projection::Orthographic(ortho)) = proj_query.get_single() else { return };
    let Ok(window) = windows.get_single() else { return };
    let Some((origin, forward)) = ortho_pick_ray(window, ortho.scale, &orbit) else { return };

    let mut best: Option<(Entity, f32)> = None;
    for (entity, gt, cs) in &monsters {
        if let Some(t) = ray_sphere_t(origin, forward, gt.translation(), cs.radius) {
            if best.map_or(true, |(_, bt)| t < bt) { best = Some((entity, t)); }
        }
    }
    if let Some((wild, _)) = best {
        commands.insert_resource(BattleSession::new(wild));
        next_state.set(GameState::Battle);
    }
    let _ = &mut commands; // silence if unused in some build configs
}
```

> `tracker.drag_sq()` requires a public accessor. In `solar.rs` add to `impl ClickTracker { pub fn drag_sq(&self) -> f32 { self.drag_sq } }` and make the field stay private. (`BattleSession` is defined in Task 8 — this task may need Task 8's type to compile; if executing strictly in order, add a minimal `BattleSession` stub now and flesh it out in Task 8, or do Task 8 first. Recommended: do Task 8 before wiring this system in.)

- [ ] **Step 6: Commit (compile after Task 8 wiring)**

```bash
git add src/creatures/pick.rs src/renderer/solar.rs
git commit -m "feat(creatures): wild-monster picker starts a battle"
```

---

## Phase 2 — Starter select

### Task 8: BattleSession resource + starter resource wiring

**Files:**
- Modify: `src/creatures/mod.rs`

- [ ] **Step 1: Add the `BattleSession` resource and turn enum**

In `mod.rs`:

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Turn { Player, Enemy }

#[derive(Resource)]
pub struct BattleSession {
    pub wild_entity: Entity,
    pub turn: Turn,
    /// Entities spawned only for the battle view (player creature, HP bars), despawned on teardown.
    pub view_entities: Vec<Entity>,
    /// Snapshot of camera framing to restore when the battle ends.
    pub saved_orbit: Option<crate::camera::zoom::OrbitState>,
    pub player_hp: i32,
    pub wild_hp: i32,
}

impl BattleSession {
    pub fn new(wild_entity: Entity) -> Self {
        Self { wild_entity, turn: Turn::Player, view_entities: Vec::new(),
               saved_orbit: None, player_hp: 0, wild_hp: 0 }
    }
}
```

- [ ] **Step 2: Verify build**

Run: `cargo build`
Expected: compiles. Now go back and finish Task 7 Step 5/6 wiring if not yet done.

- [ ] **Step 3: Commit**

```bash
git add src/creatures/mod.rs
git commit -m "feat(creatures): BattleSession resource"
```

---

### Task 9: Starter-select screen

**Files:**
- Modify: `src/creatures/starter.rs`, `src/creatures/mod.rs`

- [ ] **Step 1: Implement starter spawn + pick**

`src/creatures/starter.rs`:

```rust
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use super::{CreatureType, Species, Creature, PlayerCreature, GameState};
use super::spawn::{build_visual, spawn_creature_visual};
use super::pick::{ortho_pick_ray, ray_sphere_t};
use crate::camera::zoom::OrbitState;

#[derive(Component)]
pub struct StarterChoice(pub CreatureType);

#[derive(Component)]
pub struct StarterRoot;

const STARTER_SCALE: f32 = 60.0;

pub fn spawn_starters(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    orbit: Res<OrbitState>,
) {
    // Place the two choices in front of the camera, flanking the pivot.
    let cam = orbit.camera_pos();
    let fwd = (orbit.pivot - cam).normalize();
    let right = fwd.cross(Vec3::Y).normalize();
    let mid = cam + fwd * (orbit.distance * 0.5);

    for (i, t) in [CreatureType::Fire, CreatureType::Water].into_iter().enumerate() {
        let side = if i == 0 { -1.0 } else { 1.0 };
        let pos = mid + right * side * STARTER_SCALE * 2.0;
        let vis = build_visual(t, STARTER_SCALE, &mut meshes, &mut materials);
        let root = commands.spawn((
            Transform::from_translation(pos),
            GlobalTransform::default(),
            Visibility::Visible,
            StarterChoice(t),
            StarterRoot,
            super::ClickSphere { radius: STARTER_SCALE * 1.5 },
        )).id();
        spawn_creature_visual(&mut commands, root, Transform::IDENTITY, &vis);
    }
}

pub fn pick_starter(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    proj_query: Query<&Projection, With<Camera3d>>,
    orbit: Res<OrbitState>,
    choices: Query<(&GlobalTransform, &super::ClickSphere, &StarterChoice)>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if !mouse_buttons.just_pressed(MouseButton::Left) { return; }
    let Ok(Projection::Orthographic(ortho)) = proj_query.get_single() else { return };
    let Ok(window) = windows.get_single() else { return };
    let Some((origin, fwd)) = ortho_pick_ray(window, ortho.scale, &orbit) else { return };

    let mut best: Option<(CreatureType, f32)> = None;
    for (gt, cs, choice) in &choices {
        if let Some(t) = ray_sphere_t(origin, fwd, gt.translation(), cs.radius) {
            if best.map_or(true, |(_, bt)| t < bt) { best = Some((choice.0, t)); }
        }
    }
    if let Some((t, _)) = best {
        commands.insert_resource(PlayerCreature(Creature::new(Species::of_type(t), 5)));
        next_state.set(GameState::Exploring);
    }
}

pub fn despawn_starters(mut commands: Commands, roots: Query<Entity, With<StarterRoot>>) {
    for e in &roots { commands.entity(e).despawn_recursive(); }
}
```

- [ ] **Step 2: Register the state-scoped systems in `mod.rs`**

In `CreaturesPlugin::build`:

```rust
           .add_systems(OnEnter(GameState::StarterSelect), starter::spawn_starters)
           .add_systems(OnExit(GameState::StarterSelect), starter::despawn_starters)
           .add_systems(Update, starter::pick_starter.run_if(in_state(GameState::StarterSelect)))
           .add_systems(Update, pick::pick_wild_monster.run_if(in_state(GameState::Exploring)))
```

> Also gate the existing planet picker so clicks don't fight: in `RendererPlugin` (`src/renderer/mod.rs`), add `.run_if(in_state(crate::creatures::GameState::Exploring))` to `solar::pick_planet`. Import as needed.

- [ ] **Step 3: Verify the flow**

Run: `cargo run`
Expected: on launch two creatures (a fiery one, a watery one) float in view; clicking one removes both and enters Exploring; clicking planets/monsters now works.

- [ ] **Step 4: Commit**

```bash
git add src/creatures/starter.rs src/creatures/mod.rs src/renderer/mod.rs
git commit -m "feat(creatures): starter selection screen"
```

---

## Phase 3 — Battle core

### Task 10: Damage + leveling logic (TDD)

**Files:**
- Modify: `src/creatures/mod.rs`

- [ ] **Step 1: Write failing tests**

Add to the `tests` module:

```rust
#[test]
fn damage_scales_with_type_and_power() {
    let fire = Creature::new(Species::Emberling, 5);
    let water = Creature::new(Species::Tideling, 5);
    let strong = Move { name: "Torrent", power: 14 };
    // water -> fire is x2; deterministic roll = 1.0
    let d = damage(&water, &fire, strong, 1.0);
    // fire -> water is x0.5
    let d2 = damage(&fire, &water, Move { name: "Flare", power: 14 }, 1.0);
    assert!(d > d2, "super-effective hit should beat not-very-effective: {d} vs {d2}");
    assert!(d >= 1);
}

#[test]
fn xp_levels_up_and_raises_stats() {
    let mut c = Creature::new(Species::Emberling, 1);
    let (hp0, atk0) = (c.max_hp, c.attack);
    apply_xp(&mut c, 1000); // plenty to level up at least once
    assert!(c.level > 1);
    assert!(c.max_hp > hp0 && c.attack > atk0);
    assert_eq!(c.hp, c.max_hp, "level-up heals to full");
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test damage_scales xp_levels_up`
Expected: FAIL — missing `damage` / `apply_xp`.

- [ ] **Step 3: Implement the functions**

```rust
/// `roll` is a damage multiplier in [0.85, 1.0]; pass 1.0 for deterministic tests.
pub fn damage(attacker: &Creature, defender: &Creature, mv: Move, roll: f32) -> i32 {
    let mult = type_multiplier(attacker.creature_type(), defender.creature_type());
    let base = mv.power as f32 * (attacker.attack as f32 / 10.0) * mult * roll;
    base.round().max(1.0) as i32
}

pub fn xp_reward(wild_level: u32) -> u32 { wild_level * 20 }
pub fn xp_to_next(level: u32) -> u32 { level * 50 }

pub fn apply_xp(c: &mut Creature, gained: u32) {
    c.xp += gained;
    while c.xp >= xp_to_next(c.level) {
        c.xp -= xp_to_next(c.level);
        c.level += 1;
        c.max_hp += 5;
        c.attack += 2;
    }
    c.hp = c.max_hp;
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test damage_scales xp_levels_up`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/creatures/mod.rs
git commit -m "feat(creatures): damage and xp/leveling logic"
```

---

### Task 11: Battle setup & teardown (camera framing, spawn player creature, HP bars)

**Files:**
- Modify: `src/creatures/battle.rs`, `src/creatures/battle_view.rs`, `src/creatures/mod.rs`

- [ ] **Step 1: Implement battle setup in `battle.rs`**

`src/creatures/battle.rs`:

```rust
use bevy::prelude::*;
use super::{BattleSession, PlayerCreature, Creature, WildMonster, GameState, Turn};
use super::spawn::build_visual;
use super::spawn::spawn_creature_visual;
use super::battle_view::{spawn_hp_bar, HpBar, BattleSide};
use crate::camera::zoom::OrbitState;

const BATTLE_CREATURE_SCALE: f32 = 40.0;

pub fn setup_battle(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut session: ResMut<BattleSession>,
    player: Res<PlayerCreature>,
    mut orbit: ResMut<OrbitState>,
    wild_q: Query<(&GlobalTransform, &Creature), With<WildMonster>>,
) {
    let Ok((wild_gt, wild)) = wild_q.get(session.wild_entity) else { return };
    let wild_pos = wild_gt.translation();

    // Frame the battle: pivot on the wild monster, pull camera to a fixed close distance.
    session.saved_orbit = Some(orbit.clone());
    orbit.pivot = wild_pos;
    orbit.distance = BATTLE_CREATURE_SCALE * 12.0;
    orbit.elevation = 0.2;

    session.wild_hp = wild.hp;
    session.player_hp = player.0.max_hp; // heal to full at battle start
    session.turn = Turn::Player;

    // Player creature stands a bit toward the camera-left of the wild monster.
    let cam = orbit.camera_pos();
    let fwd = (orbit.pivot - cam).normalize();
    let right = fwd.cross(Vec3::Y).normalize();
    let player_pos = wild_pos - fwd * BATTLE_CREATURE_SCALE * 4.0 + right * BATTLE_CREATURE_SCALE * 2.0;

    let vis = build_visual(player.0.creature_type(), BATTLE_CREATURE_SCALE, &mut meshes, &mut materials);
    let player_root = commands.spawn((
        Transform::from_translation(player_pos),
        GlobalTransform::default(),
        Visibility::Visible,
        BattleSide::Player,
    )).id();
    spawn_creature_visual(&mut commands, player_root, Transform::IDENTITY, &vis);
    session.view_entities.push(player_root);

    // HP bars (billboarded), one above each creature.
    let pbar = spawn_hp_bar(&mut commands, &mut meshes, &mut materials, player_pos + Vec3::Y * BATTLE_CREATURE_SCALE * 2.0, BattleSide::Player);
    let wbar = spawn_hp_bar(&mut commands, &mut meshes, &mut materials, wild_pos + Vec3::Y * BATTLE_CREATURE_SCALE * 2.0, BattleSide::Wild);
    session.view_entities.push(pbar);
    session.view_entities.push(wbar);
}

pub fn teardown_battle(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    mut orbit: ResMut<OrbitState>,
) {
    if let Some(saved) = session.saved_orbit.take() { *orbit = saved; }
    for e in session.view_entities.drain(..) {
        commands.entity(e).despawn_recursive();
    }
}
```

- [ ] **Step 2: Implement HP-bar billboards in `battle_view.rs`**

`src/creatures/battle_view.rs`:

```rust
use bevy::prelude::*;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum BattleSide { Player, Wild }

#[derive(Component)]
pub struct HpBar { pub side: BattleSide }

const BAR_W: f32 = 90.0;
const BAR_H: f32 = 10.0;

/// Spawns a billboarded HP bar: dark background quad + green fill quad (child).
pub fn spawn_hp_bar(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
    side: BattleSide,
) -> Entity {
    let bg_mesh = meshes.add(Rectangle::new(BAR_W, BAR_H));
    let bg_mat = materials.add(StandardMaterial { base_color: Color::srgb(0.05, 0.05, 0.05), unlit: true, ..default() });
    let fill_mesh = meshes.add(Rectangle::new(BAR_W, BAR_H));
    let fill_mat = materials.add(StandardMaterial { base_color: Color::srgb(0.2, 0.9, 0.3), unlit: true, ..default() });

    commands.spawn((
        Mesh3d(bg_mesh),
        MeshMaterial3d(bg_mat),
        Transform::from_translation(pos),
        HpBar { side },
    )).with_children(|c| {
        c.spawn((
            Mesh3d(fill_mesh),
            MeshMaterial3d(fill_mat),
            Transform::from_xyz(0.0, 0.0, 0.1), // sit just in front of bg
            HpFill,
        ));
    }).id()
}

#[derive(Component)]
pub struct HpFill;

/// Billboard all HP bars to face the camera each frame.
pub fn billboard_hp_bars(
    camera: Query<&Transform, (With<Camera3d>, Without<HpBar>)>,
    mut bars: Query<&mut Transform, With<HpBar>>,
) {
    let Ok(cam) = camera.get_single() else { return };
    for mut t in &mut bars {
        let look = t.translation + (t.translation - cam.translation);
        t.look_at(look, Vec3::Y);
    }
}

/// Scale the green fill to the current HP fraction (anchored on the left edge).
pub fn update_hp_bars(
    session: Res<super::BattleSession>,
    player: Res<super::PlayerCreature>,
    wild_q: Query<&super::Creature, With<super::WildMonster>>,
    bars: Query<(&HpBar, &Children)>,
    mut fills: Query<&mut Transform, With<HpFill>>,
) {
    let wild_max = wild_q.get(session.wild_entity).map(|c| c.max_hp).unwrap_or(1).max(1);
    for (bar, children) in &bars {
        let frac = match bar.side {
            BattleSide::Player => session.player_hp as f32 / player.0.max_hp.max(1) as f32,
            BattleSide::Wild   => session.wild_hp as f32 / wild_max as f32,
        }.clamp(0.0, 1.0);
        for &child in children {
            if let Ok(mut t) = fills.get_mut(child) {
                t.scale.x = frac;
                // keep left edge fixed: shift by half the removed width
                t.translation.x = -BAR_W * (1.0 - frac) * 0.5;
            }
        }
    }
}
```

- [ ] **Step 3: Register battle lifecycle + view systems in `mod.rs`**

```rust
           .add_systems(OnEnter(GameState::Battle), battle::setup_battle)
           .add_systems(OnExit(GameState::Battle), battle::teardown_battle)
           .add_systems(Update, (
               battle_view::billboard_hp_bars,
               battle_view::update_hp_bars,
           ).run_if(in_state(GameState::Battle)))
```

- [ ] **Step 4: Verify build + manual framing check**

Run: `cargo build` then `cargo run`
Expected: compiles. Pick a starter, zoom to a planet, click a wild monster → camera reframes, your creature appears facing it, two HP bars float above. (Battle has no controls yet — that's Task 12. Temporarily, you can return to Exploring by adding a debug key, or proceed to Task 12.)

- [ ] **Step 5: Commit**

```bash
git add src/creatures/battle.rs src/creatures/battle_view.rs src/creatures/mod.rs
git commit -m "feat(creatures): battle setup/teardown, framing, HP bars"
```

---

### Task 12: Turn resolution + input (one move each, win/lose)

**Files:**
- Modify: `src/creatures/battle.rs`, `src/creatures/mod.rs`

- [ ] **Step 1: Implement turn input + resolution**

Add to `battle.rs`:

```rust
use rand::SeedableRng;
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use super::{Inventory, Collection, apply_xp, xp_reward, damage, trap_chance};

#[derive(Resource)]
pub struct BattleRng(pub ChaCha8Rng);

pub fn battle_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut session: ResMut<BattleSession>,
    mut player: ResMut<PlayerCreature>,
    mut inventory: ResMut<Inventory>,
    mut collection: ResMut<Collection>,
    mut wild_q: Query<&mut Creature, With<WildMonster>>,
    mut commands: Commands,
    mut rng: ResMut<BattleRng>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if session.turn != Turn::Player { return; }
    let Ok(mut wild) = wild_q.get_mut(session.wild_entity) else { return };

    let move_idx = if keys.just_pressed(KeyCode::Digit1) { Some(0) }
                   else if keys.just_pressed(KeyCode::Digit2) { Some(1) }
                   else { None };
    let throw = keys.just_pressed(KeyCode::KeyF);

    if let Some(idx) = move_idx {
        let mv = player.0.species.moves()[idx];
        let roll = rng.0.gen_range(0.85_f32..=1.0);
        let dmg = damage(&player.0, &wild, mv, roll);
        session.wild_hp -= dmg;
        wild.hp = session.wild_hp;
        if session.wild_hp <= 0 {
            // WIN
            apply_xp(&mut player.0, xp_reward(wild.level));
            commands.entity(session.wild_entity).despawn_recursive();
            next_state.set(GameState::Exploring);
            return;
        }
        session.turn = Turn::Enemy;
    } else if throw {
        if inventory.force_fields == 0 { return; }
        inventory.force_fields -= 1;
        let p = trap_chance(session.wild_hp, wild.max_hp, 0.2);
        if rng.0.gen_range(0.0_f32..1.0) < p {
            // CAUGHT
            let mut caught = wild.clone();
            caught.hp = caught.max_hp;
            collection.0.push(caught);
            commands.entity(session.wild_entity).despawn_recursive();
            next_state.set(GameState::Exploring);
            return;
        }
        session.turn = Turn::Enemy; // failed throw wastes the turn
    }
}

pub fn enemy_turn(
    mut session: ResMut<BattleSession>,
    mut player: ResMut<PlayerCreature>,
    wild_q: Query<&Creature, With<WildMonster>>,
    mut rng: ResMut<BattleRng>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if session.turn != Turn::Enemy { return; }
    let Ok(wild) = wild_q.get(session.wild_entity) else { return };
    let mv = wild.species.moves()[rng.0.gen_range(0..2)];
    let roll = rng.0.gen_range(0.85_f32..=1.0);
    let dmg = damage(wild, &player.0, mv, roll);
    session.player_hp -= dmg;
    if session.player_hp <= 0 {
        // LOSE: faint, heal, back to explore (monster stays)
        player.0.heal_full();
        next_state.set(GameState::Exploring);
        return;
    }
    session.turn = Turn::Player;
}
```

> `trap_chance` is implemented in Task 13. If executing strictly in order, implement Task 13's `trap_chance` first (it is a tiny pure fn) or stub it. Recommended ordering: do Task 13 before Task 12's compile.

- [ ] **Step 2: Register systems + RNG resource**

In `mod.rs` `CreaturesPlugin::build`:

```rust
           .insert_resource(battle::BattleRng(rand_chacha::ChaCha8Rng::seed_from_u64(0x_B47_71E)))
           .add_systems(Update, (
               battle::battle_input,
               battle::enemy_turn,
           ).chain().run_if(in_state(GameState::Battle)))
```

Add `use rand::SeedableRng;` and `use rand_chacha;` imports to `mod.rs` as needed.

- [ ] **Step 3: Verify the full loop**

Run: `cargo run`
Expected: in battle, `1`/`2` damage the wild monster (HP bar drains, super-effective hits drain more), enemy hits back, reducing your bar. Reaching 0 wild HP returns to Exploring with the monster gone; reaching 0 player HP returns to Exploring with the monster still present.

- [ ] **Step 4: Commit**

```bash
git add src/creatures/battle.rs src/creatures/mod.rs
git commit -m "feat(creatures): turn-based battle resolution and win/lose"
```

---

## Phase 4 — (folded in) Type matchup, two moves, XP

Type matchup (Task 2), two moves per species (Task 1 `moves()`), and XP/leveling (Task 10) are already implemented and exercised by Task 12. No additional tasks — verify coverage:

- [ ] **Verify:** `cargo test` passes all logic tests; a manual battle confirms super-effective vs not-very-effective damage differs and a win raises your level after enough XP.

---

## Phase 5 — Capture (force field + collection)

### Task 13: Trap chance logic (TDD)

**Files:**
- Modify: `src/creatures/mod.rs`

- [ ] **Step 1: Write failing tests**

Add to the `tests` module:

```rust
#[test]
fn trap_chance_rises_as_hp_falls_and_is_clamped() {
    // full HP → only the base chance
    let full = trap_chance(30, 30, 0.2);
    // near-zero HP → close to base + 1, clamped to 1
    let low = trap_chance(1, 30, 0.2);
    assert!(low > full);
    assert!((0.0..=1.0).contains(&full));
    assert!((0.0..=1.0).contains(&low));
    assert_eq!(trap_chance(0, 30, 0.2), 1.0_f32.min(0.2 + 1.0).min(1.0));
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test trap_chance_rises -- --exact`
Expected: FAIL — missing `trap_chance`.

- [ ] **Step 3: Implement `trap_chance`**

```rust
pub fn trap_chance(hp: i32, max_hp: i32, base: f32) -> f32 {
    let frac = (hp.max(0) as f32) / (max_hp.max(1) as f32);
    (base + (1.0 - frac)).clamp(0.0, 1.0)
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test trap_chance_rises -- --exact`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/creatures/mod.rs
git commit -m "feat(creatures): trap chance logic"
```

> The force-field throw + `Collection` push is already wired in Task 12 (`battle_input`, `KeyCode::KeyF`). With `trap_chance` now defined, the throw path is complete.

- [ ] **Step 6: Manual capture check**

Run: `cargo run`
Expected: weaken a wild monster, press `F` — with low enemy HP it is usually captured (disappears, added to `Collection`); a failed throw consumes a force field and gives the enemy a turn. After 3 throws with no successes, `F` does nothing.

---

## Final verification

- [ ] **Run the whole suite**

Run: `cargo test`
Expected: all tests pass.

- [ ] **Full manual playthrough**

Run: `cargo run`
Expected: pick starter → explore → click wild monster → fight (moves, type damage, win=XP/level, lose=heal) → trap a weakened monster with `F`. No panics.

- [ ] **Final commit / branch ready for review**

```bash
git add -A
git commit -m "feat(creatures): complete creature-battles loop"
```
