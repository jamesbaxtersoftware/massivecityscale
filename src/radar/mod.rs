//! Flight radar: a small top-down minimap (bottom-centre) plotting nearby planets
//! relative to the ship's heading, with the locked target highlighted. Forward is
//! up. Shown only in flight.

use bevy::math::DVec3;
use bevy::prelude::*;
use crate::galaxy::PlanetBody;
use crate::onfoot::Mode;
use crate::origin::WorldPos;
use crate::ship::PlayerShip;
use crate::targeting::Targets;

const SIZE: f32 = 160.0;
const HALF: f32 = SIZE / 2.0;
/// Metres of range the radar edge represents.
const RANGE: f64 = 6.0e7;

#[derive(Component)]
struct RadarPanel;
#[derive(Component)]
struct RadarDot;

pub struct RadarPlugin;

impl Plugin for RadarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_radar)
            .add_systems(Update, update_radar);
    }
}

fn spawn_radar(mut commands: Commands) {
    commands
        .spawn((
            RadarPanel,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(44.0),
                right: Val::Px(16.0),
                width: Val::Px(SIZE),
                height: Val::Px(SIZE),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.06, 0.10, 0.55)),
            BorderColor(Color::srgb(0.3, 0.45, 0.7)),
            GlobalZIndex(100),
        ))
        .with_children(|p| {
            // Ship marker at centre (forward = up).
            p.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(HALF - 3.0),
                    top: Val::Px(HALF - 3.0),
                    width: Val::Px(6.0),
                    height: Val::Px(6.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.9, 0.95, 1.0)),
            ));
        });
}

#[allow(clippy::type_complexity)]
fn update_radar(
    mut commands: Commands,
    mode: Res<State<Mode>>,
    ship: Query<(&WorldPos, &Transform), With<PlayerShip>>,
    planets: Query<(Entity, &WorldPos, &PlanetBody), Without<PlayerShip>>,
    targets: Res<Targets>,
    mut panel: Query<(Entity, &mut Visibility), With<RadarPanel>>,
    dots: Query<Entity, With<RadarDot>>,
) {
    let Ok((panel_e, mut vis)) = panel.get_single_mut() else { return };
    // Rebuild dots fresh each frame.
    for d in &dots {
        commands.entity(d).despawn_recursive();
    }
    if *mode.get() != Mode::Flight {
        *vis = Visibility::Hidden;
        return;
    }
    *vis = Visibility::Visible;
    let Ok((swp, stf)) = ship.get_single() else { return };
    let fwd = stf.rotation * Vec3::NEG_Z;
    let right = stf.rotation * Vec3::X;
    let fwd = DVec3::new(fwd.x as f64, fwd.y as f64, fwd.z as f64);
    let right = DVec3::new(right.x as f64, right.y as f64, right.z as f64);
    let scale = HALF as f64 / RANGE;

    for (e, p, _body) in &planets {
        let rel = p.0 - swp.0;
        if rel.length() > RANGE {
            continue;
        }
        let rx = rel.dot(right) * scale;
        let ry = rel.dot(fwd) * scale; // forward distance
        let px = (HALF + rx as f32).clamp(2.0, SIZE - 4.0);
        let py = (HALF - ry as f32).clamp(2.0, SIZE - 4.0); // forward = up
        let is_target = targets.selected == Some(e);
        let (s, color) = if is_target {
            (8.0, Color::srgb(1.0, 0.85, 0.3))
        } else {
            (5.0, Color::srgb(0.45, 0.8, 1.0))
        };
        commands.entity(panel_e).with_children(|panel| {
            panel.spawn((
                RadarDot,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(px - s / 2.0),
                    top: Val::Px(py - s / 2.0),
                    width: Val::Px(s),
                    height: Val::Px(s),
                    ..default()
                },
                BackgroundColor(color),
            ));
        });
    }
}
