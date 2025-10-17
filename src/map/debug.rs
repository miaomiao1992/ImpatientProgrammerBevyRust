// src/map/debug.rs
use bevy::prelude::*;
use super::Map;

/// Resource to track if debug visualization is enabled
#[derive(Resource, Default)]
pub struct DebugCollisionEnabled(pub bool);

/// Toggle debug collision visualization with F3 key
pub fn toggle_debug_collision(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut debug_enabled: ResMut<DebugCollisionEnabled>,
) {
    if keyboard.just_pressed(KeyCode::F3) {
        debug_enabled.0 = !debug_enabled.0;
        if debug_enabled.0 {
            info!("🔍 Collision debug visualization ENABLED (F3 to toggle)");
        } else {
            info!("Collision debug visualization disabled");
        }
    }
}

/// Draw colored rectangles over tiles to show walkability
/// Green = walkable, Red = unwalkable
pub fn debug_draw_collision(
    map: Option<Res<Map>>,
    debug_enabled: Res<DebugCollisionEnabled>,
    mut gizmos: Gizmos,
) {
    // Skip if debug is disabled or map not ready
    if !debug_enabled.0 {
        return;
    }
    
    let Some(map) = map.as_ref() else {
        return;
    };
    
    // Draw a colored square for each tile
    for y in 0..map.height {
        for x in 0..map.width {
            let idx = map.xy_idx(x, y);
            let tile = &map.tiles[idx];
            
            // Calculate world position for this tile (center of tile)
            let world_x = map.grid_origin_x + (x as f32 * map.tile_size) + (map.tile_size / 2.0);
            let world_y = map.grid_origin_y + (y as f32 * map.tile_size) + (map.tile_size / 2.0);
            
            // Choose color based on walkability
            let color = if tile.is_walkable() {
                Color::srgba(0.0, 1.0, 0.0, 0.3) // Green, 30% opacity for walkable
            } else {
                Color::srgba(1.0, 0.0, 0.0, 0.5) // Red, 50% opacity for unwalkable
            };
            
            // Draw rectangle (position, size, color)
            gizmos.rect_2d(
                Vec2::new(world_x, world_y),
                Vec2::splat(map.tile_size * 0.95), // Slightly smaller to see grid
                color,
            );
        }
    }
    
    // Draw legend in corner
    draw_legend(&mut gizmos, map);
}

/// Draw a legend showing what the colors mean
fn draw_legend(gizmos: &mut Gizmos, map: &Map) {
    let legend_x = map.grid_origin_x + 40.0;
    let legend_y = map.grid_origin_y + (map.height as f32 * map.tile_size) - 40.0;
    
    // Green square for walkable
    gizmos.rect_2d(
        Vec2::new(legend_x, legend_y),
        Vec2::splat(20.0),
        Color::srgba(0.0, 1.0, 0.0, 0.8),
    );
    
    // Red square for unwalkable
    gizmos.rect_2d(
        Vec2::new(legend_x, legend_y - 30.0),
        Vec2::splat(20.0),
        Color::srgba(1.0, 0.0, 0.0, 0.8),
    );
}

/// Draw player position indicator
pub fn debug_player_position(
    player: Query<&Transform, With<crate::player::Player>>,
    map: Option<Res<Map>>,
    debug_enabled: Res<DebugCollisionEnabled>,
    mut gizmos: Gizmos,
) {
    if !debug_enabled.0 {
        return;
    }
    
    let Some(map) = map.as_ref() else {
        return;
    };
    
    let Ok(transform) = player.single() else {
        return;
    };
    
    let pos = Vec2::new(transform.translation.x, transform.translation.y);
    let grid = map.world_to_grid(pos);
    
    // Draw yellow circle around player
    gizmos.circle_2d(pos, 50.0, Color::srgb(1.0, 1.0, 0.0));
    
    // NEW: Draw the actual collider circle (8px radius = light yellow)
    let collider_radius = 24.0;
    gizmos.circle_2d(
        pos, 
        collider_radius, 
        Color::srgba(1.0, 1.0, 0.5, 0.7)
    );
    
    // Draw grid cell outline
    if map.in_bounds(grid.x, grid.y) {
        let cell_center = Vec2::new(
            map.grid_origin_x + (grid.x as f32 * map.tile_size) + (map.tile_size / 2.0),
            map.grid_origin_y + (grid.y as f32 * map.tile_size) + (map.tile_size / 2.0),
        );
        
        gizmos.rect_2d(
            cell_center,
            Vec2::splat(map.tile_size),
            Color::srgb(1.0, 1.0, 0.0), // Yellow outline
        );
        
        // Check if current tile is walkable and print warning if on unwalkable
        let idx = map.xy_idx(grid.x, grid.y);
        let tile = &map.tiles[idx];
        if !tile.is_walkable() {
            // Draw red X over player if they're on unwalkable tile
            let offset = 15.0;
            gizmos.line_2d(
                Vec2::new(pos.x - offset, pos.y - offset),
                Vec2::new(pos.x + offset, pos.y + offset),
                Color::srgb(1.0, 0.0, 0.0),
            );
            gizmos.line_2d(
                Vec2::new(pos.x - offset, pos.y + offset),
                Vec2::new(pos.x + offset, pos.y - offset),
                Color::srgb(1.0, 0.0, 0.0),
            );
        }
    }
}

/// Print tile info to console when player moves
pub fn debug_log_tile_info(
    player: Query<&Transform, (With<crate::player::Player>, Changed<Transform>)>,
    map: Option<Res<Map>>,
    debug_enabled: Res<DebugCollisionEnabled>,
) {
    if !debug_enabled.0 {
        return;
    }
    
    let Some(map) = map.as_ref() else {
        return;
    };
    
    for transform in &player {
        let pos = Vec2::new(transform.translation.x, transform.translation.y);
        let grid = map.world_to_grid(pos);
        
        if map.in_bounds(grid.x, grid.y) {
            let idx = map.xy_idx(grid.x, grid.y);
            let tile = &map.tiles[idx];
            
            debug!(
                "Player at grid ({}, {}) | world ({:.1}, {:.1}) | tile: {:?} | walkable: {}",
                grid.x, grid.y, pos.x, pos.y, tile, tile.is_walkable()
            );
        }
    }
}

