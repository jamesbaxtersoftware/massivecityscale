use bevy::prelude::*;
use super::{CreatureType, Species};

/// Mesh/material handles for a creature of a given type. Accent is the flame
/// spike (Fire) or fin (Water) attached above/behind the body.
pub struct CreatureVisual {
    pub body_mesh:    Handle<Mesh>,
    pub body_mat:     Handle<StandardMaterial>,
    pub accent_mesh:  Handle<Mesh>,
    pub accent_mat:   Handle<StandardMaterial>,
    pub accent_xform: Transform,
    /// Approximate bounding radius for click spheres and HP-bar placement.
    pub radius: f32,
}

pub fn build_visual(
    t: CreatureType,
    scale: f32,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> CreatureVisual {
    let radius = scale;
    match t {
        CreatureType::Fire => CreatureVisual {
            body_mesh: meshes.add(Sphere::new(scale).mesh().uv(16, 12)),
            body_mat: materials.add(StandardMaterial {
                base_color: Color::srgb(0.95, 0.35, 0.10),
                emissive: LinearRgba::rgb(0.6, 0.2, 0.0),
                ..default()
            }),
            accent_mesh: meshes.add(Cone { radius: scale * 0.5, height: scale * 1.4 }.mesh()),
            accent_mat: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.8, 0.2),
                emissive: LinearRgba::rgb(0.8, 0.5, 0.0),
                unlit: true,
                ..default()
            }),
            accent_xform: Transform::from_xyz(0.0, scale * 1.1, 0.0),
            radius,
        },
        CreatureType::Water => CreatureVisual {
            body_mesh: meshes.add(Sphere::new(scale).mesh().uv(16, 12)),
            body_mat: materials.add(StandardMaterial {
                base_color: Color::srgb(0.15, 0.45, 0.9),
                perceptual_roughness: 0.3,
                ..default()
            }),
            accent_mesh: meshes.add(Cuboid::new(scale * 0.2, scale * 1.2, scale * 0.8)),
            accent_mat: materials.add(StandardMaterial {
                base_color: Color::srgb(0.5, 0.8, 1.0),
                perceptual_roughness: 0.2,
                ..default()
            }),
            accent_xform: Transform::from_xyz(0.0, scale * 0.9, -scale * 0.3),
            radius,
        },
    }
}

/// Spawn a creature's body + accent as children of `parent`, with the body at
/// `local`. The accent is parented to the body so it follows it.
pub fn spawn_creature_visual(
    commands: &mut Commands,
    parent: Entity,
    local: Transform,
    vis: &CreatureVisual,
) {
    commands.entity(parent).with_children(|c| {
        c.spawn((
            Mesh3d(vis.body_mesh.clone()),
            MeshMaterial3d(vis.body_mat.clone()),
            local,
        )).with_children(|b| {
            b.spawn((
                Mesh3d(vis.accent_mesh.clone()),
                MeshMaterial3d(vis.accent_mat.clone()),
                vis.accent_xform,
            ));
        });
    });
}
