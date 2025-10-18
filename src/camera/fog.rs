//! Fog of war module
//! Handles circular fog of war vision system

use bevy::prelude::*;
use bevy::{
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d},
};

#[derive(Component)]
pub struct FogOfWar;

#[derive(Resource)]
pub struct VisionRadius(pub f32);

// Custom material for circular fog of war vision
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct CircularFogMaterial {
    #[uniform(0)]
    pub player_pos: Vec2,
    #[uniform(0)]
    pub vision_radius: f32,
}

impl Material2d for CircularFogMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/circular_fog.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// System to spawn the fog of war overlay
pub fn setup_fog_of_war(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CircularFogMaterial>>,
    vision_radius: Res<VisionRadius>,
) {
    let mesh = meshes.add(Rectangle::new(5000.0, 5000.0));
    let material = materials.add(CircularFogMaterial {
        player_pos: Vec2::ZERO,
        vision_radius: vision_radius.0,
    });
    
    commands.spawn((
        Mesh2d(mesh),
        MeshMaterial2d(material),
        Transform::from_translation(Vec3::new(0.0, 0.0, 900.0)),
        FogOfWar,
    ));
}

/// System to make the fog follow the player
/// 
/// OPTIMIZATION: Only updates when player moves significantly (more than 1 pixel)
pub fn follow_fog(
    player_query: Query<&Transform, (With<crate::player::Player>, Changed<Transform>)>,
    mut fog_query: Query<(&mut Transform, &MeshMaterial2d<CircularFogMaterial>), (With<FogOfWar>, Without<crate::player::Player>)>,
    mut materials: ResMut<Assets<CircularFogMaterial>>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let Ok((mut fog_transform, material_handle)) = fog_query.single_mut() else {
        return;
    };

    let player_pos = Vec2::new(player_transform.translation.x, player_transform.translation.y);

    // Update fog overlay position to follow player
    fog_transform.translation.x = player_pos.x;
    fog_transform.translation.y = player_pos.y;
    fog_transform.translation.z = 900.0;

    // Update fog material uniforms
    if let Some(material) = materials.get_mut(&material_handle.0) {
        material.player_pos = player_pos;
    }
}
