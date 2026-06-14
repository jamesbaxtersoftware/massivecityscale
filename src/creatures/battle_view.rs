use bevy::prelude::*;
use super::{BattleSession, PlayerCreature, Creature, WildMonster, Inventory, Turn};

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum BattleSide { Player, Wild }

// ── Battle HUD / move menu ──────────────────────────────────────────────────

/// Root of the battle menu overlay; despawned on battle exit.
#[derive(Component)]
pub struct BattleHud;

/// The dynamic status line (HP totals, whose turn, force-field count).
#[derive(Component)]
pub struct BattleStatusText;

/// Spawn a bottom-anchored menu the moment a battle starts: a title, a live
/// status line, and the move controls.
pub fn spawn_battle_hud(
    mut commands: Commands,
    player: Res<PlayerCreature>,
    session: Res<BattleSession>,
    wild_q: Query<&Creature, With<WildMonster>>,
) {
    let moves = player.0.species.moves();
    let (wild_name, wild_level) = wild_q
        .get(session.wild_entity)
        .map(|c| (c.species.name(), c.level))
        .unwrap_or(("Wild", 0));

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.0),
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(8.0),
            padding: UiRect::all(Val::Px(18.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.02, 0.06, 0.8)),
        BattleHud,
    )).with_children(|root| {
        root.spawn((
            Text::new(format!("A wild {wild_name} (Lv {wild_level}) blocks your path!")),
            TextFont { font_size: 26.0, ..default() },
            TextColor(Color::WHITE),
        ));
        root.spawn((
            Text::new(String::new()),
            TextFont { font_size: 20.0, ..default() },
            TextColor(Color::srgb(0.7, 0.95, 0.8)),
            BattleStatusText,
        ));
        root.spawn((
            Text::new(format!(
                "[1] {}     [2] {}     [F] Throw Force Field",
                moves[0].name, moves[1].name,
            )),
            TextFont { font_size: 22.0, ..default() },
            TextColor(Color::srgb(0.8, 0.85, 1.0)),
        ));
    });
}

/// Refresh the status line each frame with live HP, force fields, and turn.
pub fn update_battle_hud(
    session: Res<BattleSession>,
    player: Res<PlayerCreature>,
    inventory: Res<Inventory>,
    wild_q: Query<&Creature, With<WildMonster>>,
    mut text_q: Query<&mut Text, With<BattleStatusText>>,
) {
    let wild_max = wild_q.get(session.wild_entity).map(|c| c.max_hp).unwrap_or(1).max(1);
    let turn = match session.turn {
        Turn::Player => "Your move",
        Turn::Enemy  => "Enemy attacking...",
    };
    for mut text in &mut text_q {
        *text = Text::new(format!(
            "You {}/{} HP    Wild {}/{} HP    Force Fields: {}    —  {}",
            session.player_hp.max(0), player.0.max_hp,
            session.wild_hp.max(0), wild_max,
            inventory.force_fields, turn,
        ));
    }
}

/// Despawn the battle menu when the battle ends.
pub fn despawn_battle_hud(mut commands: Commands, roots: Query<Entity, With<BattleHud>>) {
    for e in &roots { commands.entity(e).despawn_recursive(); }
}

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
