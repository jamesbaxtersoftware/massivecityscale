use bevy::prelude::*;

mod galaxy;
mod origin;
mod palette;
mod pause;
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
        .insert_resource(ClearColor(Color::srgb(0.01, 0.01, 0.03)))
        .add_systems(Startup, setup);

    // DEV harness (env-gated, never runs in normal play): captures a framebuffer
    // screenshot so the headless dev loop can actually see the rendered scene.
    if std::env::var("GR_SHOT").is_ok() {
        app.add_systems(Update, dev_screenshot);
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
    }
    if *frame == 120 {
        exit.send(AppExit::Success);
    }
    *frame += 1;
}
