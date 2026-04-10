use bevy::prelude::*;
use halestorm_common::protocol::EntityKind;
use halestorm_common::types::{Direction, EntityId, TilePosition};
use std::collections::{HashMap, HashSet};

use super::animation::{SpriteAnimation, idle_index, lpc_atlas_layout, walk_index};
use super::game::ClientState;
use super::rendering::tile_to_world;

pub struct EntitiesPlugin;

impl Plugin for EntitiesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RemoteEntities>()
            .add_systems(Update, (process_snapshot, interpolate_remote_entities).chain());
    }
}

/// Tracks spawned remote entity sprites and their interpolation state.
#[derive(Resource, Default)]
struct RemoteEntities {
    entities: HashMap<EntityId, RemoteEntityData>,
}

struct RemoteEntityData {
    bevy_entity: Entity,
    from_pos: TilePosition,
    to_pos: TilePosition,
    direction: Direction,
    progress: f32,
    /// Duration to interpolate over — measured from actual position change intervals.
    interp_duration: f32,
    /// Time of last position change, for measuring move intervals.
    last_move_time: f32,
}

/// Marker for remote entity sprites.
#[derive(Component)]
#[allow(dead_code)]
struct RemoteEntity(EntityId);

const TILE_SIZE: f32 = 32.0;
/// Default interpolation duration before we measure the actual move interval.
const DEFAULT_INTERP_DURATION: f32 = 0.3;

fn process_snapshot(
    mut commands: Commands,
    mut state: ResMut<ClientState>,
    mut remote: ResMut<RemoteEntities>,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    time: Res<Time>,
) {
    let Some((_tick, ref entities)) = state.latest_snapshot else {
        return;
    };

    let my_entity = state.entity_id;
    let mut seen = HashSet::new();

    for entity_state in entities {
        if Some(entity_state.entity_id) == my_entity {
            continue;
        }

        seen.insert(entity_state.entity_id);
        let new_pos = entity_state.position;

        if let Some(data) = remote.entities.get_mut(&entity_state.entity_id) {
            // Entity already exists — update interpolation target
            if data.to_pos != new_pos {
                // Measure time since last move to set interpolation duration
                let now = time.elapsed_secs();
                let interval = now - data.last_move_time;
                if interval > 0.05 && interval < 2.0 {
                    // Use measured interval, slightly padded for smoothness
                    data.interp_duration = interval * 1.05;
                }
                data.last_move_time = now;

                data.from_pos = data.to_pos;
                data.to_pos = new_pos;
                data.progress = 0.0;
                data.direction = entity_state.direction;
            }
        } else {
            // Spawn new entity
            let sprite_file = match &entity_state.kind {
                EntityKind::Player { class } => class.sprite_file().to_string(),
                EntityKind::Monster { kind, .. } => kind.sprite_file().to_string(),
            };

            let texture: Handle<Image> = asset_server.load(&sprite_file);
            let layout = lpc_atlas_layout();
            let layout_handle = texture_atlas_layouts.add(layout);
            let idle = idle_index(entity_state.direction);
            let world_pos = tile_to_world(new_pos, TILE_SIZE);

            let bevy_entity = commands
                .spawn((
                    Sprite {
                        image: texture,
                        texture_atlas: Some(TextureAtlas {
                            layout: layout_handle,
                            index: idle,
                        }),
                        ..default()
                    },
                    Transform::from_xyz(world_pos.x, world_pos.y, 10.0 - world_pos.y * 0.001),
                    RemoteEntity(entity_state.entity_id),
                    SpriteAnimation::default(),
                ))
                .id();

            remote.entities.insert(
                entity_state.entity_id,
                RemoteEntityData {
                    bevy_entity,
                    from_pos: new_pos,
                    to_pos: new_pos,
                    direction: entity_state.direction,
                    progress: 1.0,
                    interp_duration: DEFAULT_INTERP_DURATION,
                    last_move_time: time.elapsed_secs(),
                },
            );
        }
    }

    // Despawn entities no longer in snapshot
    let to_remove: Vec<EntityId> = remote
        .entities
        .keys()
        .filter(|id| !seen.contains(id))
        .copied()
        .collect();

    for id in to_remove {
        if let Some(data) = remote.entities.remove(&id) {
            commands.entity(data.bevy_entity).despawn();
        }
    }

    state.latest_snapshot = None;
}

fn interpolate_remote_entities(
    time: Res<Time>,
    mut remote: ResMut<RemoteEntities>,
    mut query: Query<(&mut Transform, &mut Sprite, &mut SpriteAnimation), With<RemoteEntity>>,
) {
    for data in remote.entities.values_mut() {
        let is_moving = data.progress < 1.0;
        if is_moving {
            data.progress += time.delta_secs() / data.interp_duration;
            if data.progress > 1.0 {
                data.progress = 1.0;
            }
        }

        let from_world = tile_to_world(data.from_pos, TILE_SIZE);
        let to_world = tile_to_world(data.to_pos, TILE_SIZE);
        let lerped = from_world.lerp(to_world, data.progress);

        if let Ok((mut transform, mut sprite, mut anim)) = query.get_mut(data.bevy_entity) {
            transform.translation.x = lerped.x;
            transform.translation.y = lerped.y;
            transform.translation.z = 10.0 - lerped.y * 0.001;
            anim.facing = data.direction;

            let index = if data.progress < 1.0 {
                // Walking: cycle through frames 1..=8
                anim.timer.tick(time.delta());
                if anim.timer.just_finished() {
                    anim.frame = (anim.frame % 8) + 1;
                }
                walk_index(data.direction, anim.frame.max(1))
            } else {
                // Idle
                anim.frame = 0;
                idle_index(anim.facing)
            };

            if let Some(ref mut atlas) = sprite.texture_atlas {
                atlas.index = index;
            }
        }
    }
}
