use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::camera::zoom::OrbitState;
use super::{WildMonster, ClickSphere, GameState, BattleSession};

/// Build an orthographic picking ray from the cursor. Returns (origin, forward dir).
pub fn ortho_pick_ray(
    window: &Window,
    ortho_scale: f32,
    orbit: &OrbitState,
) -> Option<(Vec3, Vec3)> {
    let cursor = window.cursor_position()?;
    let cam_pos = orbit.camera_pos();
    let forward = (orbit.pivot - cam_pos).normalize();
    let right   = forward.cross(Vec3::Y).normalize();
    let up      = right.cross(forward);

    let win = Vec2::new(window.width(), window.height());
    let ndc = Vec2::new(cursor.x / win.x * 2.0 - 1.0, 1.0 - cursor.y / win.y * 2.0);
    let half_w = ortho_scale * win.x / 2.0;
    let half_h = ortho_scale * win.y / 2.0;
    let origin = cam_pos + ndc.x * half_w * right + ndc.y * half_h * up;
    Some((origin, forward))
}

/// Nearest front-face ray-sphere hit distance `t`, or None.
pub fn ray_sphere_t(origin: Vec3, dir: Vec3, center: Vec3, radius: f32) -> Option<f32> {
    let l = origin - center;
    let b = l.dot(dir);
    let c = l.dot(l) - radius * radius;
    let d = b * b - c;
    if d < 0.0 { return None; }
    let t = -b - d.sqrt();
    if t < 0.0 { None } else { Some(t) }
}

/// In Exploring, a non-drag left click on a wild monster opens a battle.
pub fn pick_wild_monster(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    proj_query: Query<&Projection, With<Camera3d>>,
    orbit: Res<OrbitState>,
    monsters: Query<(Entity, &GlobalTransform, &ClickSphere), With<WildMonster>>,
    tracker: Res<crate::renderer::solar::ClickTracker>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if !mouse_buttons.just_released(MouseButton::Left) { return; }
    if tracker.drag_sq() > 25.0 { return; } // was a drag, not a click

    let Ok(Projection::Orthographic(ortho)) = proj_query.get_single() else { return };
    let Ok(window) = windows.get_single() else { return };
    let Some((origin, forward)) = ortho_pick_ray(window, ortho.scale, &orbit) else { return };

    let mut best: Option<(Entity, f32)> = None;
    for (entity, gt, cs) in &monsters {
        if let Some(t) = ray_sphere_t(origin, forward, gt.translation(), cs.radius) {
            if best.map_or(true, |(_, bt)| t < bt) { best = Some((entity, t)); }
        }
    }
    if let Some((wild, _)) = best {
        commands.insert_resource(BattleSession::new(wild));
        next_state.set(GameState::Battle);
    }
}
