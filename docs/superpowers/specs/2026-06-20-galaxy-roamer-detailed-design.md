# Galaxy Roamer — Detailed Design (Fleshed-Out, with Verification)

**Date:** 2026-06-20
**Companion to:** `2026-06-20-galaxy-roamer-full-mechanics-inventory.md` (the index).
**Decisions (locked):** D1 = 3D + pixel-art post-process · D2 = realistic scale +
warp travel · D3 = keyboard+mouse now, gamepad-ready action-map.

Each feature below gives: **What** (the mechanic), **Params** (concrete values to
start from), **Deps**, and **Verify** — how I will prove it works *myself* before
calling it done. Verification tools available: `cargo test` (pure logic), framebuffer
screenshots via the dev harness (`GR_SHOT`), scripted flight (`GR_FLY`), steering
(`GR_TURN`), and new scripted scenarios I add per feature. **Rule: no feature is
"done" without its Verify step passing, with the evidence recorded.**

Status: ✅ done · 🟡 partial · 🔴 not started · ⚠️ rework.

---

## Scale & units baseline (from D2) — applies everywhere

- **Unit = 1 metre.** World positions stay f64 (`DVec3`) — metres across a galaxy fit
  with the floating origin.
- Planets **2,000–10,000 km** radius; moons **300–2,000 km**; ship **~30 m** long
  (collision radius ~20 m); player **~2 m**.
- Camera chase: **~70 m** back, **~25 m** up (re-tuned for the 30 m ship).
- Speeds: surface fine **50–500 m/s**; in-system cruise up to **~5,000 km/s**;
  interstellar via **warp** (instant/animated jump), never literal flight.
- LOD thresholds recomputed in metres (see §5).
- **Verify (baseline):** a `scale_sanity` unit test asserting the constants hold the
  intended ratios (ship << moon << planet), plus a screenshot of the ship next to a
  planet showing the ship as a speck against a wall of planet.

---

## 0/D. Foundational decisions — DONE (locked above). 

## 1. Foundational / cross-cutting

- **1. Scale overhaul** ⚠️→ adopt the metre baseline; rescale ship, planets, camera,
  collision, LOD, terrain. **Verify:** scale_sanity test + side-by-side screenshot.
- **2. Multi-scale traversal / warp** 🔴 — three regimes: *surface* (m/s), *cruise*
  (km/s, in-system), *warp* (select a star on the map → animated jump, LY shown).
  **Params:** cruise accel curve; warp = 1.5 s animated transition. **Verify:**
  `GR_FLY` script that cruises to a planet AND a scripted warp between two systems;
  screenshot before/after + assert ship WorldPos jumped to target sector.
- **3. Galactic precision** 🟡→ extend floating origin with i64 sector tiles for
  LY-scale. **Verify:** unit test: a position 5 LY out yields a jitter-free relative
  transform after warp.
- **4. Performance budget** 🟡 — target ≥60 fps (hard requirement). `FrameTimeDiagnosticsPlugin`
  wired; dev harness prints FPS at capture (currently ~120, vsync-capped). **Verify:**
  FPS readout under a stress scene (max live bodies) stays ≥60.
- **5. Game state machine** 🔴 — `Space ↔ Approach ↔ Descent ↔ OnFoot ↔ Battle ↔
  Menu`, each gating its systems (extends the pause `AppState` pattern). **Verify:**
  state-transition unit tests for every legal/illegal edge.

## 1A. Controls / input (D3 action-map)

- **C-map** 🔴 — a `GameAction` enum + bindings resource (kbd+mouse now; gamepad
  table stubbed). All systems read actions, never raw keys. **Verify:** unit test
  that each action resolves to its default key; a scripted run pressing each action
  and asserting the effect (e.g. action `Land` → enters Descent state).
- **C9 Land / C8 Scan / C23 Board** etc. flesh out under their systems (§6, §7).
  **Verify:** per-binding scripted press + state/ъeffect assertion + screenshot of the
  on-screen prompt.

## 2. Space flight — movement & controls

- **6. Steering** 🟡 — mouse yaw/pitch, no roll. Re-tune `turn_rate` for the new
  scale. **Verify:** `GR_TURN` (done pattern) — view pans correct way, no roll.
