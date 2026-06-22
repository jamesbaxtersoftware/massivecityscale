//! Turn-based battles with a Pokémon-style 2x2 action menu, driven by keyboard
//! OR gamepad. Engage a creature (ENTER / A) -> Encounter flash -> Battle. Navigate
//! the menu with arrows / D-pad, confirm with Enter / A, back out with Esc / B.

use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use crate::creatures::{
    build_creature_children, capture_chance, effectiveness, gain_exp, CaughtCreature, Collection,
    Creature, CreatureKind, Engaged, Inventory, PlayerStats, CAPTURE_REWARD,
};
use crate::hud::Wallet;
use crate::onfoot::{Mode, SURFACE_LAYER};

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Phase {
    #[default]
    Roam,
    Encounter,
    Battle,
}

const ENCOUNTER_DURATION: f32 = 0.9;

#[derive(Resource, Default)]
struct EncounterTimer(f32);
#[derive(Component)]
struct EncounterOverlay;

/// The active battle.
#[derive(Resource, Default)]
pub struct Battle {
    pub enemy: Option<Entity>,
    pub kind: Option<CreatureKind>,
    pub level: u32,
    pub hp: f32,
    pub max_hp: f32,
    pub bait_bonus: f32,
    pub hit_timer: f32,
    pub player_lunge: f32,
    pub enemy_stunned: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Page {
    Main,
    Move,
    Item,
}

/// 2x2 menu state: which page and the cursor (0=TL, 1=TR, 2=BL, 3=BR).
#[derive(Resource)]
pub(crate) struct Menu {
    pub(crate) page: Page,
    cursor: usize,
}
impl Default for Menu {
    fn default() -> Self {
        Self { page: Page::Main, cursor: 0 }
    }
}

#[derive(Resource, Default)]
struct BattleLog(String);

pub struct BattlePlugin;

impl Plugin for BattlePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<Phase>()
            .init_resource::<Battle>()
            .init_resource::<EncounterTimer>()
            .init_resource::<Menu>()
            .init_resource::<BattleLog>()
            .add_systems(
                Update,
                start_battle
                    .run_if(in_state(Mode::OnFoot))
                    .run_if(in_state(Phase::Roam)),
            )
            .add_systems(OnEnter(Phase::Encounter), spawn_encounter_overlay)
            .add_systems(OnExit(Phase::Encounter), despawn_encounter_overlay)
            .add_systems(Update, encounter_anim.run_if(in_state(Phase::Encounter)))
            .add_systems(
                Update,
                (menu_input, battle_camera, animate_player_creature, update_battle_ui)
                    .run_if(in_state(Phase::Battle)),
            )
            .add_systems(OnEnter(Phase::Battle), (spawn_battle_ui, spawn_player_creature))
            .add_systems(OnExit(Phase::Battle), (despawn_battle_ui, despawn_player_creature));
    }
}

/// A move's damage: base power scaled gently by level, times type effectiveness.
pub fn move_damage(power: f32, level: u32, eff: f32) -> f32 {
    (power + level as f32 * 2.0) * eff
}
/// Enemy damage: softer than the player's so a bad type matchup stings without
/// being an instant KO.
pub fn enemy_damage(power: f32, level: u32, eff: f32) -> f32 {
    (power * 0.6 + level as f32) * eff
}

fn start_battle(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    engaged: Res<Engaged>,
    creatures: Query<&Creature>,
    mut battle: ResMut<Battle>,
    mut next: ResMut<NextState<Phase>>,
) {
    let pad = gamepads.iter().next().is_some_and(|g| g.just_pressed(GamepadButton::South));
    if !(keys.just_pressed(KeyCode::Enter) || pad) {
        return;
    }
    let Some((e, c)) = engaged.0.and_then(|e| creatures.get(e).ok().map(|c| (e, c))) else {
        return;
    };
    *battle = Battle {
        enemy: Some(e),
        kind: Some(c.kind),
        level: c.level,
        hp: c.hp,
        max_hp: c.max_hp,
        bait_bonus: 0.0,
        hit_timer: 0.0,
        player_lunge: 0.0,
        enemy_stunned: false,
    };
    next.set(Phase::Encounter);
}

// ── Encounter transition ─────────────────────────────────────────────────────
fn spawn_encounter_overlay(mut commands: Commands, mut timer: ResMut<EncounterTimer>) {
    timer.0 = 0.0;
    commands.spawn((
        EncounterOverlay,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::BLACK),
        GlobalZIndex(1000),
    ));
}

