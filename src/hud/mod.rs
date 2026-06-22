//! Space HUD overlay (bevy_ui): shield/hull/fuel bars, boost meter, speed,
//! crystals, objective, and nearest-world distance — laid out like the reference.
//! Renders crisp over the pixelated 3D scene via the window UI camera.

use bevy::prelude::*;
use crate::galaxy::PlanetBody;
use crate::origin::WorldPos;
use crate::ship::{PlayerShip, ShipVelocity};

// ── Backing data (live where it exists; scaffolded otherwise) ────────────────

/// Ship condition, each 0..1. Drained by later systems (damage/fuel); full for now.
#[derive(Resource)]
pub struct ShipStatus {
    pub shield: f32,
    pub hull: f32,
    pub fuel: f32,
}
impl Default for ShipStatus {
    fn default() -> Self {
        Self { shield: 1.0, hull: 1.0, fuel: 1.0 }
    }
}

#[derive(Resource, Default)]
pub struct Wallet {
    pub crystals: u32,
}

#[derive(Resource)]
pub struct Objective(pub String);
impl Default for Objective {
    fn default() -> Self {
        Self("Find a new planet".into())
    }
}

// ── UI markers ───────────────────────────────────────────────────────────────
#[derive(Component)]
struct ShieldBar;
#[derive(Component)]
struct HullBar;
#[derive(Component)]
struct FuelBar;
#[derive(Component)]
struct SpeedText;
#[derive(Component)]
struct CrystalsText;
#[derive(Component)]
struct NearestText;
#[derive(Component)]
struct ObjectiveText;
#[derive(Component)]
struct LandPromptText;
/// Root tags for swapping HUDs by mode.
#[derive(Component)]
struct SpaceHud;
#[derive(Component)]
struct OnFootHud;
#[derive(Component)]
struct ItemBarText;
#[derive(Component)]
struct EngagedText;
#[derive(Component)]
struct PlayerPanelText;
#[derive(Component)]
struct PartyText;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShipStatus>()
            .init_resource::<Wallet>()
            .init_resource::<Objective>()
            .add_systems(Startup, (setup_hud, setup_onfoot_hud))
            .add_systems(Update, (update_hud, hud_mode_visibility, update_onfoot_hud));
    }
}

fn label_color() -> Color {
    Color::srgb(0.7, 0.8, 0.92)
}

/// Spawn a labelled bar (dark track + coloured fill tagged with `marker`).
fn spawn_bar<M: Component>(parent: &mut ChildBuilder, label: &str, fill: Color, marker: M) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                TextFont { font_size: 14.0, ..default() },
                TextColor(label_color()),
            ));
            row.spawn((
                Node {
                    width: Val::Px(170.0),
                    height: Val::Px(13.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.08, 0.10, 0.16, 0.85)),
            ))
            .with_children(|track| {
                track.spawn((
                    marker,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(fill),
                ));
            });
        });
}

fn corner(top: Option<f32>, left: Option<f32>, right: Option<f32>, bottom: Option<f32>) -> Node {
    Node {
        position_type: PositionType::Absolute,
        top: top.map(Val::Px).unwrap_or(Val::Auto),
        left: left.map(Val::Px).unwrap_or(Val::Auto),
        right: right.map(Val::Px).unwrap_or(Val::Auto),
        bottom: bottom.map(Val::Px).unwrap_or(Val::Auto),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(6.0),
        padding: UiRect::all(Val::Px(10.0)),
        ..default()
    }
}

fn setup_hud(mut commands: Commands) {
    // Bottom-left: condition bars.
    commands
        .spawn((corner(None, Some(16.0), None, Some(16.0)), SpaceHud))
        .with_children(|c| {
            spawn_bar(c, "SHIELD", Color::srgb(0.25, 0.7, 1.0), ShieldBar);
            spawn_bar(c, "HULL", Color::srgb(0.3, 0.9, 0.5), HullBar);
            spawn_bar(c, "FUEL", Color::srgb(0.95, 0.6, 0.2), FuelBar);
        });

    // Bottom-right: speed.
    commands
        .spawn((corner(None, None, Some(16.0), Some(16.0)), SpaceHud))
        .with_children(|c| {
            c.spawn((
                SpeedText,
                Text::new("SPEED  0 km/s"),
                TextFont { font_size: 18.0, ..default() },
                TextColor(label_color()),
            ));
        });

    // Top-left: objective.
    commands
        .spawn((corner(Some(16.0), Some(16.0), None, None), SpaceHud))
        .with_children(|c| {
            c.spawn((
                Text::new("OBJECTIVE"),
                TextFont { font_size: 13.0, ..default() },
                TextColor(Color::srgb(0.5, 0.6, 0.72)),
            ));
            c.spawn((
                ObjectiveText,
                Text::new(""),
                TextFont { font_size: 18.0, ..default() },
                TextColor(Color::srgb(0.9, 0.95, 1.0)),
            ));
        });

    // Top-right: crystals + nearest world.
    commands
        .spawn((
            {
                let mut n = corner(Some(16.0), None, Some(16.0), None);
                n.align_items = AlignItems::End;
                n
            },
            SpaceHud,
        ))
        .with_children(|c| {
            c.spawn((
                CrystalsText,
                Text::new("0"),
                TextFont { font_size: 20.0, ..default() },
                TextColor(Color::srgb(0.6, 0.9, 1.0)),
            ));
            c.spawn((
                NearestText,
                Text::new(""),
                TextFont { font_size: 16.0, ..default() },
                TextColor(label_color()),
            ));
        });

    // Centered land prompt (shown only when in range, in flight).
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                top: Val::Percent(58.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            SpaceHud,
        ))
        .with_children(|c| {
            c.spawn((
                LandPromptText,
                Text::new(""),
                TextFont { font_size: 26.0, ..default() },
                TextColor(Color::srgb(1.0, 0.9, 0.3)),
            ));
        });
}

