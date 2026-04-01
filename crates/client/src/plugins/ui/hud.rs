use bevy::prelude::*;

use super::GameScreen;
use crate::plugins::game::ClientState;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameScreen::InGame), spawn_hud)
            .add_systems(OnExit(GameScreen::InGame), despawn_hud)
            .add_systems(
                Update,
                update_position_text.run_if(in_state(GameScreen::InGame)),
            );
    }
}

#[derive(Component)]
struct HudRoot;

#[derive(Component)]
struct PositionText;

fn spawn_hud(mut commands: Commands) {
    let root = commands
        .spawn((
            HudRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                left: Val::Px(10.0),
                ..default()
            },
        ))
        .id();

    let pos_text = commands
        .spawn((
            PositionText,
            Text::new("Position: (-, -)"),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgb(0.8, 0.8, 0.8)),
        ))
        .id();

    commands.entity(root).add_child(pos_text);
}

fn update_position_text(
    state: Res<ClientState>,
    mut query: Query<&mut Text, With<PositionText>>,
) {
    if let Some(pos) = state.position
        && let Ok(mut text) = query.single_mut()
    {
        **text = format!("Position: ({}, {})", pos.x, pos.y);
    }
}

fn despawn_hud(mut commands: Commands, query: Query<Entity, With<HudRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
