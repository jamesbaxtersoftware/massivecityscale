# Galaxy Roamer — From-Scratch Design Brief

**Date:** 2026-06-20
**Type:** Brainstorming brief (input to a fresh design session — NOT a build spec)
**Origin:** Distilled from the `massivecityscale` project, rebuilt around a new core pillar.

> **How to use this document.** Paste it into a new brainstorming session as the
> starting brief. It states the vision, the decisions already made, the hard
> problems to solve, and the open questions to resolve. It deliberately does
> **not** lock down architecture — that's the brainstorming session's job.

---

## 1. Vision & Core Pillar

Build a game where you **freely fly a spaceship across a procedural galaxy of
thousands of worlds**, descend to any planet without a loading screen, land,
get out, walk around, and battle the creatures you find there.

**The core pillar is flight between worlds.** Everything radiates from the
feeling of piloting a ship through space and down to a surface. This is the one
sentence the whole project is accountable to.

**Lesson from v1 (the hard-won reason this brief exists).** The previous
project was built renderer-first: a zoom/orbit camera over a single solar
system, then creature battles, and *only at the very end* was free flight
retrofitted by ripping out the orbit camera. The retrofit worked but fought the
architecture the whole way. **This time flight is the spine from day one** —
the camera, coordinate system, LOD, and streaming are all designed in service
of a ship moving freely through a huge space.

---

## 2. Player Fantasy & Core Loop

1. **Fly** — pilot a ship through open space; the galaxy stretches out around you.
2. **Choose a destination** — pick a star, approach a system, pick a planet.
3. **Descend** — seamless transition from orbit to atmosphere to surface (no
   loading screen — this is the signature moment).
4. **Land & disembark** — set down, leave the ship, walk the surface on foot.
5. **Encounter & battle** — wild creatures roam each world; engage them in
   in-world turn-based battles.
6. **Capture / level up** — defeat for XP or trap to capture; build a collection.
7. **Return & repeat** — back to the ship, lift off, fly to the next world.

The loop must feel continuous — space, descent, surface, and battle are one
unbroken world, not separate screens.

---

## 3. Flight Feel & Controls *(first-class — the make-or-break of the project)*

Flight is the pillar, so "does it feel good to fly?" is the primary success
metric. Treat this section as a design driver, not polish.

**Design goals for feel:**
- **Responsive but weighty** — the ship answers immediately, yet carries
  momentum so it feels like a craft, not a free-floating camera. Coast and
  drift, don't stop dead.
- **Readable orientation** — the player always knows which way is "forward,"
  where the nearest planet is, and roughly how fast they're going.
- **Effortless scale traversal** — the same controls must feel good both for
  fine maneuvering near a surface and for crossing interplanetary gulfs
  (a boost/throttle range that spans orders of magnitude without feeling
  twitchy up close or glacial far away).
- **Smooth descent** — pitching down toward a planet and dropping into the
  atmosphere should feel like one continuous arc, with the camera and speed
  scaling naturally as the ground rushes up.

**Things to nail in brainstorming (open, but flagged as critical):**
- **Control scheme** — mouse-steer vs. keyboard yaw/pitch vs. full 6DOF;
  whether there's roll; whether there's a pointer-lock "cockpit" feel or a
  free cursor. (v1 used mouse-offset-from-center steering with no roll and a
  chase camera — a known-decent baseline to start from and improve.)
- **Throttle model** — discrete thrust+drag (v1) vs. a held throttle/cruise;
  how boost works (v1: hold to multiply thrust & max-speed ~10×).
- **Camera** — chase camera offset/smoothing; how it reframes during descent
  and on the surface; FOV/speed-based effects for a sense of velocity.
- **Assists** — auto-leveling, flight-toward-target lock-on, collision
  "slide don't bounce" against planet surfaces (v1 clamped the ship outside a
  planet sphere and zeroed inward velocity — keep that idea).
- **Feedback** — engine glow, screen-space speed cues, subtle camera shake
  under boost. Cheap, but they sell the feel.

**Tunables must live in one place** and be adjustable without a recompile loop
where possible — feel is found by playtesting, not by guessing constants once.

---

## 4. World Structure — The Galaxy

- **Seed-deterministic galaxy.** A single seed reproduces the entire galaxy
  (star positions, system contents, planet types, creature populations).
  Determinism keeps it shareable, testable, and debuggable.
- **Scale:** thousands of stars. **Each star has 1–100 planets and moons** →
  easily tens of thousands of explorable bodies.
- **Generate-on-demand.** Only the galaxy-level star map exists up front; a
  star system's planets, terrain, and creatures are generated when the player
  approaches and discarded/cached when they leave. Nothing exists in full
  detail everywhere at once.
