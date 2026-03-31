use bevy::prelude::*;
use halestorm_common::map_loader::ParsedMap;
use halestorm_common::types::TilePosition;

pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_map);
    }
}

/// Marker component for tilemap sprite entities.
#[derive(Component)]
struct TileSprite;

/// Resource holding the parsed map data for rendering reference.
#[derive(Resource)]
#[allow(dead_code)]
pub struct MapData {
    pub parsed: ParsedMap,
}

fn load_map(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    // Load the map
    let map_path = std::path::Path::new("assets/maps/test_map.tmj");
    let parsed = match halestorm_common::map_loader::load_tmj(map_path) {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to load map: {e}");
            return;
        }
    };

    info!(
        "Map loaded: {}x{}, {} blocked tiles, spawn at ({}, {})",
        parsed.width,
        parsed.height,
        parsed.collision_map.blocked_count(),
        parsed.spawn_point.x,
        parsed.spawn_point.y,
    );

    // Load tileset texture and create atlas layout
    let texture: Handle<Image> = asset_server.load("sprites/terrain.png");
    let layout = TextureAtlasLayout::from_grid(UVec2::new(32, 32), 4, 1, None, None);
    let layout_handle = texture_atlas_layouts.add(layout);

    let tile_size = parsed.tile_size as f32;

    // Render ground layer
    for (i, &tile_id) in parsed.ground_tiles.iter().enumerate() {
        if tile_id == 0 {
            continue;
        }
        let x = (i as i32) % parsed.width;
        let y = (i as i32) / parsed.width;

        // Convert tile coords to world coords (Bevy Y is up, tile Y is down)
        let world_x = x as f32 * tile_size;
        let world_y = -(y as f32) * tile_size;

        commands.spawn((
            Sprite {
                image: texture.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: layout_handle.clone(),
                    index: (tile_id - 1) as usize, // Tiled gids are 1-indexed
                }),
                ..default()
            },
            Transform::from_xyz(world_x, world_y, 0.0),
            TileSprite,
        ));
    }

    // Render wall layer on top
    for (i, &tile_id) in parsed.wall_tiles.iter().enumerate() {
        if tile_id == 0 {
            continue;
        }
        let x = (i as i32) % parsed.width;
        let y = (i as i32) / parsed.width;

        let world_x = x as f32 * tile_size;
        let world_y = -(y as f32) * tile_size;

        commands.spawn((
            Sprite {
                image: texture.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: layout_handle.clone(),
                    index: (tile_id - 1) as usize,
                }),
                ..default()
            },
            Transform::from_xyz(world_x, world_y, 1.0), // z=1 above ground
            TileSprite,
        ));
    }

    // Store collision map as resource for the server plugin
    commands.insert_resource(parsed.collision_map.clone());
    commands.insert_resource(MapData { parsed });
}

/// Convert a TilePosition to world coordinates.
#[allow(dead_code)]
pub fn tile_to_world(pos: TilePosition, tile_size: f32) -> Vec2 {
    Vec2::new(pos.x as f32 * tile_size, -(pos.y as f32) * tile_size)
}
