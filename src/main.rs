use bevy::prelude::*;

mod battle;
mod creatures;
mod galaxy;
mod hud;
mod onfoot;
mod origin;
mod palette;
mod pause;
mod pixelate;
mod save;
mod ship;
mod spacefx;
mod streaming;
mod targeting;
mod terrain;
mod warp;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Galaxy Roamer".into(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(origin::OriginPlugin)
        .add_plugins(galaxy::GalaxyPlugin)
        .add_plugins(ship::ShipPlugin)
        .add_plugins(streaming::StreamingPlugin)
        .add_plugins(terrain::TerrainPlugin)
        .add_plugins(pause::PauseMenuPlugin)
        .add_plugins(spacefx::SpaceFxPlugin)
        .add_plugins(warp::WarpPlugin)
        .add_plugins(hud::HudPlugin)
        .add_plugins(targeting::TargetingPlugin)
        .add_plugins(onfoot::OnFootPlugin)
        .add_plugins(creatures::CreaturesPlugin)
        .add_plugins(battle::BattlePlugin)
        .add_plugins(pixelate::PixelatePlugin)
        .add_plugins(save::SavePlugin)
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin)
        .insert_resource(ClearColor(Color::srgb(0.01, 0.01, 0.03)))
        // Faint blue fill so planets' shadowed sides read as lit spheres rather
        // than pure black when you warp in from an arbitrary angle.
        .insert_resource(AmbientLight {
            color: Color::srgb(0.55, 0.65, 0.9),
            brightness: 320.0,
        })
        .add_systems(Startup, setup);

    // On-screen gamepad tester (GR_PADTEST=1): shows live controller input so an
    // unrecognized pad's buttons/axes can be diagnosed.
    if std::env::var("GR_PADTEST").is_ok() {
        app.init_resource::<PadRaw>()
            .add_systems(Startup, pad_debug_setup)
            .add_systems(Update, (pad_raw_log, pad_debug_update).chain());
    }

    // DEV harness (env-gated, never runs in normal play): captures a framebuffer
    // screenshot so the headless dev loop can actually see the rendered scene.
    if std::env::var("GR_SHOT").is_ok() {
        app.add_systems(Update, dev_screenshot);
    }
    // DEV autopilot: drive the ship through its real controls (thrust/boost toward
    // the nearest planet) and capture a flight sequence. Gated by GR_FLY=<prefix>.
    if std::env::var("GR_FLY").is_ok() {
        app.add_systems(
            Update,
            dev_autopilot
                .after(ship::systems::flight_input)
                .before(ship::systems::ship_move),
        );
    }
    // DEV turn test: hold position and yaw left, then right, capturing the view at
    // each so the steering direction can be checked. Gated by GR_TURN=<prefix>.
    if std::env::var("GR_TURN").is_ok() {
        app.add_systems(
            Update,
            dev_turn_test
                .after(ship::systems::flight_input)
                .before(ship::systems::ship_move),
        );
    }

    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        DirectionalLight { illuminance: 10_000.0, shadows_enabled: false, ..default() },
        Transform::from_xyz(1.0, 2.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

#[derive(Component)]
struct PadDebugText;

fn pad_debug_setup(mut commands: Commands) {
    commands.spawn((
        PadDebugText,
        Text::new("GAMEPAD TEST: waiting..."),
        TextFont { font_size: 20.0, ..default() },
        TextColor(Color::srgb(1.0, 0.9, 0.3)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        GlobalZIndex(2000),
    ));
}

#[derive(Resource, Default)]
struct PadRaw {
    last_btn: String,
    last_axis: String,
}

fn pad_raw_log(
    mut btn: EventReader<bevy::input::gamepad::RawGamepadButtonChangedEvent>,
    mut axis: EventReader<bevy::input::gamepad::RawGamepadAxisChangedEvent>,
    mut raw: ResMut<PadRaw>,
) {
    for e in btn.read() {
        raw.last_btn = format!("{:?} = {:.2}", e.button, e.value);
    }
    for e in axis.read() {
        if e.value.abs() > 0.3 {
            raw.last_axis = format!("{:?} = {:.2}", e.axis, e.value);
        }
    }
}

fn pad_debug_update(
    gamepads: Query<&Gamepad>,
    raw: Res<PadRaw>,
    mut text: Query<&mut Text, With<PadDebugText>>,
) {
    let Ok(mut t) = text.get_single_mut() else { return };
    let n = gamepads.iter().count();
    let mut s = format!("GAMEPADS DETECTED: {n}\n");
    if let Some(g) = gamepads.iter().next() {
        use bevy::input::gamepad::{GamepadAxis as A, GamepadButton as B};
        let mut pressed = Vec::new();
        for b in [
            B::South, B::East, B::West, B::North, B::DPadUp, B::DPadDown, B::DPadLeft,
            B::DPadRight, B::LeftTrigger, B::RightTrigger, B::LeftTrigger2, B::RightTrigger2,
            B::Start, B::Select, B::LeftThumb, B::RightThumb,
        ] {
            if g.pressed(b) {
                pressed.push(format!("{b:?}"));
            }
        }
        s += &format!("PRESSED: {}\n", pressed.join(" "));
        s += &format!(
            "LX {:.2} LY {:.2}   RX {:.2} RY {:.2}",
            g.get(A::LeftStickX).unwrap_or(0.0),
            g.get(A::LeftStickY).unwrap_or(0.0),
            g.get(A::RightStickX).unwrap_or(0.0),
            g.get(A::RightStickY).unwrap_or(0.0),
        );
    } else {
        s += "(no gamepad entity — not detected by the engine)";
    }
    s += &format!("\nRAW last button: {}", raw.last_btn);
    s += &format!("\nRAW last axis:   {}", raw.last_axis);
    t.0 = s;
}

/// DEV-only: optionally teleport the ship (GR_SHIPZ), wait for the scene to settle,
/// save a screenshot to GR_SHOT, then exit. Gated by the GR_SHOT env var.
fn dev_screenshot(
    mut frame: Local<u32>,
    mut commands: Commands,
    mut ship: Query<(&mut origin::WorldPos, &mut ship::ShipControl, &mut Transform), With<ship::PlayerShip>>,
    vel_q: Query<&ship::ShipVelocity, With<ship::PlayerShip>>,
    warp_targets: Res<galaxy::WarpTargets>,
    mut next_mode: ResMut<NextState<onfoot::Mode>>,
    mut autopilot: ResMut<targeting::Autopilot>,
    mut next_phase: ResMut<NextState<battle::Phase>>,
    mut battle_res: ResMut<battle::Battle>,
    mut landed_biome: ResMut<onfoot::LandedBiome>,
    creatures_q: Query<(Entity, &creatures::Creature)>,
    diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
    mut exit: EventWriter<AppExit>,
) {
    // GR_BATTLE: once creatures exist, point the battle at a real one (for the camera).
    if *frame == 6 && std::env::var("GR_BATTLE").is_ok() {
        if let Some((e, c)) = creatures_q.iter().next() {
            battle_res.enemy = Some(e);
            battle_res.kind = Some(c.kind);
            battle_res.level = c.level;
            battle_res.hp = c.hp;
            battle_res.max_hp = c.max_hp;
        }
    }
    use bevy::render::view::screenshot::{save_to_disk, Screenshot};
    if *frame == 2 && std::env::var("GR_AP").is_ok() {
        // Engage warp-to-target once targets have been populated.
        autopilot.on = true;
    }
    if *frame == 0 {
        if std::env::var("GR_FOOT").is_ok() || std::env::var("GR_BATTLE").is_ok() {
            next_mode.set(onfoot::Mode::OnFoot);
        }
        if let Ok(bi) = std::env::var("GR_BIOME") {
            landed_biome.0 = match bi.as_str() {
                "desert" => onfoot::Biome::Desert,
                "tundra" => onfoot::Biome::Tundra,
                "volcanic" => onfoot::Biome::Volcanic,
                "alien" => onfoot::Biome::Alien,
                _ => onfoot::Biome::Grass,
            };
        }
        if std::env::var("GR_BATTLE").is_ok() {
            next_phase.set(battle::Phase::Battle);
            *battle_res = battle::Battle {
                enemy: None,
                kind: Some(creatures::CreatureKind::Flarehog),
                level: 12,
                hp: 30.0,
                max_hp: 52.0,
                bait_bonus: 0.0,
                hit_timer: 0.0,
                enemy_stunned: false,
            };
        }
        let g = galaxy::gen::generate(galaxy::WORLD_SEED);
        // GR_NEAR=<index>: place the ship just above that planet's surface.
        if let Ok(idx) = std::env::var("GR_NEAR").and_then(|s| s.parse::<usize>().map_err(|_| std::env::VarError::NotPresent)) {
            if let (Some(p), Ok((mut wp, _, _))) = (g.planets.get(idx), ship.get_single_mut()) {
                wp.0 = p.pos + bevy::math::DVec3::new(0.0, 0.0, p.radius + 60_000.0);
            }
        }
        // GR_WARP: jump to the first far system (verifies warp arrival).
        if std::env::var("GR_WARP").is_ok() {
            if let (Some(&t), Ok((mut wp, mut ctl, mut tf))) =
                (warp_targets.positions.first(), ship.get_single_mut())
            {
                wp.0 = warp::warp_arrival(t);
                ctl.yaw = 0.0;
                ctl.pitch = 0.0;
                tf.rotation = ship::physics::ship_rotation(0.0, 0.0);
            }
        }
        for (i, p) in g.planets.iter().enumerate() {
            eprintln!("PLANET {i}: pos={:?} radius={:.0} kind={:?}", p.pos, p.radius, p.kind);
        }
    }
    if *frame == 90 {
        if let Ok((wp, _, _)) = ship.get_single() {
            let v = vel_q.get_single().map(|v| v.0).unwrap_or(Vec3::ZERO);
            eprintln!("SHIP WorldPos = {:?}  vel = {:?} ({:.0} m/s)", wp.0, v, v.length());
        }
        let path = std::env::var("GR_SHOT").unwrap();
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
        if let Some(fps) = diagnostics
            .get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS)
            .and_then(|d| d.smoothed())
        {
            eprintln!("FPS at capture: {fps:.1}");
        }
    }
    if *frame == 120 {
        exit.send(AppExit::Success);
    }
    *frame += 1;
}

/// DEV-only autopilot: steers and throttles the ship straight at the nearest
/// planet (overriding the headless cursor drift) so it actually reaches the
/// surface, then captures the approach→descent to `GR_FLY`_<n>.png. Runs after
/// flight_input and before ship_move, so it has the last word on heading/velocity
/// and the real ship_move/collision/streaming/terrain systems do the rest.
fn dev_autopilot(
    mut shot_idx: Local<usize>,
    mut frame: Local<u32>,
    mut commands: Commands,
    mut ship: Query<(&origin::WorldPos, &mut ship::ShipVelocity, &mut Transform), With<ship::PlayerShip>>,
    planets: Query<(&origin::WorldPos, &galaxy::PlanetBody)>,
    mut exit: EventWriter<AppExit>,
) {
    use bevy::render::view::screenshot::{save_to_disk, Screenshot};
    *frame += 1;

    let Ok((swp, mut vel, mut tf)) = ship.get_single_mut() else { return };
    // Pick the nearest planet and head straight for it.
    let mut nearest: Option<(f64, bevy::math::DVec3)> = None;
    for (p, body) in &planets {
        let surf = (p.0 - swp.0).length() - body.radius as f64;
        if nearest.map_or(true, |(b, _)| surf < b) {
            nearest = Some((surf, p.0));
        }
    }
    let Some((surf, ppos)) = nearest else { return };

    let dir = (ppos - swp.0).as_vec3().normalize_or_zero();
    let (yaw, pitch) = ship::physics::look_yaw_pitch(dir);
    tf.rotation = ship::physics::ship_rotation(yaw, pitch);
    // Fast while far, gentle on final approach so the slide-collision settles (m/s).
    let speed = if surf > 2.0e6 { 4.0e6 } else { 2.0e5 };
    vel.0 = dir * speed;

    // Capture by surface distance in metres (framerate-independent): far→descent→landed.
    let prefix = std::env::var("GR_FLY").unwrap_or_else(|_| "/tmp/fly".into());
    let thresholds = [3.0e7, 1.0e7, 3.0e6, 1.0e6, 3.0e5, 5.0e4];
    while *shot_idx < thresholds.len() && surf <= thresholds[*shot_idx] {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(format!("{prefix}_{}.png", *shot_idx)));
        eprintln!("FLY shot {}: surface dist = {surf:.0} km", *shot_idx);
        *shot_idx += 1;
    }
    if *shot_idx >= thresholds.len() || *frame > 4000 {
        exit.send(AppExit::Success);
    }
}

