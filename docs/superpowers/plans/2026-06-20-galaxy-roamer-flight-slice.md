# Galaxy Roamer — Flight Vertical Slice Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Bevy/Rust vertical slice where you fly a ship through a deterministic starfield, approach a planet, and descend until terrain resolves under you — and it feels good.

**Architecture:** A flight-first spine built on a camera-relative floating origin: authoritative positions are stored in f64 (`WorldPos`), the ship's position is published each frame as the `FloatingOrigin`, and every render `Transform` is derived as `(WorldPos − origin) as f32` so the ship always renders at `(0,0,0)`. The same mechanism carries a planet continuously from a distant dot, to a sphere, to displaced terrain — no loading screen, no coordinate switch. Distance-based LOD swaps body meshes; the v1 flight feel (mouse-steer, no roll, momentum, slide-collision) ports in as pure functions.

**Tech Stack:** Rust (edition 2021), Bevy 0.15, `noise` 0.9, `rand` 0.8 + `rand_chacha` 0.3.

## Global Constraints

- Bevy version: `0.15`, Rust edition `2021` (do not bump either).
- Procedural geometry only — no external art/audio assets (Bevy mesh primitives + flat/emissive `StandardMaterial`).
- All generation is deterministic from a single seed and unit-tested (same seed → identical output).
- All flight/feel tunables live in **one** resource, `FlightTuning` (no scattered `const`s once Task 3 lands).
- Every phase compiles and runs on its own; commit at the end of every task.
- Window: `1280.0 x 720.0`, title `"Galaxy Roamer"`. Target 60 fps.
- Distances are in kilometres (km); positions in world space are `f64` (`DVec3`), velocities are `f32` (`Vec3`) km/s.

---

## File Structure

```
src/
  main.rs              # window, plugin wiring, light, quit-on-escape
  origin/mod.rs        # WorldPos, FloatingOrigin, FrameSet, world_to_render, sync_transforms
  ship/
    mod.rs             # ShipPlugin, FlightTuning, components, system wiring
    physics.rs         # pure: steer, ship_rotation, integrate_velocity, clamp_to_planet (carried from v1)
    systems.rs         # flight_input, ship_move, set_origin, chase_camera
  galaxy/
    mod.rs             # GalaxyPlugin, GalaxyData resource, generate(), spawn_galaxy
    gen.rs             # pure: PlanetType, Star, Planet, generate(seed) -> GalaxyData
  palette/mod.rs       # Palette: colors per PlanetType + star color + background
  streaming/mod.rs     # StreamingPlugin, LodTier, LodBody, tier_for_distance, update_lod
  terrain/mod.rs       # TerrainPlugin, displace_height, terrain swap on descent
```

---

## Task 0: Clean Slate (P0)

Delete the v1 renderer-first codebase and reduce `main.rs` to a window + camera that runs. v1 remains in git history as reference.

**Files:**
- Delete: `src/world_gen/`, `src/camera/`, `src/lod/`, `src/renderer/`, `src/creatures/`, `src/flight/`, `src/theme/` (entire directories)
- Modify: `src/main.rs` (full rewrite)

**Interfaces:**
- Consumes: nothing.
- Produces: a running empty app with a `Camera3d` and a `DirectionalLight`. No public Rust interface yet.

- [ ] **Step 1: Delete all v1 source modules**

```bash
git rm -r src/world_gen src/camera src/lod src/renderer src/creatures src/flight src/theme
```

- [ ] **Step 2: Rewrite `src/main.rs` to a minimal running app**

```rust
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Galaxy Roamer".into(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.01, 0.01, 0.03)))
        .add_systems(Startup, setup)
        .add_systems(Update, quit_on_escape)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 50.0, 200.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight { illuminance: 10_000.0, shadows_enabled: false, ..default() },
        Transform::from_xyz(1.0, 2.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn quit_on_escape(keys: Res<ButtonInput<KeyCode>>, mut exit: EventWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.send(AppExit::Success);
    }
}
```

- [ ] **Step 3: Verify it builds**

Run: `cargo build`
Expected: compiles with no errors (warnings about unused `noise`/`rand` deps are fine).

- [ ] **Step 4: Verify it runs**

Run: `cargo run` (then press Escape to quit)
Expected: a 1280×720 window titled "Galaxy Roamer" opens with a dark background, then closes on Escape.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(p0): clean slate — delete v1 src, minimal window+camera app"
```

---

## Task 1: Floating-Origin Core (P1)

The spine. `WorldPos` (f64) component, `FloatingOrigin` resource, the pure `world_to_render` conversion, the `sync_transforms` system, and the `FrameSet` ordering every later system hooks into.

**Files:**
- Create: `src/origin/mod.rs`
- Modify: `src/main.rs` (add `mod origin;`, add `OriginPlugin`)

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub struct WorldPos(pub bevy::math::DVec3)` — `Component, Clone, Copy, Debug`
  - `pub struct FloatingOrigin(pub bevy::math::DVec3)` — `Resource, Default`
  - `pub fn world_to_render(world: DVec3, origin: DVec3) -> Vec3`
  - `pub enum FrameSet { Input, Move, Origin, Sync, Camera, Stream }` — `SystemSet`, ordered Input→Move→Origin→Sync→Camera→Stream
  - `pub struct OriginPlugin` — registers `FloatingOrigin`, configures `FrameSet` chain, runs `sync_transforms` in `FrameSet::Sync`
  - `pub fn sync_transforms` (system)

- [ ] **Step 1: Write the failing test for `world_to_render`**

Create `src/origin/mod.rs`:

```rust
use bevy::prelude::*;
use bevy::math::DVec3;

pub fn world_to_render(world: DVec3, origin: DVec3) -> Vec3 {
    (world - origin).as_vec3()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_pos_is_world_minus_origin() {
        let world = DVec3::new(1_000_000.5, 0.0, -2_000_000.25);
        let origin = DVec3::new(1_000_000.0, 0.0, -2_000_000.0);
        let r = world_to_render(world, origin);
        assert!((r - Vec3::new(0.5, 0.0, -0.25)).length() < 1e-4,
            "far-from-origin pair resolves to a small, precise offset");
    }

    #[test]
    fn ship_at_origin_renders_at_zero() {
        let p = DVec3::new(9_876_543.0, 12_345.0, -555_555.0);
        assert_eq!(world_to_render(p, p), Vec3::ZERO);
    }
}
```

- [ ] **Step 2: Run the test to verify it passes**

Run: `cargo test --lib origin::tests`
Expected: PASS (2 tests). (The function is already implemented; these lock in the contract.)

- [ ] **Step 3: Add the components, resource, FrameSet, plugin, and sync system**

Append to `src/origin/mod.rs` (above the `#[cfg(test)]` block):

