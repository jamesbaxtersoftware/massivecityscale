//! Transient on-screen toast notifications (catches, level-ups). Push a message
//! to the `Toasts` resource from anywhere; it appears top-centre, stacks, and
//! fades out after a couple of seconds.

use bevy::prelude::*;

const TOAST_TTL: f32 = 2.8;
const FADE: f32 = 0.7;

/// Queue of pending messages. Push from any system via `toasts.push(...)`.
#[derive(Resource, Default)]
pub struct Toasts {
    pending: Vec<String>,
}
impl Toasts {
    pub fn push(&mut self, msg: impl Into<String>) {
        self.pending.push(msg.into());
    }
}

#[derive(Component)]
struct ToastRoot;

#[derive(Component)]
struct Toast {
    ttl: f32,
}

pub struct ToastPlugin;

impl Plugin for ToastPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Toasts>()
            .add_systems(Startup, spawn_root)
            .add_systems(Update, (drain_pending, tick_toasts).chain());
    }
}

fn spawn_root(mut commands: Commands) {
    commands.spawn((
        ToastRoot,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(64.0),
            left: Val::Px(0.0),
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(6.0),
            ..default()
        },
        GlobalZIndex(1500),
    ));
}

fn drain_pending(
    mut commands: Commands,
    mut toasts: ResMut<Toasts>,
    root: Query<Entity, With<ToastRoot>>,
) {
    if toasts.pending.is_empty() {
        return;
    }
    let Ok(root_e) = root.get_single() else { return };
    for msg in toasts.pending.drain(..) {
        commands.entity(root_e).with_children(|p| {
            p.spawn((
                Toast { ttl: TOAST_TTL },
                Node {
                    padding: UiRect::axes(Val::Px(14.0), Val::Px(7.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.05, 0.07, 0.12, 0.9)),
                BorderColor(Color::srgb(1.0, 0.85, 0.35)),
            ))
            .with_children(|b| {
                b.spawn((
                    Text::new(msg),
                    TextFont { font_size: 23.0, ..default() },
                    TextColor(Color::srgb(1.0, 0.92, 0.55)),
                ));
            });
        });
    }
}

fn tick_toasts(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Toast, &mut BackgroundColor, &Children)>,
    mut texts: Query<&mut TextColor>,
) {
    for (e, mut t, mut bg, children) in &mut q {
        t.ttl -= time.delta_secs();
        if t.ttl <= 0.0 {
            commands.entity(e).despawn_recursive();
            continue;
        }
        let a = (t.ttl / FADE).min(1.0);
        bg.0 = bg.0.with_alpha(0.9 * a);
        for &c in children {
            if let Ok(mut tc) = texts.get_mut(c) {
                tc.0 = tc.0.with_alpha(a);
            }
        }
    }
}
