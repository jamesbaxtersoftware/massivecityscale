# Spaceship Flight & Collision-Triggered Battles Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the orbit/zoom camera with free 3D spaceship flight (WASD + mouse-steer, hold-E 10× boost) where colliding with a wild monster starts the existing in-world battle.

**Architecture:** A new `src/flight/` module owns a `PlayerShip` entity, flight input/motion, a perspective chase camera, and a sphere-collision battle trigger. The orbit camera and click-picking are deleted. `ZoomLevel` survives but becomes *derived* from camera-to-nearest-planet distance, so the LOD/renderer stack is untouched. Battle framing writes the camera `Transform` directly instead of mutating orbit state.

**Tech Stack:** Rust, Bevy 0.15 (ECS, `Transform`/`Projection`, `ButtonInput`, states), `rand`/`rand_chacha` (existing). Tests are headless using `MinimalPlugins` + `RunSystemOnce`, matching the existing suite.

---

## File Structure

**New files:**
- `src/flight/mod.rs` — `FlightPlugin`, components (`PlayerShip`, `ShipVelocity`), tuning constants, `CollisionCooldown` resource, plugin wiring.
- `src/flight/physics.rs` — pure functions (velocity integration, drag, speed clamp, planet clamp, sphere overlap, battle camera pose) + unit tests.
- `src/flight/ship.rs` — `spawn_ship` (ship mesh + engine-glow accent).
- `src/flight/systems.rs` — ECS systems (`flight_input`, `ship_move`, `planet_clamp`, `chase_camera`, `derive_zoom_from_distance`, `collide_with_monsters`, `nudge_ship_off_monster`).

**Modified files:**
- `src/camera/zoom.rs` — trim to `ZoomLevel` + `to_ortho_scale` + new `zoom_value_from_distance`; delete `OrbitState`, `DoubleClickState`, all input handlers, `sync_camera_transform`, `ray_sphere_intersect`, and their tests.
- `src/camera/mod.rs` — `CameraPlugin` spawns a perspective camera, inserts `ZoomLevel`; remove orbit resources/systems.
- `src/creatures/mod.rs` — remove `pub mod pick;` and its system registration; remove `saved_orbit` from `BattleSession`; register `FlightPlugin` is done in `main.rs` instead.
- `src/creatures/battle.rs` — `setup_battle` frames the camera via `flight::physics::battle_camera_pose`; `teardown_battle` no longer restores orbit.
- `src/renderer/solar.rs` — `orbit_bodies` drops `OrbitState`/`ActivePlanet`; delete `pick_planet`, `ActivePlanet`, `ClickTracker`.
- `src/renderer/mod.rs` — drop `ClickTracker` init, `pick_planet` system, `ActivePlanet` insert.
- `src/main.rs` — add `mod flight;` and `.add_plugins(FlightPlugin)`.

**Deleted file:**
- `src/creatures/pick.rs`.

---

## Milestone 1 — Flight physics (pure functions, TDD)

### Task 1: Module skeleton + velocity integration

**Files:**
- Create: `src/flight/mod.rs`
- Create: `src/flight/physics.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create the module skeleton**

`src/flight/mod.rs`:

```rust
pub mod physics;

use bevy::prelude::*;

// ── Tuning constants (playtest to adjust) ──────────────────────────────────
pub const BASE_MAX_SPEED: f32   = 100.0; // km/s
pub const BOOST_MULT: f32       = 10.0;
pub const THRUST_ACCEL: f32     = 80.0;  // km/s^2
pub const STRAFE_ACCEL: f32     = 50.0;  // km/s^2
pub const DRAG_HALF_LIFE: f32   = 0.7;   // s
pub const TURN_RATE: f32        = 1.2;   // rad/s at full cursor offset
pub const ROLL_RATE: f32        = 1.5;   // rad/s while strafing
pub const SHIP_RADIUS: f32      = 10.0;  // km
pub const PLANET_MARGIN: f32    = 12.0;  // km clearance kept outside a planet
pub const CAM_BACK: f32         = 60.0;  // km behind ship
pub const CAM_UP: f32           = 25.0;  // km above ship
pub const CAM_LOOKAHEAD: f32    = 80.0;  // km ahead of ship for look target
pub const CAM_SMOOTH_HALF_LIFE: f32 = 0.15; // s
pub const COLLISION_COOLDOWN: f32 = 2.0; // s

// ── Components ─────────────────────────────────────────────────────────────
#[derive(Component)]
pub struct PlayerShip {
    pub radius: f32,
}

