use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use rand::SeedableRng;
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use crate::lod::LodRange;
use crate::renderer::scale_consts::{LOD_SOLAR, SPHERE_CENTER_Y};
use crate::camera::zoom::OrbitState;

const N_STARS: usize       = 4_000;
const STAR_SPHERE_R: f32   = 19_000.0; // km — large enough that frustum corners never escape it
const STAR_SIZE_MIN: f32   = 12.0;     // km — looks like 1-2 px at solar zoom
const STAR_SIZE_MAX: f32   = 35.0;
const LOD_STARS: (f32, f32) = (1.5, 20.0); // appear from mid-zoom outward

// ω = KEPLER_K / radius^1.5  (50% slower than original 18 730)
const KEPLER_K: f32 = 9_365.0;
const SUN_POS: Vec3 = Vec3::new(-5000.0, 0.0, 0.0);

// ── Orbital body component ────────────────────────────────────────────────────

/// Marker for the home planet's root entity so orbit_bodies can update the camera pivot.
#[derive(Component)]
pub struct HomePlanet;

/// Marker for the star-field mesh so it can be recentred on the camera every frame.
#[derive(Component)]
pub struct StarSphere;

#[derive(Component)]
pub struct OrbitalBody {
    center:      Vec3,
    radius:      f32,
    pub angle:   f32,  // current angle, radians
    speed:       f32,  // rad/s, derived from Kepler
    inclination: f32,  // tilt of orbital plane from XZ, radians
}

impl OrbitalBody {
    pub fn new(radius: f32, initial_angle: f32, inclination: f32) -> Self {
        Self {
            center: SUN_POS,
            radius,
            angle: initial_angle,
            speed: KEPLER_K / radius.powf(1.5),
            inclination,
        }
    }

    pub fn position(&self) -> Vec3 {
        let (sin_t, cos_t) = self.angle.sin_cos();
        let sin_i = self.inclination.sin();
        let cos_i = self.inclination.cos();
        self.center + Vec3::new(
            self.radius * cos_t,
            self.radius * sin_t * sin_i,
            self.radius * sin_t * cos_i,
        )
    }
}

// ── Update system ─────────────────────────────────────────────────────────────

pub fn orbit_bodies(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut OrbitalBody, Option<&HomePlanet>)>,
    mut orbit_state: ResMut<OrbitState>,
) {
    for (mut transform, mut orbit, home) in &mut query {
        orbit.angle += orbit.speed * time.delta_secs();
        let pos = orbit.position();
        transform.translation = pos;
        // rotation left untouched so rings keep their world-space tilt
        if home.is_some() {
            // Sphere center sits SPHERE_CENTER_Y below the root entity
            orbit_state.pivot = pos + Vec3::new(0.0, SPHERE_CENTER_Y, 0.0);
        }
    }
}

// ── Spawn ─────────────────────────────────────────────────────────────────────

struct PlanetDef {
    orbit_radius:  f32,
    initial_angle: f32,
    inclination:   f32,
    sphere_radius: f32,
    color:         Color,
    has_ring:      bool,
}

pub fn spawn_solar_system(
    mut commands: Commands,
    mut meshes:   ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Point light at the sun — illuminates all planets in 3D. Spawned separately so its
    // visibility is independent of the sun mesh's LOD range.
    commands.spawn((
        PointLight {
            color:             Color::srgb(1.0, 0.95, 0.85), // warm white sunlight
            intensity:         3e11,                          // lumens; gives ~1000 lux at ~5000 km
            range:             16_000.0,                      // km — covers all orbits
            shadows_enabled:   false,
            ..default()
        },
        Transform::from_translation(SUN_POS),
    ));

    // Sun mesh — unlit so it shows its full bright colour regardless of lighting
    // and stays in the opaque depth-tested pass (no HDR emissive bleed).
    let sun_mesh = meshes.add(Sphere::new(800.0).mesh().uv(32, 18));
    let sun_mat  = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.95, 0.4),
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(sun_mesh),
        MeshMaterial3d(sun_mat),
        Transform::from_translation(SUN_POS),
        LodRange { min_scale: LOD_SOLAR.0, max_scale: LOD_SOLAR.1 },
        Visibility::Hidden,
    ));

    //              orbit_radius    angle   incl    sphere_radius  color
    let planets = [
        PlanetDef { orbit_radius:  1_800.0, initial_angle: 0.80, inclination: 0.05, sphere_radius: 180.0, color: Color::srgb(0.95, 0.28, 0.05), has_ring: false }, // lava
        PlanetDef { orbit_radius:  2_600.0, initial_angle: 2.10, inclination: 0.08, sphere_radius: 260.0, color: Color::srgb(0.80, 0.62, 0.20), has_ring: false }, // desert
        PlanetDef { orbit_radius:  3_400.0, initial_angle: 4.50, inclination: 0.03, sphere_radius: 200.0, color: Color::srgb(0.55, 0.18, 0.12), has_ring: false }, // dark red rocky
        PlanetDef { orbit_radius:  4_300.0, initial_angle: 1.30, inclination: 0.12, sphere_radius: 300.0, color: Color::srgb(0.10, 0.38, 0.90), has_ring: false }, // ocean
        PlanetDef { orbit_radius:  5_200.0, initial_angle: 3.70, inclination: 0.15, sphere_radius: 220.0, color: Color::srgb(0.55, 0.15, 0.80), has_ring: false }, // purple exotic
        PlanetDef { orbit_radius:  6_300.0, initial_angle: 5.80, inclination: 0.06, sphere_radius: 380.0, color: Color::srgb(0.35, 0.82, 0.88), has_ring: false }, // ice giant
        PlanetDef { orbit_radius:  7_400.0, initial_angle: 2.90, inclination: 0.09, sphere_radius: 320.0, color: Color::srgb(0.72, 0.60, 0.42), has_ring: true  }, // ringed
        PlanetDef { orbit_radius:  8_600.0, initial_angle: 1.00, inclination: 0.04, sphere_radius: 500.0, color: Color::srgb(0.78, 0.52, 0.22), has_ring: false }, // gas giant
        PlanetDef { orbit_radius:  9_500.0, initial_angle: 4.20, inclination: 0.18, sphere_radius: 140.0, color: Color::srgb(0.82, 0.90, 0.95), has_ring: false }, // ice rocky
        PlanetDef { orbit_radius: 10_800.0, initial_angle: 0.30, inclination: 0.07, sphere_radius: 270.0, color: Color::srgb(0.15, 0.58, 0.22), has_ring: false }, // forest
    ];

    for p in &planets {
        let orbit = OrbitalBody::new(p.orbit_radius, p.initial_angle, p.inclination);
        let pos   = orbit.position();
        let mesh  = meshes.add(Sphere::new(p.sphere_radius).mesh().uv(32, 18));
        // PBR material — lit by the sun PointLight, giving a lit/dark side that reads as 3D.
        let mat   = materials.add(StandardMaterial {
            base_color:         p.color,
            perceptual_roughness: 0.8,
            metallic:           0.0,
            ..default()
        });
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            Transform::from_translation(pos),
            orbit,
            LodRange { min_scale: LOD_SOLAR.0, max_scale: LOD_SOLAR.1 },
            Visibility::Hidden,
        ));

        if p.has_ring {
            let ring_orbit = OrbitalBody::new(p.orbit_radius, p.initial_angle, p.inclination);
            let ring_mesh  = meshes.add(
                Torus { minor_radius: 30.0, major_radius: p.sphere_radius * 2.2 }
                    .mesh().minor_resolution(4).major_resolution(48),
            );
            let ring_mat = materials.add(StandardMaterial {
                base_color:         Color::srgba(0.78, 0.70, 0.55, 0.6),
                perceptual_roughness: 0.9,
                alpha_mode:         AlphaMode::Blend,
                ..default()
            });
            commands.spawn((
                Mesh3d(ring_mesh),
                MeshMaterial3d(ring_mat),
                Transform::from_translation(pos).with_rotation(Quat::from_rotation_x(0.35)),
                ring_orbit,
                LodRange { min_scale: LOD_SOLAR.0, max_scale: LOD_SOLAR.1 },
                Visibility::Hidden,
            ));
        }
    }
}

