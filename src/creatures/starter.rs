use bevy::prelude::*;
use super::{CreatureType, Species, Creature, PlayerCreature, GameState};

#[derive(Component)]
pub struct StarterChoice(pub CreatureType);

/// Root of the starter-select UI overlay; despawned on exit.
#[derive(Component)]
pub struct StarterRoot;

/// Spawn a screen-space UI overlay with one button per starter. Rendered in
/// screen space so it is immune to camera zoom/drift (unlike in-world meshes,
/// which were tiny and slid out of frame at solar zoom).
pub fn spawn_starters(mut commands: Commands) {
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(32.0),
            ..default()
        },
        StarterRoot,
    )).with_children(|root| {
        root.spawn((
            Text::new("Choose your starter"),
            TextFont { font_size: 44.0, ..default() },
            TextColor(Color::WHITE),
        ));
        root.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(48.0),
            ..default()
        }).with_children(|row| {
            spawn_choice(row, CreatureType::Fire,  "Emberling", Color::srgba(0.95, 0.35, 0.10, 0.9));
            spawn_choice(row, CreatureType::Water, "Tideling",  Color::srgba(0.15, 0.45, 0.90, 0.9));
        });
    });
}

fn spawn_choice(parent: &mut ChildBuilder, t: CreatureType, name: &str, color: Color) {
    parent.spawn((
        Button,
        Node {
            width: Val::Px(240.0),
            height: Val::Px(160.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect::all(Val::Px(3.0)),
            ..default()
        },
        BackgroundColor(color),
        BorderColor(Color::WHITE),
        StarterChoice(t),
    )).with_children(|b| {
        b.spawn((
            Text::new(name),
            TextFont { font_size: 30.0, ..default() },
            TextColor(Color::WHITE),
        ));
    });
}

/// Clicking a starter button locks it in as the player's creature and enters Exploring.
pub fn pick_starter(
    interactions: Query<(&Interaction, &StarterChoice), Changed<Interaction>>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (interaction, choice) in &interactions {
        if *interaction == Interaction::Pressed {
            commands.insert_resource(PlayerCreature(Creature::new(Species::of_type(choice.0), 5)));
            next_state.set(GameState::Exploring);
        }
    }
}

pub fn despawn_starters(mut commands: Commands, roots: Query<Entity, With<StarterRoot>>) {
    for e in &roots { commands.entity(e).despawn_recursive(); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn pressing_a_starter_selects_it_and_enters_exploring() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
           .add_plugins(bevy::state::app::StatesPlugin)
           .init_state::<GameState>();

        // Simulate the user pressing the Fire starter button.
        app.world_mut().spawn((Interaction::Pressed, StarterChoice(CreatureType::Fire)));
        app.world_mut().run_system_once(pick_starter).unwrap();

        // The player creature is created from the chosen type...
        let pc = app.world().get_resource::<PlayerCreature>()
            .expect("PlayerCreature should be inserted on selection");
        assert_eq!(pc.0.species, Species::Emberling);

        // ...and the game advances out of StarterSelect into Exploring.
        match app.world().resource::<NextState<GameState>>() {
            NextState::Pending(s) => assert_eq!(*s, GameState::Exploring),
            _ => panic!("expected a pending transition to Exploring"),
        }
    }
}