```rust
#[derive(Component, Clone, Copy, Debug)]
pub struct WorldPos(pub DVec3);

#[derive(Resource, Default)]
pub struct FloatingOrigin(pub DVec3);

/// Ordered phases every per-frame system slots into. Configured as a chain in
/// `OriginPlugin` so input → movement → origin publish → transform sync →
/// camera → streaming always run in that order.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum FrameSet {
    Input,
    Move,
    Origin,
    Sync,
    Camera,
    Stream,
}

/// Derive every entity's render `Transform` from its f64 `WorldPos` relative to
/// the current floating origin (the ship). Runs in `FrameSet::Sync`.
pub fn sync_transforms(
    origin: Res<FloatingOrigin>,
    mut q: Query<(&WorldPos, &mut Transform)>,
) {
    for (wp, mut tf) in &mut q {
        tf.translation = world_to_render(wp.0, origin.0);
    }
}

pub struct OriginPlugin;

impl Plugin for OriginPlugin {
    fn build(&self, app: &mut App) {
        use bevy::ecs::schedule::IntoSystemSetConfigs;
        app.init_resource::<FloatingOrigin>()
            .configure_sets(Update, (
                FrameSet::Input,
                FrameSet::Move,
                FrameSet::Origin,
                FrameSet::Sync,
                FrameSet::Camera,
                FrameSet::Stream,
            ).chain())
            .add_systems(Update, sync_transforms.in_set(FrameSet::Sync));
    }
}
```

- [ ] **Step 4: Write an App-level test for `sync_transforms`**

Add inside the `#[cfg(test)] mod tests` block in `src/origin/mod.rs`:

```rust
    #[test]
    fn sync_transforms_places_entity_relative_to_origin() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(OriginPlugin);
        app.insert_resource(FloatingOrigin(DVec3::new(1_000_000.0, 0.0, 0.0)));

        let e = app.world_mut().spawn((
            WorldPos(DVec3::new(1_000_010.0, 0.0, 0.0)),
            Transform::default(),
        )).id();

        app.update();

        let tf = app.world().get::<Transform>(e).unwrap();
        assert!((tf.translation - Vec3::new(10.0, 0.0, 0.0)).length() < 1e-3,
            "entity renders 10 km from the origin");
    }
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test --lib origin::tests`
Expected: PASS (3 tests).

- [ ] **Step 6: Wire `OriginPlugin` into `main.rs`**

In `src/main.rs`, add `mod origin;` at the top and `.add_plugins(origin::OriginPlugin)` in the `App` builder (after `DefaultPlugins`). Then:

Run: `cargo build`
Expected: compiles.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat(p1): floating-origin core — WorldPos, FloatingOrigin, FrameSet, sync_transforms"
```

---

## Task 2: Ship Physics (pure, carried from v1) (P1)

Port v1's proven, pure flight math into the new `ship` module: yaw/pitch steering with no roll, velocity integration with drag/boost clamp, and slide-collision against a planet shell. These are pure functions with v1's tests.

**Files:**
- Create: `src/ship/mod.rs` (module declaration only this task), `src/ship/physics.rs`
- Modify: `src/main.rs` (add `mod ship;`)

**Interfaces:**
- Consumes: nothing (pure math over `glam`/`bevy::math` types).
- Produces (all in `ship::physics`):
  - `pub const MAX_PITCH: f32`
  - `pub fn steer(yaw: f32, pitch: f32, cursor_off: Vec2, rate: f32, dt: f32) -> (f32, f32)`
  - `pub fn ship_rotation(yaw: f32, pitch: f32) -> Quat`
  - `pub fn integrate_velocity(v: Vec3, thrust_accel: Vec3, drag_half_life: f32, max_speed: f32, dt: f32) -> Vec3`
  - `pub fn clamp_to_planet(pos: Vec3, vel: Vec3, center: Vec3, radius: f32, margin: f32) -> (Vec3, Vec3)`

- [ ] **Step 1: Create `src/ship/mod.rs` declaring the physics submodule**

```rust
pub mod physics;
```

- [ ] **Step 2: Write `src/ship/physics.rs` with the pure functions and their failing-then-passing tests**

```rust
use bevy::prelude::*;
use bevy::math::EulerRot;

/// Maximum pitch from level, in radians (~69°). Keeps the ship from flipping
/// over the top and keeps the chase camera's `look_at` well away from vertical.
pub const MAX_PITCH: f32 = 1.2;

/// Update `(yaw, pitch)` from a normalized cursor offset where centre = (0,0),
/// right = +x, down = +y. No roll is ever introduced, so world up stays up.
pub fn steer(yaw: f32, pitch: f32, cursor_off: Vec2, rate: f32, dt: f32) -> (f32, f32) {
    let new_yaw = yaw - cursor_off.x * rate * dt;
    let new_pitch = (pitch - cursor_off.y * rate * dt).clamp(-MAX_PITCH, MAX_PITCH);
    (new_yaw, new_pitch)
}

/// Build the ship's rotation from yaw/pitch with world up preserved (no roll).
/// Forward is `-Z` at neutral, matching Bevy's default facing.
pub fn ship_rotation(yaw: f32, pitch: f32) -> Quat {
    Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0)
}

/// Integrate one physics step: apply thrust acceleration, exponential drag,
/// then clamp to `max_speed`. `drag_half_life` is the time for an un-thrusted
/// velocity to halve.
pub fn integrate_velocity(
    v: Vec3,
    thrust_accel: Vec3,
    drag_half_life: f32,
    max_speed: f32,
    dt: f32,
) -> Vec3 {
    let mut next = v + thrust_accel * dt;
    let decay = 0.5_f32.powf(dt / drag_half_life.max(1e-6));
    next *= decay;
    if next.length() > max_speed {
        next = next.normalize_or_zero() * max_speed;
    }
    next
}

