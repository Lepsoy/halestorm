use bevy::prelude::*;
use halestorm_common::protocol::ClientMessage;
use halestorm_common::transport::MessageOutbox;

use super::text_input::{
    TextInput, TextInputLink, handle_tab_focus, handle_text_input_focus, update_text_inputs,
};
use super::GameScreen;
use crate::plugins::game::{ClientPhase, ClientState};

pub struct LoginPlugin;

impl Plugin for LoginPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameScreen::Login), spawn_login_ui)
            .add_systems(OnExit(GameScreen::Login), despawn_login_ui)
            .add_systems(
                Update,
                (
                    update_text_inputs,
                    handle_text_input_focus,
                    handle_tab_focus,
                    handle_login_buttons,
                    handle_enter_key,
                    check_login_success,
                )
                    .distributive_run_if(in_state(GameScreen::Login)),
            );
    }
}

#[derive(Component)]
struct LoginUiRoot;

#[derive(Component)]
enum LoginButton {
    Login,
    CreateAccount,
}

#[derive(Component)]
struct UsernameField;

#[derive(Component)]
struct PasswordField;

#[derive(Component)]
struct StatusText;

fn spawn_login_ui(mut commands: Commands) {
    let root = commands
        .spawn((
            LoginUiRoot,
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
            Text::new("HALESTORM"),
            TextFont {
                font_size: 48.0,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.8, 0.3)),
        ))
        .id();

    // Username field
    let username_input = commands
        .spawn((
            TextInput::new("Username"),
            UsernameField,
            Text::new("Username"),
            TextFont {
                font_size: 20.0,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
        ))
        .id();

    let username_box = commands
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
            TextInputLink(username_input),
        ))
        .id();

    commands.entity(username_box).add_child(username_input);

    // Password field
    let password_input = commands
        .spawn((
            TextInput::password("Password"),
            PasswordField,
            Text::new("Password"),
            TextFont {
                font_size: 20.0,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
        ))
        .id();

    let password_box = commands
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
            TextInputLink(password_input),
        ))
        .id();

    commands.entity(password_box).add_child(password_input);

    // Buttons row
    let button_row = commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(12.0),
            ..default()
        })
        .id();

    let login_btn = spawn_button(&mut commands, "Login", LoginButton::Login);
    let create_btn = spawn_button(&mut commands, "Create Account", LoginButton::CreateAccount);
    commands
        .entity(button_row)
        .add_children(&[login_btn, create_btn]);

    // Status text
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
        .add_children(&[title, username_box, password_box, button_row, status]);
}

fn spawn_button(commands: &mut Commands, label: &str, button_type: LoginButton) -> Entity {
    let btn = commands
        .spawn((
            button_type,
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

    let text = commands
        .spawn((
            Text::new(label),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
        ))
        .id();

    commands.entity(btn).add_child(text);
    btn
}

fn handle_login_buttons(
    buttons: Query<(&Interaction, &LoginButton), Changed<Interaction>>,
    username_q: Query<&TextInput, With<UsernameField>>,
    password_q: Query<&TextInput, With<PasswordField>>,
    mut outbox: ResMut<MessageOutbox<ClientMessage>>,
    mut status_q: Query<(&mut Text, &mut TextColor), With<StatusText>>,
) {
    for (interaction, button_type) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let Ok(username_input) = username_q.single() else {
            return;
        };
        let Ok(password_input) = password_q.single() else {
            return;
        };

        let username = username_input.value.trim().to_string();
        let password = password_input.value.trim().to_string();

        if username.is_empty() || password.is_empty() {
            if let Ok((mut text, mut color)) = status_q.single_mut() {
                **text = "Please enter username and password".to_string();
                *color = TextColor(Color::srgb(1.0, 0.4, 0.4));
            }
            return;
        }

        match button_type {
            LoginButton::Login => {
                crate::plugins::game::send_message(
                    &mut outbox,
                    ClientMessage::Login { username, password },
                );
                if let Ok((mut text, mut color)) = status_q.single_mut() {
                    **text = "Logging in...".to_string();
                    *color = TextColor(Color::srgb(0.7, 0.7, 0.7));
                }
            }
            LoginButton::CreateAccount => {
                crate::plugins::game::send_message(
                    &mut outbox,
                    ClientMessage::CreateAccount {
                        username: username.clone(),
                        password: password.clone(),
                    },
                );
                if let Ok((mut text, mut color)) = status_q.single_mut() {
                    **text = "Creating account...".to_string();
                    *color = TextColor(Color::srgb(0.7, 0.7, 0.7));
                }
            }
        }
    }
}

fn handle_enter_key(
    keyboard: Res<ButtonInput<KeyCode>>,
    username_q: Query<&TextInput, With<UsernameField>>,
    password_q: Query<&TextInput, With<PasswordField>>,
    mut outbox: ResMut<MessageOutbox<ClientMessage>>,
) {
    if !keyboard.just_pressed(KeyCode::Enter) {
        return;
    }

    let Ok(username_input) = username_q.single() else {
        return;
    };
    let Ok(password_input) = password_q.single() else {
        return;
    };

    let username = username_input.value.trim().to_string();
    let password = password_input.value.trim().to_string();

    if !username.is_empty() && !password.is_empty() {
        crate::plugins::game::send_message(
            &mut outbox,
            ClientMessage::Login { username, password },
        );
    }
}

fn check_login_success(
    mut state: ResMut<ClientState>,
    mut next_screen: ResMut<NextState<GameScreen>>,
    mut status_q: Query<(&mut Text, &mut TextColor), With<StatusText>>,
) {
    if !state.is_changed() {
        return;
    }

    if state.phase == ClientPhase::LoggedIn {
        next_screen.set(GameScreen::CharacterCreate);
        return;
    }

    // Show status messages from server responses
    if let Some(msg) = state.status_message.take()
        && let Ok((mut text, mut color)) = status_q.single_mut()
    {
        **text = msg;
        if state.account_created {
            *color = TextColor(Color::srgb(0.4, 1.0, 0.4));
            state.account_created = false;
        } else {
            *color = TextColor(Color::srgb(1.0, 0.4, 0.4));
        }
    }
}

fn despawn_login_ui(mut commands: Commands, query: Query<Entity, With<LoginUiRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
