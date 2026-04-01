use bevy::prelude::*;
use halestorm_common::protocol::ClientMessage;
use halestorm_common::transport::MessageOutbox;

use super::text_input::{
    TextInput, TextInputLink, handle_tab_focus, handle_text_input_focus, update_text_inputs,
};
use super::GameScreen;
use crate::plugins::game::{ClientPhase, ClientState};

pub struct CharacterCreatePlugin;

impl Plugin for CharacterCreatePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameScreen::CharacterCreate), spawn_ui)
            .add_systems(OnExit(GameScreen::CharacterCreate), despawn_ui)
            .add_systems(
                Update,
                (
                    update_text_inputs,
                    handle_text_input_focus,
                    handle_tab_focus,
                    handle_create_button,
                    handle_enter_key,
                    check_enter_world,
                )
                    .distributive_run_if(in_state(GameScreen::CharacterCreate)),
            );
    }
}

#[derive(Component)]
struct CharacterCreateRoot;

#[derive(Component)]
struct NameField;

#[derive(Component)]
struct CreateButton;

#[derive(Component)]
struct StatusText;

fn spawn_ui(mut commands: Commands) {
    let root = commands
        .spawn((
            CharacterCreateRoot,
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
            Text::new("Create Character"),
            TextFont {
                font_size: 36.0,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.8, 0.3)),
        ))
        .id();

    // Name input
    let name_input = commands
        .spawn((
            TextInput::new("Character name"),
            NameField,
            Text::new("Character name"),
            TextFont {
                font_size: 20.0,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
        ))
        .id();

    let name_box = commands
        .spawn((
            Node {
                width: Val::Px(300.0),
                height: Val::Px(40.0),
                padding: UiRect::all(Val::Px(8.0)),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.15, 0.15, 0.2)),
            Interaction::default(),
            TextInputLink(name_input),
        ))
        .id();

    commands.entity(name_box).add_child(name_input);

    // Create button
    let btn = commands
        .spawn((
            CreateButton,
            Button,
            Node {
                padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.25, 0.25, 0.35)),
        ))
        .id();

    let btn_text = commands
        .spawn((
            Text::new("Create & Enter World"),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
        ))
        .id();

    commands.entity(btn).add_child(btn_text);

    // Status
    let status = commands
        .spawn((
            StatusText,
            Text::new(""),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.4, 0.4)),
        ))
        .id();

    commands
        .entity(root)
        .add_children(&[title, name_box, btn, status]);
}

fn handle_create_button(
    buttons: Query<&Interaction, (Changed<Interaction>, With<CreateButton>)>,
    name_q: Query<&TextInput, With<NameField>>,
    mut outbox: ResMut<MessageOutbox<ClientMessage>>,
    mut status_q: Query<&mut Text, With<StatusText>>,
) {
    for interaction in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        try_create_character(&name_q, &mut outbox, &mut status_q);
    }
}

fn handle_enter_key(
    keyboard: Res<ButtonInput<KeyCode>>,
    name_q: Query<&TextInput, With<NameField>>,
    mut outbox: ResMut<MessageOutbox<ClientMessage>>,
    mut status_q: Query<&mut Text, With<StatusText>>,
) {
    if keyboard.just_pressed(KeyCode::Enter) {
        try_create_character(&name_q, &mut outbox, &mut status_q);
    }
}

fn try_create_character(
    name_q: &Query<&TextInput, With<NameField>>,
    outbox: &mut ResMut<MessageOutbox<ClientMessage>>,
    status_q: &mut Query<&mut Text, With<StatusText>>,
) {
    let Ok(name_input) = name_q.single() else {
        return;
    };

    let name = name_input.value.trim().to_string();
    if name.is_empty() {
        if let Ok(mut text) = status_q.single_mut() {
            **text = "Please enter a character name".to_string();
        }
        return;
    }

    crate::plugins::game::send_message(
        outbox,
        ClientMessage::CreateCharacter { name },
    );
    // Also immediately request to enter the world
    crate::plugins::game::send_message(outbox, ClientMessage::EnterWorld);

    if let Ok(mut text) = status_q.single_mut() {
        **text = "Entering world...".to_string();
    }
}

fn check_enter_world(
    state: Res<ClientState>,
    mut next_screen: ResMut<NextState<GameScreen>>,
) {
    if state.is_changed() && state.phase == ClientPhase::InWorld {
        next_screen.set(GameScreen::InGame);
    }
}

fn despawn_ui(mut commands: Commands, query: Query<Entity, With<CharacterCreateRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