/// On-foot HUD: engaged-creature panel (centre) + item bar (bottom).
fn setup_onfoot_hud(mut commands: Commands) {
    // Engaged creature info, centred above the item bar.
    commands
        .spawn((
            OnFootHud,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                top: Val::Percent(20.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|c| {
            c.spawn((
                EngagedText,
                Text::new(""),
                TextFont { font_size: 22.0, ..default() },
                TextColor(Color::srgb(0.95, 0.95, 1.0)),
            ));
        });

    // Item bar, bottom-centre.
    commands
        .spawn((
            OnFootHud,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                bottom: Val::Px(18.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|c| {
            c.spawn((
                ItemBarText,
                Text::new(""),
                TextFont { font_size: 18.0, ..default() },
                TextColor(label_color()),
            ));
        });

    // Player panel, bottom-left.
    commands
        .spawn((
            OnFootHud,
            corner(None, Some(16.0), None, Some(16.0)),
        ))
        .with_children(|c| {
            c.spawn((
                PlayerPanelText,
                Text::new(""),
                TextFont { font_size: 18.0, ..default() },
                TextColor(Color::srgb(0.92, 0.95, 1.0)),
            ));
        });

    // Party panel, top-right.
    commands
        .spawn((
            OnFootHud,
            corner(Some(16.0), None, Some(16.0), None),
        ))
        .with_children(|c| {
            c.spawn((
                Text::new("PARTY"),
                TextFont { font_size: 13.0, ..default() },
                TextColor(Color::srgb(0.5, 0.6, 0.72)),
            ));
            c.spawn((
                PartyText,
                Text::new(""),
                TextFont { font_size: 16.0, ..default() },
                TextColor(label_color()),
            ));
        });
}

/// Swap which HUD shows based on mode.
fn hud_mode_visibility(
    mode: Res<State<crate::onfoot::Mode>>,
    phase: Res<State<crate::battle::Phase>>,
    mut space: Query<&mut Visibility, (With<SpaceHud>, Without<OnFootHud>)>,
    mut foot: Query<&mut Visibility, (With<OnFootHud>, Without<SpaceHud>)>,
) {
    let flight = *mode.get() == crate::onfoot::Mode::Flight;
    // On-foot HUD only while roaming on foot (the battle has its own overlay).
    let roaming_foot = !flight && *phase.get() == crate::battle::Phase::Roam;
    for mut v in &mut space {
        *v = if flight { Visibility::Inherited } else { Visibility::Hidden };
    }
    for mut v in &mut foot {
        *v = if roaming_foot { Visibility::Inherited } else { Visibility::Hidden };
    }
}

/// Fill the on-foot HUD: engaged creature (kind/Lv/HP/catch%) + item counts.
#[allow(clippy::type_complexity)]
fn update_onfoot_hud(
    inv: Res<crate::creatures::Inventory>,
    engaged: Res<crate::creatures::Engaged>,
    stats: Res<crate::creatures::PlayerStats>,
    collection: Res<crate::creatures::Collection>,
    creatures: Query<&crate::creatures::Creature>,
    mut texts: ParamSet<(
        Query<&mut Text, With<EngagedText>>,
        Query<&mut Text, With<ItemBarText>>,
        Query<&mut Text, With<PlayerPanelText>>,
        Query<&mut Text, With<PartyText>>,
    )>,
) {
    if let Ok(mut t) = texts.p0().get_single_mut() {
        t.0 = match engaged.0.and_then(|e| creatures.get(e).ok()) {
            Some(c) => {
                let pct = crate::creatures::capture_chance(c.hp, c.max_hp, c.level) * 100.0;
                format!(
                    "{:?}  Lv{}   HP {:.0}/{:.0}   CATCH {pct:.0}%\n[ENTER] battle   [F] take off",
                    c.kind, c.level, c.hp, c.max_hp
                )
            }
            None => "Approach a creature to engage   \u{2022}   [F] take off".into(),
        };
    }
    if let Ok(mut t) = texts.p1().get_single_mut() {
        t.0 = format!(
            "DISC {}    BAIT {}    HEAL {}    REVIVE {}    FLASH {}",
            inv.capture_disc, inv.bait, inv.heal_spray, inv.revive, inv.flash_bomb
        );
    }
    if let Ok(mut t) = texts.p2().get_single_mut() {
        let next = crate::creatures::exp_to_next(stats.level);
        t.0 = format!(
            "Lv.{}   HP {:.0}/{:.0}   SP {:.0}/{:.0}   EXP {}/{}   [C] party   [H] help",
            stats.level, stats.hp, stats.max_hp, stats.sp, stats.max_sp, stats.exp, next
        );
    }
    if let Ok(mut t) = texts.p3().get_single_mut() {
        if collection.party.is_empty() {
            t.0 = "(empty)".into();
        } else {
            t.0 = collection
                .party
                .iter()
                .rev()
                .take(6)
                .map(|c| format!("{:?}  Lv{}", c.kind, c.level))
                .collect::<Vec<_>>()
                .join("\n");
        }
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn update_hud(
    mode: Res<State<crate::onfoot::Mode>>,
    status: Res<ShipStatus>,
    wallet: Res<Wallet>,
    objective: Res<Objective>,
    targets: Res<crate::targeting::Targets>,
    autopilot: Res<crate::targeting::Autopilot>,
    ship: Query<(&WorldPos, &ShipVelocity), With<PlayerShip>>,
    planets: Query<(&WorldPos, &PlanetBody), Without<PlayerShip>>,
    target_q: Query<(&WorldPos, &PlanetBody)>,
    mut shield: Query<&mut Node, (With<ShieldBar>, Without<HullBar>, Without<FuelBar>)>,
    mut hull: Query<&mut Node, (With<HullBar>, Without<ShieldBar>, Without<FuelBar>)>,
    mut fuel: Query<&mut Node, (With<FuelBar>, Without<ShieldBar>, Without<HullBar>)>,
    mut texts: ParamSet<(
        Query<&mut Text, With<SpeedText>>,
        Query<&mut Text, With<CrystalsText>>,
        Query<&mut Text, With<NearestText>>,
        Query<&mut Text, With<ObjectiveText>>,
        Query<&mut Text, With<LandPromptText>>,
    )>,
) {
    if let Ok(mut n) = shield.get_single_mut() {
        n.width = Val::Percent(status.shield.clamp(0.0, 1.0) * 100.0);
    }
    if let Ok(mut n) = hull.get_single_mut() {
        n.width = Val::Percent(status.hull.clamp(0.0, 1.0) * 100.0);
    }
    if let Ok(mut n) = fuel.get_single_mut() {
        n.width = Val::Percent(status.fuel.clamp(0.0, 1.0) * 100.0);
    }

    let Ok((swp, vel)) = ship.get_single() else { return };
    let speed_kms = vel.0.length() as f64 / 1000.0;
    if let Ok(mut t) = texts.p0().get_single_mut() {
        t.0 = format!("SPEED  {speed_kms:.0} km/s");
    }
    if let Ok(mut t) = texts.p1().get_single_mut() {
        t.0 = format!("\u{25C6} {}", wallet.crystals);
    }
    // Prefer the selected target's live distance; else the nearest world.
    let (label, dist, target_pos) = match targets.selected.and_then(|e| target_q.get(e).ok()) {
        Some((p, body)) => ("TARGET", (p.0 - swp.0).length() - body.radius as f64, Some(p.0)),
        None => {
            let mut n = f64::MAX;
            for (p, body) in &planets {
                n = n.min((p.0 - swp.0).length() - body.radius as f64);
            }
            ("NEAREST", n, None)
        }
    };
    if let Ok(mut t) = texts.p2().get_single_mut() {
        if dist.is_finite() {
            let km = dist / 1000.0;
            let d = if km > 1.0e9 {
                format!("{:.2} LY", dist / 9.4607e15)
            } else {
                format!("{km:.0} km")
            };
            let warp = if autopilot.on { "  \u{25B6} WARP" } else { "" };
            // Predict the target's biome (matches what you land in) and its
            // signature species, so you can hunt specific creatures for the dex.
            let biome_line = match target_pos {
                Some(pos) => {
                    let b = crate::onfoot::Biome::from_pos(pos);
                    let sig = crate::creatures::CreatureKind::biome_pool(b)[0].name();
                    format!("\n{} - {}", b.label(), sig)
                }
                None => String::new(),
            };
            t.0 = format!("{label}  {d}{warp}{biome_line}\n[T] target   [G] warp   [H] help");
        }
    }
    if let Ok(mut t) = texts.p3().get_single_mut() {
        t.0 = objective.0.clone();
    }
    // Land prompt: in flight and within landing range of a planet.
    let nearest_surface = planets
        .iter()
        .map(|(p, b)| (p.0 - swp.0).length() - b.radius as f64)
        .fold(f64::MAX, f64::min);
    if let Ok(mut t) = texts.p4().get_single_mut() {
        t.0 = if *mode.get() == crate::onfoot::Mode::Flight
            && nearest_surface <= crate::onfoot::LAND_RANGE
        {
            "\u{25B6} PRESS F TO LAND".into()
        } else {
            String::new()
        };
    }
}
