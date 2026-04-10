use std::path::Path;

use crate::map::CollisionMap;
use crate::types::TilePosition;

/// Metadata for a single tileset referenced by a Tiled map.
#[derive(Debug, Clone)]
pub struct TilesetInfo {
    /// First global tile ID for this tileset (Tiled `firstgid`).
    pub firstgid: u32,
    /// Relative path to the tileset image (from the map file).
    pub image: String,
    /// Number of columns in the tileset atlas.
    pub columns: u32,
    /// Number of rows in the tileset atlas.
    pub rows: u32,
    /// Total tile count in this tileset.
    pub tile_count: u32,
}

/// Parsed data from a .tmj Tiled map file.
pub struct ParsedMap {
    pub width: i32,
    pub height: i32,
    pub collision_map: CollisionMap,
    pub spawn_point: TilePosition,
    pub ground_tiles: Vec<u32>,
    pub wall_tiles: Vec<u32>,
    pub tile_size: i32,
    pub tilesets: Vec<TilesetInfo>,
}

/// Load and parse a .tmj (Tiled JSON) map file.
pub fn load_tmj(path: &Path) -> Result<ParsedMap, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("Failed to read map: {e}"))?;
    parse_tmj(&content)
}

/// Parse .tmj content from a string.
pub fn parse_tmj(content: &str) -> Result<ParsedMap, String> {
    let root: serde_json::Value =
        serde_json::from_str(content).map_err(|e| format!("Invalid JSON: {e}"))?;

    let width = root["width"].as_i64().ok_or("missing width")? as i32;
    let height = root["height"].as_i64().ok_or("missing height")? as i32;
    let tile_size = root["tilewidth"].as_i64().ok_or("missing tilewidth")? as i32;

    let mut tilesets = Vec::new();
    if let Some(ts_array) = root["tilesets"].as_array() {
        for ts in ts_array {
            let firstgid = ts["firstgid"].as_u64().unwrap_or(1) as u32;
            let image = ts["image"].as_str().unwrap_or("").to_string();
            let columns = ts["columns"].as_u64().unwrap_or(1) as u32;
            let tile_count = ts["tilecount"].as_u64().unwrap_or(1) as u32;
            let rows = if columns > 0 {
                (tile_count + columns - 1) / columns
            } else {
                1
            };
            tilesets.push(TilesetInfo {
                firstgid,
                image,
                columns,
                rows,
                tile_count,
            });
        }
    }
    // Sort by firstgid so lookups work correctly
    tilesets.sort_by_key(|t| t.firstgid);

    let layers = root["layers"].as_array().ok_or("missing layers")?;

    let mut ground_tiles = Vec::new();
    let mut wall_tiles = Vec::new();
    let mut collision_map = CollisionMap::new(width, height);
    let mut spawn_point = TilePosition::new(width / 2, height / 2); // default center

    for layer in layers {
        let layer_type = layer["type"].as_str().unwrap_or("");

        match layer_type {
            "tilelayer" => {
                let name = layer["name"].as_str().unwrap_or("");
                let data: Vec<u32> = layer["data"]
                    .as_array()
                    .ok_or(format!("missing data in layer '{name}'"))?
                    .iter()
                    .map(|v| v.as_u64().unwrap_or(0) as u32)
                    .collect();

                // Check if this layer has a collision property
                let is_collision = layer
                    .get("properties")
                    .and_then(|p| p.as_array())
                    .map(|props| {
                        props.iter().any(|prop| {
                            prop["name"].as_str() == Some("collision")
                                && prop["value"].as_bool() == Some(true)
                        })
                    })
                    .unwrap_or(false);

                if is_collision {
                    // Any non-zero tile in a collision layer is blocked
                    for (i, &tile_id) in data.iter().enumerate() {
                        if tile_id != 0 {
                            let x = (i as i32) % width;
                            let y = (i as i32) / width;
                            collision_map.set_blocked(TilePosition::new(x, y));
                        }
                    }
                    wall_tiles = data;
                } else if name == "ground" {
                    ground_tiles = data;
                }
            }

            "objectgroup" => {
                if let Some(objects) = layer["objects"].as_array() {
                    for obj in objects {
                        if obj["type"].as_str() == Some("spawn_point") {
                            let px = obj["x"].as_f64().unwrap_or(0.0);
                            let py = obj["y"].as_f64().unwrap_or(0.0);
                            spawn_point =
                                TilePosition::new((px / tile_size as f64) as i32, (py / tile_size as f64) as i32);
                        }
                    }
                }
            }

            _ => {}
        }
    }

    // Water tiles (tile id 3 in our tileset, which is gid 3) are also not walkable
    // For ground layer: if a tile is water and there's no wall above it, block it
    for (i, &tile_id) in ground_tiles.iter().enumerate() {
        if tile_id == 3 {
            let x = (i as i32) % width;
            let y = (i as i32) / width;
            collision_map.set_blocked(TilePosition::new(x, y));
        }
    }

    Ok(ParsedMap {
        width,
        height,
        collision_map,
        spawn_point,
        ground_tiles,
        wall_tiles,
        tile_size,
        tilesets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_test_map() {
        let content =
            std::fs::read_to_string("../../assets/maps/test_map.tmj").expect("test map exists");
        let map = parse_tmj(&content).expect("valid map");

        assert_eq!(map.width, 100);
        assert_eq!(map.height, 100);
        assert_eq!(map.tile_size, 32);
        assert_eq!(map.spawn_point, TilePosition::new(10, 10));

        // Ground layer should have width * height tiles
        assert_eq!(map.ground_tiles.len(), 10000);

        // Walls should block movement
        assert!(!map.collision_map.is_walkable(TilePosition::new(0, 0))); // border wall
        assert!(!map.collision_map.is_walkable(TilePosition::new(0, 50))); // left border

        // Town area should be walkable
        assert!(map.collision_map.is_walkable(TilePosition::new(10, 10)));

        // Spawn point should be walkable
        assert!(map.collision_map.is_walkable(map.spawn_point));

        // Lake water should be blocked
        assert!(!map.collision_map.is_walkable(TilePosition::new(50, 73)));

        // Out of bounds should be blocked
        assert!(!map.collision_map.is_walkable(TilePosition::new(-1, 0)));
        assert!(!map.collision_map.is_walkable(TilePosition::new(100, 0)));
    }
}