fn despawn_encounter_overlay(mut commands: Commands, q: Query<Entity, With<EncounterOverlay>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

fn encounter_anim(
    time: Res<Time>,
    mut timer: ResMut<EncounterTimer>,
    mut next: ResMut<NextState<Phase>>,
    mut overlay: Query<&mut BackgroundColor, With<EncounterOverlay>>,
) {
    timer.0 += time.delta_secs();
    let t = timer.0;
    let color = if t < 0.55 && (t * 14.0) as i32 % 2 == 0 {
        Color::srgb(1.0, 1.0, 1.0)
    } else {
        Color::BLACK
    };
    if let Ok(mut bg) = overlay.get_single_mut() {
        bg.0 = color;
    }
    if t >= ENCOUNTER_DURATION {
        next.set(Phase::Battle);
    }
}

fn battle_camera(
    time: Res<Time>,
    mut battle: ResMut<Battle>,
    mut creatures: Query<&mut Transform, (With<Creature>, Without<Camera3d>)>,
    mut cam: Query<&mut Transform, With<Camera3d>>,
) {
    let Some(e) = battle.enemy else { return };
    let Ok(mut enemy) = creatures.get_mut(e) else { return };
    let Ok(mut c) = cam.get_single_mut() else { return };
    let cam_pos = enemy.translation + Vec3::new(0.2, 1.7, 4.6);
    c.translation = cam_pos;
    c.look_at(enemy.translation + Vec3::Y * 0.8, Vec3::Y);
    let mut to_cam = cam_pos - enemy.translation;
    to_cam.y = 0.0;
    if to_cam.length_squared() > 1e-4 {
        enemy.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, to_cam.normalize());
    }
    if battle.hit_timer > 0.0 {
        battle.hit_timer = (battle.hit_timer - time.delta_secs()).max(0.0);
        let k = battle.hit_timer / 0.22;
        enemy.scale = Vec3::new(1.0 + 0.3 * k, 1.0 - 0.2 * k, 1.0 + 0.3 * k);
    } else {
        enemy.scale = Vec3::ONE;
    }
}

const LUNGE_DURATION: f32 = 0.28;

// ── Player's creature in the VS frame ────────────────────────────────────────
#[derive(Component)]
struct PlayerBattleCreature;

/// Rest pose of the buddy, so the attack lunge can spring out and back.
#[derive(Component)]
struct CreatureRest {
    pos: Vec3,
    fwd: Vec3,
}

/// Spawn the lead party creature in the foreground, back to the camera, facing
/// the enemy — classic Pokémon VS staging.
fn spawn_player_creature(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    collection: Res<Collection>,
    battle: Res<Battle>,
    enemies: Query<&Transform, With<Creature>>,
) {
    let Some(lead) = collection.party.first() else { return };
    let Some(e) = battle.enemy else { return };
    let Ok(enemy_t) = enemies.get(e) else { return };
    let layer = RenderLayers::layer(SURFACE_LAYER);
    let base = enemy_t.translation;
    // Foreground lower-left: left of the enemy, between it and the camera, sized
    // up so its back fills the corner above the message box.
    let pos = base + Vec3::new(-1.1, 0.0, 2.5);
    let mut to_enemy = base - pos;
    to_enemy.y = 0.0;
    let rot = if to_enemy.length_squared() > 1e-4 {
        Quat::from_rotation_arc(Vec3::NEG_Z, to_enemy.normalize())
    } else {
        Quat::IDENTITY
    };
    let kind = lead.kind;
    commands
        .spawn((
            PlayerBattleCreature,
            CreatureRest { pos, fwd: rot * Vec3::NEG_Z },
            layer.clone(),
            Transform { translation: pos, rotation: rot, scale: Vec3::splat(1.3) },
            Visibility::default(),
        ))
        .with_children(|cr| {
            build_creature_children(cr, &mut meshes, &mut materials, kind.color(), &layer);
        });
}

fn despawn_player_creature(mut commands: Commands, q: Query<Entity, With<PlayerBattleCreature>>) {
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
}

/// Spring the buddy forward and back along its facing while the lunge timer runs.
fn animate_player_creature(
    time: Res<Time>,
    mut battle: ResMut<Battle>,
    mut q: Query<(&mut Transform, &CreatureRest), With<PlayerBattleCreature>>,
) {
    let Ok((mut t, rest)) = q.get_single_mut() else { return };
    if battle.player_lunge > 0.0 {
        battle.player_lunge = (battle.player_lunge - time.delta_secs()).max(0.0);
        // k: 1 at start -> 0 at end; sin(pi*k) eases out to a peak mid-lunge.
        let k = battle.player_lunge / LUNGE_DURATION;
        let amt = (std::f32::consts::PI * k).sin() * 0.9;
        t.translation = rest.pos + rest.fwd * amt;
    } else {
        t.translation = rest.pos;
    }
}