- **Planet typing drives content.** Each planet derives a type/biome (extending
  v1's Fire/Water typing) from its seed; biome determines terrain look and
  which creatures spawn there.
- **Hand-tunable vs. procedural:** the brainstorming session should decide what
  is purely seed-driven vs. what gets authored tuning curves (e.g., planet-count
  distribution, biome rarity, creature-difficulty-by-distance-from-core).

---

## 5. The Hard Problems *(flag for brainstorming — do not pre-solve here)*

These are the technical risks that the architecture must be designed around.
They are the reason flight-first matters.

1. **Floating-point precision across galactic distances.** A single flat
   coordinate space spanning a galaxy will shatter into jitter. Needs a
   **floating origin** and/or **nested coordinate spaces** (v1's pattern:
   small, precise local coordinates composed through a transform hierarchy).
   This is the foundational decision everything else sits on.
2. **LOD & streaming for tens of thousands of bodies.** GPU instancing for the
   star/planet field; generate full detail only near the player; stream in/out
   as the ship moves. Budget how many bodies are "live" at once.
3. **Seamless space ↔ surface transition.** The signature challenge: no loading
   screen between orbit and walking on the ground. How does a planet go from a
   dot, to a sphere, to terrain under your feet, continuously? This likely
   constrains the coordinate/LOD design more than anything else.
4. **Performance budget at scale.** Define the target (resolution, framerate,
   body counts) early so streaming and LOD aim at a real number.
5. **On-foot mode vs. flight mode.** Two control regimes (piloting vs. walking)
   and the handoff between them (exit ship, re-board) — state machine + camera
   handling.

---

## 6. Battles

- **On-foot, in-world, turn-based** — reuse v1's proven mechanics rather than
  reinventing: type matchups, damage with type multipliers, XP/leveling, and a
  capture/"force-field" trap mechanic with a collection.
- **Trigger:** surface encounter (walk into / approach a wild creature) rather
  than the v1 ship-collision trigger, since battles now happen on foot.
- **Presentation:** in-world 3D framing with billboarded HP bars (v1 style) —
  not a separate 2D battle screen.
- **Creature variety scales with biomes** — each planet type hosts themed
  creatures; the species count grows with the number of biomes.

---

## 7. Tech Recommendation

**Recommended: Bevy / Rust.** For 10,000s of bodies the bottleneck isn't
drawing spheres (any modern engine instances those trivially) — it's the two
things that bite at galactic scale:

- **Precision across huge distances** — Rust/Bevy's ECS + transform hierarchy
  is well-suited to a floating-origin / nested-coordinate design, and v1 already
  proved this pattern here.
- **Many-entity throughput** — an ECS handles tens of thousands of streamed
  entities far more comfortably than a JS scene graph.

**Tradeoff recorded:** a web stack (Three.js + TypeScript) wins on
shareability, instant runs, and iteration speed, and can instance thousands of
planets fine — but precision management and many-entity streaming are more
error-prone in JS, and the performance ceiling is lower. Keep this as a live
option if "runs in a browser / easy to share" outranks raw scale.

---

## 8. Carry Over From v1 (what already worked)

- **Nested coordinate spaces / floating-origin thinking** — the precision
  solution; start here, generalize it from one system to a galaxy.
- **Procedural-geometry art style** — boxes, spheres, cones, flat/emissive
  colors, zero external art assets. Fast, cohesive, cheap.
- **Themeable color palettes** — swappable at runtime (dark-neon / pastel /
  earthy). Cheap variety; reuse for biomes.
- **In-world 3D battle framing with billboarded HP bars** — keep, move on-foot.
- **Phased, independently-runnable build order** — every milestone compiles and
  runs on its own (v1 did this well; preserve it).
- **Deterministic-from-seed generation with unit tests** — keep generation pure
  and testable.

---

## 9. Open Questions for the Brainstorming Session

- **Galaxy size & density** — how many stars; planet-count distribution per
  star; how spread out; is the galaxy bounded or does it wrap/repeat?
- **Surface scale & traversal** — how big is a walkable planet surface? Walk
  only, or a rover/vehicle? Full sphere or a generated patch where you land?
- **Landing model** — fully piloted manual landing, assisted/auto-land, or a
  hybrid? How forgiving?
- **Flight control scheme** — the open items in §3 (6DOF? roll? mouse vs keys?
  cockpit vs chase?).
- **Creature scaling** — how many species; how variety maps to biomes;
  difficulty curve across the galaxy.
- **Persistence** — save/load of position + collection across runs? (Likely a
  later phase, but decide.)
- **Audio** — engine/ambient sound? (Cheap feel multiplier; v1 had none.)
- **Art direction** — stay minimalist-procedural, or push toward richer shaders
  (atmosphere scattering, planet rings, nebulae) for the "wow" of arrival?

---

## 10. Explicit Non-Goals (keep v1 focused)

- Multiplayer / networking.
- Base building, economy, crafting, trading.
- Combat *while flying* (no space dogfights) — flight is travel; battles are
  on foot.
- Realistic orbital mechanics / n-body physics — fake it for feel.
- External art/audio asset pipelines — stay procedural for the first version.
- Mobile / gamepad input (keyboard + mouse first; revisit later).

---

## 11. Suggested First Milestone (proof of the pillar)

Before any battles or biomes: **a ship you can fly through a starfield, pick a
star, approach a planet, and seamlessly descend until terrain resolves under
you — and it feels good.** If that single vertical slice feels great, the
project is on track. If it doesn't, nothing else matters yet.
