// src/map/generate.rs
use bevy::prelude::*;
use bevy_procedural_tilemaps::prelude::*;
use std::collections::HashMap;

use crate::map::{
    assets::{load_assets, prepare_tilemap_handles},
    rules::build_world,
    Map, TileType, TileTypeMarker,
};

// -----------------  Configurable values ---------------------------
/// Modify these values to control the map size.
pub const GRID_X: u32 = 25;
pub const GRID_Y: u32 = 18;

// ------------------------------------------------------------------

const ASSETS_PATH: &str = "tile_layers";
const TILEMAP_FILE: &str = "tilemap.png";
/// Size of a block in world units (in Bevy 2d, 1 pixel is 1 world unit)
pub const TILE_SIZE: f32 = 64.;
/// Size of a grid node in world units
const NODE_SIZE: Vec3 = Vec3::new(TILE_SIZE, TILE_SIZE, 1.);

const ASSETS_SCALE: Vec3 = Vec3::new(2.0, 2.0, 1.0);
/// Number of z layers in the map, derived from the default terrain layers.
const GRID_Z: u32 = 5;

pub fn map_pixel_dimensions() -> Vec2 {
    Vec2::new(TILE_SIZE * GRID_X as f32, TILE_SIZE * GRID_Y as f32)
}

pub fn setup_generator(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    // 1. Rules Initialization - Get tile definitions and connection rules
    let (assets_definitions, models, socket_collection) = build_world();

    let rules = RulesBuilder::new_cartesian_3d(models, socket_collection)
        // Use ZForward as the up axis (rotation axis for models) since we are using Bevy in 2D
        .with_rotation_axis(Direction::ZForward)
        .build()
        .unwrap();

    // 2. Grid - Create 3D world space with wrapping behavior (false, false, false)
    let grid = CartesianGrid::new_cartesian_3d(GRID_X, GRID_Y, GRID_Z, false, false, false);

    // 3. Configuring the Algorithm - Set up WFC behavior
    let gen_builder = GeneratorBuilder::new()
        .with_rules(rules)
        .with_grid(grid.clone())
        .with_rng(RngMode::RandomSeed)
        .with_node_heuristic(NodeSelectionHeuristic::MinimumRemainingValue)
        .with_model_heuristic(ModelSelectionHeuristic::WeightedProbability);

    let generator = gen_builder.build().unwrap();

    // 4. Loading Assets - Load sprite atlas and convert to renderable assets
    let tilemap_handles =
        prepare_tilemap_handles(&asset_server, &mut atlas_layouts, ASSETS_PATH, TILEMAP_FILE);
    let models_assets = load_assets(&tilemap_handles, assets_definitions);

    // 5. Spawning the Generator - Create entity with Transform and NodesSpawner
    commands.spawn((
        Transform::from_translation(Vec3 {
            x: -TILE_SIZE * grid.size_x() as f32 / 2.,
            y: -TILE_SIZE * grid.size_y() as f32 / 2.,
            z: 0.,
        }),
        grid,
        generator,
        NodesSpawner::new(models_assets, NODE_SIZE, ASSETS_SCALE).with_z_offset_from_y(true),
    ));
}

/// Resource to track if we've built the collision map yet
#[derive(Resource, Default)]
pub struct CollisionMapBuilt(pub bool);

