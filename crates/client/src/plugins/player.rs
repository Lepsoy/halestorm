use bevy::prelude::*;
use halestorm_common::types::{Direction, TilePosition};

use super::camera::GameCamera;
use super::game::{ClientPhase, ClientState};
use super::rendering::tile_to_world;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                spawn_player_on_enter,
                update_movement,
                camera_follow_player,
            ),
        );
    }
}

/// Marker component for the local player sprite.
#[derive(Component)]
pub struct LocalPlayer;

/// Tracks the player's visual movement between tiles.
#[derive(Resource)]
pub struct PlayerMovement {
    pub from: TilePosition,
    pub to: TilePosition,
    pub direction: Direction,
    pub progress: f32,
    pub moving: bool,
    /// Duration of one tile transition in seconds.
    pub move_duration: f32,
}

impl Default for PlayerMovement {
    fn default() -> Self {
        Self {
            from: TilePosition::default(),
            to: TilePosition::default(),
            direction: Direction::South,
            progress: 0.0,
            moving: false,
            move_duration: 0.15,
        }
    }
}

impl PlayerMovement {
    pub fn is_moving(&self) -> bool {
        self.moving
    }

    pub fn start_move(&mut self, from: TilePosition, to: TilePosition, direction: Direction) {
        self.from = from;
        self.to = to;
        self.direction = direction;
        self.progress = 0.0;
        self.moving = true;
    }
}

const TILE_SIZE: f32 = 32.0;

fn spawn_player_on_enter(
    mut commands: Commands,
    state: Res<ClientState>,
    asset_server: Res<AssetServer>,
    player_query: Query<&LocalPlayer>,
    mut has_spawned: Local<bool>,
) {
    if state.phase != ClientPhase::InWorld || *has_spawned {
        return;
    }
    // Don't spawn twice
    if !player_query.is_empty() {
        *has_spawned = true;
        return;
    }

    let Some(position) = state.position else {
        return;
    };

    let world_pos = tile_to_world(position, TILE_SIZE);
    let texture: Handle<Image> = asset_server.load("sprites/player.png");

    commands.spawn((
        Sprite {
            image: texture,
            ..default()
        },
        // z=10 to render above all tile layers
        Transform::from_xyz(world_pos.x, world_pos.y, 10.0),
        LocalPlayer,
    ));

    commands.insert_resource(PlayerMovement {
        from: position,
        to: position,
        ..default()
    });

    info!("Player sprite spawned at ({}, {})", position.x, position.y);
    *has_spawned = true;
}

fn update_movement(
    time: Res<Time>,
    player_mov: Option<ResMut<PlayerMovement>>,
    mut player_query: Query<&mut Transform, With<LocalPlayer>>,
) {
    let Some(mut mov) = player_mov else {
        return;
    };
    let Ok(mut transform) = player_query.single_mut() else {
        return;
    };

    if mov.moving {
        mov.progress += time.delta_secs() / mov.move_duration;

        if mov.progress >= 1.0 {
            // Transition complete
            mov.progress = 1.0;
            mov.moving = false;
            mov.from = mov.to;
        }

        // Lerp between from and to positions
        let from_world = tile_to_world(mov.from, TILE_SIZE);
        let to_world = tile_to_world(mov.to, TILE_SIZE);
        let lerped = from_world.lerp(to_world, mov.progress);

        transform.translation.x = lerped.x;
        transform.translation.y = lerped.y;

        // Y-sorting: lower on screen (higher Y in tile space = lower Bevy Y) draws on top
        transform.translation.z = 10.0 - transform.translation.y * 0.001;
    }
}

fn camera_follow_player(
    player_query: Query<&Transform, (With<LocalPlayer>, Without<GameCamera>)>,
    mut camera_query: Query<&mut Transform, (With<GameCamera>, Without<LocalPlayer>)>,
    time: Res<Time>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let Ok(mut camera_transform) = camera_query.single_mut() else {
        return;
    };

    let target = Vec3::new(
        player_transform.translation.x,
        player_transform.translation.y,
        camera_transform.translation.z,
    );

    // Smooth camera follow with lerp
    let speed = 8.0;
    camera_transform.translation = camera_transform
        .translation
        .lerp(target, speed * time.delta_secs());
}
