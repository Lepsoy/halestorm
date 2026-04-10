use bevy::prelude::*;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera);
    }
}

/// Marker for the main game camera.
#[derive(Component)]
pub struct GameCamera;

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        GameCamera,
        Projection::from(OrthographicProjection {
            scale: 0.6,
            ..OrthographicProjection::default_2d()
        }),
    ));
}
