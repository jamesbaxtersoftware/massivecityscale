use bevy::prelude::*;
use super::{PlayerShip, ShipVelocity, ShipControl, SHIP_RADIUS};

/// Spawn the player ship near the home planet if one does not already exist.
/// A dart body (cone) with an emissive engine-glow accent behind it.
pub fn spawn_ship(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    existing: Query<(), With<PlayerShip>>,
) {
    if !existing.is_empty() {
        return; // already spawned (re-entering Exploring after a battle)
    }

    // Home planet starts near world origin; place the ship just above its
    // north pole, facing tangentially (-Z).
    let start = Vec3::new(0.0, 80.0, 600.0);

    let body_mesh = meshes.add(Cone { radius: SHIP_RADIUS * 0.6, height: SHIP_RADIUS * 2.0 }.mesh());
    let body_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.75, 0.78, 0.85),
        perceptual_roughness: 0.4,
        metallic: 0.6,
        ..default()
    });
    let glow_mesh = meshes.add(Sphere::new(SHIP_RADIUS * 0.4).mesh().uv(8, 6));
    let glow_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.7, 1.0),
        emissive: LinearRgba::rgb(0.2, 0.6, 1.0),
        unlit: true,
        ..default()
    });

    commands.spawn((
        PlayerShip { radius: SHIP_RADIUS },
        ShipVelocity::default(),
        ShipControl::default(), // yaw=0, pitch=0 → faces -Z, upright
        Transform::from_translation(start).looking_to(Vec3::NEG_Z, Vec3::Y),
        GlobalTransform::default(),
        Visibility::Visible,
    )).with_children(|ship| {
        // Cone's +Y is its tip; rotate so the tip points along -Z (forward).
        ship.spawn((
            Mesh3d(body_mesh),
            MeshMaterial3d(body_mat),
            Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ));
        ship.spawn((
            Mesh3d(glow_mesh),
            MeshMaterial3d(glow_mat),
            Transform::from_xyz(0.0, 0.0, SHIP_RADIUS * 1.1), // behind the tip
        ));
    });
}
