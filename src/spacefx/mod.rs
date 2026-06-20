//! Space game-feel: motion streaks that convey speed. The floating origin keeps
//! the ship at the render origin, so distant stars barely move — these near-field
//! streaks (stretched along the velocity, drifting backward) sell the sense of
//! travel. Rendered as emissive cubes, so they pixelate with the rest of the 3D.

use bevy::prelude::*;
use rand::Rng;
use crate::origin::FrameSet;
use crate::ship::{PlayerShip, ShipVelocity};

const COUNT: usize = 220;
/// Half-extent (m) of the box of streaks kept around the ship.
const BOX_R: f32 = 320.0;

#[derive(Component)]
struct SpeedCue {
    /// Position in render space, relative to the ship (which sits at ~origin).
    pos: Vec3,
}

pub struct SpaceFxPlugin;

impl Plugin for SpaceFxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_cues)
            .add_systems(Update, update_cues.in_set(FrameSet::Camera));
    }
}

fn spawn_cues(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.7, 0.85, 1.0),
        emissive: LinearRgba::rgb(1.4, 1.8, 2.6),
        unlit: true,
        ..default()
    });
    let mut rng = rand::thread_rng();
    for _ in 0..COUNT {
        let pos = Vec3::new(
            rng.gen_range(-BOX_R..BOX_R),
            rng.gen_range(-BOX_R..BOX_R),
            rng.gen_range(-BOX_R..BOX_R),
        );
        commands.spawn((
            SpeedCue { pos },
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_translation(pos),
            Visibility::Hidden,
        ));
    }
}

/// Stretch each streak along the travel direction (length ∝ speed) and drift it
/// backward past the ship; hide them near rest so parked space stays clean.
fn update_cues(
    time: Res<Time>,
    ship: Query<&ShipVelocity, With<PlayerShip>>,
    mut cues: Query<(&mut SpeedCue, &mut Transform, &mut Visibility)>,
) {
    let dt = time.delta_secs();
    let Ok(vel) = ship.get_single() else { return };
    let speed = vel.0.length();
    let dir = if speed > 1.0 { vel.0 / speed } else { Vec3::NEG_Z };
    // Map speed (m/s) to a streak length; cap so it stays a near-field effect.
    let streak = (speed * 2.0e-5).clamp(0.0, 140.0);

    for (mut cue, mut tf, mut vis) in &mut cues {
        if streak < 2.0 {
            *vis = Visibility::Hidden;
            continue;
        }
        *vis = Visibility::Visible;
        // Drift backward relative to travel, wrapping within the box.
        cue.pos -= dir * (streak * 4.0 * dt);
        for a in 0..3 {
            if cue.pos[a] > BOX_R {
                cue.pos[a] -= 2.0 * BOX_R;
            } else if cue.pos[a] < -BOX_R {
                cue.pos[a] += 2.0 * BOX_R;
            }
        }
        tf.translation = cue.pos;
        tf.rotation = Quat::from_rotation_arc(Vec3::Z, dir);
        tf.scale = Vec3::new(0.5, 0.5, streak);
    }
}
