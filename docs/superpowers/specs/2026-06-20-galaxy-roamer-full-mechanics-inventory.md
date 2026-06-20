# Galaxy Roamer — Full Mechanics Inventory (Rethink)

**Date:** 2026-06-20
**Purpose:** Complete, schedulable list of every gameplay mechanic, system, and
graphics/feel element for the full game (not the flight slice). Each item is one
line you can turn into a scheduled task.

**Status legend:** ✅ done · 🟡 partial · 🔴 not started · ⚠️ needs rework / decision

**Honest baseline:** what exists today is a flight *skeleton* — floating-origin
coordinates, a box "ship" with mouse-steer/thrust/boost, a tiny deterministic
galaxy (3 systems), distance-based LOD, slide-collision, a displaced-sphere
"terrain" that appears on close approach, and a pause menu. Everything that makes
it a *game* (landing, on-foot, catching, HUD, economy, progression) is unbuilt.

---

## Decisions (LOCKED 2026-06-20)

- **D1 = 3D + pixel-art post-process.** Keep the 3D engine (preserves floating
  origin, seamless descent, terrain); add a pixelation post-process + flat/banded
  shading; sprite-based creatures & UI.
- **D2 = realistic scale + warp travel.** Planets thousands of km, ship ~30 m; fly
  in-system, warp/jump across light-years (LY in HUD); fine control near surface.
- **D3 = keyboard+mouse now, gamepad-ready.** Implement kbd+mouse via an action-map
  so gamepad is a cheap later add.

Fleshed-out detail with per-feature verification:
`2026-06-20-galaxy-roamer-detailed-design.md`.

## 0. Two foundational decisions (block everything else)

- **D1 — Art direction.** ⚠️ Reference is detailed **pixel-art / 2.5D**; current is
  flat 3D primitives. Options: (a) keep 3D-procedural and stylize, (b) go full 2D/
  2.5D pixel-art, (c) 3D world with pixel-art creatures/UI. This decides the
  renderer, assets, and most graphics tasks below.
- **D2 — Scale & traversal model.** ⚠️ Define real sizes (e.g. planets 2,000–
  10,000 km, moons 300–2,000 km, ship ~30 m, player ~2 m) AND how you cross
  interstellar distances (shown in **light-years**): cruise + warp/jump vs. literal
  flight. Current km-flight is far too slow and the ship/planet ratio is inverted.
- **D3 — Input target.** ⚠️ Keyboard+mouse only, or keyboard+mouse **and** gamepad?
  The reference HUD uses gamepad prompts (Y/A/B/+/RT). Drives the controls in §1A.

---

## 1. Foundational / cross-cutting

1. ⚠️ **Scale overhaul** — realistic body/ship/player sizes; recompute camera,
   collision, LOD, terrain thresholds in the new scale.
2. 🔴 **Multi-scale traversal** — speed spanning orders of magnitude: fine near
   surface, cruise in-system, warp/jump across LY.
3. 🟡 **Galactic precision** — f64 floating-origin exists; extend to sectors +
   warp targeting for LY-scale distances.
4. 🔴 **Performance budget** — target resolution / framerate / live-body counts.
5. 🔴 **Game state machine** — Space ↔ Descent ↔ On-foot ↔ Battle ↔ Menus.

## 1A. Controls & input bindings (the explicit "press X to Y" layer)

The action *systems* live in the sections below; this section defines the **input
map** — what the player presses to trigger each, in both modes, with on-screen
prompts (as in the reference: `Y Scan · A Throw Disc · B Crouch · + Menu · RT Boost`).
Keyboard+mouse first; gamepad bindings listed alongside. ⚠️ Gamepad support is a
decision (D3) — the reference UI is gamepad-style.

**Space mode**
- C1. 🟡 Steer — mouse move (yaw/pitch).
- C2. 🟡 Thrust fwd/back — `W`/`S`.
- C3. 🟡 Strafe — `A`/`D`.
- C4. 🟡 Boost — `Shift` / `RT` (held).
- C5. 🔴 Throttle/cruise set — scroll / `LT`+`RT` (if cruise model chosen).
- C6. 🔴 **Warp/jump to selected target** — `J` / `Y`.
- C7. 🔴 Lock-on / cycle target — `Tab` / `RB`.
- C8. 🔴 **Scan detected planet** — `Q` / `Y`.
- C9. 🔴 **Land / begin descent (when in range)** — `F` / `A` (prompt appears when
  over a valid planet; press to initiate the seamless descent).
