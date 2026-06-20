use bevy::prelude::*;

/// Top-level app state. `Running` ticks the sim; `Paused` freezes it and shows
/// the pause menu. Esc toggles between them.
#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    Running,
    Paused,
}

/// Marker on the pause-menu UI root so it can be despawned on resume.
#[derive(Component)]
pub struct PauseMenu;

pub struct PauseMenuPlugin;

impl Plugin for PauseMenuPlugin {
    fn build(&self, app: &mut App) {
        use bevy::ecs::schedule::IntoSystemSetConfigs;
        use crate::origin::FrameSet;
        app.init_state::<AppState>()
            // Freeze the whole per-frame pipeline while paused — one gate over the
            // entire FrameSet chain owned by OriginPlugin (origin stays decoupled
            // from pause; the condition only exists when this plugin is present).
            .configure_sets(
                Update,
                (
                    FrameSet::Input,
                    FrameSet::Move,
                    FrameSet::Origin,
                    FrameSet::Sync,
                    FrameSet::Camera,
                    FrameSet::Stream,
                )
                    .run_if(in_state(AppState::Running)),
            )
            .add_systems(Update, toggle_pause)
            .add_systems(Update, quit_from_menu.run_if(in_state(AppState::Paused)))
            .add_systems(OnEnter(AppState::Paused), spawn_pause_menu)
            .add_systems(OnExit(AppState::Paused), despawn_pause_menu);
    }
}

/// Controls cheat-sheet shown in the pause overlay.
const CONTROLS: &str = "Mouse  —  Steer\n\
W / S  —  Thrust forward / back\n\
A / D  —  Strafe left / right\n\
Shift  —  Boost\n\
Esc    —  Resume\n\
Q      —  Quit";

// ── Systems ──────────────────────────────────────────────────────────────────

/// Esc flips between Running and Paused. Runs in both states so it can resume.
fn toggle_pause(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        next.set(match state.get() {
            AppState::Running => AppState::Paused,
            AppState::Paused => AppState::Running,
        });
    }
}

/// Q quits — only registered to run while paused (quitting lives behind the menu).
fn quit_from_menu(keys: Res<ButtonInput<KeyCode>>, mut exit: EventWriter<AppExit>) {
    if keys.just_pressed(KeyCode::KeyQ) {
        exit.send(AppExit::Success);
    }
}

/// Spawn the dim full-screen overlay with the PAUSED title and controls list.
fn spawn_pause_menu(mut commands: Commands) {
    commands
        .spawn((
            PauseMenu,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(18.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.02, 0.72)),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("PAUSED"),
                TextFont { font_size: 48.0, ..default() },
                TextColor(Color::srgb(0.90, 0.95, 1.0)),
            ));
            root.spawn((
                Text::new(CONTROLS),
                TextFont { font_size: 22.0, ..default() },
                TextColor(Color::srgb(0.74, 0.80, 0.90)),
            ));
        });
}

/// Despawn the overlay (and its children) on resume.
fn despawn_pause_menu(mut commands: Commands, q: Query<Entity, With<PauseMenu>>) {
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::DVec3;
    use crate::origin::{FloatingOrigin, OriginPlugin, WorldPos};

    /// Fresh input each tap so `just_pressed` fires cleanly (no carry-over of the
    /// `pressed` set between frames).
    fn tap(app: &mut App, key: KeyCode) {
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(key);
        app.insert_resource(input);
    }

    /// Clear input and run one more update so a `NextState` set during the prior
    /// Update is applied by the StateTransition schedule (which runs before
    /// Update each frame). Without clearing, the held key would re-toggle.
    fn settle(app: &mut App) {
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.update();
    }

    fn base_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::state::app::StatesPlugin)
            .add_plugins(PauseMenuPlugin);
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.add_event::<AppExit>();
        app
    }

    fn state(app: &App) -> AppState {
        app.world().resource::<State<AppState>>().get().clone()
    }

    #[test]
    fn esc_toggles_running_to_paused_and_back() {
        let mut app = base_app();
        app.update();
        assert_eq!(state(&app), AppState::Running, "starts running");

        tap(&mut app, KeyCode::Escape);
        app.update();
        settle(&mut app);
        assert_eq!(state(&app), AppState::Paused, "esc pauses");

        tap(&mut app, KeyCode::Escape);
        app.update();
        settle(&mut app);
        assert_eq!(state(&app), AppState::Running, "esc resumes");
    }

    #[test]
    fn entering_paused_spawns_one_menu_then_resume_despawns_it() {
        let mut app = base_app();
        app.update();

        app.world_mut().resource_mut::<NextState<AppState>>().set(AppState::Paused);
        app.update();
        let count = app
            .world_mut()
            .query_filtered::<Entity, With<PauseMenu>>()
            .iter(app.world())
            .count();
        assert_eq!(count, 1, "one pause-menu root spawned on enter");

        app.world_mut().resource_mut::<NextState<AppState>>().set(AppState::Running);
        app.update();
        let count = app
            .world_mut()
            .query_filtered::<Entity, With<PauseMenu>>()
            .iter(app.world())
            .count();
        assert_eq!(count, 0, "pause-menu despawned on resume");
    }

    #[test]
    fn q_quits_only_while_paused() {
        let mut app = base_app();
        app.update();

        // Running: Q does nothing.
        tap(&mut app, KeyCode::KeyQ);
        app.update();
        assert!(
            app.world().resource::<Events<AppExit>>().is_empty(),
            "Q while running does not quit"
        );

        // Paused: Q quits.
        app.world_mut().resource_mut::<NextState<AppState>>().set(AppState::Paused);
        app.update();
        tap(&mut app, KeyCode::KeyQ);
        app.update();
        assert!(
            !app.world().resource::<Events<AppExit>>().is_empty(),
            "Q while paused quits"
        );
    }

    #[test]
    fn paused_freezes_transform_sync() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::state::app::StatesPlugin)
            .add_plugins(OriginPlugin)
            .add_plugins(PauseMenuPlugin);
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.insert_resource(FloatingOrigin(DVec3::ZERO));
        let e = app
            .world_mut()
            .spawn((WorldPos(DVec3::new(10.0, 0.0, 0.0)), Transform::default()))
            .id();

        // Running: sync places the entity at (10,0,0).
        app.update();
        assert_eq!(
            app.world().get::<Transform>(e).unwrap().translation,
            Vec3::new(10.0, 0.0, 0.0),
            "sync runs while running"
        );

        // Move the entity's world position, then pause: the render Transform must
        // NOT follow (the FrameSet chain is gated off).
        app.world_mut().get_mut::<WorldPos>(e).unwrap().0 = DVec3::new(99.0, 0.0, 0.0);
        app.world_mut().resource_mut::<NextState<AppState>>().set(AppState::Paused);
        app.update(); // applies pause
        app.update(); // sync should be frozen
        assert_eq!(
            app.world().get::<Transform>(e).unwrap().translation,
            Vec3::new(10.0, 0.0, 0.0),
            "sync frozen while paused"
        );
    }
}
