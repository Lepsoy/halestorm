use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_systems(Startup, startup)
        .run();
}

fn startup() {
    info!("Halestorm server starting");
}