/// DEV turn test: keep the ship stationary and swing its heading left, back to
/// centre, then right, capturing each once the chase camera has settled. Lets us
/// confirm left/right steering pans the view the correct way. Gated by GR_TURN.
fn dev_turn_test(
    mut frame: Local<u32>,
    mut commands: Commands,
    mut ship: Query<(&mut ship::ShipVelocity, &mut ship::ShipControl, &mut Transform), With<ship::PlayerShip>>,
    mut exit: EventWriter<AppExit>,
) {
    use bevy::render::view::screenshot::{save_to_disk, Screenshot};
    let f = *frame;
    *frame += 1;

    let Ok((mut vel, mut ctl, mut tf)) = ship.get_single_mut() else { return };
    vel.0 = Vec3::ZERO; // hover in place

    // Yaw schedule (positive yaw faces -X = left; negative faces +X = right).
    // Long holds so the smoothed chase camera fully settles before each capture.
    let yaw = if f < 90 {
        0.0
    } else if f < 240 {
        0.7 // turned LEFT
    } else if f < 390 {
        0.0
    } else {
        -0.7 // turned RIGHT
    };
    ctl.yaw = yaw;
    ctl.pitch = 0.0;
    tf.rotation = ship::physics::ship_rotation(yaw, 0.0);

    let prefix = std::env::var("GR_TURN").unwrap_or_else(|_| "/tmp/turn".into());
    let shots = [(80u32, "0_forward"), (230, "1_left"), (380, "2_forward"), (520, "3_right")];
    for (at, name) in shots {
        if f == at {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(format!("{prefix}_{name}.png")));
            eprintln!("TURN shot {name} at frame {f}, yaw={yaw:.2}");
        }
    }
    if f >= 560 {
        exit.send(AppExit::Success);
    }
}