/// If `pos` is inside `radius + margin` of a planet at `center`, project it back
/// onto that shell and remove the inward component of `vel` (slide, not bounce).
pub fn clamp_to_planet(
    pos: Vec3,
    vel: Vec3,
    center: Vec3,
    radius: f32,
    margin: f32,
) -> (Vec3, Vec3) {
    let shell = radius + margin;
    let offset = pos - center;
    let dist = offset.length();
    if dist >= shell || dist < 1e-4 {
        return (pos, vel);
    }
    let outward = offset / dist;
    let new_pos = center + outward * shell;
    let inward = vel.dot(outward).min(0.0);
    let new_vel = vel - outward * inward;
    (new_pos, new_vel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ship_rotation_neutral_faces_forward_and_upright() {
        let r = ship_rotation(0.0, 0.0);
        assert!((r * Vec3::NEG_Z - Vec3::NEG_Z).length() < 1e-5);
        assert!((r * Vec3::Y - Vec3::Y).length() < 1e-5);
    }

    #[test]
    fn cursor_up_pitches_nose_up_down_pitches_down() {
        let (y, p) = steer(0.0, 0.0, Vec2::new(0.0, -1.0), 1.0, 0.1);
        assert!((ship_rotation(y, p) * Vec3::NEG_Z).y > 0.0);
        let (y, p) = steer(0.0, 0.0, Vec2::new(0.0, 1.0), 1.0, 0.1);
        assert!((ship_rotation(y, p) * Vec3::NEG_Z).y < 0.0);
    }

    #[test]
    fn cursor_right_turns_right_left_turns_left() {
        let (y, p) = steer(0.0, 0.0, Vec2::new(1.0, 0.0), 1.0, 0.1);
        assert!((ship_rotation(y, p) * Vec3::NEG_Z).x > 0.0);
        let (y, p) = steer(0.0, 0.0, Vec2::new(-1.0, 0.0), 1.0, 0.1);
        assert!((ship_rotation(y, p) * Vec3::NEG_Z).x < 0.0);
    }

    #[test]
    fn pitch_is_clamped() {
        let (_, p) = steer(0.0, 0.0, Vec2::new(0.0, -100.0), 5.0, 1.0);
        assert!(p <= MAX_PITCH + 1e-6 && p >= -MAX_PITCH - 1e-6);
    }

    #[test]
    fn ship_never_rolls_up_stays_above_horizon() {
        for &yaw in &[-3.1_f32, -1.0, 0.0, 2.0, 3.1] {
            for &pitch in &[-MAX_PITCH, -0.5_f32, 0.0, 0.5, MAX_PITCH] {
                let up = ship_rotation(yaw, pitch) * Vec3::Y;
                assert!(up.y > 0.0, "up stays above horizon (yaw={yaw}, pitch={pitch})");
            }
        }
    }

    #[test]
    fn thrust_increases_speed_along_direction() {
        let thrust = Vec3::new(0.0, 0.0, -1.0) * 80.0;
        let v1 = integrate_velocity(Vec3::ZERO, thrust, 1.0e9, 1.0e9, 0.5);
        assert!(v1.z < 0.0);
        assert!((v1.length() - 40.0).abs() < 1.0);
    }

    #[test]
    fn drag_halves_velocity_over_half_life() {
        let v1 = integrate_velocity(Vec3::new(0.0, 0.0, -100.0), Vec3::ZERO, 0.7, 1.0e9, 0.7);
        assert!((v1.length() - 50.0).abs() < 1.0);
    }

    #[test]
    fn speed_is_clamped_to_max() {
        let v1 = integrate_velocity(Vec3::new(0.0, 0.0, -1000.0), Vec3::ZERO, 1.0e9, 100.0, 0.016);
        assert!(v1.length() <= 100.0 + 1e-3);
    }

    #[test]
    fn boost_raises_the_clamp() {
        let base = integrate_velocity(Vec3::new(0.0, 0.0, -1000.0), Vec3::ZERO, 1.0e9, 100.0, 0.016);
        let boost = integrate_velocity(Vec3::new(0.0, 0.0, -1000.0), Vec3::ZERO, 1.0e9, 1000.0, 0.016);
        assert!(boost.length() > base.length());
    }

    #[test]
    fn clamp_pushes_ship_outside_planet_and_zeros_inward_velocity() {
        let (new_pos, new_vel) = clamp_to_planet(
            Vec3::new(0.0, 0.0, 250.0), Vec3::new(0.0, 0.0, -100.0),
            Vec3::ZERO, 350.0, 12.0);
        assert!((new_pos.length() - 362.0).abs() < 1e-2);
        assert!(new_vel.z >= -1e-3);
    }

    #[test]
    fn clamp_leaves_distant_ship_untouched() {
        let (p, v) = clamp_to_planet(
            Vec3::new(0.0, 0.0, 5000.0), Vec3::new(1.0, 2.0, 3.0),
            Vec3::ZERO, 350.0, 12.0);
        assert_eq!(p, Vec3::new(0.0, 0.0, 5000.0));
        assert_eq!(v, Vec3::new(1.0, 2.0, 3.0));
    }
}
```

- [ ] **Step 3: Add `mod ship;` to `main.rs` and run the tests**

In `src/main.rs` add `mod ship;` near the other module declarations.

Run: `cargo test --lib ship::physics`
Expected: PASS (11 tests).

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(p1): ship physics (pure) — steer/rotation/integrate/clamp ported from v1"
```

---

## Task 3: Ship Entity, Flight Systems & Chase Camera (P1)

Wire the physics into a flyable ship on the floating-origin spine, with all tunables in one `FlightTuning` resource. Deliverable: fly a cube through empty space with mouse-steer + chase camera. **This is the first "does it feel good?" checkpoint.**

**Files:**
- Modify: `src/ship/mod.rs`
- Create: `src/ship/systems.rs`
- Modify: `src/main.rs` (add `.add_plugins(ship::ShipPlugin)`, remove the temporary camera/spawn from Task 0's `setup` so the ship owns the camera)

**Interfaces:**
- Consumes: `origin::{WorldPos, FloatingOrigin, FrameSet}`; `ship::physics::*`.
- Produces:
  - `pub struct FlightTuning { base_max_speed, boost_mult, thrust_accel, strafe_accel, drag_half_life, turn_rate, steer_deadzone, cam_back, cam_up, cam_lookahead, cam_smooth_half_life: f32 }` — `Resource`, with `Default`
  - `pub struct PlayerShip { pub radius: f32 }` — `Component`
  - `pub struct ShipVelocity(pub Vec3)` — `Component, Default`
  - `pub struct ShipControl { pub yaw: f32, pub pitch: f32 }` — `Component, Default`
  - `pub struct ShipPlugin`
  - systems `flight_input` (FrameSet::Input), `ship_move` (FrameSet::Move), `set_origin` (FrameSet::Origin), `chase_camera` (FrameSet::Camera)

- [ ] **Step 1: Replace `src/ship/mod.rs` with components, tuning, and the plugin**

```rust
pub mod physics;
pub mod systems;

use bevy::prelude::*;
use bevy::ecs::schedule::IntoSystemConfigs;
use crate::origin::FrameSet;

/// Every flight/feel tunable in one place (Global Constraint). Adjust here.
#[derive(Resource)]
pub struct FlightTuning {
    pub base_max_speed: f32,
    pub boost_mult: f32,
    pub thrust_accel: f32,
    pub strafe_accel: f32,
    pub drag_half_life: f32,
    pub turn_rate: f32,
    pub steer_deadzone: f32,
    pub cam_back: f32,
    pub cam_up: f32,
    pub cam_lookahead: f32,
    pub cam_smooth_half_life: f32,
}

impl Default for FlightTuning {
    fn default() -> Self {
        Self {
            base_max_speed: 100.0,
            boost_mult: 10.0,
            thrust_accel: 80.0,
            strafe_accel: 50.0,
            drag_half_life: 0.7,
            turn_rate: 1.2,
            steer_deadzone: 0.08,
            cam_back: 60.0,
            cam_up: 25.0,
            cam_lookahead: 80.0,
            cam_smooth_half_life: 0.15,
        }
    }
}

#[derive(Component)]
pub struct PlayerShip {
    pub radius: f32,
}

#[derive(Component, Default)]
pub struct ShipVelocity(pub Vec3);

#[derive(Component, Default)]
pub struct ShipControl {
    pub yaw: f32,
    pub pitch: f32,
}

pub struct ShipPlugin;

impl Plugin for ShipPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FlightTuning>()
            .add_systems(Startup, systems::spawn_ship)
            .add_systems(Update, systems::flight_input.in_set(FrameSet::Input))
            .add_systems(Update, systems::ship_move.in_set(FrameSet::Move))
            .add_systems(Update, systems::set_origin.in_set(FrameSet::Origin))
            .add_systems(Update, systems::chase_camera.in_set(FrameSet::Camera));
    }
}
```

- [ ] **Step 2: Create `src/ship/systems.rs`**

```rust
use bevy::prelude::*;
use bevy::math::DVec3;
use bevy::window::PrimaryWindow;
use crate::origin::{WorldPos, FloatingOrigin};
use super::physics::{steer, ship_rotation, integrate_velocity};
use super::{FlightTuning, PlayerShip, ShipVelocity, ShipControl};

/// Spawn the player ship (a cube) at the world origin, plus its chase camera.
pub fn spawn_ship(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        PlayerShip { radius: 10.0 },
        ShipVelocity::default(),
        ShipControl::default(),
        WorldPos(DVec3::ZERO),
        Mesh3d(meshes.add(Cuboid::new(8.0, 8.0, 16.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.85, 0.95),
            emissive: LinearRgba::rgb(0.1, 0.2, 0.4),
            ..default()
        })),
        Transform::default(),
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 25.0, 60.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Mouse steers (cursor offset from screen centre), W/S thrust, A/D strafe,
/// hold Shift boosts. Facing tracked as yaw/pitch and rebuilt each frame so the
/// ship never rolls.
pub fn flight_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    tune: Res<FlightTuning>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut ship_q: Query<(&mut Transform, &mut ShipVelocity, &mut ShipControl), With<PlayerShip>>,
) {
    let dt = time.delta_secs();
    let Ok((mut tf, mut vel, mut ctl)) = ship_q.get_single_mut() else { return };

    if let Ok(window) = windows.get_single() {
        if let Some(cursor) = window.cursor_position() {
            let half = Vec2::new(window.width(), window.height()) * 0.5;
            let mut off = (cursor - half) / half;
            if off.length() < tune.steer_deadzone { off = Vec2::ZERO; }
            let (yaw, pitch) = steer(ctl.yaw, ctl.pitch, off, tune.turn_rate, dt);
            ctl.yaw = yaw;
            ctl.pitch = pitch;
        }
    }
    tf.rotation = ship_rotation(ctl.yaw, ctl.pitch);

    let boosting = keys.pressed(KeyCode::ShiftLeft);
    let max_speed = if boosting { tune.base_max_speed * tune.boost_mult } else { tune.base_max_speed };
    let thrust_mag = if boosting { tune.thrust_accel * tune.boost_mult } else { tune.thrust_accel };

    let fwd = tf.forward().as_vec3();
    let right = tf.right().as_vec3();
    let mut accel = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) { accel += fwd * thrust_mag; }
    if keys.pressed(KeyCode::KeyS) { accel -= fwd * thrust_mag; }
    if keys.pressed(KeyCode::KeyD) { accel += right * tune.strafe_accel; }
    if keys.pressed(KeyCode::KeyA) { accel -= right * tune.strafe_accel; }

    vel.0 = integrate_velocity(vel.0, accel, tune.drag_half_life, max_speed, dt);
}

/// Integrate the ship's f64 world position from its f32 velocity (km/s).
pub fn ship_move(
    time: Res<Time>,
    mut ship_q: Query<(&mut WorldPos, &ShipVelocity), With<PlayerShip>>,
) {
    let dt = time.delta_secs();
    let Ok((mut wp, vel)) = ship_q.get_single_mut() else { return };
    wp.0 += (vel.0 * dt).as_dvec3();
}

/// Publish the ship's world position as the floating origin, so it renders at
/// (0,0,0) and everything else is positioned relative to it.
pub fn set_origin(
    ship_q: Query<&WorldPos, With<PlayerShip>>,
    mut origin: ResMut<FloatingOrigin>,
) {
    if let Ok(wp) = ship_q.get_single() {
        origin.0 = wp.0;
    }
}

/// Chase camera: smoothed follow behind and above the ship, in render space
/// (the ship sits at ~origin after sync_transforms).
pub fn chase_camera(
    time: Res<Time>,
    tune: Res<FlightTuning>,
    ship_q: Query<&Transform, (With<PlayerShip>, Without<Camera3d>)>,
    mut cam_q: Query<&mut Transform, With<Camera3d>>,
) {
    let Ok(ship) = ship_q.get_single() else { return };
    let Ok(mut cam) = cam_q.get_single_mut() else { return };

    let fwd = ship.forward().as_vec3();
    let target = ship.translation - fwd * tune.cam_back + Vec3::Y * tune.cam_up;
    let look = ship.translation + fwd * tune.cam_lookahead;

    let t = 1.0 - 0.5_f32.powf(time.delta_secs() / tune.cam_smooth_half_life.max(1e-6));
    cam.translation = cam.translation.lerp(target, t);
    cam.look_at(look, Vec3::Y);
}
```

- [ ] **Step 3: Write an App test that the ship moves forward in world space**

Add to the bottom of `src/ship/systems.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn ship_move_advances_world_pos_along_velocity() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let e = app.world_mut().spawn((
            PlayerShip { radius: 10.0 },
            ShipVelocity(Vec3::new(0.0, 0.0, -100.0)),
            WorldPos(DVec3::ZERO),
        )).id();
        // Advance time so delta_secs() > 0.
        app.update();
        app.world_mut().run_system_once(ship_move).unwrap();
        let wp = app.world().get::<WorldPos>(e).unwrap();
        assert!(wp.0.z < 0.0, "ship advanced in -Z world space");
    }

    #[test]
    fn set_origin_follows_the_ship() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).init_resource::<FloatingOrigin>();
        app.world_mut().spawn((
            PlayerShip { radius: 10.0 },
            WorldPos(DVec3::new(42.0, 0.0, -7.0)),
        ));
        app.world_mut().run_system_once(set_origin).unwrap();
        let origin = app.world().resource::<FloatingOrigin>();
        assert_eq!(origin.0, DVec3::new(42.0, 0.0, -7.0));
    }
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib ship::systems`
Expected: PASS (2 tests). (`ship_move`'s assertion only needs `delta_secs() > 0`, which holds after the first `app.update()`.)

- [ ] **Step 5: Wire `ShipPlugin` into `main.rs` and drop the temporary camera**

In `src/main.rs`: add `.add_plugins(ship::ShipPlugin)`. In `setup`, **remove** the `Camera3d` spawn (the ship now owns the camera) but keep the `DirectionalLight`. Keep `quit_on_escape`.

- [ ] **Step 6: Build and fly it (manual feel check)**

Run: `cargo run`
Expected: a cube sits centred; moving the mouse off-centre yaws/pitches it (no roll — up stays up), W/S thrust forward/back, A/D strafe, hold Left-Shift to boost. The chase camera trails smoothly. Coast and drift after releasing thrust. Press Escape to quit.
**Checkpoint:** does flying feel responsive-but-weighty? Note any tuning to revisit in `FlightTuning`.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat(p1): flyable ship on floating origin — mouse-steer, chase cam, FlightTuning"
```

---

## Task 4: Deterministic Galaxy Generation (pure) (P2)

Pure, seeded generation of the slice's content: a backdrop starfield, a few star systems, and one designated descendable planet — all with f64 positions and a derived `PlanetType`.

**Files:**
- Create: `src/galaxy/mod.rs` (declares `gen`, re-exports types this task; plugin added in Task 6), `src/galaxy/gen.rs`
- Modify: `src/main.rs` (add `mod galaxy;`)

**Interfaces:**
- Consumes: nothing (pure; uses `rand`, `rand_chacha`, `bevy::math::DVec3`).
- Produces (in `galaxy::gen`):
  - `pub enum PlanetType { Rock, Fire, Water }` — `Clone, Copy, PartialEq, Debug`
  - `pub struct Star { pub pos: DVec3 }` — `Clone, Copy`
  - `pub struct Planet { pub pos: DVec3, pub radius: f64, pub kind: PlanetType, pub descendable: bool }` — `Clone, Copy`
  - `pub struct GalaxyData { pub stars: Vec<Star>, pub planets: Vec<Planet> }`
  - `pub fn generate(seed: u64) -> GalaxyData`

- [ ] **Step 1: Create `src/galaxy/mod.rs`**

```rust
pub mod gen;

pub use gen::{GalaxyData, Planet, PlanetType, Star};
```

- [ ] **Step 2: Write `src/galaxy/gen.rs` with `generate` and determinism tests**

```rust
use bevy::math::DVec3;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Number of distant backdrop stars (LOD point tier). Slice budget, not galaxy scale.
pub const BACKDROP_STARS: usize = 500;
/// Backdrop stars are scattered in a cube of this half-extent (km).
pub const BACKDROP_HALF_EXTENT: f64 = 2_000_000.0;
/// Number of nearby star systems the player can approach.
pub const SYSTEMS: usize = 3;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PlanetType {
    Rock,
    Fire,
    Water,
}

#[derive(Clone, Copy)]
pub struct Star {
    pub pos: DVec3,
}

#[derive(Clone, Copy)]
pub struct Planet {
    pub pos: DVec3,
    pub radius: f64,
    pub kind: PlanetType,
    pub descendable: bool,
}

pub struct GalaxyData {
    pub stars: Vec<Star>,
    pub planets: Vec<Planet>,
}

fn planet_type(rng: &mut ChaCha8Rng) -> PlanetType {
    match rng.gen_range(0..3) {
        0 => PlanetType::Rock,
        1 => PlanetType::Fire,
        _ => PlanetType::Water,
    }
}

/// Build the entire slice galaxy deterministically from `seed`.
pub fn generate(seed: u64) -> GalaxyData {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    let mut stars = Vec::with_capacity(BACKDROP_STARS);
    for _ in 0..BACKDROP_STARS {
        let e = BACKDROP_HALF_EXTENT;
        stars.push(Star {
            pos: DVec3::new(
                rng.gen_range(-e..e),
                rng.gen_range(-e..e),
                rng.gen_range(-e..e),
            ),
        });
    }

    let mut planets = Vec::new();
    for s in 0..SYSTEMS {
        // Systems sit a few thousand km out in front of the player's start.
        let center = DVec3::new(
            rng.gen_range(-3_000.0..3_000.0),
            rng.gen_range(-1_000.0..1_000.0),
            -4_000.0 - s as f64 * 6_000.0,
        );
        let n = rng.gen_range(1..=3);
        for p in 0..n {
            let offset = DVec3::new(
                rng.gen_range(-1_500.0..1_500.0),
                rng.gen_range(-400.0..400.0),
                rng.gen_range(-1_500.0..1_500.0),
            );
            planets.push(Planet {
                pos: center + offset,
                radius: rng.gen_range(200.0..500.0),
                kind: planet_type(&mut rng),
                // The first planet of the first system is the descend target.
                descendable: s == 0 && p == 0,
            });
        }
    }

    GalaxyData { stars, planets }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_produces_identical_galaxy() {
        let a = generate(42);
        let b = generate(42);
        assert_eq!(a.stars.len(), b.stars.len());
        assert_eq!(a.planets.len(), b.planets.len());
        for (x, y) in a.stars.iter().zip(b.stars.iter()) {
            assert_eq!(x.pos, y.pos);
        }
        for (x, y) in a.planets.iter().zip(b.planets.iter()) {
            assert_eq!(x.pos, y.pos);
            assert_eq!(x.radius, y.radius);
            assert_eq!(x.kind, y.kind);
            assert_eq!(x.descendable, y.descendable);
        }
    }

    #[test]
    fn different_seeds_differ() {
        let a = generate(1);
        let b = generate(2);
        assert!(a.stars[0].pos != b.stars[0].pos, "seed changes the starfield");
    }

    #[test]
    fn has_backdrop_stars_and_at_least_one_planet() {
        let g = generate(7);
        assert_eq!(g.stars.len(), BACKDROP_STARS);
        assert!(!g.planets.is_empty());
    }

    #[test]
    fn exactly_one_descendable_planet() {
        let g = generate(7);
        assert_eq!(g.planets.iter().filter(|p| p.descendable).count(), 1);
    }
}
```

- [ ] **Step 3: Add `mod galaxy;` to `main.rs` and run the tests**

Add `mod galaxy;` to `src/main.rs`.

Run: `cargo test --lib galaxy::gen`
Expected: PASS (4 tests).

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(p2): deterministic galaxy generation — starfield, systems, descend target"
```

---

## Task 5: Palette (P2)

A tiny carried-over color module mapping `PlanetType` → body color, plus a star color. Used by rendering in Task 6 and terrain in Task 9.

**Files:**
- Create: `src/palette/mod.rs`
- Modify: `src/main.rs` (add `mod palette;`)

**Interfaces:**
- Consumes: `galaxy::gen::PlanetType`.
- Produces (in `palette`):
  - `pub const STAR_COLOR: Color`
  - `pub fn planet_color(kind: PlanetType) -> Color`

- [ ] **Step 1: Write `src/palette/mod.rs` with the mapping and a test**

```rust
use bevy::prelude::*;
use crate::galaxy::gen::PlanetType;

/// Emissive-ish off-white for distant stars.
pub const STAR_COLOR: Color = Color::srgb(0.95, 0.93, 0.80);

/// Flat body color per planet type (dark-neon palette carried from v1).
pub fn planet_color(kind: PlanetType) -> Color {
    match kind {
        PlanetType::Rock => Color::srgb(0.35, 0.33, 0.30),
        PlanetType::Fire => Color::srgb(0.70, 0.25, 0.12),
        PlanetType::Water => Color::srgb(0.12, 0.30, 0.55),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_type_maps_to_a_distinct_color() {
        let r = planet_color(PlanetType::Rock).to_srgba();
        let f = planet_color(PlanetType::Fire).to_srgba();
        let w = planet_color(PlanetType::Water).to_srgba();
        assert!(r != f && f != w && r != w, "planet types are visually distinct");
    }
}
```

- [ ] **Step 2: Add `mod palette;` to `main.rs` and run the test**

Add `mod palette;` to `src/main.rs`.

Run: `cargo test --lib palette`
Expected: PASS (1 test).

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(p2): palette — planet-type and star colors"
```

---

## Task 6: Spawn the Galaxy (P2)

Turn `GalaxyData` into live entities: backdrop stars and system planets, each with a `WorldPos` so the floating origin renders them relative to the ship. Deliverable: fly through a starfield and approach distant planets.

**Files:**
- Modify: `src/galaxy/mod.rs` (add `GalaxyPlugin`, `GalaxyData` as a resource wrapper, `spawn_galaxy`)
- Modify: `src/main.rs` (add `.add_plugins(galaxy::GalaxyPlugin)`)

**Interfaces:**
- Consumes: `galaxy::gen::{generate, GalaxyData, Planet, Star}`; `origin::WorldPos`; `palette::{STAR_COLOR, planet_color}`.
- Produces:
  - `pub struct GalaxyPlugin`
  - `pub struct StarBody` — `Component` (marker)
  - `pub struct PlanetBody { pub radius: f32, pub descendable: bool }` — `Component`
  - resource `Galaxy(pub gen::GalaxyData)` — generated at startup with `WORLD_SEED`
  - `pub const WORLD_SEED: u64`

- [ ] **Step 1: Extend `src/galaxy/mod.rs` with the plugin and spawn system**

Replace `src/galaxy/mod.rs` with:

```rust
pub mod gen;

pub use gen::{GalaxyData, Planet, PlanetType, Star};

use bevy::prelude::*;
use crate::origin::WorldPos;
use crate::palette::{planet_color, STAR_COLOR};

pub const WORLD_SEED: u64 = 42;

#[derive(Resource)]
pub struct Galaxy(pub GalaxyData);

#[derive(Component)]
pub struct StarBody;

#[derive(Component)]
pub struct PlanetBody {
    pub radius: f32,
    pub descendable: bool,
}

pub struct GalaxyPlugin;

impl Plugin for GalaxyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_galaxy);
    }
}