#[derive(Component, Default)]
pub struct ShipVelocity(pub Vec3);
```

- [ ] **Step 2: Write the failing test for velocity integration**

Add to `src/flight/physics.rs`:

```rust
use bevy::prelude::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thrust_increases_speed_along_direction() {
        // Starting at rest, one second of forward thrust, no drag clamp issues.
        let v0 = Vec3::ZERO;
        let thrust = Vec3::new(0.0, 0.0, -1.0) * 80.0; // accel km/s^2
        let v1 = integrate_velocity(v0, thrust, 1.0e9, 1.0e9, 0.5);
        assert!(v1.z < 0.0, "should gain velocity in -Z");
        assert!((v1.length() - 40.0).abs() < 1.0, "≈ accel * dt with negligible drag");
    }
}
```

Run: `cargo test --lib flight::physics::tests::thrust_increases_speed_along_direction`
Expected: FAIL — `integrate_velocity` not found.

- [ ] **Step 3: Implement `integrate_velocity`**

Add above the `tests` module in `src/flight/physics.rs`:

```rust
/// Integrate one physics step: apply thrust acceleration, exponential drag,
/// then clamp to `max_speed`. `drag_half_life` is the time for an un-thrusted
/// velocity to halve. Large half-life / max_speed effectively disable each.
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
```

- [ ] **Step 4: Run the test**

Run: `cargo test --lib flight::physics::tests::thrust_increases_speed_along_direction`
Expected: PASS.

- [ ] **Step 5: Wire the module into the crate**

In `src/main.rs`, add `mod flight;` after `mod creatures;` (line 6). Do NOT add the plugin yet (systems don't exist).

Run: `cargo build`
Expected: builds (unused-warnings are fine).

- [ ] **Step 6: Commit**

```bash
git add src/flight/mod.rs src/flight/physics.rs src/main.rs
git commit -m "feat(flight): module skeleton + velocity integration"
```

---

### Task 2: Drag and speed clamp behavior

**Files:**
- Modify: `src/flight/physics.rs`

- [ ] **Step 1: Write failing tests**

Add inside the `tests` module in `src/flight/physics.rs`:

```rust
    #[test]
    fn drag_halves_velocity_over_half_life() {
        let v0 = Vec3::new(0.0, 0.0, -100.0);
        let v1 = integrate_velocity(v0, Vec3::ZERO, 0.7, 1.0e9, 0.7);
        assert!((v1.length() - 50.0).abs() < 1.0, "one half-life halves speed");
    }

    #[test]
    fn speed_is_clamped_to_max() {
        let v0 = Vec3::new(0.0, 0.0, -1000.0);
        let v1 = integrate_velocity(v0, Vec3::ZERO, 1.0e9, 100.0, 0.016);
        assert!(v1.length() <= 100.0 + 1e-3, "clamped to max speed");
    }

    #[test]
    fn boost_raises_the_clamp() {
        let v0 = Vec3::new(0.0, 0.0, -1000.0);
        let base = integrate_velocity(v0, Vec3::ZERO, 1.0e9, 100.0, 0.016);
        let boost = integrate_velocity(v0, Vec3::ZERO, 1.0e9, 100.0 * 10.0, 0.016);
        assert!(boost.length() > base.length(), "boost allows higher speed");
    }
```

Run: `cargo test --lib flight::physics::tests`
Expected: the three new tests PASS immediately (already covered by `integrate_velocity`). If any fail, fix `integrate_velocity` before continuing.

- [ ] **Step 2: Commit**

```bash
git add src/flight/physics.rs
git commit -m "test(flight): drag, speed clamp, and boost behavior"
```

---

### Task 3: Planet clamp (keep the ship outside planets)

**Files:**
- Modify: `src/flight/physics.rs`

- [ ] **Step 1: Write the failing test**

Add inside the `tests` module:

```rust
    #[test]
    fn clamp_pushes_ship_outside_planet_and_zeros_inward_velocity() {
        let center = Vec3::ZERO;
        let radius = 350.0;
        let margin = 12.0;
        // Ship 100 km deep inside the planet, moving further inward.
        let pos = Vec3::new(0.0, 0.0, 250.0);
        let vel = Vec3::new(0.0, 0.0, -100.0); // heading toward center
        let (new_pos, new_vel) = clamp_to_planet(pos, vel, center, radius, margin);
        let surface = radius + margin;
        assert!((new_pos.length() - surface).abs() < 1e-2, "pushed to surface+margin");
        // Inward (toward center) component removed; here outward is +Z.
        assert!(new_vel.z >= -1e-3, "inward velocity removed");
    }

    #[test]
    fn clamp_leaves_distant_ship_untouched() {
        let pos = Vec3::new(0.0, 0.0, 5000.0);
        let vel = Vec3::new(1.0, 2.0, 3.0);
        let (p, v) = clamp_to_planet(pos, vel, Vec3::ZERO, 350.0, 12.0);
        assert_eq!(p, pos);
        assert_eq!(v, vel);
    }
