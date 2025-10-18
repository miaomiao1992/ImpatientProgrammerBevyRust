//! Rendering utilities module
//! Handles player depth calculation and other rendering-related systems

use bevy::prelude::*;

/// System to update player depth based on Y position to match tilemap Z system
/// This mirrors the same Z-depth calculation that bevy_procedural_tilemaps uses
/// with with_z_offset_from_y(true)
/// 
/// OPTIMIZATION: Only runs when player transform actually changes
pub fn update_player_depth(mut player_query: Query<&mut Transform, (With<crate::player::Player>, Changed<Transform>)>) {
    for mut transform in player_query.iter_mut() {
        let player_center_y = transform.translation.y;
        
        // Map configuration (from generate.rs)
        const TILE_SIZE: f32 = 64.0;
        const GRID_Y: u32 = 18;
        
        // CRITICAL FIX: Use player's FEET position for depth sorting, not center!
        // The player sprite is anchored at center, but for proper depth sorting
        // we need to consider where the player's feet are (bottom of sprite)
        // Player scale is 1.2, so sprite height is TILE_SIZE * 1.2 = 76.8
        // Feet are at: center_y - (sprite_height / 2) = center_y - 38.4
        const PLAYER_SCALE: f32 = 1.2;
        const PLAYER_SPRITE_HEIGHT: f32 = TILE_SIZE * PLAYER_SCALE; // 76.8
        let player_feet_y = player_center_y - (PLAYER_SPRITE_HEIGHT / 2.0); // Bottom of player sprite
        
        let map_height = TILE_SIZE * GRID_Y as f32;
        let map_y0 = -TILE_SIZE * GRID_Y as f32 / 2.0; // Map origin Y (from generate.rs)
        
        // Normalize player FEET Y to [0, 1] across the whole grid height
        let t = ((player_feet_y - map_y0) / map_height).clamp(0.0, 1.0);
        
        // Use the Y-to-Z formula from bevy_procedural_tilemaps:
        // z = base_z + NODE_SIZE.z * (1.0 - y / grid_height)
        // Where NODE_SIZE.z = 1.0 and base_z varies by layer (1.0 for dirt, 3.0 for yellowgrass, etc)
        // Props (trees, rocks) typically have base_z ≈ 4.0-5.0
        // To ensure proper Y-sorting with props, we need to be in the SAME Z range as props
        // but with a small offset to ensure consistent rendering order
        const NODE_SIZE_Z: f32 = 1.0;
        const PLAYER_BASE_Z: f32 = 4.0; // Match props base Z range for proper Y-sorting
        const PLAYER_Z_OFFSET: f32 = 0.5; // Larger offset to ensure player is ALWAYS above props
        let player_z = PLAYER_BASE_Z + NODE_SIZE_Z * (1.0 - t) + PLAYER_Z_OFFSET;
        
        transform.translation.z = player_z;
    }
}
