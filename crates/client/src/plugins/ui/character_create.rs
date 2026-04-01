use bevy::prelude::*;
use halestorm_common::protocol::ClientMessage;
use halestorm_common::transport::MessageOutbox;
use halestorm_common::types::PrimaryClass;
use strum::IntoEnumIterator;

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
                    handle_class_buttons,
                    handle_create_button,
                    handle_enter_key,
                    check_enter_world,
                    skip_to_enter_if_has_character,
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

#[derive(Component)]
struct ClassButton(PrimaryClass);

#[derive(Resource, Default)]
struct SelectedClass(Option<PrimaryClass>);

fn spawn_ui(mut commands: Commands) {
    commands.init_resource::<SelectedClass>();

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

    // Class selection label
    let class_label = commands
        .spawn((
            Text::new("Choose your class:"),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::srgb(0.7, 0.7, 0.7)),
        ))
        .id();

    // Class buttons grid (2 rows of 3)
    let class_grid = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            ..default()
        })
        .id();

    let classes: Vec<PrimaryClass> = PrimaryClass::iter().collect();
    for row in classes.chunks(3) {
        let row_node = commands
            .spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
                ..default()
            })
            .id();

        for &class in row {
            let btn = commands
                .spawn((
                    ClassButton(class),
                    Button,
                    Node {
                        width: Val::Px(140.0),
                        padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.2, 0.3)),
                ))
                .id();

            let label = commands
                .spawn((
                    Text::new(class.to_string()),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.8, 0.8, 0.8)),
                ))
                .id();

            commands.entity(btn).add_child(label);
            commands.entity(row_node).add_child(btn);
        }

        commands.entity(class_grid).add_child(row_node);
    }

    // Name input
    let name_label = commands
        .spawn((
            Text::new("Character name:"),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::srgb(0.7, 0.7, 0.7)),
        ))
        .id();

    let name_input = commands
        .spawn((
            TextInput::new("Enter name"),
            NameField,
            Text::new("Enter name"),
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

    commands.entity(root).add_children(&[
        title,
        class_label,
        class_grid,
        name_label,
        name_box,
        btn,
        status,
    ]);
}

fn handle_class_buttons(
    buttons: Query<(&Interaction, &ClassButton), Changed<Interaction>>,
    mut selected: ResMut<SelectedClass>,
    mut bg_query: Query<(&ClassButton, &mut BackgroundColor)>,
) {
    for (interaction, class_btn) in &buttons {
        if *interaction == Interaction::Pressed {
            selected.0 = Some(class_btn.0);

            // Update button colors
            for (btn, mut bg) in &mut bg_query {
                if btn.0 == class_btn.0 {
                    *bg = BackgroundColor(Color::srgb(0.3, 0.5, 0.3));
                } else {
                    *bg = BackgroundColor(Color::srgb(0.2, 0.2, 0.3));
                }
            }
        }
    }
}

fn handle_create_button(
    buttons: Query<&Interaction, (Changed<Interaction>, With<CreateButton>)>,
    name_q: Query<&TextInput, With<NameField>>,
    selected: Res<SelectedClass>,
    mut outbox: ResMut<MessageOutbox<ClientMessage>>,
    mut status_q: Query<&mut Text, With<StatusText>>,
) {
    for interaction in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        try_create(&name_q, &selected, &mut outbox, &mut status_q);
    }
}

fn handle_enter_key(
    keyboard: Res<ButtonInput<KeyCode>>,
    name_q: Query<&TextInput, With<NameField>>,
    selected: Res<SelectedClass>,
    mut outbox: ResMut<MessageOutbox<ClientMessage>>,
    mut status_q: Query<&mut Text, With<StatusText>>,
) {
    if keyboard.just_pressed(KeyCode::Enter) {
        try_create(&name_q, &selected, &mut outbox, &mut status_q);
    }
}

fn try_create(
    name_q: &Query<&TextInput, With<NameField>>,
    selected: &Res<SelectedClass>,
    outbox: &mut ResMut<MessageOutbox<ClientMessage>>,
    status_q: &mut Query<&mut Text, With<StatusText>>,
) {
    let Some(class) = selected.0 else {
        if let Ok(mut text) = status_q.single_mut() {
            **text = "Please select a class".to_string();
        }
        return;
    };

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

    crate::plugins::game::send_message(outbox, ClientMessage::CreateCharacter { name, class });
    crate::plugins::game::send_message(outbox, ClientMessage::EnterWorld);

    if let Ok(mut text) = status_q.single_mut() {
        **text = "Entering world...".to_string();
    }
}

/// If the account already has a character, skip creation and enter world directly.
fn skip_to_enter_if_has_character(
    state: Res<ClientState>,
    mut outbox: ResMut<MessageOutbox<ClientMessage>>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    // The server loads the character on login. If the client gets LoginSuccess
    // and the server already has character data, EnterWorld will work immediately.
    // We check if the client has character info from a previous session.
    if state.phase == ClientPhase::LoggedIn && state.has_character {
        crate::plugins::game::send_message(&mut outbox, ClientMessage::EnterWorld);
        *done = true;
    }
}

fn check_enter_world(state: Res<ClientState>, mut next_screen: ResMut<NextState<GameScreen>>) {
    if state.is_changed() && state.phase == ClientPhase::InWorld {
        next_screen.set(GameScreen::InGame);
    }
}

fn despawn_ui(mut commands: Commands, query: Query<Entity, With<CharacterCreateRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<SelectedClass>();
}