- C10. 🔴 Open galaxy map — `M` / `D-pad`.
- C11. 🟡 Pause / menu — `Esc` / `+`.

**On-foot mode**
- C12. 🔴 Move — `W/A/S/D` / left stick.
- C13. 🔴 Look / camera — mouse / right stick.
- C14. 🔴 Run — `Shift` / stick-click.
- C15. 🔴 **Crouch** — `C` / `B`.
- C16. 🔴 **Interact / approach** — `E` / `A`.
- C17. 🔴 **Scan creature** — `Q` / `Y`.
- C18. 🔴 **Throw Capture Disc** — left-click / `A`.
- C19. 🔴 Select item (Disc/Bait/Heal/Revive/Flash) — `1`–`5` / item wheel.
- C20. 🔴 Use selected item — `R` / `X`.
- C21. 🔴 Open inventory — `I`.
- C22. 🔴 Open party / collection — `P`.
- C23. 🔴 **Return to ship / take off (near ship)** — `F` / `A` (board prompt).
- C24. 🟡 Pause / menu — `Esc` / `+`.
- C25. 🔴 **Rebindable controls + on-screen prompts** — config + contextual button hints.

## 2. Space flight — movement & controls

6. 🟡 **Steering** — mouse yaw/pitch, no roll (works; needs feel tuning).
7. 🟡 **Thrust / strafe** — W/S thrust, A/D strafe (works).
8. 🟡 **Boost** — exists; needs a meter + cooldown/heat.
9. 🔴 **Throttle/cruise model** — held-throttle or warp for long hops.
10. 🔴 **Fuel** — consumption, low-fuel warning, refuel.
11. 🔴 **Shield + Hull** — damage (asteroids/hazards), regen, destruction.
12. 🟡 **Slide collision** vs planets (works); extend to asteroids/stations.
13. 🔴 **Lock-on / autopilot** to a detected planet (player-facing, not dev-only).
14. 🟡 **Chase camera** (works); add boost FOV widening + shake.

## 3. Space — graphics & game-feel

15. 🟡 **Engine glow** (static now) → scale with throttle/boost.
16. 🔴 **Speed cues** — star streaks / motion particles flowing past the ship.
17. 🔴 **Real ship model** (currently a box).
18. 🔴 **Planet visuals** — atmosphere rim glow, clouds, rings, day/night, biome
    tint visible from orbit.
19. 🔴 **Galaxy backdrop** — nebulae, distant spiral galaxy, proper skybox (now
    just point stars).
20. 🔴 **Asteroid / debris fields** (hazard + visual interest).
21. 🔴 **Space stations / points of interest.**
22. 🔴 **Atmospheric entry effect** (heat/glow during descent).
23. 🟡 **Lighting** — per-system star as light source (now one global light).

## 4. Space — HUD / UI

24. 🔴 **Shield / Hull / Fuel bars.**
25. 🔴 **Boost meter.**
26. 🔴 **Radar / minimap** — compass + body/objective blips.
27. 🔴 **Distance-to-target readout (LY).**
28. 🔴 **"Planet detected" panel** — name, type, distance, thumbnail.
29. 🔴 **Objective tracker** ("Find a new planet").
30. 🔴 **Currency (crystals) counter.**
31. 🔴 **Progression track bar** — milestone nodes + boss marker.
32. 🔴 **Speed/throttle indicator + reticle.**

## 5. Galaxy / world structure

33. 🟡 **Seed galaxy** — thousands of stars (now 3 systems); scale up.
34. 🟡 **Systems** — 1–100 planets/moons each (now 1–3).
35. 🟡 **Planet typing/biomes from seed** (Fire/Water/Rock now).
36. 🔴 **Generate-on-demand streaming** — load/unload systems as you travel (now
    everything spawns at once).
37. 🔴 **Procedural naming** (Veridia Prime, etc.).
38. 🔴 **Galaxy map / star chart** — navigation + fast-travel/warp targeting.

## 6. Planet approach, scan, descent, landing

39. 🔴 **Detection** — nearby planets surfaced in HUD with distance.
40. 🔴 **Scan action** — reveal type/biome/creatures/resources.
41. 🟡 **Seamless descent** — terrain resolves on approach (works visually); not a
    real landing yet.
42. 🔴 **Landing** — in-range *land* prompt (C9), choose/auto a site, touchdown,
    ship settles.
43. 🔴 **Flight→on-foot handoff** — exit-ship prompt, camera/state switch.
44. 🔴 **Take-off / re-board** — board prompt near ship (C23), lift off, return to
    space.

