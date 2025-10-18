mod collision;
mod map;
mod player;

use bevy::{
    prelude::*,
    window::{Window, WindowPlugin, WindowMode, MonitorSelection},
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d, Material2dPlugin},
};
use bevy_procedural_tilemaps::prelude::*;

use crate::map::generate::{setup_generator, build_collision_map, CollisionMapBuilt};
use crate::player::PlayerPlugin;

#[cfg(debug_assertions)]
use crate::collision::{DebugCollisionEnabled, toggle_debug_collision, debug_draw_collision, debug_player_position, debug_log_tile_info};

#[derive(Component)]
struct CameraFollow;

#[derive(Component)]
struct FogOfWar;

// Custom material for circular fog of war vision
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct CircularFogMaterial {
    #[uniform(0)]
    player_pos: Vec2,
    #[uniform(0)]
    vision_radius: f32,
}

impl Material2d for CircularFogMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/circular_fog.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

#[derive(Resource)]
struct VisionRadius(f32);

fn main() {
    let vision_radius = 320.0;

    let mut app = App::new();
    
    app.insert_resource(ClearColor(Color::BLACK))
        .insert_resource(VisionRadius(vision_radius))
        .add_plugins((
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: "src/assets".into(),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Bevy Game".into(),
                        mode: WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
            Material2dPlugin::<CircularFogMaterial>::default(),
            ProcGenSimplePlugin::<Cartesian3D, Sprite>::default(),
            PlayerPlugin,
        ))
        .init_resource::<CollisionMapBuilt>()
        .add_systems(Startup, (setup_camera, setup_generator, setup_fog_of_war))
        .add_systems(Update, (build_collision_map, follow_player_and_fog));

    // Debug systems - only in debug builds
    #[cfg(debug_assertions)]
    {
        app.init_resource::<DebugCollisionEnabled>()
            .add_systems(Update, (
                toggle_debug_collision,
                debug_draw_collision,
                debug_player_position,
                debug_log_tile_info,
            ));
    }

    app.run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d::default(), CameraFollow));
}


fn setup_fog_of_war(
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

fn follow_player_and_fog(
    player_query: Query<&Transform, With<crate::player::Player>>,
    mut camera_query: Query<&mut Transform, (With<Camera2d>, Without<crate::player::Player>, Without<FogOfWar>)>,
    mut fog_query: Query<(&mut Transform, &MeshMaterial2d<CircularFogMaterial>), (With<FogOfWar>, Without<Camera2d>, Without<crate::player::Player>)>,
    mut materials: ResMut<Assets<CircularFogMaterial>>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let player_pos = Vec2::new(player_transform.translation.x, player_transform.translation.y);

    // Update camera with smooth following
    if let Ok(mut camera_transform) = camera_query.single_mut() {
        let lerp_speed = 0.1;
        camera_transform.translation.x += (player_pos.x - camera_transform.translation.x) * lerp_speed;
        camera_transform.translation.y += (player_pos.y - camera_transform.translation.y) * lerp_speed;
        
        // Snap to pixel boundaries for crisp rendering
        camera_transform.translation.x = camera_transform.translation.x.round();
        camera_transform.translation.y = camera_transform.translation.y.round();
        camera_transform.translation.z = 1000.0;
    }

    // Update fog of war overlay
    if let Ok((mut fog_transform, material_handle)) = fog_query.single_mut() {
        fog_transform.translation.x = player_pos.x;
        fog_transform.translation.y = player_pos.y;
        fog_transform.translation.z = 900.0;

        if let Some(material) = materials.get_mut(&material_handle.0) {
            material.player_pos = player_pos;
        }
    }
}
