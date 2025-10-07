//main.rs
use bevy::prelude::*;
use crate::player::PlayerPlugin;

mod player;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::WHITE)) // We have updated the bg color to white
        .add_plugins(
            DefaultPlugins.set(AssetPlugin {
                // Our assets live in `src/assets` for this project
                file_path: "src/assets".into(),
                ..default()
            }),
        )
        .add_systems(Startup, setup_camera)
        .add_plugins(PlayerPlugin)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
