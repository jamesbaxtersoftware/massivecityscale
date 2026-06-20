use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use noise::{NoiseFn, Perlin};
use crate::galaxy::PlanetBody;
use crate::origin::WorldPos;
use crate::streaming::{LodBody, LodTier};

/// Vertical relief as a fraction of planet radius at full descent.
pub const TERRAIN_AMPLITUDE_FRAC: f32 = 0.06;
/// Subdivisions of the displaced sphere (higher = finer relief, more verts).
pub const TERRAIN_SUBDIVS: u32 = 6;

/// Displaced surface radius along `unit_dir` for a planet of base `radius`.
/// Deterministic (fixed Perlin seed) so the same planet always looks the same.
pub fn displace_height(unit_dir: Vec3, radius: f32, amplitude: f32) -> f32 {
    let perlin = Perlin::new(1);
    let p = unit_dir * 3.7;
    let n = perlin.get([p.x as f64, p.y as f64, p.z as f64]) as f32; // ~[-1,1]
    radius + n * amplitude
}

#[derive(Component)]
pub struct TerrainResolved;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        // Runs after streaming has set tiers this frame.
        app.add_systems(Update, resolve_terrain.after(crate::origin::FrameSet::Stream));
    }
}

/// When a planet reaches the Terrain tier, replace its smooth sphere mesh with a
/// noise-displaced one (once). The floating origin keeps it jitter-free up close.
pub fn resolve_terrain(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    q: Query<(Entity, &PlanetBody, &LodBody), (With<WorldPos>, Without<TerrainResolved>)>,
) {
    for (entity, body, lod) in &q {
        if lod.tier != LodTier::Terrain {
            continue;
        }
        let amplitude = body.radius * TERRAIN_AMPLITUDE_FRAC;
        let mesh = displaced_sphere(body.radius, amplitude, TERRAIN_SUBDIVS);
        commands.entity(entity)
            .insert(Mesh3d(meshes.add(mesh)))
            .insert(TerrainResolved);
    }
}

/// Build a UV-sphere whose vertices are pushed out by `displace_height`.
fn displaced_sphere(radius: f32, amplitude: f32, subdivs: u32) -> Mesh {
    let stacks = subdivs.max(2) * 2;
    let slices = subdivs.max(2) * 2;
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    for i in 0..=stacks {
        let v = i as f32 / stacks as f32;
        let phi = v * std::f32::consts::PI;
        for j in 0..=slices {
            let u = j as f32 / slices as f32;
            let theta = u * std::f32::consts::TAU;
            let dir = Vec3::new(
                phi.sin() * theta.cos(),
                phi.cos(),
                phi.sin() * theta.sin(),
            );
            let r = displace_height(dir, radius, amplitude);
            let pos = dir * r;
            positions.push([pos.x, pos.y, pos.z]);
            normals.push([dir.x, dir.y, dir.z]);
        }
    }
    let row = slices + 1;
    for i in 0..stacks {
        for j in 0..slices {
            let a = i * row + j;
            let b = a + row;
            indices.extend_from_slice(&[a, a + 1, b, a + 1, b + 1, b]);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::render::mesh::{Indices, VertexAttributeValues};

    #[test]
    fn displaced_sphere_triangles_face_outward() {
        let mesh = displaced_sphere(350.0, 20.0, 6);

        let positions = match mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap() {
            VertexAttributeValues::Float32x3(v) => v.clone(),
            _ => panic!("unexpected position format"),
        };

        let idx = match mesh.indices().unwrap() {
            Indices::U32(v) => v.clone(),
            _ => panic!("unexpected index format"),
        };

        for tri in idx.chunks_exact(3) {
            let (i0, i1, i2) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
            let p0 = Vec3::from(positions[i0]);
            let p1 = Vec3::from(positions[i1]);
            let p2 = Vec3::from(positions[i2]);
            let n = (p1 - p0).cross(p2 - p0);
            // Skip degenerate triangles at poles (near-zero area due to collapsed vertices).
            // At phi=0 or phi=PI all vertices in that ring share the same position;
            // floating-point residuals give a tiny but unreliable normal, so skip those.
            if n.length_squared() < 1.0 {
                continue;
            }
            let c = (p0 + p1 + p2) / 3.0;
            assert!(
                n.dot(c) > 0.0,
                "triangle ({i0},{i1},{i2}) faces inward: n={n:?} c={c:?}"
            );
        }
    }

    #[test]
    fn displacement_stays_within_amplitude_band() {
        let radius = 350.0;
        let amp = 20.0;
        for dir in [Vec3::X, Vec3::Y, Vec3::Z, Vec3::NEG_X, Vec3::new(1.0, 1.0, 1.0).normalize()] {
            let r = displace_height(dir, radius, amp);
            assert!((r - radius).abs() <= amp + 1e-3,
                "relief within ±amplitude (dir={dir:?}, r={r})");
        }
    }

    #[test]
    fn displacement_is_deterministic() {
        let d = Vec3::new(0.3, -0.7, 0.5).normalize();
        assert_eq!(displace_height(d, 350.0, 20.0), displace_height(d, 350.0, 20.0));
    }

    #[test]
    fn displacement_actually_varies_across_the_surface() {
        let a = displace_height(Vec3::X, 350.0, 20.0);
        let b = displace_height(Vec3::Y, 350.0, 20.0);
        assert!((a - b).abs() > 1e-3, "relief is not flat");
    }
}
