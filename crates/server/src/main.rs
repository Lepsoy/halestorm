mod plugins;

use bevy::prelude::*;
use plugins::game::ServerGamePlugin;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(bevy::log::LogPlugin::default())
        .add_plugins(ServerGamePlugin)
        .add_systems(Startup, || info!("Halestorm server starting"))
        .run();
}