```

Run: `cargo test --lib flight::physics::tests::clamp_pushes_ship_outside_planet_and_zeros_inward_velocity`
Expected: FAIL — `clamp_to_planet` not found.

- [ ] **Step 2: Implement `clamp_to_planet`**

Add above the `tests` module:

```rust
/// If `pos` is inside `radius + margin` of a planet at `center`, project it
/// back onto that shell and remove the inward component of `vel` (slide, not
/// bounce). Returns the corrected (pos, vel). Outside the shell, returns the
/// inputs unchanged.
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
    let inward = vel.dot(outward).min(0.0); // negative when heading inward
    let new_vel = vel - outward * inward;    // subtract inward component
    (new_pos, new_vel)
}
```

- [ ] **Step 3: Run the tests**

Run: `cargo test --lib flight::physics::tests`
Expected: PASS (all clamp tests green).

- [ ] **Step 4: Commit**

```bash
git add src/flight/physics.rs
git commit -m "feat(flight): planet clamp keeps ship outside planets"
```

---

## Milestone 2 — Derived zoom (pure, TDD)

### Task 4: `zoom_value_from_distance`

**Files:**
- Modify: `src/camera/zoom.rs`

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `src/camera/zoom.rs`:

```rust
    #[test]
    fn zoom_value_round_trips_through_distance() {
        // Pick a mid value, find the distance that maps to its ortho scale,
        // then recover the value.
        let v = 0.5_f32;
        let scale = ZoomLevel { value: v }.to_ortho_scale();
        let dist = scale / ZOOM_DIST_K;
        let recovered = zoom_value_from_distance(dist);
        assert!((recovered - v).abs() < 1e-3, "round trip: {recovered} vs {v}");
    }

    #[test]
    fn zoom_value_clamps_at_both_ends() {
        assert_eq!(zoom_value_from_distance(0.0), 0.0);
        assert_eq!(zoom_value_from_distance(1.0e9), 1.0);
    }

    #[test]
    fn close_distance_below_solar_band_far_above() {
        // Solar LOD turns on at ortho scale 3.0.
        let close = zoom_value_from_distance(50.0);
        let far = zoom_value_from_distance(8000.0);
        assert!(ZoomLevel { value: close }.to_ortho_scale() < 3.0, "close: street/buildings");
        assert!(ZoomLevel { value: far }.to_ortho_scale() >= 3.0, "far: solar visible");
    }
```

Run: `cargo test --lib camera::zoom::tests::zoom_value_round_trips_through_distance`
Expected: FAIL — `ZOOM_DIST_K` / `zoom_value_from_distance` not found.

- [ ] **Step 2: Implement the function and constant**

Add near the other `pub const` declarations at the top of `src/camera/zoom.rs` (after line 10):

```rust
/// Maps camera-to-nearest-planet-surface distance (km) to an effective ortho
/// scale before inverting `to_ortho_scale`. Tuned so close flight lands in the
/// buildings band and interplanetary distance crosses the solar band (3.0).
pub const ZOOM_DIST_K: f32 = 0.0015;
```

Add this free function after the `ZoomLevel` impl block (after line 35):

```rust
/// Derive a [0,1] `ZoomLevel::value` from the camera's distance to the nearest
/// planet surface. Inverts `to_ortho_scale` so the existing LOD bands work
/// unchanged. Clamped to [0,1].
pub fn zoom_value_from_distance(dist_to_surface: f32) -> f32 {
    let scale = (dist_to_surface.max(0.0) * ZOOM_DIST_K).max(1e-6);
    ((scale / 0.001).log10() / ZOOM_EXPONENT).clamp(ZOOM_MIN, ZOOM_MAX)
}
```

- [ ] **Step 3: Run the tests**

Run: `cargo test --lib camera::zoom::tests`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/camera/zoom.rs
git commit -m "feat(camera): derive zoom value from camera-planet distance"
```

---

## Milestone 3 — Collision + battle framing (pure, TDD)

### Task 5: Sphere overlap and battle camera pose

**Files:**
- Modify: `src/flight/physics.rs`

- [ ] **Step 1: Write the failing tests**

Add inside the `tests` module in `src/flight/physics.rs`:

```rust
    #[test]
    fn spheres_overlap_detects_contact() {
        assert!(spheres_overlap(Vec3::ZERO, 10.0, Vec3::new(15.0, 0.0, 0.0), 10.0));
        assert!(!spheres_overlap(Vec3::ZERO, 10.0, Vec3::new(25.0, 0.0, 0.0), 10.0));
    }

    #[test]
    fn battle_camera_pose_faces_the_monster_at_distance() {
        let wild = Vec3::new(100.0, 0.0, 100.0);
        let scale = 40.0;
        let pose = battle_camera_pose(wild, scale);
        let d = (pose.camera - wild).length();
        assert!((d - scale * 12.0).abs() < 1.0, "camera at framing distance");
        assert_eq!(pose.look_at, wild, "camera looks at the monster");
        // Player creature is placed near the monster, not on top of it.
        let pd = (pose.player - wild).length();
        assert!(pd > scale && pd < scale * 8.0, "player framed beside the monster");
    }
```

Run: `cargo test --lib flight::physics::tests::battle_camera_pose_faces_the_monster_at_distance`
Expected: FAIL — `spheres_overlap` / `battle_camera_pose` / `BattlePose` not found.

- [ ] **Step 2: Implement overlap, pose struct, and pose function**

Add above the `tests` module in `src/flight/physics.rs`:

