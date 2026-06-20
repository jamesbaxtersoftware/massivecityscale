# Galaxy Roamer — Flight Vertical Slice (Design Spec)

**Date:** 2026-06-20
**Type:** Build spec (input to an implementation plan)
**Origin:** First sub-project of the galaxy-roamer rebuild. See the brainstorming
brief: [`2026-06-20-galaxy-roamer-from-scratch-brief.md`](2026-06-20-galaxy-roamer-from-scratch-brief.md).
**Stack:** Bevy / Rust (clean slate over the v1 `massivecityscale` codebase).

---

## 1. Goal & Acceptance Test

Build the proof-of-pillar vertical slice from the brief's §11:

> **Fly a ship through a deterministic starfield with mouse-steer + chase camera,
> pick a star, approach its planet, and descend seamlessly until terrain resolves
> under you — and it feels good.**

This is the single acceptance test. If the descent feels great, the project is on
track. If it doesn't, nothing else matters yet.

**In scope:** flight, a deterministic starfield + a few star systems, distance-based
LOD/streaming, one descendable planet, seamless space→surface terrain resolution.

**Out of scope (this slice):** battles, on-foot/walking, disembarking, biome
variety beyond a color or two, persistence, audio, atmosphere/ring shaders. The
slice *ends* the moment terrain resolves under the ship — there is no landing,
exit, or walking yet.

---

## 2. The Spine: Camera-Relative Floating Origin

This is the foundational decision everything else sits on, and it is built first.

**The problem.** A single flat f32 coordinate space spanning a galaxy shatters into
jitter — f32 carries ~7 significant digits, so positions thousands of km from the
origin lose sub-metre precision. Worse, the signature descent spans ~6 orders of
magnitude, from a planet radius (hundreds–thousands of km) down to metre-scale
terrain under the ship.

**The solution.**
- Authoritative positions are stored in **f64** via a `WorldPos(DVec3)` component
  on every body and on the ship.
- The ship's `WorldPos` is published each frame as the `FloatingOrigin` resource.
- Every render `Transform` is derived as `(WorldPos − origin) as f32`. **The ship
  always renders at or near `(0, 0, 0)`**, so the f32 offset is tiny exactly where
  precision matters most — at the ship, and at whatever it is descending toward.
- The *same* mechanism carries a planet continuously from a distant dot, to a
  sphere, to terrain under the player's feet — with no coordinate switch and no
  loading screen. The seam simply does not exist.

**Deliberately deferred.** Sector / `i64`-tile nesting (composing many small precise
local spaces across a full galaxy) is the eventual generalization for 10,000s of
bodies. The slice's scale does not need it; the camera-relative origin is
sufficient and simpler. This is recorded as future work, not built now.

---

## 3. Modules

A new flight-first spine that fully replaces v1. Each module has one clear purpose
and a small, testable surface.

| Module | Purpose | Depends on |
| --- | --- | --- |
| `origin` | Floating-origin core: `WorldPos`, `FloatingOrigin`, the transform-sync system. | — (everything depends on it) |
| `ship` | Player ship + flight controls (mouse-offset yaw/pitch, **no roll**, key throttle, hold-to-boost, momentum/drag) + smoothed chase camera. | `origin`, tunables |
| `galaxy` | Seed-deterministic generation: backdrop starfield + a few star systems, each with planets (`WorldPos`, radius, type/color). Pure & unit-tested. | `origin`, `palette` |
| `streaming` | Distance-tiered LOD: cull / instanced point / low sphere / full sphere / terrain-resolving. Drives the dot→sphere→surface continuum. | `origin`, `galaxy` |
| `terrain` | For the approached planet, resolves a displaced surface patch under the ship during descent. | `origin`, `streaming` |
| `palette` | Carried-over swappable colors (minimal rewrite from v1's themes) for bodies. | — |

**Tunables live in one place.** A single tuning resource (thrust, drag half-life,
boost multiplier, turn rate, camera offsets/smoothing, LOD distance thresholds)
so feel is found by playtesting, not by guessing constants once (brief §3).

---

## 4. Data Flow (per frame)

Systems run in this order:

1. `ship_input` — read mouse offset from screen centre (yaw/pitch, with deadzone)
   and throttle/boost keys.
2. `ship_move` — integrate thrust, drag, and boost into the ship's `WorldPos`.
3. `set_origin` — publish ship `WorldPos` → `FloatingOrigin`.
4. `sync_transforms` — set every entity's `Transform = (WorldPos − origin) as f32`.
5. `chase_camera` — position the camera behind/above the ship (render space, ship
   near origin), smoothed.
6. `streaming_update` — per body: distance(ship, body) selects an LOD tier and
   swaps mesh/visibility.
7. `terrain_resolve` — for the nearest planet within descent range, generate/refine
   the surface patch under the ship.

**Collision ("slide, don't bounce", carried from v1):** clamp the ship to stay
outside the planet sphere (plus a small margin so it can still reach the surface)
and zero the inward component of velocity, rather than reflecting it.

---

## 5. The Three Hard Problems, Scoped to the Slice

1. **Floating-point precision** → the camera-relative floating origin (§2).
2. **LOD & streaming** → the slice proves the *mechanism*, not 10,000s of bodies.
   Body budget: ~500 instanced backdrop stars + ≤8 live system bodies + 1
   descendable planet. The galaxy-scale budget is a later spec.
3. **Seamless space↔surface transition** → falls out of §2 + §3 for free: one
   continuum from dot to terrain, no transition seam to engineer.

---

## 6. Performance Target

1280×720, **60 fps**, at the §5 body budget on the development machine. This gives
streaming and LOD a real number to aim at. The full-galaxy budget (resolution,
framerate, body counts at scale) is deferred to its own spec.

---

## 7. Testing

**Deterministic / pure (unit tests):**
- `galaxy::generate(seed)` — same seed reproduces the same starfield, systems, and
  planet types/positions.
- LOD-tier selection — correct tier chosen at each distance threshold boundary.
- Floating-origin math — a `WorldPos` far from origin yields the correct,
  jitter-free relative `Transform` when the origin is near it.

**Manual playtest gate (the real acceptance test):** the "feels good" descent from
§1, exercised by running the app. Feel cannot be unit-tested; it is the gate that
matters most.

---

## 8. Phased Build Order

Every phase compiles and runs on its own (brief §8).

- **P0 — Clean slate.** Delete all of v1 `src/` (`world_gen`, `camera`, `lod`,
  `renderer`, `creatures`, `flight`, `theme`) and reduce `main.rs` to a window + a
  camera that runs. Keep `Cargo.toml` / `Cargo.lock`. v1 remains in git history as
  reference.
- **P1 — Floating-origin spine.** Fly a ship cube with mouse-steer (no roll) and a
  smoothed chase camera through empty space, on `WorldPos`/`FloatingOrigin`, with
  the central tunables resource. *Proves feel early.*
- **P2 — Starfield + galaxy gen.** Deterministic `galaxy::generate`; fly through the
  backdrop starfield and approach a star system.
- **P3 — Planet as LOD body.** A planet resolves dot→sphere as the ship approaches;
  slide-collision keeps the ship outside the sphere.
- **P4 — Seamless descent → terrain.** Terrain resolves under the ship as it
  descends. **Acceptance: it feels good.**

---

## 9. Non-Goals (this slice)

Battles, on-foot/walking, disembarking, biome variety, persistence, audio, and
atmosphere/ring shaders. The richer "wow of arrival" shaders and the on-foot battle
loop are noted as future phases with their own specs.