fn spawn_galaxy(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let data = gen::generate(WORLD_SEED);

    // Shared star mesh+material → Bevy batches identical handles (cheap "instancing").
    let star_mesh = meshes.add(Sphere::new(1.0).mesh().ico(1).unwrap());
    let star_mat = materials.add(StandardMaterial {
        base_color: STAR_COLOR,
        emissive: LinearRgba::rgb(2.0, 1.9, 1.5),
        unlit: true,
        ..default()
    });
    for star in &data.stars {
        commands.spawn((
            StarBody,
            WorldPos(star.pos),
            Mesh3d(star_mesh.clone()),
            MeshMaterial3d(star_mat.clone()),
            Transform::default(),
        ));
    }

    for planet in &data.planets {
        commands.spawn((
            PlanetBody { radius: planet.radius as f32, descendable: planet.descendable },
            WorldPos(planet.pos),
            Mesh3d(meshes.add(Sphere::new(planet.radius as f32).mesh().ico(4).unwrap())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: planet_color(planet.kind),
                ..default()
            })),
            Transform::default(),
        ));
    }

    commands.insert_resource(Galaxy(data));
}
```

- [ ] **Step 2: Wire `GalaxyPlugin` into `main.rs`**

Add `.add_plugins(galaxy::GalaxyPlugin)` to the `App` builder in `src/main.rs`.

- [ ] **Step 3: Build and verify there are no errors**

Run: `cargo build`
Expected: compiles.

- [ ] **Step 4: Fly the starfield (manual check)**

Run: `cargo run`
Expected: a field of bright star points surrounds you; ahead (−Z) sit a few colored planet spheres you can fly toward. Stars stay put relative to world as you move (parallax reads as motion). Escape quits.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(p2): spawn galaxy — backdrop starfield + system planets on floating origin"
```