```rust
/// True when two spheres touch or intersect.
pub fn spheres_overlap(a: Vec3, ra: f32, b: Vec3, rb: f32) -> bool {
    let r = ra + rb;
    a.distance_squared(b) <= r * r
}

/// Camera + creature placement for an in-world battle framed on a wild monster.
pub struct BattlePose {
    pub camera: Vec3,
    pub look_at: Vec3,
    pub player: Vec3,
}

/// Frame the camera on a wild monster at a fixed distance/elevation and place
/// the player's creature beside it. Mirrors the previous orbit-based framing
/// (distance = scale*12, elevation 0.2) but as a pure function.
pub fn battle_camera_pose(wild_pos: Vec3, creature_scale: f32) -> BattlePose {
    let distance = creature_scale * 12.0;
    let elevation = 0.2_f32;
    // Fixed azimuth (0) offset in the XZ plane, raised by elevation.
    let dir = Vec3::new(0.0, elevation.sin(), elevation.cos()).normalize();
    let camera = wild_pos + dir * distance;
    let fwd = (wild_pos - camera).normalize();
    let right = fwd.cross(Vec3::Y).normalize();
    let player = wild_pos - fwd * creature_scale * 4.0 + right * creature_scale * 2.0;
    BattlePose { camera, look_at: wild_pos, player }
}
```

- [ ] **Step 3: Run the tests**

Run: `cargo test --lib flight::physics::tests`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/flight/physics.rs
git commit -m "feat(flight): sphere overlap + battle camera pose helpers"
```

---

## Milestone 4 — The swap (integration)

This milestone rips out the orbit camera and wires in flight. `OrbitState` is load-bearing across several files, so the removal lands as one coherent change that ends compiling with all tests green. Work through the steps in order; do not run `cargo build` until Step 9 (intermediate states will not compile).

### Task 6: Spawn the ship and its visuals

**Files:**
- Create: `src/flight/ship.rs`
- Modify: `src/flight/mod.rs`

- [ ] **Step 1: Write `spawn_ship`**

`src/flight/ship.rs`:

```rust
use bevy::prelude::*;
use super::{PlayerShip, ShipVelocity, SHIP_RADIUS};

/// Spawn the player ship near the home planet if one does not already exist.
/// A dart body (cone) with an emissive engine-glow accent behind it.
pub fn spawn_ship(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    existing: Query<(), With<PlayerShip>>,
) {
    if !existing.is_empty() {
        return; // already spawned (re-entering Exploring after a battle)
    }

    // Home planet starts near world origin; place the ship just above its
    // north pole, facing tangentially (-Z).
    let start = Vec3::new(0.0, 80.0, 600.0);

    let body_mesh = meshes.add(Cone { radius: SHIP_RADIUS * 0.6, height: SHIP_RADIUS * 2.0 }.mesh());
    let body_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.75, 0.78, 0.85),
        perceptual_roughness: 0.4,
        metallic: 0.6,
        ..default()
    });
    let glow_mesh = meshes.add(Sphere::new(SHIP_RADIUS * 0.4).mesh().uv(8, 6));
    let glow_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.7, 1.0),
        emissive: LinearRgba::rgb(0.2, 0.6, 1.0),
        unlit: true,
        ..default()
    });

    commands.spawn((
        PlayerShip { radius: SHIP_RADIUS },
        ShipVelocity::default(),
        Transform::from_translation(start).looking_to(Vec3::NEG_Z, Vec3::Y),
        GlobalTransform::default(),
        Visibility::Visible,
    )).with_children(|ship| {
        // Cone's +Y is its tip; rotate so the tip points along -Z (forward).
        ship.spawn((
            Mesh3d(body_mesh),
            MeshMaterial3d(body_mat),
            Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ));
        ship.spawn((
            Mesh3d(glow_mesh),
            MeshMaterial3d(glow_mat),
            Transform::from_xyz(0.0, 0.0, SHIP_RADIUS * 1.1), // behind the tip
        ));
    });
}
```

- [ ] **Step 2: Register the submodule**

In `src/flight/mod.rs`, add after `pub mod physics;`:

```rust
pub mod ship;
pub mod systems;
```

(`systems` is created in Task 7; declaring it now keeps the next edits compiling once that file exists. If building before Task 7, comment out `pub mod systems;` — but per the milestone note, do not build until Step 9.)

Defer commit until the milestone compiles (Step 9).

---

### Task 7: Flight systems (input, motion, chase camera, derived zoom, collision)

**Files:**
- Create: `src/flight/systems.rs`

- [ ] **Step 1: Write the systems file**

`src/flight/systems.rs`:

```rust
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use super::physics::{integrate_velocity, clamp_to_planet, spheres_overlap};
use super::{
    PlayerShip, ShipVelocity, CollisionCooldown,
    BASE_MAX_SPEED, BOOST_MULT, THRUST_ACCEL, STRAFE_ACCEL, DRAG_HALF_LIFE,
    TURN_RATE, ROLL_RATE, PLANET_MARGIN,
    CAM_BACK, CAM_UP, CAM_LOOKAHEAD, CAM_SMOOTH_HALF_LIFE,
};
use crate::camera::zoom::{ZoomLevel, zoom_value_from_distance};
use crate::renderer::solar::CelestialBody;
use crate::creatures::{WildMonster, ClickSphere, GameState, BattleSession};

