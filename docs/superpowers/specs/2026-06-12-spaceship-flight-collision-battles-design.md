# Spaceship Flight & Collision-Triggered Battles — Design

**Date:** 2026-06-12
**Status:** Approved for planning

## Summary

Replace the orbit/zoom camera with free 3D spaceship flight through the solar
system. The player pilots a visible ship with a perspective chase camera;
holding E boosts thrust 10×. Battles trigger by colliding with wild monsters
instead of clicking them. The existing in-world battle (camera framing, HP
bars, keys 1/2/F, turn/damage/XP/capture logic) is kept, re-triggered and
re-framed.

## Goals

- Exploring the solar system feels like flying a ship, not dragging a camera.
- E-boost (10× thrust, raised max speed) makes interplanetary travel fast.
- Physical collision with a wild monster starts a battle.
- Reuse all existing battle mechanics and the LOD/renderer stack unchanged.

## Non-goals

- Pokémon-style 2D battle screen (explicitly rejected: keep in-world battle).
- Combat while flying, weapons, enemy ships.
- New creature content, moves, or balance changes.
- Touch/gamepad input.

## Architecture

### New module: `src/flight/`

Owns the ship entity, flight input, motion, the chase camera, and the
collision trigger.

**`PlayerShip` entity** — components:
- `PlayerShip` marker with a bounding `radius` (~10 km, used for collisions
  and planet clamping).
- `Transform` (position + facing), `ShipVelocity(Vec3)`.
- Visual children in the existing placeholder style: elongated cone/dart body
  plus an emissive engine-glow accent (reuses the `spawn_creature_visual`
  parent/child pattern, not its species visuals).

Spawned on `OnEnter(GameState::Exploring)` the first time only (guard against
re-entry from battles): positioned near the home planet at a safe altitude,
facing tangentially to the surface.

**Flight systems**, all `run_if(in_state(GameState::Exploring))`:

1. `flight_input` — reads keyboard/mouse, updates `ShipVelocity` and facing:
   - Mouse steering: the ship yaws/pitches toward the cursor's offset from
     screen centre, scaled by a turn-rate constant. Cursor at centre = fly
     straight. No pointer lock; the visible cursor's offset is the input.
   - `W`/`S`: thrust along ship forward / brake (reverse thrust, clamped so
     braking stops rather than reverses at low speed).
   - `A`/`D`: lateral strafe thrust; applies a small visual roll for feel.
   - Hold `E`: multiplies thrust by 10 and raises the max-speed clamp 10×.
2. `ship_move` — integrates velocity into the transform (frame-rate
   independent via `Time::delta_secs`), applies exponential drag so the ship
   coasts to a stop, clamps speed to max (base vs. boost).
