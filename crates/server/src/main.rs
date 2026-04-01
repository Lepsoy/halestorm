mod plugins;

use bevy::prelude::*;
use plugins::game::ServerGamePlugin;
use plugins::persistence::PersistencePlugin;
use plugins::world::ServerWorldPlugin;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(bevy::log::LogPlugin::default())
        .add_plugins(PersistencePlugin)
        .add_plugins(ServerGamePlugin)
        .add_plugins(ServerWorldPlugin)
        .add_systems(Startup, || info!("Halestorm server starting"))
        .run();
}
