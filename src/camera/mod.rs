//! Camera management module
//! Handles camera setup, following, projection configuration, fog of war, and rendering utilities

pub mod fog;
pub mod rendering;

use bevy::prelude::*;
use bevy::camera::Projection;

#[derive(Component)]
pub struct CameraFollow;

/// Setup the main camera
pub fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d::default(), CameraFollow));
}

/// System to configure camera projection to prevent Z-depth culling issues
/// 
/// OPTIMIZATION: Runs only once at startup instead of every frame
pub fn configure_camera_projection(
    mut camera_query: Query<&mut Projection, (With<Camera2d>, With<CameraFollow>)>,
) {
    for mut projection in camera_query.iter_mut() {
        if let Projection::Orthographic(ref mut ortho) = *projection {
            // Widen the camera's clip range to prevent objects from being culled
            // This makes debugging less brittle and prevents Z-depth issues
            ortho.near = -2000.0;
            ortho.far = 2000.0;
        }
    }
}

/// System to make the camera follow the player smoothly
/// 
/// OPTIMIZATION: Early exit if camera is already close to target position
pub fn follow_camera(
    player_query: Query<&Transform, With<crate::player::Player>>,
    mut camera_query: Query<&mut Transform, (With<Camera2d>, With<CameraFollow>, Without<crate::player::Player>)>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let Ok(mut camera_transform) = camera_query.single_mut() else {
        return;
    };

    let player_pos = Vec2::new(player_transform.translation.x, player_transform.translation.y);
    let camera_pos = Vec2::new(camera_transform.translation.x, camera_transform.translation.y);
    
    // Early exit if camera is already very close to player (within 0.5 pixels)
    let distance = player_pos.distance(camera_pos);
    if distance < 0.5 {
        return;
    }

    // Smoothly follow player
    let lerp_speed = 0.1;
    camera_transform.translation.x += (player_pos.x - camera_transform.translation.x) * lerp_speed;
    camera_transform.translation.y += (player_pos.y - camera_transform.translation.y) * lerp_speed;
    
    // Snap camera to pixel boundaries to prevent grid lines/shimmer
    camera_transform.translation.x = camera_transform.translation.x.round();
    camera_transform.translation.y = camera_transform.translation.y.round();
    camera_transform.translation.z = 1000.0; // Keep camera Z high
}