---

## Task 7: Distance-Based LOD / Streaming (P3)

Swap each body's representation by distance so a planet resolves from a far dot (point tier) to a full sphere on approach, and far bodies stay cheap. Pure tier-selection function + a system that applies it.

**Files:**
- Create: `src/streaming/mod.rs`
- Modify: `src/main.rs` (add `mod streaming;`, `.add_plugins(streaming::StreamingPlugin)`)
- Modify: `src/galaxy/mod.rs` (attach `LodBody` to planets in `spawn_galaxy`)

**Interfaces:**
- Consumes: `origin::{WorldPos, FloatingOrigin, FrameSet}`; `galaxy::PlanetBody`.
- Produces (in `streaming`):
  - `pub enum LodTier { Culled, Point, Sphere, Terrain }` — `Clone, Copy, PartialEq, Debug`
  - `pub struct LodThresholds { pub cull: f64, pub sphere: f64, pub terrain: f64 }` — `Resource`, `Default`
  - `pub fn tier_for_distance(surface_dist: f64, t: &LodThresholds) -> LodTier`
  - `pub struct LodBody { pub tier: LodTier }` — `Component`
  - `pub struct StreamingPlugin`
  - system `update_lod` (FrameSet::Stream)

- [ ] **Step 1: Write `src/streaming/mod.rs` with the pure tier function and tests**

