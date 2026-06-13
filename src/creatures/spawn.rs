use bevy::prelude::*;
use rand::SeedableRng;
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use super::{CreatureType, Species};
use super::{Creature, PlanetType, WildMonster, ClickSphere};
use crate::renderer::solar::CelestialBody;
use crate::lod::LodRange;
use crate::renderer::scale_consts::LOD_SOLAR;

const MONSTERS_PER_PLANET: usize = 3;
const MONSTER_SCALE: f32 = 28.0; // km

pub fn spawn_wild_monsters(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    planets: Query<(Entity, &PlanetType, &CelestialBody)>,
) {
    let mut rng = ChaCha8Rng::seed_from_u64(0x_C0FF_EE15_600D);
    for (planet, ptype, body) in &planets {
        let species = Species::of_type(ptype.0);
        let vis = build_visual(ptype.0, MONSTER_SCALE, &mut meshes, &mut materials);
        for _ in 0..MONSTERS_PER_PLANET {
            let dir = Vec3::new(
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
            ).normalize_or_zero();
            let local = Transform::from_translation(dir * (body.radius + MONSTER_SCALE));
            let level: u32 = rng.gen_range(2..=5);
            let monster = commands.spawn((
                Creature::new(species, level),
                WildMonster,
                ClickSphere { radius: MONSTER_SCALE * 1.5 },
                local,
                GlobalTransform::default(),
                Visibility::Inherited,
                LodRange { min_scale: 0.0, max_scale: LOD_SOLAR.1 },
            )).id();
            commands.entity(planet).add_child(monster);
            spawn_creature_visual(&mut commands, monster, Transform::IDENTITY, &vis);
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use crate::creatures::{CreaturesPlugin, PlanetType, WildMonster, CreatureType};
    use crate::renderer::solar::CelestialBody;

    #[test]
    fn wild_monsters_spawn_as_children_of_a_planet() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
           .add_plugins(bevy::state::app::StatesPlugin)
           .add_plugins(AssetPlugin::default())
           .init_asset::<Mesh>()
           .init_asset::<StandardMaterial>()
           .add_plugins(CreaturesPlugin);

        let planet = app.world_mut().spawn((
            Transform::from_xyz(1000.0, 0.0, 0.0),
            GlobalTransform::default(),
            PlanetType(CreatureType::Fire),
            CelestialBody { radius: 200.0 },
        )).id();

        app.world_mut().run_system_once(spawn_wild_monsters).unwrap();

        let count = app.world_mut()
            .query::<(&WildMonster, &Parent)>()
            .iter(app.world())
            .filter(|(_, p)| p.get() == planet)
            .count();
        assert!(count > 0, "expected wild monsters parented to the planet");
    }
}
