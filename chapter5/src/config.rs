pub mod player {
    /// Collision radius for the player's collider (in world units)
    pub const COLLIDER_RADIUS: f32 = 16.0;
    
    
    /// Z-position for player rendering (above terrain, below UI)
    pub const PLAYER_Z_POSITION: f32 = 20.0;
    
    /// Visual scale of the player sprite
    pub const PLAYER_SCALE: f32 = 0.8;
}

pub mod pickup {
    /// Default radius for item pickup detection (in world units)
    pub const DEFAULT_RADIUS: f32 = 40.0;
}

/// Map/terrain configuration
pub mod map {
    /// Size of a single tile in world units
    pub const TILE_SIZE: f32 = 32.0;
    
    /// Grid dimensions
    pub const GRID_X: u32 = 25;
    pub const GRID_Y: u32 = 18;
}