use bevy::prelude::*;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum BattleSide { Player, Wild }

#[derive(Component)]
pub struct HpBar { pub side: BattleSide }

#[derive(Component)]
pub struct HpFill;

const BAR_W: f32 = 90.0;
const BAR_H: f32 = 10.0;

/// Spawns a billboarded HP bar (dark background quad + green fill child). Returns the bar entity.
pub fn spawn_hp_bar(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
    side: BattleSide,
) -> Entity {
    let bg_mesh = meshes.add(Rectangle::new(BAR_W, BAR_H));
    let bg_mat = materials.add(StandardMaterial { base_color: Color::srgb(0.05, 0.05, 0.05), unlit: true, ..default() });
    let fill_mesh = meshes.add(Rectangle::new(BAR_W, BAR_H));
    let fill_mat = materials.add(StandardMaterial { base_color: Color::srgb(0.2, 0.9, 0.3), unlit: true, ..default() });

    commands.spawn((
        Mesh3d(bg_mesh),
        MeshMaterial3d(bg_mat),
        Transform::from_translation(pos),
        HpBar { side },
    )).with_children(|c| {
        c.spawn((
            Mesh3d(fill_mesh),
            MeshMaterial3d(fill_mat),
            Transform::from_xyz(0.0, 0.0, 0.1),
            HpFill,
        ));
    }).id()
}

/// Billboard all HP bars so their front (+Z) faces the camera each frame.
pub fn billboard_hp_bars(
    camera: Query<&Transform, (With<Camera3d>, Without<HpBar>)>,
    mut bars: Query<&mut Transform, With<HpBar>>,
) {
    let Ok(cam) = camera.get_single() else { return };
    for mut t in &mut bars {
        let away = t.translation + (t.translation - cam.translation);
        t.look_at(away, Vec3::Y);
    }
}

/// Scale + recolor the green→red fill to the current HP fraction, anchored on the left edge.
pub fn update_hp_bars(
    session: Res<super::BattleSession>,
    player: Res<super::PlayerCreature>,
    wild_q: Query<&super::Creature, With<super::WildMonster>>,
    bars: Query<(&HpBar, &Children)>,
    mut fills: Query<(&mut Transform, &MeshMaterial3d<StandardMaterial>), With<HpFill>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let wild_max = wild_q.get(session.wild_entity).map(|c| c.max_hp).unwrap_or(1).max(1);
    for (bar, children) in &bars {
        let frac = match bar.side {
            BattleSide::Player => session.player_hp as f32 / player.0.max_hp.max(1) as f32,
            BattleSide::Wild   => session.wild_hp as f32 / wild_max as f32,
        }.clamp(0.0, 1.0);
        for &child in children {
            if let Ok((mut t, mat)) = fills.get_mut(child) {
                t.scale.x = frac;
                t.translation.x = -BAR_W * (1.0 - frac) * 0.5;
                if let Some(m) = materials.get_mut(&mat.0) {
                    // full = green, empty = red
                    m.base_color = Color::srgb(0.9 - 0.7 * frac, 0.2 + 0.7 * frac, 0.2);
                }
            }
        }
    }
}