/// System that builds the collision map from spawned tiles
/// Runs once after WFC generation completes and tiles are spawned
/// 
/// IMPORTANT: This handles the multi-layer problem!
/// Your map has 5 z-layers (dirt, grass, yellow grass, water, props).
/// At each (x,y) position, we only keep the TOPMOST layer for collision.
/// Example: If there's dirt (z=0), grass (z=1), and water (z=3) at position (10,5),
/// we only mark it as Water (the highest/visible layer).
pub fn build_collision_map(
    mut commands: Commands,
    mut built: ResMut<CollisionMapBuilt>,
    tile_query: Query<(&TileTypeMarker, &Transform)>,
) {
    // Skip if already built
    if built.0 {
        return;
    }
    
    // Check if we have any tiles yet
    let tile_count = tile_query.iter().count();
    if tile_count == 0 {
        // WFC hasn't generated tiles yet, wait
        return;
    }
    
    info!("Building collision map from {} tiles...", tile_count);
    
    // Debug: Find the ACTUAL bounds of spawned tiles
    let (mut min_x, mut max_x) = (i32::MAX, i32::MIN);
    let (mut min_y, mut max_y) = (i32::MAX, i32::MIN);
    let grid_origin_x = -TILE_SIZE * GRID_X as f32 / 2.0;
    let grid_origin_y = -TILE_SIZE * GRID_Y as f32 / 2.0;
    
    for (marker, transform) in tile_query.iter() {
        let world_x = transform.translation.x;
        let world_y = transform.translation.y;
        let grid_x = ((world_x - grid_origin_x) / TILE_SIZE).floor() as i32;
        let grid_y = ((world_y - grid_origin_y) / TILE_SIZE).floor() as i32;
        
        min_x = min_x.min(grid_x);
        max_x = max_x.max(grid_x);
        min_y = min_y.min(grid_y);
        max_y = max_y.max(grid_y);
    }
    
    info!("🗺️  ACTUAL tile bounds: X [{} to {}] (width: {}), Y [{} to {}] (height: {})",
          min_x, max_x, max_x - min_x + 1, min_y, max_y, max_y - min_y + 1);
    info!("📏 Expected grid size: {}x{}", GRID_X, GRID_Y);
    
    // Debug: Count tile types
    let mut type_counts = HashMap::new();
    for (marker, _) in tile_query.iter() {
        *type_counts.entry(format!("{:?}", marker.tile_type)).or_insert(0) += 1;
    }
    info!("📊 Tile types found: {:?}", type_counts);
    
    // Create the map using ACTUAL bounds (not expected grid size)
    // The WFC can spawn tiles outside the grid due to offsets in models
    let actual_width = (max_x - min_x + 1) as i32;
    let actual_height = (max_y - min_y + 1) as i32;
    
    // Use the SAME grid_origin from bounds detection to ensure consistency
    let mut map = Map::with_origin(actual_width, actual_height, TILE_SIZE, grid_origin_x, grid_origin_y);
    
    info!("🎯 Created collision map: {}x{} at origin ({:.1}, {:.1})",
          actual_width, actual_height, grid_origin_x, grid_origin_y);
    
    // Track the highest z-layer at each (x,y) position
    // This solves the multi-layer problem: only the topmost visible tile matters!
    let mut layer_tracker: HashMap<(i32, i32), (TileType, f32)> = HashMap::new();
    
    // Scan all tiles and keep only the highest z-layer per position
    // Reusing grid_origin calculated above for consistency
    for (marker, transform) in tile_query.iter() {
        // Convert world position to grid coordinates
        let world_x = transform.translation.x;
        let world_y = transform.translation.y;
        let world_z = transform.translation.z; // Check z-height for layering
        
        let grid_x = ((world_x - grid_origin_x) / TILE_SIZE).floor() as i32;
        let grid_y = ((world_y - grid_origin_y) / TILE_SIZE).floor() as i32;
        
        let key = (grid_x, grid_y);
        
        // Only keep the tile with the HIGHEST z value at this position
        // This ensures water on top of dirt takes precedence
        match layer_tracker.get(&key) {
            Some((_, existing_z)) if world_z <= *existing_z => {
                // Lower or equal layer found, ignore this tile
                continue;
            }
            _ => {
                // Higher layer or first tile at this position - keep it
                layer_tracker.insert(key, (marker.tile_type, world_z));
            }
        }
    }
    
    info!("Processed {} tiles into {} unique grid positions", 
          tile_count, layer_tracker.len());
    
    // Debug: Count final tile types by category
    let mut final_counts = HashMap::new();
    for ((grid_x, grid_y), (tile_type, z_height)) in layer_tracker.iter() {
        *final_counts.entry(format!("{:?}", tile_type)).or_insert(0) += 1;
        
        // Convert world grid coordinates to local map array coordinates
        let local_x = grid_x - min_x;
        let local_y = grid_y - min_y;
        map.set_tile(local_x, local_y, *tile_type);
        
        // Debug: print unwalkable tiles with their layer info
        if !tile_type.is_walkable() {
            debug!("Unwalkable {:?} at world grid ({}, {}) → map [{}, {}] z={:.1}", 
                   tile_type, grid_x, grid_y, local_x, local_y, z_height);
        }
    }
    info!("📊 Final collision map tiles: {:?}", final_counts);
    
    // Post-processing: Convert water edges to shore tiles (walkable)
    convert_water_edges_to_shore(&mut map);
    
    // Recount after shore conversion
    let mut walkable = 0;
    let mut unwalkable = 0;
    for tile in &map.tiles {
        if tile.is_walkable() {
            walkable += 1;
        } else {
            unwalkable += 1;
        }
    }
    
    info!("Collision map built! Walkable: {}, Unwalkable: {}", 
          walkable, unwalkable);
    
    // DEBUG: Dump the collision map to a file for inspection
    dump_collision_map_to_file(&map, &layer_tracker, min_x, min_y);
    
    // Insert the map as a resource
    commands.insert_resource(map);
    
    // Mark as built
    built.0 = true;
}