- **7. Thrust/strafe** 🟡 — W/S, A/D. **Verify:** `GR_FLY` straight-line distance/time
  matches expected accel.
- **8. Boost + meter** 🟡→ boost drains a meter; recharge when off; can't boost empty.
  **Params:** 4 s full boost, 6 s recharge. **Verify:** scripted hold-boost; assert
  meter depletes to 0 then thrust drops to base; screenshot meter.
- **9. Throttle/cruise** 🔴 — held-throttle with a cruise cap; smooth ramp. **Verify:**
  scripted throttle ramp; assert velocity follows the curve.
- **10. Fuel** 🔴 — cruise/warp consume fuel; low-fuel warning; refuel at stations/
  planets. **Params:** warp costs N fuel/LY. **Verify:** scripted warp; assert fuel
  decremented; warning fires below threshold (screenshot).
- **11. Shield + Hull** 🔴 — asteroid/hazard hits drain shield then hull; shield
  regens after delay; hull 0 = destroyed→respawn. **Verify:** scripted collision with
  an asteroid; assert shield→hull order and regen; screenshot bars.
- **12. Slide collision** 🟡 (works for planets) → extend to asteroids/stations.
  **Verify:** scripted approach into an asteroid; assert no penetration.
- **13. Lock-on / autopilot** 🔴 — select detected planet → assisted heading/approach
  (player-facing version of the dev autopilot). **Verify:** press lock-on in script;
  assert heading aligns to target within tolerance; screenshot reticle on target.
- **14. Chase camera** 🟡→ boost widens FOV + subtle shake. **Verify:** screenshot at
  rest vs boosting; assert FOV value changed.

## 3. Space — graphics & feel (D1 pixel-art 3D)

- **15. Engine glow scaling** 🟡→ emissive intensity & length track throttle/boost.
  **Verify:** screenshots at idle/thrust/boost; glow visibly larger under boost.
- **16. Speed cues** 🔴 — star-streak/particle field flowing past, length ∝ speed.
  **Verify:** screenshot at low vs high speed; streaks present/longer when fast.
- **17. Ship model** 🔴 — proper low-poly/sprite ship (replaces the box). **Verify:**
  screenshot; ship silhouette reads as a craft.
- **18. Planet visuals** 🔴 — atmosphere rim glow, banded/flat-shaded surface, optional
  rings, day/night terminator (have), cloud layer. **Verify:** orbit screenshot shows
  atmosphere rim + bands.
- **19. Galaxy backdrop** 🔴 — skybox/cubemap nebula + distant spiral (ref image),
  parallax. **Verify:** screenshot facing away from systems shows nebula, not black.
- **20. Asteroids/debris** 🔴 — instanced fields near some systems (hazard + interest).
  **Verify:** screenshot of a field; collision test (§12).
- **21. Space stations** 🔴 — POIs you can dock at (shops, refuel). **Verify:** screenshot;
  dock prompt appears in range.
- **22. Atmospheric entry FX** 🔴 — heat glow/streaks during descent. **Verify:** `GR_FLY`
  descent screenshot shows entry glow in the entry band.
- **23. Per-star lighting** 🟡→ the system's star is the light source; brightness with
  distance. **Verify:** screenshot near vs far side of a planet shows correct lit side.
- **D1 pixel post-process** ✅ — render the 3D scene to a low-res image (960x540,
  tunable via `pixelate::LORES_W/H`) and upscale nearest to the window for the
  pixel-art look while keeping crisp 3D geometry underneath. Bonus: rendering at
  low-res is *cheaper* than native, which helps framerate. **Verified:** native-res
  screenshot crops show pixel-stepped edges; FPS readout = ~120 (vsync cap).

## 4. Space — HUD / UI (match reference)

- **24. Shield/Hull/Fuel bars** 🔴. **25. Boost meter** 🔴. **26. Radar/minimap** (compass
  + blips) 🔴. **27. Distance(LY)** 🔴. **28. Planet-detected panel** (name/type/dist/
  thumb) 🔴. **29. Objective tracker** 🔴. **30. Crystals counter** 🔴. **31. Progression
  track + boss** 🔴. **32. Speed/throttle + reticle** 🔴.
  **Verify (all HUD):** drive a scenario that sets each value (e.g. take damage, near a
  planet, gain crystals) and screenshot; assert the widget shows the expected value.

