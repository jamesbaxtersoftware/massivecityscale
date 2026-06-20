# Pause Menu + Controls — Design Spec

**Date:** 2026-06-20
**Type:** Build spec (small feature on the galaxy-roamer flight slice)
**Branch:** galaxy-roamer

## Goal

Add a pause menu that freezes the simulation and shows a controls list. Esc toggles
pause; the menu is keyboard-driven.

## Behavior

- **`AppState` (Bevy states): `Running` | `Paused`**, starts `Running`.
- **Esc** toggles `Running` ↔ `Paused` (works in both states).
- While **`Paused`** the whole per-frame pipeline is frozen (no flight, movement,
  camera, streaming) and a menu overlay is shown.
- **Q** while paused quits the app (`AppExit::Success`). Quitting now lives behind the
  menu — bare Esc no longer quits.

## Freezing mechanism

The per-frame systems already run through the chained `FrameSet`
(Input→Move→Origin→Sync→Camera→Stream) configured in `OriginPlugin`. Add
`.run_if(in_state(AppState::Running))` to that chain — a single gate freezes flight,
movement, origin/transform sync, chase camera, and streaming together (transforms stop
updating, so the scene holds still). `terrain::resolve_terrain` reads frozen LOD tiers,
so it needs no separate gate.

## Module structure

New `src/pause/mod.rs` (`PauseMenuPlugin`):
- `AppState` enum (`States`) + `init_state`.
- `PauseMenu` marker component (on the UI root).
- `toggle_pause` — Esc `just_pressed` flips state; ungated (runs in both states).
- `spawn_pause_menu` on `OnEnter(AppState::Paused)` — spawns a `PauseMenu`-marked
  bevy_ui overlay: a dim full-screen `Node` panel with a "PAUSED" title and the controls
  list (Mouse — Steer · W/S — Thrust · A/D — Strafe · Shift — Boost · Esc — Resume ·
  Q — Quit).
- `despawn_pause_menu` on `OnExit(AppState::Paused)` — despawns the `PauseMenu` root
  (recursively).
- `quit_from_menu` — Q `just_pressed`, only while `Paused`, sends `AppExit::Success`.

Edits:
- `src/origin/mod.rs`: add the `.run_if(in_state(AppState::Running))` gate to the
  `FrameSet` chain (imports `crate::pause::AppState`).
- `src/main.rs`: `mod pause;`, add `PauseMenuPlugin`, remove `quit_on_escape`.

## Rendering notes

Bevy 0.15 `Node` UI + `Text` using Bevy's built-in default font (ships with the engine;
nothing added to `assets/`, so the "procedural, no external assets" constraint holds).
UI draws over the existing `Camera3d`.

## Testing

State-machine tests in the existing App-based style:
- Esc toggles `Running` → `Paused` → `Running`.
- Entering `Paused` spawns exactly one `PauseMenu` root; exiting despawns it.
- Q while `Paused` emits an `AppExit` event; Q while `Running` does not.

The visual layout of the overlay is the deferred manual check (no display in CI), like the
rest of the slice.

## Non-goals

Mouse-clickable buttons (keyboard-only menu), settings/options, save/load, audio.