// ── Menu input (keyboard + gamepad) ──────────────────────────────────────────
fn just(keys: &ButtonInput<KeyCode>, gp: Option<&Gamepad>, kb: &[KeyCode], pad: GamepadButton) -> bool {
    kb.iter().any(|k| keys.just_pressed(*k)) || gp.is_some_and(|g| g.just_pressed(pad))
}

#[allow(clippy::too_many_arguments)]
fn menu_input(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut menu: ResMut<Menu>,
    mut battle: ResMut<Battle>,
    mut stats: ResMut<PlayerStats>,
    mut inv: ResMut<Inventory>,
    mut collection: ResMut<Collection>,
    mut wallet: ResMut<Wallet>,
    mut log: ResMut<BattleLog>,
    mut toasts: ResMut<crate::toast::Toasts>,
    mut next: ResMut<NextState<Phase>>,
    mut commands: Commands,
) {
    let gp = gamepads.iter().next();
    let up = just(&keys, gp, &[KeyCode::ArrowUp], GamepadButton::DPadUp);
    let down = just(&keys, gp, &[KeyCode::ArrowDown], GamepadButton::DPadDown);
    let left = just(&keys, gp, &[KeyCode::ArrowLeft], GamepadButton::DPadLeft);
    let right = just(&keys, gp, &[KeyCode::ArrowRight], GamepadButton::DPadRight);
    let confirm = just(&keys, gp, &[KeyCode::Enter, KeyCode::Space], GamepadButton::South);
    let back = just(&keys, gp, &[KeyCode::Escape, KeyCode::Backspace], GamepadButton::East);

    let (mut r, mut c) = (menu.cursor / 2, menu.cursor % 2);
    if up {
        r = 0;
    }
    if down {
        r = 1;
    }
    if left {
        c = 0;
    }
    if right {
        c = 1;
    }
    menu.cursor = r * 2 + c;

    if back && menu.page == Page::Item {
        menu.page = Page::Main;
        menu.cursor = 0;
        return;
    }
    if !confirm {
        return;
    }

    let kind = battle.kind.unwrap_or(CreatureKind::Grasshog);
    let lead_kind = collection
        .party
        .first()
        .map(|l| l.kind)
        .unwrap_or(CreatureKind::Grasshog);
    // Resolve the selection.
    enum Act {
        OpenMove,
        Move(usize),
        Capture,
        OpenItem,
        Flee,
        Heal,
        Bait,
        Flash,
        Back,
    }
    let act = match (menu.page, menu.cursor) {
        (Page::Main, 0) => Act::OpenMove,
        (Page::Main, 1) => Act::Capture,
        (Page::Main, 2) => Act::OpenItem,
        (Page::Main, _) => Act::Flee,
        (Page::Move, 3) => Act::Back,
        (Page::Move, i) => Act::Move(i),
        (Page::Item, 0) => Act::Heal,
        (Page::Item, 1) => Act::Bait,
        (Page::Item, 2) => Act::Flash,
        (Page::Item, _) => Act::Back,
    };

    match act {
        Act::OpenMove => {
            menu.page = Page::Move;
            menu.cursor = 0;
            return;
        }
        Act::OpenItem => {
            menu.page = Page::Item;
            menu.cursor = 0;
            return;
        }
        Act::Back => {
            menu.page = Page::Main;
            menu.cursor = 0;
            return;
        }
        Act::Flee => {
            log.0 = "Got away safely.".into();
            next.set(Phase::Roam);
            return;
        }
        Act::Move(i) => {
            let Some(m) = lead_kind.moves().get(i).copied() else { return };
            if stats.sp < m.sp_cost as f32 {
                log.0 = format!("Not enough SP for {}!", m.name);
                return;
            }
            stats.sp -= m.sp_cost as f32;
            let eff = effectiveness(m.element, kind.element());
            let dmg = move_damage(m.power, stats.level, eff);
            battle.hp = (battle.hp - dmg).max(0.0);
            battle.hit_timer = 0.22;
            battle.player_lunge = LUNGE_DURATION;
            let tag = match eff {
                e if e > 1.0 => "  Super effective!",
                e if e < 1.0 => "  Not very effective...",
                _ => "",
            };
            log.0 = format!("{} hits for {dmg:.0}!{tag}", m.name);
            menu.page = Page::Main;
            menu.cursor = 0;
        }
        Act::Capture => {
            if inv.capture_disc == 0 {
                log.0 = "Out of Capture Discs!".into();
                return;
            }
            inv.capture_disc -= 1;
            let chance = (capture_chance(battle.hp, battle.max_hp, battle.level) + battle.bait_bonus)
                .clamp(0.0, 1.0);
            if rand::random::<f32>() < chance {
                if let Some(e) = battle.enemy {
                    commands.entity(e).despawn();
                }
                collection.party.push(CaughtCreature { kind, level: battle.level });
                let levels = gain_exp(&mut stats, battle.level * 20);
                wallet.crystals += CAPTURE_REWARD;
                toasts.push(format!("Caught {}!", kind.name()));
                if levels > 0 {
                    toasts.push(format!("Level up!  Lv.{}", stats.level));
                }
                log.0 = format!("Gotcha! {kind:?} was caught!");
                next.set(Phase::Roam);
                return;
            }
            log.0 = format!("Aw, the {kind:?} broke free!");
        }
        Act::Heal => {
            if inv.heal_spray == 0 {
                log.0 = "No Heal Spray!".into();
                return;
            }
            inv.heal_spray -= 1;
            stats.hp = (stats.hp + 25.0).min(stats.max_hp);
            stats.sp = (stats.sp + 10.0).min(stats.max_sp);
            log.0 = "You spray on a heal (+25 HP, +10 SP).".into();
            menu.page = Page::Main;
            menu.cursor = 0;
        }
        Act::Bait => {
            if inv.bait == 0 {
                log.0 = "No Bait!".into();
                return;
            }
            inv.bait -= 1;
            battle.bait_bonus = (battle.bait_bonus + 0.15).min(0.45);
            log.0 = format!("You toss Bait — the {kind:?} is intrigued.");
            menu.page = Page::Main;
            menu.cursor = 0;
        }
        Act::Flash => {
            if inv.flash_bomb == 0 {
                log.0 = "No Flash Bombs!".into();
                return;
            }
            inv.flash_bomb -= 1;
            battle.enemy_stunned = true;
            log.0 = format!("Flash! The {kind:?} is dazzled.");
            menu.page = Page::Main;
            menu.cursor = 0;
        }
    }

    // Enemy fainted from the hit?
    if battle.hp <= 0.0 {
        if let Some(e) = battle.enemy {
            commands.entity(e).despawn();
        }
        let levels = gain_exp(&mut stats, battle.level * 8);
        if levels > 0 {
            toasts.push(format!("Level up!  Lv.{}", stats.level));
        }
        log.0 = format!("The {kind:?} fled, exhausted.");
        next.set(Phase::Roam);
        return;
    }
    // Enemy's turn (unless stunned).
    if battle.enemy_stunned {
        battle.enemy_stunned = false;
        log.0 = format!("{}  The {kind:?} is too dazed to move!", log.0);
        return;
    }
    let emoves = kind.moves();
    let m = emoves[rand::random::<u32>() as usize % emoves.len()];
    let eff = effectiveness(m.element, lead_kind.element());
    let edmg = enemy_damage(m.power, battle.level, eff);
    stats.hp = (stats.hp - edmg).max(0.0);
    let tag = if eff > 1.0 { "  It's a rough matchup!" } else { "" };
    log.0 = format!("{}  Wild {kind:?} used {} for {edmg:.0}!{tag}", log.0, m.name);
    if stats.hp <= 0.0 {
        stats.hp = stats.max_hp * 0.5;
        log.0 = "You were overwhelmed — retreated to safety!".into();
        next.set(Phase::Roam);
    }
}

