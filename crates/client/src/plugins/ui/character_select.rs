use bevy::prelude::*;
use halestorm_common::protocol::ClientMessage;
use halestorm_common::transport::MessageOutbox;

use super::GameScreen;
use crate::plugins::game::{ClientPhase, ClientState};

pub struct CharacterSelectPlugin;

impl Plugin for CharacterSelectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameScreen::CharacterSelect), spawn_ui)
            .add_systems(OnExit(GameScreen::CharacterSelect), despawn_ui)
            .add_systems(
                Update,
                (
                    handle_character_buttons,
                    handle_create_new_button,
                    check_enter_world,
                )
                    .distributive_run_if(in_state(GameScreen::CharacterSelect)),
            );
    }
}

#[derive(Component)]
struct CharacterSelectRoot;

#[derive(Component)]
struct CharacterButton(u64);

#[derive(Component)]
struct CreateNewButton;

fn spawn_ui(mut commands: Commands, state: Res<ClientState>) {
    let root = commands
        .spawn((
            CharacterSelectRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
        ))
        .id();

    // Title
    let title = commands
        .spawn((
            Text::new("Select Character"),
            TextFont {
                font_size: 36.0,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.8, 0.3)),
        ))
        .id();

    commands.entity(root).add_child(title);

    // Character buttons
    for character in &state.characters {
        let btn = commands
            .spawn((
                CharacterButton(character.id),
                Button,
                Node {
                    width: Val::Px(350.0),
                    padding: UiRect::axes(Val::Px(20.0), Val::Px(12.0)),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Row,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.2, 0.2, 0.3)),
            ))
            .id();

        let name_text = commands
            .spawn((
                Text::new(&character.name),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
            ))
            .id();

        let class_text = commands
            .spawn((
                Text::new(character.class.to_string()),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.8, 0.6)),
            ))
            .id();

        commands.entity(btn).add_children(&[name_text, class_text]);
        commands.entity(root).add_child(btn);
    }

    // "Create New Character" button
    let create_btn = commands
        .spawn((
            CreateNewButton,
            Button,
            Node {
                width: Val::Px(350.0),
                padding: UiRect::axes(Val::Px(20.0), Val::Px(12.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                margin: UiRect::top(Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.2, 0.3, 0.2)),
        ))
        .id();

    let create_text = commands
        .spawn((
            Text::new("+ Create New Character"),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::srgb(0.8, 0.9, 0.8)),
        ))
        .id();

    commands.entity(create_btn).add_child(create_text);
    commands.entity(root).add_child(create_btn);
}

fn handle_character_buttons(
    buttons: Query<(&Interaction, &CharacterButton), Changed<Interaction>>,
    mut outbox: ResMut<MessageOutbox<ClientMessage>>,
) {
    for (interaction, char_btn) in &buttons {
        if *interaction == Interaction::Pressed {
            crate::plugins::game::send_message(
                &mut outbox,
                ClientMessage::SelectCharacter {
                    character_id: char_btn.0,
                },
            );
        }
    }
}

fn handle_create_new_button(
    buttons: Query<&Interaction, (Changed<Interaction>, With<CreateNewButton>)>,
    mut next_screen: ResMut<NextState<GameScreen>>,
) {
    for interaction in &buttons {
        if *interaction == Interaction::Pressed {
            next_screen.set(GameScreen::CharacterCreate);
        }
    }
}

fn check_enter_world(state: Res<ClientState>, mut next_screen: ResMut<NextState<GameScreen>>) {
    if state.is_changed() && state.phase == ClientPhase::InWorld {
        next_screen.set(GameScreen::InGame);
    }
}

fn despawn_ui(mut commands: Commands, query: Query<Entity, With<CharacterSelectRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
