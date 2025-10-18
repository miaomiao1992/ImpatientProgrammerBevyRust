use bevy::prelude::*;

use super::components::{
    ANIM_DT, AnimationState, AnimationTimer, DirectionalClips, Facing, MOVE_SPEED, PLAYER_Z,
    Player, TILE_SIZE, WALK_FRAMES,
};
use crate::collision::CollisionMap;

/// Resource to track if player has been spawned
#[derive(Resource, Default)]
struct PlayerSpawned(bool);

fn spawn_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    map: Option<Res<CollisionMap>>,
    mut player_spawned: ResMut<PlayerSpawned>,
    player_query: Query<(), With<Player>>,
) {
    // Skip if player already spawned
    if player_spawned.0 || !player_query.is_empty() {
        return;
    }
    
    // Only spawn if collision map is ready
    if map.is_none() {
        return;
    }
    let texture = asset_server.load("male_spritesheet.png");
    let layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::splat(TILE_SIZE),
        WALK_FRAMES as u32,
        12,
        None,
        None,
    ));

    let facing = Facing::Down;
    let directional_clips = DirectionalClips::walk(WALK_FRAMES);
    let start_index = directional_clips.clip(facing).start();

    // Find a walkable spawn position
    let spawn_pos = if let Some(map) = map {
        find_walkable_spawn_position(&map)
    } else {
        // Fallback to center if no map available yet
        Vec3::new(0., 0., PLAYER_Z)
    };

    commands.spawn((
        Sprite::from_atlas_image(
            texture,
            TextureAtlas {
                layout,
                index: start_index,
            },
        ),
        Transform::from_translation(spawn_pos).with_scale(Vec3::splat(1.2)),
        Player,
        directional_clips,
        AnimationState {
            facing,
            moving: false,
            was_moving: false,
        },
        AnimationTimer(Timer::from_seconds(ANIM_DT, TimerMode::Repeating)),
    ));
    
    // Mark as spawned
    player_spawned.0 = true;
    info!("🎮 Player spawned at walkable position: ({:.1}, {:.1})", spawn_pos.x, spawn_pos.y);
}

/// Find a walkable spawn position on the map
fn find_walkable_spawn_position(map: &CollisionMap) -> Vec3 {
    use rand::Rng;
    
    // FEET-BASED SPAWN: Ensure player's feet are on a walkable tile
    // Player scale is 1.2, so sprite height is TILE_SIZE * 1.2 = 76.8
    // Feet are at: center_y - (sprite_height / 2) = center_y - 38.4
    const PLAYER_SCALE: f32 = 1.2;
    const PLAYER_SPRITE_HEIGHT: f32 = TILE_SIZE as f32 * PLAYER_SCALE; // 76.8
    let feet_offset = PLAYER_SPRITE_HEIGHT / 2.0; // 38.4
    let collider_radius = 16.0; // Same as movement collision
    
    // Try to find a walkable position, starting from center and spiraling outward
    let center_x = map.width / 2;
    let center_y = map.height / 2;
    
    // Convert grid center to world position (this will be the CENTER of the player)
    let world_center_x = map.grid_origin_x + (center_x as f32 + 0.5) * map.tile_size;
    let world_center_y = map.grid_origin_y + (center_y as f32 + 0.5) * map.tile_size;
    
    // Check center first - ensure feet position is walkable
    let center_feet = Vec2::new(world_center_x, world_center_y - feet_offset);
    if map.is_world_pos_clear_circle(center_feet, collider_radius) {
        return Vec3::new(world_center_x, world_center_y, PLAYER_Z);
    }
    
    // Spiral outward from center
    for radius in 1..=std::cmp::min(map.width, map.height) / 2 {
        for dx in -radius..=radius {
            for dy in -radius..=radius {
                // Only check the perimeter of current radius
                if dx.abs() != radius && dy.abs() != radius {
                    continue;
                }
                
                let x = center_x + dx;
                let y = center_y + dy;
                
                if map.in_bounds(x, y) {
                    let world_x = map.grid_origin_x + (x as f32 + 0.5) * map.tile_size;
                    let world_y = map.grid_origin_y + (y as f32 + 0.5) * map.tile_size;
                    let feet_pos = Vec2::new(world_x, world_y - feet_offset);
                    
                    if map.is_world_pos_clear_circle(feet_pos, collider_radius) {
                        return Vec3::new(world_x, world_y, PLAYER_Z);
                    }
                }
            }
        }
    }
    
    // Fallback: random walkable position
    let mut rng = rand::thread_rng();
    for _ in 0..100 { // Try up to 100 random positions
        let x = rng.gen_range(0..map.width);
        let y = rng.gen_range(0..map.height);
        
        let world_x = map.grid_origin_x + (x as f32 + 0.5) * map.tile_size;
        let world_y = map.grid_origin_y + (y as f32 + 0.5) * map.tile_size;
        let feet_pos = Vec2::new(world_x, world_y - feet_offset);
        
        if map.is_world_pos_clear_circle(feet_pos, collider_radius) {
            return Vec3::new(world_x, world_y, PLAYER_Z);
        }
    }
    
    // Ultimate fallback: center of map
    warn!("Could not find walkable spawn position, using center");
    Vec3::new(world_center_x, world_center_y, PLAYER_Z)
}