// ── Battle UI ────────────────────────────────────────────────────────────────
#[derive(Component)]
struct BattleUi;
#[derive(Component)]
struct EnemyLine;
#[derive(Component)]
struct LogLine;
#[derive(Component)]
struct PlayerLine;
#[derive(Component)]
struct EnemyHpFill;
#[derive(Component)]
struct PlayerHpFill;
#[derive(Component)]
struct MenuCell(usize);
#[derive(Component)]
struct MenuLabel(usize);

fn spawn_battle_ui(mut commands: Commands, mut menu: ResMut<Menu>, mut log: ResMut<BattleLog>) {
    menu.page = Page::Main;
    menu.cursor = 0;
    if log.0.is_empty() {
        log.0 = "A wild creature blocks your path!".into();
    }
    commands
        .spawn((
            BattleUi,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::all(Val::Px(28.0)),
                ..default()
            },
        ))
        .with_children(|root| {
            // Top: enemy name + HP bar.
            root.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            })
            .with_children(|top| {
                top.spawn((
                    EnemyLine,
                    Text::new(""),
                    TextFont { font_size: 26.0, ..default() },
                    TextColor(Color::srgb(1.0, 0.92, 0.85)),
                ));
                top.spawn((
                    Node { width: Val::Px(280.0), height: Val::Px(16.0), ..default() },
                    BackgroundColor(Color::srgba(0.08, 0.10, 0.16, 0.9)),
                ))
                .with_children(|track| {
                    track.spawn((
                        EnemyHpFill,
                        Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
                        BackgroundColor(Color::srgb(0.3, 0.9, 0.4)),
                    ));
                });
            });

            // Bottom: message box (left) + 2x2 menu (right).
            root.spawn(Node {
                width: Val::Percent(100.0),
                column_gap: Val::Px(16.0),
                align_items: AlignItems::FlexEnd,
                ..default()
            })
            .with_children(|bottom| {
                // Message / log box.
                bottom
                    .spawn((
                        Node {
                            flex_grow: 1.0,
                            min_height: Val::Px(110.0),
                            padding: UiRect::all(Val::Px(14.0)),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(8.0),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.05, 0.07, 0.12, 0.92)),
                        BorderColor(Color::srgb(0.4, 0.55, 0.8)),
                    ))
                    .with_children(|box_| {
                        box_.spawn((
                            LogLine,
                            Text::new(""),
                            TextFont { font_size: 20.0, ..default() },
                            TextColor(Color::srgb(0.95, 0.97, 1.0)),
                        ));
                        box_.spawn((
                            PlayerLine,
                            Text::new(""),
                            TextFont { font_size: 16.0, ..default() },
                            TextColor(Color::srgb(0.7, 0.85, 1.0)),
                        ));
                        // Your HP bar, mirroring the enemy's.
                        box_.spawn((
                            Node { width: Val::Px(260.0), height: Val::Px(12.0), ..default() },
                            BackgroundColor(Color::srgba(0.08, 0.10, 0.16, 0.9)),
                        ))
                        .with_children(|track| {
                            track.spawn((
                                PlayerHpFill,
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.3, 0.9, 0.4)),
                            ));
                        });
                    });

                // 2x2 menu grid.
                bottom
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        ..default()
                    })
                    .with_children(|grid| {
                        for row in 0..2 {
                            grid.spawn(Node {
                                column_gap: Val::Px(6.0),
                                ..default()
                            })
                            .with_children(|r| {
                                for col in 0..2 {
                                    let idx = row * 2 + col;
                                    r.spawn((
                                        MenuCell(idx),
                                        Node {
                                            width: Val::Px(150.0),
                                            height: Val::Px(46.0),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            border: UiRect::all(Val::Px(2.0)),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgba(0.10, 0.12, 0.2, 0.95)),
                                        BorderColor(Color::srgb(0.3, 0.4, 0.6)),
                                    ))
                                    .with_children(|cell| {
                                        cell.spawn((
                                            MenuLabel(idx),
                                            Text::new(""),
                                            TextFont { font_size: 19.0, ..default() },
                                            TextColor(Color::srgb(0.95, 0.97, 1.0)),
                                        ));
                                    });
                                }
                            });
                        }
                    });
            });
        });
}

