//! Full-screen party / collection viewer, toggled with C (or gamepad Select)
//! while on foot and out of battle. Lists every caught creature with a colour
//! swatch, name, level and element.

use bevy::prelude::*;
use crate::battle::Phase;
use crate::creatures::{Collection, CreatureKind};
use crate::onfoot::Mode;

#[derive(Resource, Default)]
pub struct PartyView {
    pub open: bool,
}

#[derive(Component)]
struct PartyViewRoot;

pub struct PartyViewPlugin;

impl Plugin for PartyViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PartyView>()
            .add_systems(Update, (toggle, sync_panel).chain());
    }
}

fn toggle(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut view: ResMut<PartyView>,
) {
    let pad = gamepads
        .iter()
        .next()
        .is_some_and(|g| g.just_pressed(GamepadButton::Select));
    if keys.just_pressed(KeyCode::KeyC) || pad {
        view.open = !view.open;
    }
}

fn sync_panel(
    mut commands: Commands,
    mut view: ResMut<PartyView>,
    mode: Res<State<Mode>>,
    phase: Res<State<Phase>>,
    collection: Res<Collection>,
    existing: Query<Entity, With<PartyViewRoot>>,
) {
    // The viewer only makes sense while walking around, not in flight or battle.
    if *mode.get() != Mode::OnFoot || *phase.get() != Phase::Roam {
        view.open = false;
    }
    let shown = existing.get_single().ok();
    match (view.open, shown) {
        (true, None) => spawn_panel(&mut commands, &collection),
        (false, Some(e)) => commands.entity(e).despawn_recursive(),
        _ => {}
    }
}

fn spawn_panel(commands: &mut Commands, collection: &Collection) {
    commands
        .spawn((
            PartyViewRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(10.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.06, 0.86)),
            GlobalZIndex(900),
        ))
        .with_children(|root| {
            let species = CreatureKind::ALL
                .iter()
                .filter(|k| collection.party.iter().any(|c| c.kind == **k))
                .count();
            root.spawn((
                Text::new(format!(
                    "PARTY  ({})        DEX  {}/{}",
                    collection.party.len(),
                    species,
                    CreatureKind::ALL.len()
                )),
                TextFont { font_size: 30.0, ..default() },
                TextColor(Color::srgb(1.0, 0.92, 0.6)),
            ));

            // Card list (party head first = active lead).
            root.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                width: Val::Px(440.0),
                ..default()
            })
            .with_children(|list| {
                if collection.party.is_empty() {
                    list.spawn((
                        Text::new("No creatures caught yet."),
                        TextFont { font_size: 20.0, ..default() },
                        TextColor(Color::srgb(0.7, 0.78, 0.9)),
                    ));
                }
                for (i, c) in collection.party.iter().rev().enumerate() {
                    let lead = i == 0;
                    list.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(48.0),
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(12.0),
                            padding: UiRect::horizontal(Val::Px(12.0)),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.08, 0.10, 0.16, 0.95)),
                        BorderColor(if lead {
                            Color::srgb(1.0, 0.85, 0.3)
                        } else {
                            Color::srgb(0.3, 0.4, 0.6)
                        }),
                    ))
                    .with_children(|row| {
                        // Colour swatch.
                        row.spawn((
                            Node { width: Val::Px(26.0), height: Val::Px(26.0), ..default() },
                            BackgroundColor(c.kind.color()),
                        ));
                        row.spawn((
                            Text::new(format!(
                                "{}{}   Lv{}   {:?}",
                                if lead { "> " } else { "" },
                                c.kind.name(),
                                c.level,
                                c.kind.element(),
                            )),
                            TextFont { font_size: 20.0, ..default() },
                            TextColor(Color::srgb(0.95, 0.97, 1.0)),
                        ));
                    });
                }
            });

            root.spawn((
                Text::new("[C] close"),
                TextFont { font_size: 16.0, ..default() },
                TextColor(Color::srgb(0.6, 0.7, 0.85)),
            ));
        });
}