## 5. Galaxy / world

- **33. Thousands of stars** 🟡→ scale gen to N stars in sectors. **34. 1–100 bodies/
  system** 🟡→ widen distribution. **35. Biome typing** 🟡→ expand to the biome set
  (§14). **Verify (gen):** determinism + distribution unit tests (counts, ranges).
- **36. On-demand streaming** 🔴 — spawn a system's bodies when within range, despawn
  when far; budget live count. **Verify:** scripted fly past 3 systems; assert entity
  count rises/falls within budget (logged), no leak.
- **37. Procedural naming** 🔴 — seeded names (Veridia Prime…). **Verify:** unit test
  same seed → same names; uniqueness within a region.
- **38. Galaxy map / star chart** 🔴 — open map, pick a star, warp. **Verify:** scripted
  open-map + select + warp; assert position jumped; screenshot map.

## 6. Approach → scan → descent → land

- **39. Detection** 🔴 — bodies within range populate the HUD panel with live distance.
  **Verify:** fly toward a system; assert panel populates; screenshot.
- **40. Scan (C8)** 🔴 — press Scan in range → reveal type/biome/creature hints.
  **Verify:** scripted scan; assert scanned-data resource filled; screenshot panel.
- **41. Seamless descent** 🟡 (terrain resolves) → re-tune to new scale + entry FX.
  **Verify:** `GR_FLY` descent sequence screenshots: dot→disc→surface, continuous.
- **42. Landing (C9)** 🔴 — in-range Land prompt → auto/assisted touchdown → ship
  settles on surface, transitions toward OnFoot-ready. **Verify:** scripted Land press;
  assert state→Descent→Landed, ship velocity ~0 at surface; screenshot ship landed.
- **43. Flight→on-foot handoff** 🔴 — exit-ship prompt spawns the avatar beside the
  ship, switches camera/state to OnFoot. **Verify:** scripted exit; assert avatar
  exists, state=OnFoot, camera is third-person; screenshot.
- **44. Take-off / re-board (C23)** 🔴 — board prompt near ship → despawn avatar, state
  →Space, lift off. **Verify:** scripted board; assert state=Space, avatar gone.

## 7. On-foot mode

- **45. Avatar** 🔴 (sprite/low-poly character). **46. Walk/run + 3rd-person camera** 🔴.
  **47. Crouch (C15)** 🔴. **48. Walkable surface** 🔴 (generated patch around landing).
  **49. Surface extent/streaming** 🔴 (patch grid that streams as you walk). **50.
  Interaction (C16)** 🔴. **51. On-foot scan (C17)** 🔴.
  **Verify (on-foot):** scripted walk path; assert avatar WorldPos moves over terrain
  (stays on surface), camera follows; crouch lowers height; screenshots of each.

## 8. Creatures — encounter & battle

- **52. Wild creatures roam** 🔴 — spawn per biome on the surface, simple wander AI.
  **53. Encounter trigger (C16)** 🔴 — approach → enter Battle state. **54. Battle model = turn-based, in-world 3D framing** ✅DECISION — carry v1's proven
  turn-based combat (move menu, type matchups, deterministic) with billboarded HP
  bars; the capture (weaken → throw Disc → % ring) is the reference moment. Chosen
  for fidelity to the brief and because it's fully self-verifiable via tests; pure
  real-time action is a noted future option.
  **55. Type matchups** 🔴 (Fire/Water/Rock/… multiplier table). **56. Damage/weaken** 🔴.
  **57. Creature stats** 🔴 (HP/SP/level/moves). **58. Enemy AI** 🔴. **59. In-world HP
  bars** 🔴 (billboarded).
  **Verify:** unit tests for type-multiplier + damage + capture-readiness math; scripted
  encounter → battle → weaken; screenshot HP bar at expected %.

## 9. Capture & collection