```rust
use bevy::prelude::*;
use crate::origin::{FloatingOrigin, FrameSet, WorldPos};
use crate::galaxy::PlanetBody;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum LodTier {
    Culled,
    Point,
    Sphere,
    Terrain,
}

/// Distance thresholds (km, measured to the planet *surface*) at which a body
/// changes representation. Slice defaults; tune for feel.
#[derive(Resource)]
pub struct LodThresholds {
    pub cull: f64,
    pub sphere: f64,
    pub terrain: f64,
}

impl Default for LodThresholds {
    fn default() -> Self {
        Self { cull: 3_000_000.0, sphere: 8_000.0, terrain: 120.0 }
    }
}

/// Choose a representation from the distance to the body's surface.
pub fn tier_for_distance(surface_dist: f64, t: &LodThresholds) -> LodTier {
    if surface_dist > t.cull {
        LodTier::Culled
    } else if surface_dist > t.sphere {
        LodTier::Point
    } else if surface_dist > t.terrain {
        LodTier::Sphere
    } else {
        LodTier::Terrain
    }
}

#[derive(Component)]
pub struct LodBody {
    pub tier: LodTier,
}

pub struct StreamingPlugin;

impl Plugin for StreamingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LodThresholds>()
            .add_systems(Update, update_lod.in_set(FrameSet::Stream));
    }
}

/// For each planet, compute distance from the floating origin (the ship) to its
/// surface, pick a tier, and toggle visibility. Planets are hidden only when
/// fully culled; tier transitions to Sphere/Terrain are consumed by `terrain`.
pub fn update_lod(
    origin: Res<FloatingOrigin>,
    thresholds: Res<LodThresholds>,
    mut q: Query<(&WorldPos, &PlanetBody, &mut LodBody, &mut Visibility)>,
) {
    for (wp, body, mut lod, mut vis) in &mut q {
        let center_dist = (wp.0 - origin.0).length();
        let surface_dist = center_dist - body.radius as f64;
        let tier = tier_for_distance(surface_dist, &thresholds);
        lod.tier = tier;
        *vis = match tier {
            LodTier::Culled => Visibility::Hidden,
            _ => Visibility::Visible,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiers_step_in_by_distance() {
        let t = LodThresholds::default();
        assert_eq!(tier_for_distance(5_000_000.0, &t), LodTier::Culled);
        assert_eq!(tier_for_distance(50_000.0, &t), LodTier::Point);
        assert_eq!(tier_for_distance(1_000.0, &t), LodTier::Sphere);
        assert_eq!(tier_for_distance(10.0, &t), LodTier::Terrain);
    }

    #[test]
    fn boundaries_round_down_to_the_nearer_tier() {
        let t = LodThresholds::default();
        // Exactly at a threshold is NOT "greater than", so it falls to the nearer tier.
        assert_eq!(tier_for_distance(t.terrain, &t), LodTier::Terrain);
        assert_eq!(tier_for_distance(t.sphere, &t), LodTier::Sphere);
    }
}
```