/// Mouse steers (cursor offset from screen centre), W/S thrust, A/D strafe,
/// hold E boosts. Updates ship facing and `ShipVelocity`.
pub fn flight_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut ship_q: Query<(&mut Transform, &mut ShipVelocity), With<PlayerShip>>,
) {
    let dt = time.delta_secs();
    let Ok((mut tf, mut vel)) = ship_q.get_single_mut() else { return };

    // Mouse steering: normalized cursor offset from centre in [-1, 1].
    if let Ok(window) = windows.get_single() {
        if let Some(cursor) = window.cursor_position() {
            let half = Vec2::new(window.width(), window.height()) * 0.5;
            let off = (cursor - half) / half; // y down in screen space
            let yaw = -off.x * TURN_RATE * dt;
            let pitch = -off.y * TURN_RATE * dt; // cursor down → pitch down
            tf.rotate_local_y(yaw);
            tf.rotate_local_x(pitch);
        }
    }

    let boosting = keys.pressed(KeyCode::KeyE);
    let max_speed = if boosting { BASE_MAX_SPEED * BOOST_MULT } else { BASE_MAX_SPEED };
    let thrust_mag = if boosting { THRUST_ACCEL * BOOST_MULT } else { THRUST_ACCEL };

    let fwd = tf.forward().as_vec3();
    let right = tf.right().as_vec3();
    let mut accel = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) { accel += fwd * thrust_mag; }
    if keys.pressed(KeyCode::KeyS) { accel -= fwd * thrust_mag; }
    if keys.pressed(KeyCode::KeyD) { accel += right * STRAFE_ACCEL; tf.rotate_local_z(-ROLL_RATE * dt); }
    if keys.pressed(KeyCode::KeyA) { accel -= right * STRAFE_ACCEL; tf.rotate_local_z(ROLL_RATE * dt); }

    vel.0 = integrate_velocity(vel.0, accel, DRAG_HALF_LIFE, max_speed, dt);
}

/// Integrate position from velocity, then keep the ship outside every planet.
pub fn ship_move(
    time: Res<Time>,
    planets: Query<(&GlobalTransform, &CelestialBody)>,
    mut ship_q: Query<(&mut Transform, &mut ShipVelocity, &PlayerShip)>,
) {
    let dt = time.delta_secs();
    let Ok((mut tf, mut vel, ship)) = ship_q.get_single_mut() else { return };
    tf.translation += vel.0 * dt;
    for (gt, body) in &planets {
        let (p, v) = clamp_to_planet(
            tf.translation, vel.0, gt.translation(),
            body.radius + ship.radius, PLANET_MARGIN,
        );
        tf.translation = p;
        vel.0 = v;
    }
}

/// Chase camera: smoothed follow behind and above the ship.
pub fn chase_camera(
    time: Res<Time>,
    ship_q: Query<&Transform, (With<PlayerShip>, Without<Camera3d>)>,
    mut cam_q: Query<&mut Transform, With<Camera3d>>,
) {
    let Ok(ship) = ship_q.get_single() else { return };
    let Ok(mut cam) = cam_q.get_single_mut() else { return };

    let fwd = ship.forward().as_vec3();
    let up = ship.up().as_vec3();
    let target = ship.translation - fwd * CAM_BACK + up * CAM_UP;
    let look = ship.translation + fwd * CAM_LOOKAHEAD;

    let t = 1.0 - 0.5_f32.powf(time.delta_secs() / CAM_SMOOTH_HALF_LIFE.max(1e-6));
    cam.translation = cam.translation.lerp(target, t);
    cam.look_at(look, Vec3::Y);
}

/// Derive `ZoomLevel` from the camera's distance to the nearest planet surface.
/// Runs in all states so the LOD system stays driven.
pub fn derive_zoom_from_distance(
    cam_q: Query<&Transform, With<Camera3d>>,
    planets: Query<(&GlobalTransform, &CelestialBody)>,
    mut zoom: ResMut<ZoomLevel>,
) {
    let Ok(cam) = cam_q.get_single() else { return };
    let mut nearest = f32::MAX;
    for (gt, body) in &planets {
        let d = (cam.translation - gt.translation()).length() - body.radius;
        nearest = nearest.min(d.max(0.0));
    }
    if nearest == f32::MAX { zoom.value = 1.0; return; } // no planets → solar view
    zoom.value = zoom_value_from_distance(nearest);
}

/// Start a battle when the ship overlaps a wild monster (unless cooling down).
pub fn collide_with_monsters(
    time: Res<Time>,
    mut cooldown: ResMut<CollisionCooldown>,
    mut ship_q: Query<(&Transform, &mut ShipVelocity, &PlayerShip)>,
    monsters: Query<(Entity, &GlobalTransform, &ClickSphere), With<WildMonster>>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if cooldown.0.tick(time.delta()).remaining_secs() > 0.0 && !cooldown.0.finished() {
        return;
    }
    let Ok((tf, mut vel, ship)) = ship_q.get_single_mut() else { return };
    for (entity, gt, cs) in &monsters {
        if spheres_overlap(tf.translation, ship.radius, gt.translation(), cs.radius) {
            vel.0 = Vec3::ZERO;
            commands.insert_resource(BattleSession::new(entity));
            next_state.set(GameState::Battle);
            return;
        }
    }
}

