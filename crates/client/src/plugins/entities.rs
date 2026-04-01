use bevy::prelude::*;
use halestorm_common::protocol::EntityKind;
use halestorm_common::types::{Direction, EntityId, TilePosition};
use std::collections::{HashMap, HashSet};

use super::animation::{SpriteAnimation, idle_index, lpc_atlas_layout};
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
    /// Duration to interpolate over (matches server tick interval).
    interp_duration: f32,
}

/// Marker for remote entity sprites.
#[derive(Component)]
#[allow(dead_code)]
struct RemoteEntity(EntityId);

const TILE_SIZE: f32 = 32.0;
/// Server sends snapshots at 20Hz = every 50ms. We interpolate over slightly
/// more than one tick to stay smooth even if a snapshot arrives late.
const INTERP_DURATION: f32 = 0.065;

fn process_snapshot(
    mut commands: Commands,
    mut state: ResMut<ClientState>,
    mut remote: ResMut<RemoteEntities>,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
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
                // New position: start interpolating from current visual position
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
                    interp_duration: INTERP_DURATION,
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
    mut query: Query<(&mut Transform, &mut SpriteAnimation), With<RemoteEntity>>,
) {
    for data in remote.entities.values_mut() {
        if data.progress < 1.0 {
            data.progress += time.delta_secs() / data.interp_duration;
            if data.progress > 1.0 {
                data.progress = 1.0;
            }
        }

        let from_world = tile_to_world(data.from_pos, TILE_SIZE);
        let to_world = tile_to_world(data.to_pos, TILE_SIZE);
        let lerped = from_world.lerp(to_world, data.progress);

        if let Ok((mut transform, mut anim)) = query.get_mut(data.bevy_entity) {
            transform.translation.x = lerped.x;
            transform.translation.y = lerped.y;
            transform.translation.z = 10.0 - lerped.y * 0.001;
            anim.facing = data.direction;
        }
    }
}