fn despawn_battle_ui(mut commands: Commands, q: Query<Entity, With<BattleUi>>) {
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
}

/// HP bar colour: green when healthy, yellow when low, red when critical.
fn hp_color(frac: f32) -> Color {
    if frac > 0.5 {
        Color::srgb(0.3, 0.9, 0.4)
    } else if frac > 0.2 {
        Color::srgb(0.95, 0.8, 0.2)
    } else {
        Color::srgb(0.9, 0.3, 0.25)
    }
}

#[allow(clippy::type_complexity)]
fn update_battle_ui(
    battle: Res<Battle>,
    stats: Res<PlayerStats>,
    inv: Res<Inventory>,
    menu: Res<Menu>,
    log: Res<BattleLog>,
    collection: Res<Collection>,
    mut hpbar: Query<
        (&mut Node, &mut BackgroundColor),
        (With<EnemyHpFill>, Without<MenuCell>, Without<PlayerHpFill>),
    >,
    mut playerbar: Query<
        (&mut Node, &mut BackgroundColor),
        (With<PlayerHpFill>, Without<MenuCell>, Without<EnemyHpFill>),
    >,
    mut cells: Query<
        (&MenuCell, &mut BackgroundColor, &mut BorderColor),
        (Without<EnemyHpFill>, Without<PlayerHpFill>),
    >,
    mut q: ParamSet<(
        Query<&mut Text, With<EnemyLine>>,
        Query<&mut Text, With<LogLine>>,
        Query<&mut Text, With<PlayerLine>>,
        Query<(&MenuLabel, &mut Text)>,
    )>,
) {
    let kind = battle.kind.unwrap_or(CreatureKind::Grasshog);
    let frac = (battle.hp / battle.max_hp.max(1.0)).clamp(0.0, 1.0);
    if let Ok((mut n, mut col)) = hpbar.get_single_mut() {
        n.width = Val::Percent(frac * 100.0);
        col.0 = hp_color(frac);
    }
    let pfrac = (stats.hp / stats.max_hp.max(1.0)).clamp(0.0, 1.0);
    if let Ok((mut n, mut col)) = playerbar.get_single_mut() {
        n.width = Val::Percent(pfrac * 100.0);
        col.0 = hp_color(pfrac);
    }
    if let Ok(mut t) = q.p0().get_single_mut() {
        t.0 = format!("Wild {kind:?}   Lv{}", battle.level);
    }
    if let Ok(mut t) = q.p1().get_single_mut() {
        t.0 = log.0.clone();
    }
    if let Ok(mut t) = q.p2().get_single_mut() {
        let lead = collection
            .party
            .first()
            .map(|l| format!("{} Lv{}   ", l.kind.name(), l.level))
            .unwrap_or_default();
        t.0 = format!(
            "{lead}YOU  HP {:.0}/{:.0}  SP {:.0}/{:.0}",
            stats.hp, stats.max_hp, stats.sp, stats.max_sp
        );
    }

    let labels_src = match menu.page {
        Page::Main => [
            "Attack".to_string(),
            "Capture".to_string(),
            "Item".to_string(),
            "Flee".to_string(),
        ],
        Page::Move => {
            let lead = collection.party.first().map(|l| l.kind).unwrap_or(CreatureKind::Grasshog);
            let mvs = lead.moves();
            let label = |i: usize| {
                mvs.get(i)
                    .map(|m| {
                        if m.sp_cost == 0 {
                            m.name.to_string()
                        } else {
                            format!("{} {}sp", m.name, m.sp_cost)
                        }
                    })
                    .unwrap_or_else(|| "-".to_string())
            };
            [label(0), label(1), label(2), "Back".to_string()]
        }
        Page::Item => [
            format!("Heal ({})", inv.heal_spray),
            format!("Bait ({})", inv.bait),
            format!("Flash ({})", inv.flash_bomb),
            "Back".to_string(),
        ],
    };
    for (l, mut t) in &mut q.p3() {
        t.0 = labels_src[l.0].clone();
    }
    for (cell, mut bg, mut border) in &mut cells {
        if cell.0 == menu.cursor {
            bg.0 = Color::srgb(0.95, 0.8, 0.25);
            border.0 = Color::srgb(1.0, 1.0, 1.0);
        } else {
            bg.0 = Color::srgba(0.10, 0.12, 0.2, 0.95);
            border.0 = Color::srgb(0.3, 0.4, 0.6);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enemy_damage_scales_and_softer_than_player() {
        assert!(enemy_damage(18.0, 10, 1.0) > enemy_damage(18.0, 1, 1.0));
        assert!(enemy_damage(18.0, 5, 2.0) > enemy_damage(18.0, 5, 1.0));
        // Same move hits softer from the enemy than from the player.
        assert!(enemy_damage(18.0, 5, 1.0) < move_damage(18.0, 5, 1.0));
    }

    #[test]
    fn move_damage_respects_effectiveness_and_level() {
        // Super-effective beats neutral beats not-very-effective at the same level.
        assert!(move_damage(18.0, 5, 2.0) > move_damage(18.0, 5, 1.0));
        assert!(move_damage(18.0, 5, 1.0) > move_damage(18.0, 5, 0.5));
        // Higher level hits harder for the same move.
        assert!(move_damage(18.0, 20, 1.0) > move_damage(18.0, 1, 1.0));
    }
}