// ── Starfield ─────────────────────────────────────────────────────────────────
// Single mesh entity — all stars in one draw call.

pub fn spawn_starfield(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut rng = ChaCha8Rng::seed_from_u64(0xDEAD_BEEF_1234_5678);

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(N_STARS * 4);
    let mut normals:   Vec<[f32; 3]> = Vec::with_capacity(N_STARS * 4);
    let mut uvs:       Vec<[f32; 2]> = Vec::with_capacity(N_STARS * 4);
    let mut indices:   Vec<u32>      = Vec::with_capacity(N_STARS * 6);

    for i in 0..N_STARS {
        // Uniform random point on a sphere (Marsaglia method)
        let (x, y, z) = loop {
            let u: f32 = rng.gen_range(-1.0..1.0);
            let v: f32 = rng.gen_range(-1.0..1.0);
            let s = u * u + v * v;
            if s < 1.0 {
                break (2.0 * u * (1.0_f32 - s).sqrt(),
                       2.0 * v * (1.0_f32 - s).sqrt(),
                       1.0 - 2.0 * s);
            }
        };
        let normal = Vec3::new(x, y, z); // already unit length
        let center = normal * STAR_SPHERE_R;
        let size   = rng.gen_range(STAR_SIZE_MIN..STAR_SIZE_MAX);
        let half   = size * 0.5;

        // Two tangent vectors perpendicular to the outward normal
        let right = if normal.x.abs() < 0.9 {
            Vec3::X.cross(normal).normalize()
        } else {
            Vec3::Y.cross(normal).normalize()
        };
        let up = normal.cross(right);

        // Slight brightness variation: white to faint blue-white
        let _brightness = rng.gen_range(0.6_f32..1.0_f32); // not used in vertex colour; kept for reference

        let base = i as u32 * 4;
        // 4 corners of the tiny quad
        for &(ru, uu) in &[(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            let p = center + right * (ru * half) + up * (uu * half);
            positions.push(p.to_array());
            normals.push((-normal).to_array()); // face inward (visible from inside sphere)
            uvs.push([(ru + 1.0) * 0.5, (uu + 1.0) * 0.5]);
        }
        // Two triangles: 0-1-2, 0-2-3
        indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL,   normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0,     uvs);
    mesh.insert_indices(Indices::U32(indices));

    let mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.7, 0.75, 0.9, 0.25), // dim blue-white, faded
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,                           // visible from inside the sphere
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(mat),
        Transform::IDENTITY,
        StarSphere,
        LodRange { min_scale: LOD_STARS.0, max_scale: LOD_STARS.1 },
        Visibility::Hidden,
    ));
}

// ── Star follow ───────────────────────────────────────────────────────────────
// Keep the star sphere centred on the camera every frame so the edge of the
// sphere is never visible regardless of where the camera has orbited to.

pub fn center_starfield_on_camera(
    camera: Query<&Transform, With<Camera3d>>,
    mut stars: Query<&mut Transform, (With<StarSphere>, Without<Camera3d>)>,
) {
    let Ok(cam) = camera.get_single() else { return };
    let pos = cam.translation;
    for mut t in &mut stars {
        t.translation = pos;
    }
}