/// On leaving a battle, push the ship just outside the (surviving) monster's
/// collision sphere and restart the cooldown so it does not re-trigger.
pub fn nudge_ship_off_monster(
    session: Res<BattleSession>,
    mut cooldown: ResMut<CollisionCooldown>,
    monsters: Query<(&GlobalTransform, &ClickSphere), With<WildMonster>>,
    mut ship_q: Query<(&mut Transform, &PlayerShip)>,
) {
    cooldown.0.reset();
    let Ok((mut tf, ship)) = ship_q.get_single_mut() else { return };
    if let Ok((gt, cs)) = monsters.get(session.wild_entity) {
        let away = (tf.translation - gt.translation()).normalize_or_zero();
        let away = if away == Vec3::ZERO { Vec3::Z } else { away };
        tf.translation = gt.translation() + away * (cs.radius + ship.radius + 5.0);
    }
}
```

Defer commit until the milestone compiles (Step 9 of Task 8).

---

### Task 8: Rewire plugins, delete orbit, wire flight

**Files:**
- Modify: `src/flight/mod.rs`
- Modify: `src/camera/zoom.rs`
- Modify: `src/camera/mod.rs`
- Modify: `src/renderer/solar.rs`
- Modify: `src/renderer/mod.rs`
- Modify: `src/creatures/mod.rs`
- Modify: `src/creatures/battle.rs`
- Modify: `src/main.rs`
- Delete: `src/creatures/pick.rs`

- [ ] **Step 1: Finish `FlightPlugin` and the cooldown resource**

In `src/flight/mod.rs`, add the resource after the components:

```rust
use bevy::time::{Timer, TimerMode};

/// Suppresses collision re-triggers for a short time after a battle ends.
#[derive(Resource)]
pub struct CollisionCooldown(pub Timer);

impl Default for CollisionCooldown {
    fn default() -> Self {
        let mut t = Timer::from_seconds(COLLISION_COOLDOWN, TimerMode::Once);
        t.tick(std::time::Duration::from_secs_f32(COLLISION_COOLDOWN)); // start finished
        Self(t)
    }
}
```

Append the plugin to `src/flight/mod.rs`:

```rust
use bevy::ecs::schedule::IntoSystemConfigs;
use crate::creatures::GameState;

pub struct FlightPlugin;

impl Plugin for FlightPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CollisionCooldown>()
           .add_systems(OnEnter(GameState::Exploring), ship::spawn_ship)
           .add_systems(OnExit(GameState::Battle), systems::nudge_ship_off_monster)
           .add_systems(Update, systems::derive_zoom_from_distance)
           .add_systems(Update, (
               systems::flight_input,
               systems::ship_move,
               systems::chase_camera,
               systems::collide_with_monsters,
           ).chain().run_if(in_state(GameState::Exploring)));
    }
}
```

- [ ] **Step 2: Trim `src/camera/zoom.rs`**

Delete these items entirely (and any tests that reference them): `OrbitState` struct + impl + `Default`, `DoubleClickState`, `handle_scroll`, `handle_double_click_zoom`, `handle_keyboard_orbit`, `handle_orbit`, `sync_camera_transform`, `ray_sphere_intersect`, and the constants used only by them (`KEYBOARD_ORBIT_FACTOR`, `ORBIT_PIVOT_Y`, `SPHERE_RADIUS_KM`, `MAX_ELEVATION`).

Delete these tests: `orbit_state_camera_pos_round_trips`, `elevation_clamped_at_max`. Keep: `zoom_clamps_to_range`, `ortho_scale_increases_with_zoom`, `street_scale_shows_buildings`, `solar_scale_sees_solar_system`, `default_zoom_is_mid_range`, and the three added in Task 4.

Remove the now-unused imports at the top (`MouseWheel`, `MouseMotion`, `PrimaryWindow`). The file should keep only: `use bevy::prelude::*;`, the zoom constants, `ZOOM_DIST_K`, `ZoomLevel` + impl + `Default`, `zoom_value_from_distance`, and the surviving tests.

- [ ] **Step 3: Rewrite `src/camera/mod.rs`**

Replace the whole file with:

```rust
pub mod zoom;