- [ ] **Step 2: Add `mod streaming;` to `main.rs` and run the tests**

Add `mod streaming;` to `src/main.rs`.

Run: `cargo test --lib streaming`
Expected: PASS (2 tests).

- [ ] **Step 3: Attach `LodBody` to planets in `galaxy::spawn_galaxy`**

In `src/galaxy/mod.rs`, add `use crate::streaming::{LodBody, LodTier};` near the other imports, and add `LodBody { tier: LodTier::Point }` to the planet spawn tuple (alongside `PlanetBody { .. }`).

- [ ] **Step 4: Wire `StreamingPlugin` into `main.rs`**

Add `.add_plugins(streaming::StreamingPlugin)` to the `App` builder.

- [ ] **Step 5: Build, then verify approach behavior (manual check)**

Run: `cargo build` then `cargo run`
Expected: distant planets are visible as small spheres; as you fly toward the descend target it grows smoothly to fill the view with no jitter (the floating origin keeps it crisp up close). Escape quits.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(p3): distance-based LOD — tier_for_distance + update_lod, dot→sphere on approach"
```

---

## Task 8: Slide Collision Against the Planet (P3)

Keep the ship outside the planet shell using v1's "slide, don't bounce" clamp, integrated on the floating-origin spine so it works up close without jitter.

**Files:**
- Modify: `src/ship/systems.rs` (extend `ship_move` to clamp against planets)

**Interfaces:**
- Consumes: `ship::physics::clamp_to_planet`; `galaxy::PlanetBody`; `origin::WorldPos`.
- Produces: `ship_move` now also resolves planet collisions (no new public symbols).

- [ ] **Step 1: Write the failing test for clamped movement**

Add to the `tests` module in `src/ship/systems.rs`:

```rust
    #[test]
    fn ship_move_keeps_ship_outside_planet() {
        use crate::galaxy::PlanetBody;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // Planet of radius 300 km at +200 on Z; ship just inside heading inward.
        app.world_mut().spawn((
            PlanetBody { radius: 300.0, descendable: true },
            WorldPos(DVec3::new(0.0, 0.0, 0.0)),
        ));
        let ship = app.world_mut().spawn((
            PlayerShip { radius: 10.0 },
            ShipVelocity(Vec3::new(0.0, 0.0, -50.0)),
            WorldPos(DVec3::new(0.0, 0.0, 250.0)),
        )).id();
        app.update();
        app.world_mut().run_system_once(ship_move).unwrap();
        let wp = app.world().get::<WorldPos>(ship).unwrap();
        let dist = wp.0.length();
        assert!(dist >= 300.0 + 10.0 - 1.0, "ship stays outside planet shell (got {dist})");
    }
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test --lib ship::systems::tests::ship_move_keeps_ship_outside_planet`
Expected: FAIL — the ship penetrates the planet (current `ship_move` ignores planets).

- [ ] **Step 3: Extend `ship_move` to clamp against planets**

Replace the `ship_move` function in `src/ship/systems.rs` with:

```rust
/// Integrate the ship's f64 world position from its f32 velocity, then keep it
/// outside every planet shell (slide, not bounce). Collision math runs in f32
/// relative to each planet, where the offset is small enough to stay precise.
pub fn ship_move(
    time: Res<Time>,
    planets: Query<(&WorldPos, &crate::galaxy::PlanetBody)>,
    mut ship_q: Query<(&mut WorldPos, &mut ShipVelocity, &PlayerShip), Without<crate::galaxy::PlanetBody>>,
) {
    use super::physics::clamp_to_planet;
    const PLANET_MARGIN: f32 = 0.1; // km clearance so the ship can still reach the surface
    let dt = time.delta_secs();
    let Ok((mut wp, mut vel, ship)) = ship_q.get_single_mut() else { return };
    wp.0 += (vel.0 * dt).as_dvec3();

    for (planet_wp, body) in &planets {
        let rel = (wp.0 - planet_wp.0).as_vec3(); // small near the surface
        let (new_rel, new_vel) = clamp_to_planet(
            rel, vel.0, Vec3::ZERO, body.radius + ship.radius, PLANET_MARGIN,
        );
        wp.0 = planet_wp.0 + new_rel.as_dvec3();
        vel.0 = new_vel;
    }
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --lib ship::systems`
Expected: PASS (3 tests).

- [ ] **Step 5: Build and feel the collision (manual check)**

Run: `cargo build` then `cargo run`
Expected: flying into the planet no longer punches through — the ship slides along the surface, keeping momentum tangentially. Escape quits.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(p3): slide-collision — ship stays outside planet shell on the floating origin"
```

---

## Task 9: Seamless Terrain Resolution on Descent (P4)

The signature moment. When the ship enters the descend target's terrain range, swap its smooth sphere for a noise-displaced surface so relief resolves continuously under you — no loading screen, no seam. **Acceptance: it feels good.**

**Files:**
- Create: `src/terrain/mod.rs`
- Modify: `src/main.rs` (add `mod terrain;`, `.add_plugins(terrain::TerrainPlugin)`)

**Interfaces:**
- Consumes: `streaming::{LodBody, LodTier}`; `galaxy::PlanetBody`; `origin::WorldPos`; `palette::planet_color`; `galaxy::Galaxy`.
- Produces (in `terrain`):
  - `pub fn displace_height(unit_dir: Vec3, radius: f32, amplitude: f32) -> f32` — surface radius at a point (pure, noise-based)
  - `pub struct TerrainPlugin`
  - `pub struct TerrainResolved` — `Component` marker (so the swap happens once)
  - system `resolve_terrain` (runs after `FrameSet::Stream`)

- [ ] **Step 1: Write `src/terrain/mod.rs` with the pure displacement function and a test**

```rust
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use noise::{NoiseFn, Perlin};
use crate::galaxy::PlanetBody;
use crate::origin::WorldPos;
use crate::streaming::{LodBody, LodTier};

/// Vertical relief as a fraction of planet radius at full descent.
pub const TERRAIN_AMPLITUDE_FRAC: f32 = 0.06;
/// Subdivisions of the displaced sphere (higher = finer relief, more verts).
pub const TERRAIN_SUBDIVS: u32 = 6;

/// Displaced surface radius along `unit_dir` for a planet of base `radius`.
/// Deterministic (fixed Perlin seed) so the same planet always looks the same.
pub fn displace_height(unit_dir: Vec3, radius: f32, amplitude: f32) -> f32 {
    let perlin = Perlin::new(1);
    let p = unit_dir * 3.0;
    let n = perlin.get([p.x as f64, p.y as f64, p.z as f64]) as f32; // ~[-1,1]
    radius + n * amplitude
}

#[derive(Component)]
pub struct TerrainResolved;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        // Runs after streaming has set tiers this frame.
        app.add_systems(Update, resolve_terrain.after(crate::origin::FrameSet::Stream));
    }
}

/// When a planet reaches the Terrain tier, replace its smooth sphere mesh with a
/// noise-displaced one (once). The floating origin keeps it jitter-free up close.
pub fn resolve_terrain(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    q: Query<(Entity, &PlanetBody, &LodBody), (With<WorldPos>, Without<TerrainResolved>)>,
) {
    for (entity, body, lod) in &q {
        if lod.tier != LodTier::Terrain {
            continue;
        }
        let amplitude = body.radius * TERRAIN_AMPLITUDE_FRAC;
        let mesh = displaced_sphere(body.radius, amplitude, TERRAIN_SUBDIVS);
        commands.entity(entity)
            .insert(Mesh3d(meshes.add(mesh)))
            .insert(TerrainResolved);
    }
}

/// Build a UV-sphere whose vertices are pushed out by `displace_height`.
fn displaced_sphere(radius: f32, amplitude: f32, subdivs: u32) -> Mesh {
    let stacks = subdivs.max(2) * 2;
    let slices = subdivs.max(2) * 2;
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    for i in 0..=stacks {
        let v = i as f32 / stacks as f32;
        let phi = v * std::f32::consts::PI;
        for j in 0..=slices {
            let u = j as f32 / slices as f32;
            let theta = u * std::f32::consts::TAU;
            let dir = Vec3::new(
                phi.sin() * theta.cos(),
                phi.cos(),
                phi.sin() * theta.sin(),
            );
            let r = displace_height(dir, radius, amplitude);
            let pos = dir * r;
            positions.push([pos.x, pos.y, pos.z]);
            normals.push([dir.x, dir.y, dir.z]);
        }
    }
    let row = slices + 1;
    for i in 0..stacks {
        for j in 0..slices {
            let a = i * row + j;
            let b = a + row;
            indices.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displacement_stays_within_amplitude_band() {
        let radius = 350.0;
        let amp = 20.0;
        for dir in [Vec3::X, Vec3::Y, Vec3::Z, Vec3::NEG_X, Vec3::new(1.0, 1.0, 1.0).normalize()] {
            let r = displace_height(dir, radius, amp);
            assert!((r - radius).abs() <= amp + 1e-3,
                "relief within ±amplitude (dir={dir:?}, r={r})");
        }
    }

    #[test]
    fn displacement_is_deterministic() {
        let d = Vec3::new(0.3, -0.7, 0.5).normalize();
        assert_eq!(displace_height(d, 350.0, 20.0), displace_height(d, 350.0, 20.0));
    }

    #[test]
    fn displacement_actually_varies_across_the_surface() {
        let a = displace_height(Vec3::X, 350.0, 20.0);
        let b = displace_height(Vec3::Y, 350.0, 20.0);
        assert!((a - b).abs() > 1e-3, "relief is not flat");
    }
}
```

- [ ] **Step 2: Add `mod terrain;` to `main.rs` and run the tests**

Add `mod terrain;` to `src/main.rs`.

Run: `cargo test --lib terrain`
Expected: PASS (3 tests).

- [ ] **Step 3: Wire `TerrainPlugin` into `main.rs`**

Add `.add_plugins(terrain::TerrainPlugin)` to the `App` builder.

- [ ] **Step 4: Build, then run the full descent (the acceptance test)**

Run: `cargo build` then `cargo run`
Expected: fly to the descend-target planet and drop toward it. As you cross into terrain range the smooth sphere resolves into visible relief under you, continuously, with no loading screen and no jitter — and the slide-collision lets you skim the surface.
**Acceptance:** the descent feels good. If it doesn't, note what's off (camera framing, speed scaling near the surface, relief amount) and tune `FlightTuning` / `LodThresholds` / `TERRAIN_AMPLITUDE_FRAC` before calling the slice done.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(p4): seamless terrain resolution — displaced sphere swaps in on descent"
```

---

## Self-Review Notes

**Spec coverage:**
- §1 acceptance test → Task 9 Step 4 (full descent) is the literal gate.
- §2 floating origin → Task 1 (`WorldPos`/`FloatingOrigin`/`sync_transforms`), consumed by every later task.
- §3 modules → one task (or pair) per module: origin (T1), ship (T2–T3, T8), galaxy (T4, T6), streaming (T7), terrain (T9), palette (T5).
- §4 data flow → `FrameSet` chain (T1) orders Input→Move→Origin→Sync→Camera→Stream; terrain runs after Stream (T9).
- §4 slide-collision → Task 8.
- §5 body budget (~500 stars, ≤8 bodies, 1 descendable) → Task 4 constants (`BACKDROP_STARS`, `SYSTEMS`, planets-per-system 1–3, one `descendable`).
- §6 perf target → 1280×720 window (T0); 60 fps is a manual observation during the run checkpoints.
- §7 testing → deterministic gen tests (T4), LOD-tier tests (T7), floating-origin math test (T1), displacement tests (T9), feel gates (T3, T9).
- §8 phased build order P0–P4 → Tasks 0 / 1–3 / 4–6 / 7–8 / 9. Each task ends in a compiling, committable state.
- §9 non-goals → no battle/on-foot/biome/persistence/audio/atmosphere tasks present.

**Type consistency:** `WorldPos(DVec3)`, `FloatingOrigin(DVec3)`, `FrameSet` set names, `FlightTuning` field names, `PlanetType`/`Planet`/`Star`/`GalaxyData`, `PlanetBody { radius, descendable }`, `LodTier`/`LodThresholds`/`LodBody { tier }`, and `displace_height(unit_dir, radius, amplitude)` are referenced consistently across tasks.

**Placeholder scan:** no TBD/TODO; every code step carries complete code.
```