/// Debug helper to dump collision map to a text file
fn dump_collision_map_to_file(
    map: &Map, 
    layer_tracker: &HashMap<(i32, i32), (TileType, f32)>,
    min_x: i32,
    min_y: i32,
) {
    use std::fs::File;
    use std::io::Write;
    
    let mut file = File::create("collision_map_debug.txt").unwrap();
    
    writeln!(file, "=== COLLISION MAP DEBUG ===").unwrap();
    writeln!(file, "Map dimensions: {}x{}", map.width, map.height).unwrap();
    writeln!(file, "Tile size: {}", map.tile_size).unwrap();
    writeln!(file, "Grid origin: ({:.1}, {:.1})", map.grid_origin_x, map.grid_origin_y).unwrap();
    writeln!(file, "Min coords offset: ({}, {})", min_x, min_y).unwrap();
    writeln!(file, "").unwrap();
    
    writeln!(file, "=== LAYER TRACKER (World Grid Coords) ===").unwrap();
    let mut sorted_tracker: Vec<_> = layer_tracker.iter().collect();
    sorted_tracker.sort_by_key(|((x, y), _)| (y, x));
    
    for ((world_x, world_y), (tile_type, z)) in sorted_tracker.iter().take(50) {
        let local_x = world_x - min_x;
        let local_y = world_y - min_y;
        writeln!(file, "World grid ({:3}, {:3}) → Local [{:3}, {:3}] = {:?} (z={:.1})",
                 world_x, world_y, local_x, local_y, tile_type, z).unwrap();
    }
    writeln!(file, "... ({} total entries)", layer_tracker.len()).unwrap();
    writeln!(file, "").unwrap();
    
    writeln!(file, "=== COLLISION MAP GRID (Local Coords) ===").unwrap();
    writeln!(file, "Legend: . = walkable, # = unwalkable, ? = empty").unwrap();
    writeln!(file, "").unwrap();
    
    // Print column headers
    write!(file, "     ").unwrap();
    for x in 0..map.width.min(50) {
        write!(file, "{}", x % 10).unwrap();
    }
    writeln!(file, "").unwrap();
    
    // Print rows
    for y in (0..map.height).rev() {
        write!(file, "{:3}: ", y).unwrap();
        for x in 0..map.width.min(50) {
            let idx = map.xy_idx(x, y);
            let tile = &map.tiles[idx];
            let ch = match tile {
                TileType::Empty => '?',
                _ if tile.is_walkable() => '.',
                _ => '#',
            };
            write!(file, "{}", ch).unwrap();
        }
        writeln!(file, "").unwrap();
    }
    
    writeln!(file, "").unwrap();
    writeln!(file, "=== TILE TYPE DETAILS ===").unwrap();
    for y in (0..map.height).rev().take(20) {
        for x in 0..map.width.min(20) {
            let idx = map.xy_idx(x, y);
            let tile = &map.tiles[idx];
            if *tile != TileType::Empty {
                writeln!(file, "[{:2}, {:2}] = {:?} (walkable: {})",
                         x, y, tile, tile.is_walkable()).unwrap();
            }
        }
    }
    
    info!("📝 Collision map dumped to collision_map_debug.txt");
}

/// Convert water edges adjacent to walkable tiles into shore tiles
/// This makes water edges traversable, creating a natural beach/shoreline
fn convert_water_edges_to_shore(map: &mut Map) {
    let mut shores_to_create = Vec::new();
    
    // Find all water tiles that touch walkable tiles
    for y in 0..map.height {
        for x in 0..map.width {
            let idx = map.xy_idx(x, y);
            
            // Only process water tiles
            if map.tiles[idx] != TileType::Water {
                continue;
            }
            
            // Check 8 neighbors (including diagonals)
            let neighbors = [
                (x - 1, y),     // left
                (x + 1, y),     // right
                (x, y - 1),     // down
                (x, y + 1),     // up
                (x - 1, y - 1), // bottom-left
                (x + 1, y - 1), // bottom-right
                (x - 1, y + 1), // top-left
                (x + 1, y + 1), // top-right
            ];
            
            // If any neighbor is walkable and in bounds, this water edge becomes shore
            for (nx, ny) in neighbors {
                if map.in_bounds(nx, ny) {
                    let neighbor_idx = map.xy_idx(nx, ny);
                    if map.tiles[neighbor_idx].is_walkable() {
                        shores_to_create.push((x, y));
                        break; // Found a walkable neighbor, mark as shore
                    }
                }
            }
        }
    }
    
    // Convert detected shore positions
    let shore_count = shores_to_create.len();
    for (x, y) in shores_to_create {
        map.set_tile(x, y, TileType::Shore);
    }
    
    if shore_count > 0 {
        info!("🏖️  Created {} shore tiles from water edges", shore_count);
    }
}
