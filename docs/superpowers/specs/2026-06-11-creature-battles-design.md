# Design: Creature Battles ("little monsters")

**Date:** 2026-06-11
**Status:** Approved, pending implementation plan

## Summary

Layer a creature-collecting/battling mechanic (Pokémon-style) onto the existing
solar-system → planet-surface renderer. Each planet is typed Fire or Water and
hosts wild monsters of that type. The player picks a starter creature, clicks a
wild monster to start an in-world turn-based battle, and can defeat (for XP) or
trap (capture) it. Two creature species ship initially — one Fire, one Water.

## Game Loop

1. **Launch → Starter select:** two starter creatures (Fire, Water) float in
   front of the camera; click one to claim it as `PlayerCreature`.
2. **Explore:** orbit/zoom the solar system as today. Each planet is typed Fire
   or Water (derived from its color) and has a handful of wild monsters of its
   type roaming its surface — visible as small shapes in solar view, full
   creatures up close (parented to the planet so they orbit with it).
3. **Click a wild monster → Battle** (in-world, approach B): the camera glides
   to frame the player creature vs the wild one; floating HP bars appear above
   each.
4. **Turn-based:** the player picks a move (keyboard `1`/`2`) or throws a
   **force field** (`F`) to trap the monster. Damage resolves with a type
   multiplier and a quick lunge/flash; the enemy then takes its turn.
5. **Outcomes:**
   - **Win** → XP + possible level-up (more HP/attack); wild monster fades out
     and is removed from the planet.
   - **Trap success** → monster captured into the player's `Collection`,
     removed from the planet.
   - **Lose** → player creature faints, heals to full, battle ends, monster
     stays.
6. Camera restores; back to Explore.

## Architecture: Approach B (in-world 3D battle)

The battle is rendered in the 3D world with real creature meshes facing off and
billboarded HP bars — not a 2D overlay panel. A `GameState` enum gates input and
systems but is orthogonal to presentation.

### State machine

`GameState` (Bevy `States`): `StarterSelect → Exploring → Battle`.

### Module layout (new `src/creatures/`)

- `mod.rs` — `CreaturesPlugin`, shared types, resources, `GameState`, and the
  **type-matchup function**.
- `spawn.rs` — derive each planet's `CreatureType` from its color; spawn N wild
  monsters per planet; build the two creature meshes.
- `pick.rs` — **shared orthographic ray-builder extracted from
  `solar::pick_planet`**; clicking a wild monster (while in `Exploring`) opens a
  battle; planets still re-target the camera as today.
- `battle.rs` — battle state machine: spawn the player creature into the scene,
  turn resolution, type damage, XP/level, trap odds, win/lose, cleanup.
- `battle_view.rs` — camera framing lerp, billboarded HP-bar quads, hit flash,
  action input handling.
- `starter.rs` — starter-select state and its click handling.

**Refactor note:** the orthographic ray-construction code currently inline in
`solar::pick_planet` is extracted into a reusable helper so both the planet
picker and the monster picker share it. This is a targeted improvement in
service of the feature, not unrelated refactoring.

## Data Model

- `enum CreatureType { Fire, Water }`
- `Species` — 2 initially (one Fire, one Water); carries base stats + move set.
- `struct Creature { species, type, level, xp, hp, max_hp, attack }` — used for
  both the player's creature and wild ones.
- Resources:
  - `PlayerCreature(Creature)`
  - `Collection(Vec<Creature>)` — captured monsters
  - `Inventory { force_fields: u32 }` — starts at 3
  - `BattleSession { wild_entity: Entity, turn: Turn }` — present only during
    `Battle`
- Wild monster entity: `Creature` + `WildMonster` marker + a click sphere
  (radius for raycasting, mirroring `CelestialBody`), **parented to its planet**
  so it orbits along with it. Carries an `LodRange` so it shows across zoom
  levels.

## Battle Mechanics

- **Type matchup:** Water move → Fire target = ×2; Fire move → Water target =
  ×0.5; same type = ×1. Implemented as a single `type_multiplier(attacker,
  defender)` function.
- **Moves:** each creature has 2 themed moves of differing power
  (e.g. Fire: *Ember* / *Flare*; Water: *Splash* / *Torrent*).
- **Damage:** `base_power × attack_factor × type_mult` with light randomness.
  All tunable in one place in `battle.rs`.
- **XP / level:** a win grants XP proportional to the wild creature's level; on
  crossing a threshold the creature levels up → `max_hp` and `attack` increase.
- **Healing:** the player creature heals to full after every battle (win or
  lose). No healing items yet (YAGNI).
- **Force field (trap):** success probability
  `p = clamp(base + (1 − hp/max_hp), 0..1)`; using one consumes an `Inventory`
  force field; on failure the wild monster gets a free turn. Start with 3 force
  fields; no way to acquire more yet (YAGNI).

## Visuals (matching the existing simple-geometry style)

- **Fire creature:** reddish-orange teardrop/sphere body with a couple of cone
  "flame" spikes; unlit warm material.
- **Water creature:** blue rounded body with a flattened-box fin; smooth
  material.
- **HP bars:** thin billboard quads (dark background + green→red fill) that face
  the camera, floating above each creature. No 2D panel.
- **Hit feedback:** brief emissive flash + a small scale "lunge" on the
  attacker.

## Input (approach B, panel-free)

- **Starter select:** click one of the two floating starter creatures.
- **Battle:** keyboard — `1`/`2` choose moves, `F` throws a force field.
- **Explore:** unchanged (drag to orbit, scroll to zoom, click planet to
  re-target).

## Build Order (for the implementation plan)

Each phase is independently runnable and testable.

1. **Foundation:** data model + planet typing + wild-monster spawning +
   click-detect (log only, no battle).
2. **Starter select** state and creature claim.
3. **Battle core:** `Battle` state, camera framing, spawn creatures, HP bars,
   one move each, win/lose + cleanup.
4. **Depth:** type matchup, 2 moves per creature, XP/leveling.
5. **Capture:** force-field trap + `Collection`.

## Testing

- `type_multiplier` unit tests (all 4 type pairings).
- Damage calculation determinism with a seeded RNG.
- Trap probability bounds (0 HP → high, full HP → low; clamped to [0,1]).
- Planet-type derivation from color is deterministic.
- Spawn system produces wild-monster entities parented to planets (headless app
  test, mirroring the existing `renderer_plugin_spawns_mesh_entities` test).
- State transitions: click wild monster in `Exploring` → enters `Battle`;
  win/lose/trap → returns to `Exploring`.

## Non-Goals (for this iteration)

- More than 2 species / types.
- Party management or switching creatures mid-battle.
- Healing items, status effects, multi-move-per-turn.
- Acquiring additional force fields.
- Persistent save/load of collection across runs.