fn move_player(
    input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    map: Option<Res<CollisionMap>>,
    mut player: Query<(&mut Transform, &mut AnimationState), With<Player>>,
) {
    let Ok((mut transform, mut anim)) = player.single_mut() else {
        return;
    };

    let mut direction = Vec2::ZERO;
    if input.pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0;
    }
    if input.pressed(KeyCode::ArrowRight) {
        direction.x += 1.0;
    }
    if input.pressed(KeyCode::ArrowUp) {
        direction.y += 1.0;
    }
    if input.pressed(KeyCode::ArrowDown) {
        direction.y -= 1.0;
    }

    if direction != Vec2::ZERO {
        // Calculate the proposed new position
        let delta = direction.normalize() * MOVE_SPEED * time.delta_secs();
        let current_center = Vec2::new(transform.translation.x, transform.translation.y);
        let new_center = current_center + delta;
        
        // FEET-BASED COLLISION: Use player's feet position for collision detection
        // This makes movement feel more natural - player can get closer to obstacles
        // Player scale is 1.2, so sprite height is TILE_SIZE * 1.2 = 76.8
        // Feet are at: center_y - (sprite_height / 2) = center_y - 38.4
        const PLAYER_SCALE: f32 = 1.2;
        const PLAYER_SPRITE_HEIGHT: f32 = TILE_SIZE as f32 * PLAYER_SCALE; // 76.8
        let feet_offset = PLAYER_SPRITE_HEIGHT / 2.0; // 38.4
        
        let current_feet = Vec2::new(current_center.x, current_center.y - feet_offset);
        let new_feet = Vec2::new(new_center.x, new_center.y - feet_offset);
        
        // Use robust circle-based collision with swept movement
        // Player collider radius = 16 pixels (reduced for more natural movement with leeway)
        let collider_radius = 16.0;
        
        let new_feet = if let Some(map) = map.as_ref() {
            // Use swept movement to prevent tunneling and ensure smooth collision
            map.try_move_circle(current_feet, new_feet, collider_radius)
        } else {
            new_feet
        };
        
        // Convert feet position back to center position
        let new_center = Vec2::new(new_feet.x, new_feet.y + feet_offset);
        
        // Only move if position changed (collision prevented perfect movement)
        let can_move = new_center != current_center;
        
        // Only move if the destination is walkable
        if can_move {
            transform.translation.x = new_center.x;
            transform.translation.y = new_center.y;
            anim.moving = true;

            // Update facing direction
            if direction.x.abs() > direction.y.abs() {
                anim.facing = if direction.x > 0.0 {
                    Facing::Right
                } else {
                    Facing::Left
                };
            } else {
                anim.facing = if direction.y > 0.0 {
                    Facing::Up
                } else {
                    Facing::Down
                };
            }
        } else {
            // Blocked by unwalkable tile - stop moving
            anim.moving = false;
        }
    } else {
        anim.moving = false;
    }
}

fn animate_player(
    time: Res<Time>,
    mut query: Query<
        (
            &DirectionalClips,
            &mut AnimationState,
            &mut AnimationTimer,
            &mut Sprite,
        ),
        With<Player>,
    >,
) {
    let Ok((clips, mut anim, mut timer, mut sprite)) = query.single_mut() else {
        return;
    };

    let atlas = match sprite.texture_atlas.as_mut() {
        Some(atlas) => atlas,
        None => return,
    };

    let clip = clips.clip(anim.facing);
    if !clip.contains(atlas.index) {
        atlas.index = clip.start();
        timer.reset();
    }

    let just_started = anim.moving && !anim.was_moving;
    let just_stopped = !anim.moving && anim.was_moving;

    if anim.moving {
        if just_started {
            atlas.index = clip.next(clip.start());
            timer.reset();
        } else {
            timer.tick(time.delta());
            if timer.just_finished() {
                atlas.index = clip.next(atlas.index);
            }
        }
    } else if just_stopped {
        atlas.index = clip.start();
        timer.reset();
    }

    anim.was_moving = anim.moving;
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerSpawned>()
            .add_systems(Update, (spawn_player, move_player, animate_player));
    }
}