## 7. On-foot mode

45. 🔴 **Player avatar** (character model).
46. 🔴 **Walk/run + third-person camera.**
47. 🔴 **Crouch.**
48. 🔴 **Walkable surface** — area/patch per landing.
49. 🔴 **Surface extent** — patch vs full sphere; streaming.
50. 🔴 **Interaction** — approach creatures/objects, prompts.
51. 🔴 **On-foot scan.**

## 8. Creatures — encounter & battle

52. 🔴 **Wild creatures roam the surface.**
53. 🔴 **Encounter trigger** (approach).
54. ⚠️ **Battle model** — original brief said turn-based; reference looks
    action/real-time catch. Decide.
55. 🔴 **Type system & matchups** (Fire/Water/Rock/…).
56. 🔴 **Damage / HP / weaken-to-catch.**
57. 🔴 **Creature stats** — HP, SP, level, moves.
58. 🔴 **Enemy AI.**
59. 🔴 **In-world battle framing** + billboarded HP bars.

## 9. Capture & collection

60. 🔴 **Capture Disc throw + capture %** (shown as a ring/percentage).
61. 🔴 **Capture-rate formula** (weaken × level × type × item).
62. 🔴 **Bait** (affect capture/lure).
63. 🔴 **Flash Bomb** (stun/escape).
64. 🔴 **Collection / "dex"** — species discovered, fill the collection.
65. 🔴 **Party** — active team (Flarehog/Aquabud/Rockfang…), up to N.
66. 🔴 **Storage** for extra creatures.

## 10. Items & inventory

67. 🔴 **Item system + counts** (Capture Disc, Bait, Heal Spray, Revive, Flash Bomb).
68. 🔴 **Heal Spray** (restore HP) / **Revive** (restore fainted).
69. 🔴 **Inventory UI** — quick-slots / item bar.
70. 🔴 **Item acquisition** — find / buy / craft.

## 11. Player & creature progression

71. 🔴 **Player level + EXP** (Lv.18, 348/800).
72. 🔴 **Player HP & SP.**
73. 🔴 **Creature leveling + XP from battles.**
74. 🔴 **Unlocks/abilities by level.**

## 12. Economy

75. 🔴 **Crystals currency** — earn/spend.
76. 🔴 **Shops / vendors** (station-based?).
77. 🔴 **Rewards** — catching, objectives, exploration.

## 13. Objectives / quests / meta-progression

78. 🔴 **Objective system** ("Find a new planet").
79. 🔴 **Progression track + boss encounters.**
80. 🔴 **Quest log.**

## 14. Biomes & content scaling

81. 🟡 **Biome definitions** (Grasslands shown) → terrain look + palette + creature
    pool (3 type-colors now).
82. 🔴 **Biome-specific creature pools.**
83. 🔴 **Difficulty scaling by distance from core.**

## 15. On-foot graphics & UI

84. 🔴 **Surface environment art** — sky, clouds, water, cliffs, trees, moon-in-sky.
85. 🔴 **On-foot HUD** — player HP/SP/EXP, item bar, party panel, tips, control
    prompts, biome/location label.
86. 🔴 **Capture VFX** — disc throw, containment bubble, capture animation.

## 16. Audio

87. 🔴 **Space audio** — engine, ambient, boost.
88. 🔴 **Surface/battle audio** — ambient, SFX, capture jingle, music.

## 17. Persistence & front-end

89. 🔴 **Save/load** — position, collection, party, items, currency, progress.
90. ✅ **Pause menu + controls.**
91. 🔴 **Settings** — audio/controls/graphics.
92. 🔴 **Main menu** — new game, continue, seed selection.

---

## Suggested milestone ordering (for scheduling)

1. **Decisions D1 (art) + D2 (scale/traversal)** — unblock everything.
2. **Re-scale + fast traversal + space game-feel** (items 1–2, 15–23) — make space
   actually feel good to fly and believable in scale.
3. **Space HUD** (24–32) — make it read like the reference.
4. **Approach → scan → land → on-foot handoff** (39–44, 5) — the core loop spine.
5. **On-foot movement + surface** (45–51, 84).
6. **Creatures: encounter → weaken → capture** (52–66, 86) — the "catch" pillar.
7. **Items, progression, economy, objectives** (67–80).
8. **Galaxy scale-up + streaming + map** (33–38).
9. **Audio, persistence, front-end** (87–92).

Each milestone becomes its own spec → plan → build cycle.
