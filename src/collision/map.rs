// src/collision/map.rs
use bevy::prelude::*;
use super::TileType;

/// Collision map resource that stores walkability information for the entire game map
#[derive(Resource)]
pub struct CollisionMap {
    pub tiles: Vec<TileType>,
    pub width: i32,
    pub height: i32,
    pub tile_size: f32,
    pub grid_origin_x: f32,
    pub grid_origin_y: f32,
}

impl CollisionMap {
    /// Create a map with explicit origin coordinates
    pub fn with_origin(width: i32, height: i32, tile_size: f32, origin_x: f32, origin_y: f32) -> Self {
        let size = (width * height) as usize;
        Self {
            tiles: vec![TileType::Empty; size],
            width,
            height,
            tile_size,
            grid_origin_x: origin_x,
            grid_origin_y: origin_y,
        }
    }
    
    /// Convert 2D grid coordinates to 1D array index
    pub fn xy_idx(&self, x: i32, y: i32) -> usize {
        (y as usize * self.width as usize) + x as usize
    }
    
    /// Check if grid coordinates are within bounds
    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < self.width && y >= 0 && y < self.height
    }
    
    /// Check if a grid position is walkable
    pub fn is_walkable(&self, x: i32, y: i32) -> bool {
        if !self.in_bounds(x, y) {
            return false;
        }
        let idx = self.xy_idx(x, y);
        self.tiles[idx].is_walkable()
    }
    
    /// Set a tile at grid position
    pub fn set_tile(&mut self, x: i32, y: i32, tile_type: TileType) {
        if self.in_bounds(x, y) {
            let idx = self.xy_idx(x, y);
            self.tiles[idx] = tile_type;
        }
    }
    
    /// Convert world position to grid coordinates
    pub fn world_to_grid(&self, world_pos: Vec2) -> IVec2 {
        let grid_x = ((world_pos.x - self.grid_origin_x) / self.tile_size).floor() as i32;
        let grid_y = ((world_pos.y - self.grid_origin_y) / self.tile_size).floor() as i32;
        IVec2::new(grid_x, grid_y)
    }
    
    /// Check if world position is walkable (single point)
    pub fn is_world_pos_walkable(&self, world_pos: Vec2) -> bool {
        let grid_pos = self.world_to_grid(world_pos);
        self.is_walkable(grid_pos.x, grid_pos.y)
    }
    
    /// Robust circle-based collision test
    pub fn is_world_pos_clear_circle(&self, world_pos: Vec2, radius_world: f32) -> bool {
        if !self.is_world_pos_within_bounds(world_pos, radius_world) {
            return false;
        }

        if radius_world <= 0.0 {
            return self.is_world_pos_walkable(world_pos);
        }

        let min_gx = ((world_pos.x - radius_world - self.grid_origin_x) / self.tile_size).floor() as i32;
        let max_gx = ((world_pos.x + radius_world - self.grid_origin_x) / self.tile_size).floor() as i32;
        let min_gy = ((world_pos.y - radius_world - self.grid_origin_y) / self.tile_size).floor() as i32;
        let max_gy = ((world_pos.y + radius_world - self.grid_origin_y) / self.tile_size).floor() as i32;

        for gy in min_gy..=max_gy {
            for gx in min_gx..=max_gx {
                if !self.in_bounds(gx, gy) {
                    return false;
                }
                if let Some(t) = self.get_tile(gx, gy) {
                    if !t.is_walkable() {
                        let effective_radius = match t {
                            TileType::Shore => radius_world + 0.1 * self.tile_size, // Shore gets extra buffer
                            TileType::Tree | TileType::Rock => radius_world + 0.05 * self.tile_size, // Props get small buffer for natural movement
                            _ => radius_world, // Water and other obstacles stay strict
                        };
                        
                        if self.circle_intersects_tile(world_pos, effective_radius, gx, gy) {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }

    /// Swept movement with axis-sliding
    pub fn try_move_circle(&self, start: Vec2, desired_end: Vec2, radius_world: f32) -> Vec2 {
        let delta = desired_end - start;
        let delta_len = delta.length();
        
        if delta_len < 0.001 {
            return start;
        }
        
        let max_step = self.tile_size * 0.25;
        let steps = (delta_len / max_step).ceil().max(1.0) as i32;
        let step_v = delta / steps as f32;

        let mut p = start;
        for _ in 0..steps {
            let candidate = p + step_v;
            
            if self.is_world_pos_clear_circle(candidate, radius_world) {
                p = candidate;
            } else {
                let try_x = Vec2::new(candidate.x, p.y);
                if self.is_world_pos_clear_circle(try_x, radius_world) {
                    p = try_x;
                    continue;
                }
                
                let try_y = Vec2::new(p.x, candidate.y);
                if self.is_world_pos_clear_circle(try_y, radius_world) {
                    p = try_y;
                    continue;
                }
                
                break;
            }
        }
        p
    }

    fn is_world_pos_within_bounds(&self, world_pos: Vec2, radius_world: f32) -> bool {
        let left = self.grid_origin_x;
        let right = self.grid_origin_x + self.width as f32 * self.tile_size;
        let bottom = self.grid_origin_y;
        let top = self.grid_origin_y + self.height as f32 * self.tile_size;

        world_pos.x - radius_world >= left
            && world_pos.x + radius_world <= right
            && world_pos.y - radius_world >= bottom
            && world_pos.y + radius_world <= top
    }

    fn circle_intersects_tile(&self, center: Vec2, radius: f32, gx: i32, gy: i32) -> bool {
        let min = Vec2::new(
            self.grid_origin_x + gx as f32 * self.tile_size,
            self.grid_origin_y + gy as f32 * self.tile_size,
        );
        let max = min + Vec2::splat(self.tile_size);

        let cx = center.x.clamp(min.x, max.x);
        let cy = center.y.clamp(min.y, max.y);

        let dx = center.x - cx;
        let dy = center.y - cy;
        dx * dx + dy * dy <= radius * radius
    }
    
    fn get_tile(&self, x: i32, y: i32) -> Option<TileType> {
        if self.in_bounds(x, y) {
            let idx = self.xy_idx(x, y);
            Some(self.tiles[idx])
        } else {
            None
        }
    }
}
