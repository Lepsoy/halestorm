use std::path::Path;

use crate::map::CollisionMap;
use crate::types::TilePosition;

/// Parsed data from a .tmj Tiled map file.
pub struct ParsedMap {
    pub width: i32,
    pub height: i32,
    pub collision_map: CollisionMap,
    pub spawn_point: TilePosition,
    pub ground_tiles: Vec<u32>,
    pub wall_tiles: Vec<u32>,
    pub tile_size: i32,
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
