mod plugins;

use bevy::prelude::*;
use halestorm_common::local_transport_plugin::LocalTransportPlugin;
use halestorm_server::plugins::game::ServerGamePlugin;
use halestorm_server::plugins::world::ServerWorldPlugin;
use plugins::camera::CameraPlugin;
use plugins::game::ClientGamePlugin;
use plugins::input::InputPlugin;
use plugins::player::PlayerPlugin;
use plugins::rendering::RenderingPlugin;
use plugins::ui::UiPlugin;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Halestorm".to_string(),
                        resolution: (1280u32, 720u32).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    // Assets are at the workspace root, not next to the client crate
                    file_path: concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets").to_string(),
                    ..default()
                }),
        )
        .insert_resource(ClearColor(Color::srgb(0.1, 0.1, 0.15)))
        // Camera
        .add_plugins(CameraPlugin)
        // Map rendering
        .add_plugins(RenderingPlugin)
        // Embedded server (single-player mode)
        .add_plugins(ServerGamePlugin)
        .add_plugins(ServerWorldPlugin)
        // Local transport (in-process channels)
        .add_plugins(LocalTransportPlugin)
        // Client game logic
        .add_plugins(ClientGamePlugin)
        // Player sprite and movement interpolation
        .add_plugins(PlayerPlugin)
        // WASD input
        .add_plugins(InputPlugin)
        // UI screens (login, character create, HUD)
        .add_plugins(UiPlugin)
        .add_systems(Startup, || info!("Halestorm client starting (single-player mode)"))
        .run();
}
