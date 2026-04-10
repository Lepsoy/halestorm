use bevy::prelude::*;
use halestorm_common::map_loader::{ParsedMap, TilesetInfo};
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
    // Load the map for rendering (server loads its own copy for game logic)
    let map_path = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/maps/test_map.tmj"
    ));
    let parsed = halestorm_common::map_loader::load_tmj(map_path)
        .expect("Failed to load map — cannot start without it");

    info!(
        "Map loaded: {}x{}, {} blocked tiles, spawn at ({}, {})",
        parsed.width,
        parsed.height,
        parsed.collision_map.blocked_count(),
        parsed.spawn_point.x,
        parsed.spawn_point.y,
    );

    // Load all tilesets referenced by the map
    let mut loaded_tilesets: Vec<(u32, Handle<Image>, Handle<TextureAtlasLayout>)> = Vec::new();
    for ts in &parsed.tilesets {
        // Tileset image paths in .tmj are relative to the map file (e.g. "../sprites/foo.png").
        // Strip any leading path components to get the asset-relative path.
        let asset_path = ts
            .image
            .strip_prefix("../")
            .unwrap_or(&ts.image)
            .to_string();
        let texture: Handle<Image> = asset_server.load(&asset_path);
        let layout = TextureAtlasLayout::from_grid(
            UVec2::new(parsed.tile_size as u32, parsed.tile_size as u32),
            ts.columns,
            ts.rows,
            None,
            None,
        );
        let layout_handle = texture_atlas_layouts.add(layout);
        loaded_tilesets.push((ts.firstgid, texture, layout_handle));
    }

    let tile_size = parsed.tile_size as f32;

    // Render ground layer
    for (i, &tile_id) in parsed.ground_tiles.iter().enumerate() {
        if tile_id == 0 {
            continue;
        }
        if let Some(sprite) = resolve_tile(&loaded_tilesets, &parsed.tilesets, tile_id) {
            let x = (i as i32) % parsed.width;
            let y = (i as i32) / parsed.width;
            let world_x = x as f32 * tile_size;
            let world_y = -(y as f32) * tile_size;
            commands.spawn((sprite, Transform::from_xyz(world_x, world_y, 0.0), TileSprite));
        }
    }

    // Render wall layer on top
    for (i, &tile_id) in parsed.wall_tiles.iter().enumerate() {
        if tile_id == 0 {
            continue;
        }
        if let Some(sprite) = resolve_tile(&loaded_tilesets, &parsed.tilesets, tile_id) {
            let x = (i as i32) % parsed.width;
            let y = (i as i32) / parsed.width;
            let world_x = x as f32 * tile_size;
            let world_y = -(y as f32) * tile_size;
            commands.spawn((sprite, Transform::from_xyz(world_x, world_y, 1.0), TileSprite));
        }
    }

    commands.insert_resource(MapData { parsed });
}

/// Resolve a Tiled global tile ID to a Sprite using the correct tileset.
fn resolve_tile(
    loaded: &[(u32, Handle<Image>, Handle<TextureAtlasLayout>)],
    tilesets: &[TilesetInfo],
    gid: u32,
) -> Option<Sprite> {
    // Find which tileset this gid belongs to (last one whose firstgid <= gid)
    let ts_idx = tilesets.iter().rposition(|ts| ts.firstgid <= gid)?;
    let ts = &tilesets[ts_idx];
    let (_, ref texture, ref layout_handle) = loaded[ts_idx];
    let local_id = gid - ts.firstgid;
    if local_id >= ts.tile_count {
        return None;
    }
    Some(Sprite {
        image: texture.clone(),
        texture_atlas: Some(TextureAtlas {
            layout: layout_handle.clone(),
            index: local_id as usize,
        }),
        ..default()
    })
}

/// Convert a TilePosition to world coordinates.
pub fn tile_to_world(pos: TilePosition, tile_size: f32) -> Vec2 {
    Vec2::new(pos.x as f32 * tile_size, -(pos.y as f32) * tile_size)
}