use bevy::prelude::*;
pub use zoom::ZoomLevel;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ZoomLevel::default())
           .add_systems(Startup, spawn_camera);
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 60.0_f32.to_radians(),
            near: 1.0,
            far: 60_000.0,
            ..default()
        }),
        Transform::from_xyz(0.0, 120.0, 700.0).looking_at(Vec3::new(0.0, 0.0, 600.0), Vec3::Y),
    ));
}
```

- [ ] **Step 4: Simplify `orbit_bodies` and delete picking in `src/renderer/solar.rs`**

Replace `orbit_bodies` (lines 92–106) with:

```rust
pub fn orbit_bodies(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut OrbitalBody)>,
) {
    for (mut transform, mut orbit) in &mut query {
        orbit.angle += orbit.speed * time.delta_secs();
        transform.translation = orbit.position();
    }
}
```

Delete: the `ActivePlanet` struct (lines ~41–45), the `ClickTracker` struct + impl (lines ~48–56), and the entire `pick_planet` function (lines ~296–352). Remove now-unused imports at the top: `bevy::window::PrimaryWindow`, `bevy::input::mouse::MouseMotion`, and `use crate::camera::zoom::OrbitState;`. Keep `HomePlanet`, `StarSphere`, `CelestialBody`, `OrbitalBody`, `spawn_solar_system`, `spawn_starfield`, `center_starfield_on_camera`.

- [ ] **Step 5: Update `src/renderer/mod.rs`**

In `spawn_planet_root`, remove the line `commands.insert_resource(solar::ActivePlanet { entity: id });`. In `RendererPlugin::build`, remove `.init_resource::<solar::ClickTracker>()` and the `solar::pick_planet` system line. The remaining `Update` systems are `orbit_bodies` (still gated `not(Battle)`) and `center_starfield_on_camera`.

- [ ] **Step 6: Update `src/creatures/mod.rs`**

Remove `pub mod pick;` (line 2). Remove the `saved_orbit` field from `BattleSession` (line 117) and from `BattleSession::new` (line 128). Remove the system registration line `.add_systems(Update, pick::pick_wild_monster.run_if(in_state(GameState::Exploring)))` (line 261).

- [ ] **Step 7: Rewrite battle framing in `src/creatures/battle.rs`**

Replace the `use crate::camera::zoom::OrbitState;` import with `use crate::flight::physics::battle_camera_pose;`.

Replace `setup_battle`'s signature and framing block. New `setup_battle`:

```rust
pub fn setup_battle(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut session: ResMut<BattleSession>,
    player: Res<PlayerCreature>,
    mut cam_q: Query<&mut Transform, With<Camera3d>>,
    wild_q: Query<(&GlobalTransform, &Creature), With<WildMonster>>,
) {
    let Ok((wild_gt, wild)) = wild_q.get(session.wild_entity) else { return };
    let wild_pos = wild_gt.translation();

    let pose = battle_camera_pose(wild_pos, BATTLE_CREATURE_SCALE);
    if let Ok(mut cam) = cam_q.get_single_mut() {
        cam.translation = pose.camera;
        cam.look_at(pose.look_at, Vec3::Y);
    }

    session.wild_hp = wild.hp;
    session.player_hp = player.0.max_hp;
    session.turn = Turn::Player;

    let player_pos = pose.player;

    let vis = build_visual(player.0.creature_type(), BATTLE_CREATURE_SCALE, &mut meshes, &mut materials);
    let player_root = commands.spawn((
        Transform::from_translation(player_pos),
        GlobalTransform::default(),
        Visibility::Visible,
        BattleSide::Player,
    )).id();
    spawn_creature_visual(&mut commands, player_root, Transform::IDENTITY, &vis);
    session.view_entities.push(player_root);

    let pbar = spawn_hp_bar(&mut commands, &mut meshes, &mut materials,
        player_pos + Vec3::Y * BATTLE_CREATURE_SCALE * 2.0, BattleSide::Player);
    let wbar = spawn_hp_bar(&mut commands, &mut meshes, &mut materials,
        wild_pos + Vec3::Y * BATTLE_CREATURE_SCALE * 2.0, BattleSide::Wild);
    session.view_entities.push(pbar);
    session.view_entities.push(wbar);
}
```

Replace `teardown_battle` with (no orbit restore; chase camera resumes next frame):

```rust
pub fn teardown_battle(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
) {
    for e in session.view_entities.drain(..) {
        commands.entity(e).despawn_recursive();
    }
}
```

- [ ] **Step 8: Delete `pick.rs` and register the plugin**

```bash
git rm src/creatures/pick.rs
```

In `src/main.rs`: add `use flight::FlightPlugin;` near the other `use` lines, and add `.add_plugins(FlightPlugin)` after `.add_plugins(CreaturesPlugin)`.

- [ ] **Step 9: Build and run the full test suite**

Run: `cargo build`
Expected: compiles. Fix any leftover references to deleted items (`OrbitState`, `ActivePlanet`, `ClickTracker`, `saved_orbit`, `pick::`) until it does.

Run: `cargo test`
Expected: all tests pass (the orbit-state tests are gone; everything else green).

- [ ] **Step 10: Commit**

```bash
git add -A
git commit -m "feat(flight): replace orbit camera with spaceship flight + collision battles"
```

---

## Milestone 5 — Collision trigger & re-collision guard (integration tests)

### Task 9: Test the collision trigger and cooldown

**Files:**
- Modify: `src/flight/systems.rs` (add a `tests` module)

- [ ] **Step 1: Write the failing integration test**

Add to the bottom of `src/flight/systems.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use crate::creatures::{Creature, Species};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
           .add_plugins(bevy::state::app::StatesPlugin)
           .init_state::<GameState>()
           .init_resource::<crate::flight::CollisionCooldown>();
        app
    }

    #[test]
    fn overlap_starts_a_battle() {
        let mut app = test_app();
        // Finish the cooldown so collisions are live.
        app.world_mut().resource_mut::<crate::flight::CollisionCooldown>().0.reset();
        app.world_mut().resource_mut::<crate::flight::CollisionCooldown>().0
            .tick(std::time::Duration::from_secs(5));

        let ship = app.world_mut().spawn((
            PlayerShip { radius: 10.0 },
            ShipVelocity(Vec3::new(0.0, 0.0, -50.0)),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::default(),
        )).id();
        let _ = ship;

        app.world_mut().spawn((
            WildMonster,
            ClickSphere { radius: 20.0 },
            Transform::from_xyz(15.0, 0.0, 0.0),
            GlobalTransform::from_xyz(15.0, 0.0, 0.0),
            Creature::new(Species::Emberling, 3),
        ));

        app.world_mut().run_system_once(collide_with_monsters).unwrap();

        assert!(app.world().get_resource::<BattleSession>().is_some(),
            "battle session inserted on overlap");
        match app.world().resource::<NextState<GameState>>() {
            NextState::Pending(s) => assert_eq!(*s, GameState::Battle),
            _ => panic!("expected transition to Battle"),
        }
    }

    #[test]
    fn no_battle_when_separated() {
        let mut app = test_app();
        app.world_mut().resource_mut::<crate::flight::CollisionCooldown>().0.reset();
        app.world_mut().resource_mut::<crate::flight::CollisionCooldown>().0
            .tick(std::time::Duration::from_secs(5));

        app.world_mut().spawn((
            PlayerShip { radius: 10.0 },
            ShipVelocity::default(),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::default(),
        ));
        app.world_mut().spawn((
            WildMonster,
            ClickSphere { radius: 10.0 },
            Transform::from_xyz(500.0, 0.0, 0.0),
            GlobalTransform::from_xyz(500.0, 0.0, 0.0),
            Creature::new(Species::Emberling, 3),
        ));

        app.world_mut().run_system_once(collide_with_monsters).unwrap();
        assert!(app.world().get_resource::<BattleSession>().is_none());
    }
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test --lib flight::systems::tests`
Expected: PASS. If `collide_with_monsters`'s cooldown check blocks the first test, confirm the cooldown was ticked to finished in the test setup; the system should proceed when `cooldown.0.finished()`.

> **Note on the cooldown guard:** the early-return in `collide_with_monsters` must allow collisions once the timer is finished. Verify the condition reads as "return early only while the timer is still running." If the test reveals the boolean logic is inverted, simplify the guard to:
> ```rust
> cooldown.0.tick(time.delta());
> if !cooldown.0.finished() { return; }
> ```

- [ ] **Step 3: Commit**

```bash
git add src/flight/systems.rs
git commit -m "test(flight): collision trigger starts battle, separation does not"
```

---

## Milestone 6 — Manual playtest

### Task 10: Playtest and tune

- [ ] **Step 1: Run the game**

Run: `cargo run`

- [ ] **Step 2: Verify the experience**

Check each, tuning the constants at the top of `src/flight/mod.rs` as needed:
- Starter select still works; choosing one drops you into flight.
- WASD thrusts/strafes; the ship turns toward the mouse; releasing thrust coasts to a stop.
- Holding E noticeably accelerates and raises top speed (~10×).
- Flying toward the home planet streams in terrain/buildings; boosting away fades up the solar system and stars (LOD via derived zoom).
- The ship cannot pass through planets.
- Flying into a wild monster starts the battle; the camera frames both creatures with HP bars; keys 1/2/F work.
- Winning despawns the monster and returns to flight; losing/fleeing returns to flight without instantly re-triggering the same battle (cooldown + nudge).

- [ ] **Step 3: Commit any tuning changes**

```bash
git add src/flight/mod.rs
git commit -m "tune(flight): playtest adjustments"
```

---

## Self-Review Notes

- **Spec coverage:** flight model (Tasks 1–3, 6, 7), controls incl. E-boost (Task 7), replace camera + derived zoom (Tasks 4, 8), collision trigger (Tasks 5, 7, 9), re-collision guard cooldown + nudge (Tasks 7, 8), in-world battle reframing without `OrbitState` (Tasks 5, 8), deletions (Task 8). All spec sections map to tasks.
- **Type consistency:** `integrate_velocity`, `clamp_to_planet`, `spheres_overlap`, `battle_camera_pose`/`BattlePose`, `zoom_value_from_distance`/`ZOOM_DIST_K`, `PlayerShip`/`ShipVelocity`/`CollisionCooldown` are used with identical names across tasks.
- **Known risk:** Bevy 0.15 `Timer`/`finished()` semantics in the cooldown guard — Task 9 Step 2 calls this out with a fallback simplification.
