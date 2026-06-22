pub mod gen;

use bevy::prelude::*;
use bevy::math::DVec3;
use bevy::ecs::schedule::IntoSystemConfigs;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use crate::origin::{FrameSet, WorldPos};
use crate::palette::{planet_color, STAR_COLOR};
use crate::ship::PlayerShip;
use crate::streaming::{LodBody, LodTier};

pub const WORLD_SEED: u64 = 42;
/// One light-year in metres.
pub const LIGHT_YEAR: f64 = 9.4607e15;

#[derive(Component)]
pub struct StarBody;

#[derive(Component)]
pub struct PlanetBody {
    pub radius: f32,
    pub kind: gen::PlanetType,
}

/// Far backdrop (stars, nebulae): kept at a fixed direction+distance from the ship
/// each frame so the sky surrounds the player everywhere — including after a warp.
#[derive(Component)]
pub struct Backdrop {
    pub dir: Vec3,
    pub dist: f64,
}

/// Warp destinations (distant star systems), in light-year-scale world coords.
#[derive(Resource, Default)]
pub struct WarpTargets {
    pub positions: Vec<DVec3>,
    pub current: usize,
}

pub struct GalaxyPlugin;

impl Plugin for GalaxyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WarpTargets>()
            .add_systems(Startup, spawn_galaxy)
            .add_systems(Update, recenter_backdrop.in_set(FrameSet::Origin));
    }
}

/// Pin every backdrop body to `ship + dir * dist` so stars/nebulae always surround
/// the ship. Runs in `FrameSet::Origin` (after the ship has moved, before sync).
fn recenter_backdrop(
    ship: Query<&WorldPos, With<PlayerShip>>,
    mut backdrop: Query<(&Backdrop, &mut WorldPos), Without<PlayerShip>>,
) {
    let Ok(s) = ship.get_single() else { return };
    for (b, mut wp) in &mut backdrop {
        wp.0 = s.0 + b.dir.as_dvec3() * b.dist;
    }
}

