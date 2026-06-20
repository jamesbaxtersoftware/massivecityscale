use bevy::prelude::*;

mod galaxy;
mod origin;
mod palette;
mod pause;
mod pixelate;
mod ship;
mod streaming;
mod terrain;

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
        .add_plugins(pixelate::PixelatePlugin)
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin)
        .insert_resource(ClearColor(Color::srgb(0.01, 0.01, 0.03)))
        .add_systems(Startup, setup);

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

/// DEV-only: optionally teleport the ship (GR_SHIPZ), wait for the scene to settle,
/// save a screenshot to GR_SHOT, then exit. Gated by the GR_SHOT env var.
fn dev_screenshot(
    mut frame: Local<u32>,
    mut commands: Commands,
    mut ship: Query<&mut origin::WorldPos, With<ship::PlayerShip>>,
    diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
    mut exit: EventWriter<AppExit>,
) {
    use bevy::render::view::screenshot::{save_to_disk, Screenshot};
    if *frame == 0 {
        let g = galaxy::gen::generate(galaxy::WORLD_SEED);
        // GR_NEAR=<index>: place the ship just above that planet's surface (for a
        // descent shot). Otherwise leave it at the spawn for a start shot.
        if let Ok(idx) = std::env::var("GR_NEAR").and_then(|s| s.parse::<usize>().map_err(|_| std::env::VarError::NotPresent)) {
            if let (Some(p), Ok(mut wp)) = (g.planets.get(idx), ship.get_single_mut()) {
                wp.0 = p.pos + bevy::math::DVec3::new(0.0, 0.0, p.radius + 90.0);
            }
        }
        for (i, p) in g.planets.iter().enumerate() {
            eprintln!("PLANET {i}: pos={:?} radius={:.0} kind={:?}", p.pos, p.radius, p.kind);
        }
    }
    if *frame == 90 {
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
    // Fast while far, gentle on final approach so the slide-collision settles.
    let speed = if surf > 400.0 { 1000.0 } else { 120.0 };
    vel.0 = dir * speed;

    // Capture by surface distance (framerate-independent): far → descent → landed.
    let prefix = std::env::var("GR_FLY").unwrap_or_else(|_| "/tmp/fly".into());
    let thresholds = [3000.0, 1500.0, 600.0, 250.0, 110.0, 30.0];
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
