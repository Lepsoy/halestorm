use bevy::prelude::*;
use halestorm_common::protocol::EntityKind;
use halestorm_common::types::EntityId;
use std::collections::{HashMap, HashSet};

use super::animation::{SpriteAnimation, idle_index, lpc_atlas_layout};
use super::game::ClientState;
use super::rendering::tile_to_world;

pub struct EntitiesPlugin;

impl Plugin for EntitiesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RemoteEntities>()
            .add_systems(Update, sync_remote_entities);
    }
}

/// Tracks spawned remote entity sprites.
#[derive(Resource, Default)]
struct RemoteEntities {
    entities: HashMap<EntityId, Entity>,
}

/// Marker for remote entity sprites.
#[derive(Component)]
#[allow(dead_code)]
struct RemoteEntity(EntityId);

const TILE_SIZE: f32 = 32.0;

fn sync_remote_entities(
    mut commands: Commands,
    mut state: ResMut<ClientState>,
    mut remote: ResMut<RemoteEntities>,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut query: Query<(&mut Transform, &mut SpriteAnimation), With<RemoteEntity>>,
) {
    let Some((_tick, ref entities)) = state.latest_snapshot else {
        return;
    };

    let my_entity = state.entity_id;
    let mut seen = HashSet::new();

    for entity_state in entities {
        // Skip our own entity
        if Some(entity_state.entity_id) == my_entity {
            continue;
        }

        seen.insert(entity_state.entity_id);
        let world_pos = tile_to_world(entity_state.position, TILE_SIZE);

        if let Some(&bevy_entity) = remote.entities.get(&entity_state.entity_id) {
            // Update existing entity position
            if let Ok((mut transform, mut anim)) = query.get_mut(bevy_entity) {
                transform.translation.x = world_pos.x;
                transform.translation.y = world_pos.y;
                transform.translation.z = 10.0 - world_pos.y * 0.001;
                anim.facing = entity_state.direction;
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

            remote.entities.insert(entity_state.entity_id, bevy_entity);
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
        if let Some(bevy_entity) = remote.entities.remove(&id) {
            commands.entity(bevy_entity).despawn();
        }
    }

    // Clear snapshot after processing
    state.latest_snapshot = None;
}