- **60. Capture Disc + % (C18)** 🔴 — throw → containment ring shows capture chance.
  **61. Capture formula** 🔴 — `f(maxHP, curHP, level, type, item, disc tier)` → %.
  **62. Bait** 🔴 (raises stay/lowers resist). **63. Flash Bomb** 🔴 (stun/flee window).
  **64. Collection/dex** 🔴. **65. Party** (≤6) 🔴. **66. Storage** 🔴.
  **Verify:** unit tests for the capture-% formula across HP/level/item cases (monotonic,
  bounded 0–100); scripted weaken+throw → assert capture roll path; screenshot ring %.

## 10. Items & inventory

- **67. Item system + counts** 🔴 (Capture Disc/Bait/Heal Spray/Revive/Flash Bomb).
  **68. Heal Spray / Revive** 🔴. **69. Inventory UI / quick-slots (C19/C20)** 🔴.
  **70. Acquisition** (find/buy/craft) 🔴.
  **Verify:** unit tests: using Heal Spray restores HP capped at max; Revive only on
  fainted; counts decrement; scripted use + screenshot of the item bar.

## 11. Progression

- **71. Player level + EXP** 🔴 (curve, e.g. EXP_to_next = base·level^1.5). **72. Player
  HP/SP** 🔴. **73. Creature leveling/XP** 🔴. **74. Unlocks by level** 🔴.
  **Verify:** unit tests: XP award crosses a level boundary → level+1, stats grow,
  overflow carries; screenshot the EXP bar (Lv/▮▮ 348/800 style).

## 12. Economy

- **75. Crystals** 🔴 (earn on catch/objective/explore; spend in shops). **76. Shops** 🔴
  (station/planet vendors). **77. Rewards** 🔴.
  **Verify:** unit tests: catch grants crystals; purchase deducts & grants item, blocked
  if insufficient; screenshot counter change.

## 13. Objectives / quests

- **78. Objective system** 🔴 ("Find a new planet" → completes on first scan of an
  undiscovered planet). **79. Progression track + bosses** 🔴 (milestone nodes; boss at
  the skull). **80. Quest log** 🔴.
  **Verify:** unit test: completing the objective's condition flips it done & advances
  the track; screenshot the tracker tick.

## 14. Biomes & content scaling

- **81. Biome defs** 🟡→ {Grasslands, Desert, Ocean, Volcanic, Ice, Rock…} → terrain
  palette + surface props + creature pool. **82. Biome creature pools** 🔴. **83.
  Difficulty by distance from core** 🔴 (level band ∝ sector distance).
  **Verify:** unit tests: biome derives deterministically from seed; creature pool &
  level band match biome/sector; screenshot two distinct biomes.

## 15. On-foot graphics & UI

- **84. Surface environment art** 🔴 (sky gradient, clouds, water, cliffs, trees, the
  ringed-moon-in-sky from the ref). **85. On-foot HUD** 🔴 (player HP/SP/EXP, item bar,
  party panel, tips, control prompts, biome label). **86. Capture VFX** 🔴 (disc throw,
  containment bubble, capture pop).
  **Verify:** screenshot the on-foot scene vs the reference layout; assert each HUD
  widget present and reading live values.

## 16. Audio

- **87. Space audio** 🔴 (engine loop pitched to throttle, ambient, boost whoosh).
  **88. Surface/battle audio** 🔴 (ambient, SFX, capture jingle, music).
  **Verify:** asset-load + play-on-event unit/integration checks (event fires → sound
  handle requested); manual listen is the human gate (flagged).

## 17. Persistence & front-end

- **89. Save/load** 🔴 (serialize position/sector, party, collection, items, currency,
  objectives → file; load restores). **Verify:** round-trip unit test: save→load→state
  equal. **90. Pause + controls** ✅. **91. Settings** 🔴 (audio/controls/graphics).
  **92. Main menu** 🔴 (new game/continue/seed). **Verify:** scripted new-game enters
  Space with a fresh seed; continue restores a save; screenshots.

---

## How verification works going forward (the discipline you asked for)

For every scheduled feature I will, before claiming it done:
1. Add/extend the relevant **test** (pure logic) — and run it.
2. Drive a **scripted scenario** through the dev harness to exercise it in the real app.
3. Capture a **framebuffer screenshot** (or assert state) showing the expected result.
4. Record the evidence (test output + image) in the task report. No "should work."

The one thing I can't self-verify is **subjective feel/audio** — I'll get it as close as
the numbers/screenshots allow and flag those for your playtest.
