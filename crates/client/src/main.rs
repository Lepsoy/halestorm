mod plugins;

use bevy::prelude::*;
use halestorm_common::local_transport_plugin::LocalTransportPlugin;
use halestorm_common::protocol::ClientMessage;
use halestorm_common::transport::MessageOutbox;
use halestorm_server::plugins::game::ServerGamePlugin;
use plugins::camera::CameraPlugin;
use plugins::game::ClientGamePlugin;
use plugins::rendering::RenderingPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Halestorm".to_string(),
                resolution: (1280u32, 720u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.1, 0.1, 0.15)))
        // Camera
        .add_plugins(CameraPlugin)
        // Map rendering
        .add_plugins(RenderingPlugin)
        // Embedded server (single-player mode)
        .add_plugins(ServerGamePlugin)
        // Local transport (in-process channels)
        .add_plugins(LocalTransportPlugin)
        // Client game logic
        .add_plugins(ClientGamePlugin)
        // Startup: auto-login for testing
        .add_systems(Startup, || info!("Halestorm client starting (single-player mode)"))
        .add_systems(Update, auto_login)
        .run();
}

/// Temporary auto-login sequence for testing the transport.
/// Creates an account, logs in, creates a character, and enters the world.
fn auto_login(
    mut outbox: ResMut<MessageOutbox<ClientMessage>>,
    state: Res<plugins::game::ClientState>,
    mut step: Local<u8>,
) {
    use plugins::game::{send_message, ClientPhase};

    match *step {
        0 => {
            send_message(
                &mut outbox,
                ClientMessage::CreateAccount {
                    username: "test".into(),
                    password: "test".into(),
                },
            );
            *step = 1;
        }
        1 => {
            send_message(
                &mut outbox,
                ClientMessage::Login {
                    username: "test".into(),
                    password: "test".into(),
                },
            );
            *step = 2;
        }
        2 if state.phase == ClientPhase::LoggedIn => {
            send_message(
                &mut outbox,
                ClientMessage::CreateCharacter {
                    name: "Hero".into(),
                },
            );
            *step = 3;
        }
        3 if state.phase == ClientPhase::LoggedIn => {
            send_message(&mut outbox, ClientMessage::EnterWorld);
            *step = 4;
        }
        4 if state.phase == ClientPhase::InWorld => {
            info!("In world! Position: {:?}", state.position);
            *step = 5; // Done, stop sending
        }
        _ => {}
    }
}