fn spawn_galaxy(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut warp: ResMut<WarpTargets>,
) {
    let data = gen::generate(WORLD_SEED);

    // Shared star mesh+material → Bevy batches identical handles (cheap "instancing").
    let star_mesh = meshes.add(Sphere::new(1.0).mesh().ico(1).unwrap());
    let star_mat = materials.add(StandardMaterial {
        base_color: STAR_COLOR,
        emissive: LinearRgba::rgb(2.0, 1.9, 1.5),
        unlit: true,
        ..default()
    });
    for star in &data.stars {
        // A 1 km sphere is sub-pixel at galactic range, so scale each star by its
        // distance to subtend a few pixels — a visible backdrop point. Scale rides
        // on the Transform (sync_transforms only writes translation), so all stars
        // keep sharing one mesh+material and stay batched.
        let dist = star.pos.length();
        let size = (dist as f32 * 0.0025).max(400.0);
        commands.spawn((
            StarBody,
            Backdrop { dir: star.pos.normalize_or_zero().as_vec3(), dist },
            WorldPos(star.pos),
            Mesh3d(star_mesh.clone()),
            MeshMaterial3d(star_mat.clone()),
            Transform::from_scale(Vec3::splat(size)),
        ));
    }

    // Distant nebula clouds: a handful of large, dim, translucent emissive spheres
    // far out in coloured patches so space reads as a nebula rather than a black
    // void. Deterministic from the seed; act as a fixed backdrop like the stars.
    let neb_mesh = meshes.add(Sphere::new(1.0).mesh().ico(2).unwrap());
    let neb_colors = [
        LinearRgba::rgb(0.45, 0.12, 0.65), // purple
        LinearRgba::rgb(0.10, 0.25, 0.70), // blue
        LinearRgba::rgb(0.70, 0.18, 0.45), // magenta
        LinearRgba::rgb(0.10, 0.45, 0.55), // teal
    ];
    let mut nrng = ChaCha8Rng::seed_from_u64(WORLD_SEED ^ 0x4e45_4255_4c41);
    for i in 0..14 {
        let dir = DVec3::new(
            nrng.gen_range(-1.0..1.0),
            nrng.gen_range(-1.0..1.0),
            nrng.gen_range(-1.0..1.0),
        )
        .normalize_or_zero();
        // Far and large so they read as diffuse background haze, not foreground balls.
        let dist = nrng.gen_range(2.2e9..3.2e9);
        let radius = nrng.gen_range(5.0e8..1.1e9) as f32;
        let c = neb_colors[i % neb_colors.len()];
        commands.spawn((
            Backdrop { dir: dir.as_vec3(), dist },
            WorldPos(dir * dist),
            Mesh3d(neb_mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(c.red, c.green, c.blue, 0.04),
                emissive: LinearRgba::rgb(c.red * 0.16, c.green * 0.16, c.blue * 0.16),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            Transform::from_scale(Vec3::splat(radius)),
        ));
    }

    // Shared unit sphere + translucent shell material for atmosphere rims; scaled
    // per planet via the child Transform so the handle stays shared.
    let atmo_mesh = meshes.add(Sphere::new(1.0).mesh().ico(4).unwrap());
    let atmo_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.35, 0.6, 1.0, 0.16),
        emissive: LinearRgba::rgb(0.15, 0.35, 0.7),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        // Show the far side of the shell so it reads as a rim halo around the disc.
        cull_mode: Some(bevy::render::render_resource::Face::Front),
        ..default()
    });

    for planet in &data.planets {
        let r = planet.radius as f32;
        commands
            .spawn((
                PlanetBody { radius: r, kind: planet.kind },
                WorldPos(planet.pos),
                Mesh3d(meshes.add(Sphere::new(r).mesh().ico(4).unwrap())),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: planet_color(planet.kind),
                    ..default()
                })),
                Transform::default(),
                Visibility::default(),
                LodBody { tier: LodTier::Point },
            ))
            .with_children(|p| {
                // Atmosphere shell ~6% larger than the planet.
                p.spawn((
                    Mesh3d(atmo_mesh.clone()),
                    MeshMaterial3d(atmo_mat.clone()),
                    Transform::from_scale(Vec3::splat(r * 1.06)),
                ));
            });
    }

    // Distant star systems (warp destinations) at light-year scale: a bright sun
    // plus a planet to arrive at. Far enough to be culled until you warp in.
    let far_dirs = [
        DVec3::new(0.6, 0.15, -0.8),
        DVec3::new(-0.9, 0.1, -0.4),
        DVec3::new(0.2, -0.3, 0.95),
    ];
    let far_kinds = [gen::PlanetType::Water, gen::PlanetType::Rock, gen::PlanetType::Fire];
    let sun_mesh = meshes.add(Sphere::new(1.0).mesh().ico(3).unwrap());
    for (i, d) in far_dirs.iter().enumerate() {
        let sun_pos = d.normalize() * ((2.0 + i as f64 * 2.0) * LIGHT_YEAR);
        commands.spawn((
            WorldPos(sun_pos),
            Mesh3d(sun_mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.85, 0.4),
                emissive: LinearRgba::rgb(6.0, 4.5, 1.5),
                unlit: true,
                ..default()
            })),
            Transform::from_scale(Vec3::splat(8.0e8)),
        ));
        let r = 4.0e6_f32;
        // Planet orbits ~10 million km out, so the sun reads as a bright disc, not a wall.
        let ppos = sun_pos + DVec3::new(1.0e10, 0.0, 0.0);
        commands
            .spawn((
                PlanetBody { radius: r, kind: far_kinds[i] },
                WorldPos(ppos),
                Mesh3d(meshes.add(Sphere::new(r).mesh().ico(4).unwrap())),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: planet_color(far_kinds[i]),
                    ..default()
                })),
                Transform::default(),
                Visibility::default(),
                LodBody { tier: LodTier::Point },
            ))
            .with_children(|p| {
                p.spawn((
                    Mesh3d(atmo_mesh.clone()),
                    MeshMaterial3d(atmo_mat.clone()),
                    Transform::from_scale(Vec3::splat(r * 1.06)),
                ));
            });
        warp.positions.push(ppos);
    }
}
