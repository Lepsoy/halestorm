use bevy::prelude::*;
use halestorm_common::map_loader;

use super::game::ServerState;

/// Server-side world plugin: loads map data (collision, spawn points).
/// In single-player mode, the client's RenderingPlugin also loads the map
/// for visual rendering. This plugin loads only the data the server needs.
pub struct ServerWorldPlugin;

impl Plugin for ServerWorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_map_data);
    }
}

fn load_map_data(mut commands: Commands, mut server_state: ResMut<ServerState>) {
    let map_path = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/maps/test_map.tmj"
    ));
    let parsed = map_loader::load_tmj(map_path).expect("Failed to load map — server cannot start");

    info!(
        "Server loaded map: {}x{}, {} blocked tiles, spawn at ({}, {})",
        parsed.width,
        parsed.height,
        parsed.collision_map.blocked_count(),
        parsed.spawn_point.x,
        parsed.spawn_point.y,
    );

    server_state.spawn_point = parsed.spawn_point;
    commands.insert_resource(parsed.collision_map);
}