3. `planet_clamp` — for each planet (`CelestialBody`), if the ship is inside
   `radius + margin`, project it back onto that sphere (slide, don't bounce)
   and zero the inward velocity component.
4. `chase_camera` — positions the camera behind and above the ship
   (offset in ship-local space), smoothed with an exponential lerp;
   `look_at` a point slightly ahead of the ship.
5. `collide_with_monsters` — see Battles below.

### Camera

- Projection switches from orthographic to **perspective** (~60° FOV) at app
  setup. All orbit machinery is deleted: `handle_orbit`,
  `handle_keyboard_orbit`, `handle_scroll`, `handle_double_click_zoom`,
  `sync_camera_transform`, `DoubleClickState`, `OrbitState`, and
  `creatures/pick.rs` (`ortho_pick_ray` and the wild-monster picker).
- `ZoomLevel` survives as a resource but becomes **derived, not input**: a
  `derive_zoom_from_distance` system (runs in all states) computes the
  camera's distance to the nearest planet surface, maps it to an effective
  ortho-equivalent scale (`scale = k · distance`, k tuned so street-level
  flight lands in the buildings LOD band and interplanetary distance lands in
  the solar band), then inverts the existing formula
  (`value = log10(scale / 0.001) / ZOOM_EXPONENT`, clamped to [0, 1]).
- Consequence: `lod/`, `LodRange` components, and all renderer LOD wiring are
  untouched. Flying toward a planet streams in terrain/buildings; boosting
  away fades up the solar system.

### Battles

**Trigger.** `collide_with_monsters` (in `Exploring`): sphere-vs-sphere test
between the ship (`PlayerShip.radius`) and each `WildMonster`'s `ClickSphere`
using `GlobalTransform` positions (monsters are children of orbiting
planets). On overlap: zero the ship velocity, insert
`BattleSession::new(wild_entity)`, set `GameState::Battle`. This is the same
hand-off point click-picking used, so turns, damage, XP, capture, and
win/lose flow are reused unchanged.

**Framing without `OrbitState`.** `setup_battle` computes the camera pose
directly: same framing math as today (pivot on the wild monster, distance
`BATTLE_CREATURE_SCALE * 12`, elevation 0.2) but written straight to the
camera `Transform`. `BattleSession.saved_orbit` is removed — on battle exit
the chase camera reasserts itself behind the parked ship next frame. The
ship stays visible where the collision happened; flight systems are gated to
`Exploring` so it cannot move. Planet orbiting stays frozen during battle
(existing behavior).

**Re-collision guard.** If the player loses or flees, the monster survives
with the ship inside its collision sphere. Two-part fix:
- On `OnExit(GameState::Battle)`, nudge the ship to just outside the
  monster's collision radius (along the ship→monster axis, away from it).
- A `CollisionCooldown` timer resource (~2 s) suppresses
  `collide_with_monsters` after every battle exit.

### State & scheduling summary

| State | Active systems |
|---|---|
| `StarterSelect` | starter UI (unchanged) |
| `Exploring` | flight_input, ship_move, planet_clamp, chase_camera, collide_with_monsters |
| `Battle` | battle_input, enemy_turn, HP-bar systems (unchanged) |
| all | derive_zoom_from_distance (feeds LOD) |

## Deleted code

- `creatures/pick.rs` (entire file) and its plugin registration.
- In `camera/zoom.rs`: `OrbitState`, `DoubleClickState`, all four input
  handlers, `sync_camera_transform`, `ray_sphere_intersect`, and their tests.
  `ZoomLevel` + `to_ortho_scale` (and their tests) stay.
- `BattleSession.saved_orbit` and the save/restore in setup/teardown.

## Error handling

- Battle setup with a despawned wild entity already early-returns; unchanged.
- `planet_clamp` guards against zero-length direction vectors
  (`normalize_or_zero`).
- Derived zoom clamps to [0, 1]; degenerate "no planets" case falls back to
  zoom = 1.0 (solar view).

## Testing

Unit tests (headless, following existing patterns):
- **Derived zoom:** distance→value round-trips through `to_ortho_scale`;
  clamps at both ends; near-surface distance lands in the buildings LOD band,
  interplanetary distance in the solar band.
- **Flight math:** thrust integrates into velocity; drag decays it; boost
  multiplies thrust/max-speed 10×; speed clamp holds; planet clamp keeps the
  ship outside `radius + margin` and zeroes inward velocity.
- **Collision:** overlap inserts `BattleSession` and switches state; no
  trigger outside radius; cooldown suppresses an immediate re-trigger.
- **Battle framing:** camera pose derived from monster position matches the
  expected distance/elevation.

Manual playtest for feel (turn rate, drag, base/boost speeds, camera offset
and smoothing) — constants exposed at module top for easy tuning.

## Open tuning constants (initial guesses, playtest to adjust)

| Constant | Initial value |
|---|---|
| Base max speed | 100 km/s |
| Boost multiplier | 10× |
| Thrust accel | 80 km/s² |
| Drag half-life | ~0.7 s |
| Turn rate | 1.2 rad/s at full mouse offset |
| Camera offset | 60 km behind, 25 km above |
| Ship radius | 10 km |
| Collision cooldown | 2 s |
