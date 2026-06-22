//! Toggle-able controls reference (press H, or gamepad Start). A full-screen
//! overlay grouping every binding by context. Available in any mode.

use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct HelpView {
    pub open: bool,
}

#[derive(Component)]
struct HelpRoot;

type Section = (&'static str, &'static [(&'static str, &'static str)]);

const SECTIONS: &[Section] = &[
    (
        "FLIGHT",
        &[
            ("Mouse", "steer"),
            ("W / S", "thrust fwd / back"),
            ("A / D", "strafe"),
            ("Shift", "boost"),
            ("T", "cycle target"),
            ("G", "warp to target"),
            ("F", "land (in range)"),
        ],
    ),
    (
        "ON FOOT",
        &[
            ("W A S D", "walk"),
            ("Mouse", "look"),
            ("Enter", "engage battle"),
            ("C", "party / dex"),
            ("F", "take off"),
        ],
    ),
    (
        "BATTLE",
        &[
            ("Arrows", "move cursor"),
            ("Enter / Space", "confirm"),
            ("Esc", "back"),
        ],
    ),
    (
        "GENERAL",
        &[
            ("F5 / F9", "save / load"),
            ("Esc", "pause"),
            ("H", "close this help"),
        ],
    ),
];

pub struct HelpPlugin;

impl Plugin for HelpPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HelpView>()
            .add_systems(Update, (toggle, sync_panel).chain());
    }
}

fn toggle(keys: Res<ButtonInput<KeyCode>>, gamepads: Query<&Gamepad>, mut view: ResMut<HelpView>) {
    let pad = gamepads
        .iter()
        .next()
        .is_some_and(|g| g.just_pressed(GamepadButton::Start));
    if keys.just_pressed(KeyCode::KeyH) || pad {
        view.open = !view.open;
    }
}

fn sync_panel(mut commands: Commands, view: Res<HelpView>, existing: Query<Entity, With<HelpRoot>>) {
    let shown = existing.get_single().ok();
    match (view.open, shown) {
        (true, None) => spawn_panel(&mut commands),
        (false, Some(e)) => commands.entity(e).despawn_recursive(),
        _ => {}
    }
}

fn spawn_panel(commands: &mut Commands) {
    commands
        .spawn((
            HelpRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.06, 0.9)),
            GlobalZIndex(1800),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("CONTROLS"),
                TextFont { font_size: 32.0, ..default() },
                TextColor(Color::srgb(1.0, 0.92, 0.6)),
            ));
            // Two columns of sections.
            root.spawn(Node {
                column_gap: Val::Px(60.0),
                ..default()
            })
            .with_children(|cols| {
                for chunk in SECTIONS.chunks(2) {
                    cols.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(16.0),
                        ..default()
                    })
                    .with_children(|col| {
                        for (title, rows) in chunk {
                            spawn_section(col, title, rows);
                        }
                    });
                }
            });
            root.spawn((
                Text::new("[H] close"),
                TextFont { font_size: 16.0, ..default() },
                TextColor(Color::srgb(0.6, 0.7, 0.85)),
            ));
        });
}

fn spawn_section(col: &mut ChildBuilder, title: &str, rows: &[(&str, &str)]) {
    col.spawn(Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(3.0),
        width: Val::Px(280.0),
        ..default()
    })
    .with_children(|sec| {
        sec.spawn((
            Text::new(title),
            TextFont { font_size: 21.0, ..default() },
            TextColor(Color::srgb(0.5, 0.85, 1.0)),
        ));
        for (key, action) in rows {
            sec.spawn(Node {
                column_gap: Val::Px(8.0),
                ..default()
            })
            .with_children(|line| {
                line.spawn((
                    Node { width: Val::Px(118.0), ..default() },
                    Text::new(*key),
                    TextFont { font_size: 17.0, ..default() },
                    TextColor(Color::srgb(0.95, 0.9, 0.55)),
                ));
                line.spawn((
                    Text::new(*action),
                    TextFont { font_size: 17.0, ..default() },
                    TextColor(Color::srgb(0.9, 0.93, 1.0)),
                ));
            });
        }
    });
}
